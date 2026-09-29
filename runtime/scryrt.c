/*
 * Minimal freestanding runtime for programs compiled with scry-cc.
 *
 * Programs running on the Scry simulator have no operating system, so this
 * file provides the handful of C library functions that compilers emit calls
 * to on their own (memcpy, memset, memmove) plus a few commonly used string
 * functions and a trivial bump allocator.
 *
 * It must be compiled with `-fno-builtin` (scry-cc does this) so that the
 * optimizer does not turn the copy loops back into calls to memcpy.
 */

#include <stddef.h>
#include <stdint.h>

#ifndef SCRY_HEAP_SIZE
#define SCRY_HEAP_SIZE 8192
#endif

void *memcpy(void *dst, const void *src, size_t n)
{
    unsigned char *d = dst;
    const unsigned char *s = src;
    while (n--)
        *d++ = *s++;
    return dst;
}

void *memmove(void *dst, const void *src, size_t n)
{
    unsigned char *d = dst;
    const unsigned char *s = src;
    if (d == s || n == 0)
        return dst;
    if (d < s) {
        while (n--)
            *d++ = *s++;
    } else {
        d += n;
        s += n;
        while (n--)
            *--d = *--s;
    }
    return dst;
}

void *memset(void *dst, int c, size_t n)
{
    unsigned char *d = dst;
    while (n--)
        *d++ = (unsigned char)c;
    return dst;
}

int memcmp(const void *a, const void *b, size_t n)
{
    const unsigned char *p = a;
    const unsigned char *q = b;
    for (; n; n--, p++, q++) {
        if (*p != *q)
            return (int)*p - (int)*q;
    }
    return 0;
}

size_t strlen(const char *s)
{
    size_t n = 0;
    while (s[n])
        n++;
    return n;
}

int strcmp(const char *a, const char *b)
{
    while (*a && *a == *b) {
        a++;
        b++;
    }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}

int strncmp(const char *a, const char *b, size_t n)
{
    for (; n; n--, a++, b++) {
        if (*a != *b || *a == '\0')
            return (int)(unsigned char)*a - (int)(unsigned char)*b;
    }
    return 0;
}

char *strcpy(char *dst, const char *src)
{
    char *d = dst;
    while ((*d++ = *src++))
        ;
    return dst;
}

char *strncpy(char *dst, const char *src, size_t n)
{
    size_t i = 0;
    for (; i < n && src[i]; i++)
        dst[i] = src[i];
    for (; i < n; i++)
        dst[i] = '\0';
    return dst;
}

char *strcat(char *dst, const char *src)
{
    strcpy(dst + strlen(dst), src);
    return dst;
}

char *strchr(const char *s, int c)
{
    for (;; s++) {
        if (*s == (char)c)
            return (char *)s;
        if (*s == '\0')
            return NULL;
    }
}

int abs(int v)
{
    return v < 0 ? -v : v;
}

/* A bump allocator over a fixed-size heap; free() does nothing. */
static unsigned char scry_heap[SCRY_HEAP_SIZE] __attribute__((aligned(16)));
static size_t scry_heap_used;

void *malloc(size_t n)
{
    size_t aligned = (n + 15) & ~(size_t)15;
    if (aligned == 0)
        aligned = 16;
    if (aligned > SCRY_HEAP_SIZE - scry_heap_used)
        return NULL;
    void *p = scry_heap + scry_heap_used;
    scry_heap_used += aligned;
    return p;
}

void *calloc(size_t nmemb, size_t size)
{
    size_t total = nmemb * size;
    if (size != 0 && total / size != nmemb)
        return NULL;
    void *p = malloc(total);
    if (p)
        memset(p, 0, total);
    return p;
}

void *realloc(void *old, size_t n)
{
    /* Without allocation metadata the old block cannot be extended in
       place; copy conservatively (the old size is unknown, so copy n
       bytes, which may over-read the last block but stays inside the
       heap). */
    void *p = malloc(n);
    if (p && old) {
        size_t avail = (size_t)((scry_heap + SCRY_HEAP_SIZE) - (unsigned char *)old);
        memcpy(p, old, n < avail ? n : avail);
    }
    return p;
}

void free(void *p)
{
    (void)p;
}

void abort(void)
{
    __builtin_trap();
}
