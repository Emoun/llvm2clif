int f9(int *p, int n) { int best = -1; for (int i = 0; i < n; i++) { if (p[i] > best) best = p[i]; } for (int i = n - 1; i >= 0; i--) p[i] = best >> 1; return best; }
