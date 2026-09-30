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

## 4. No 64-bit values

The backend does not support 64-bit values yet, so llvm2clif never emits
`i64` (or any Cranelift type wider than `i32`): LLVM integers of 33 to 128
bits are lowered to 32-bit parts by the translator (`src/translate/wide.rs`),
and 64-bit division goes through a helper function that llvm2clif adds to
each object file. (An earlier revision of these notes reported `i64` as
working because small cases compiled and ran; that mapping is no longer
used.)

## 5. Compile time explodes with the size of a straight-line function

**Reproducers:** `compile_cliff_13.clif` and `compile_cliff_14.clif`, the
translations of a function with 13 and 14 chained `unsigned long long`
multiply/shift/add statements (`compile_cliff_14.c` is the 14-statement
source; the two differ by one statement). Both are a single basic block of
32-bit instructions only.

```sh
llvm2clif docs/backend-issues/compile_cliff_13.clif -o ok.o    # about 3 s
llvm2clif docs/backend-issues/compile_cliff_14.clif -o hang.o  # never finishes
```

Compile times of the same function for a growing number of statements
(release build; the 32-bit version of the function, with `unsigned` instead
of `unsigned long long`, compiles in 0.05 s for 80 statements):

| statements | instructions | compile time |
|-----------:|-------------:|-------------:|
| 5          | 247          | 0.28 s       |
| 10         | 457          | 1.45 s       |
| 11         | 499          | 1.87 s       |
| 12         | 541          | 2.37 s       |
| 13         | 583          | 2.87 s       |
| 14         | 625          | > 2 minutes  |

The time is spent inside `ScryBackend::compile_function`
(`cranelift/codegen/src/isa/scry/mod.rs`) itself, not in lowering or
register allocation: a backtrace taken while it spins is

```
#0  __memset_avx512_unaligned_erms
#1  hashbrown::raw::RawTableInner::fallible_with_capacity
#2  hashbrown::raw::RawTable<(usize, u16)>::reserve_rehash
#3  <HashMap<usize, u16> as FromIterator<(usize, u16)>>::from_iter
#4  <ScryBackend as TargetIsa>::compile_function
#5  cranelift_codegen::context::Context::compile_stencil
#6  cranelift_codegen::context::Context::compile
#7  cranelift_object::backend::ObjectModule::define_function_with_control_plane
```

i.e. a `HashMap<usize, u16>` is rebuilt from an iterator over and over
inside a loop of the scheduling phase, whose cost grows super-linearly with
the number of instructions and live values of the block (the 64-bit
lowering produces many values with several uses each: carries, partial
products and the spilled bits of shifts). This is what makes the test
programs `int64.c` (`test`: 680 instructions) and `int64_ops.c` (`test`:
1550 instructions) fail to compile end to end; every function in them of
ordinary size compiles quickly, and both programs pass in the Cranelift
interpreter.
