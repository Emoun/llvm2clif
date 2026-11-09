#!/usr/bin/env python3
"""Static and dynamic metrics of the Embench executables built by run_embench.py.

For every <out>/<bench>/<bench>.elf: decodes the code with `scrymetrics static`
and runs the program on scryer with its event trace piped into `scrymetrics
dynamic` (nothing is stored). Writes <out>/<bench>/metrics.json and prints
Markdown tables; --tables-only re-reads existing metrics.json files.

Usage: python3 benchmarks/embench/metrics.py [--out benchmarks/embench/out] [--jobs 3]
Needs tools/scrymetrics built (cargo build --release in tools/scrymetrics).
"""
import argparse, json, os, statistics, subprocess, sys
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


def pct(part, whole):
    return f"{100.0 * part / whole:5.1f}" if whole else "  -  "


def name(r):
    """Benchmark name, starred when the trace was cut short by an instruction limit."""
    return r["bench"] + ("*" if r["dynamic"].get("timed_out") else "")


def tables(results):
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
    a = ap.parse_args()
    benches = sorted(p.parent.name for p in Path(a.out).glob("*/*.elf") if p.stem == p.parent.name)
    if a.only:
        benches = [b for b in benches if b in a.only.split(",")]
    if a.tables_only:
        results = [json.loads((Path(a.out) / b / "metrics.json").read_text()) for b in benches if (Path(a.out) / b / "metrics.json").exists()]
    else:
        with ThreadPoolExecutor(a.jobs) as ex:
            results = list(ex.map(lambda b: measure(a, b), benches))
            for r in results:
                print(f"{r['bench']:16} {r['dynamic']['steps']:>12} instructions, returned {r['dynamic']['returned']}", file=sys.stderr, flush=True)
    print(tables(results))


if __name__ == "__main__":
    main()
