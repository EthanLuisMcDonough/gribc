#ifndef util_h
#define util_h

#define MIN_MAC(a, b) ((a) < (b) ? (a) : (b))
#define MAX_MAC(a, b) ((a) > (b) ? (a) : (b))
#define CMP_MAC(val, min, max) (MIN_MAC(MAX_MAC(val, min), max))

#endif
