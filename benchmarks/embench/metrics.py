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
<out>/<bench>/rv32imc.json and prints comparison tables. --skip-scry leaves the
Scry passes out (their metrics.json files are re-read).

Usage: python3 benchmarks/embench/metrics.py [--out benchmarks/embench/out] [--jobs 3] [--rv32]
Needs tools/scrymetrics built (cargo build --release in tools/scrymetrics); the
RISC-V part needs clang with the RISC-V target, opt, llc, ld.lld and qemu-riscv32
(SCRY_CLANG, SCRY_OPT, SCRY_LLC, SCRY_LLD, SCRY_QEMU name them).
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


def build_rv32(a, bench, march):
    """Compiles the benchmark for RISC-V the way scry-cc compiles it for Scry (same
    sources, runtime, flags and `opt -O2` pipeline), with llc as the backend, and
    links it with ld.lld for qemu-riscv32; returns the ELF path."""
    repo = HERE / "embench-iot"
    d = Path(a.out) / bench
    srcs = sorted((repo / "src" / bench).glob("*.c")) + [repo / "support/main.c", repo / "support/beebsc.c", HERE / "boardsupport.c"]
    cflags, heap = scry_cc_flags(d)
    target = ["--target=riscv32-unknown-none-elf", f"-march={march}", "-mabi=ilp32"]
    inc = ["-isystem", str(ROOT / "runtime/include"), f"-I{repo / 'support'}", f"-I{HERE / 'include'}"]
    tmp = Path(tempfile.mkdtemp(prefix=f"{bench}-{march}-"))
    try:
        objs = []
        for src, extra in [(c, cflags) for c in srcs] + [(ROOT / "runtime/scryrt.c", ["-fno-builtin", f"-DSCRY_HEAP_SIZE={heap}"])]:
            pre, ll, obj = tmp / f"{src.stem}.pre.ll", tmp / f"{src.stem}.ll", tmp / f"{src.stem}.o"
            run([a.clang, *target, *RV_FRONT, *inc, *extra, "-o", pre, src])
            run([a.opt, "-O2", "-S", "-o", ll, pre])
            run([a.llc, "-O2", "-march=riscv32", "-mattr=+m,+c" if march == "rv32imc" else "-mattr=+m", "-filetype=obj", "-o", obj, ll])
            objs.append(obj)
        crt = tmp / "crt0.o"
        run([a.clang, *target, "-c", "-o", crt, HERE / "rv32_crt0.S"])
        elf = d / f"{march}.elf"
        run([a.lld, "-o", elf, crt, *objs])
    finally:
        shutil.rmtree(tmp)
    return elf


def measure_rv32(a, bench):
    d = Path(a.out) / bench
    static = {march: json.loads(run([a.scrymetrics, "rv32-static", build_rv32(a, bench, march)]).stdout) for march in ("rv32im", "rv32imc")}
    sim = subprocess.Popen([a.qemu, "-d", "exec,in_asm,nochain", str(d / "rv32imc.elf")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    dyn = subprocess.run([a.scrymetrics, "rv32"], stdin=sim.stderr, capture_output=True, text=True, timeout=a.timeout)
    result = {"bench": bench, "exit_code": sim.wait(), "text_bytes": {m: st["text_bytes"] for m, st in static.items()},
              "static": static, "dynamic": json.loads(dyn.stdout)}
    (d / "rv32imc.json").write_text(json.dumps(result, indent=1))
    return result


def load(a, bench):
    """The stored results of a benchmark: metrics.json (Scry) plus rv32imc.json when present."""
    d = Path(a.out) / bench
    r = json.loads((d / "metrics.json").read_text()) if (d / "metrics.json").exists() else {"bench": bench}
    if (d / "rv32imc.json").exists():
        r["rv32"] = json.loads((d / "rv32imc.json").read_text())
    return r


def process(a, bench):
    if not a.tables_only and not a.skip_scry:
        measure(a, bench)
    if a.rv32 and not a.tables_only:
        measure_rv32(a, bench)
    return load(a, bench)


def pct(part, whole):
    return f"{100.0 * part / whole:5.1f}" if whole else "  -  "


def name(r):
    """Benchmark name, starred when the trace was cut short by an instruction limit."""
    return r["bench"] + ("*" if r["dynamic"].get("timed_out") else "")


def ratio(x, y):
    return f"{x / y:.2f}" if y else "  -  "


def rv_tables(results):
    rs = [r for r in results if r.get("rv32") and r.get("dynamic")]
    if not rs:
        return []
    out = ["\n### Scry against rv32imc: instructions\n",
           "| benchmark | executed on Scry | executed on rv32imc | ratio | instruction bytes fetched, ratio | static Scry | static rv32imc | ratio |",
           "|---|---:|---:|---:|---:|---:|---:|---:|"]
    tot = [0] * 6
    for r in rs:
        s = r["dynamic"]["steps"]; v = r["rv32"]["dynamic"]["instructions"]
        vb = 4 * v - 2 * r["rv32"]["dynamic"]["forms"]["compressed"]  # 2 bytes per compressed instruction, 4 otherwise
        ss = r["static"]["instructions"]; vs = r["rv32"]["static"]["rv32imc"]["instructions"]
        mark = "†" if r["rv32"]["exit_code"] else ""
        out.append(f"| {name(r)}{mark} | {s} | {v} | {ratio(s, v)} | {ratio(2 * s, vb)} | {ss} | {vs} | {ratio(ss, vs)} |")
        for i, x in enumerate((s, v, vb, ss, vs)):
            tot[i] += x
    out.append(f"| **total** | **{tot[0]}** | **{tot[1]}** | **{ratio(tot[0], tot[1])}** | **{ratio(2 * tot[0], tot[2])}** | **{tot[3]}** | **{tot[4]}** | **{ratio(tot[3], tot[4])}** |")
    out.append("\n### Code size against rv32imc and rv32im\n")
    out.append("| benchmark | Scry bytes | rv32imc bytes | ratio | rv32im bytes | ratio | rv32imc compressed % |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
    tot = [0] * 3
    for r in rs:
        sb = r["static"]["text_bytes"]; cb = r["rv32"]["text_bytes"]["rv32imc"]; ib = r["rv32"]["text_bytes"]["rv32im"]
        st = r["rv32"]["static"]["rv32imc"]
        out.append(f"| {r['bench']} | {sb} | {cb} | {ratio(sb, cb)} | {ib} | {ratio(sb, ib)} | {pct(st['compressed'], st['instructions'])} |")
        for i, x in enumerate((sb, cb, ib)):
            tot[i] += x
    out.append(f"| **total** | **{tot[0]}** | **{tot[1]}** | **{ratio(tot[0], tot[1])}** | **{tot[2]}** | **{ratio(tot[0], tot[2])}** | |")
    out.append("\n### rv32imc dynamic instruction mix\n")
    out.append("| benchmark | instructions | alu % | constant % | move % | control % | memory % | other % | compressed % | branches taken % |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in rs:
        d = r["rv32"]["dynamic"]; n = d["instructions"]; bc = d["by_category"]; fm = d["forms"]
        other = sum(v for k, v in bc.items() if k not in ("alu", "constant", "move", "control", "memory"))
        out.append(f"| {r['bench']} | {n} | " + " | ".join(pct(bc.get(c, 0), n) for c in ("alu", "constant", "move", "control", "memory"))
                   + f" | {pct(other, n)} | {pct(fm['compressed'], n)} | {pct(fm['branches_taken'], fm['branches'])} |")
    out.append("\n### Values: Scry operands against rv32imc register values (Scry / rv32imc)\n")
    out.append("| benchmark | lifetime median | mean | p90 | live values mean | max | never used % |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
    for r in rs:
        so = r["dynamic"]["operands"]; sl = so["lifetime"]; sq = r["dynamic"]["occupancy"]
        vo = r["rv32"]["dynamic"]["operands"]; vl = vo["lifetime"]; vq = r["rv32"]["dynamic"]["occupancy"]
        out.append(f"| {name(r)} | {sl['median']} / {vl['median']} | {sl['mean']:.1f} / {vl['mean']:.1f} | {sl['p90']} / {vl['p90']} | "
                   f"{sq['mean']:.1f} / {vq['mean']:.1f} | {sq['max']} / {vq['max']} | "
                   f"{pct(so['discarded'], so['produced']).strip()} / {pct(vo['discarded'], vo['defined']).strip()} |")
    out.append("\n### Control flow and memory: Scry against rv32imc (Scry / rv32imc)\n")
    out.append("| benchmark | block length mean | median | calls | loads+stores | of which stack-relative | max stack bytes |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
    for r in rs:
        sd = r["dynamic"]; vd = r["rv32"]["dynamic"]; sm = sd["memory"]; vm = vd["memory"]
        s_acc = sm["loads"] + sm["stores"] + sm["stack_loads"] + sm["stack_stores"]; v_acc = vm["loads"] + vm["stores"]
        out.append(f"| {name(r)} | {sd['blocks']['mean']:.1f} / {vd['blocks']['mean']:.1f} | {sd['blocks']['median']} / {vd['blocks']['median']} | "
                   f"{sd['control'].get('Call', {}).get('count', 0)} / {vd['forms']['calls']} | "
                   f"{s_acc} / {v_acc} | "
                   f"{sm['stack_loads'] + sm['stack_stores']} / {vm['stack_loads'] + vm['stack_stores']} | "
                   f"{sd['stack']['max_depth_bytes']} / {vd['stack']['max_depth_bytes'] if vd['stack']['known'] else '?'} |")
    if any(r["rv32"]["exit_code"] for r in rs):
        out.append("\n† the rv32imc build did not return 0 from `main` (its verification failed).")
    if any(not r["rv32"]["dynamic"]["stack"]["known"] for r in rs):
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
    ap.add_argument("--rv32", action="store_true", help="also build, run and measure rv32imc versions")
    ap.add_argument("--skip-scry", action="store_true", help="do not rerun the Scry passes")
    for tool, default in [("clang", "clang"), ("opt", "opt"), ("llc", "llc"), ("lld", "ld.lld"), ("qemu", "qemu-riscv32")]:
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
            print(line, file=sys.stderr, flush=True)
    print(tables(results))


if __name__ == "__main__":
    main()
