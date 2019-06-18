#include "err.h"
#include <stdlib.h>

void print_err(const char* msg) {
    fprintf(stderr, "%s\n", msg);
}

void grib_panic(const char* msg) {
    print_err(msg);
    exit(EXIT_FAILURE);
}
