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

Status with the backend revision llvm2clif builds against (`fad5e62`; the
issues were found on `cfd59dc`, `faad34b`, `21187c1`, `504ebe1` and
`a5daf64`):

| issue | status at `fad5e62` |
|-------|---------------------|
| 1. `echo.l` forwards every queued operand | fixed in `faad34b` |
| 2. type-tag conflicts on block parameters | fixed in `faad34b` |
| 3. `.bss` loaded as uninitialized memory | simulator unchanged, still worked around |
| 4. no 64-bit values | unchanged, lowered by the translator |
| 5. infinite loop on a reference distance over 1023 | fixed in `21187c1` |
| 6. nondeterministic code generation | open |
| 7. type-analysis panic on `ireduce` followed by a signed use | fixed in `504ebe1` |
| 8. re-tagging of `smax`/`smin` results | fixed in `a5daf64` |
| 9. unsigned result tag of `iabs` | fixed in `504ebe1` |
| 10. return values not re-tagged to the signature | fixed in `fad5e62` |
| 11. type-analysis panic on a function with a sub-word parameter | open |
| 12. type-analysis panic on a comparison result used through `sshr` | open |
| 13. reference distance assignment does not converge | open |
| 14. compile time explodes on nested loops with long-lived values | open |
| compile time cubic in the block size | open |
| simulator: 4 KiB stack and 64 KiB image | open (limits) |

Issues 11 to 14 and the simulator limits were found by running
[Embench-IoT](../../benchmarks/embench/README.md) on the toolchain.

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

**Fixed in `a5daf64`** (commit `cd0f4af`, "further fixes to reference
generation", which also adds a final check of the reference distances, and
stronger re-tagging rules for `pick` results): `minmax_chain.clif`,
`sort4_minmax.clif`, `bubble6.clif` and `onepass_bsearch.clif` return the
right values in 12 of 12 compiles each, and `tests/programs/sort.c` passes
in 8 of 8. What remains is the tag of the returned value, issue 10. The
history of the issue follows.

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

## 10. Return values are not re-tagged to the signature's extension

**Fixed in `fad5e62`** ("picks getting different signedness now output
unsigned unless downstream consumers specify otherwise"): `sort4_caller.clif`
returns 2147481154 and its callee's result arrives tagged `u32` in 12 of 12
compiles, and `tests/programs/rettag.c` passes in 12 of 12. The description
below is kept for reference.

**Reproducers:** `sort4_caller.clif` (a caller of `sort4_minmax.clif`'s
function that shifts the result logically) and `tests/programs/rettag.c`
(the same in C: the caller treats the `int` result of a non-inlined
bubble sort as `unsigned`).

```sh
for i in 1 2 3 4 5 6; do
  llvm2clif docs/backend-issues/sort4_caller.clif -o $i.o
  wild -m elf32scry -e caller -z noexecstack -o $i.elf $i.o
  scryer $i.elf --target=scry32-unknown-none-elf -i=13u32 -i=4294967291u32 -i=0u32 -i=0u32
done
# expected 2147481154u32 ((0xFFFFEC85 >> 1) + (0xFFFFEC85 <u 5));
# about half of the compiles print 4294964802u32 (the arithmetic shift)
```

`sort4` is declared `-> i32 uext`, so its caller's type analysis takes the
call's result as unsigned and emits no re-tagging cast before the logical
shift. The callee, however, emits no cast before its return either: the
result arrives with whatever tag the final `iadd`'s operands carry, and
with issue 6 deciding where the casts of the `smin`/`smax` chains land,
that tag is `i32` in about half of the compiles (run `sort4` alone and the
simulator prints `-4987i32` or `4294962309u32`). The caller then executes
the shift as arithmetic. Arguments are not affected: a signed-tagged value
passed to a `uext` parameter is re-tagged at the call in 12 of 12 compiles
(tested with `smin`/`smax` results in both argument orders). The same
re-tagging at the return would close this. `rettag.c` fails in about 3 of
12 compiles, only for the cases whose sorted value is negative.

## 11. Type-analysis panic on a function with a sub-word parameter (crash)

**Reproducer:** `subword_param_typing.clif` (panics; expected to return its
second parameter).

```
function %subword_param_typing(i16 sext, i32 uext, i32 uext) -> i32 uext {
block0(v0: i16, v1: i32, v2: i32):
    return v1
}
```

The backend panics in `type_analysis` (`isa/scry/mod.rs:2074`, `called
Option::unwrap() on a None value`) where the entry block's parameter
registers are zipped with the signature and each register's type is refined
to the ABI type: one register already carries a type that conflicts with the
signature entry it is paired with. The variants tried:

| parameters | returns | result |
|------------|---------|--------|
| `i16 sext, i32 uext` | v1 | compiles |
| `i16 sext, i32 uext, i32 uext` | v1 | **panics** |
| `i16 sext, i32 uext ×3` | v1 | **panics** |
| `i16 sext, i32 uext ×3` | v3 | compiles |
| `i16 sext, i32 uext ×4` | v3 | **panics** |
| `i16 sext, i32 uext ×4` | v4 | compiles |
| `i16 sext, i32 uext ×5` | v3 or v4 | **panics** |
| `i16 sext, i32 uext ×5` | v0 (sign-extended) or v5 | compiles |
| `i32 uext ×5, i16 sext` or `i8 sext` or `i16 uext` | v3 or v4 | **panics** |
| `i32 uext ×5, i16 sext` | v5 (sign-extended) | compiles |
| `i32 uext ×4, i16 sext` | v4 (sign-extended), or v0 | compiles |
| `i32 uext ×6, i16 sext ×2` | v4 | **panics** |
| `i32 uext ×6, i16 sext ×2` | v0 | compiles |
| `i32 uext ×8` | v4 | compiles |

So an 8-, 16-bit parameter anywhere in the list, together with the use of
some (not every) 32-bit parameter, is enough; which parameter positions
fail suggests that the entry block's register list and the signature are
paired up in different orders once a sub-word parameter is present. In
Embench this stops `edn` (`codebook`, eight parameters of which two are
`short`); any C function such as `int f(short a, int b, int c) { return b; }`
is affected.

## 12. Type-analysis panic on a comparison result used through `sshr` (crash)

**Reproducer:** `cmp_result_sshr.clif` (panics; expected 2 for the input 5).

```
function %cmp_result_sshr(i32 uext) -> i8 uext {
block0(v0: i32):
    v1 = iconst.i32 -1
    v2 = icmp sgt v0, v1
    v3 = iconst.i8 0
    v4 = sshr v2, v3
    v5 = iadd v2, v4
    return v5
}
```

`resolve_instruction_types` (`isa/scry/mod.rs:2672`) insists that the result
of a comparison is a `u8` boolean, but by the time the `IntCmp` arm runs, the
signed shift has demanded a signed type for the same register, and the
conflict is unwrapped instead of being resolved with a re-tagging cast (as
other conflicts are). The second use of the comparison result (the `iadd`
here; a `select` or a second comparison in the original) is needed: with the
shift alone, or when the function returns `i8 sext`, everything is typed
signed and it compiles. Using `ishl 7` before the `sshr 7` (the original
form) fails the same way.

The pattern comes from clang's `sext i1` (`s < 0 ? 0 : 255` in Embench's
picojpeg `clamp`), which llvm2clif lowered as `ishl 7; sshr 7` of the i8
comparison result. It now lowers a one-bit sign extension as `ineg` of the
zero-extended boolean, which needs no signed operation at all and is one
instruction shorter, so translated code no longer contains the pattern; the
backend still crashes on the reproducer.

## 13. Reference distance assignment does not converge (crash)

**Reproducer:** `ref_distance_convergence.clif` (panics; expected to return
its second parameter).

```
function %ref_distance_convergence(i32 uext, i8 uext) -> i8 uext {
block0(v0: i32, v1: i8):
    jump block1

block1:
    br_table v0, block4(v1), [block2(v1, v0, v1), block4(v1), block4(v1), block4(v1), block4(v1), block4(v1), block4(v1), block4(v1), block4(v1), block4(v1)]

block2(v2: i8, v3: i32, v4: i8):
    brif v1, block4(v1), block3(v1, v0, v1)

block3(v5: i8, v6: i32, v7: i8):
    brif v1, block4(v1), block4(v1)

block4(v8: i8):
    return v8
}
```

The fixed-point loop in `compile_function` (`isa/scry/mod.rs:3827`) that
alternates `insert_ref_distances`, `widen_far_jumps` and `fix_orderings`
gives up after 16 rounds with `Reference distance assignment did not
converge`: widening a jump inserts instructions, which changes the
reference distances, which changes the orderings, and for this shape the
two keep undoing each other. Found in Embench's picojpeg
(`pjpeg_decode_init` at `-O2` and `-Os`, `pjpeg_decode_mcu` at `-O1`; the
uninlined build compiles). The function was cut down automatically with
`tools/reduce_clif.py` (instructions) and `tools/reduce_clif_blocks.py`
(blocks) and then simplified by hand. Three things are needed together:

* a `br_table` outside the entry block with 10 to 12 entries (2 to 9 and
  16 entries compile, so it is about the distance the table jump has to
  cover, not the table itself);
* a first target with three block parameters (two compile);
* that target branching on to a second three-parameter block.

Whether the block arguments are function parameters or constants does not
matter; a 10-entry table in the entry block compiles unless the arguments
are constants defined in that block.

## Simulator limits: 4 KiB stack, 64 KiB image

`scryer` places the stack at address `0x10000` with a 4 KiB buffer
(`stack_base = 1 << 16; stack_buffer = 1 << 12` in `src/lib.rs`), and
stack reservations are powers of two. Two Embench benchmarks do not fit:

* `huffbench`: `compdecomp` has about 5 KiB of local arrays; the simulator
  panics at start-up with `TODO: Reserve inadequate buffer`
  (`scry_sim/src/execution.rs:359`).
* `nsichneu`: the program image is 68 KiB, so the loaded segments overlap
  the stack block and the simulator panics with `assertion failed:
  self.blocks.iter().all(...)` (`scry_sim/src/memory.rs:305`).

With the two constants raised (a 1 MiB stack at 16 MiB), both benchmarks
run and verify. Neither panic is a clean error message.

## Compile time: Embench data points

The cubic compile time (above) determines which Embench benchmarks are
practical. `aha-mont64` (`benchmark_body`: one basic block of 1006
instructions after inlining and the 64-bit lowering) and `nettle-sha256`
(`_nettle_sha256_compress`: a block of 837 instructions, the rounds being
unrolled by hand in the source) did not finish compiling within two hours; the
other benchmarks compile in 0.2 to 8.5 seconds, and `aha-mont64` compiles
in 0.6 seconds when inlining is disabled.

Block length alone does not explain `aha-mont64`: straight-line 64-bit
code generated for the purpose (`a = a * b + i` chains, with or without a
reused operand or a call in every step) compiles in these times,

| instructions in the block | compile time |
|---------------------------|--------------|
| 227 | 0.25 s |
| 451 | 1.8 s |
| 675 | 5.8 s |
| 899 | 14 s |
| 1347 | 45 s |

about cubic, so a 1006-instruction chain would take 20 seconds, not 30
minutes. `montmul` from the same benchmark (311 instructions in one block)
compiles in 0.44 seconds. What makes the inlined loop body two orders of
magnitude slower is issue 14.

## 14. Compile time explodes on nested loops with long-lived values

**Reproducer:** `compile_time_nested_loops.clif` (14 blocks, 30
instructions; not compiled within 15 minutes).

The function is the skeleton of aha-mont64's `benchmark_body`: an outer
loop around a middle loop around a chain of six self-looping blocks with
six parameters each, a few values defined before the loops and consumed
in the innermost block and after the middle loop, and the outer loop's
latch (`brif v17, block13(v82), block2(v82, v16)`) testing and passing a
*second* pair of zero constants defined in the entry block. The
reduction was automatic (`tools/reduce_clif.py` and
`tools/reduce_clif_blocks.py` with "compiles for more than 10 seconds" as
the predicate), followed by ablation of the result:

| change | compile time |
|--------|--------------|
| as is | > 900 s |
| middle loop removed (block10 always goes to block11) | 0.16 s |
| outer loop removed (block12 always returns) | 0.06 s |
| self-loops removed (the chain is straight) | 0.04 s |
| block9's or block11's arithmetic removed | 0.1 s |
| latch uses v3 and v2 instead of v17 and v16 | 0.15 s |
| latch uses v3 and v16, or v17 and v2 | 0.3 to 0.5 s |
| v17 = 1 instead of 0 | > 60 s |

So two extra values that are defined at the entry and live across every
loop until the outermost latch turn a sub-second compile into one that
does not finish, while either one alone is harmless. In `benchmark_body`
the corresponding values are the loop bound and a counter.
