#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <ctype.h>

/* Killer Runtime Library for Phase 2 Compiler */

/* ============================================================================
   VALUE TYPE SYSTEM
   Represents all Killer data types in C
   ============================================================================ */

typedef enum {
    VALUE_NULL = 0,
    VALUE_NUMBER = 1,
    VALUE_STRING = 2,
    VALUE_BOOLEAN = 3,
    VALUE_ARRAY = 4,
    VALUE_OBJECT = 5
} ValueType;

typedef struct Value {
    ValueType type;
    double num_val;
    char* str_val;
    int bool_val;
    struct Value** array_val;
    int array_len;
    int array_capacity;
} Value;

/* ============================================================================
   MEMORY MANAGEMENT
   ============================================================================ */

Value* value_create() {
    Value* v = (Value*)malloc(sizeof(Value));
    v->type = VALUE_NULL;
    v->num_val = 0;
    v->str_val = NULL;
    v->bool_val = 0;
    v->array_val = NULL;
    v->array_len = 0;
    v->array_capacity = 0;
    return v;
}

void value_free(Value* v) {
    if (!v) return;
    if (v->type == VALUE_STRING && v->str_val) {
        free(v->str_val);
    }
    if (v->type == VALUE_ARRAY && v->array_val) {
        for (int i = 0; i < v->array_len; i++) {
            value_free(v->array_val[i]);
        }
        free(v->array_val);
    }
    free(v);
}

/* ============================================================================
   VALUE CONSTRUCTORS
   ============================================================================ */

Value* value_null() {
    return value_create();
}

Value* value_number(double n) {
    Value* v = value_create();
    v->type = VALUE_NUMBER;
    v->num_val = n;
    return v;
}

Value* value_string(const char* s) {
    Value* v = value_create();
    v->type = VALUE_STRING;
    v->str_val = (char*)malloc(strlen(s) + 1);
    strcpy(v->str_val, s);
    return v;
}

Value* value_boolean(int b) {
    Value* v = value_create();
    v->type = VALUE_BOOLEAN;
    v->bool_val = b ? 1 : 0;
    return v;
}

Value* value_array(int capacity) {
    Value* v = value_create();
    v->type = VALUE_ARRAY;
    v->array_capacity = capacity > 0 ? capacity : 10;
    v->array_val = (Value**)malloc(sizeof(Value*) * v->array_capacity);
    v->array_len = 0;
    return v;
}

/* ============================================================================
   ARRAY OPERATIONS
   ============================================================================ */

void array_push(Value* arr, Value* item) {
    if (arr->type != VALUE_ARRAY) return;
    
    if (arr->array_len >= arr->array_capacity) {
        arr->array_capacity *= 2;
        arr->array_val = (Value**)realloc(arr->array_val, 
                                          sizeof(Value*) * arr->array_capacity);
    }
    arr->array_val[arr->array_len++] = item;
}

Value* array_get(Value* arr, int idx) {
    if (arr->type != VALUE_ARRAY) return value_null();
    if (idx < 0 || idx >= arr->array_len) return value_null();
    return arr->array_val[idx];
}

void array_set(Value* arr, int idx, Value* item) {
    if (arr->type != VALUE_ARRAY) return;
    if (idx < 0 || idx >= arr->array_len) return;
    value_free(arr->array_val[idx]);
    arr->array_val[idx] = item;
}

int array_length(Value* arr) {
    if (arr->type != VALUE_ARRAY) return 0;
    return arr->array_len;
}

/* ============================================================================
   TYPE CONVERSION
   ============================================================================ */

int value_is_truthy(Value* v) {
    if (!v) return 0;
    switch (v->type) {
        case VALUE_NULL: return 0;
        case VALUE_NUMBER: return v->num_val != 0 && !isnan(v->num_val);
        case VALUE_BOOLEAN: return v->bool_val;
        case VALUE_STRING: return v->str_val && strlen(v->str_val) > 0;
        case VALUE_ARRAY: return v->array_len > 0;
        default: return 1;
    }
}

char* value_to_string(Value* v) {
    if (!v) return "null";
    
    static char buffer[256];
    
    switch (v->type) {
        case VALUE_NULL:
            return "null";
        case VALUE_NUMBER:
            if (v->num_val == (int)v->num_val) {
                snprintf(buffer, sizeof(buffer), "%d", (int)v->num_val);
            } else {
                snprintf(buffer, sizeof(buffer), "%g", v->num_val);
            }
            return buffer;
        case VALUE_STRING:
            return v->str_val ? v->str_val : "";
        case VALUE_BOOLEAN:
            return v->bool_val ? "true" : "false";
        case VALUE_ARRAY:
            snprintf(buffer, sizeof(buffer), "[Array(%d)]", v->array_len);
            return buffer;
        case VALUE_OBJECT:
            return "[Object]";
        default:
            return "unknown";
    }
}

double value_to_number(Value* v) {
    if (!v) return 0;
    switch (v->type) {
        case VALUE_NUMBER: return v->num_val;
        case VALUE_BOOLEAN: return v->bool_val ? 1 : 0;
        case VALUE_STRING:
            if (!v->str_val) return 0;
            return atof(v->str_val);
        case VALUE_NULL: return 0;
        default: return 0;
    }
}

int value_to_boolean(Value* v) {
    return value_is_truthy(v);
}

/* ============================================================================
   COMPARISON & EQUALITY
   ============================================================================ */

int value_equals(Value* a, Value* b) {
    if (!a || !b) return a == b;
    if (a->type != b->type) return 0;
    
    switch (a->type) {
        case VALUE_NULL: return 1;
        case VALUE_NUMBER: return a->num_val == b->num_val;
        case VALUE_BOOLEAN: return a->bool_val == b->bool_val;
        case VALUE_STRING:
            return strcmp(a->str_val ? a->str_val : "", 
                         b->str_val ? b->str_val : "") == 0;
        default: return a == b;  /* Reference equality for arrays/objects */
    }
}

int value_less_than(Value* a, Value* b) {
    double an = value_to_number(a);
    double bn = value_to_number(b);
    return an < bn;
}

/* ============================================================================
   ARITHMETIC OPERATIONS
   ============================================================================ */

Value* op_add(Value* a, Value* b) {
    if (a->type == VALUE_STRING || b->type == VALUE_STRING) {
        char* as = value_to_string(a);
        char* bs = value_to_string(b);
        char* result = (char*)malloc(strlen(as) + strlen(bs) + 1);
        strcpy(result, as);
        strcat(result, bs);
        Value* v = value_string(result);
        free(result);
        return v;
    }
    return value_number(value_to_number(a) + value_to_number(b));
}

Value* op_subtract(Value* a, Value* b) {
    return value_number(value_to_number(a) - value_to_number(b));
}

Value* op_multiply(Value* a, Value* b) {
    return value_number(value_to_number(a) * value_to_number(b));
}

Value* op_divide(Value* a, Value* b) {
    double bn = value_to_number(b);
    if (bn == 0) {
        fprintf(stderr, "Error: Division by zero\n");
        return value_null();
    }
    return value_number(value_to_number(a) / bn);
}

Value* op_modulo(Value* a, Value* b) {
    int an = (int)value_to_number(a);
    int bn = (int)value_to_number(b);
    if (bn == 0) {
        fprintf(stderr, "Error: Modulo by zero\n");
        return value_null();
    }
    return value_number(an % bn);
}

Value* op_power(Value* a, Value* b) {
    return value_number(pow(value_to_number(a), value_to_number(b)));
}

/* ============================================================================
   MATH FUNCTIONS
   ============================================================================ */

Value* math_abs(Value* v) {
    return value_number(fabs(value_to_number(v)));
}

Value* math_sqrt(Value* v) {
    return value_number(sqrt(fabs(value_to_number(v))));
}

Value* math_floor(Value* v) {
    return value_number(floor(value_to_number(v)));
}

Value* math_ceil(Value* v) {
    return value_number(ceil(value_to_number(v)));
}

Value* math_round(Value* v) {
    return value_number(round(value_to_number(v)));
}

Value* math_max(Value* a, Value* b) {
    double an = value_to_number(a);
    double bn = value_to_number(b);
    return value_number(an > bn ? an : bn);
}

Value* math_min(Value* a, Value* b) {
    double an = value_to_number(a);
    double bn = value_to_number(b);
    return value_number(an < bn ? an : bn);
}

Value* math_random() {
    return value_number((double)rand() / RAND_MAX);
}

Value* math_sin(Value* v) {
    return value_number(sin(value_to_number(v)));
}

Value* math_cos(Value* v) {
    return value_number(cos(value_to_number(v)));
}

Value* math_tan(Value* v) {
    return value_number(tan(value_to_number(v)));
}

/* ============================================================================
   STRING FUNCTIONS
   ============================================================================ */

Value* string_length(Value* s) {
    if (s->type != VALUE_STRING) return value_number(0);
    return value_number(strlen(s->str_val ? s->str_val : ""));
}

Value* string_upper(Value* s) {
    if (s->type != VALUE_STRING) return value_string("");
    
    char* str = s->str_val ? s->str_val : "";
    char* result = (char*)malloc(strlen(str) + 1);
    for (int i = 0; str[i]; i++) {
        result[i] = toupper(str[i]);
    }
    result[strlen(str)] = '\0';
    Value* v = value_string(result);
    free(result);
    return v;
}

Value* string_lower(Value* s) {
    if (s->type != VALUE_STRING) return value_string("");
    
    char* str = s->str_val ? s->str_val : "";
    char* result = (char*)malloc(strlen(str) + 1);
    for (int i = 0; str[i]; i++) {
        result[i] = tolower(str[i]);
    }
    result[strlen(str)] = '\0';
    Value* v = value_string(result);
    free(result);
    return v;
}

Value* string_trim(Value* s) {
    if (s->type != VALUE_STRING) return value_string("");
    
    char* str = s->str_val ? s->str_val : "";
    int start = 0, end = strlen(str) - 1;
    
    while (start <= end && isspace(str[start])) start++;
    while (end >= start && isspace(str[end])) end--;
    
    int len = end - start + 1;
    if (len <= 0) return value_string("");
    
    char* result = (char*)malloc(len + 1);
    strncpy(result, str + start, len);
    result[len] = '\0';
    Value* v = value_string(result);
    free(result);
    return v;
}

Value* string_substring(Value* s, int start, int end) {
    if (s->type != VALUE_STRING) return value_string("");
    
    char* str = s->str_val ? s->str_val : "";
    int len = strlen(str);
    
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start >= end) return value_string("");
    
    char* result = (char*)malloc(end - start + 1);
    strncpy(result, str + start, end - start);
    result[end - start] = '\0';
    Value* v = value_string(result);
    free(result);
    return v;
}

Value* string_indexOf(Value* s, Value* search) {
    if (s->type != VALUE_STRING) return value_number(-1);
    
    char* str = s->str_val ? s->str_val : "";
    char* search_str = search->str_val ? search->str_val : "";
    
    char* found = strstr(str, search_str);
    if (found) {
        return value_number(found - str);
    }
    return value_number(-1);
}

/* ============================================================================
   PRINT FUNCTION
   ============================================================================ */

void killer_print(Value* v) {
    if (!v) {
        printf("null\n");
        return;
    }
    
    switch (v->type) {
        case VALUE_NULL:
            printf("null\n");
            break;
        case VALUE_NUMBER:
            if (v->num_val == (int)v->num_val) {
                printf("%d\n", (int)v->num_val);
            } else {
                printf("%g\n", v->num_val);
            }
            break;
        case VALUE_STRING:
            printf("%s\n", v->str_val ? v->str_val : "");
            break;
        case VALUE_BOOLEAN:
            printf("%s\n", v->bool_val ? "true" : "false");
            break;
        case VALUE_ARRAY:
            printf("[");
            for (int i = 0; i < v->array_len; i++) {
                if (i > 0) printf(", ");
                printf("%s", value_to_string(v->array_val[i]));
            }
            printf("]\n");
            break;
        case VALUE_OBJECT:
            printf("[Object]\n");
            break;
        default:
            printf("unknown\n");
    }
}

void killer_print_multiple(int count, Value** values) {
    for (int i = 0; i < count; i++) {
        if (i > 0) printf(" ");
        printf("%s", value_to_string(values[i]));
    }
    printf("\n");
}

/* ============================================================================
   PARSE FUNCTIONS
   ============================================================================ */

Value* parse_int(Value* v) {
    return value_number((double)(int)value_to_number(v));
}

Value* parse_float(Value* v) {
    return value_number(value_to_number(v));
}

Value* is_nan(Value* v) {
    double n = value_to_number(v);
    return value_boolean(isnan(n));
}

Value* is_finite(Value* v) {
    double n = value_to_number(v);
    return value_boolean(isfinite(n));
}

/* ============================================================================
   ARRAY HELPER FUNCTIONS
   ============================================================================ */

Value* array_is_array(Value* v) {
    return value_boolean(v && v->type == VALUE_ARRAY);
}
