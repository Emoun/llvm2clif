# Embench-IoT on Scry

[Embench-IoT](https://github.com/embench/embench-iot) is a suite of 19
embedded benchmark programs. `run_embench.py` builds each of them with
`scry-cc` and runs it on the `scryer` simulator; a benchmark passes when its
own `verify_benchmark` accepts the result (`main` returns 0).

```sh
cargo build --release
python3 benchmarks/embench/run_embench.py            # all benchmarks, 2 at a time
python3 benchmarks/embench/run_embench.py --only crc32,edn --jobs 1
python3 benchmarks/embench/run_embench.py --reparse  # re-read the logs of the last run
```

The script clones Embench at the pinned commit `09c2ed8`
(`embench-0.5-100-g09c2ed8`) into `benchmarks/embench/embench-iot` on first
use; `--repo` points it at an existing checkout. Each benchmark is compiled
from `src/<name>/*.c`, `support/main.c`, `support/beebsc.c` and the board
support in this directory (`boardsupport.c`: empty `initialise_board` and
trigger functions) with `-DWARMUP_HEAT=1 -DGLOBAL_SCALE_FACTOR=1` and
`scry-cc`'s default `-O2`. `include/math.h` is a stub for the two sources
that include it. Logs and ELF files go to `benchmarks/embench/out/<name>/`,
the summary to `out/results.json`.

The simulator is slow (roughly half a million instructions per second), so
a full run takes about half an hour of simulation plus the compile time of
the two slow benchmarks; `--compile-timeout` and `--run-timeout` (seconds)
cap each step.

## Results

With rust-wasmtime `f9fa226`, scryer `b1737e6` and wild `2b9f2b4`:

| benchmark | result | notes |
|-----------|--------|-------|
| crc32 | pass | 16.5 M instructions |
| depthconv | pass | 18.2 M |
| edn | pass | 25.9 M; was stopped by backend issue 11 before `f9fa226` |
| matmult-int | pass | 12.4 M |
| md5sum | pass | 18.1 M |
| nettle-aes | pass | 18.7 M |
| qrduino | pass | 21.0 M |
| sglib-combined | pass | 21.0 M |
| slre | pass | 23.7 M; needs `memchr`, which LLVM makes out of `strchr` |
| statemate | pass | 10.1 M |
| tarfind | pass | 15.0 M |
| ud | pass | 17.4 M |
| xgboost | pass | 35.0 M |
| nsichneu | pass | 16.7 M; needed the stack moved up before scryer `8059eee` (its image is 68 KiB) |
| picojpeg | pass | 15.4 M; needed the stack moved up before scryer `8059eee` (73 KiB image), and was stopped by backend issues 12 and 13 before `f9fa226` |
| huffbench | pass | 25.0 M; needs the 1 MiB stack pool the runner asks for (one of its reservations exhausts scryer's default 4 KiB) |
| aha-mont64 | backend bug | [issue 14](../../docs/backend-issues/README.md): the inlined loop does not compile in 30 minutes; passes with `--cflags=-fno-inline-functions` (67.3 M, compiles in 0.5 s) |
| nettle-sha256 | compile time | one 837-instruction block (the rounds are unrolled by hand, so no flag helps: the block is the same size at `-O1` and `-Oz`); see below |
| wikisort | unsupported | calls `sqrt` on a `double`; no floating point on Scry |

Instruction counts are the `InstructionReads` metric of scryer `6fc6ac9`
for one iteration (`GLOBAL_SCALE_FACTOR=1`, including one warm-up run);
they vary slightly between backend revisions and compiles, since code
generation is not deterministic. scryer `8e73d4a` and later print no metrics,
so the runner's `instr` column is 0 with them. The runner starts the
simulator with `--stack-buffer 1048576` (a 1 MiB stack pool, which
huffbench needs); `--sim-args` adds or overrides simulator options, and
since `b1737e6` a run that prints no events needs no memory to speak of.

**Simulator limits.** scryer places the stack at `0x100000` by default
(`0x10000` before `8059eee`) with a 4 KiB pool for stack growth, and a
program image must fit below the stack. Since `459c71b` both are options,
`--stack-base` and `--stack-buffer`; the runner sets the pool to 1 MiB,
which huffbench needs. Nothing in the generated code depends on the stack
address.

**Compile time.** The backend's compile time grows cubically with the size
of a basic block (see the open issue in `docs/backend-issues`), and the two
benchmarks with very long straight-line blocks did not finish compiling
within two hours (`nettle-sha256` still does not within the sweep's
30-minute cap on `f9fa226`); the others compile in under 10 seconds. `aha-mont64`'s block is
the benchmark loop with everything inlined, and what makes it slow is not
its length but the loop nesting (issue 14, with a 30-instruction
reproducer); built with `--cflags=-fno-inline-functions` (the `=` form,
because the value starts with a dash) it compiles in under a second and
passes.

## Metrics

Beyond pass or fail, `metrics.py` measures what the compiler emits and what
the programs do while they run. It is built on `tools/scrymetrics`, a small
Rust tool with two passes: `static` decodes the `.text` section of an ELF
file with the `scry_isa` crate, and `dynamic` reads the event stream of
`scryer --print-events` from standard input, one step at a time (nothing is
stored; a 20 M-instruction benchmark streams about 2 GB through the pipe).
With `--rv64` and `--rv32` it also builds every benchmark for RISC-V, runs
it on qemu and measures it the same way, for comparison: rv64imc through
`llvm2clif` and Cranelift's riscv64 backend (the same translation as for
Scry), and rv32imc through LLVM's own backend.

```sh
cargo build --release && cargo build --release --manifest-path tools/scrymetrics/Cargo.toml
python3 benchmarks/embench/metrics.py                 # every out/<name>/<name>.elf, 3 at a time
python3 benchmarks/embench/metrics.py --tables-only   # re-print the tables from out/*/*.json
python3 benchmarks/embench/metrics.py --only crc32 --sim-args "--timeout 1000000 --timeout-type instructions"
SCRY_CLANG=clang-18 SCRY_OPT=opt-18 SCRY_LLC=llc-18 SCRY_LLD=ld.lld-18 \
    python3 benchmarks/embench/metrics.py --rv64 --rv32 --skip-scry   # the RISC-V sides only (needs qemu-user)
python3 benchmarks/embench/metrics.py --scry-speed --skip-scry   # Scry with Cranelift's optimizer on (needs wild)
```

Results go to `out/<name>/metrics.json`, `metrics_speed.json`, `rv64cl.json`
and `rv32imc.json`; the tables below are what the script prints. Streaming events costs the simulator most of its speed
(about 80 k instructions per second, against several hundred thousand
without), so the metrics run over the 17 benchmarks takes about half an
hour with three jobs. `--sim-args` passes options on to the
simulator; a run cut short by `--timeout N --timeout-type instructions` is
marked with `*` in the tables.

**Categories.** Every instruction is counted in one category:

| category | mnemonics | what it does |
|---|---|---|
| alu | `add sub mul div and or xor sh eq lt gt isnar narto` | computes a value |
| dataflow | `echo dup pick` | moves a value to an instruction its producer cannot reach directly, or to a second consumer |
| control | `jmp call ret trap` | control flow |
| constant | `const grow` | builds immediates and addresses 8 bits at a time (`const` starts a value, each `grow` adds 8 bits) |
| memory | `ld st saddr rsrv free` | memory access and stack management |
| cast | `cast` | re-tags a value with another type |
| nop | `nop` | padding |

Two instruction forms are told apart. A jump whose target offset is 0 takes
its target from an operand (a *far* jump: the target is built by the
`const`/`grow` chain in front of it); the other jumps carry the target in
their 7-bit offset. An `echo` whose reference does not fit the short
form's 5 bits is a *long* echo (one 10-bit reference; the simulator prints
both forms as `echo`).

**Lifetimes.** The simulator gives every produced value an id and reports
when it is forwarded by an `echo` or `dup`, consumed, or discarded. The
*lifetime* of an operand is the number of instructions from its production
to its final consumption (or discard), echo hops included; *forwards per
operand* is the number of hops. *Live operands* is the number of
produced-but-not-yet-consumed values after each instruction. For control
flow, the lifetime is the number of instructions between a jump, call or
return being issued and its trigger.

The tables cover the 16 benchmarks that pass and aha-mont64 in its
`-fno-inline-functions` build; the executables are those of the results
above (rust-wasmtime `f9fa226`, scryer `b1737e6`, wild `2b9f2b4`).

### Code size and static instruction mix (Scry)

| benchmark | code bytes | instructions | alu % | dataflow % | control % | constant % | memory % | cast % | nop % | other % | long echoes % of echoes | far jumps % of jumps | grow per const |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 4048 | 2024 |  15.2 |  51.5 |   3.1 |  19.5 |   7.9 |   1.3 |   1.5 |   0.0 |  56.1 |  87.5 | 0.63 |
| crc32 | 672 | 336 |   9.5 |  34.8 |  11.9 |  36.3 |   2.4 |   0.6 |   4.5 |   0.0 |  38.3 |  50.0 | 1.18 |
| depthconv | 1068 | 534 |  20.6 |  30.5 |   7.7 |  30.3 |   3.6 |   4.3 |   3.0 |   0.0 |  75.5 |   8.3 | 0.69 |
| edn | 4264 | 2132 |  16.7 |  45.9 |   3.3 |  25.4 |   3.5 |   2.7 |   2.4 |   0.0 |  52.5 |  44.7 | 0.59 |
| huffbench | 7228 | 3614 |  12.4 |  52.5 |   4.5 |  21.5 |   4.3 |   1.0 |   3.8 |   0.0 |  30.4 |  71.1 | 0.54 |
| matmult-int | 2860 | 1430 |  17.1 |  39.9 |   4.2 |  31.9 |   3.6 |   0.3 |   3.0 |   0.0 |  55.8 |  26.7 | 0.94 |
| md5sum | 2284 | 1142 |  11.3 |  41.2 |   7.4 |  32.0 |   3.7 |   0.5 |   3.9 |   0.0 |  32.2 |  42.9 | 1.23 |
| nettle-aes | 8728 | 4364 |  23.4 |  40.0 |   2.1 |  25.2 |   5.2 |   2.5 |   1.5 |   0.0 |  62.3 |  68.5 | 0.26 |
| nsichneu | 66736 | 33368 |   9.2 |  32.1 |   3.9 |  46.7 |   7.3 |   0.0 |   0.8 |   0.0 |  36.9 |  69.3 | 1.82 |
| picojpeg | 66556 | 33278 |   9.4 |  46.3 |   2.9 |  31.1 |   4.2 |   3.0 |   3.0 |   0.0 |  35.2 |  87.1 | 1.01 |
| qrduino | 28024 | 14012 |  14.7 |  43.0 |   3.4 |  28.5 |   4.5 |   3.1 |   2.8 |   0.0 |  33.0 |  82.5 | 0.75 |
| sglib-combined | 15644 | 7822 |   8.4 |  59.9 |   4.7 |  19.0 |   3.9 |   0.4 |   3.8 |   0.0 |  21.6 |  75.9 | 0.57 |
| slre | 19680 | 9840 |   6.8 |  66.6 |   3.4 |  15.8 |   2.5 |   0.8 |   4.0 |   0.0 |  27.8 |  87.7 | 0.74 |
| statemate | 12292 | 6146 |   6.8 |  18.2 |   4.9 |  58.0 |  10.3 |   0.3 |   1.6 |   0.0 |  62.7 |  82.7 | 1.34 |
| tarfind | 3588 | 1794 |   8.5 |  57.3 |   6.1 |  20.5 |   2.1 |   0.6 |   4.8 |   0.0 |  15.6 |  68.1 | 0.68 |
| ud | 3136 | 1568 |  11.8 |  38.8 |   4.8 |  33.7 |   6.4 |   1.2 |   3.4 |   0.0 |  44.3 |  52.3 | 0.93 |
| xgboost | 1308 | 654 |  11.3 |  48.9 |   6.4 |  21.7 |   7.5 |   1.7 |   2.4 |   0.0 |  35.4 |  42.9 | 0.80 |

### Dynamic instruction mix (Scry)

| benchmark | instructions | alu % | dataflow % | control % | constant % | memory % | cast % | nop % | other % | echo | of which long | dup | const+grow | cast | far jumps % of jumps | 8/16-bit operands % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 66837654 |  25.4 |  53.2 |   0.7 |  18.4 |   0.2 |   2.0 |   0.0 |   0.0 |  36.9 |  38.3 |  13.6 |  18.4 |   2.0 |  61.4 |  14.0 |
| crc32 | 15950330 |  19.8 |  30.8 |   4.4 |  41.7 |   3.3 |   0.0 |   0.0 |   0.0 |  27.5 |  12.0 |   3.3 |  41.7 |   0.0 |   0.1 |   3.3 |
| depthconv | 18134577 |  27.2 |  36.2 |   0.6 |  26.7 |   3.2 |   6.1 |   0.0 |   0.0 |  21.7 |  29.3 |  12.5 |  26.7 |   6.1 |  47.6 |   8.8 |
| edn | 25880475 |  20.1 |  47.3 |   2.9 |  22.4 |   3.8 |   3.4 |   0.1 |   0.0 |  39.4 |  14.9 |   7.9 |  22.4 |   3.4 |  23.5 |  15.0 |
| huffbench | 25154520 |  15.9 |  53.4 |   3.6 |  20.4 |   4.9 |   0.6 |   1.3 |   0.0 |  45.6 |   7.3 |   7.5 |  20.4 |   0.6 |  67.9 |  26.4 |
| matmult-int | 12214699 |  22.3 |  39.7 |   2.4 |  30.5 |   5.0 |   0.0 |   0.1 |   0.0 |  30.7 |  35.1 |   9.0 |  30.5 |   0.0 |  10.9 |   6.4 |
| md5sum | 17861357 |  17.6 |  52.9 |   4.2 |  22.0 |   2.7 |   0.4 |   0.1 |   0.0 |  43.7 |  12.7 |   9.2 |  22.0 |   0.4 |  54.2 |  22.2 |
| nettle-aes | 18691343 |  30.1 |  37.8 |   0.6 |  25.9 |   4.6 |   0.7 |   0.2 |   0.0 |  29.0 |  42.4 |   8.8 |  25.9 |   0.7 |  66.9 |   3.8 |
| nsichneu | 16681595 |   9.3 |  35.0 |   5.6 |  41.8 |   7.4 |   0.0 |   0.9 |   0.0 |  33.1 |   0.0 |   1.9 |  41.8 |   0.0 |  50.5 |  20.0 |
| picojpeg | 15514899 |  18.7 |  41.4 |   2.7 |  26.8 |   5.1 |   4.4 |   0.8 |   0.0 |  30.0 |  13.6 |  10.5 |  26.8 |   4.4 |  77.0 |  56.6 |
| qrduino | 21502235 |  15.0 |  53.6 |   3.2 |  21.9 |   3.1 |   2.3 |   0.9 |   0.0 |  44.9 |   8.8 |   8.0 |  21.9 |   2.3 |  85.4 |  46.0 |
| sglib-combined | 20901068 |  11.5 |  61.1 |   4.0 |  16.8 |   4.4 |   0.5 |   1.6 |   0.0 |  52.9 |   8.3 |   7.8 |  16.8 |   0.5 |  78.4 |  36.3 |
| slre | 23862386 |   8.1 |  69.4 |   3.6 |  13.8 |   1.4 |   1.2 |   2.4 |   0.0 |  62.8 |   7.9 |   6.3 |  13.8 |   1.2 |  85.5 |  49.2 |
| statemate | 10091747 |   5.1 |  13.8 |   3.3 |  63.5 |  13.6 |   0.1 |   0.6 |   0.0 |   9.8 |  21.3 |   3.3 |  63.5 |   0.1 |  68.2 |  25.3 |
| tarfind | 15431905 |  16.8 |  45.5 |   6.7 |  25.9 |   3.6 |   1.4 |   0.2 |   0.0 |  34.4 |   6.3 |  11.1 |  25.9 |   1.4 |   6.9 |  23.2 |
| ud | 17481193 |  19.2 |  48.0 |   2.6 |  24.6 |   3.6 |   0.8 |   1.3 |   0.0 |  39.3 |  13.1 |   8.7 |  24.6 |   0.8 |  61.7 |  15.6 |
| xgboost | 35299960 |  12.2 |  58.1 |   2.4 |  18.3 |   4.8 |   3.7 |   0.6 |   0.0 |  47.6 |  13.7 |   9.4 |  18.3 |   3.7 |  43.7 |  32.9 |

### Dynamic instruction mix (Scry, Cranelift's optimizer on)

| benchmark | instructions | alu % | dataflow % | control % | constant % | memory % | cast % | nop % | other % | echo | of which long | dup | const+grow | cast | far jumps % of jumps | 8/16-bit operands % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 33588868 |  27.3 |  55.1 |   1.5 |  13.4 |   0.4 |   2.3 |   0.1 |   0.0 |  35.7 |  40.8 |  18.3 |  13.4 |   2.3 |  61.4 |  17.6 |
| crc32 | 15601154 |  20.2 |  38.2 |   4.5 |  33.7 |   3.4 |   0.0 |   0.0 |   0.0 |  32.6 |  20.7 |   5.6 |  33.7 |   0.0 |   0.1 |   3.7 |
| depthconv | 15241713 |  26.2 |  49.4 |   0.7 |  13.0 |   3.8 |   6.9 |   0.0 |   0.0 |  26.6 |  51.7 |  20.7 |  13.0 |   6.9 |  47.6 |  10.3 |
| edn | 31596306 |  14.8 |  66.7 |   2.4 |   9.3 |   3.1 |   3.6 |   0.1 |   0.0 |  56.1 |  25.4 |  10.7 |   9.3 |   3.6 |  52.6 |  22.8 |
| huffbench | 24116910 |  14.8 |  61.6 |   3.8 |  14.9 |   2.9 |   0.6 |   1.4 |   0.0 |  49.8 |   4.1 |  11.6 |  14.9 |   0.6 |  68.8 |  30.5 |
| md5sum | 19350172 |  15.6 |  63.3 |   3.9 |  14.3 |   2.5 |   0.3 |   0.1 |   0.0 |  52.6 |  14.9 |  10.6 |  14.3 |   0.3 |  54.2 |  24.6 |
| nettle-aes | 19750659 |  28.7 |  52.2 |   0.6 |  13.2 |   4.3 |   0.7 |   0.2 |   0.0 |  34.2 |  70.2 |  18.0 |  13.2 |   0.7 |  67.4 |   3.8 |
| nsichneu | 22428594 |   6.9 |  69.8 |   4.1 |  12.9 |   5.5 |   0.0 |   0.7 |   0.0 |  62.9 |   0.1 |   6.9 |  12.9 |   0.0 |  93.2 |  38.8 |
| qrduino | 55896767 |   6.0 |  84.2 |   1.2 |   5.8 |   1.2 |   1.2 |   0.4 |   0.0 |  78.3 |  18.7 |   5.6 |   5.8 |   1.2 |  85.4 |  40.5 |
| slre | 20339754 |   9.4 |  67.1 |   4.2 |  13.5 |   1.6 |   1.5 |   2.6 |   0.0 |  59.0 |   5.5 |   7.8 |  13.5 |   1.5 |  85.5 |  53.6 |
| tarfind | 14660564 |  17.4 |  45.9 |   7.0 |  24.9 |   3.8 |   0.8 |   0.2 |   0.0 |  31.9 |   9.4 |  14.0 |  24.9 |   0.8 |  10.7 |  21.4 |
| xgboost | 41440575 |  11.2 |  68.9 |   2.0 |  10.1 |   4.1 |   3.1 |   0.5 |   0.0 |  57.5 |  19.7 |  10.5 |  10.1 |   3.1 |  87.4 |  41.5 |

### Operand lifetimes and queue occupancy (Scry)

| benchmark | operands | lifetime mean | median | p90 | p99 | max | discarded % | forwards per operand | live operands mean | max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 39782357 | 94.03 | 4 | 172 | 576 | 282437 |   4.6 | 1.19 | 55.83 | 79 |
| crc32 | 10688598 | 13.29 | 1 | 7 | 90 | 93285 |   0.0 | 0.82 | 8.92 | 12 |
| depthconv | 13658711 | 20.67 | 1 | 44 | 162 | 22107 |   2.7 | 0.54 | 15.56 | 30 |
| edn | 14780500 | 22.66 | 1 | 35 | 101 | 325027 |   0.2 | 1.5 | 13.08 | 85 |
| huffbench | 12110897 | 33.65 | 1 | 34 | 161 | 4188924 |   3.4 | 2.32 | 17.32 | 26 |
| matmult-int | 8016427 | 53.86 | 1 | 26 | 885 | 302954 |   0.3 | 0.98 | 36.21 | 72 |
| md5sum | 9062597 | 37.81 | 1 | 33 | 221 | 503985 |   0.0 | 2.26 | 20.46 | 28 |
| nettle-aes | 13044221 | 54.65 | 1 | 155 | 555 | 485098 |   0.3 | 0.74 | 39.02 | 72 |
| nsichneu | 10066427 | 10.26 | 1 | 4 | 9 | 13534 |   3.1 | 1.33 | 6.28 | 9 |
| picojpeg | 9801217 | 24.51 | 2 | 40 | 161 | 2584471 |   3.2 | 1.07 | 16.08 | 43 |
| qrduino | 10739368 | 35.12 | 1 | 38 | 214 | 7167001 |   3.5 | 2.28 | 19.43 | 68 |
| sglib-combined | 8369791 | 44.56 | 2 | 34 | 173 | 652132 |   5.2 | 3.39 | 18.33 | 36 |
| slre | 7272549 | 136.84 | 2 | 75 | 2129 | 407846 |  10.2 | 5.46 | 65.16 | 90 |
| statemate | 7876006 | 12.62 | 1 | 5 | 113 | 6002 |   1.7 | 0.24 | 9.87 | 38 |
| tarfind | 8563480 | 26.79 | 1 | 24 | 116 | 328619 |   0.4 | 1.68 | 15.65 | 20 |
| ud | 9806272 | 28.47 | 1 | 42 | 231 | 19515 |   1.6 | 1.53 | 16.13 | 25 |
| xgboost | 17055402 | 35.58 | 1 | 81 | 412 | 17649923 |   3.4 | 2.37 | 16.98 | 22 |

### Control flow (Scry)

| benchmark | calls | jumps | returns | jump issue to trigger mean | p99 | max | block length mean | median | dataflow instr per block |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 9469 | 424756 | 9470 | 0.0 | 0 | 0 | 136.26 | 68 | 72.5 |
| crc32 | 175284 | 175619 | 175285 | 0.0 | 0 | 0 | 22.74 | 16 | 7.0 |
| depthconv | 1649 | 54188 | 1650 | 0.0 | 0 | 0 | 164.86 | 7 | 59.7 |
| edn | 256 | 447027 | 257 | 0.0 | 0 | 0 | 34.06 | 22 | 16.1 |
| huffbench | 154 | 606007 | 155 | 0.0 | 0 | 0 | 27.67 | 19 | 14.8 |
| matmult-int | 90 | 149043 | 91 | 0.0 | 0 | 0 | 41.31 | 22 | 16.4 |
| md5sum | 545 | 380428 | 546 | 0.0 | 0 | 0 | 23.54 | 19 | 12.5 |
| nettle-aes | 394 | 58932 | 395 | 0.0 | 0 | 0 | 158.05 | 34 | 59.8 |
| nsichneu | 9 | 773110 | 10 | 0.0 | 0 | 0 | 17.92 | 18 | 6.3 |
| picojpeg | 8316 | 227390 | 8317 | 0.1 | 0 | 36 | 36.66 | 27 | 15.2 |
| qrduino | 2657 | 485773 | 2658 | 0.0 | 0 | 28 | 31.66 | 23 | 17.0 |
| sglib-combined | 33579 | 452959 | 33580 | 0.0 | 0 | 0 | 24.97 | 20 | 15.3 |
| slre | 35109 | 328304 | 35110 | 0.9 | 8 | 8 | 28.08 | 23 | 19.5 |
| statemate | 26657 | 239963 | 26658 | 0.22 | 12 | 12 | 29.98 | 11 | 4.1 |
| tarfind | 37985 | 478928 | 37986 | 0.0 | 0 | 0 | 14.94 | 19 | 6.8 |
| ud | 1797 | 286004 | 1798 | 0.0 | 0 | 0 | 37.9 | 24 | 18.2 |
| xgboost | 265 | 633804 | 266 | 0.0 | 0 | 0 | 42.08 | 41 | 24.4 |

### Memory traffic and stack (Scry)

| benchmark | loads | stores | stack-indexed loads | stack-indexed stores | bytes loaded | bytes stored | loads+stores per 100 instr | max stack bytes |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 6 | 20815 | 46355 | 21759 | 185426 | 170287 | 0.1 | 128 |
| crc32 | 350208 | 175275 | 1 | 1 | 1400836 | 701104 | 3.3 | 16 |
| depthconv | 524864 | 52480 | 1 | 1 | 839748 | 52484 | 3.2 | 16 |
| edn | 860225 | 117588 | 1 | 1 | 1669822 | 211400 | 3.8 | 16 |
| huffbench | 432568 | 273756 | 1 | 1 | 1225040 | 531808 | 2.8 | 7696 |
| matmult-int | 467200 | 144801 | 1 | 1 | 1475204 | 195208 | 5.0 | 16 |
| md5sum | 273502 | 207499 | 1 | 1 | 893012 | 210719 | 2.7 | 16 |
| nettle-aes | 794817 | 57134 | 309 | 309 | 2898846 | 111500 | 4.6 | 32 |
| nsichneu | 1228085 | 3699 | 1 | 1 | 4912344 | 14800 | 7.4 | 16 |
| picojpeg | 414795 | 380832 | 13 | 1 | 643810 | 586684 | 5.1 | 48 |
| qrduino | 582389 | 85206 | 1 | 1 | 1054152 | 85630 | 3.1 | 16 |
| sglib-combined | 575688 | 199904 | 19459 | 19489 | 2069644 | 750660 | 3.9 | 1616 |
| slre | 191997 | 13221 | 29370 | 27730 | 421446 | 163804 | 1.1 | 2544 |
| statemate | 609720 | 759469 | 1 | 17 | 869551 | 1209222 | 13.6 | 80 |
| tarfind | 57289 | 498670 | 1 | 1 | 166286 | 607949 | 3.6 | 16 |
| ud | 416300 | 150108 | 1 | 1787 | 1664724 | 607340 | 3.3 | 416 |
| xgboost | 1676176 | 2560 | 1283 | 1539 | 1685660 | 11276 | 4.8 | 64 |

### Against RISC-V

Two RISC-V builds of every benchmark stand next to the Scry one, both from
the same optimized IR: each source goes through the same
`clang -O2 -S -emit-llvm -Xclang -disable-llvm-passes` and `opt -O2` steps
as in `scry-cc`, with the same runtime library, board support and flags,
and only the back end differs.

* **rv64imc through llvm2clif and Cranelift** (`rv64imc Cranelift` in the
  tables): the IR is produced for `riscv64-unknown-none-elf` (64-bit
  pointers, native `i64`) and translated by `llvm2clif --target riscv64`,
  the same translation as for Scry with a 64-bit word, then compiled by
  Cranelift's riscv64 backend with the compressed encodings enabled and,
  as for Scry, Cranelift's optimizer off (the IR arrives optimized). The
  backend insists on the G extensions being enabled, but the programs use
  no floating point or atomics, so the code it emits is RV64IMC; it is
  checked by the decoder, which counts floating-point and atomic
  instructions apart (none run). Functions and globals are addressed
  PC-relatively; 64-bit constants that do not fit an immediate come from a
  constant pool in the code (`auipc` and a load), which the static count
  shows as `other`. The same build with Cranelift's optimizer on
  (`opt_level=speed`) is given as a further data point in the first table.
  This is the comparison that isolates the ISA: the same compiler front
  end, the same translation and the same compiler infrastructure on both
  sides.
* **rv32imc through LLVM** (`rv32imc llc` in the tables): the IR for
  `riscv32-unknown-none-elf` compiled with `llc` (and `rv32im` too, for its
  code size), a conventional compiler's best effort for the ISA, which
  shows how much of the distance is the back end rather than the ISA.

Both are linked with `ld.lld` (relaxation on for the LLVM build; Cranelift
emits no relaxation relocations) and a four-instruction `_start`
(`riscv_crt0.S`), and run on qemu 8.2 in user mode (`qemu-riscv64`,
`qemu-riscv32`) with the execution log of `-d exec,in_asm,nochain` piped
into `scrymetrics rv64` or `rv32`, which decodes the instructions itself
and derives the register data flow from the decoded operands: a value lives
from the instruction that writes a register to the last one that reads it
before the register is written again, and the live values after an
instruction are those still to be read. In the RISC-V mix, `constant` is
`lui`, `auipc` and `li`, plus the `addi` that completes a two-instruction
constant or address and the constant-pool load that follows an `auipc`;
`move` is `mv`; everything else that is not control flow or memory is
`alu`. Every benchmark returns 0 from `main` on qemu, so all the RISC-V
builds pass their verification too, and the instruction counts were checked
against runs with one instruction per translated block. The code bytes are
the `.text` sections of the linked executables.

### RISC-V dynamic instruction mix (rv64imc Cranelift / rv64imc Cranelift, optimizer on / rv32imc llc)

| benchmark | alu % | constant % | move % | control % | memory % | other % | compressed % | branches taken % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 58.2 / 59.8 / 90.6 | 15.5 / 8.9 / 0.2 | 9.6 / 11.3 / 0.9 | 14.8 / 17.9 / 7.2 | 2.0 / 2.0 / 1.1 | 0.0 / 0.0 / 0.0 | 43.3 / 40.9 / 24.7 | 45.7 / 24.4 / 47.1 |
| crc32 | 41.7 / 48.6 / 52.2 | 41.7 / 28.6 / 26.1 | 0.0 / 2.9 / 0.0 | 8.3 / 11.4 / 13.1 | 8.3 / 8.6 / 8.7 | 0.0 / 0.0 / 0.0 | 36.1 / 42.9 / 56.5 | 99.6 / 0.1 / 99.6 |
| depthconv | 50.6 / 65.2 / 63.9 | 23.8 / 7.8 / 4.6 | 3.3 / 4.4 / 0.0 | 4.9 / 5.9 / 9.9 | 17.4 / 16.8 / 21.6 | 0.0 / 0.0 / 0.0 | 33.5 / 20.1 / 22.2 | 97.9 / 67.0 / 26.4 |
| edn | 63.1 / 66.2 / 61.6 | 16.9 / 5.3 / 0.4 | 2.4 / 6.0 / 2.4 | 6.1 / 9.7 / 10.0 | 11.5 / 12.8 / 25.7 | 0.0 / 0.0 / 0.0 | 45.2 / 46.4 / 38.0 | 62.4 / 2.8 / 97.3 |
| huffbench | 42.3 / 45.0 / 45.9 | 19.2 / 5.0 / 0.9 | 8.3 / 10.0 / 7.3 | 17.6 / 24.1 / 22.2 | 12.5 / 15.9 / 23.6 | 0.0 / 0.0 / 0.0 | 54.1 / 44.0 / 53.0 | 22.2 / 30.7 / 60.0 |
| matmult-int | 39.9 / 37.8 / 49.4 | 17.3 / 12.1 / 0.1 | 9.0 / 6.5 / 6.9 | 9.6 / 13.1 / 8.0 | 24.2 / 30.5 / 35.6 | 0.0 / 0.0 / 0.0 | 43.8 / 56.1 / 35.6 | 3.2 / 1.5 / 98.1 |
| md5sum | 41.6 / 42.0 / 52.6 | 15.3 / 13.0 / 0.2 | 15.5 / 16.3 / 17.6 | 16.8 / 17.6 / 14.8 | 10.7 / 11.2 / 14.8 | 0.0 / 0.0 / 0.0 | 38.5 / 40.3 / 55.2 | 12.5 / 12.5 / 64.0 |
| nettle-aes | 75.3 / 74.4 / 73.2 | 9.2 / 8.1 / 0.1 | 1.3 / 0.0 / 0.3 | 1.8 / 1.8 / 2.2 | 12.4 / 15.7 / 24.2 | 0.0 / 0.0 / 0.0 | 43.0 / 55.3 / 35.1 | 86.7 / 81.3 / 58.0 |
| nsichneu | 11.6 / 21.0 / 0.1 | 51.2 / 11.5 / 0.0 | 0.0 / 0.0 / 0.1 | 14.5 / 26.2 / 38.5 | 22.7 / 41.3 / 61.3 | 0.0 / 0.0 / 0.0 | 37.4 / 29.4 / 35.6 | 20.7 / 21.6 / 21.1 |
| picojpeg | 40.8 / 55.2 / 48.3 | 11.2 / 5.7 / 11.2 | 4.3 / 3.8 / 3.9 | 33.3 / 20.7 / 10.7 | 10.4 / 14.6 / 25.9 | 0.0 / 0.0 / 0.0 | 49.0 / 39.7 / 39.1 | 43.3 / 51.1 / 72.0 |
| qrduino | 55.3 / 64.0 / 64.0 | 15.5 / 3.0 / 0.9 | 3.6 / 4.5 / 1.6 | 14.0 / 15.5 / 15.0 | 11.5 / 13.0 / 18.5 | 0.0 / 0.0 / 0.0 | 31.6 / 32.7 / 39.4 | 46.7 / 51.2 / 39.1 |
| sglib-combined | 41.2 / 41.7 / 29.8 | 11.6 / 7.8 / 6.0 | 5.4 / 5.7 / 6.1 | 18.1 / 19.5 / 24.6 | 23.6 / 25.4 / 33.5 | 0.0 / 0.0 / 0.0 | 46.5 / 49.0 / 60.7 | 39.9 / 40.6 / 43.7 |
| slre | 41.2 / 41.4 / 20.7 | 17.3 / 14.5 / 17.4 | 5.3 / 4.9 / 5.2 | 16.2 / 17.0 / 23.4 | 20.1 / 22.1 / 33.4 | 0.0 / 0.0 / 0.0 | 43.2 / 44.1 / 53.7 | 49.0 / 51.2 / 45.4 |
| statemate | 8.1 / 8.6 / 6.8 | 56.7 / 21.7 / 37.1 | 1.2 / 1.0 / 6.4 | 6.1 / 6.6 / 13.4 | 27.9 / 62.1 / 36.3 | 0.0 / 0.0 / 0.0 | 11.0 / 33.5 / 15.0 | 57.1 / 64.3 / 83.0 |
| tarfind | 34.3 / 35.3 / 35.3 | 12.4 / 19.8 / 8.2 | 11.8 / 9.9 / 16.4 | 26.6 / 22.5 / 20.9 | 14.9 / 12.5 / 19.3 | 0.0 / 0.0 / 0.0 | 44.2 / 48.0 / 30.2 | 9.8 / 8.7 / 95.8 |
| ud | 47.9 / 51.9 / 52.2 | 29.3 / 14.3 / 2.8 | 2.4 / 2.6 / 9.5 | 8.1 / 11.0 / 12.5 | 12.4 / 20.3 / 23.0 | 0.0 / 0.0 / 0.0 | 32.1 / 32.8 / 53.1 | 60.4 / 60.9 / 51.0 |
| xgboost | 53.3 / 57.8 / 49.3 | 23.6 / 9.9 / 1.6 | 1.5 / 5.6 / 7.8 | 7.2 / 8.9 / 16.4 | 14.4 / 17.8 / 24.9 | 0.0 / 0.0 / 0.0 | 23.0 / 22.3 / 38.8 | 68.3 / 68.0 / 37.3 |

### All builds side by side

The tables below set the five builds against each other, one column each:
Scry as built by `scry-cc` (Cranelift's optimizer off, the configuration of
every other table), Scry with Cranelift's optimizer on
(`scry-cc --cranelift-opt-level speed`, built and measured by
`metrics.py --scry-speed`; a dash marks the five benchmarks whose build did
not finish compiling within 30 minutes), rv64imc through llvm2clif and
Cranelift without and with the optimizer, and rv32imc through LLVM. Where a
metric has a static counterpart it follows the dynamic one after a middle
dot. The Scry columns come from the same traces as the tables above; the
distances are in instructions.

### Instructions: executed / static

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 66837654 / 2024 | 33588868 / 1658 | 4544346 / 972 | 3735470 / 980 | 6495889 / 788 |
| crc32 | 15950330 / 336 | 15601154 / 350 | 6308852 / 769 | 6133581 / 799 | 4030049 / 493 |
| depthconv | 18134577 / 534 | 15241713 / 520 | 3323259 / 762 | 3639654 / 818 | 2721074 / 533 |
| edn | 25880475 / 2132 | 31596306 / 2284 | 8629078 / 1521 | 7843978 / 1574 | 3867234 / 1006 |
| huffbench | 25154520 / 3614 | 24116910 / 4002 | 6162414 / 1509 | 4829482 / 1402 | 2991816 / 992 |
| matmult-int | 12214699 / 1430 | - | 5717048 / 1258 | 4233602 / 1156 | 1876139 / 763 |
| md5sum | 17861357 / 1142 | 19350172 / 1236 | 4510404 / 907 | 4311284 / 926 | 3272668 / 642 |
| nettle-aes | 18691343 / 4364 | 19750659 / 4612 | 6910189 / 2302 | 7042836 / 2435 | 3537884 / 1434 |
| nsichneu | 16681595 / 33368 | 22428594 / 41328 | 5424206 / 10819 | 2979167 / 6055 | 2009931 / 4674 |
| picojpeg | 15514899 / 33278 | - | 8645819 / 12950 | 6705791 / 11804 | 3608659 / 6515 |
| qrduino | 21502235 / 14012 | 55896767 / 23230 | 6059164 / 5640 | 5488094 / 5303 | 3604362 / 3493 |
| sglib-combined | 20901068 / 7822 | - | 4404868 / 4724 | 4398356 / 4974 | 2882838 / 3174 |
| slre | 23862386 / 9840 | 20339754 / 9400 | 5582906 / 2946 | 5647631 / 2918 | 2937030 / 1537 |
| statemate | 10091747 / 6146 | - | 5090968 / 4235 | 5094263 / 4312 | 3485148 / 2047 |
| tarfind | 15431905 / 1794 | 14660564 / 1916 | 3733769 / 914 | 4439243 / 931 | 2702109 / 652 |
| ud | 17481193 / 1568 | - | 4842804 / 1157 | 3577199 / 1222 | 2581831 / 744 |
| xgboost | 35299960 / 654 | 41440575 / 686 | 11693931 / 935 | 9508055 / 946 | 6775071 / 618 |

### Instruction bytes: fetched (dynamic) / code size (static)

Scry instructions are 2 bytes; RISC-V ones 2 or 4.

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 133675308 / 4048 | 67177736 / 3316 | 14241742 / 2752 | 11883208 / 2742 | 22776406 / 2188 |
| crc32 | 31900660 / 672 | 31202308 / 700 | 20677012 / 2246 | 19276872 / 2276 | 11564596 / 1378 |
| depthconv | 36269154 / 1068 | 30483426 / 1040 | 11068794 / 2286 | 13098674 / 2396 | 9677094 / 1574 |
| edn | 51760950 / 4264 | 63192612 / 4568 | 26712894 / 4582 | 24094654 / 4580 | 12531454 / 2928 |
| huffbench | 50309040 / 7228 | 48233820 / 8004 | 17977030 / 4406 | 15071694 / 4134 | 8795158 / 2750 |
| matmult-int | 24429398 / 2860 | - | 17861624 / 3774 | 12187852 / 3284 | 6169022 / 2208 |
| md5sum | 35722714 / 2284 | 38700344 / 2472 | 14571326 / 2710 | 13765994 / 2692 | 9479868 / 1846 |
| nettle-aes | 37382686 / 8728 | 39501318 / 9224 | 21702802 / 7142 | 20380734 / 7108 | 11669364 / 4290 |
| nsichneu | 33363190 / 66736 | 44857188 / 82656 | 17642400 / 35126 | 10163022 / 19260 | 6609292 / 13828 |
| picojpeg | 31029798 / 66556 | - | 26117630 / 42854 | 21505238 / 36972 | 11611166 / 21316 |
| qrduino | 43004470 / 28024 | 111793534 / 46460 | 20413100 / 18430 | 18367320 / 16956 | 11576014 / 10852 |
| sglib-combined | 41802136 / 15644 | - | 13522812 / 13374 | 13285478 / 14188 | 8031618 / 8464 |
| slre | 47724772 / 19680 | 40679508 / 18800 | 17511258 / 8598 | 17614734 / 8548 | 8595050 / 4386 |
| statemate | 20183494 / 12292 | - | 19243730 / 14382 | 16965400 / 14172 | 12893746 / 6944 |
| tarfind | 30863810 / 3588 | 29321128 / 3832 | 11637336 / 2734 | 13493656 / 2764 | 9178458 / 1838 |
| ud | 34962386 / 3136 | - | 16257494 / 3470 | 11965608 / 3548 | 7582936 / 2172 |
| xgboost | 70599920 / 1308 | 82881150 / 1372 | 41393354 / 2726 | 33793220 / 2724 | 21840166 / 1788 |

### Value lifetimes: median / mean / p90 (instructions from production to the last consumption)

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 4 / 94.0 / 172 | 4 / 80.8 / 104 | 2 / 17.4 / 17 | 4 / 14.8 / 16 | 3 / 22.4 / 28 |
| crc32 | 1 / 13.3 / 7 | 1 / 19.6 / 17 | 1 / 9.5 / 4 | 1 / 11.3 / 11 | 2 / 11.6 / 21 |
| depthconv | 1 / 20.7 / 44 | 4 / 40.1 / 128 | 2 / 8.2 / 9 | 2 / 14.2 / 30 | 4 / 15.7 / 51 |
| edn | 1 / 22.7 / 35 | 3 / 50.5 / 79 | 1 / 15.0 / 17 | 2 / 21.4 / 16 | 5 / 21.6 / 12 |
| huffbench | 1 / 33.6 / 34 | 3 / 44.5 / 57 | 1 / 12.5 / 14 | 2 / 20.5 / 18 | 4 / 20.8 / 15 |
| matmult-int | 1 / 53.9 / 26 | - | 2 / 13.5 / 9 | 4 / 14.1 / 42 | 3 / 19.1 / 6 |
| md5sum | 1 / 37.8 / 33 | 3 / 60.9 / 54 | 2 / 16.7 / 10 | 2 / 23.6 / 29 | 4 / 28.3 / 33 |
| nettle-aes | 1 / 54.6 / 155 | 3 / 104.7 / 258 | 1 / 21.0 / 8 | 1 / 17.6 / 66 | 1 / 23.1 / 19 |
| nsichneu | 1 / 10.3 / 4 | 2 / 41.7 / 137 | 1 / 7.3 / 3 | 2 / 18.0 / 2 | 1 / 27.5 / 2 |
| picojpeg | 2 / 24.5 / 40 | - | 2 / 14.9 / 23 | 2 / 17.3 / 29 | 3 / 16.8 / 23 |
| qrduino | 1 / 35.1 / 38 | 3 / 177.6 / 232 | 1 / 15.1 / 14 | 1 / 21.0 / 14 | 2 / 22.3 / 18 |
| sglib-combined | 2 / 44.6 / 34 | - | 2 / 15.9 / 13 | 2 / 17.5 / 19 | 3 / 22.9 / 16 |
| slre | 2 / 136.8 / 75 | 3 / 138.5 / 98 | 2 / 12.9 / 33 | 2 / 13.8 / 37 | 2 / 15.6 / 40 |
| statemate | 1 / 12.6 / 5 | - | 1 / 6.5 / 4 | 1 / 10.5 / 26 | 3 / 20.0 / 39 |
| tarfind | 1 / 26.8 / 24 | 2 / 26.9 / 18 | 3 / 20.8 / 6 | 3 / 21.9 / 7 | 3 / 25.8 / 7 |
| ud | 1 / 28.5 / 42 | - | 1 / 12.1 / 18 | 1 / 14.6 / 22 | 5 / 28.7 / 38 |
| xgboost | 1 / 35.6 / 81 | 3 / 61.9 / 100 | 1 / 16.1 / 16 | 1 / 20.2 / 14 | 2 / 28.0 / 34 |

### First use of a value: dynamic median / mean · static median / mean

Dynamic: instructions from a value's production to its first forward or consumption (Scry) or to the first read of the register (RISC-V). Static: the output reference distance of every instruction (Scry) or the distance to the first reader of the register within the block (RISC-V).

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 1 / 5.3 · 3 / 15.8 | 1 / 8.8 · 3 / 16.0 | 2 / 6.8 · 1 / 1.6 | 2 / 7.0 · 1 / 1.5 | 2 / 11.2 · 1 / 2.1 |
| crc32 | 1 / 3.1 · 1 / 3.6 | 1 / 5.7 · 1 / 4.2 | 1 / 5.0 · 1 / 1.6 | 1 / 6.2 · 1 / 1.5 | 1 / 4.0 · 1 / 2.0 |
| depthconv | 1 / 6.2 · 1 / 8.2 | 1 / 8.2 · 1 / 12.2 | 2 / 4.4 · 1 / 1.7 | 2 / 6.2 · 1 / 2.3 | 3 / 5.5 · 1 / 2.3 |
| edn | 1 / 4.8 · 2 / 16.9 | 1 / 7.5 · 5 / 17.1 | 1 / 9.6 · 1 / 3.1 | 1 / 10.4 · 1 / 2.9 | 2 / 7.1 · 2 / 3.3 |
| huffbench | 1 / 4.4 · 3 / 6.6 | 1 / 6.4 · 5 / 8.7 | 1 / 4.1 · 1 / 1.5 | 2 / 5.5 · 1 / 1.5 | 2 / 5.4 · 1 / 1.8 |
| matmult-int | 1 / 2.8 · 1 / 20.6 | - | 2 / 4.0 · 1 / 2.5 | 3 / 10.4 · 1 / 4.0 | 3 / 4.4 · 2 / 4.8 |
| md5sum | 1 / 6.9 · 2 / 5.7 | 1 / 8.7 · 3 / 7.6 | 2 / 5.6 · 1 / 1.7 | 2 / 7.2 · 1 / 1.7 | 3 / 8.7 · 1 / 2.1 |
| nettle-aes | 1 / 3.7 · 1 / 21.3 | 1 / 3.4 · 2 / 30.0 | 1 / 8.1 · 1 / 2.2 | 1 / 10.3 · 1 / 6.6 | 1 / 7.4 · 1 / 3.6 |
| nsichneu | 1 / 1.5 · 1 / 5.6 | 1 / 3.9 · 8 / 11.1 | 1 / 2.7 · 1 / 1.8 | 2 / 3.7 · 1 / 1.4 | 1 / 4.6 · 1 / 1.3 |
| picojpeg | 1 / 4.8 · 2 / 8.4 | - | 2 / 9.7 · 1 / 1.9 | 1 / 7.3 · 1 / 1.6 | 2 / 7.8 · 1 / 2.2 |
| qrduino | 1 / 4.2 · 1 / 6.8 | 1 / 7.7 · 8 / 14.8 | 1 / 7.5 · 1 / 1.9 | 1 / 6.4 · 1 / 1.8 | 1 / 6.1 · 1 / 1.9 |
| sglib-combined | 1 / 4.7 · 4 / 7.0 | - | 1 / 8.4 · 1 / 1.5 | 1 / 7.5 · 1 / 1.5 | 1 / 4.5 · 1 / 1.8 |
| slre | 1 / 5.2 · 7 / 10.2 | 1 / 17.2 · 6 / 9.8 | 2 / 6.1 · 1 / 1.7 | 2 / 6.7 · 1 / 1.6 | 1 / 7.4 · 1 / 1.8 |
| statemate | 1 / 2.0 · 1 / 3.3 | - | 1 / 4.0 · 1 / 1.4 | 1 / 4.6 · 1 / 1.4 | 2 / 10.0 · 1 / 1.9 |
| tarfind | 1 / 10.0 · 3 / 5.6 | 1 / 9.5 · 4 / 6.2 | 3 / 8.3 · 1 / 1.6 | 2 / 11.0 · 1 / 1.5 | 2 / 6.7 · 1 / 1.9 |
| ud | 1 / 4.7 · 1 / 5.7 | - | 1 / 3.9 · 1 / 1.7 | 1 / 3.8 · 1 / 1.5 | 2 / 6.7 · 1 / 2.3 |
| xgboost | 1 / 5.4 · 1 / 4.8 | 1 / 12.0 · 2 / 9.1 | 1 / 5.2 · 1 / 1.7 | 1 / 5.6 · 1 / 1.7 | 1 / 6.6 · 1 / 1.9 |

### Load to consume: dynamic median / mean / p90 · static median / mean

Dynamic: instructions from a load to the consumption of its value (Scry: the consuming instruction; RISC-V: the first read of the loaded register). Static: the load's output reference (Scry) or the distance to the first reader in the block (RISC-V).

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 159 / 1764.5 / 383 · 1 / 1.0 | 40 / 913.9 / 166 · 1 / 1.0 | 11 / 201.3 / 48 · 2 / 2.0 | 13 / 89.7 / 80 · 2 / 2.0 | 10 / 314.1 / 40 · 2 / 2.8 |
| crc32 | 10 / 8.5 / 12 · 1 / 1.0 | 10 / 9.0 / 13 · 1 / 1.0 | 3 / 2.5 / 3 · 2 / 1.9 | 3 / 3.0 / 3 · 2 / 2.3 | 3 / 3.0 / 3 · 2 / 2.6 |
| depthconv | 1 / 13.8 / 7 · 1 / 1.0 | 121 / 114.0 / 149 · 1 / 1.0 | 2 / 3.1 / 5 · 2 / 2.2 | 26 / 23.7 / 37 · 3 / 5.8 | 4 / 4.7 / 9 · 3 / 2.8 |
| edn | 2 / 5.3 / 23 · 1 / 1.0 | 9 / 12.2 / 30 · 1 / 1.0 | 1 / 31.4 / 12 · 1 / 2.5 | 6 / 16.6 / 22 · 3 / 6.7 | 2 / 10.6 / 4 · 3 / 3.9 |
| huffbench | 3 / 21.5 / 10 · 1 / 1.0 | 3 / 19.2 / 10 · 1 / 1.0 | 2 / 7.8 / 5 · 2 / 2.3 | 2 / 5.8 / 4 · 2 / 2.1 | 3 / 4.2 / 4 · 2 / 2.6 |
| matmult-int | 1 / 16.7 / 4 · 1 / 1.0 | - | 2 / 1.9 / 2 · 2 / 4.0 | 1 / 16.6 / 65 · 1 / 9.0 | 4 / 4.7 / 5 · 3 / 10.7 |
| md5sum | 1 / 2.3 / 4 · 1 / 1.0 | 12 / 10.9 / 19 · 1 / 1.0 | 1 / 3.2 / 2 · 2 / 2.4 | 7 / 23.6 / 9 · 2 / 3.4 | 4 / 8.2 / 6 · 3 / 3.1 |
| nettle-aes | 1 / 6.4 / 21 · 1 / 1.0 | 116 / 121.3 / 217 · 1 / 1.0 | 1 / 31.8 / 8 · 1 / 2.0 | 53 / 59.8 / 121 · 4 / 31.4 | 8 / 11.8 / 16 · 4 / 5.0 |
| nsichneu | 2 / 3.7 / 9 · 1 / 1.0 | 2 / 2.3 / 5 · 1 / 1.0 | 2 / 2.4 / 4 · 1 / 2.0 | 2 / 1.6 / 2 · 1 / 1.5 | 1 / 1.4 / 2 · 1 / 1.3 |
| picojpeg | 2 / 9.4 / 29 · 1 / 1.0 | - | 4 / 28.9 / 8 · 1 / 2.3 | 4 / 24.7 / 14 · 1 / 2.1 | 3 / 16.8 / 11 · 3 / 3.4 |
| qrduino | 3 / 8.4 / 19 · 1 / 1.0 | 9 / 32.6 / 52 · 1 / 1.0 | 2 / 28.0 / 9 · 1 / 2.7 | 4 / 22.0 / 12 · 2 / 2.9 | 3 / 3.2 / 5 · 3 / 2.7 |
| sglib-combined | 3 / 7.6 / 15 · 1 / 1.0 | - | 2 / 16.5 / 8 · 1 / 1.9 | 2 / 14.1 / 10 · 1 / 1.9 | 2 / 6.0 / 5 · 1 / 1.9 |
| slre | 3 / 12.7 / 35 · 1 / 1.0 | 3 / 33.4 / 37 · 1 / 1.0 | 2 / 12.8 / 39 · 1 / 1.7 | 3 / 14.4 / 46 · 1 / 1.9 | 2 / 8.9 / 26 · 1 / 2.0 |
| statemate | 5 / 18.3 / 108 · 1 / 1.0 | - | 3 / 14.5 / 42 · 2 / 2.4 | 1 / 5.3 / 4 · 1 / 2.2 | 6 / 42.2 / 305 · 2 / 3.2 |
| tarfind | 5 / 3.9 / 5 · 1 / 1.0 | 5 / 3.9 / 5 · 1 / 1.0 | 3 / 2.5 / 3 · 2 / 1.7 | 3 / 153.8 / 3 · 2 / 2.1 | 2 / 2.3 / 3 · 2 / 2.4 |
| ud | 10 / 18.1 / 56 · 1 / 1.0 | - | 5 / 9.7 / 19 · 2 / 2.6 | 2 / 5.4 / 12 · 1 / 2.1 | 2 / 3.2 / 8 · 2 / 2.6 |
| xgboost | 3 / 21.9 / 5 · 1 / 1.0 | 3 / 25.9 / 15 · 1 / 1.0 | 5 / 10.9 / 9 · 2 / 2.1 | 3 / 7.9 / 6 · 2 / 2.6 | 3 / 6.1 / 4 · 2 / 2.2 |

### Live values: mean / max; never used %

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 55.8 / 79; 4.6 | 61.8 / 83; 1.9 | 17.3 / 22; 3.1 | 14.7 / 20; 3.8 | 22.7 / 28; 0.2 |
| crc32 | 8.9 / 12; 0.0 | 12.1 / 16; 0.0 | 10.7 / 13; 0.0 | 12.0 / 15; 0.0 | 11.1 / 13; 0.0 |
| depthconv | 15.6 / 30; 2.7 | 27.8 / 44; 3.0 | 9.7 / 12; 0.0 | 15.1 / 21; 0.1 | 14.9 / 19; 0.3 |
| edn | 13.1 / 85; 0.2 | 24.0 / 109; 0.4 | 19.7 / 27; 0.1 | 20.1 / 27; 0.2 | 21.1 / 30; 0.5 |
| huffbench | 17.3 / 26; 3.4 | 23.4 / 42; 4.0 | 12.8 / 23; 3.8 | 17.7 / 24; 5.3 | 17.2 / 23; 1.9 |
| matmult-int | 36.2 / 72; 0.3 | - | 13.0 / 27; 0.1 | 13.2 / 27; 0.1 | 17.6 / 27; 0.1 |
| md5sum | 20.5 / 28; 0.0 | 27.9 / 40; 0.1 | 14.8 / 19; 1.9 | 20.2 / 27; 2.1 | 23.2 / 30; 0.3 |
| nettle-aes | 39.0 / 72; 0.3 | 78.5 / 142; 0.3 | 22.5 / 26; 0.0 | 18.9 / 26; 0.1 | 23.3 / 29; 0.1 |
| nsichneu | 6.3 / 9; 3.1 | 18.0 / 22; 4.5 | 8.3 / 18; 0.0 | 16.3 / 21; 0.1 | 18.8 / 20; 0.0 |
| picojpeg | 16.1 / 43; 3.2 | - | 15.5 / 25; 3.4 | 16.1 / 27; 3.6 | 18.2 / 28; 1.4 |
| qrduino | 19.4 / 68; 3.5 | 39.7 / 104; 3.9 | 14.5 / 24; 2.2 | 18.8 / 27; 2.6 | 20.0 / 29; 0.6 |
| sglib-combined | 18.3 / 36; 5.2 | - | 14.2 / 22; 3.1 | 15.2 / 21; 3.6 | 16.1 / 24; 2.2 |
| slre | 65.2 / 90; 10.2 | 73.3 / 101; 9.6 | 13.7 / 23; 8.1 | 13.4 / 22; 7.8 | 14.9 / 24; 10.2 |
| statemate | 9.9 / 38; 1.7 | - | 7.0 / 18; 2.1 | 8.7 / 27; 3.8 | 14.7 / 21; 1.8 |
| tarfind | 15.7 / 20; 0.4 | 17.2 / 21; 0.2 | 14.7 / 17; 0.1 | 15.8 / 18; 0.2 | 17.3 / 19; 0.7 |
| ud | 16.1 / 25; 1.6 | - | 12.6 / 19; 0.6 | 13.9 / 21; 1.8 | 23.9 / 30; 6.6 |
| xgboost | 17.0 / 22; 3.4 | 24.4 / 35; 4.9 | 17.0 / 20; 3.4 | 21.7 / 27; 2.0 | 23.8 / 27; 2.9 |

### Jump issue to trigger (Scry): dynamic mean / p99 / max · static mean / max

The distance between a jump's issue and its trigger, in instructions; RISC-V has no such delay.

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| crc32 | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| depthconv | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| edn | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| huffbench | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| matmult-int | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - | - |
| md5sum | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| nettle-aes | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| nsichneu | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| picojpeg | 0.1 / 0 / 36 · 0.8 / 40 | - | - | - | - |
| qrduino | 0.0 / 0 / 28 · 0.3 / 30 | 0.0 / 0 / 28 · 0.3 / 30 | - | - | - |
| sglib-combined | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - | - |
| slre | 0.9 / 8 / 8 · 0.1 / 8 | 0.9 / 8 / 8 · 0.1 / 8 | - | - | - |
| statemate | 0.2 / 12 / 12 · 0.7 / 12 | - | - | - | - |
| tarfind | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |
| ud | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - | - |
| xgboost | 0.0 / 0 / 0 · 0.0 / 0 | 0.0 / 0 / 0 · 0.0 / 0 | - | - | - |

### Executed block length: mean / median; calls

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 136.3 / 68; 9469 | 68.5 / 18; 9469 | 6.8 / 8; 9470 | 5.6 / 5; 9470 | 13.9 / 9; 9468 |
| crc32 | 22.7 / 16; 175284 | 22.2 / 18; 175284 | 12.0 / 15; 175285 | 8.8 / 5; 175285 | 7.7 / 9; 175283 |
| depthconv | 164.9 / 7; 1649 | 138.6 / 10; 1649 | 20.2 / 7; 1650 | 16.9 / 5; 1650 | 10.1 / 3; 1649 |
| edn | 34.1 / 22; 256 | 41.6 / 24; 256 | 16.4 / 17; 257 | 10.3 / 7; 257 | 10.0 / 7; 256 |
| huffbench | 27.7 / 19; 154 | 26.5 / 20; 154 | 5.7 / 4; 155 | 4.2 / 4; 155 | 4.5 / 4; 154 |
| matmult-int | 41.3 / 22; 90 | - | 10.4 / 6; 91 | 7.6 / 5; 91 | 12.6 / 6; 90 |
| md5sum | 23.5 / 19; 545 | 25.5 / 20; 545 | 6.0 / 4; 546 | 5.7 / 5; 546 | 6.8 / 4; 477 |
| nettle-aes | 158.1 / 34; 394 | 167.0 / 35; 394 | 56.4 / 12; 395 | 56.3 / 10; 395 | 45.1 / 33; 394 |
| nsichneu | 17.9 / 18; 9 | 24.1 / 24; 9 | 6.9 / 7; 10 | 3.8 / 3; 10 | 2.6 / 3; 9 |
| picojpeg | 36.7 / 27; 8316 | - | 3.0 / 2; 8317 | 4.8 / 2; 8317 | 9.4 / 8; 8322 |
| qrduino | 31.7 / 23; 2657 | 82.3 / 43; 2657 | 7.1 / 4; 2658 | 6.5 / 5; 2658 | 6.7 / 5; 2651 |
| sglib-combined | 25.0 / 20; 33579 | - | 5.5 / 4; 33580 | 5.1 / 4; 33580 | 4.1 / 3; 33578 |
| slre | 28.1 / 23; 35109 | 23.9 / 19; 35109 | 6.2 / 4; 35110 | 5.9 / 4; 35110 | 4.3 / 3; 34640 |
| statemate | 30.0 / 11; 26657 | - | 16.4 / 5; 26658 | 15.1 / 5; 26658 | 7.5 / 4; 29989 |
| tarfind | 14.9 / 19; 37985 | 14.2 / 18; 37985 | 3.8 / 4; 37986 | 4.5 / 5; 37986 | 4.8 / 4; 37984 |
| ud | 37.9 / 24; 1797 | - | 12.4 / 8; 1798 | 9.1 / 8; 1798 | 8.0 / 7; 1796 |
| xgboost | 42.1 / 41; 265 | 49.4 / 52; 265 | 13.9 / 15; 266 | 11.2 / 12; 266 | 6.1 / 6; 264 |

### Memory: loads+stores / of which stack-relative / bytes

Stack-relative: Scry's stack-indexed `ld`/`st`, RISC-V's `sp`-relative loads and stores (spills and locals).

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 88935 / 68114 / 355713 | 88935 / 68114 / 355713 | 91837 / 81422 / 734625 | 75759 / 65344 / 606001 | 70073 / 49258 / 280283 |
| crc32 | 525485 / 2 / 2101940 | 525485 / 2 / 2101940 | 525529 / 46 / 4204224 | 525533 / 50 / 4204256 | 350415 / 36 / 1401660 |
| depthconv | 577346 / 2 / 892232 | 577346 / 2 / 892232 | 577374 / 30 / 892456 | 610174 / 32830 / 1154856 | 587196 / 9854 / 931638 |
| edn | 977815 / 2 / 1881222 | 977815 / 2 / 1881222 | 994241 / 14828 / 2117546 | 1004827 / 25414 / 2202234 | 993540 / 15730 / 1944124 |
| huffbench | 706326 / 2 / 1756848 | 706326 / 2 / 1756848 | 772300 / 96 / 3242912 | 768500 / 112 / 3212512 | 706544 / 256 / 1757720 |
| matmult-int | 612003 / 2 / 1670412 | - | 1385237 / 514036 / 7453084 | 1289557 / 418356 / 6687644 | 668557 / 56556 / 1896628 |
| md5sum | 481003 / 2 / 1103731 | 481003 / 2 / 1103731 | 482383 / 1382 / 1118255 | 484196 / 3195 / 1132759 | 482842 / 2050 / 1110283 |
| nettle-aes | 852569 / 618 / 3010346 | 852569 / 618 / 3010346 | 857687 / 5736 / 3053754 | 1108822 / 256871 / 5062834 | 856325 / 4374 / 3025370 |
| nsichneu | 1231786 / 2 / 4927144 | 1229320 / 2 / 4917280 | 1231850 / 66 / 8597120 | 1229384 / 66 / 8582324 | 1231787 / 18 / 4927148 |
| picojpeg | 795641 / 14 / 1230494 | - | 903171 / 107568 / 2091418 | 981111 / 185520 / 2715022 | 935628 / 159790 / 1773720 |
| qrduino | 667597 / 2 / 1139782 | 667597 / 2 / 1139782 | 698879 / 31122 / 2018556 | 713105 / 45348 / 2132364 | 665868 / 952 / 1132944 |
| sglib-combined | 814540 / 38948 / 2820304 | - | 1039566 / 262658 / 5927728 | 1117336 / 339460 / 6549888 | 964649 / 227812 / 3420740 |
| slre | 262318 / 57100 / 585250 | 262318 / 57100 / 585250 | 1119617 / 891233 / 7714614 | 1245767 / 1015862 / 8723814 | 979566 / 785931 / 3435990 |
| statemate | 1369207 / 18 / 2078773 | - | 1422535 / 79995 / 2851897 | 3161361 / 1818821 / 16762505 | 1266041 / 99999 / 1895804 |
| tarfind | 555961 / 2 / 774235 | 555961 / 2 / 774235 | 556017 / 58 / 1065707 | 556397 / 438 / 1068747 | 520213 / 585 / 631243 |
| ud | 568196 / 1788 / 2272064 | - | 600632 / 33988 / 4794536 | 727470 / 160826 / 5809240 | 595046 / 30430 / 2379464 |
| xgboost | 1681558 / 2822 / 1696936 | 1681044 / 2308 / 1694880 | 1685174 / 8742 / 1735096 | 1688508 / 12076 / 1761768 | 1686200 / 9768 / 1715504 |

### Stack: max depth / bytes reserved / reservations · static frame size mean / max

Dynamic: the deepest stack extent, the bytes reserved over the run (`rsrv` on Scry, constant decrements of `sp` on RISC-V) and how often. Static: the sizes of those reservations in the code.

| benchmark | Scry | Scry, optimizer on | rv64imc Cranelift | rv64imc Cranelift, optimizer on | rv32imc llc |
|---|---:|---:|---:|---:|---:|
| aha-mont64 | 128 / 196880 / 18925 · 11.8 / 32 | 128 / 196880 / 18925 · 11.8 / 32 | 320 / 371216 / 18928 · 24.4 / 144 | 352 / 265392 / 9468 · 26.2 / 208 | 208 / 98656 / 1895 · 42.7 / 128 |
| crc32 | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 112 / 192 / 8 · 18.3 / 48 | 128 / 224 / 8 · 19.6 / 64 | 48 / 80 / 3 · 20.0 / 32 |
| depthconv | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 80 / 128 / 8 · 16.0 / 16 | 160 / 131328 / 3288 · 18.9 / 64 | 64 / 26304 / 1644 · 16.0 / 16 |
| edn | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 320 / 14736 / 175 · 25.9 / 160 | 416 / 18768 / 175 · 26.7 / 208 | 192 / 6768 / 87 · 34.3 / 80 |
| huffbench | 7696 / 92176 / 49 · 1539.2 / 4096 | 7696 / 92176 / 49 · 1539.2 / 4096 | ? / 400 / 23 · 17.1 / 32 | ? / 464 / 23 · 19.4 / 64 | ? / 3184 / 17 · 58.7 / 256 |
| matmult-int | 16 / 16 / 1 · 16.0 / 16 | - | 336 / 688 / 11 · 31.1 / 272 | 400 / 816 / 11 · 29.4 / 336 | 144 / 272 / 5 · 29.7 / 112 |
| md5sum | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 208 / 6624 / 276 · 20.6 / 48 | 288 / 14128 / 410 · 22.6 / 112 | 144 / 5472 / 137 · 36.0 / 64 |
| nettle-aes | 32 / 2480 / 309 · 9.6 / 16 | 32 / 2480 / 309 · 9.6 / 16 | 176 / 24800 / 778 · 21.7 / 64 | 384 / 46016 / 780 · 30.7 / 160 | 160 / 10016 / 158 · 34.0 / 64 |
| nsichneu | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 160 / 288 / 8 · 22.7 / 96 | 160 / 288 / 8 · 20.8 / 96 | 48 / 64 / 4 · 16.0 / 16 |
| picojpeg | 48 / 304 / 19 · 16.0 / 16 | - | 400 / 496400 / 16581 · 28.9 / 128 | 768 / 649488 / 16587 · 40.9 / 352 | 320 / 346224 / 8285 · 48.0 / 160 |
| qrduino | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 352 / 124144 / 5003 · 28.1 / 128 | 448 / 125648 / 5039 · 30.3 / 192 | 144 / 2320 / 107 · 25.9 / 64 |
| sglib-combined | 1616 / 3696 / 41 · 269.3 / 1024 | - | ? / 868496 / 47753 · 36.3 / 1152 | ? / 1175712 / 66954 · 34.1 / 1168 | 1312 / 487712 / 23876 · 82.8 / 1152 |
| slre | 2544 / 1364736 / 30189 · 112.5 / 2048 | 2544 / 1364736 / 30189 · 112.5 / 2048 | ? / 3089056 / 28790 · 33.1 / 208 | ? / 3287584 / 68570 · 27.5 / 176 | 3008 / 2701408 / 14628 · 448.0 / 2032 |
| statemate | 80 / 80 / 2 · 40.0 / 64 | - | 192 / 320016 / 6672 · 24.0 / 80 | 1152 / 3038528 / 19996 · 59.7 / 832 | 160 / 213424 / 3336 · 42.7 / 80 |
| tarfind | 16 / 16 / 1 · 16.0 / 16 | 16 / 16 / 1 · 16.0 / 16 | 144 / 256 / 8 · 21.3 / 80 | 176 / 1760 / 102 · 20.0 / 80 | 112 / 208 / 3 · 42.7 / 96 |
| ud | 416 / 714496 / 5361 · 82.7 / 256 | - | 976 / 1572064 / 3584 · 80.0 / 864 | 1312 / 1544208 / 3584 · 74.0 / 848 | 512 / 771792 / 1790 · 124.8 / 432 |
| xgboost | 64 / 8240 / 259 · 21.3 / 32 | 64 / 8240 / 259 · 21.3 / 32 | 208 / 24768 / 520 · 22.9 / 80 | 288 / 41184 / 520 · 24.7 / 144 | 128 / 16496 / 259 · 36.0 / 64 |

? the RISC-V code sets `sp` from a register (a variable-length array), which the execution log cannot follow.

### Reading the numbers

Over the 17 benchmarks together (the 16 that pass and aha-mont64 without
inlining; 377 M executed instructions) the dynamic mix is 18.2 % ALU,
3.6 % memory and 2.8 % control flow. The other 75 % move values and build
constants: 48.9 % dataflow, 24.1 % constants, 1.8 % casts, 0.6 % padding.
The static code (124 k instructions) looks the same: 10.5 % ALU, 5.3 %
memory, 3.7 % control, 43.1 % dataflow, 33.5 % constants. Per benchmark,
the instructions that compute, access memory or branch are between 13 %
(slre) and 35 % (nettle-aes) of those executed. The rest is
the price of the explicit dataflow model and the 16-bit encoding as the
compiler uses them today, and it is where the leverage is.

* **Constants** are 14–64 % of the executed instructions. `const` loads
  8 bits and every `grow` appends 8 more, so a 32-bit address costs three
  or four instructions; the static grow-per-const ratio (0.3–1.8) says most
  constants are 16 to 24 bits wide, which are the addresses of globals and
  the targets of far jumps. statemate is the extreme (64 % constants, 14
  memory instructions per 100): a state machine over global variables,
  where every access builds the variable's address from scratch. The
  compiler emits a new chain for every use. A wider immediate form or
  PC-relative addressing of data would cut this on the ISA side; keeping an
  address alive for its next use would trade constants for echoes on the
  compiler side.

* **Far jumps.** 53 % of all executed jumps (0.1 % in crc32, 85 % in
  qrduino and slre) do not fit the 7-bit offset of the short form and take
  their target from an operand, which costs the `const`/`grow` chain that
  builds it, two or three instructions per jump. Statically
  76 % of all jumps are far. A block layout that keeps a block's
  successors within reach of the short form, or a two-word jump with a
  longer offset, would reclaim most of these chains.

* **Dataflow.** Echoes and dups are 49 % of the executed instructions
  (14 % in statemate, 69 % in slre). A value is forwarded 0.2–5.5 times on
  average before it is consumed (5.5 in slre), and an executed basic block
  holds 4–60 dataflow instructions. This is mostly the cost of keeping
  values alive without registers: a value that is live across a block
  boundary, or further ahead than a reference reaches, is re-sent in every
  block it passes through, so a loop-carried or loop-invariant value costs
  an echo per block per iteration. At run time 83 % of the echoes use the
  short form with two 5-bit references (long echoes are 0 % of the echoes
  in nsichneu and 42 % in nettle-aes), while 35 % of the static echoes are
  long: the hot loops are compact.

* **Operand lifetimes.** The median lifetime is 1 or 2 instructions
  (4 in aha-mont64): most values are consumed by the next instruction,
  which is what the forwarding model is built for. The mean is 10–137 and
  the 90th percentile 4–172, so a minority of values lives long, and the maxima (up to 17.6 M
  instructions in xgboost) are values that live for the whole run, such as
  the iteration count of the benchmark harness. Between 6 and 65 values
  are in flight on average and up to 90 at once (slre); whatever buffers
  produced values in hardware has to hold that many. 0–10 % of the produced
  values are never consumed (10 % in slre, 3–5 % in seven of the programs):
  results the compiler materializes and then drops.

* **Control flow.** Nearly every jump, call and return is issued with
  trigger offset 0 and takes effect at once. The issue-to-trigger distance
  is nonzero only in the backend's multi-way branches, where several jumps
  are issued ahead of one shared trigger point (`jmp 1, 8`, then the
  compare and the address chain of a far `jmp 0, 4`, then those of a far
  `jmp 0, 0`): 11 % of the jumps in slre, 2 % in statemate, under 0.5 %
  elsewhere. Executed basic blocks are 15–165 instructions long on average
  (median 7–68), so there would be room to issue a block's final jump early
  and overlap its address chain with the block's work; today the trigger
  delay is used for multi-way branches only. Calls are rare: most
  benchmarks are inlined down to a few hundred or thousand calls per run,
  and crc32 (175 k), tarfind, slre, sglib-combined and statemate (27–38 k)
  are the ones that call per element.

* **Memory.** Loads and stores are 0.1–14 per 100 instructions and fetch
  1 to 4 bytes each (1.0 on average in xgboost, 4.0 in crc32, nsichneu and
  ud). The stack is almost unused: twelve benchmarks never go deeper than
  16 bytes, and the stack-indexed loads and stores (the `ld`/`st` forms
  that name a stack slot, used for spills and locals) are a handful in
  most, with slre (2.5 KB, 57 k stack accesses), sglib-combined (1.6 KB,
  39 k) and huffbench (7.7 KB of local arrays, addressed through `saddr`
  and ordinary loads) as the exceptions. Values live in the operand
  network rather than in memory, and the cost of that shows up in the
  dataflow column, not in memory traffic.

* **Operand widths.** 22 % of the produced values are 8- or 16-bit (3 % in
  crc32, 57 % in picojpeg); over all benchmarks the types are 62.1 % u32,
  16.1 % i32, 14.1 % u8, 5.6 % i16, 1.1 % i8, 1.0 % u16. Types ride on
  the values, so narrow types cost a `cast` only where C's promotions
  demand one: 0–6 % of the executed instructions.

* **aha-mont64**, built with `-fno-inline-functions` (see above), is the
  one benchmark with 64-bit arithmetic, which the compiler lowers to pairs
  of 32-bit operations. Its 66.8 M instructions are 25 % ALU, 53 %
  dataflow and 18 % constants, with almost no memory traffic (0.2 %: six
  loads and 21 k stores, plus 68 k stack-indexed accesses for the 64-bit
  halves it keeps in stack slots). Its values live longest of all: a
  median lifetime of 4 instructions, a mean of 94, and 56 values in flight
  on average; 38 % of its echoes are long and its blocks are the longest
  (136 instructions on average), since each 64-bit operation becomes a
  straight-line sequence over two halves that keeps more values alive for
  longer.

#### Against RISC-V

* **The same translation on both ISAs.** Through llvm2clif and Cranelift,
  Scry executes 3.7× as many instructions as rv64imc over the suite, from
  2.0× (statemate) to 14.7× (aha-mont64, whose 64-bit arithmetic is native
  on rv64 and lowered to pairs of 32-bit operations on Scry); the median
  benchmark is at 3.6×. Against LLVM's rv32imc the factor is 6.4×, and the
  two differ by 1.7×, which is how much more code Cranelift's riscv64
  backend executes than LLVM's for the same IR (without its optimizer, as
  in the Scry build). So of the 6.4× between Scry and a conventional
  RISC-V compiler, about 3.7× is the ISA and the Scry backend and about
  1.7× is the translation path and Cranelift. With Cranelift's optimizer on
  the rv64imc code shrinks by 12 % over the suite (4.2× against Scry), but
  not uniformly: nsichneu loses almost half, ud a quarter and aha-mont64 a
  fifth, while tarfind, depthconv and nettle-aes gain instructions.

* **Where the instructions go.** Against rv64imc through Cranelift, Scry
  executes 1.5× the ALU instructions, 0.8× the control instructions
  (Cranelift's unoptimized code runs 4.8 M unconditional jumps where
  LLVM's runs 0.7 M, with the same 7 M conditional branches) and 0.85×
  the loads and stores (Cranelift spills more: 2 % of its instructions are
  stack-relative accesses, 16 % in slre, where Scry forwards instead).
  Constants are a similar share on both, 24 % on Scry and 22 % on
  rv64imc, since Cranelift without its optimizer materializes every
  constant and address at each use as Scry does (`lui`/`auipc`+`addi`
  pairs, `li`, constant-pool loads), where LLVM's code has 7 %. The
  difference is the dataflow: 49 % of Scry's instructions forward values,
  against 4 % register moves on rv64imc.

* **Code size and fetch.** Scry's executables take 1.45× the bytes of the
  rv64imc Cranelift build (2.3× its static instructions) and 2.7× those
  of LLVM's rv32imc; Cranelift's riscv64 code is itself 1.9× LLVM's in
  bytes. Weighted by execution, Scry fetches 2.3× the instruction bytes of
  rv64imc (38 % of whose instructions are compressed) and 4.0× those of
  rv32imc.

* **Values.** With the same translation, the median lifetime is 1 to 3
  instructions on rv64imc as on Scry (1 to 4), the means 7 to 21 against
  10 to 137 and the 90th percentiles 3 to 33 against 4 to 172; rv64imc
  keeps 7 to 23 register values live on average and at most 27, Scry 6 to
  65 and up to 90. Through LLVM (rv32imc) the figures are a little longer
  (means 12 to 29, 11 to 24 live) because its code is denser. The
  never-used shares are alike on all three (slre 8 to 10 %, most others
  below 4 %), so the dead values come from the IR rather than from a
  backend.

* **Blocks and memory.** Branch, call and return counts agree (same IR),
  and the executed blocks are 3 to 56 instructions long on rv64imc and 3
  to 45 on rv32imc against 15 to 165 on Scry. Loads and stores match where
  nothing spills (crc32: 525,485 on Scry, 525,529 on rv64imc, 525,519 on
  rv32imc); the Cranelift build spills most in matmult-int (514 k
  stack-relative accesses, 2.3× Scry's memory traffic) and slre (891 k,
  4.3×).

* **Against LLVM's rv32imc** specifically: 67 to 100 % of its instructions
  compute, access memory or branch (constants 0 to 27 %, register moves 0
  to 18 %); Scry executes 2.2× its ALU instructions and 1.2× its control
  instructions; its small kernels are smaller than Scry's (crc32 0.49×,
  depthconv 0.68×, xgboost 0.73×) and nsichneu (4.8×) and slre (4.5×) the
  largest, with far jumps and three-byte addresses on the Scry side.

#### Scry with Cranelift's optimizer on

* **What the optimizer does to the Scry code.** `scry-cc --cranelift-opt-level
  speed` runs Cranelift's mid-end (value numbering, constant folding, code
  motion) on the CLIF before the Scry backend. Twelve of the seventeen
  benchmarks compiled that way; matmult-int, picojpeg, sglib-combined,
  statemate and ud did not finish compiling within 30 minutes (the
  backend's compile-time problem, see `docs/backend-issues`; the base
  builds of the same programs compile in seconds). Over the twelve, the
  optimized builds execute 4 % more instructions than the base builds
  (20 % more without aha-mont64), from half as many in aha-mont64, whose
  64-bit lowering is full of shared sub-expressions, to 2.6× as many in
  qrduino; crc32, depthconv, huffbench, slre and tarfind execute 2 to 16 %
  fewer instructions, edn, md5sum, nettle-aes, nsichneu and xgboost 6 to
  34 % more. The code is 24 % larger.

* **Why.** The optimizer does what it should: constant instructions fall
  by 41 % and ALU instructions by 17 %, as repeated address chains and
  common sub-expressions are computed once. But every value computed once
  and used several times has to stay alive and be forwarded to each use,
  and on Scry that costs an echo or dup per block per use: dataflow
  instructions grow by 34 % (from 50 % to 64 % of the stream), each value
  is forwarded 2.9 times instead of 1.7, operand lifetimes lengthen (means
  20 to 178 instructions against 10 to 137) and more values are in flight
  (12 to 78 on average, up to 142). On Scry, recomputing a constant is
  often cheaper than keeping it, and the optimizer, written for register
  machines, cannot know that. Against RISC-V the picture does not change:
  over the same twelve benchmarks the optimized Scry build executes 4.3×
  the instructions of the rv64imc Cranelift build (4.1× for the base
  build) and 7.0× those of LLVM's rv32imc (6.7×).

#### Distances and the stack, dynamic against static

* **Instruction bytes.** Weighted by execution, the Scry build fetches
  755 MB of instructions for the suite (2 bytes each), the rv64imc
  Cranelift build 329 MB (38 % of its instructions compressed), LLVM's
  rv32imc 191 MB; the static code is 248 KB, 172 KB and 91 KB. Scry's
  code is the densest per instruction and the least dense per unit of
  work.

* **First use of a value.** On Scry a value reaches its first consumer or
  forwarder after 1 instruction in the median (mean 1.5 to 10), which is
  what the output references encode: statically the mean reference
  distance is 3.3 to 21 instructions, the long echoes stretching it. On
  RISC-V a register's first read follows its definition after 1 to 3
  instructions in the median (mean 2.7 to 11), and the static distance to
  the first reader within the block is 1.3 to 4.9 on average.

* **Load to consume.** The Scry backend always sends a load's value to the
  next instruction (static reference distance 1 for every load in every
  benchmark), which is an echo or dup as often as the consumer; the value
  reaches its consumer after 1 to 10 instructions in the median (159 in
  aha-mont64, whose loads feed the loop end) and 2 to 22 on average (1,765
  in aha-mont64). A load's value is thus held in the operand network for
  tens of instructions before it is used, where a RISC-V register value
  loaded by `ld`/`lw` is read after 1 to 11 instructions in the median
  (means 1.4 to 314 across the builds, aha-mont64 again the outlier). For
  a pipeline this is the slack available to hide load latency: Scry's
  schedule has the consumer a median 1 to 10 instructions behind the load,
  LLVM's rv32imc code 1 to 10, Cranelift's rv64imc 1 to 11.

* **Jump issue to trigger.** The static trigger offsets of the Scry code
  say the same as the dynamic distances: 0 for every jump, call and
  return except in the multi-way branches of four benchmarks (static mean
  up to 0.85, maximum 40; dynamic mean up to 0.9, maximum 36). The
  compiler never issues a jump ahead of its block's end.

* **Stack.** Over the suite the Scry builds reserve 2.4 MB of stack in
  55 k reservations, the rv64imc Cranelift build 6.9 MB in 129 k and LLVM's
  rv32imc 4.7 MB in 56 k; Scry's static frames are 16 bytes in most
  functions (mean 9.6 to 1,539 bytes, the 4 KB maximum in huffbench, with
  address-taken arrays), RISC-V frames 16 to 80 bytes on average
  (Cranelift) and 16 to 448 (LLVM, slre's regex matcher at 2 KB). The
  deepest extents are 16 bytes for twelve benchmarks on Scry against
  48 to 3,008 bytes on RISC-V where the log can follow them, since RISC-V
  frames hold saved registers and spills that Scry keeps in flight; slre, huffbench and
  sglib-combined, with variable-length arrays, cannot be followed on
  RISC-V from the log. Memory traffic in bytes is 30 MB on Scry, 34 MB on
  rv32imc and 60 MB on rv64imc, whose spills and pointers are 8 bytes.

**Simulator speed.** scryer `b1737e6` runs about 80 k instructions per
second with events streamed to a pipe (the flush per step dominates) and
350–650 k without, so the dynamic pass over the 17 benchmarks is about 80
minutes of simulator time (half an hour with three jobs); the traces are
1.5–7 GB per benchmark and are never written to disk.

**Not measured.** None of this is time. Neither simulator has a timing
model (scryer steps instructions, qemu translates them), so a cycle count
needs a model of each pipeline (issue width, the latency of an echo,
memory) before instruction counts turn into performance; with 6.4× the
instructions, Scry has to issue them that much faster, or wider, to match
rv32imc. And how many of the dataflow instructions are inherent to the ISA
and how many a better schedule or block layout could avoid can only be
measured inside the compiler, by counting the echoes it inserts at block
boundaries against those it inserts for reach.
