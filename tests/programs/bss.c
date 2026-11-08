// Zero-initialized globals (.bss): read before any write, and read back
// after a few writes. The simulator must zero-fill the file-less part of
// the data segment for these to hold.
// CASES: 0 0 0 0 => 0
// CASES: 5 3 100 7 => 31
// CASES: 1023 -1 0 2 => -259
// CASES: 77 12 300 1 => 588
static int table[1024];
static unsigned char bytes[300];
static long long wide[8];
static struct { short s; int i; char c[6]; } mixed[4];

int test(int a, int b, int c, int d)
{
    int r = table[a & 1023] + bytes[c % 300] + (int)wide[d & 7] + mixed[d & 3].i;
    table[a & 1023] += b;
    bytes[c % 300] = (unsigned char)(a + b);
    wide[d & 7] += (long long)a * b;
    mixed[d & 3].s = (short)b; mixed[d & 3].c[5] = (char)d;
    r += table[a & 1023] * 2 + bytes[c % 300] + (int)(wide[d & 7] >> 1);
    r += mixed[d & 3].s + mixed[d & 3].c[5] + mixed[(d + 1) & 3].s;
    for (int i = 0; i < 1024; i += 97) r += table[i];
    return r;
}
