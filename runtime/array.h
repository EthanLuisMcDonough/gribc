#ifndef array_h
#define array_h

struct GribArray {
    size_t length;
    size_t capacity;
    struct GribValue* ptr;
};

#endif
