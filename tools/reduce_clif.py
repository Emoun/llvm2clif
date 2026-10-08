#!/usr/bin/env python3
"""Delta-debugging reducer for a single-function .clif file.

Removes instructions (ddmin) while `llvm2clif <file>` keeps failing with
stderr containing --match. A removed instruction's results are replaced, at
their uses, by a zero constant of the same type defined at the top of the
entry block, so most removals keep the function valid; variants that still
do not parse or verify are rejected. Terminators and block headers stay.
Progress is written to <out> after every successful reduction.

Usage: python3 tools/reduce_clif.py in.clif out.clif --match "did not converge" \
           [--llvm2clif target/release/llvm2clif] [--timeout 120]
"""
import argparse, re, subprocess, time
from pathlib import Path

TERMINATORS = ("jump", "brif", "br_table", "return", "trap", "fallthrough")
EXPLICIT = re.compile(r"= [a-z_]+\.(i8|i16|i32|i64|i128)\b")
RESULTS = re.compile(r"^\s*(v\d+(?:, v\d+)*) = ")


def value_types(lines):
    """Infers the integer type of every value from the CLIF text."""
    types = {}
    sigs = {}
    for l in lines:
        m = re.match(r"\s*(sig\d+) = \((.*?)\) -> (\S+)", l)
        if m:
            sigs[m.group(1)] = m.group(3)
        m = re.match(r"\s*(fn\d+) = %\S+ (sig\d+)", l)
        if m:
            sigs[m.group(1)] = sigs.get(m.group(2))
    for l in lines:
        for v, t in re.findall(r"(v\d+): (i\d+)", l):
            types[v] = t
        m = RESULTS.match(l)
        if not m:
            continue
        results = m.group(1).split(", ")
        rhs = l[m.end():]
        op = rhs.split()[0].split(".")[0]
        e = EXPLICIT.search(l)
        if e:
            t = e.group(1)
        elif op in ("icmp", "icmp_imm"):
            t = "i8"
        elif op == "call":
            t = sigs.get(rhs.split()[1].split("(")[0])
        elif op == "select":
            ops = re.findall(r"\bv\d+\b", rhs)
            t = types.get(ops[1]) if len(ops) > 1 else None
        else:
            ops = re.findall(r"\bv\d+\b", rhs)
            t = types.get(ops[0]) if ops else None
        for r in results:
            if t:
                types[r] = t
    return types


def interesting(lines, a):
    tmp = Path(a.out).with_suffix(".try.clif")
    tmp.write_text("".join(lines))
    try:
        p = subprocess.run([a.llvm2clif, str(tmp), "-o", "/dev/null"], capture_output=True, text=True, timeout=a.timeout)
    except subprocess.TimeoutExpired:
        return a.match == "TIMEOUT"
    return p.returncode != 0 and a.match in p.stderr


def is_cand(l):
    s = l.strip()
    return l.startswith("    ") and s and not s.startswith(";") and s.split()[0].split(".")[0] not in TERMINATORS and not s.endswith("; dummy")


def remove(lines, drop, types, dummies):
    """Removes the candidate lines in `drop`, substituting their results."""
    subst = {}
    for i in drop:
        m = RESULTS.match(lines[i])
        if m:
            for r in m.group(1).split(", "):
                d = dummies.get(types.get(r))
                if d is None:
                    return None
                subst[r] = d
    out = []
    pat = re.compile(r"\b(" + "|".join(sorted(subst, key=len, reverse=True)) + r")\b") if subst else None
    for i, l in enumerate(lines):
        if i in drop:
            continue
        if pat and l.startswith("    "):
            l = pat.sub(lambda m: subst[m.group(1)], l)
        out.append(l)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("input"); ap.add_argument("out")
    ap.add_argument("--llvm2clif", default="llvm2clif")
    ap.add_argument("--match", required=True, help="substring of stderr that marks a reproduction")
    ap.add_argument("--timeout", type=float, default=600)
    a = ap.parse_args()
    lines = Path(a.input).read_text().splitlines(keepends=True)
    types = value_types(lines)
    # Dummy constants of every integer type at the top of the entry block.
    top = next(i for i, l in enumerate(lines) if l.startswith("block"))
    maxv = max(int(v[1:]) for v in types) if types else 0
    dummies = {}
    for t in ("i8", "i16", "i32", "i64"):
        maxv += 1
        dummies[t] = f"v{maxv}"
        lines.insert(top + 1, f"    v{maxv} = iconst.{t} 0  ; dummy\n")
    assert interesting(lines, a), "the input (with dummy constants added) does not reproduce"
    tests = 0
    n = 2
    while True:
        cands = [i for i, l in enumerate(lines) if is_cand(l)]
        if not cands:
            break
        n = min(n, len(cands))
        chunk = max(1, -(-len(cands) // n))
        reduced = False
        for start in range(0, len(cands), chunk):
            drop = set(cands[start:start + chunk])
            trial = remove(lines, drop, types, dummies)
            if trial is None:
                continue
            tests += 1
            if interesting(trial, a):
                lines = trial
                Path(a.out).write_text("".join(lines))
                left = len([l for l in lines if is_cand(l)])
                print(f"[{time.strftime('%H:%M:%S')}] test {tests}: {left} instructions left (dropped {len(drop)})", flush=True)
                reduced = True
                n = max(n - 1, 2)
                break
        if not reduced:
            if n >= len(cands):
                break
            n = min(n * 2, len(cands))
    Path(a.out).write_text("".join(lines))
    print(f"done after {tests} tests: {len([l for l in lines if is_cand(l)])} instructions left in {a.out}", flush=True)


if __name__ == "__main__":
    main()
