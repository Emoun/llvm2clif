// More 64-bit integer coverage: shifts by variable and constant amounts
// across the 32-bit boundary, conversions, 64-bit arguments and results,
// loops with 64-bit induction variables, a switch on a 64-bit value, the
// overflow builtins, rotates, division, and 64-bit struct fields in memory.
// CASES: 1 2 3 4 => 1157
// CASES: -5 17 200 -300 => 16780914
// CASES: 123456789 -987654321 31 63 => 495757
// CASES: 2147483647 -2147483648 0 1 => 8391819
// CASES: 0 0 0 0 => 882
// CASES: -1 -1 40 33 => 16782812
typedef long long i64;
typedef unsigned long long u64;

struct pair {
    int tag;
    i64 value;
    u64 bits;
};

__attribute__((noinline)) static u64 rotl64(u64 x, unsigned n)
{
    n &= 63;
    return n ? (x << n) | (x >> (64 - n)) : x;
}

__attribute__((noinline)) static i64 sum_squares(i64 n)
{
    i64 s = 0;
    for (i64 i = 1; i <= n; i++)
        s += i * i;
    return s;
}

__attribute__((noinline)) static u64 mul_parts(u64 a, u64 b, u64 *lo)
{
    *lo = a * b;
    return (a >> 32) * (b >> 32);
}

__attribute__((noinline)) static int classify(i64 v)
{
    switch (v) {
    case 0: return 1;
    case -1: return 2;
    case 1LL << 40: return 3;
    case 123456789012345LL: return 4;
    default: return 5;
    }
}

__attribute__((noinline)) static void fill(struct pair *p, int tag, i64 v)
{
    p->tag = tag;
    p->value = v;
    p->bits = (u64)v ^ 0xA5A5A5A5A5A5A5A5ull;
}

int test(int a, int b, int c, int d)
{
    unsigned sa = (unsigned)c & 63, sb = (unsigned)d & 63;
    u64 ua = ((u64)(unsigned)a << 32) | (unsigned)b;
    i64 sa64 = (i64)ua;
    int r = 0;
    /* variable shifts */
    r += (int)((ua << sa) >> 7) & 0xff;
    r += (int)((ua >> sb) & 0xff);
    r += (int)((sa64 >> sa) & 0xff);
    r += (int)((sa64 >> sb) >> 32) & 0xff;
    /* constant shifts across the 32-bit boundary */
    r += (int)((ua << 40) >> 48) & 0xff;
    r += (int)(sa64 >> 33) & 0xff;
    r += (int)(ua >> 31) & 0xff;
    r += (int)(ua << 1 >> 33) & 0xff;
    /* conversions */
    r += (int)(u64)(unsigned char)a;
    r += (int)((i64)(short)b >> 40) & 0xff;
    r += (int)(unsigned)(ua >> 3) & 0xff;
    r += (int)((u64)(unsigned)c >> 2);
    /* 64-bit arguments, results and induction variables */
    r += (int)(sum_squares((i64)(c & 31)) & 0xffff);
    u64 lo;
    r += (int)(mul_parts(ua, ua ^ 0x1234567ull, &lo) & 0xff) + (int)(lo >> 60);
    r += (int)(rotl64(ua, sa) & 0xff) + (int)(rotl64(ua, 13) >> 56);
    /* comparisons */
    if (sa64 < (i64)b) r += 1;
    if (sa64 >= -(i64)c) r += 2;
    if (ua > (u64)d) r += 4;
    if (ua <= 0xffffffffull) r += 8;
    if ((i64)a * 1000000007LL > (i64)b * 1000000009LL) r += 16;
    i64 m = sa64 < (i64)a ? sa64 : (i64)a;
    u64 um = ua > (u64)(unsigned)b ? ua : (u64)(unsigned)b;
    r += (int)(m & 0xff) + (int)(um >> 40);
    /* overflow builtins */
    i64 sres;
    u64 ures;
    if (__builtin_add_overflow(sa64, (i64)a * 3, &sres)) r += 32; else r += (int)(sres & 0xff);
    if (__builtin_mul_overflow(ua, (u64)(unsigned)c, &ures)) r += 64; else r += (int)(ures >> 56);
    if (__builtin_mul_overflow(sa64, (i64)b, &sres)) r += 128; else r += (int)(sres & 0xff);
    if (__builtin_sub_overflow(ua, (u64)(unsigned)d, &ures)) r += 256; else r += (int)(ures & 0xff);
    /* division and remainder with 64-bit divisors */
    u64 dv = ((ua | 1) >> 20) | 1;
    r += (int)((ua / dv) & 0xff) + (int)((ua % dv) & 0xff);
    i64 sdv = (i64)(c | 1);
    r += (int)((sa64 / sdv) & 0xff) + (int)((sa64 % sdv) & 0xff);
    r += (int)((-(i64)a * 7 - 3) / (i64)((b | 1) & 0xffff)) & 0xff;
    r += (int)((u64)-1 / (ua | 3)) & 0xff;
    /* 64-bit fields in memory */
    struct pair p;
    fill(&p, a, sa64 + 5);
    r += p.tag & 0xff;
    r += (int)(p.value & 0xff) + (int)(p.bits >> 56);
    /* switch on a 64-bit value */
    r += classify(sa64) + classify(0) + classify(-1) + classify(1LL << 40) + classify(123456789012345LL);
    /* bit builtins */
    r += __builtin_clzll(ua | 1) + __builtin_ctzll(ua | (1ull << 50)) + __builtin_popcountll(ua ^ 0xff);
    r += (int)(__builtin_bitreverse64(ua) >> 60);
    r += (int)(__builtin_bswap64(ua) & 0xff);
    r += (int)(__builtin_llabs(sa64 - 1000) & 0xff);
    return r;
}
