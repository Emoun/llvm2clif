#!/usr/bin/env python3
"""Build every Embench-IoT benchmark with scry-cc and run it on scryer.

Each benchmark passes when main() returns 0, i.e. verify_benchmark() accepted
the result. Results go to <out>/results.json and <out>/<bench>/*.log; see
README.md in this directory for the expected outcomes.

Usage: python3 benchmarks/embench/run_embench.py [--only crc32,edn] [--jobs 2]
The Embench sources are cloned into benchmarks/embench/embench-iot (at the
pinned commit) unless --repo points at a checkout. scry-cc finds wild and
scryer on PATH or through SCRY_WILD and SCRY_SCRYER.
"""
import argparse, json, os, re, subprocess, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
EMBENCH_GIT = "https://github.com/embench/embench-iot.git"
EMBENCH_REV = "09c2ed8"  # embench-0.5-100-g09c2ed8


def checkout(repo):
    """Clones Embench at the pinned revision unless `repo` already exists."""
    if (repo / "src").is_dir():
        return
    print(f"cloning {EMBENCH_GIT} into {repo}", flush=True)
    subprocess.check_call(["git", "clone", "-q", EMBENCH_GIT, str(repo)])
    subprocess.check_call(["git", "-C", str(repo), "checkout", "-q", EMBENCH_REV])


def run(cmd, timeout, log):
    t0 = time.time()
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        rc, out, err, to = p.returncode, p.stdout, p.stderr, False
    except subprocess.TimeoutExpired as e:
        dec = lambda b: b.decode(errors="replace") if isinstance(b, bytes) else (b or "")
        rc, out, err, to = None, dec(e.stdout), dec(e.stderr), True
    dt = time.time() - t0
    log.write_text("$ " + " ".join(map(str, cmd)) + f"\n[exit {rc}, {dt:.1f}s{', TIMEOUT' if to else ''}]\n--- stdout\n{out}\n--- stderr\n{err}\n")
    return rc, out, err, dt, to


def classify_compile(err):
    for line in err.splitlines():
        l = line.lower()
        if "panicked at" in l:
            return "backend panic", line.strip()
        if "not supported" in l or "unsupported" in l:
            return "unsupported", line.strip()
    for line in err.splitlines():
        if "error" in line.lower():
            return "compile error", line.strip()
    return "compile error", (err.strip().splitlines() or ["?"])[-1]


def parse_run(rc, o, e, to, timeout):
    """Classifies a simulator run from its exit code and output."""
    r = {}
    for m in re.finditer(r"^(InstructionReads|DataReads|DataWrites|StackReads|StackWrites):\s*(\d+)", o, re.M):
        r[m.group(1)] = int(m.group(2))
    if to:
        r.update(status="run timeout", detail=f"> {timeout}s")
        return r
    lines = o.splitlines()
    returned = ""
    for i, l in enumerate(lines):
        if "Returned Operands" in l and i + 1 < len(lines):
            returned = lines[i + 1].split(",")[0].strip()
            break
    if rc != 0:
        panic = next((l.strip() for l in (e + o).splitlines() if "panicked" in l or "error" in l.lower()), "")
        r.update(status="simulator error", detail=panic or f"exit {rc}")
    elif re.fullmatch(r"0[ui]\d+", returned):
        r.update(status="pass", detail="")
    else:
        r.update(status="verify failed", detail=f"returned {returned or '?'}")
    return r


def one(a, bench):
    repo, out = Path(a.repo), Path(a.out) / bench
    out.mkdir(parents=True, exist_ok=True)
    srcs = sorted((repo / "src" / bench).glob("*.c"))
    elf = out / f"{bench}.elf"
    cmd = [a.scry_cc, *map(str, srcs), str(repo / "support/main.c"), str(repo / "support/beebsc.c"),
           str(Path(a.support) / "boardsupport.c"), f"-I{repo / 'support'}", f"-I{Path(a.support) / 'include'}",
           f"-DWARMUP_HEAT={a.warmup}", f"-DGLOBAL_SCALE_FACTOR={a.gsf}", *a.cflags.split(), "-o", str(elf)]
    r = {"bench": bench}
    rc, o, e, dt, to = run(cmd, a.compile_timeout, out / "compile.log")
    r["compile_s"] = round(dt, 1)
    if to:
        r.update(status="compile timeout", detail=f"> {a.compile_timeout}s")
        return r
    if rc != 0:
        st, detail = classify_compile(e + o)
        r.update(status=st, detail=detail)
        return r
    r["elf_bytes"] = elf.stat().st_size
    if a.no_run:
        r.update(status="built", detail="")
        return r
    rc, o, e, dt, to = run([a.scryer, str(elf), "--target=scry32-unknown-none-elf"], a.run_timeout, out / "run.log")
    r["run_s"] = round(dt, 1)
    r.update(parse_run(rc, o, e, to, a.run_timeout))
    return r


def reparse(a, bench):
    """Re-derives a benchmark's result from the logs of an earlier run."""
    out = Path(a.out) / bench
    r = {"bench": bench}
    log = out / "compile.log"
    if not log.exists():
        return r | {"status": "missing", "detail": "no compile.log"}
    head, _, rest = log.read_text().partition("\n--- stdout\n")
    m = re.search(r"\[exit (\S+), ([\d.]+)s(, TIMEOUT)?\]", head)
    o, _, e = rest.partition("\n--- stderr\n")
    r["compile_s"] = float(m.group(2))
    if m.group(3):
        return r | {"status": "compile timeout", "detail": ""}
    if m.group(1) != "0":
        st, detail = classify_compile(e + o)
        return r | {"status": st, "detail": detail}
    log = out / "run.log"
    if not log.exists():
        return r | {"status": "built", "detail": ""}
    head, _, rest = log.read_text().partition("\n--- stdout\n")
    m = re.search(r"\[exit (\S+), ([\d.]+)s(, TIMEOUT)?\]", head)
    o, _, e = rest.partition("\n--- stderr\n")
    r["run_s"] = float(m.group(2))
    r.update(parse_run(None if m.group(1) == "None" else int(m.group(1)), o, e, bool(m.group(3)), a.run_timeout))
    return r


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo", default=str(HERE / "embench-iot"), help="embench-iot checkout (cloned if missing)")
    ap.add_argument("--support", default=str(HERE), help="directory with boardsupport.c and include/")
    ap.add_argument("--out", default=str(HERE / "out"))
    ap.add_argument("--scry-cc", default=str(ROOT / "target/release/scry-cc") if (ROOT / "target/release/scry-cc").exists() else "scry-cc")
    ap.add_argument("--scryer", default=os.environ.get("SCRY_SCRYER", "scryer"))
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("--compile-timeout", type=float, default=1800)
    ap.add_argument("--run-timeout", type=float, default=1800)
    ap.add_argument("--only", default="", help="comma-separated benchmark names")
    ap.add_argument("--skip", default="")
    ap.add_argument("--cflags", default="")
    ap.add_argument("--warmup", default="1")
    ap.add_argument("--gsf", default="1")
    ap.add_argument("--no-run", action="store_true")
    ap.add_argument("--reparse", action="store_true", help="re-read the logs of an earlier run instead of running")
    a = ap.parse_args()
    checkout(Path(a.repo))
    benches = sorted(p.name for p in (Path(a.repo) / "src").iterdir() if p.is_dir())
    if a.only:
        benches = [b for b in benches if b in a.only.split(",")]
    if a.skip:
        benches = [b for b in benches if b not in a.skip.split(",")]
    Path(a.out).mkdir(parents=True, exist_ok=True)
    results = []
    with ThreadPoolExecutor(a.jobs) as ex:
        for r in ex.map(lambda b: reparse(a, b) if a.reparse else one(a, b), benches):
            results.append(r)
            print(f"{r['bench']:16} {r['status']:18} compile {r.get('compile_s', 0):7.1f}s  run {r.get('run_s', 0):7.1f}s  "
                  f"instr {r.get('InstructionReads', 0):>12}  {r.get('detail', '')[:100]}", flush=True)
    (Path(a.out) / "results.json").write_text(json.dumps(results, indent=1))
    n = sum(r["status"] == "pass" for r in results)
    print(f"\n{n} of {len(results)} benchmarks pass")


if __name__ == "__main__":
    main()
