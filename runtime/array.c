#include <stdio.h>
#include <stdlib.h>
#include "util.h"
#include "value.h"
#include "array.h"

void increase_capacity_array(struct GribArray* arr) {
    arr->capacity += 1;
    arr->capacity *= 2;
    arr->ptr = (struct GribValue*) realloc(arr->ptr, arr->capacity * sizeof(struct GribValue));
}

size_t add_slot_array(struct GribArray* arr) {
    size_t old_index = arr->length;
    arr->length += 1;
    if (arr->length >= arr->capacity) {
        increase_capacity_array(arr);
    }
    return old_index;
}

struct GribArray new_array(size_t capacity) {
    struct GribValue* ptr = (struct GribValue*) calloc(capacity, sizeof(struct GribValue));
    return (struct GribArray) {
        .length = 0,
        .capacity = capacity,
        .ptr = ptr
    };
}

size_t insert_element_array(struct GribArray* arr, struct GribValue val, size_t index) {
    index = CMP_MAC(index, 0, arr->length);
    
    for (size_t i = add_slot_array(arr); i > index; i--) {
        struct GribValue v = arr->ptr[i - 1];
        arr->ptr[i] = v;
    }
    
    arr->ptr[index] = val;
    
    return arr->length;
}

struct GribValue remove_element_array(struct GribArray* arr, size_t index) {
    if (arr->length == 0 || index >= arr->length) {
        return GRIBNIL;
    }
    
    struct GribValue val = arr->ptr[index];
    
    for (size_t i = index; i < --arr->length; i++) {
        struct GribValue next = arr->ptr[i + 1];
        arr->ptr[i] = next;
    }
    
    return val;
}

size_t push_element_array(struct GribArray* arr, struct GribValue val) {
    size_t new_index = add_slot_array(arr);
    arr->ptr[new_index] = val;
    return arr->length;
}

size_t unshift_element_array(struct GribArray* arr, struct GribValue val) {
    return insert_element_array(arr, val, 0);
}

struct GribValue pop_element_array(struct GribArray* arr) {
    if (arr->length == 0) { return GRIBNIL; }
    return remove_element_array(arr, arr->length - 1);
}

struct GribValue shift_element_array(struct GribArray* arr) {
    return remove_element_array(arr, 0);
}

struct GribArray concat_array(struct GribArray* one, struct GribArray two) {
    size_t one_len = one->length, two_len = two.length;
    struct GribArray n = new_array(one_len + two_len);
    
    for (int32_t i = 0; i < one_len; i++) {
        n.ptr[i] = one->ptr[i];
    }
    for (int32_t i = 0; i < two_len; i++) {
        n.ptr[i + two_len] = two.ptr[i];
    }
    
    return n;
}
