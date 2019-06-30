#ifndef string_h
#define string_h

#include <stdint.h>
#include <stdbool.h>

struct GribString {
    size_t length;
    uint32_t* ptr;
    bool alloced;
};

struct GribString new_string(size_t size, uint32_t* ptr, bool alloced);
struct GribString string_concat(struct GribString one, struct GribString two);
struct GribString string_slice(struct GribString str, int32_t one, int32_t two);
struct GribString string_from_cstr(const char* str);
struct GribString num_to_string(double d);

double parse_string_double(struct GribString);

int64_t parse_string_int(struct GribString str, uint8_t radix);
int32_t string_index_of(struct GribString str, struct GribString pattern);

void string_free(struct GribString str);

#endif
