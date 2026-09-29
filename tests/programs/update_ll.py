#!/usr/bin/env python3
"""Regenerates tests/programs/ll/*.ll (the optimized LLVM IR of each test
program, as produced by `scry-cc --emit-llvm`) so that the interpreter-based
tests can run without an LLVM installation.

Usage: python3 tests/programs/update_ll.py [file.c ...]
Environment: SCRY_CC (path to scry-cc, default target/debug/scry-cc).
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SCRY_CC = os.environ.get("SCRY_CC", os.path.join(ROOT, "target", "debug", "scry-cc"))


def main():
    files = sys.argv[1:] or sorted(
        os.path.join(HERE, f) for f in os.listdir(HERE) if f.endswith(".c") and f != "native_main.c"
    )
    os.makedirs(os.path.join(HERE, "ll"), exist_ok=True)
    for f in files:
        name = os.path.splitext(os.path.basename(f))[0]
        out = os.path.join(HERE, "ll", name + ".ll")
        subprocess.check_call([SCRY_CC, "--emit-llvm", "-o", out, f])
        # Drop the module identifier line (it embeds a temporary path) and
        # keep only the base name of the source file.
        with open(out) as fh:
            lines = []
            for l in fh.read().split("\n"):
                if l.startswith("; ModuleID"):
                    continue
                if l.startswith("source_filename = "):
                    l = f'source_filename = "{os.path.basename(f)}"'
                lines.append(l)
        with open(out, "w") as fh:
            fh.write("\n".join(lines))
        print("updated", os.path.relpath(out, ROOT))


if __name__ == "__main__":
    main()
