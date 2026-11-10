# scrymetrics

Static and dynamic instruction metrics for Scry programs, and the same for
RISC-V (RV32IMC and RV64IMC) programs run on qemu, for comparison.

```sh
cargo build --release --manifest-path tools/scrymetrics/Cargo.toml
tools/scrymetrics/target/release/scrymetrics static prog.elf
scryer prog.elf --print-events | tools/scrymetrics/target/release/scrymetrics dynamic
tools/scrymetrics/target/release/scrymetrics rv64-static prog-rv64.elf   # or rv32-static
qemu-riscv64 -d exec,in_asm,nochain prog-rv64.elf 2>&1 >/dev/null | tools/scrymetrics/target/release/scrymetrics rv64
qemu-riscv32 -d exec,in_asm,nochain prog-rv32.elf 2>&1 >/dev/null | tools/scrymetrics/target/release/scrymetrics rv32
```

`static` decodes the `.text` section (every Scry instruction is 16 bits) and
counts the instructions by mnemonic and category, telling two forms apart:
a *far* jump (target offset 0: the target comes from an operand materialized
by a `const`/`grow` chain) from a short one, and a *long* `echo` (one 10-bit
reference) from the short form with two 5-bit references. `dynamic` reads
the simulator's event trace from standard input, one step at a time without
storing it, and reports:

* the dynamic instruction mix by mnemonic and category, with the same two
  forms (`forms`: `jmp_far`, `jmp_short`, `echo_long`, `echo_short`);
* operand lifetimes: instructions from a value's production to its final
  consumption (mean, median, 90th and 99th percentile, maximum), how many
  values are produced and never consumed (`Discard`), and how many times a
  value is forwarded by `echo` on its way;
* occupancy: how many produced values are alive per step;
* control flow: instructions from a jump, call or return being issued to its
  trigger (zero when the compiler uses no trigger delay, as it does today),
  and the length of the straight-line runs between control instructions;
* memory traffic: loads and stores to data and to the stack, with bytes;
* the deepest stack extent seen;
* `timed_out`: whether the simulator stopped at its `--timeout` instead of
  running to the end (then `returned` is empty and the counts cover the
  instructions executed up to the limit).

`rv32-static` and `rv64-static` decode the `.text*` sections of a RISC-V
ELF file (16- and 32-bit encodings; the 64-bit variant knows `ld`, `sd`,
the `*w` operations and their compressed forms) and count the instructions
by mnemonic, category and size; data that a backend keeps in the code
(Cranelift's constant pools) decodes as `other`. `rv32` and `rv64` read the
log of `qemu-riscv32`/`qemu-riscv64 -d exec,in_asm,nochain`: the
`in_asm` blocks give the instructions of every translated block, the
`Trace` lines the order in which the blocks ran (`nochain` makes qemu report
each one), and nothing but the encodings is used. It reports the dynamic
mix, the lifetime of register values (from the instruction that writes a
register to the last one that reads it before it is overwritten; a value
that is never read counts as discarded), the number of live register values
after each instruction, branch and call counts, block lengths, loads and
stores (with those addressed from `sp` apart) and the stack depth, which it
follows through the constant adjustments of `sp` (`"known": false` when an
instruction set `sp` from another register, as a variable-length array
does).

The RISC-V categories are `alu`, `constant`, `move` (`mv`), `control`,
`memory`, `nop`, `system` (`ecall`, `fence`, CSR accesses) and `other`
(floating point and atomics, under `fload`, `fstore`, `fp` and `atomic`,
and undecodable words). `constant` is `lui`, `auipc` and `li`, plus the
`addi` that completes a two-instruction constant or address and the load
that follows an `auipc` (a constant-pool load, reported as `ld.pool` and
`memory.pool_loads`, and not counted as memory traffic), so the constant
and move categories can be set against Scry's constant and dataflow
instructions. `li` and `mv` are the `addi` forms with `x0` as the source or
0 as the immediate.

All commands print one JSON object; `benchmarks/embench/metrics.py` runs
them over the Embench executables and prints Markdown tables.

The categories are: `alu` (`add sub mul div and or xor sh eq lt gt isnar
narto`), `dataflow` (`echo dup pick`), `control` (`jmp call ret trap`),
`constant` (`const grow`), `memory` (`ld st saddr rsrv free`), `cast`,
`nop`. The per-mnemonic counts are in the JSON for any other grouping.
