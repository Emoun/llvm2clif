// Sorting and searching arrays; nested loops.
// CASES: 0 0 0 0 => 21099
// CASES: 7 3 0 0 => 21110
// CASES: 13 -5 0 0 => 21090
static void bubble(int *a, int n)
{
    for (int i = 0; i < n - 1; i++)
        for (int j = 0; j < n - 1 - i; j++)
            if (a[j] > a[j + 1]) { int t = a[j]; a[j] = a[j + 1]; a[j + 1] = t; }
}
static int bsearch_int(const int *a, int n, int key)
{
    int lo = 0, hi = n - 1;
    while (lo <= hi) {
        int mid = lo + (hi - lo) / 2;
        if (a[mid] == key) return mid;
        if (a[mid] < key) lo = mid + 1; else hi = mid - 1;
    }
    return -1;
}
int test(int a, int b, int c, int d)
{
    int arr[20];
    unsigned seed = (unsigned)a * 2654435761u + 12345u;
    for (int i = 0; i < 20; i++) { seed = seed * 1103515245u + 12345u; arr[i] = (int)(seed >> 16) % 100 + b; }
    bubble(arr, 20);
    int r = 0;
    for (int i = 1; i < 20; i++) r += (arr[i - 1] <= arr[i]);
    r += bsearch_int(arr, 20, arr[7]) == 7 || arr[bsearch_int(arr, 20, arr[7])] == arr[7];
    r += bsearch_int(arr, 20, 1000) == -1;
    return r * 1000 + arr[0] + arr[19];
}
