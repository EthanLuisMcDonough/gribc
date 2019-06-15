#include "string.h"
#include "util.h"
#include <stdbool.h>
#include <stdlib.h>

struct GribString new_string(size_t size, uint32_t* ptr) {
    return (struct GribString) {
        .length = size,
        .ptr = ptr
    };
}

struct GribString new_string_cap(size_t size) {
    return new_string(size, (uint32_t*) calloc(size, sizeof(uint32_t)));
}

struct GribString string_concat(struct GribString one, struct GribString two) {
    struct GribString n = new_string_cap(one.length + two.length);
    
    for (uint32_t i = 0; i < one.length; i++) {
        n.ptr[i] = one.ptr[i];
    }
    for (uint32_t i = 0; i < two.length; i++) {
        n.ptr[one.length + i] = two.ptr[i];
    }
    
    return n;
}

struct GribString string_slice(struct GribString str, int32_t one, int32_t two) {
    size_t l = str.length;
    one = one < 0 ? ((int32_t) l) + one : ((int32_t) l);
    two = two < 0 ? ((int32_t) l) + two : ((int32_t) l);
    
    struct GribString new_str = new_string_cap((size_t) MAX_MAC(one - two, 0));
    
    uint32_t uone = (uint32_t) CMP_MAC(one, 0, l),
    utwo = (uint32_t) CMP_MAC(two, 0, l);
    
    for (uint32_t i = uone, str_ind = 0; i < utwo; i++, str_ind++) {
        new_str.ptr[str_ind] = str.ptr[i];
    }
    
    return new_str;
}

int32_t string_index_of(struct GribString str, struct GribString pattern) {
    int32_t total_len = ((int32_t) str.length) - ((int32_t) pattern.length),
    pat_len = (int32_t) pattern.length;
    
    for (int32_t i = 0; i <= total_len; i++) {
        bool ret = true;
        
        for (int32_t j = 0; j < pat_len && ret; j++) {
            ret = pattern.ptr[j] == str.ptr[i + j];
        }
        
        if (ret) { return i; }
    }
    
    return -1;
}

double parse_string_double(struct GribString str) {
    // @TODO
    return 0.0;
}
