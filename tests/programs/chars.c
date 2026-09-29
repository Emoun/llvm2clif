// Byte and short manipulation, sign extension, string functions.
// CASES: 65 3 0 0 => 8700
// CASES: 200 -100 0 0 => 47152
// CASES: 0 0 0 0 => 5513
#include <string.h>
static unsigned hash(const char *s) { unsigned h = 5381; while (*s) h = h * 33 + (unsigned char)*s++; return h; }
static void upper(char *s) { for (; *s; s++) if (*s >= 'a' && *s <= 'z') *s -= 32; }
static int sum_shorts(const short *p, int n) { int s = 0; for (int i = 0; i < n; i++) s += p[i]; return s; }

int test(int a, int b, int c, int d)
{
    char buf[32];
    strcpy(buf, "hello, scry");
    upper(buf);
    buf[0] = (char)a;
    signed char sc = (signed char)b;
    unsigned char uc = (unsigned char)b;
    short sh[6] = {(short)a, (short)b, -1, 32767, -32768, 7};
    int r = (int)(hash(buf) & 0xffff);
    r += sc + uc * 2 + (sc < 0) + (uc > 100);
    r += sum_shorts(sh, 6);
    r += strcmp(buf, "HELLO, SCRY") == 0;
    r += strlen(buf) + (strchr(buf, 'S') ? 1 : 0);
    unsigned char bytes[4] = {(unsigned char)a, (unsigned char)b, 0x7f, 0x80};
    r += bytes[2] + bytes[3] + (signed char)bytes[3];
    return r;
}
