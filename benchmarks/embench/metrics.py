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
With --scry-speed, rebuilds each benchmark for Scry with Cranelift's optimizer
on (scry-cc --cranelift-opt-level speed, the command taken from compile.log)
and measures it too (metrics_speed.json). --skip-scry leaves the Scry passes
out (their metrics.json files are re-read).

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


def measure_scry(a, bench, elf):
    """Static and dynamic metrics of a Scry executable."""
    st = json.loads(subprocess.check_output([a.scrymetrics, "static", str(elf)], text=True))
    sim = subprocess.Popen([a.scryer, str(elf), "--stack-buffer", "1048576", "--print-events", *a.sim_args.split()],
                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    dyn = subprocess.run([a.scrymetrics, "dynamic"], stdin=sim.stdout, capture_output=True, text=True, timeout=a.timeout)
    sim.wait()
    return {"bench": bench, "static": st, "dynamic": json.loads(dyn.stdout)}


def measure(a, bench):
    d = Path(a.out) / bench
    result = measure_scry(a, bench, d / f"{bench}.elf")
    (d / "metrics.json").write_text(json.dumps(result, indent=1))
    return result


def measure_scry_speed(a, bench):
    """The benchmark built again with Cranelift's optimizer on (`scry-cc
    --cranelift-opt-level speed`; the command comes from run_embench.py's
    compile.log) and its metrics, in metrics_speed.json."""
    d = Path(a.out) / bench
    log = d / "compile.log"
    if not log.exists():
        raise RuntimeError(f"{log} is needed to rebuild {bench} with Cranelift's optimizer on")
    args = log.read_text().splitlines()[0].split()[2:]
    elf = d / f"{bench}.speed.elf"
    args[args.index("-o") + 1] = str(elf)
    run([a.scry_cc, *args, "--cranelift-opt-level", "speed"], timeout=a.compile_timeout)
    result = measure_scry(a, bench, elf)
    (d / "metrics_speed.json").write_text(json.dumps(result, indent=1))
    return result


def run(cmd, timeout=1800):
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
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
    if (d / "metrics_speed.json").exists():
        r["scry_speed"] = json.loads((d / "metrics_speed.json").read_text())
    if (d / "rv32imc.json").exists():
        r["rv32"] = json.loads((d / "rv32imc.json").read_text())
    if (d / "rv64cl.json").exists():
        r["rv64"] = json.loads((d / "rv64cl.json").read_text())
    return r


def process(a, bench):
    """Runs the requested measurements of one benchmark; a failure is reported at
    once and leaves that benchmark's earlier results in place."""
    try:
        if not a.tables_only and not a.skip_scry:
            measure(a, bench)
        if a.scry_speed and not a.tables_only:
            measure_scry_speed(a, bench)
        if a.rv32 and not a.tables_only:
            measure_rv32(a, bench)
        if a.rv64 and not a.tables_only:
            measure_rv64(a, bench)
    except Exception as e:  # noqa: BLE001 - reported, the other benchmarks go on
        print(f"{bench}: FAILED: {e}", file=sys.stderr, flush=True)
        a.failures.append(bench)
    return load(a, bench)


def pct(part, whole):
    return f"{100.0 * part / whole:5.1f}" if whole else "  -  "


def ratio(x, y):
    return f"{x / y:.2f}" if y else "  -  "


def name(r):
    """Benchmark name, starred when the trace was cut short by an instruction limit."""
    return r["bench"] + ("*" if r["dynamic"].get("timed_out") else "")


def variants(r):
    """The builds of a benchmark that have results, in a fixed order: label, kind
    (scry or riscv), static and dynamic metrics, whether the run verified."""
    out = []
    if r.get("dynamic"):
        d = r["dynamic"]
        out.append({"label": "Scry", "kind": "scry", "static": r["static"], "dynamic": d,
                    "ok": d.get("returned") == "0u32" and not d.get("timed_out")})
    if r.get("scry_speed"):
        v = r["scry_speed"]
        out.append({"label": "Scry, optimizer on", "kind": "scry", "static": v["static"], "dynamic": v["dynamic"],
                    "ok": v["dynamic"].get("returned") == "0u32"})
    if r.get("rv64"):
        for key, label in (("none", "rv64imc Cranelift"), ("speed", "rv64imc Cranelift, optimizer on")):
            v = r["rv64"][key]
            out.append({"label": label, "kind": "riscv", "static": v["static"], "dynamic": v["dynamic"], "ok": v["exit_code"] == 0})
    if r.get("rv32"):
        v = r["rv32"]
        out.append({"label": "rv32imc llc", "kind": "riscv", "static": v["static"]["rv32imc"], "dynamic": v["dynamic"], "ok": v["exit_code"] == 0})
    return out


def is_scry(v):
    return v["kind"] == "scry"


def instr(v):
    return v["dynamic"]["steps"] if is_scry(v) else v["dynamic"]["instructions"]


def fetched(v):
    d = v["dynamic"]
    return 2 * d["steps"] if is_scry(v) else 4 * d["instructions"] - 2 * d["forms"]["compressed"]


def mem_ops(v):
    m = v["dynamic"]["memory"]
    return m["loads"] + m["stores"] + (m["stack_loads"] + m["stack_stores"] if is_scry(v) else 0)


def hist(h, *keys):
    return " / ".join(f"{h[k]:.1f}" if isinstance(h[k], float) else str(h[k]) for k in keys)


LABELS = ["Scry", "Scry, optimizer on", "rv64imc Cranelift", "rv64imc Cranelift, optimizer on", "rv32imc llc"]


def variant_table(out, title, results, cell, note=""):
    """A table with one column per build: `cell(v)` renders a build's value."""
    present = {v["label"] for r in results for v in variants(r)}
    labels = [l for l in LABELS if l in present]
    out.append(f"\n### {title}\n")
    if note:
        out.append(note + "\n")
    out.append("| benchmark | " + " | ".join(labels) + " |")
    out.append("|---|" + "---:|" * len(labels))
    for r in results:
        by_label = {v["label"]: v for v in variants(r)}
        cells = []
        for l in labels:
            v = by_label.get(l)
            cells.append((cell(v) + ("" if v["ok"] else "†")) if v else "-")
        out.append(f"| {name(r)} | " + " | ".join(cells) + " |")


def mix_rows(out, results, pick):
    """The Scry dynamic mix table for the build `pick(r)` returns (None skips the row)."""
    out.append("| benchmark | instructions | " + " | ".join(f"{c} %" for c in CATS) + " | echo | of which long | dup | const+grow | cast | far jumps % of jumps | 8/16-bit operands % |")
    out.append("|---|---:|" + "---:|" * (len(CATS) + 7))
    for r in results:
        d = pick(r)
        if not d:
            continue
        n = d["steps"]; bc = d["by_category"]; bm = d["by_mnemonic"]; fm = d.get("forms", {})
        jumps = fm.get("jmp_far", 0) + fm.get("jmp_short", 0)
        bt = d["operands"]["by_type"]; ops = sum(bt.values())
        narrow = sum(v for t, v in bt.items() if t in ("u8", "i8", "u16", "i16"))
        out.append(f"| {name(r)} | {n} | " + " | ".join(pct(bc.get(c, 0), n) for c in CATS)
                   + f" | {pct(bm.get('echo', 0), n)} | {pct(fm.get('echo_long', 0), bm.get('echo', 0))} | {pct(bm.get('dup', 0), n)} | {pct(bm.get('const', 0) + bm.get('grow', 0), n)} | {pct(bm.get('cast', 0), n)} | {pct(fm.get('jmp_far', 0), jumps)} | {pct(narrow, ops)} |")


def tables(results):
    results = [r for r in results if r.get("dynamic")]
    out = []
    out.append("### Code size and static instruction mix (Scry)\n")
    out.append("| benchmark | code bytes | instructions | " + " | ".join(f"{c} %" for c in CATS) + " | long echoes % of echoes | far jumps % of jumps | grow per const |")
    out.append("|---|---:|---:|" + "---:|" * (len(CATS) + 3))
    for r in results:
        s = r["static"]; n = s["instructions"]; bc = s["by_category"]; fm = s.get("forms", {})
        echoes = fm.get("echo_long", 0) + fm.get("echo_short", 0); jumps = fm.get("jmp_far", 0) + fm.get("jmp_short", 0)
        out.append(f"| {r['bench']} | {s['text_bytes']} | {n} | " + " | ".join(pct(bc.get(c, 0), n) for c in CATS)
                   + f" | {pct(fm.get('echo_long', 0), echoes)} | {pct(fm.get('jmp_far', 0), jumps)} | {s['by_mnemonic'].get('grow', 0) / max(s['by_mnemonic'].get('const', 0), 1):.2f} |")

    out.append("\n### Dynamic instruction mix (Scry)\n")
    mix_rows(out, results, lambda r: r["dynamic"])
    if any(r.get("scry_speed") for r in results):
        out.append("\n### Dynamic instruction mix (Scry, Cranelift's optimizer on)\n")
        mix_rows(out, results, lambda r: r.get("scry_speed", {}).get("dynamic"))

    out.append("\n### Operand lifetimes and queue occupancy (Scry)\n")
    out.append("| benchmark | operands | lifetime mean | median | p90 | p99 | max | discarded % | forwards per operand | live operands mean | max |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        o = r["dynamic"]["operands"]; lt = o["lifetime"]; occ = r["dynamic"]["occupancy"]
        out.append(f"| {name(r)} | {o['produced']} | {lt['mean']} | {lt['median']} | {lt['p90']} | {lt['p99']} | {lt['max']} | {pct(o['discarded'], o['produced'])} | "
                   f"{o['forwards_per_operand']['mean']} | {occ['mean']} | {occ['max']} |")

    out.append("\n### Control flow (Scry)\n")
    out.append("| benchmark | calls | jumps | returns | jump issue to trigger mean | p99 | max | block length mean | median | dataflow instr per block |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        c = r["dynamic"]["control"]; b = r["dynamic"]["blocks"]
        j = c.get("Jump", {"mean": 0, "p99": 0, "max": 0, "count": 0})
        blocks = r["dynamic"]["by_category"].get("control", 0)
        out.append(f"| {name(r)} | {c.get('Call', {}).get('count', 0)} | {j['count']} | {c.get('Return', {}).get('count', 0)} | "
                   f"{j['mean']} | {j['p99']} | {j['max']} | {b['mean']} | {b['median']} | "
                   f"{r['dynamic']['by_category'].get('dataflow', 0) / max(blocks, 1):.1f} |")

    out.append("\n### Memory traffic and stack (Scry)\n")
    out.append("| benchmark | loads | stores | stack-indexed loads | stack-indexed stores | bytes loaded | bytes stored | loads+stores per 100 instr | max stack bytes |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in results:
        m = r["dynamic"]["memory"]; n = r["dynamic"]["steps"]
        out.append(f"| {name(r)} | {m['loads']} | {m['stores']} | {m['stack_loads']} | {m['stack_stores']} | {m['load_bytes']} | {m['store_bytes']} | "
                   f"{100.0 * (m['loads'] + m['stores'] + m['stack_loads'] + m['stack_stores']) / n:.1f} | {r['dynamic']['stack']['max_depth_bytes']} |")

    rv = [r for r in results if r.get("rv32") or r.get("rv64")]
    if rv:
        labels = [v["label"] for v in variants(rv[0]) if not is_scry(v)]
        out.append(f"\n### RISC-V dynamic instruction mix ({' / '.join(labels)})\n")
        out.append("| benchmark | alu % | constant % | move % | control % | memory % | other % | compressed % | branches taken % |")
        out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
        for r in rv:
            ds = [v["dynamic"] for v in variants(r) if not is_scry(v)]
            cols = []
            for c in ("alu", "constant", "move", "control", "memory"):
                cols.append(" / ".join(pct(d["by_category"].get(c, 0), d["instructions"]).strip() for d in ds))
            cols.append(" / ".join(pct(sum(x for k, x in d["by_category"].items() if k not in ("alu", "constant", "move", "control", "memory")), d["instructions"]).strip() for d in ds))
            cols.append(" / ".join(pct(d["forms"]["compressed"], d["instructions"]).strip() for d in ds))
            cols.append(" / ".join(pct(d["forms"]["branches_taken"], d["forms"]["branches"]).strip() for d in ds))
            out.append(f"| {r['bench']} | " + " | ".join(cols) + " |")

    multi = [r for r in results if len(variants(r)) > 1]
    if multi:
        variant_table(out, "Instructions: executed / static", multi, lambda v: f"{instr(v)} / {v['static']['instructions']}")
        variant_table(out, "Instruction bytes: fetched (dynamic) / code size (static)", multi, lambda v: f"{fetched(v)} / {v['static']['text_bytes']}",
                      "Scry instructions are 2 bytes; RISC-V ones 2 or 4.")
        variant_table(out, "Value lifetimes: median / mean / p90 (instructions from production to the last consumption)", multi,
                      lambda v: hist(v["dynamic"]["operands"]["lifetime"], "median", "mean", "p90"))
        variant_table(out, "First use of a value: dynamic median / mean · static median / mean", multi,
                      lambda v: hist(v["dynamic"]["operands"]["first_hop" if is_scry(v) else "first_use"], "median", "mean") + " · "
                      + hist(v["static"]["out_refs" if is_scry(v) else "first_use"], "median", "mean"),
                      "Dynamic: instructions from a value's production to its first forward or consumption (Scry) or to the first read of the "
                      "register (RISC-V). Static: the output reference distance of every instruction (Scry) or the distance to the first "
                      "reader of the register within the block (RISC-V).")
        variant_table(out, "Load to consume: dynamic median / mean / p90 · static median / mean", multi,
                      lambda v: hist(v["dynamic"]["operands"]["load_lifetime" if is_scry(v) else "load_use"], "median", "mean", "p90") + " · "
                      + hist(v["static"]["load_refs" if is_scry(v) else "load_use"], "median", "mean"),
                      "Dynamic: instructions from a load to the consumption of its value (Scry: the consuming instruction; RISC-V: the first "
                      "read of the loaded register). Static: the load's output reference (Scry) or the distance to the first reader in the "
                      "block (RISC-V).")
        variant_table(out, "Live values: mean / max; never used %", multi,
                      lambda v: hist(v["dynamic"]["occupancy"], "mean", "max") + "; "
                      + pct(v["dynamic"]["operands"]["discarded"], v["dynamic"]["operands"]["produced" if is_scry(v) else "defined"]).strip())
        variant_table(out, "Jump issue to trigger (Scry): dynamic mean / p99 / max · static mean / max", multi,
                      lambda v: (hist(v["dynamic"]["control"].get("Jump", {"mean": 0.0, "p99": 0, "max": 0}), "mean", "p99", "max") + " · "
                                 + hist(v["static"]["trigger"], "mean", "max")) if is_scry(v) else "-",
                      "The distance between a jump's issue and its trigger, in instructions; RISC-V has no such delay.")
        variant_table(out, "Executed block length: mean / median; calls", multi,
                      lambda v: hist(v["dynamic"]["blocks"], "mean", "median") + "; " + str(v["dynamic"]["control"].get("Call", {}).get("count", 0) if is_scry(v) else v["dynamic"]["forms"]["calls"]))
        variant_table(out, "Memory: loads+stores / of which stack-relative / bytes", multi,
                      lambda v: f"{mem_ops(v)} / {v['dynamic']['memory']['stack_loads'] + v['dynamic']['memory']['stack_stores']} / {v['dynamic']['memory']['load_bytes'] + v['dynamic']['memory']['store_bytes']}",
                      "Stack-relative: Scry's stack-indexed `ld`/`st`, RISC-V's `sp`-relative loads and stores (spills and locals).")
        variant_table(out, "Stack: max depth / bytes reserved / reservations · static frame size mean / max", multi,
                      lambda v: f"{v['dynamic']['stack']['max_depth_bytes'] if is_scry(v) or v['dynamic']['stack']['known'] else '?'} / "
                                f"{v['dynamic']['stack']['reserved_bytes']} / {v['dynamic']['stack']['reservations']} · " + hist(v["static"]["frames"], "mean", "max"),
                      "Dynamic: the deepest stack extent, the bytes reserved over the run (`rsrv` on Scry, constant decrements of `sp` on "
                      "RISC-V) and how often. Static: the sizes of those reservations in the code.")
    if any(not v["ok"] for r in results for v in variants(r)):
        out.append("\n† the run did not return 0 from `main` (its verification failed) or was cut short.")
    if any(not v["dynamic"]["stack"]["known"] for r in results for v in variants(r) if not is_scry(v)):
        out.append("\n? the RISC-V code sets `sp` from a register (a variable-length array), which the execution log cannot follow.")
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
    ap.add_argument("--timeout", type=float, default=7200, help="seconds per simulator run")
    ap.add_argument("--compile-timeout", type=float, default=1800, help="seconds per scry-cc build (--scry-speed)")
    ap.add_argument("--only", default="")
    ap.add_argument("--tables-only", action="store_true")
    ap.add_argument("--scry-speed", action="store_true", help="also rebuild each benchmark with Cranelift's optimizer on and measure it")
    ap.add_argument("--scry-cc", default=os.environ.get("SCRY_CC", str(ROOT / "target/release/scry-cc")))
    ap.add_argument("--rv32", action="store_true", help="also build, run and measure rv32imc versions (llc)")
    ap.add_argument("--rv64", action="store_true", help="also build, run and measure rv64imc versions (llvm2clif and Cranelift)")
    ap.add_argument("--skip-scry", action="store_true", help="do not rerun the Scry passes")
    for tool, default in [("clang", "clang"), ("opt", "opt"), ("llc", "llc"), ("lld", "ld.lld"), ("qemu", "qemu-riscv32"),
                          ("qemu64", "qemu-riscv64"), ("llvm2clif", str(ROOT / "target/release/llvm2clif"))]:
        ap.add_argument(f"--{tool}", default=os.environ.get(f"SCRY_{tool.upper()}", default))
    a = ap.parse_args()
    a.failures = []
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
            if r.get("scry_speed"):
                line += f" (optimized {r['scry_speed']['dynamic']['steps']}, returned {r['scry_speed']['dynamic']['returned']})"
            if r.get("rv32"):
                line += f"; rv32imc {r['rv32']['dynamic']['instructions']:>12} instructions, exit {r['rv32']['exit_code']}"
            if r.get("rv64"):
                v = r["rv64"]
                line += f"; rv64imc {v['none']['dynamic']['instructions']:>12} instructions, exit {v['none']['exit_code']} (optimized {v['speed']['dynamic']['instructions']}, exit {v['speed']['exit_code']})"
            print(line, file=sys.stderr, flush=True)
    print(tables(results))
    if a.failures:
        sys.exit(f"failed: {', '.join(a.failures)}")


if __name__ == "__main__":
    main()
