// String functions that LLVM rewrites into other library calls (strchr on a
// constant string becomes memchr, memcmp(...) == 0 may become bcmp).
// CASES: 42 3 0 0 => 13103
// CASES: 36 0 0 0 => 12103
// CASES: 94 7 1 1 => 10104
// CASES: 0 0 0 0 => 12101
#include <string.h>
static const char metacharacters[] = "^$().[]*+?|\\Ssdbfnrtv";
static int is_meta(int c) { return strchr(metacharacters, c) != 0; }

int test(int a, int b, int c, int d)
{
    char buf[16];
    memset(buf, 0, sizeof buf);
    buf[0] = (char)a;
    buf[1] = (char)('a' + b);
    int r = is_meta(a) * 100 + is_meta('x') * 10 + is_meta('\\');
    r += memcmp(buf, "*d", 2) == 0 ? 1000 : 0;
    r += memcmp(buf, "^", 1) == 0 ? 2000 : 0;
    r += memchr(buf, c, sizeof buf) != 0 ? 4000 : 0;
    r += bcmp("abc", "abd", 3) != 0 ? 8000 : 0;
    r += (int)strlen(buf) + d;
    return r;
}
