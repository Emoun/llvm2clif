/* Entry point for the RISC-V builds of the benchmarks (RV32 and RV64) that
   run under qemu's user mode: set the global pointer, call main and exit
   with its result. */
    .section .text._start,"ax",@progbits
    .globl _start
_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    call main
    li a7, 93
    ecall
