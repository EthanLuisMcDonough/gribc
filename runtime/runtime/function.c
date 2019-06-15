#include "function.h"
#include "value.h"

struct GribValue gribfn_invoke(struct GribFunction fn, struct GribValue* args, size_t count) {
    return fn.fn(args, count, fn.bound_value);
}

