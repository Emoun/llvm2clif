/* stdlib.h for scry-cc programs: implemented by the scry runtime. */
#ifndef SCRY_STDLIB_H
#define SCRY_STDLIB_H

#include <stddef.h>

void *malloc(size_t n);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *p, size_t n);
void free(void *p);
void abort(void);
int abs(int v);

#endif
