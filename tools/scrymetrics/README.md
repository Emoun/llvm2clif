# scrymetrics

Static and dynamic instruction metrics for Scry programs.

```sh
cargo build --release --manifest-path tools/scrymetrics/Cargo.toml
tools/scrymetrics/target/release/scrymetrics static prog.elf
scryer prog.elf --print-events | tools/scrymetrics/target/release/scrymetrics dynamic
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

Both commands print one JSON object; `benchmarks/embench/metrics.py` runs
them over the Embench executables and prints Markdown tables.

The categories are: `alu` (`add sub mul div and or xor sh eq lt gt isnar
narto`), `dataflow` (`echo dup pick`), `control` (`jmp call ret trap`),
`constant` (`const grow`), `memory` (`ld st saddr rsrv free`), `cast`,
`nop`. The per-mnemonic counts are in the JSON for any other grouping.
