#ifndef conversions_h
#define conversions_h

#include <stdio.h>
#include "value.h"

double gribvalue_to_number(struct GribValue val);
struct GribString gribvalue_to_string(struct GribValue val);

#endif
