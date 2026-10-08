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

With rust-wasmtime `fad5e62`, scryer `6fc6ac9` and wild `2b9f2b4`:

| benchmark | result | notes |
|-----------|--------|-------|
| crc32 | pass | 16.3 M instructions |
| depthconv | pass | 18.3 M |
| matmult-int | pass | 12.2 M |
| md5sum | pass | 17.8 M |
| nettle-aes | pass | 18.7 M |
| qrduino | pass | 21.3 M |
| sglib-combined | pass | 21.2 M |
| slre | pass | 24.5 M; needs `memchr`, which LLVM makes out of `strchr` |
| statemate | pass | 10.1 M |
| tarfind | pass | 15.3 M |
| ud | pass | 17.7 M |
| xgboost | pass | 36.2 M |
| huffbench | simulator limit | passes with a bigger stack (25.3 M); needs a 5 KiB frame, scryer has 4 KiB |
| nsichneu | simulator limit | passes with the stack moved up (16.7 M); its 68 KiB image overlaps the stack at 64 KiB |
| edn | backend bug | [issue 11](../../docs/backend-issues/README.md): sub-word parameter typing panic |
| picojpeg | backend bug | [issue 13](../../docs/backend-issues/README.md): reference distances do not converge (after [issue 12](../../docs/backend-issues/README.md) was avoided in llvm2clif) |
| aha-mont64 | backend bug | [issue 14](../../docs/backend-issues/README.md): the inlined loop does not compile in 30 minutes; passes with `--cflags=-fno-inline-functions` (67.1 M, compiles in 0.6 s) |
| nettle-sha256 | compile time | one 837-instruction block (the rounds are unrolled by hand, so no flag helps); see below |
| wikisort | unsupported | calls `sqrt` on a `double`; no floating point on Scry |

Instruction counts are scryer's `InstructionReads` for one iteration
(`GLOBAL_SCALE_FACTOR=1`, including one warm-up run).

**Simulator limits.** scryer fixes the stack at address `0x10000` with a
4 KiB buffer (`stack_base` and `stack_buffer` in its `src/lib.rs`), and a
program image must fit below it. Raising the two constants (the results
above for huffbench and nsichneu used `1 << 24` and `1 << 20`) lets both
benchmarks run and verify; nothing in the generated code depends on the
stack address.

**Compile time.** The backend's compile time grows cubically with the size
of a basic block (see the open issue in `docs/backend-issues`), and the two
benchmarks with very long straight-line blocks did not finish compiling
within two hours; the others compile in under 10 seconds. `aha-mont64`'s block is
the benchmark loop with everything inlined, and what makes it slow is not
its length but the loop nesting (issue 14, with a 30-instruction
reproducer); built with `--cflags=-fno-inline-functions` (the `=` form,
because the value starts with a dash) it compiles in under a second and
passes.
