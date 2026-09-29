/* Reproducer for the scryer .bss issue (see README.md in this directory).
 * `counter` lives in .bss; the simulator loads NOBITS segments as
 * uninitialized memory and faults on the first read.
 *
 *   scry-cc --no-bss-workaround ... (not available: llvm2clif always emits
 *   explicit zero bytes as a workaround; build the object with a stock
 *   toolchain, or place an uninitialized variable through the linker, to
 *   observe the fault.)
 */
int counter;
int main(void) { counter += 5; return counter; }
