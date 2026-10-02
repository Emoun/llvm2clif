// Sub-word conversions of 32-bit values that are then used as signed: an
// `int` parameter (unsigned ABI tag) truncated to `signed char`/`short` and
// passed to a function taking `signext` parameters, or sign-extended again
// (`trunc` followed by `sext` or a signed compare). The Scry backend's type
// analysis used to panic on this pattern in most compiles
// (docs/backend-issues/README.md, issue 7, fixed).
// CASES: 1 2 3 4 => 39
// CASES: -1 -2 -3 -4 => 2009
// CASES: 100 -100 7 65535 => 2984
// CASES: -2147483648 2147483647 0 -1 => 20
// CASES: 0 0 0 0 => 25
// CASES: -50 250 -300 1000 => -1545
typedef signed char i8;
typedef short i16;

__attribute__((noinline)) static i8 sc_add(i8 a, i8 b) { return (i8)(a + b); }

__attribute__((noinline)) static i16 ss_mix(i16 a, unsigned short b) { return (i16)(a - (i16)b); }

int test(int a, int b, int c, int d)
{
    unsigned r = 0;
    r += (unsigned)sc_add((i8)a, (i8)b) * 5u + (unsigned)ss_mix((i16)c, (unsigned short)d);
    i8 ca = (i8)(a + 100);
    i16 sb = (i16)(b * 3);
    r += (ca < 0 ? 1000u : 0u) + (sb < -5 ? 2000u : 0u) + (unsigned)(ca >> 2) + (unsigned)(sb >> 5);
    return (int)r;
}
