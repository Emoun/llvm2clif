// Structs by value (small and large), struct returns, arrays of structs.
// CASES: 1 2 3 4 => 185
// CASES: -4 9 100 -100 => 20271
// CASES: 0 0 0 0 => 10
struct Pt { int x, y; };
struct Small { unsigned char a; short b; };
struct Big { int v[5]; char tag; };

static struct Pt mk(int x, int y) { struct Pt p = {x, y}; return p; }
static struct Pt add(struct Pt a, struct Pt b) { return mk(a.x + b.x, a.y + b.y); }
static int dot(struct Pt a, struct Pt b) { return a.x * b.x + a.y * b.y; }
static int sum_small(struct Small s) { return s.a * 3 + s.b; }
static int sum_big(struct Big b) { int s = b.tag; for (int i = 0; i < 5; i++) s += b.v[i]; return s; }
static struct Big make_big(int seed) { struct Big b; for (int i = 0; i < 5; i++) b.v[i] = seed + i; b.tag = (char)seed; return b; }
static void scale(struct Pt *p, int k) { p->x *= k; p->y *= k; }

int test(int a, int b, int c, int d)
{
    struct Pt p = mk(a, b), q = mk(c, d);
    struct Pt s = add(p, q);
    scale(&s, 2);
    struct Small sm = {(unsigned char)a, (short)b};
    struct Big big = make_big(c);
    struct Pt pts[3] = {{a, b}, {b, c}, {c, d}};
    int r = 0;
    for (int i = 0; i < 3; i++)
        r += dot(pts[i], s);
    return r + sum_small(sm) + sum_big(big) + s.x - s.y;
}
