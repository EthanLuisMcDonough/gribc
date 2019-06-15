#ifndef string_h
#define string_h

#include <stdint.h>

struct GribString {
    size_t length;
    uint32_t* ptr;
};

double parse_string_double(struct GribString);

#endif
