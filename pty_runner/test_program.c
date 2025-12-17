#include <stdio.h>

int main(void)
{
    int mode, num;

    // Read mode from stdin
    if (scanf("%d", &mode) != 1)
    {
        printf("Error: Invalid mode\n");
        return 1;
    }

    // Consume newline
    getchar();

    if (mode == 1)
    {
        // Mode 1: Simple calculation
        if (scanf("%d", &num) != 1)
        {
            printf("Error reading number\n");
            return 1;
        }
        printf("Result: %d\n", num * 2);
    }

    return 0;
}
