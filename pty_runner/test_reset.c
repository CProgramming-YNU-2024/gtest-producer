// Test ANSI reset behavior
#include <stdio.h>

int main(void) {
    printf("\033[31mRed Text\033[m\n");
    printf("After Reset\n");
    return 0;
}
