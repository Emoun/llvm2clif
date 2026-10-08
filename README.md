# llvm2clif

`llvm2clif` translates LLVM IR into Cranelift IR (CLIF) and compiles it for the
[Scry](https://github.com/Scry-arch) instruction set using the Scry backend of
the [Scry-arch/rust-wasmtime](https://github.com/Scry-arch/rust-wasmtime)
Cranelift fork. Together with clang, LLVM's `opt`, the
[scry-wild](https://github.com/Scry-arch/scry-wild) linker and the
[scryer](https://github.com/Scry-arch/scryer) simulator it forms a C compiler
for Scry without an LLVM backend:

```
  C source ──clang──▶ LLVM IR ──opt──▶ optimized LLVM IR ──llvm2clif──▶ Scry object ──wild──▶ ELF ──scryer──▶ result
                                                         (Cranelift IR inside)
```

The repository provides two programs:

* `llvm2clif` — the translator itself. Reads a textual LLVM IR file (`.ll`)
  and writes a Scry ELF object file (`--emit obj`, the default), textual
  Cranelift IR (`--emit clif`) or the Scry machine code as disassembly
  (`--emit asm`). A `.clif` input is compiled directly.
* `scry-cc` — a `cc`-like driver that runs the whole pipeline for you.

Everything is written in Rust and runs on Linux, macOS and Windows.

## Installation

### 1. Rust

Install Rust with [rustup](https://rustup.rs) (a recent stable toolchain is
required; the Scry Cranelift fork needs 1.95 or newer):

```sh
rustup update stable
```

### 2. clang and opt (LLVM 15 or newer)

Only the LLVM *tools* are needed, not the LLVM libraries.

* **Ubuntu / Debian:** `sudo apt install clang llvm` (for a specific version
  e.g. `clang-18 llvm-18`; then use `--clang clang-18 --opt opt-18` or put
  the versioned tools first on `PATH`). Alternatively use
  [apt.llvm.org](https://apt.llvm.org).
* **macOS:** `brew install llvm`, then add `$(brew --prefix llvm)/bin` to
  `PATH` (Apple's Xcode clang does not ship `opt`).
* **Windows:** install LLVM from the
  [GitHub releases](https://github.com/llvm/llvm-project/releases) (the
  `LLVM-<version>-win64.exe` installer or the `clang+llvm-*-x86_64-pc-windows-msvc.tar.xz`
  archive, both contain `clang.exe` and `opt.exe`), or `winget install LLVM.LLVM`,
  and add the `bin` directory to `PATH`.

### 3. The Scry linker and simulator

Both are Rust programs installed with cargo (this compiles them from
source, which takes a few minutes):

```sh
cargo install --locked --git https://github.com/Scry-arch/scryer.git scryer
cargo install --locked --git https://github.com/Scry-arch/scry-wild.git --bin wild wild-linker
```

Both binaries land in `~/.cargo/bin` (`%USERPROFILE%\.cargo\bin` on Windows),
which rustup puts on `PATH`.

### 4. llvm2clif and scry-cc

From a clone of this repository:

```sh
cargo install --path .
```

This installs `llvm2clif` and `scry-cc` into `~/.cargo/bin`. (Or run them
from the build directory: `cargo build --release` puts them in
`target/release/`.) The first build fetches the Scry Cranelift fork from git,
which is large; be patient.

## Usage

### Compiling and running a C program

```c
// sum.c
int sum(const int *p, int n) { int s = 0; for (int i = 0; i < n; i++) s += p[i]; return s; }
int main(void) { int a[4] = {1, 2, 3, 4}; return sum(a, 4) * 4; }
```

```sh
scry-cc sum.c -o sum.elf
scryer sum.elf --target=scry32-unknown-none-elf
```

prints the returned operand (`40u32`) and the simulation metrics. With
`--machine-mode` the simulator instead exits with the program's return value
as its exit code:

```sh
scryer sum.elf --target=scry32-unknown-none-elf --machine-mode; echo $?   # 40
```

`scry-cc --run` does the linking and the simulator invocation in one go;
arguments after `--` are passed to `scryer`:

```sh
scry-cc sum.c --run -- --machine-mode
```

### Passing inputs

Programs on the simulator have no operating system: no `argv`, no I/O. The
simulator delivers the values given with `-i` to the entry function as its
arguments, so a program can take its inputs as parameters of `main` (or of
whatever function is chosen as the entry point with `--entry`):

```c
int main(int a, int b) { return a * b + 1; }
```

```sh
scry-cc mul.c -o mul.elf
scryer mul.elf --target=scry32-unknown-none-elf -i=6u32 -i=7u32     # 43u32
```

Values are given with their type; llvm2clif passes and expects all integer
arguments with an *unsigned* tag (`u32` for `int`), so give negative numbers
as their two's complement value (`-i=4294967295u32` for `-1`).

### scry-cc options

```
scry-cc [OPTIONS] <INPUT>... [-- <SIMULATOR ARGS>...]
  -o <FILE>            Output file (default: <first input>.elf / .o / .clif / .ll)
  -c                   Compile to object files, do not link
  --emit-clif          Stop after translation and write textual Cranelift IR
  --emit-llvm          Stop after optimization and write LLVM IR
  -O<LEVEL>            Optimization level for clang/opt (0, 1, 2, 3, s, z; default 2)
  --target <TRIPLE>    Clang target triple (default riscv32-unknown-none-elf)
  --entry <SYMBOL>     Entry point symbol (default main)
  --no-runtime         Do not link the bundled runtime library
  --heap-size <BYTES>  Size of the runtime's malloc heap (default 8192)
  --no-gc-sections     Keep unreferenced sections when linking
  --skip-unsupported   Skip functions using unsupported constructs (warning)
  -Wl,<ARG>            Pass <ARG> to the linker
  --run                Run the linked program on the simulator
  --clang/--opt/--wild/--scryer <PATH>   Tool locations (or SCRY_CLANG, SCRY_OPT,
                       SCRY_WILD, SCRY_SCRYER environment variables; default: PATH)
  --keep-temps         Keep intermediate files (.pre.ll, .ll, .o)
  -v, --verbose        Print the commands that are run
```

Other options starting with `-` (`-I`, `-D`, `-std=`, `-W...`, ...) are passed
through to clang. Inputs may be C sources (`.c`), LLVM IR (`.ll`) or Scry
objects (`.o`); several inputs are linked together.

`scry-cc` compiles C with `--target=riscv32-unknown-none-elf`: a 32-bit
little-endian ILP32 target whose data layout matches Scry (32-bit pointers,
`int`/`long` 32 bits, `long long` 64 bits). Plain `char` is made signed
(`-fsigned-char`, like on x86) unless you pass `-funsigned-char`.

### The runtime library

Freestanding programs cannot use a C library. `scry-cc` bundles a small one
(see `runtime/scryrt.c`) that is compiled through the same pipeline and linked
into every program (`--no-runtime` disables it): `memcpy`, `memmove`,
`memset`, `memcmp`, `strlen`, `strcmp`, `strncmp`, `strcpy`, `strncpy`,
`strcat`, `strchr`, `abs`, a bump allocator (`malloc`, `calloc`, `realloc`,
`free` as a no-op) and `abort`. Its headers `<string.h>`, `<stdlib.h>`,
`<ctype.h>` (ASCII classification, inline), `<assert.h>` (a failed assertion
calls `abort`) and `<stdio.h>` (declares `printf` only, for code that includes
the header without calling it: there is no I/O on the simulator) are provided;
`<stdint.h>`, `<stddef.h>`, `<stdbool.h>`, `<stdarg.h>` and `<limits.h>` come
with clang. The compiler also emits calls to `memcpy`/`memset`/`memmove` on its
own for large copies, so keep the runtime unless you provide these yourself.

### Using llvm2clif directly

```sh
clang --target=riscv32-unknown-none-elf -march=rv32im -mabi=ilp32 -O2 -fsigned-char \
      -S -emit-llvm -Xclang -disable-llvm-passes -o prog.pre.ll prog.c
opt -O2 -S -o prog.ll prog.pre.ll
llvm2clif prog.ll -o prog.o                 # Scry object file
llvm2clif prog.ll --emit clif -o prog.clif  # Cranelift IR, for inspection
llvm2clif prog.ll --emit asm -o prog.s      # Scry code as the backend disassembles it
wild -flavor gnu -m elf32scry -e main -z noexecstack --gc-sections -o prog.elf prog.o
scryer prog.elf --target=scry32-unknown-none-elf
```

`llvm2clif` prints an error naming the function and source line when it meets
something it cannot translate; `--skip-unsupported` turns that into a warning
and drops the function. The `-flavor gnu` flag (which must come first) makes
wild read GNU ld options on every host; on macOS it would otherwise expect
ld64-style ones.

## What is supported

LLVM IR as produced by clang/opt for C, restricted to what the Scry backend
can execute:

* Integer types `i1`…`i128`: 8, 16 and 32 bits natively, other widths up
  to 32 bits emulated in the next larger type. The Scry backend has no
  64-bit values yet, so wider integers (`long long`, and the `i65`
  arithmetic `opt` creates for overflow-free 64-bit loop computations) are
  lowered to several 32-bit values with 32-bit operations: a `long long`
  is two 32-bit values (and takes two argument or return slots in calls),
  and 64-bit division and remainder call a small helper function that
  llvm2clif adds to each object file. Also pointers (32 bits), structs and
  arrays (as values, flattened; in memory with the target data layout,
  including bit-fields and packed structs).
* All integer arithmetic, bitwise and shift instructions, comparisons,
  `select`, casts, `phi`, `br`, `switch` (dense switches become jump tables),
  `unreachable`, `alloca` with constant size, `load`/`store`,
  `getelementptr`, direct and indirect calls (including functions with more
  than four arguments and multiple/aggregate return values), `byval`
  arguments, `extractvalue`/`insertvalue`, `freeze`.
* Global variables and constants with initializers (including pointers to
  other globals and functions, and constant expressions), string literals,
  function aliases.
* Intrinsics: `memcpy`/`memmove`/`memset` (expanded inline up to 64 bytes,
  otherwise calls into the runtime), `lifetime`/`dbg`/`assume` markers,
  `expect`, `trap`/`debugtrap`/`ubsantrap`, `abs`, `smax`/`smin`/`umax`/`umin`,
  `bswap`, `ctpop`, `ctlz`, `cttz`, `bitreverse`, `fshl`/`fshr`,
  `sadd/uadd/ssub/usub/smul/umul.with.overflow`, `sadd/uadd/ssub/usub.sat`,
  `scmp`/`ucmp`, `objectsize`, `is.constant`, `ptrmask`.

Not supported (reported as errors, or skipped with `--skip-unsupported`):
floating point, vector types, integers wider than 128 bits, division of
integers wider than 64 bits, most intrinsics on integers wider than 64 bits,
variadic
functions (`va_arg`), variable-length arrays and other dynamic `alloca`,
exceptions (`invoke`/`landingpad`), atomics beyond plain loads/stores,
`blockaddress` (computed goto), inline assembly, thread-local storage,
`fence`. Since there is no operating system, there is no `printf`, file or
console I/O: programs communicate through their return value and the
simulator's `-i` inputs.

### Simulator limits

`scryer` maps the program at address 0 and places the stack at address
0x10000 with 4 KiB of stack memory, so a program image (code + data) must stay
below 64 KiB and deep recursion or large local arrays overflow the stack.

## Known issues in the Scry backend and simulator

Problems found in the Scry Cranelift backend and in `scryer` are described,
with minimal reproducers, in
[docs/backend-issues/README.md](docs/backend-issues/README.md), together
with their status for the backend revision this tool builds against. The ten
issues found with the test programs are fixed in that revision, and all 23
test programs run on the simulator. Open there: three backend crashes found
with Embench (a function with a `short` parameter, a comparison result
shifted with `sshr`, and a reference-distance fixed point that does not
converge), code generation that is not deterministic (the same input
compiles to different objects, which so far only matters for
reproducibility), and compile time that grows cubically with the size of a
basic block (a thousand-instruction block takes well over a minute, and two
Embench benchmarks over half an hour). The simulator's fixed 4 KiB stack
and 64 KiB image limit stop two more benchmarks. `llvm2clif` keeps one
workaround: explicit zero bytes instead of `.bss`, which the simulator does
not zero-fill.

## Tests

```sh
cargo test                      # parser, translator and interpreter tests (no external tools)
cargo test --test e2e           # end-to-end tests on scryer (needs clang, opt, wild, scryer)
```

`tests/programs/*.c` are C programs with `// CASES: a b c d => result` lines
(expected results computed natively by `tests/programs/update_expected.py`).
`tests/interp.rs` translates their checked-in LLVM IR (`tests/programs/ll/`,
regenerated by `update_ll.py`) and runs the result in Cranelift's interpreter,
which validates the translation without the Scry backend. `tests/e2e.rs` runs
the same programs through the whole toolchain on the simulator; set
`LLVM2CLIF_REQUIRE_TOOLS=1` to make missing tools an error instead of a skip.

## Benchmarks

`benchmarks/embench/run_embench.py` builds the
[Embench-IoT](https://github.com/embench/embench-iot) suite with `scry-cc`
and runs it on the simulator; `benchmarks/embench/README.md` lists what
passes and what stops each of the others (a backend bug, a simulator limit,
compile time, or floating point).

## Why a hand-written LLVM IR parser?

`src/llvm/` parses the textual IR itself instead of using a crate, because
no published crate fits a tool that only needs `clang` and `opt` installed:

- `llvm-ir` and `inkwell` parse through `llvm-sys`, so every machine that
  builds llvm2clif needs a version-matched LLVM *development* install
  (headers, static libraries, `llvm-config`). The official Windows LLVM
  installer does not ship these, and the version must match the crate's
  feature flag rather than whatever `clang` is on `PATH`.
- `llvm-bitcode` only decodes the bitstream container, not modules,
  functions or instructions.
- The pure-Rust text parsers on crates.io (`llvmkit-asmparser` 0.0.x,
  `llvm-in-rust-ir-parser` 0.1, `omniscope-ir`, `vicis`) are either
  line-oriented analysers, pre-opaque-pointer (LLVM 14 and older), or too
  young to accept real compiler output: when this was evaluated, the two
  most complete ones parsed 5 and 0 of 237 clang 18 modules produced from
  `tests/programs` at `-O0`..`-Oz`, while the parser here accepted all 237.

The parser is about 2,700 lines, needs no build-time dependencies and
targets exactly the subset `clang`/`opt` emit (opaque pointers, LLVM 15+).
Unsupported constructs are reported as such rather than mis-parsed.

## Layout

```
src/llvm/        LLVM IR lexer, parser, IR data structures, data layout
src/translate/   translation to Cranelift IR (types, functions, intrinsics, globals)
src/emit.rs      CLIF text, Scry assembly and object file output
src/driver.rs    the scry-cc pipeline
src/main.rs      llvm2clif command line
src/bin/         scry-cc command line
runtime/         the bundled C runtime library and its headers
tests/           test programs and harnesses
benchmarks/      the Embench-IoT runner and board support
tools/           split_clif.py and reduce_clif.py, for cutting down backend reproducers
docs/            backend issue reproducers
```
