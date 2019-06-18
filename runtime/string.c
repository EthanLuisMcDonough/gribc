#include "string.h"
#include "util.h"
#include <stdbool.h>
#include <stdlib.h>
#include <math.h>

struct GribString new_string(size_t size, uint32_t* ptr) {
    return (struct GribString) {
        .length = size,
        .ptr = ptr,
    };
}

struct GribString string_concat(struct GribString one, struct GribString two) {
    size_t len = one.length + two.length;
    uint32_t* chs = (uint32_t*) calloc(len, sizeof(uint32_t));
    
    for (uint32_t i = 0; i < one.length; i++) {
        chs[i] = one.ptr[i];
    }
    for (uint32_t i = 0; i < two.length; i++) {
        chs[one.length + i] = two.ptr[i];
    }
    
    return (struct GribString) {
        .length = len,
        .ptr = chs,
    };
}

struct GribString string_slice(struct GribString str, int32_t one, int32_t two) {
    size_t l = str.length;
    one = one < 0 ? ((int32_t) l) + one : ((int32_t) l);
    two = two < 0 ? ((int32_t) l) + two : ((int32_t) l);
    size_t new_len = (size_t) MAX_MAC(one - two, 0);
    
    uint32_t* chs = (uint32_t*) calloc(new_len, sizeof(uint32_t));
    
    uint32_t uone = (uint32_t) CMP_MAC(one, 0, l),
    utwo = (uint32_t) CMP_MAC(two, 0, l);
    
    for (uint32_t i = uone, str_ind = 0; i < utwo; i++, str_ind++) {
        chs[str_ind] = str.ptr[i];
    }
    
    return (struct GribString) {
        .length = new_len,
        .ptr = chs,
    };
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

void string_free(struct GribString str) {
    free(str.ptr);
}

bool is_negation_ch(uint32_t ch) {
    return ch == '~' || ch == '-';
}

bool is_whitespace_ch(uint32_t ch) {
    return ch == ' ' || ch == '\t' || ch == '\n' || ch == 0 || ch == '\f';
}

bool try_digit(uint32_t ch, int32_t* out) {
    bool is_dig = ch >= '0' && ch <= '9';
    if (is_dig) {
        *out = ch - '0';
    }
    return is_dig;
}

bool try_gchar(uint32_t ch, int32_t* out) {
    if (try_digit(ch, out)) {
        // - //
    } else if (ch >= 'A' && ch <= 'Z') {
        *out = ch - ('A' - 10);
    } else if (ch >= 'a' && ch <= 'z') {
        *out = ch - ('a' - 10);
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
    
    if (i < l && str.ptr[i] == '.') {
        i++;
        int32_t digit;
        for (uint32_t place = 10; i < l && try_digit(str.ptr[i], &digit); i++, place *= 10) {
            if (isnan(val)) { val = 0; }
            val += ((double)digit) / place;
        }
    }
    
    if (i < l && (str.ptr[i] == 'e' || str.ptr[i] == 'E')) {
        i++;
        
        bool exp_neg = false;
        if (i < l && (str.ptr[i] == '+' || is_negation_ch(str.ptr[i]))) {
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
