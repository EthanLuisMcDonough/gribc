#include "value.h"
#include <stdio.h>

#define GRIB_VAL_DEF_FN(etype, name, enum)\
    struct GribValue (grib_ ## name) (etype val) {\
        union ValueUnion u = (union ValueUnion) { .name = val };\
        return (struct GribValue) {\
            .type = enum,\
            .value = u\
        };\
    }

GRIB_VAL_DEF_FN(double, number, NUMBER);
GRIB_VAL_DEF_FN(struct GribString, string, STRING);
GRIB_VAL_DEF_FN(struct GribArray, arr, ARRAY);
GRIB_VAL_DEF_FN(struct GribFunction, fn, FUNCTION);
GRIB_VAL_DEF_FN(bool, boolean, BOOLEAN);
