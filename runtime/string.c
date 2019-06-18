#include "string.h"
#include "util.h"
#include <stdbool.h>
#include <stdlib.h>
#include <math.h>

struct GribString new_string(size_t size, uint32_t* ptr) {
    return (struct GribString) {
        .length = size,
        .ptr = ptr
    };
}

struct GribString new_string_cap(size_t size) {
    return new_string(size, (uint32_t*) calloc(size, sizeof(uint32_t)));
}

struct GribString string_concat(struct GribString one, struct GribString two) {
    struct GribString n = new_string_cap(one.length + two.length);
    
    for (uint32_t i = 0; i < one.length; i++) {
        n.ptr[i] = one.ptr[i];
    }
    for (uint32_t i = 0; i < two.length; i++) {
        n.ptr[one.length + i] = two.ptr[i];
    }
    
    return n;
}

struct GribString string_slice(struct GribString str, int32_t one, int32_t two) {
    size_t l = str.length;
    one = one < 0 ? ((int32_t) l) + one : ((int32_t) l);
    two = two < 0 ? ((int32_t) l) + two : ((int32_t) l);
    
    struct GribString new_str = new_string_cap((size_t) MAX_MAC(one - two, 0));
    
    uint32_t uone = (uint32_t) CMP_MAC(one, 0, l),
    utwo = (uint32_t) CMP_MAC(two, 0, l);
    
    for (uint32_t i = uone, str_ind = 0; i < utwo; i++, str_ind++) {
        new_str.ptr[str_ind] = str.ptr[i];
    }
    
    return new_str;
}

int32_t string_index_of(struct GribString str, struct GribString pattern) {
    int32_t total_len = ((int32_t) str.length) - ((int32_t) pattern.length),
    pat_len = (int32_t) pattern.length;
    
    for (int32_t i = 0; i <= total_len; i++) {
        bool ret = true;
        
        for (int32_t j = 0; j < pat_len && ret; j++) {
            ret = pattern.ptr[j] == str.ptr[i + j];
        }
        
        if (ret) { return i; }
    }
    
    return -1;
}

bool is_negation_ch(uint32_t ch) {
    return ch == 126 || ch == 45;
}

bool is_whitespace_ch(uint32_t ch) {
    return ch == 32 || ch == 9 || ch == 10 || ch == 0 || ch == 12;
}

bool try_digit(uint32_t ch, int32_t* out) {
    bool is_dig = ch > 47 && ch < 58;
    if (is_dig) {
        *out = ch - 48;
    }
    return is_dig;
}

bool try_gchar(uint32_t ch, int32_t* out) {
    if (try_digit(ch, out)) {
        // - //
    } else if (ch > 64 && ch < 91) { // Turn [A-Z] into a base36 digit
        *out = ch - 55;
    } else if (ch > 96 && ch < 123) { // Turn [a-z] into a base36 digit
        *out = ch - 87;
    } else {
        return false;
    }
    return true;
}

bool parse_leading(struct GribString str, size_t* i) {
    for (; *i < str.length && is_whitespace_ch(str.ptr[*i]); *i += 1) {}
    
    if (*i < str.length && is_negation_ch(str.ptr[*i])) {
        *i += 1;
        return true;
    }
    return false;
}

double parse_string_double(struct GribString str) {
    double val = NAN; size_t i = 0, l = str.length;
    
    bool is_neg = parse_leading(str, &i);
    
    for (int32_t digit; i < l && try_digit(str.ptr[i], &digit); i++) {
        if (isnan(val)) { val = 0; }
        val *= 10.0;
        val += (double) digit;
    }
    
    if (i < l && str.ptr[i] == 46 /* check if current char is '.' */) {
        i++;
        int32_t digit;
        for (uint32_t place = 10; i < l && try_digit(str.ptr[i], &digit); i++, place *= 10) {
            if (isnan(val)) { val = 0; }
            val += ((double)digit) / place;
        }
    }
    
    if (i < l && (str.ptr[i] == 101 || str.ptr[i] == 69 /* check if current char is 'e' or 'E' */)) {
        i++;
        
        bool exp_neg = false;
        // Check if current char is '+', '~', or '-'
        if (i < l && (str.ptr[i] == 43 || is_negation_ch(str.ptr[i]))) {
            exp_neg = is_negation_ch(str.ptr[i]);
            i++;
        }
        
        int32_t num = 0;
        for (int32_t digit; i < l && try_digit(str.ptr[i], &digit); i++) {
            num *= 10;
            num += digit;
        }
        
        val *= pow(10.0, ((double) num) * (exp_neg ? -1 : 1));
    }
    
    return val * (is_neg ? -1 : 1);
}

int64_t parse_string_int(struct GribString str, uint8_t radix) {
    radix = CMP_MAC(radix, 2, 36);
    int64_t integer = 0; size_t i = 0;
    
    bool is_neg = parse_leading(str, &i);
    
    for (; i < str.length; i++) {
        int digit;
        if (try_gchar(str.ptr[i], &digit) && digit < radix) {
            integer *= radix;
            integer += digit;
        } else {
            break;
        }
    }
    
    return (is_neg ? -1 : 1) * integer;
}
