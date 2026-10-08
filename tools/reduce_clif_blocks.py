#!/usr/bin/env python3
"""Block-level delta-debugging reducer for a single-function .clif file.

Removes basic blocks (ddmin) while `llvm2clif <file>` keeps failing with
stderr containing --match. A removed block is bypassed: every branch to it
goes to the first target of its own terminator instead, with the block's
parameters replaced by the `; dummy` constants of the entry block (as left
by reduce_clif.py, which should run first). Blocks ending in `return` or
`trap`, and the entry block, stay.

Usage: python3 tools/reduce_clif_blocks.py in.clif out.clif --match "did not converge" \
           [--llvm2clif target/release/llvm2clif] [--timeout 120]
"""
import argparse, re, subprocess, time
from pathlib import Path

HEADER = re.compile(r"^(block\d+)(\((.*)\))?:")


def parse(text):
    lines = text.splitlines(keepends=True)
    first = next(i for i, l in enumerate(lines) if HEADER.match(l))
    preamble, blocks, cur = lines[:first], {}, None
    tail = []
    for l in lines[first:]:
        m = HEADER.match(l)
        if m:
            cur = m.group(1)
            blocks[cur] = [l]
        elif l.startswith("}"):
            cur = None
            tail.append(l)
        elif cur is not None:
            blocks[cur].append(l)
        else:
            tail.append(l)
    return preamble, blocks, tail


def terminator_target(body):
    """The first target `(block, args_text)` of the block's terminator, or None."""
    term = next((l.strip() for l in reversed(body) if l.strip() and not l.strip().startswith(";")), "")
    term = term.split(";")[0].strip()
    m = re.match(r"jump (block\d+)(\((.*)\))?$", term)
    if m:
        return m.group(1), m.group(3) or ""
    m = re.match(r"brif \S+, (block\d+)(\(([^)]*)\))?, ", term)
    if m:
        return m.group(1), m.group(3) or ""
    m = re.match(r"br_table \S+, (block\d+)(\(([^)]*)\))?, \[", term)
    if m:
        return m.group(1), m.group(3) or ""
    return None


def params(body):
    m = HEADER.match(body[0])
    return re.findall(r"(v\d+): (i\d+)", m.group(3) or "")


def remove_blocks(blocks, drop, dummies):
    """Returns a copy of `blocks` without the blocks in `drop` (in order), or None."""
    blocks = {k: list(v) for k, v in blocks.items()}
    for b in [k for k in blocks if k in drop]:
        t = terminator_target(blocks[b])
        if t is None or t[0] == b:
            return None
        target, args = t
        for v, ty in params(blocks[b]):
            args = re.sub(rf"\b{v}\b", dummies[ty], args)
        repl = f"{target}({args})" if args else target
        pat = re.compile(rf"\b{b}\b(\([^)]*\))?")
        del blocks[b]
        for k, body in blocks.items():
            for i in range(1, len(body)):
                if body[i].startswith("    "):
                    body[i] = pat.sub(repl, body[i])
    return blocks


def interesting(text, a):
    tmp = Path(a.out).with_suffix(".try.clif")
    tmp.write_text(text)
    try:
        p = subprocess.run([a.llvm2clif, str(tmp), "-o", "/dev/null"], capture_output=True, text=True, timeout=a.timeout)
    except subprocess.TimeoutExpired:
        return a.match == "TIMEOUT"
    return p.returncode != 0 and a.match in p.stderr


def render(preamble, blocks, tail):
    out = list(preamble)
    for body in blocks.values():
        out.extend(body)
    out.extend(tail)
    return "".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("input"); ap.add_argument("out")
    ap.add_argument("--llvm2clif", default="llvm2clif")
    ap.add_argument("--match", required=True)
    ap.add_argument("--timeout", type=float, default=600)
    a = ap.parse_args()
    preamble, blocks, tail = parse(Path(a.input).read_text())
    entry = next(iter(blocks))
    dummies = {}
    for l in blocks[entry]:
        m = re.match(r"\s*(v\d+) = iconst\.(i\d+) 0\s*; dummy", l)
        if m:
            dummies[m.group(2)] = m.group(1)
    assert dummies, "no `; dummy` constants in the entry block (run reduce_clif.py first)"
    assert interesting(render(preamble, blocks, tail), a), "the input does not reproduce"
    tests = 0
    n = 2
    while True:
        cands = [k for k in blocks if k != entry and terminator_target(blocks[k])]
        if not cands:
            break
        n = min(n, len(cands))
        chunk = max(1, -(-len(cands) // n))
        reduced = False
        for start in range(0, len(cands), chunk):
            drop = set(cands[start:start + chunk])
            trial = remove_blocks(blocks, drop, dummies)
            if trial is None:
                continue
            tests += 1
            if interesting(render(preamble, trial, tail), a):
                blocks = trial
                Path(a.out).write_text(render(preamble, blocks, tail))
                print(f"[{time.strftime('%H:%M:%S')}] test {tests}: {len(blocks)} blocks left (dropped {len(drop)})", flush=True)
                reduced = True
                n = max(n - 1, 2)
                break
        if not reduced:
            if n >= len(cands):
                break
            n = min(n * 2, len(cands))
    Path(a.out).write_text(render(preamble, blocks, tail))
    print(f"done after {tests} tests: {len(blocks)} blocks left in {a.out}", flush=True)


if __name__ == "__main__":
    main()
