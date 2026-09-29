// Dense and sparse switch statements.
// CASES: 0 0 0 0 => 10022
// CASES: 3 7 0 0 => 15100
// CASES: 5 100 0 0 => 15221
// CASES: 9 1000 0 0 => -679
// CASES: -1 -1 0 0 => 1400
static int dense(int v)
{
    switch (v) {
    case 0: return 10;
    case 1: return 11;
    case 2: return 12;
    case 3: return 13;
    case 4: return 14;
    case 5: return 15;
    case 7: return 17;
    case 8: return 18;
    default: return -1;
    }
}

static int sparse(int v)
{
    switch (v) {
    case 7: return 1;
    case 100: return 2;
    case 1000: return 3;
    case -1: return 4;
    case 65536: return 5;
    default: return 0;
    }
}

static int fallthrough(int v)
{
    int r = 0;
    switch (v & 3) {
    case 0: r += 1; /* fall through */
    case 1: r += 10; break;
    case 2: r += 100; /* fall through */
    default: r += 1000;
    }
    return r;
}

int test(int a, int b, int c, int d)
{
    return dense(a) * 1000 + sparse(b) * 100 + fallthrough(a) + fallthrough(b);
}
