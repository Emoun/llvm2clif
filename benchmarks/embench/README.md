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
```

Results go to `out/<name>/metrics.json`, `rv64cl.json` and `rv32imc.json`;
the tables below are what the script prints. Streaming events costs the simulator most of its speed
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

### Code size and static instruction mix

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

### Dynamic instruction mix

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

### Operand lifetimes (instructions from production to final consumption) and queue occupancy

| benchmark | operands | mean | median | p90 | p99 | max | discarded % | forwards per operand | live operands mean | max |
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

### Control flow

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

### Memory traffic and stack

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

### Scry against RISC-V: instructions executed

| benchmark | Scry | rv64imc Cranelift | Scry ratio | rv32imc llc | Scry ratio | rv64imc Cranelift, optimizer on | Scry ratio |
|---|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 66837654 | 4544346 | 14.71 | 6495889 | 10.29 | 3735470 | 17.89 |
| crc32 | 15950330 | 6308852 | 2.53 | 4030049 | 3.96 | 6133581 | 2.60 |
| depthconv | 18134577 | 3323259 | 5.46 | 2721074 | 6.66 | 3639654 | 4.98 |
| edn | 25880475 | 8629078 | 3.00 | 3867234 | 6.69 | 7843978 | 3.30 |
| huffbench | 25154520 | 6162414 | 4.08 | 2991816 | 8.41 | 4987616 | 5.04 |
| matmult-int | 12214699 | 5717048 | 2.14 | 1876139 | 6.51 | 4233602 | 2.89 |
| md5sum | 17861357 | 4510404 | 3.96 | 3272668 | 5.46 | 4311284 | 4.14 |
| nettle-aes | 18691343 | 6910189 | 2.70 | 3537884 | 5.28 | 7042836 | 2.65 |
| nsichneu | 16681595 | 5424206 | 3.08 | 2009931 | 8.30 | 2979167 | 5.60 |
| picojpeg | 15514899 | 8646407 | 1.79 | 3608659 | 4.30 | 6705161 | 2.31 |
| qrduino | 21502235 | 6060314 | 3.55 | 3604362 | 5.97 | 5488094 | 3.92 |
| sglib-combined | 20901068 | 4411218 | 4.74 | 2882838 | 7.25 | 4398356 | 4.75 |
| slre | 23862386 | 5582906 | 4.27 | 2937030 | 8.12 | 5647631 | 4.23 |
| statemate | 10091747 | 5090968 | 1.98 | 3485148 | 2.90 | 5094263 | 1.98 |
| tarfind | 15431905 | 3733769 | 4.13 | 2702109 | 5.71 | 4439243 | 3.48 |
| ud | 17481193 | 4842804 | 3.61 | 2581831 | 6.77 | 3577199 | 4.89 |
| xgboost | 35299960 | 11693931 | 3.02 | 6775071 | 5.21 | 9508055 | 3.71 |
| **total** | **377491943** | **101592113** | **3.72** | **59379732** | **6.36** | **89765190** | **4.21** |

### Code size against RISC-V

| benchmark | Scry bytes | static instructions | rv64imc Cranelift bytes | Scry ratio | static instructions | compressed % | rv32imc llc bytes | Scry ratio | static instructions | compressed % | rv32im llc bytes | Scry ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 4048 | 2024 | 2752 | 1.47 | 972 | 58.4 | 2188 | 1.85 | 788 | 61.2 | 3160 | 1.28 |
| crc32 | 672 | 336 | 2246 | 0.30 | 769 | 54.0 | 1378 | 0.49 | 493 | 60.2 | 1972 | 0.34 |
| depthconv | 1068 | 534 | 2286 | 0.47 | 762 | 50.0 | 1574 | 0.68 | 533 | 52.3 | 2124 | 0.50 |
| edn | 4264 | 2132 | 4582 | 0.93 | 1521 | 49.4 | 2928 | 1.46 | 1006 | 54.5 | 4008 | 1.06 |
| huffbench | 7228 | 3614 | 4406 | 1.64 | 1509 | 54.0 | 2750 | 2.63 | 992 | 61.4 | 3960 | 1.83 |
| matmult-int | 2860 | 1430 | 3774 | 0.76 | 1258 | 50.0 | 2208 | 1.30 | 763 | 55.3 | 3012 | 0.95 |
| md5sum | 2284 | 1142 | 2710 | 0.84 | 907 | 50.6 | 1846 | 1.24 | 642 | 56.2 | 2568 | 0.89 |
| nettle-aes | 8728 | 4364 | 7142 | 1.22 | 2302 | 44.9 | 4290 | 2.03 | 1434 | 50.4 | 5696 | 1.53 |
| nsichneu | 66736 | 33368 | 35126 | 1.90 | 10819 | 37.7 | 13828 | 4.83 | 4674 | 52.1 | 18696 | 3.57 |
| picojpeg | 66556 | 33278 | 42852 | 1.55 | 12959 | 34.7 | 21316 | 3.12 | 6515 | 36.4 | 26156 | 2.54 |
| qrduino | 28024 | 14012 | 18428 | 1.52 | 5660 | 37.2 | 10852 | 2.58 | 3493 | 44.7 | 13956 | 2.01 |
| sglib-combined | 15644 | 7822 | 13372 | 1.17 | 4733 | 58.7 | 8464 | 1.85 | 3174 | 66.7 | 12700 | 1.23 |
| slre | 19680 | 9840 | 8598 | 2.29 | 2946 | 54.1 | 4386 | 4.49 | 1537 | 57.3 | 6104 | 3.22 |
| statemate | 12292 | 6146 | 14382 | 0.85 | 4235 | 30.2 | 6944 | 1.77 | 2047 | 30.4 | 8184 | 1.50 |
| tarfind | 3588 | 1794 | 2734 | 1.31 | 914 | 50.4 | 1838 | 1.95 | 652 | 59.0 | 2608 | 1.38 |
| ud | 3136 | 1568 | 3470 | 0.90 | 1157 | 50.0 | 2172 | 1.44 | 744 | 54.0 | 2972 | 1.06 |
| xgboost | 1308 | 654 | 2726 | 0.48 | 935 | 54.2 | 1788 | 0.73 | 618 | 55.3 | 2472 | 0.53 |
| **total** | **248116** | | **171586** | **1.45** | | | **90750** | **2.73** | | | **120348** | **2.06** |

### RISC-V dynamic instruction mix (rv64imc Cranelift / rv32imc llc)

| benchmark | alu % | constant % | move % | control % | memory % | other % | compressed % | branches taken % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 58.2 / 90.6 | 15.5 / 0.2 | 9.6 / 0.9 | 14.8 / 7.2 | 2.0 / 1.1 | 0.0 / 0.0 | 43.3 / 24.7 | 45.7 / 47.1 |
| crc32 | 41.7 / 52.2 | 41.7 / 26.1 | 0.0 / 0.0 | 8.3 / 13.1 | 8.3 / 8.7 | 0.0 / 0.0 | 36.1 / 56.5 | 99.6 / 99.6 |
| depthconv | 50.6 / 63.9 | 23.8 / 4.6 | 3.3 / 0.0 | 4.9 / 9.9 | 17.4 / 21.6 | 0.0 / 0.0 | 33.5 / 22.2 | 97.9 / 26.4 |
| edn | 63.1 / 61.6 | 16.9 / 0.4 | 2.4 / 2.4 | 6.1 / 10.0 | 11.5 / 25.7 | 0.0 / 0.0 | 45.2 / 38.0 | 62.4 / 97.3 |
| huffbench | 42.3 / 45.9 | 19.2 / 0.9 | 8.3 / 7.3 | 17.6 / 22.2 | 12.5 / 23.6 | 0.0 / 0.0 | 54.1 / 53.0 | 22.2 / 60.0 |
| matmult-int | 39.9 / 49.4 | 17.3 / 0.1 | 9.0 / 6.9 | 9.6 / 8.0 | 24.2 / 35.6 | 0.0 / 0.0 | 43.8 / 35.6 | 3.2 / 98.1 |
| md5sum | 41.6 / 52.6 | 15.3 / 0.2 | 15.5 / 17.6 | 16.8 / 14.8 | 10.7 / 14.8 | 0.0 / 0.0 | 38.5 / 55.2 | 12.5 / 64.0 |
| nettle-aes | 75.3 / 73.2 | 9.2 / 0.1 | 1.3 / 0.3 | 1.8 / 2.2 | 12.4 / 24.2 | 0.0 / 0.0 | 43.0 / 35.1 | 86.7 / 58.0 |
| nsichneu | 11.6 / 0.1 | 51.2 / 0.0 | 0.0 / 0.1 | 14.5 / 38.5 | 22.7 / 61.3 | 0.0 / 0.0 | 37.4 / 35.6 | 20.7 / 21.1 |
| picojpeg | 40.8 / 48.3 | 11.3 / 11.2 | 4.2 / 3.9 | 33.3 / 10.7 | 10.4 / 25.9 | 0.0 / 0.0 | 49.0 / 39.1 | 43.3 / 72.0 |
| qrduino | 55.3 / 64.0 | 15.6 / 0.9 | 3.5 / 1.6 | 14.0 / 15.0 | 11.5 / 18.5 | 0.0 / 0.0 | 31.7 / 39.4 | 46.7 / 39.1 |
| sglib-combined | 41.2 / 29.8 | 12.0 / 6.0 | 5.1 / 6.1 | 18.1 / 24.6 | 23.6 / 33.5 | 0.0 / 0.0 | 46.9 / 60.7 | 39.8 / 43.7 |
| slre | 41.2 / 20.7 | 17.3 / 17.4 | 5.3 / 5.2 | 16.2 / 23.4 | 20.1 / 33.4 | 0.0 / 0.0 | 43.2 / 53.7 | 49.0 / 45.4 |
| statemate | 8.1 / 6.8 | 56.7 / 37.1 | 1.2 / 6.4 | 6.1 / 13.4 | 27.9 / 36.3 | 0.0 / 0.0 | 11.0 / 15.0 | 57.1 / 83.0 |
| tarfind | 34.3 / 35.3 | 12.4 / 8.2 | 11.8 / 16.4 | 26.6 / 20.9 | 14.9 / 19.3 | 0.0 / 0.0 | 44.2 / 30.2 | 9.8 / 95.8 |
| ud | 47.9 / 52.2 | 29.3 / 2.8 | 2.4 / 9.5 | 8.1 / 12.5 | 12.4 / 23.0 | 0.0 / 0.0 | 32.1 / 53.1 | 60.4 / 51.0 |
| xgboost | 53.3 / 49.3 | 23.6 / 1.6 | 1.5 / 7.8 | 7.2 / 16.4 | 14.4 / 24.9 | 0.0 / 0.0 | 23.0 / 38.8 | 68.3 / 37.3 |

### Values: Scry operands against RISC-V register values (Scry / rv64imc Cranelift / rv32imc llc)

| benchmark | lifetime median | mean | p90 | live values mean | max | never used % |
|---|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 4 / 2 / 3 | 94.0 / 17.4 / 22.4 | 172 / 17 / 28 | 55.8 / 17.3 / 22.7 | 79 / 22 / 28 | 4.6 / 3.1 / 0.2 |
| crc32 | 1 / 1 / 2 | 13.3 / 9.5 / 11.6 | 7 / 4 / 21 | 8.9 / 10.7 / 11.1 | 12 / 13 / 13 | 0.0 / 0.0 / 0.0 |
| depthconv | 1 / 2 / 4 | 20.7 / 8.2 / 15.7 | 44 / 9 / 51 | 15.6 / 9.7 / 14.9 | 30 / 12 / 19 | 2.7 / 0.0 / 0.3 |
| edn | 1 / 1 / 5 | 22.7 / 15.0 / 21.6 | 35 / 17 / 12 | 13.1 / 19.7 / 21.1 | 85 / 27 / 30 | 0.2 / 0.1 / 0.5 |
| huffbench | 1 / 1 / 4 | 33.6 / 12.5 / 20.8 | 34 / 14 / 15 | 17.3 / 12.8 / 17.2 | 26 / 23 / 23 | 3.4 / 3.8 / 1.9 |
| matmult-int | 1 / 2 / 3 | 53.9 / 13.5 / 19.1 | 26 / 9 / 6 | 36.2 / 13.0 / 17.6 | 72 / 27 / 27 | 0.3 / 0.1 / 0.1 |
| md5sum | 1 / 2 / 4 | 37.8 / 16.7 / 28.3 | 33 / 10 / 33 | 20.5 / 14.8 / 23.2 | 28 / 19 / 30 | 0.0 / 1.9 / 0.3 |
| nettle-aes | 1 / 1 / 1 | 54.6 / 21.0 / 23.1 | 155 / 8 / 19 | 39.0 / 22.5 / 23.3 | 72 / 26 / 29 | 0.3 / 0.0 / 0.1 |
| nsichneu | 1 / 1 / 1 | 10.3 / 7.3 / 27.5 | 4 / 3 / 2 | 6.3 / 8.3 / 18.8 | 9 / 18 / 20 | 3.1 / 0.0 / 0.0 |
| picojpeg | 2 / 2 / 3 | 24.5 / 14.9 / 16.8 | 40 / 23 / 23 | 16.1 / 15.5 / 18.2 | 43 / 25 / 28 | 3.2 / 3.4 / 1.4 |
| qrduino | 1 / 1 / 2 | 35.1 / 15.1 / 22.3 | 38 / 14 / 18 | 19.4 / 14.6 / 20.0 | 68 / 25 / 29 | 3.5 / 2.2 / 0.6 |
| sglib-combined | 2 / 2 / 3 | 44.6 / 15.9 / 22.9 | 34 / 13 / 16 | 18.3 / 14.3 / 16.1 | 36 / 22 / 24 | 5.2 / 3.1 / 2.2 |
| slre | 2 / 2 / 2 | 136.8 / 12.9 / 15.6 | 75 / 33 / 40 | 65.2 / 13.7 / 14.9 | 90 / 23 / 24 | 10.2 / 8.1 / 10.2 |
| statemate | 1 / 1 / 3 | 12.6 / 6.5 / 20.0 | 5 / 4 / 39 | 9.9 / 7.0 / 14.7 | 38 / 18 / 21 | 1.7 / 2.1 / 1.8 |
| tarfind | 1 / 3 / 3 | 26.8 / 20.8 / 25.8 | 24 / 6 / 7 | 15.7 / 14.7 / 17.3 | 20 / 17 / 19 | 0.4 / 0.1 / 0.7 |
| ud | 1 / 1 / 5 | 28.5 / 12.1 / 28.7 | 42 / 18 / 38 | 16.1 / 12.6 / 23.9 | 25 / 19 / 30 | 1.6 / 0.6 / 6.6 |
| xgboost | 1 / 1 / 2 | 35.6 / 16.1 / 28.0 | 81 / 16 / 34 | 17.0 / 17.0 / 23.8 | 22 / 20 / 27 | 3.4 / 3.4 / 2.9 |

### Control flow and memory: Scry against RISC-V (Scry / rv64imc Cranelift / rv32imc llc)

| benchmark | block length mean | median | calls | loads+stores | of which stack-relative | max stack bytes |
|---|---:|---:|---:|---:|---:|---:|
| aha-mont64 | 136.3 / 6.8 / 13.9 | 68 / 8 / 9 | 9469 / 9470 / 9468 | 88935 / 91837 / 70073 | 68114 / 81422 / 49258 | 128 / 320 / 208 |
| crc32 | 22.7 / 12.0 / 7.7 | 16 / 15 / 9 | 175284 / 175285 / 175283 | 525485 / 525529 / 350415 | 2 / 46 / 36 | 16 / 112 / 48 |
| depthconv | 164.9 / 20.2 / 10.1 | 7 / 7 / 3 | 1649 / 1650 / 1649 | 577346 / 577374 / 587196 | 2 / 30 / 9854 | 16 / 80 / 64 |
| edn | 34.1 / 16.4 / 10.0 | 22 / 17 / 7 | 256 / 257 / 256 | 977815 / 994241 / 993540 | 2 / 14828 / 15730 | 16 / 320 / 192 |
| huffbench | 27.7 / 5.7 / 4.5 | 19 / 4 / 4 | 154 / 155 / 154 | 706326 / 772300 / 706544 | 2 / 96 / 256 | 7696 / ? / ? |
| matmult-int | 41.3 / 10.4 / 12.6 | 22 / 6 / 6 | 90 / 91 / 90 | 612003 / 1385237 / 668557 | 2 / 514036 / 56556 | 16 / 336 / 144 |
| md5sum | 23.5 / 6.0 / 6.8 | 19 / 4 / 4 | 545 / 546 / 477 | 481003 / 482383 / 482842 | 2 / 1382 / 2050 | 16 / 208 / 144 |
| nettle-aes | 158.1 / 56.4 / 45.1 | 34 / 12 / 33 | 394 / 395 / 394 | 852569 / 857687 / 856325 | 618 / 5736 / 4374 | 32 / 176 / 160 |
| nsichneu | 17.9 / 6.9 / 2.6 | 18 / 7 / 3 | 9 / 10 / 9 | 1231786 / 1231850 / 1231787 | 2 / 66 / 18 | 16 / 160 / 48 |
| picojpeg | 36.7 / 3.0 / 9.4 | 27 / 2 / 8 | 8316 / 8317 / 8322 | 795641 / 903171 / 935628 | 14 / 107568 / 159790 | 48 / 400 / 320 |
| qrduino | 31.7 / 7.1 / 6.7 | 23 / 4 / 5 | 2657 / 2658 / 2651 | 667597 / 699071 / 665868 | 2 / 31314 / 952 | 16 / 352 / 144 |
| sglib-combined | 25.0 / 5.5 / 4.1 | 20 / 4 / 3 | 33579 / 33580 / 33578 | 814540 / 1039566 / 964649 | 38948 / 262658 / 227812 | 1616 / ? / 1312 |
| slre | 28.1 / 6.2 / 4.3 | 23 / 4 / 3 | 35109 / 35110 / 34640 | 262318 / 1119617 / 979566 | 57100 / 891233 / 785931 | 2544 / ? / 3008 |
| statemate | 30.0 / 16.4 / 7.5 | 11 / 5 / 4 | 26657 / 26658 / 29989 | 1369207 / 1422535 / 1266041 | 18 / 79995 / 99999 | 80 / 192 / 160 |
| tarfind | 14.9 / 3.8 / 4.8 | 19 / 4 / 4 | 37985 / 37986 / 37984 | 555961 / 556017 / 520213 | 2 / 58 / 585 | 16 / 144 / 112 |
| ud | 37.9 / 12.4 / 8.0 | 24 / 8 / 7 | 1797 / 1798 / 1796 | 568196 / 600632 / 595046 | 1788 / 33988 / 30430 | 416 / 976 / 512 |
| xgboost | 42.1 / 13.9 / 6.1 | 41 / 15 / 6 | 265 / 266 / 264 | 1681558 / 1685174 / 1686200 | 2822 / 8742 / 9768 | 64 / 208 / 128 |

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
