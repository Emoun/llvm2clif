// Signed and unsigned uses of the same values, which the Scry backend has to
// tag both ways: loop counters compared as signed and used as array indices,
// values compared both ways in one function, arithmetic versus logical
// shifts, signed division, and sub-word values loaded from memory. Every
// case has inputs for which a wrong signedness gives a different result.
// (Sub-word conversions of 32-bit values are in subword.c and abs/min/max
// in absminmax.c, which the backend does not handle yet.)
// CASES: 1 2 3 4 => 5921
// CASES: -1 -2 -3 -4 => 31393
// CASES: 100 -100 7 65535 => 52407
// CASES: -2147483648 2147483647 0 -1 => -613502527
// CASES: 0 0 0 0 => 64557
// CASES: -50 250 -300 1000 => 33119
/* The classic conflict: `i` is compared as a signed integer and used as an
   address offset. */
__attribute__((noinline)) static int idx_sum(const int *p, int n)
{
    unsigned s = 0;
    for (int i = 0; i < n; i++)
        s += (unsigned)p[i] * (unsigned)(i - n / 2);
    return (int)s;
}

__attribute__((noinline)) static unsigned both_ways(int x, int y)
{
    unsigned r = 0;
    if (x < y) r += 1;                              /* signed compare */
    if ((unsigned)x < (unsigned)y) r += 2;          /* unsigned compare, same values */
    if (x >> 3 == y) r += 4;                        /* arithmetic shift */
    if ((unsigned)x >> 3 == (unsigned)y) r += 8;    /* logical shift */
    r += (unsigned)(x / 7) * 16 + (unsigned)(x % 7) * 256;  /* signed division */
    r += ((unsigned)x / 7u) & 0xff;                 /* unsigned division */
    r += (unsigned)(x >> 28) & 0xf;                 /* sign bits through an arithmetic shift */
    return r;
}

/* A signed counter that runs below zero and indexes memory. */
__attribute__((noinline)) static unsigned count_down(int n, unsigned char *buf)
{
    unsigned h = 0;
    for (int i = n; i >= -3; i--) {
        buf[(i + 3) & 15] = (unsigned char)i;
        h = h * 31 + (unsigned)i;
    }
    return h;
}

int test(int a, int b, int c, int d)
{
    int arr[16];
    unsigned char buf[16];
    for (int i = 0; i < 16; i++)
        arr[i] = (int)((unsigned)(i - 8) * (unsigned)a + (unsigned)b);
    unsigned r = (unsigned)idx_sum(arr, 16);
    r += (unsigned)idx_sum(arr + 4, (c & 7) + 1);
    r += both_ways(a, b) + both_ways(b, c) + both_ways((int)(0u - (unsigned)a), d);
    r += count_down(a & 15, buf) & 0xffff;
    r += buf[3] + buf[7] * 3u;
    /* Sub-word values in memory, loaded back signed and unsigned. */
    signed char sc[4];
    short sh[2];
    sc[0] = (signed char)a; sc[1] = (signed char)(b >> 1); sc[2] = (signed char)c; sc[3] = (signed char)d;
    sh[0] = (short)b; sh[1] = (short)(c * 3);
    r += (sc[0] < 0 ? 100u : 0u) + (unsigned)(sc[1] >> 2) + (unsigned)(sc[2] * sc[3]);
    r += (sh[0] < -5 ? 2000u : 0u) + (unsigned)(sh[1] >> 5) + (unsigned)(unsigned char)sc[0] * 3u;
    unsigned char uc = (unsigned char)a;
    r += uc > 200 ? 7u : 0u;
    /* Signed and unsigned orders differ for these operands. */
    r += (int)(a ^ 0x80000000u) < b ? 11u : 13u;
    return (int)r;
}
