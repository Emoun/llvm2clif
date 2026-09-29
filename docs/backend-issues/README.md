# Known issues in the Scry backend and simulator

While developing llvm2clif, several problems surfaced in the Scry Cranelift
backend ([Scry-arch/rust-wasmtime](https://github.com/Scry-arch/rust-wasmtime),
commit `cfd59dc`) and the simulator ([Scry-arch/scryer](https://github.com/Scry-arch/scryer),
commit `6fc6ac9`). They are outside this repository, but they determine which
programs run correctly end to end, so they are documented here together with
minimal reproducers. The reproducer `.clif` files can be compiled directly:

```sh
llvm2clif docs/backend-issues/echo_long_4params.clif -o repro.o
wild -m elf32scry -e echo_long_4params -z noexecstack -o repro.elf repro.o
scryer repro.elf --target=scry32-unknown-none-elf -i=7u32 -i=3u32 -i=0u32 -i=0u32
```

The translator's own correctness is checked independently of the backend by
running the translated CLIF in Cranelift's interpreter (`cargo test --test
interp`), which passes for every program that the interpreter can run.

## 1. `echo.l` forwards every queued operand (wrong results)

**Reproducer:** `echo_long_4params.clif` (expected 36, observed 30);
`echo_long_2params.clif` is the same function without the two unused
parameters and computes the right result.

When a value has to travel further than the 31-instruction reach of an output
reference, the backend bridges the distance with an `echo.l`. The ISA
specification (section "Data flow", *Echo long*) says that `echo.l` forwards
**all** of its inputs, and the simulator implements it that way. The backend's
expansion (`receive_chain` / `expand_echoes` in `isa/scry/mod.rs`) however
places the `echo.l` right after an `echo` with `s=1`, whose remaining inputs
are delivered to the next instruction, i.e. to the `echo.l` itself. Those
values then travel along with the long-forwarded one and arrive at its
consumer, which uses the wrong operands.

The pattern is common: any function whose first instruction receives several
values (arguments, or the results of a call) of which one is consumed far
away. In the test corpus it breaks `arith`, `chars`, `recursion`, `globals`,
`malloc` and `loops` (all of which pass in the Cranelift interpreter).

Observed variants of the same routing problem: the simulator reporting
`Cast instruction getting no inputs`, `Store instruction got no operands`
or `Cannot store`, and the assertion `ready_peek.next().is_some()` in
`scry_sim/src/execution.rs` (reproducer: `compare_ready_queue.clif`, the
translation of `compare_ready_queue.c`).

## 2. Type-tag conflicts on block parameters (compile-time panic)

**Reproducer:** `type_conflict_loop.clif` (translation of
`type_conflict_loop.c` with `llvm2clif --native-signed-ops`):

```
panicked at cranelift/codegen/src/isa/scry/mod.rs:2468
called `Option::unwrap()` on a `None` value
```

Scry values carry a signedness tag. The backend's type analysis
(`type_analysis_phase`) resolves conflicting demands within a block by
inserting re-tagging casts (`push_demand`), but when the conflicting demands
meet on a *block parameter* (in the `JumpTrigger` arm, the fold over the edge
dependencies calls `refine(..).unwrap()`) it panics instead. The classic
trigger is a loop counter that is compared as a signed integer and also used
in address arithmetic (unsigned).

**Workaround in llvm2clif (default):** signed comparisons, arithmetic shifts,
sign extensions, `abs`, `smax`/`smin` are expressed through unsigned
operations (flipping the sign bit), and all function arguments and results
use the unsigned ABI tag, so that signed demands rarely reach block
parameters. `--native-signed-ops` disables the rewrite.

## 3. `.bss` is loaded as uninitialized memory (simulator)

**Reproducer:** `bss_uninitialized.c`.

`scryer` creates the memory of each `PT_LOAD` segment with
`BlockedMemory::add_block_zeroed`, which marks the bytes as *uninitialized*,
and then writes only the `p_filesz` bytes that exist in the file. Reading a
zero-initialized global (which the linker places in `.bss`, i.e. in the
file-less tail of the segment) therefore fails in `read_data`, and the
simulator hits `todo!()` in `Executor::perform_load`. Per the ELF
specification the tail of a segment beyond `p_filesz` is zero-filled.

**Workaround in llvm2clif:** zero-initialized data is emitted as explicit
zero bytes, so it lands in `.data` instead of `.bss`.

## 4. Notes on `i64`

`i64` values, arithmetic, division, shifts and memory accesses work in the
backend and the simulator, so llvm2clif maps LLVM `i64` to Cranelift `i64`
(needed anyway: `opt` merges adjacent 32-bit stores into 64-bit ones).

## 5. Compile-time hang on a function with many 64-bit operations

**Reproducer:** `i64_compile_hang.clif` (the translation of
`tests/programs/int64.c`): `llvm2clif i64_compile_hang.clif -o out.o` spins
forever in the backend (100 % CPU, no output). Each of the 64-bit operations
in the function compiles and runs correctly on its own (the translator's
tests cover them in the interpreter, and most of them were also checked on the
simulator), so the hang is triggered by their combination in one function.
