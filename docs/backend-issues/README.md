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

Status with the backend revision llvm2clif builds against (`504ebe1`; the
issues were found on `cfd59dc`, `faad34b`, `21187c1` and `504ebe1`):

| issue | status at `504ebe1` |
|-------|---------------------|
| 1. `echo.l` forwards every queued operand | fixed in `faad34b` |
| 2. type-tag conflicts on block parameters | fixed in `faad34b` |
| 3. `.bss` loaded as uninitialized memory | simulator unchanged, still worked around |
| 4. no 64-bit values | unchanged, lowered by the translator |
| 5. infinite loop on a reference distance over 1023 | fixed in `21187c1` |
| 6. nondeterministic code generation | open |
| 7. type-analysis panic on `ireduce` followed by a signed use | fixed in `504ebe1` |
| 8. re-tagging of `smax`/`smin` results | partially fixed in `504ebe1`: wrong code or operand routing errors in some compiles (breaks `sort.c`) |
| 9. unsigned result tag of `iabs` | fixed in `504ebe1` |
| compile time cubic in the block size | open |

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
extension attribute as unsigned): the reproducer compiles, and
`tests/programs/signedness.c` (loop counters compared as signed and used as
addresses, values compared both ways, signed shifts and division, sub-word
values in memory) passes on the simulator. The description below is kept
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

The translator used to work around this by expressing signed operations
through unsigned ones; that rewrite was removed once the backend handled
the conflicts (the remaining tag problems are issues 7 to 9).

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

**Fixed in `21187c1`** ("fixed issue with references longer than the reach
of echo.l"): the bridging echoes are now placed as far from the producer
as their reach allows, so chains of them cover any distance. Verified with
the reproducers below and with far references of 1500, 2050, 4100 and
8200 units, which compile in at most 0.7 s and return the right value on
the simulator; the 64-bit test programs `int64.c` and `int64_ops.c` now
compile (4 s and 15 s) and pass. The description below is kept for
reference.

**Reproducers:** `far_reference_1025.clif` (used to never finish) and
`far_reference_1020.clif` (the same function with one `iconst`/`iadd` pair
less). Both are one basic block in which the first parameter is consumed
only by the last instruction; the block in between is a chain of
`iconst.i32 0x12345678` (a `const` + 3 `grow` chain, 4 reference units) and
`iadd` (1 unit) pairs.

**Cause** (as of `faad34b`, `insert_ref_distances` in
`cranelift/codegen/src/isa/scry/mod.rs`): an output reference that did not
fit its field (5 bits, or 10 bits for `echo.l`) was bridged by inserting an
`EchoLong` directly after the definition and rescanning the block. The echo
sat next to the definition, so the distance its own output had to cover was
exactly the distance the definition had; beyond 1023 the next scan found the
echo's reference out of range too and bridged it with another adjacent
echo, forever.

**Compile time** still grows roughly cubically with the size of a basic
block, because the passes restart their scan of the whole block after every
insertion: a chain of 513 `iadd v, v` instructions (each value used twice)
takes 14 s and 1025 of them 112 s; the 625-instruction block of 14 chained
64-bit statements (which used to hang) takes 3.6 s and 20 statements 9.5 s.

## 6. Code generation is not deterministic

**Reproducer:** any function with more than one block or with re-tagging
casts, e.g. `type_conflict_loop.clif`:

```sh
for i in 1 2 3 4 5 6; do llvm2clif docs/backend-issues/type_conflict_loop.clif -o $i.o; done
sha256sum *.o    # six different objects
```

Of the 57 functions in the test corpus, 25 compile to several different
objects over 4 compiles; the straight-line `far_reference_*.clif` functions
always compile to the same bytes. The variants differ in the order of block
parameters and jump arguments, in which copy of a duplicated value feeds
which consumer, and in echo reference distances (see the vcode diff of two
variants of `signedness`'s `idx_sum`, which differ in the `out` fields of
`Duplicate`s, the sources of `Reorder`s and the argument order of
`JumpTrigger`s).

**Cause:** iteration over `std` hash maps and sets, whose order is randomly
seeded per process, in `cranelift/codegen/src/isa/scry/mod.rs`: the block
worklist of `make_live_ins_explicit` (`worklist: HashSet<usize>` consumed
with `worklist.iter().next()`), which decides the parameter orders; the
grouping of re-tagging casts in `apply_cast_demands` (`demands.drain()`
into a `HashMap`, then iterated); and the `reg_blocks: HashMap<Reg,
HashSet<usize>>` worklists of the type analysis. This is why issue 7 panics
in only some compiles and why issue 8 produces wrong code in only some
compiles of larger functions (`bubble6.clif`, `sort.c`). Replacing these
with `BTreeMap`/`BTreeSet` or `IndexMap`, or sorting before iterating,
would make the output reproducible.

## 7. Type-analysis panic on `ireduce` followed by a signed use

**Fixed in `504ebe1`** ("fixed issue where type conflicts on picks weren't
being resolved"; the `Reduce` arm no longer panics and treats the input's
signedness as unconstrained): the reproducer compiles in 10 of 10 runs and
returns the right value, and `tests/programs/subword.c` passes in every
compile. The description below is kept for reference.

**Reproducer:** `reduce_sext.clif`, two instructions: `ireduce.i8` of a
`uext` parameter, then `sextend.i32` of the result. Compiled ten times it
panicked in most of them (issue 6 decides which):

```
panicked at cranelift/codegen/src/isa/scry/mod.rs:2586
Incompatible type requirements: rs_t Known(Uint(2)), rd_t Known(Int(0))
```

The `Reduce` arm of `type_analysis_phase` panics when the input is known
unsigned and the output known signed (here: the sign extension demands a
signed input), where the `Uextend`/`Sextend` arm pushes a re-tagging demand
instead. The successful compiles show the intended code: a cast to `u8`, a
cast to `i8`, then the extension. The C pattern is `(signed char)x` or
`(short)x` for an `int` x (unsigned ABI tag) followed by any signed use,
including passing it to a function with a `signext` parameter;
`tests/programs/subword.c` is such a program and fails to compile in most
runs.

## 8. The result of `smax`/`smin` keeps the unsigned tag of its operands

**Partially fixed in `504ebe1`** ("fixed issue where type conflicts on
picks weren't being resolved": the `Pick` arm of the type analysis now
pushes a hard demand on the result once both values are known with one
signedness). `minmax_chain.clif` and `bubble6.clif` are correct in 12 of 12
compiles. `sort4_minmax.clif` is still wrong in about 5 of 12 compiles
(-487 instead of -4987 for (13, -5, 0, 0)); its 20 vcode emissions fall
into 9 variants with 13 to 19 re-tagging casts, and a good and a bad
object differ only in echo distances and in which copy of a duplicated
value feeds which consumer. In 4 of the 12 compiles the value is right but
returned with an `i32` tag instead of the signature's `u32`.

The same compiles also produce operand routing errors: `onepass_bsearch.clif`
(a single compare-and-swap pass over 20 elements, a sortedness check and a
binary search; 216 instructions, 10 blocks) fails in about a quarter of its
compiles with "Alu2 instruction missing first operand", the Pick
instruction's `ready_peek.next().is_some()` assertion or "found Nan or Nar"
on the simulator, or returns wrong values; `tests/programs/sort.c` fails
the same way in about 40 % of its compiles. A failing object has one more
cast and one more `echo.l` than a passing one, and the instruction that
misses its operand is an `Alu2` directly after an `Echo`, so the casts
inserted for pick results appear to be placed without the operand routing
being redone for them. The original description follows.

**Reproducers:** `minmax_chain.clif` (three instructions; was
deterministic), `sort4_minmax.clif` (clang's output for a bubble sort of
the four arguments; was deterministic) and `bubble6.clif` (a six-element
bubble sort; was wrong in about half of the compiles, see issue 6).

```sh
llvm2clif docs/backend-issues/minmax_chain.clif -o m.o
wild -m elf32scry -e minmax_chain -z noexecstack -o m.elf m.o
scryer m.elf --target=scry32-unknown-none-elf -i=4294967291u32 -i=4294967283u32
# smin(smax(-5, -13), 0): expected -5 (4294967291u32), observed 0
```

**Cause** (vcode of `minmax_chain`): `smax` on unsigned-tagged operands is
lowered as a `dup` of each operand, a `cast` to `i32` of one copy of each,
a compare of the casts and a `pick` (select) between the *uncast* copies.
The pick's output therefore carries the unsigned tag, while the type
analysis records the `smax` result as signed and inserts no cast before the
second `smin`, whose compare then runs unsigned on the machine. With
signed-tagged operands (`sext` parameters) the same chain is correct, as
are single `smax`/`smin`/`icmp`/`sshr`/`sdiv`/`sextend` on unsigned
operands (verified with one function per operation). Clang turns every
compare-and-swap into `smin`/`smax` chains, so this breaks sorting code:
`tests/programs/sort.c`.

## 9. The result of `iabs` keeps the unsigned tag on its negation path

**Fixed in `504ebe1`** (the `Pick` arm now re-tags the result): the
reproducer returns 0xC0000000 and `tests/programs/absminmax.c` passes in
every compile. The description below is kept for reference.

**Reproducer:** `iabs_min.clif`: `iabs` of a `uext` parameter followed by
`sshr` by 1, with the input INT_MIN. Expected 0xC0000000 (-1073741824:
iabs(INT_MIN) wraps to INT_MIN, tagged signed), observed 0x40000000, i.e.
the shift was logical.

**Cause** (vcode): the negation is computed as `0 - x` with an unsigned
zero on an uncast copy of `x`, and the `pick` chooses between that
unsigned difference and the uncast original, so the result is always
unsigned-tagged; the type analysis records it as signed. Only INT_MIN is
observable (any other magnitude fits in 31 bits), which is how
`tests/programs/absminmax.c` fails: `clamp(INT_MIN)` returns 1000 instead
of 10. With a `sext` parameter the same function is correct.
