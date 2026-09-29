// Loops, local arrays and pointer arithmetic.
// CASES: 5 0 0 0 => 668
// CASES: 1 0 0 0 => 535
// CASES: 12 3 0 0 => 479001653
// CASES: 0 7 0 0 => 490
static int fact(int n)
{
    int r = 1;
    for (int i = 2; i <= n; i++)
        r *= i;
    return r;
}

static int fib(int n)
{
    int a = 0, b = 1;
    while (n-- > 0) {
        int t = a + b;
        a = b;
        b = t;
    }
    return a;
}

int test(int n, int m, int c, int d)
{
    int arr[16];
    for (int i = 0; i < 16; i++)
        arr[i] = i * i - m;
    int s = 0;
    for (int i = 0; i < 16; i += 3)
        s += arr[i];
    int *p = arr + 16;
    while (p != arr) {
        p -= 2;
        s ^= *p;
    }
    int k = 0;
    do {
        k += n + 1;
    } while (k < 100);
    return fact(n) + fib(n) * 3 + s + k;
}
