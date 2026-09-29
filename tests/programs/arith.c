// Integer arithmetic on mixed widths and signedness.
// CASES: 7 3 0 0 => -29667
// CASES: -7 3 0 0 => 1566008436
// CASES: 100 -9 0 0 => -30838
// CASES: -2147483647 -1 0 0 => -201356390
// CASES: 123456 789 0 0 => 98408901
int test(int a, int b, int c, int d)
{
    unsigned ua = (unsigned)a, ub = (unsigned)b;
    int r = 0;
    r += (int)(ua + ub);
    r += (int)(ua - ub);
    r += (int)(ua * ub);
    if (b != 0 && !(a == -2147483647 - 1 && b == -1)) {
        r += a / b;
        r += a % b;
        r += (int)(ua / ub);
        r += (int)(ua % ub);
    }
    r += a >> 3;
    r += (int)(ua >> 5);
    r += (int)(ua << 2);
    r += (a & b) + (a | b) + (a ^ b);
    r += ~a;
    r += (int)(0u - ub);
    unsigned short us = (unsigned short)a;
    short ss = (short)b;
    unsigned char uc = (unsigned char)a;
    signed char sc = (signed char)b;
    r += us + ss + uc + sc;
    r += (us * 3) >> 1;
    r += sc * uc;
    r += (int)((unsigned char)(uc + 200));
    r += (short)(ss - 30000);
    return r;
}
