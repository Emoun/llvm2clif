#!/usr/bin/env python3
"""Recomputes the `// CASES:` expectations of the test programs by compiling
them natively (with the host clang) and running each case.

Usage: python3 tests/programs/update_expected.py [file.c ...]
"""
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
CLANG = os.environ.get("CLANG", "clang")


def update(path):
    with open(path) as f:
        lines = f.read().split("\n")
    if not any(line.startswith("// CASES:") for line in lines):
        return False
    with tempfile.TemporaryDirectory() as tmp:
        exe = os.path.join(tmp, "native")
        subprocess.check_call([CLANG, "-O1", "-w", "-o", exe, path, os.path.join(HERE, "native_main.c")])
        out = []
        for line in lines:
            m = re.match(r"^// CASES:\s*(.*?)\s*(=>.*)?$", line)
            if not m:
                out.append(line)
                continue
            args = m.group(1).split()
            result = subprocess.check_output([exe] + args).decode().strip()
            out.append(f"// CASES: {' '.join(args)} => {result}")
    with open(path, "w") as f:
        f.write("\n".join(out))
    return True


def main():
    files = sys.argv[1:] or sorted(
        os.path.join(HERE, f) for f in os.listdir(HERE) if f.endswith(".c") and f != "native_main.c"
    )
    for f in files:
        if update(f):
            print("updated", os.path.basename(f))


if __name__ == "__main__":
    main()
