// abs, smin and smax (clang turns the ternaries into the llvm.abs/smin/smax
// intrinsics, which llvm2clif translates to iabs/smin/smax). The Scry backend
// gets the INT_MIN case wrong, and the isolated function does not run at all
// on the simulator (see docs/backend-issues/README.md, issue 9).
// CASES: 1 2 3 4 => 50
// CASES: -1 -2 -3 -4 => 50
// CASES: 100 -100 7 65535 => 3207
// CASES: -2147483648 2147483647 0 -1 => 1040
// CASES: 0 0 0 0 => 50
// CASES: -50 250 -300 1000 => 3600
__attribute__((noinline)) static int clamp_abs(int x)
{
    int a = x < 0 ? (int)(0u - (unsigned)x) : x;
    return a > 1000 ? 1000 : (a < 10 ? 10 : a);
}

int test(int a, int b, int c, int d)
{
    unsigned r = (unsigned)clamp_abs(a) + (unsigned)clamp_abs((int)((unsigned)b - (unsigned)c));
    r += (unsigned)clamp_abs(d) * 3u;
    return (int)r;
}
