/* stdio.h for scry-cc programs. The simulator has no I/O, so this only
   declares printf for code that includes the header without calling it:
   a call is rejected by llvm2clif (variadic) and not provided by the runtime. */
#ifndef SCRY_STDIO_H
#define SCRY_STDIO_H

#include <stddef.h>

#define EOF (-1)

int printf(const char *fmt, ...);

#endif
