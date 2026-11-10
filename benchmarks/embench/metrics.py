#!/usr/bin/env python3
"""Static and dynamic metrics of the Embench executables built by run_embench.py,
and the same metrics for RISC-V builds of the benchmarks.

For every <out>/<bench>/<bench>.elf: decodes the code with `scrymetrics static`
and runs the program on scryer with its event trace piped into `scrymetrics
dynamic` (nothing is stored). Writes <out>/<bench>/metrics.json and prints
Markdown tables; --tables-only re-reads existing metrics.json files.

With --rv32, also compiles each benchmark for rv32imc (and rv32im, for its code
size) through the same pipeline as scry-cc (clang -> opt -O2 -> llc), runs it on
qemu-riscv32 with its execution log piped into `scrymetrics rv32`, writes
<out>/<bench>/rv32imc.json and prints comparison tables. With --rv64, compiles
each benchmark for rv64imc through llvm2clif and Cranelift's riscv64 backend
(the same translation as for Scry, from 64-bit IR), with and without Cranelift's
optimizer, runs it on qemu-riscv64 and writes <out>/<bench>/rv64cl.json.
--skip-scry leaves the Scry passes out (their metrics.json files are re-read).

Usage: python3 benchmarks/embench/metrics.py [--out benchmarks/embench/out] [--jobs 3] [--rv32] [--rv64]
Needs tools/scrymetrics built (cargo build --release in tools/scrymetrics); the
RISC-V parts need clang with the RISC-V target, opt, llc, ld.lld, qemu-riscv32
and qemu-riscv64 (SCRY_CLANG, SCRY_OPT, SCRY_LLC, SCRY_LLD, SCRY_QEMU, SCRY_QEMU64
name them) and, for rv64, the llvm2clif binary (SCRY_LLVM2CLIF).
"""
import argparse, json, os, shutil, statistics, struct, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
CATS = ["alu", "dataflow", "control", "constant", "memory", "cast", "nop", "other"]


def measure(a, bench):
    d = Path(a.out) / bench
    elf = d / f"{bench}.elf"
    st = json.loads(subprocess.check_output([a.scrymetrics, "static", str(elf)], text=True))
    sim = subprocess.Popen([a.scryer, str(elf), "--stack-buffer", "1048576", "--print-events", *a.sim_args.split()],
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    dyn = subprocess.run([a.scrymetrics, "dynamic"], stdin=sim.stdout, capture_output=True, text=True, timeout=a.timeout)
    sim.wait()
    result = {"bench": bench, "static": st, "dynamic": json.loads(dyn.stdout)}
    (d / "metrics.json").write_text(json.dumps(result, indent=1))
    return result


def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        raise RuntimeError(f"{' '.join(map(str, cmd))}\n{r.stderr}")
    return r


def scry_cc_flags(d):
    """The -D/-f/-O/-std options and the heap size the Scry executable was compiled
    with, read from run_embench.py's compile.log (its first line is the scry-cc command)."""
    cflags, heap = ["-DWARMUP_HEAT=1", "-DGLOBAL_SCALE_FACTOR=1"], "8192"
    log = d / "compile.log"
    if log.exists():
        args = log.read_text().splitlines()[0].split()[2:]
        cflags = [x for x in args if x.startswith(("-D", "-f", "-O", "-std"))]
        if "--heap-size" in args:
            heap = args[args.index("--heap-size") + 1]
    return cflags, heap


def text_bytes(elf):
    """Sum of the .text* sections of a 32-bit little-endian ELF file."""
    data = Path(elf).read_bytes()
    shoff = struct.unpack_from("<I", data, 32)[0]
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", data, 46)
    secs = [struct.unpack_from("<IIIIII", data, shoff + i * shentsize) for i in range(shnum)]
    stroff = secs[shstrndx][4]
    def name(n):
        return data[stroff + n:data.index(b"\0", stroff + n)].decode()
    return sum(sec[5] for sec in secs if name(sec[0]).startswith(".text"))


RV_FRONT = ["-O2", "-S", "-emit-llvm", "-Xclang", "-disable-llvm-passes", "-fsigned-char", "-fno-vectorize", "-fno-slp-vectorize", "-g0"]


def build_riscv(a, bench, march, backend, elf):
    """Compiles the benchmark for RISC-V the way scry-cc compiles it for Scry (same
    sources, runtime, flags and `opt -O2` pipeline) and links it with ld.lld for
    qemu's user mode. `backend` is "llc" (LLVM's RISC-V backend) or "cranelift"
    (llvm2clif and Cranelift's riscv64 backend, the Scry translation); for the
    latter, `march` is rv64imc and `a.rv64_opt_level` selects Cranelift's
    optimization level."""
    repo = HERE / "embench-iot"
    d = Path(a.out) / bench
    srcs = sorted((repo / "src" / bench).glob("*.c")) + [repo / "support/main.c", repo / "support/beebsc.c", HERE / "boardsupport.c"]
    cflags, heap = scry_cc_flags(d)
    rv64 = march.startswith("rv64")
    target = [f"--target=riscv{64 if rv64 else 32}-unknown-none-elf", f"-march={march}", "-mabi=lp64" if rv64 else "-mabi=ilp32"]
    inc = ["-isystem", str(ROOT / "runtime/include"), f"-I{repo / 'support'}", f"-I{HERE / 'include'}"]
    tmp = Path(tempfile.mkdtemp(prefix=f"{bench}-{march}-{backend}-"))
    try:
        objs = []
        for src, extra in [(c, cflags) for c in srcs] + [(ROOT / "runtime/scryrt.c", ["-fno-builtin", f"-DSCRY_HEAP_SIZE={heap}"])]:
            pre, ll, obj = tmp / f"{src.stem}.pre.ll", tmp / f"{src.stem}.ll", tmp / f"{src.stem}.o"
            run([a.clang, *target, *RV_FRONT, *inc, *extra, "-o", pre, src])
            run([a.opt, "-O2", "-S", "-o", ll, pre])
            if backend == "llc":
                run([a.llc, "-O2", f"-march=riscv{64 if rv64 else 32}", "-mattr=+m,+c" if march.endswith("c") else "-mattr=+m", "-filetype=obj", "-o", obj, ll])
            else:
                run([a.llvm2clif, "--target", "riscv64", "--opt-level", a.rv64_opt_level, "-o", obj, ll])
            objs.append(obj)
        # Cranelift's riscv64 objects carry the double-float ABI flag (the
        # backend insists on the G extensions, though the code is integer
        # only), and the linker wants the start-up object to match.
        crt = tmp / "crt0.o"
        crt_target = ["--target=riscv64-unknown-none-elf", "-march=rv64imafdc", "-mabi=lp64d"] if backend == "cranelift" else target
        run([a.clang, *crt_target, "-c", "-o", crt, HERE / "riscv_crt0.S"])
        run([a.lld, "-o", elf, crt, *objs])
    finally:
        shutil.rmtree(tmp)
    return elf


def measure_riscv(a, elf, qemu, pass_name):
    """Runs an ELF on qemu with the execution log piped into the given pass of
    scrymetrics; returns (exit code, metrics)."""
    sim = subprocess.Popen([qemu, "-d", "exec,in_asm,nochain", str(elf)], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    dyn = subprocess.run([a.scrymetrics, pass_name], stdin=sim.stderr, capture_output=True, text=True, timeout=a.timeout)
    return sim.wait(), json.loads(dyn.stdout)


def measure_rv64(a, bench):
    """rv64imc through llvm2clif and Cranelift, once without and once with
    Cranelift's optimizer (the comparison with Scry uses the former)."""
    d = Path(a.out) / bench
    result = {"bench": bench}
    for level, key in [("none", "none"), ("speed", "speed")]:
        a.rv64_opt_level = level
        elf = build_riscv(a, bench, "rv64imc", "cranelift", d / f"rv64cl_{key}.elf")
        static = json.loads(run([a.scrymetrics, "rv64-static", elf]).stdout)
        code, dyn = measure_riscv(a, elf, a.qemu64, "rv64")
        result[key] = {"exit_code": code, "text_bytes": static["text_bytes"], "static": static, "dynamic": dyn}
    (d / "rv64cl.json").write_text(json.dumps(result, indent=1))
    return result


def measure_rv32(a, bench):
    d = Path(a.out) / bench
    static = {march: json.loads(run([a.scrymetrics, "rv32-static", build_riscv(a, bench, march, "llc", d / f"{march}.elf")]).stdout)
              for march in ("rv32im", "rv32imc")}
    code, dyn = measure_riscv(a, d / "rv32imc.elf", a.qemu, "rv32")
    result = {"bench": bench, "exit_code": code, "text_bytes": {m: st["text_bytes"] for m, st in static.items()},
              "static": static, "dynamic": dyn}
    (d / "rv32imc.json").write_text(json.dumps(result, indent=1))
    return result


def load(a, bench):
    """The stored results of a benchmark: metrics.json (Scry) plus rv32imc.json when present."""
    d = Path(a.out) / bench
    r = json.loads((d / "metrics.json").read_text()) if (d / "metrics.json").exists() else {"bench": bench}
    if (d / "rv32imc.json").exists():
        r["rv32"] = json.loads((d / "rv32imc.json").read_text())
    if (d / "rv64cl.json").exists():
        r["rv64"] = json.loads((d / "rv64cl.json").read_text())
    return r


def process(a, bench):
    if not a.tables_only and not a.skip_scry:
        measure(a, bench)
    if a.rv32 and not a.tables_only:
        measure_rv32(a, bench)
    if a.rv64 and not a.tables_only:
        measure_rv64(a, bench)
    return load(a, bench)


def pct(part, whole):
    return f"{100.0 * part / whole:5.1f}" if whole else "  -  "


def name(r):
    """Benchmark name, starred when the trace was cut short by an instruction limit."""
    return r["bench"] + ("*" if r["dynamic"].get("timed_out") else "")


def ratio(x, y):
    return f"{x / y:.2f}" if y else "  -  "


def variants(r):
    """The RISC-V results of a benchmark: (label, exit code, text bytes, static, dynamic)."""
    out = []
    if r.get("rv64"):
        v = r["rv64"]["none"]
        out.append(("rv64imc Cranelift", v["exit_code"], v["text_bytes"], v["static"], v["dynamic"]))
    if r.get("rv32"):
        v = r["rv32"]
        out.append(("rv32imc llc", v["exit_code"], v["text_bytes"]["rv32imc"], v["static"]["rv32imc"], v["dynamic"]))
    return out


def rv_tables(results):
    rs = [r for r in results if r.get("dynamic") and (r.get("rv32") or r.get("rv64"))]
    if not rs:
        return []
    labels = [v[0] for v in variants(rs[0])]
    both = len(labels) == 2
    slash = " / ".join(labels)
    out = ["\n### Scry against RISC-V: instructions executed\n"]
    head = "| benchmark | Scry |"
    for l in labels:
        head += f" {l} | Scry ratio |"
    if any(r.get("rv64") for r in rs):
        head += " rv64imc Cranelift, optimizer on | Scry ratio |"
    out += [head, "|---|---:|" + "---:|" * (head.count("|") - 3)]
    tot = {}
    for r in rs:
        s = r["dynamic"]["steps"]; tot["scry"] = tot.get("scry", 0) + s
        row = f"| {name(r)} | {s} |"
        for label, code, _, _, dyn in variants(r):
            v = dyn["instructions"]; tot[label] = tot.get(label, 0) + v
            row += f" {v}{'†' if code else ''} | {ratio(s, v)} |"
        if r.get("rv64"):
            v = r["rv64"]["speed"]["dynamic"]["instructions"]; tot["speed"] = tot.get("speed", 0) + v
            row += f" {v}{'†' if r['rv64']['speed']['exit_code'] else ''} | {ratio(s, v)} |"
        out.append(row)
    row = f"| **total** | **{tot['scry']}** |"
    for l in labels:
        row += f" **{tot[l]}** | **{ratio(tot['scry'], tot[l])}** |"
    if "speed" in tot:
        row += f" **{tot['speed']}** | **{ratio(tot['scry'], tot['speed'])}** |"
    out.append(row)

    out.append("\n### Code size against RISC-V\n")
    head = "| benchmark | Scry bytes | static instructions |"
    for l in labels:
        head += f" {l} bytes | Scry ratio | static instructions | compressed % |"
    if any(r.get("rv32") for r in rs):
        head += " rv32im llc bytes | Scry ratio |"
    out += [head, "|---|---:|---:|" + "---:|" * (head.count("|") - 4)]
    tot = {}
    for r in rs:
        sb = r["static"]["text_bytes"]; tot["scry"] = tot.get("scry", 0) + sb
        row = f"| {r['bench']} | {sb} | {r['static']['instructions']} |"
        for label, _, bytes_, st, _ in variants(r):
            tot[label] = tot.get(label, 0) + bytes_
            row += f" {bytes_} | {ratio(sb, bytes_)} | {st['instructions']} | {pct(st['compressed'], st['instructions']).strip()} |"
        if r.get("rv32"):
            ib = r["rv32"]["text_bytes"]["rv32im"]; tot["im"] = tot.get("im", 0) + ib
            row += f" {ib} | {ratio(sb, ib)} |"
        out.append(row)
    row = f"| **total** | **{tot['scry']}** | |"
    for l in labels:
        row += f" **{tot[l]}** | **{ratio(tot['scry'], tot[l])}** | | |"
    if "im" in tot:
        row += f" **{tot['im']}** | **{ratio(tot['scry'], tot['im'])}** |"
    out.append(row)

    out.append(f"\n### RISC-V dynamic instruction mix ({slash})\n")
    out.append("| benchmark | alu % | constant % | move % | control % | memory % | other % | compressed % | branches taken % |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    def cell(vals):
        return " / ".join(vals)
    for r in rs:
        vs = variants(r)
        cols = []
        for c in ("alu", "constant", "move", "control", "memory"):
            cols.append(cell(pct(d["by_category"].get(c, 0), d["instructions"]).strip() for _, _, _, _, d in vs))
        cols.append(cell(pct(sum(x for k, x in d["by_category"].items() if k not in ("alu", "constant", "move", "control", "memory")), d["instructions"]).strip() for _, _, _, _, d in vs))
        cols.append(cell(pct(d["forms"]["compressed"], d["instructions"]).strip() for _, _, _, _, d in vs))
        cols.append(cell(pct(d["forms"]["branches_taken"], d["forms"]["branches"]).strip() for _, _, _, _, d in vs))
        out.append(f"| {r['bench']} | " + " | ".join(cols) + " |")

    out.append(f"\n### Values: Scry operands against RISC-V register values (Scry / {slash})\n")
    out.append("| benchmark | lifetime median | mean | p90 | live values mean | max | never used % |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
    for r in rs:
        so = r["dynamic"]["operands"]; sl = so["lifetime"]; sq = r["dynamic"]["occupancy"]
        vs = [d for _, _, _, _, d in variants(r)]
        out.append(f"| {name(r)} | " + cell([str(sl["median"])] + [str(d["operands"]["lifetime"]["median"]) for d in vs])
                   + " | " + cell([f"{sl['mean']:.1f}"] + [f"{d['operands']['lifetime']['mean']:.1f}" for d in vs])
                   + " | " + cell([str(sl["p90"])] + [str(d["operands"]["lifetime"]["p90"]) for d in vs])
                   + " | " + cell([f"{sq['mean']:.1f}"] + [f"{d['occupancy']['mean']:.1f}" for d in vs])
                   + " | " + cell([str(sq["max"])] + [str(d["occupancy"]["max"]) for d in vs])
                   + " | " + cell([pct(so["discarded"], so["produced"]).strip()] + [pct(d["operands"]["discarded"], d["operands"]["defined"]).strip() for d in vs]) + " |")

    out.append(f"\n### Control flow and memory: Scry against RISC-V (Scry / {slash})\n")
    out.append("| benchmark | block length mean | median | calls | loads+stores | of which stack-relative | max stack bytes |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
    for r in rs:
        sd = r["dynamic"]; sm = sd["memory"]
        vs = [d for _, _, _, _, d in variants(r)]
        s_acc = sm["loads"] + sm["stores"] + sm["stack_loads"] + sm["stack_stores"]
        out.append(f"| {name(r)} | " + cell([f"{sd['blocks']['mean']:.1f}"] + [f"{d['blocks']['mean']:.1f}" for d in vs])
                   + " | " + cell([str(sd["blocks"]["median"])] + [str(d["blocks"]["median"]) for d in vs])
                   + " | " + cell([str(sd["control"].get("Call", {}).get("count", 0))] + [str(d["forms"]["calls"]) for d in vs])
                   + " | " + cell([str(s_acc)] + [str(d["memory"]["loads"] + d["memory"]["stores"]) for d in vs])
                   + " | " + cell([str(sm["stack_loads"] + sm["stack_stores"])] + [str(d["memory"]["stack_loads"] + d["memory"]["stack_stores"]) for d in vs])
                   + " | " + cell([str(sd["stack"]["max_depth_bytes"])] + [str(d["stack"]["max_depth_bytes"]) if d["stack"]["known"] else "?" for d in vs]) + " |")
    if any(code for r in rs for _, code, _, _, _ in variants(r)) or any(r.get("rv64") and r["rv64"]["speed"]["exit_code"] for r in rs):
        out.append("\n† the RISC-V build did not return 0 from `main` (its verification failed).")
    if any(not d["stack"]["known"] for r in rs for _, _, _, _, d in variants(r)):
        out.append("\n? the RISC-V code sets `sp` from a register (a variable-length array), which the execution log cannot follow.")
    return out


def tables(results):
    results = [r for r in results if r.get("dynamic")]
    out = []
    out.append("### Code size and static instruction mix\n")
    out.append("| benchmark | code bytes | instructions | " + " | ".join(f"{c} %" for c in CATS) + " | long echoes % of echoes | far jumps % of jumps | grow per const |")
    out.append("|---|---:|---:|" + "---:|" * (len(CATS) + 3))
    for r in results:
        s = r["static"]; n = s["instructions"]; bc = s["by_category"]; fm = s.get("forms", {})
        echoes = fm.get("echo_long", 0) + fm.get("echo_short", 0); jumps = fm.get("jmp_far", 0) + fm.get("jmp_short", 0)
        out.append(f"| {r['bench']} | {s['text_bytes']} | {n} | " + " | ".join(pct(bc.get(c, 0), n) for c in CATS)
                   + f" | {pct(fm.get('echo_long', 0), echoes)} | {pct(fm.get('jmp_far', 0), jumps)} | {s['by_mnemonic'].get('grow', 0) / max(s['by_mnemonic'].get('const', 0), 1):.2f} |")
    out.append("\n### Dynamic instruction mix\n")
    out.append("| benchmark | instructions | " + " | ".join(f"{c} %" for c in CATS) + " | echo | of which long | dup | const+grow | cast | far jumps % of jumps | 8/16-bit operands % |")
    out.append("|---|---:|" + "---:|" * (len(CATS) + 7))
    for r in results:
        d = r["dynamic"]; n = d["steps"]; bc = d["by_category"]; bm = d["by_mnemonic"]; fm = d.get("forms", {})
        jumps = fm.get("jmp_far", 0) + fm.get("jmp_short", 0)
        bt = d["operands"]["by_type"]; ops = sum(bt.values())
        narrow = sum(v for t, v in bt.items() if t in ("u8", "i8", "u16", "i16"))
        out.append(f"| {name(r)} | {n} | " + " | ".join(pct(bc.get(c, 0), n) for c in CATS)
                   + f" | {pct(bm.get('echo', 0), n)} | {pct(fm.get('echo_long', 0), bm.get('echo', 0))} | {pct(bm.get('dup', 0), n)} | {pct(bm.get('const', 0) + bm.get('grow', 0), n)} | {pct(bm.get('cast', 0), n)} | {pct(fm.get('jmp_far', 0), jumps)} | {pct(narrow, ops)} |")
    out.append("\n### Operand lifetimes (instructions from production to final consumption) and queue occupancy\n")
    out.append("| benchmark | operands | mean | median | p90 | p99 | max | discarded % | forwards per operand | live operands mean | max |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        o = r["dynamic"]["operands"]; lt = o["lifetime"]; occ = r["dynamic"]["occupancy"]
        out.append(f"| {name(r)} | {o['produced']} | {lt['mean']} | {lt['median']} | {lt['p90']} | {lt['p99']} | {lt['max']} | "
                   f"{pct(o['discarded'], o['produced'])} | {o['forwards_per_operand']['mean']} | {occ['mean']} | {occ['max']} |")
    out.append("\n### Control flow\n")
    out.append("| benchmark | calls | jumps | returns | jump issue to trigger mean | p99 | max | block length mean | median | dataflow instr per block |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        c = r["dynamic"]["control"]; b = r["dynamic"]["blocks"]
        j = c.get("Jump", {"mean": 0, "p99": 0, "max": 0, "count": 0})
        blocks = r["dynamic"]["by_category"].get("control", 0)
        out.append(f"| {name(r)} | {c.get('Call', {}).get('count', 0)} | {j['count']} | {c.get('Return', {}).get('count', 0)} | "
                   f"{j['mean']} | {j['p99']} | {j['max']} | {b['mean']} | {b['median']} | "
                   f"{r['dynamic']['by_category'].get('dataflow', 0) / max(blocks, 1):.1f} |")
    out.append("\n### Memory traffic and stack\n")
    out.append("| benchmark | loads | stores | stack-indexed loads | stack-indexed stores | bytes loaded | bytes stored | loads+stores per 100 instr | max stack bytes |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        m = r["dynamic"]["memory"]; n = r["dynamic"]["steps"]
        out.append(f"| {name(r)} | {m['loads']} | {m['stores']} | {m['stack_loads']} | {m['stack_stores']} | {m['load_bytes']} | {m['store_bytes']} | "
                   f"{100.0 * (m['loads'] + m['stores'] + m['stack_loads'] + m['stack_stores']) / n:.1f} | {r['dynamic']['stack']['max_depth_bytes']} |")
    out += rv_tables(results)
    if any(r["dynamic"].get("timed_out") for r in results):
        out.append("\n\\* traced up to the instruction limit given with `--sim-args` only; the program did not run to completion.")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", default=str(HERE / "out"))
    ap.add_argument("--scryer", default=os.environ.get("SCRY_SCRYER", "scryer"))
    ap.add_argument("--scrymetrics", default=str(ROOT / "tools/scrymetrics/target/release/scrymetrics"))
    ap.add_argument("--sim-args", default="")
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--timeout", type=float, default=7200)
    ap.add_argument("--only", default="")
    ap.add_argument("--tables-only", action="store_true")
    ap.add_argument("--rv32", action="store_true", help="also build, run and measure rv32imc versions (llc)")
    ap.add_argument("--rv64", action="store_true", help="also build, run and measure rv64imc versions (llvm2clif and Cranelift)")
    ap.add_argument("--skip-scry", action="store_true", help="do not rerun the Scry passes")
    for tool, default in [("clang", "clang"), ("opt", "opt"), ("llc", "llc"), ("lld", "ld.lld"), ("qemu", "qemu-riscv32"),
                          ("qemu64", "qemu-riscv64"), ("llvm2clif", str(ROOT / "target/release/llvm2clif"))]:
        ap.add_argument(f"--{tool}", default=os.environ.get(f"SCRY_{tool.upper()}", default))
    a = ap.parse_args()
    benches = sorted(p.parent.name for p in Path(a.out).glob("*/*.elf") if p.stem == p.parent.name)
    if a.only:
        benches = [b for b in benches if b in a.only.split(",")]
    with ThreadPoolExecutor(a.jobs) as ex:
        results = list(ex.map(lambda b: process(a, b), benches))
    if not a.tables_only:
        for r in results:
            line = f"{r['bench']:16}"
            if r.get("dynamic"):
                line += f" scry {r['dynamic']['steps']:>12} instructions, returned {r['dynamic']['returned']}"
            if r.get("rv32"):
                line += f"; rv32imc {r['rv32']['dynamic']['instructions']:>12} instructions, exit {r['rv32']['exit_code']}"
            if r.get("rv64"):
                v = r["rv64"]
                line += f"; rv64imc {v['none']['dynamic']['instructions']:>12} instructions, exit {v['none']['exit_code']} (optimized {v['speed']['dynamic']['instructions']}, exit {v['speed']['exit_code']})"
            print(line, file=sys.stderr, flush=True)
    print(tables(results))


if __name__ == "__main__":
    main()
