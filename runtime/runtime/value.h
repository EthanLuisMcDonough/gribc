#ifndef value_h
#define value_h

#include "string.h"
#include "array.h"
#include "function.h"
#include "operators.h"
#include "conversions.h"

#define GRIBNIL ((struct GribValue) { .type = NIL })

enum ValueType {
    NUMBER,
    STRING,
    ARRAY,
    HASH_OBJ,
    FUNCTION,
    NIL,
};

union ValueUnion {
    double number;
    struct GribString string;
    struct GribArray arr;
    struct GribFunction fn;
};

struct GribValue {
    enum ValueType type;
    union ValueUnion value;
};

struct GribValue grib_number(double val);
struct GribValue grib_string(struct GribString val);
struct GribValue grib_arr(struct GribArray val);
struct GribValue grib_fn(struct GribFunction val);

#endif
