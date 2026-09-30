typedef unsigned long long T;
int test(int a, int b, int c, int d) {
    T x = (T)(unsigned)a * 2654435761u + (T)(unsigned)b;
    x = x * (T)7922 + (x >> 2) + (T)c;
    x = x * (T)15841 + (x >> 3) + (T)c;
    x = x * (T)23760 + (x >> 4) + (T)c;
    x = x * (T)31679 + (x >> 5) + (T)c;
    x = x * (T)39598 + (x >> 6) + (T)c;
    x = x * (T)47517 + (x >> 7) + (T)c;
    x = x * (T)55436 + (x >> 8) + (T)c;
    x = x * (T)63355 + (x >> 9) + (T)c;
    x = x * (T)71274 + (x >> 10) + (T)c;
    x = x * (T)79193 + (x >> 11) + (T)c;
    x = x * (T)87112 + (x >> 12) + (T)c;
    x = x * (T)95031 + (x >> 13) + (T)c;
    x = x * (T)102950 + (x >> 1) + (T)c;
    x = x * (T)110869 + (x >> 2) + (T)c;
    return (int)x + (int)(x >> 32);
}
