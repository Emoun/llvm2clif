// Many arguments (beyond the four operand slots), indirect calls, callbacks.
// CASES: 1 2 3 4 => 283
// CASES: -1 -2 -3 -4 => 65735
// CASES: 10 20 30 40 => 3023
static int many(int a, int b, int c, int d, int e, int f, int g, int h)
{
    return a * 1 + b * 2 + c * 3 + d * 4 + e * 5 + f * 6 + g * 7 + h * 8;
}
static int mixed(char a, short b, int c, unsigned char d, unsigned short e, int f)
{
    return a + b + c + d + e + f;
}
static int apply(int (*f)(int, int), int x, int y) { return f(x, y); }
static int sub(int x, int y) { return x - y; }
static int mul(int x, int y) { return x * y; }
static int reduce(const int *v, int n, int (*f)(int, int), int init)
{
    for (int i = 0; i < n; i++) init = f(init, v[i]);
    return init;
}
struct Ret6 { int a, b, c, d, e, f; };
static struct Ret6 six(int s) { struct Ret6 r = {s, s + 1, s + 2, s + 3, s + 4, s + 5}; return r; }

int test(int a, int b, int c, int d)
{
    int v[4] = {a, b, c, d};
    struct Ret6 r6 = six(a);
    int (*op)(int, int) = (a & 1) ? sub : mul;
    return many(a, b, c, d, a + b, b + c, c + d, d + a) + mixed((char)a, (short)b, c, (unsigned char)d, (unsigned short)a, b) +
           apply(op, c, d) + reduce(v, 4, sub, 100) + r6.a + r6.f * 2 + r6.d;
}
