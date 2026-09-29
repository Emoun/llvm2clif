// Bit-fields, unions and unaligned-looking accesses.
// CASES: 5 9 0 0 => -87
// CASES: -3 300 0 0 => 61025
// CASES: 0 0 0 0 => 0
struct Flags { unsigned a : 3; unsigned b : 5; int c : 8; unsigned d : 16; };
union U { int i; unsigned char b[4]; short s[2]; };
struct __attribute__((packed)) Packed { char c; int i; short s; };

int test(int a, int b, int c, int d)
{
    struct Flags f = {0};
    f.a = (unsigned)a;
    f.b = (unsigned)b;
    f.c = a - b;
    f.d = (unsigned)(a * b);
    union U u;
    u.i = a * 1000 + b;
    struct Packed p = {(char)a, b, (short)(a + b)};
    int r = f.a + f.b * 10 + f.c * 100 + f.d;
    r += u.b[0] + u.b[3] + u.s[1];
    r += p.c + p.i + p.s;
    return r;
}
