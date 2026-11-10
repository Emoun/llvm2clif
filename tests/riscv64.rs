//! The riscv64 target: the translation with a 64-bit word, compiled by
//! Cranelift's riscv64 backend. These tests only check that representative
//! constructs translate and compile to a RISC-V object (running the result
//! needs qemu; `benchmarks/embench/metrics.py --rv64` does that). They live
//! in their own test binary because the word size is a process-wide setting.

use llvm2clif::translate::{translate_module, OptLevel, Options, Target};

const MODULE: &str = r#"
target datalayout = "e-m:e-p:64:64-i64:64-i128:128-n32:64-S128"
target triple = "riscv64-unknown-none-elf"

@table = dso_local global [4 x i64] [i64 1, i64 2, i64 3, i64 4], align 8
@ptrs = dso_local global [2 x ptr] [ptr @table, ptr null], align 8
@buf = dso_local global [40 x i8] zeroinitializer, align 1

declare void @llvm.memcpy.p0.p0.i64(ptr, ptr, i64, i1)

define dso_local i64 @sum(ptr %p, i64 %n) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i1, %body ]
  %acc = phi i64 [ 0, %entry ], [ %acc1, %body ]
  %done = icmp eq i64 %i, %n
  br i1 %done, label %exit, label %body
body:
  %q = getelementptr inbounds i64, ptr %p, i64 %i
  %v = load i64, ptr %q, align 8
  %acc1 = add i64 %acc, %v
  %i1 = add i64 %i, 1
  br label %loop
exit:
  ret i64 %acc
}

define dso_local i64 @wide(i64 %a, i64 %b) {
entry:
  %ax = zext i64 %a to i128
  %bx = zext i64 %b to i128
  %p = mul nuw i128 %ax, %bx
  %hi = lshr i128 %p, 64
  %r = trunc i128 %hi to i64
  ret i64 %r
}

define dso_local i32 @narrow(i8 signext %c, i16 zeroext %s, i32 %x) {
entry:
  %ce = sext i8 %c to i32
  %se = zext i16 %s to i32
  %t = mul i32 %ce, %se
  %u = udiv i32 %t, 7
  %v = ashr i32 %x, 3
  %w = xor i32 %u, %v
  ret i32 %w
}

define dso_local i64 @dispatch(i64 %k) {
entry:
  switch i64 %k, label %default [
    i64 0, label %zero
    i64 1, label %one
    i64 2, label %two
    i64 3, label %three
  ]
zero:
  ret i64 10
one:
  ret i64 11
two:
  ret i64 12
three:
  ret i64 13
default:
  ret i64 -1
}

define dso_local i64 @main() {
entry:
  call void @llvm.memcpy.p0.p0.i64(ptr @buf, ptr @table, i64 32, i1 false)
  %t = load ptr, ptr @ptrs, align 8
  %s = call i64 @sum(ptr %t, i64 4)
  %w = call i64 @wide(i64 %s, i64 4611686018427387904)
  %s32 = trunc i64 %s to i32
  %n = call i32 @narrow(i8 -3, i16 1000, i32 %s32)
  %ne = sext i32 %n to i64
  %d = call i64 @dispatch(i64 %w)
  %r1 = add i64 %ne, %d
  ret i64 %r1
}
"#;

#[test]
fn translates_and_compiles_for_riscv64() {
    let module = llvm2clif::llvm::parse_module(MODULE).expect("parsing the module");
    let options = Options {
        target: Target::Riscv64,
        opt_level: OptLevel::None,
        ..Options::default()
    };
    let translated = translate_module(&module, &options).expect("translating for riscv64");
    assert_eq!(translated.target, Target::Riscv64);
    assert!(translated.skipped.is_empty());
    let text = llvm2clif::emit::clif_text(&translated);
    assert!(
        text.contains("i64"),
        "64-bit values should stay 64-bit in the CLIF"
    );
    let object = llvm2clif::emit::object_file(&translated, "t").expect("compiling for riscv64");
    // ELF64, little-endian, EM_RISCV (243).
    assert_eq!(&object[..4], b"\x7fELF");
    assert_eq!(object[4], 2, "ELF64");
    assert_eq!(object[5], 1, "little-endian");
    assert_eq!(
        u16::from_le_bytes([object[18], object[19]]),
        243,
        "EM_RISCV"
    );
    assert!(
        llvm2clif::emit::assembly_text(&translated).is_ok(),
        "disassembly works for riscv64"
    );
}

#[test]
fn rejects_32_bit_ir_for_riscv64() {
    let src = "target datalayout = \"e-m:e-p:32:32-i64:64-n32-S128\"\ndefine i32 @f() {\n  ret i32 0\n}\n";
    let module = llvm2clif::llvm::parse_module(src).expect("parsing");
    let options = Options {
        target: Target::Riscv64,
        ..Options::default()
    };
    match translate_module(&module, &options) {
        Ok(_) => panic!("32-bit IR must be rejected"),
        Err(err) => assert!(err.message.contains("32-bit pointers"), "{}", err.message),
    }
}
