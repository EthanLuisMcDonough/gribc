#include "conversions.h"

// TYPE COERSION
double gribvalue_to_number(struct GribValue val) {
    switch (val.type) {
        case NUMBER: return val.value.number;
        default: return 0.0;
    }
}

// @TODO TYPE CASTING
