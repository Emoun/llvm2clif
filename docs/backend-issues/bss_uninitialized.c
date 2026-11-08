/* Fixed in scryer 01216f0 (scry_sim 8269ecf); kept as a regression check.
 * Reproducer for the scryer .bss issue (see README.md in this directory,
 * issue 3). `counter` lives in .bss; the simulator loaded NOBITS segments as
 * uninitialized memory and faulted on the first read.
 *
 *   scry-cc bss_uninitialized.c -o bss.elf && scryer bss.elf    ; 5u32
 */
int counter;
int main(void) { counter += 5; return counter; }
