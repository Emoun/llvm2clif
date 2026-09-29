// Recursion, mutual recursion and deeper call chains.
// CASES: 5 0 0 0 => 556
// CASES: 10 0 0 0 => 1269
// CASES: 0 0 0 0 => 307
// CASES: 6 4 0 0 => 783
static int fib(int n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
static int is_odd(int n);
static int is_even(int n) { return n == 0 ? 1 : is_odd(n - 1); }
static int is_odd(int n) { return n == 0 ? 0 : is_even(n - 1); }
static int ack(int m, int n)
{
    if (m == 0) return n + 1;
    if (n == 0) return ack(m - 1, 1);
    return ack(m - 1, ack(m, n - 1));
}
static int gcd(int a, int b) { return b == 0 ? a : gcd(b, a % b); }

int test(int a, int b, int c, int d)
{
    return fib(a) * 10 + is_even(a) + ack(2, a & 3) * 100 + gcd(a * 6 + 12, b * 4 + 18);
}
