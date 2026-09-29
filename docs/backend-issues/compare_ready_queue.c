// Comparisons, booleans and selects.
// CASES: 1 2 3 4 => 24960
// CASES: -1 1 0 0 => 23203
// CASES: 2147483647 -2147483648 5 5 => 31124
// CASES: 0 0 0 0 => 8986
// CASES: -5 -5 200 100 => 37934
int test(int a, int b, int c, int d)
{
    unsigned ua = (unsigned)a, ub = (unsigned)b;
    int r = 0;
    r += (a < b) * 1;
    r += (a <= b) * 2;
    r += (a > b) * 4;
    r += (a >= b) * 8;
    r += (a == b) * 16;
    r += (a != b) * 32;
    r += (ua < ub) * 64;
    r += (ua > ub) * 128;
    r += (ua <= ub) * 256;
    r += (ua >= ub) * 512;
    r += (c < d ? c : d) * 3;
    r += (c > d ? c : d) * 5;
    _Bool p = a < 0;
    _Bool q = b < 0;
    r += (p && q) * 1024;
    r += (p || q) * 2048;
    r += (p != q) * 4096;
    r += (!p) * 8192;
    signed char sa = (signed char)a, sb = (signed char)b;
    r += (sa < sb) * 16384;
    unsigned char uca = (unsigned char)c;
    r += (uca > 100) * 32768;
    return r;
}
