// memcpy/memset/memmove (inline expansions and runtime calls) and struct copies.
// CASES: 3 0 0 0 => 1192
// CASES: 0 5 0 0 => 2569
// CASES: 100 200 0 0 => 65169
#include <string.h>
struct Rec { int id; char name[12]; int scores[8]; };

int test(int a, int b, int c, int d)
{
    struct Rec r1;
    memset(&r1, 0, sizeof r1);
    r1.id = a;
    for (int i = 0; i < 8; i++) r1.scores[i] = a * i + b;
    strcpy(r1.name, "record");
    struct Rec r2 = r1;
    r2.scores[3] += 7;
    int buf[40];
    for (int i = 0; i < 40; i++) buf[i] = i + a;
    memmove(buf + 5, buf, 20 * sizeof(int));
    memmove(buf, buf + 10, 25 * sizeof(int));
    char small[8];
    memcpy(small, "abcdefg", 8);
    unsigned char big[300];
    memset(big, (unsigned char)b, sizeof big);
    int s = 0;
    for (int i = 0; i < 300; i++) s += big[i];
    for (int i = 0; i < 40; i++) s += buf[i];
    s += r2.scores[3] - r1.scores[3] + r1.id + strlen(r2.name) + small[3];
    s += memcmp(&r1, &r2, sizeof r1) != 0;
    return s;
}
