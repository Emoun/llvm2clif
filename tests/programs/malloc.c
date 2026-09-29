// Heap allocation with the runtime's bump allocator; linked lists.
// CASES: 5 0 0 0 => 575
// CASES: 1 3 0 0 => 178
// CASES: 20 -2 0 0 => 2555
#include <stdlib.h>
struct Node { int value; struct Node *next; };

static struct Node *push(struct Node *head, int v)
{
    struct Node *n = malloc(sizeof *n);
    n->value = v;
    n->next = head;
    return n;
}

int test(int a, int b, int c, int d)
{
    struct Node *head = 0;
    for (int i = 0; i < a; i++) head = push(head, i * 3 + b);
    int s = 0, count = 0;
    for (struct Node *n = head; n; n = n->next) { s += n->value; count++; }
    int *arr = calloc(10, sizeof(int));
    for (int i = 0; i < 10; i++) arr[i] += i + b;
    for (int i = 0; i < 10; i++) s += arr[i];
    free(arr);
    return s + count * 100;
}
