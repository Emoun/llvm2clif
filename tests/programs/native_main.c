/* Host-side wrapper used by update_expected.py to compute the expected
 * results of the test programs natively. Not used on Scry. */
#include <stdio.h>
#include <stdlib.h>

int test(int, int, int, int);

int main(int argc, char **argv)
{
    int v[4] = {0, 0, 0, 0};
    for (int i = 0; i < 4 && i + 1 < argc; i++)
        v[i] = (int)strtol(argv[i + 1], 0, 10);
    printf("%d\n", test(v[0], v[1], v[2], v[3]));
    return 0;
}
