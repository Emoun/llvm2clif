#!/bin/bash
# Compares the code size of the Scry executables built by run_embench.py with
# RISC-V builds of the same sources (rv32im and rv32imc, -O2, same headers and
# runtime), compiled to objects with clang and summed over their .text*
# sections. Usage: benchmarks/embench/codesize_rv32.sh [out-dir]
# Environment: SCRY_CLANG (default clang), SCRY_READELF (default llvm-readelf).
set -e
here=$(cd "$(dirname "$0")" && pwd); root=$(cd "$here/../.." && pwd)
out=${1:-$here/out}; repo=$here/embench-iot; rt=$root/runtime
clang=${SCRY_CLANG:-clang}; readelf=${SCRY_READELF:-llvm-readelf}
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

code_bytes() { # sum of the .text* sections of an ELF file
  "$readelf" -SW "$1" | python3 -c '
import sys, re
t = 0
for l in sys.stdin:
    m = re.match(r"\s*\[\s*\d+\]\s+(\.text\S*)\s+\S+\s+[0-9a-f]+\s+[0-9a-f]+\s+([0-9a-f]+)", l)
    if m: t += int(m.group(2), 16)
print(t)'
}

printf "%-16s %8s %8s %8s %8s %8s\n" benchmark rv32im rv32imc scry scry/im scry/imc
tim=0; timc=0; tsc=0
for elf in "$out"/*/*.elf; do
  b=$(basename "$elf" .elf); [ -d "$repo/src/$b" ] || continue
  for arch in rv32im rv32imc; do
    total=0
    for c in "$repo"/src/"$b"/*.c "$repo"/support/main.c "$repo"/support/beebsc.c "$here"/boardsupport.c "$rt"/scryrt.c; do
      o=$tmp/$b.$arch.$(basename "$c" .c).o
      "$clang" --target=riscv32-unknown-none-elf -march=$arch -mabi=ilp32 -O2 -fsigned-char -fno-builtin \
        -isystem "$rt/include" -I"$repo/support" -I"$here/include" -DWARMUP_HEAT=1 -DGLOBAL_SCALE_FACTOR=1 -DSCRY_HEAP_SIZE=8192 \
        -c -o "$o" "$c"
      total=$((total + $(code_bytes "$o")))
    done
    eval "c_$arch=$total"
  done
  scry=$(code_bytes "$elf")
  python3 -c "print(f'{\"$b\":16} {$c_rv32im:8} {$c_rv32imc:8} {$scry:8} {$scry/$c_rv32im:8.2f} {$scry/$c_rv32imc:8.2f}')"
  tim=$((tim + c_rv32im)); timc=$((timc + c_rv32imc)); tsc=$((tsc + scry))
done
python3 -c "print(f'{\"total\":16} {$tim:8} {$timc:8} {$tsc:8} {$tsc/$tim:8.2f} {$tsc/$timc:8.2f}')"
