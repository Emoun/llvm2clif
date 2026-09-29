// Global variables: arrays, strings, pointers to globals, function pointers.
// CASES: 0 0 0 0 => 146
// CASES: 3 1 0 0 => 161
// CASES: 7 2 0 0 => 179
// CASES: 5 3 0 0 => 169
static const int table[8] = {1, 1, 2, 3, 5, 8, 13, 21};
static const char *names[] = {"zero", "one", "two", "three"};
int counter = 10;
static int history[4];
const struct { int k; const char *s; } pairs[] = {{4, "four"}, {9, "nine"}};

static int inc(int v) { return v + 1; }
static int dbl(int v) { return v * 2; }
static int neg(int v) { return -v; }
static int (*const ops[3])(int) = {inc, dbl, neg};

static int len(const char *s) { int n = 0; while (s[n]) n++; return n; }

int test(int a, int b, int c, int d)
{
    counter += table[a & 7];
    history[b & 3] = counter;
    int r = counter + history[b & 3] + len(names[a & 3]) + names[1][0];
    r += ops[b % 3](a);
    r += pairs[b & 1].k + len(pairs[b & 1].s);
    return r;
}
