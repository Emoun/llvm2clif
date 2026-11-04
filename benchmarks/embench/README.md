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

With rust-wasmtime `f9fa226`, scryer `eb7d2b5` and wild `2b9f2b4`:

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
| huffbench | pass with `--sim-args="--stack-buffer 1048576"` | 25.0 M; one of its reservations exhausts scryer's default 4 KiB stack pool |
| aha-mont64 | backend bug | [issue 14](../../docs/backend-issues/README.md): the inlined loop does not compile in 30 minutes; passes with `--cflags=-fno-inline-functions` (67.3 M, compiles in 0.5 s) |
| nettle-sha256 | compile time | one 837-instruction block (the rounds are unrolled by hand, so no flag helps: the block is the same size at `-O1` and `-Oz`); see below |
| wikisort | unsupported | calls `sqrt` on a `double`; no floating point on Scry |

Instruction counts are the `InstructionReads` metric of scryer `6fc6ac9`
for one iteration (`GLOBAL_SCALE_FACTOR=1`, including one warm-up run);
they vary slightly between backend revisions and compiles, since code
generation is not deterministic. scryer `8e73d4a` and later print no metrics,
so the runner's `instr` column is 0 with them, and they keep every event of
a run in memory unless the events are printed (about 160 bytes per
instruction, 2 to 6 GB per benchmark here), so use `--jobs 1` unless the
machine has memory to spare, or pass `--sim-args=--print-events=/dev/null`
to keep memory flat at the cost of speed. `--sim-args` passes any other
simulator options through, such as `--stack-buffer`.

**Simulator limits.** scryer places the stack at `0x100000` by default
(`0x10000` before `8059eee`) with a 4 KiB pool for stack growth, and a
program image must fit below the stack. Since `eb7d2b5` both are options,
`--stack-base` and `--stack-buffer`; huffbench runs and verifies with the
pool at 1 MiB, which `--sim-args` passes through. Nothing in the generated
code depends on the stack address.

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
