// Sign extension of booleans: comparison results used as 0/-1 masks and
// mixed with their plain 0/1 form (the `sext i1` pattern of clang).
// CASES: 5 3 0 0 => 14
// CASES: -7 3 100 200 => 5281
// CASES: 0 0 255 1 => 256
// CASES: -1 -1 -128 127 => 376
static int mask(int a, int b) { return a < b ? -1 : 0; }
static unsigned char clamp8(short s)
{
    if ((unsigned short)s > 255U) {
        if (s < 0) return 0;
        else if (s > 255) return 255;
    }
    return (unsigned char)s;
}

int test(int a, int b, int c, int d)
{
    int r = mask(a, b) & 0x1234;
    r += (a > b) + -(a > b) * 3;             // plain and negated boolean
    r += ((a < 0) ? -1 : 0) ^ ((b < 0) ? 0 : 1);
    signed char sc = (signed char)(a == b);   // 0/1 in a byte
    r += -sc + sc;
    r += clamp8((short)c) + clamp8((short)(d * 3 - 200)) * 2;
    long long wide = (a < b) ? -1LL : 0LL;    // sext i1 to i64
    r += (int)(wide >> 40) + (int)wide;
    unsigned u = (unsigned)-(a != 0);         // all ones or zero
    r += (int)(u >> 28);
    return r;
}
