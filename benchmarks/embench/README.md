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

With rust-wasmtime `f9fa226`, scryer `6fc6ac9` and wild `2b9f2b4`:

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
| huffbench | simulator limit | passes with a bigger stack (25.0 M); needs a 5 KiB frame, scryer has 4 KiB |
| nsichneu | simulator limit | passes with the stack moved up (16.7 M); its 68 KiB image overlaps the stack at 64 KiB |
| picojpeg | simulator limit | passes with the stack moved up (15.4 M); its 73 KiB image overlaps the stack at 64 KiB. Was stopped by backend issues 12 and 13 before `f9fa226` |
| aha-mont64 | backend bug | [issue 14](../../docs/backend-issues/README.md): the inlined loop does not compile in 30 minutes; passes with `--cflags=-fno-inline-functions` (67.3 M, compiles in 0.5 s) |
| nettle-sha256 | compile time | one 837-instruction block (the rounds are unrolled by hand, so no flag helps: the block is the same size at `-O1` and `-Oz`); see below |
| wikisort | unsupported | calls `sqrt` on a `double`; no floating point on Scry |

Instruction counts are scryer's `InstructionReads` for one iteration
(`GLOBAL_SCALE_FACTOR=1`, including one warm-up run); they vary slightly
between backend revisions and compiles, since code generation is not
deterministic.

**Simulator limits.** scryer fixes the stack at address `0x10000` with a
4 KiB buffer (`stack_base` and `stack_buffer` in its `src/lib.rs`), and a
program image must fit below it. Raising the two constants (the results
above for huffbench, nsichneu and picojpeg used `1 << 24` and `1 << 20`)
lets all three benchmarks run and verify; nothing in the generated code
depends on the stack address.

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
