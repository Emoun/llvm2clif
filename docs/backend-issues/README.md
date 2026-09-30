# Known issues in the Scry backend and simulator

While developing llvm2clif, several problems surfaced in the Scry Cranelift
backend ([Scry-arch/rust-wasmtime](https://github.com/Scry-arch/rust-wasmtime))
and the simulator ([Scry-arch/scryer](https://github.com/Scry-arch/scryer),
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

Status with the backend revision llvm2clif builds against (`faad34b`, which
followed `cfd59dc` where the issues were found):

| issue | status at `faad34b` |
|-------|---------------------|
| 1. `echo.l` forwards every queued operand | fixed: the reproducer returns 36 and all affected test programs pass |
| 2. type-tag conflicts on block parameters | fixed: the reproducer compiles; the translator's unsigned rewrite is now optional |
| 3. `.bss` loaded as uninitialized memory | simulator unchanged, still worked around |
| 4. no 64-bit values | unchanged, lowered by the translator |
| 5. infinite loop on a reference distance over 1023 | still open: `far_reference_1025.clif` never finishes |
| cubic compile time with block size | unchanged |

## 1. `echo.l` forwards every queued operand (wrong results)

**Fixed in `faad34b`** (commits `58be9c8` and `faad34b`): the reproducer
now returns 36 and every test program that used to fail because of it
passes on the simulator. The description below is kept for reference.

**Reproducer:** `echo_long_4params.clif` (expected 36, observed 30 with
`cfd59dc`); `echo_long_2params.clif` is the same function without the two
unused parameters and computes the right result.

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

**Fixed in `faad34b`** (commit `3a4e61c`, "fixes issues with signedness
conflicts", together with the ABI now treating a parameter without an
extension attribute as unsigned): the reproducer compiles, and the whole
test corpus passes on the simulator with native signed operations, which
are therefore the translator's default now. The description below is kept
for reference.

**Reproducer:** `type_conflict_loop.clif` (translation of
`type_conflict_loop.c` with native signed operations):

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

**Workaround in llvm2clif (`--signed-via-unsigned`, formerly the default):**
signed comparisons, arithmetic shifts, sign extensions, `abs`, `smax`/`smin`
are expressed through unsigned operations (flipping the sign bit), and all
function arguments and results use the unsigned ABI tag, so that signed
demands rarely reach block parameters. It is kept for backends that still
show the problem.

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

## 5. Infinite loop when a value has to travel more than 1023 instructions

**Still open in `faad34b`.** Commit `faad34b` ("fixed forwarding values
through a long block") moves the bridging echo past the links of an echo
chain, but the bridge is still inserted directly after the producer, so
the reproducer below behaves exactly as before.

**Reproducers:** `far_reference_1025.clif` (never finishes) and
`far_reference_1020.clif` (the same function with one `iconst`/`iadd` pair
less; compiles in about 10 ms). Both are one basic block in which the first
parameter is consumed only by the last instruction; the block in between is
a chain of `iconst.i32 0x12345678` (a `const` + 3 `grow` chain, 4 reference
units) and `iadd` (1 unit) pairs.

```sh
llvm2clif docs/backend-issues/far_reference_1020.clif -o ok.o    # 10 ms
llvm2clif docs/backend-issues/far_reference_1025.clif -o hang.o  # never finishes
```

**Cause** (`insert_ref_distances` in `cranelift/codegen/src/isa/scry/mod.rs`):
an output reference that does not fit its field (5 bits, or 10 bits for
`echo.l`) is bridged by inserting an `EchoLong` *directly after the
definition* (`bb.inst.insert(bb.inst.len() - inst_idx, MInst::EchoLong ..)`
after `replace_all_uses`), and the block is then rescanned from the start
(`continue 'a`). The echo sits next to the definition, so the distance its
own output has to cover is exactly the distance the definition had; while
that is at most 1023 one `echo.l` suffices, but beyond 1023 the next scan
finds the echo's reference out of range too and bridges it with another
adjacent echo, and so on. The block grows by one instruction per iteration
and every iteration rebuilds the `use_pos` map and rescans the block, which
is what a backtrace of the spinning process shows (`compile_function` ->
`HashMap::from_iter` / `get_uses_mut`). The comment above the bridge
("chaining further echoes if even that is exceeded") describes the intent;
the chain would have to be placed so that each echo actually advances
towards the use (for example at most 1023 units before it).

**How ordinary code gets there:** `insert_duplicates` puts all the `dup`s of
a value that is used several times right after its definition, so the
reference distance of a value used throughout a block equals the block's
length in reference units, however close its individual uses are. The test
program `int64.c` compiles to about 1000 machine instructions for the
`test` function; its lowered 64-bit statements all use the same `int`
parameter, whose `dup` chain ends in an `EchoLong` with a reference of
1016 when the function has 13 such statements (2.9 s to compile) and would
need more than 1023 with 14 (never finishes; replacing the shared parameter
by a constant makes the 14-statement version compile in 3 s). This is why
`int64.c` and `int64_ops.c` cannot run end to end although they pass in the
Cranelift interpreter.

**Related performance problem:** the passes restart their scan of the whole
block after every insertion, so compile time grows roughly cubically with
block size even below the limit: a chain of 513 `iadd v, v` instructions
(each value used twice, one `dup` each) takes 13 s, 1025 of them more than
a minute, and a 386-instruction chain in which one value is used by every
instruction (386 `dup`s) already crosses the 1023 limit above.
