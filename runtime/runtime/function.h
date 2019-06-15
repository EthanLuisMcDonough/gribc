#ifndef function_h
#define function_h

#include <stdio.h>

#define GRIBFNDEF(name) struct GribValue name(struct GribValue* params, size_t count, struct GribValue* bound)
#define GPARAM(type, ident, index) type ident;\
    if (index < count) {\
        ident = gribvalue_to_ ## type(params[index]);\
    } else {\
        @TODO\
    }\

struct GribFunction {
    struct GribValue* bound_value;
    struct GribValue (*fn)(struct GribValue*, size_t, struct GribValue*);
};

struct GribValue gribfn_invoke(struct GribFunction fn, struct GribValue* args, size_t count);

#endif
