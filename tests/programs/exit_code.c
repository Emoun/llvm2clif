// Plain `main` returning an exit code (checked in machine mode).
// EXIT: 42
int main(void)
{
    volatile int v = 40;
    return v + 2;
}
