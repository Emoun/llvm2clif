#!/usr/bin/env python3
"""Splits a .clif file into one file per function (preamble lines are kept in each)."""
import re, sys
from pathlib import Path
src, outdir = Path(sys.argv[1]), Path(sys.argv[2])
outdir.mkdir(parents=True, exist_ok=True)
text = src.read_text()
parts = re.split(r"(?m)^(?=function )", text)
preamble, funcs = parts[0], parts[1:]
for f in funcs:
    name = re.match(r"function %(\S+?)\(", f).group(1)
    (outdir / f"{name}.clif").write_text(preamble + f)
print(f"{len(funcs)} functions -> {outdir}")
