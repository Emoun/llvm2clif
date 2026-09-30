// 64-bit integer arithmetic (long long): the Scry backend has no 64-bit values, so
// llvm2clif lowers these to pairs of 32-bit operations.
// CASES: 1 2 3 4 => 31625
// CASES: -7 3 100 -100 => 10757
// CASES: 123456789 987654321 0 0 => 28430810
// CASES: -2147483648 2147483647 1 2 => -1073710126
typedef long long i64;
typedef unsigned long long u64;

static u64 mix(u64 x) { x ^= x >> 13; x *= 0x9E3779B97F4A7C15ull; x ^= x >> 29; return x; }

int test(int a, int b, int c, int d)
{
    i64 x = (i64)a * (i64)b + c;
    u64 ux = (u64)(unsigned)a * 4294967311ull + (unsigned)b;
    i64 y = x << 7;
    i64 z = (y >> 3) + (i64)(ux >> 5);
    u64 h = mix(ux) ^ mix((u64)c * 0x100000001ull);
    int r = (int)(x & 0xffff) + (int)(x >> 32) + (int)(z % 1000) + (int)(h % 1000);
    if (x < y) r += 10000;
    if (ux > (u64)d) r += 20000;
    r += (int)((u64)x / 3ull % 100);
    r += (int)(x / (i64)(b + 5) % 100);
    r += (int)(ux % 7ull);
    r += (int)(h >> 60);
    r += __builtin_popcountll(ux) + __builtin_clzll(ux | 1) + __builtin_ctzll(ux | 0x1000000000ull);
    r += (int)(__builtin_bswap64(ux) >> 56);
    return r;
}
