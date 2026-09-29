// Bit manipulation builtins and rotations.
// CASES: 0 0 0 0 => 161
// CASES: 1 1 0 0 => 361
// CASES: -1 255 0 0 => 135390
// CASES: 305419896 19088743 0 0 => 44569
// CASES: 65536 3 0 0 => 1145
unsigned rotl(unsigned x, int r) { r &= 31; return r ? (x << r) | (x >> (32 - r)) : x; }
int test(int a, int b, int c, int d)
{
    unsigned ua = (unsigned)a, ub = (unsigned)b;
    int r = 0;
    r += __builtin_popcount(ua);
    r += __builtin_clz(ua | 1) * 2;
    r += __builtin_ctz(ua | 0x100000) * 3;
    r += (int)(__builtin_bswap32(ua) >> 24);
    r += __builtin_bswap16((unsigned short)ub);
    r += (int)rotl(ua, b) & 0xffff;
    r += __builtin_abs(a) & 0xff;
    r += __builtin_parity(ub);
    int sum;
    if (__builtin_add_overflow(a, b, &sum)) r += 1000; else r += sum & 0xff;
    int prod;
    if (__builtin_mul_overflow(a, b, &prod)) r += 2000; else r += prod & 0xff;
    unsigned usum;
    if (__builtin_add_overflow(ua, ub, &usum)) r += 4000; else r += (int)(usum & 0xff);
    int diff;
    if (__builtin_sub_overflow(a, b, &diff)) r += 8000; else r += diff & 0xff;
    unsigned char c8 = (unsigned char)a;
    r += __builtin_popcount(c8) + __builtin_clz((unsigned)c8 | 1) + __builtin_ctz((unsigned)c8 | 0x100);
    return r;
}
