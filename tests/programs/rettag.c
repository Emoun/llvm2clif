// A function result computed through smin/smax chains (clang's output for a
// bubble sort of the four arguments), used as an unsigned value by the caller.
// The callee returns `int`, which the ABI tags unsigned; the Scry backend used
// to leave the result tagged signed in some compiles, so that the caller's
// logical shift and unsigned compare ran signed (docs/backend-issues/README.md,
// issue 10, fixed). Cases with a negative sorted value expose it.
// CASES: 1 2 3 4 => 1182
// CASES: 13 -5 0 0 => -2147421127
// CASES: -9 20 -9 7 => -2147423749
// CASES: 4 3 2 1 => 1182
// CASES: -100 -200 -300 -400 => 2147279853
__attribute__((noinline)) static int sort4(int a, int b, int c, int d)
{
    int arr[4] = { a, b, c, d };
    for (int i = 0; i < 3; i++)
        for (int j = 0; j < 3 - i; j++)
            if (arr[j] > arr[j + 1]) { int t = arr[j]; arr[j] = arr[j + 1]; arr[j + 1] = t; }
    return arr[0] * 1000 + arr[1] * 100 + arr[2] * 10 + arr[3];
}

int test(int a, int b, int c, int d)
{
    unsigned u = (unsigned)sort4(a, b, c, d);
    unsigned r = (u >> 1) + (u < 5u ? 1u : 0u) + (u / 3u) % 1000u;
    unsigned v = (unsigned)sort4(d, c, b, a);
    r += (v >> 3) & 0xffff;
    return (int)r;
}
