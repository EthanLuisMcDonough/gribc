#include "conversions.h"
#include "string.h"
#include "value.h"

static uint32_t NIL_STR_MSG[3] = { 'n', 'i', 'l' };
static uint32_t TRUE_STR_MSG[4] = { 't', 'r', 'u', 'e' };
static uint32_t FALSE_STR_MSG[5] = { 'f', 'a', 'l', 's', 'e' };
static uint32_t HASH_STR_MSG[5] = { '[', 'o', 'b', 'j', ']' };
static uint32_t ARR_STR_MSG[5] = { '[', 'a', 'r', 'r', ']' };
static uint32_t FN_STR_MSG[4] = { '[', 'f', 'n', ']' };

// TYPE COERSION
double gribvalue_to_number(struct GribValue val) {
    switch (val.type) {
        case NUMBER: return val.value.number;
        case STRING: return parse_string_double(val.value.string);
        case NIL: return 0.0;
        default: return 1.0;
    }
}

struct GribString gribvalue_to_string(struct GribValue val) {
    switch (val.type) {
        case NIL: return new_string(3, NIL_STR_MSG, false);
        case BOOLEAN: return new_string(
            4 + !val.value.boolean,
            val.value.boolean ? TRUE_STR_MSG : FALSE_STR_MSG,
            false
        );
        case STRING: return val.value.string;
        case HASH_OBJ: return new_string(5, HASH_STR_MSG, false);
        case FUNCTION: return new_string(4, FN_STR_MSG, false);
        case NUMBER: return num_to_string(val.value.number);
        case ARRAY: return new_string(4, ARR_STR_MSG, false);
    }
}

// @TODO TYPE CASTING
