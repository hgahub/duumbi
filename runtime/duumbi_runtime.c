#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

/* ── Internal types ────────────────────────────────────────────────── */

typedef struct {
    uint64_t len;
    char     data[];   /* null-terminated for C interop */
} DuumbiString;

typedef struct {
    uint64_t len;
    uint64_t capacity;
    uint64_t elem_size;
    char     data[];
} DuumbiArray;

/* ── Panic ─────────────────────────────────────────────────────────── */

void duumbi_panic(const char *msg) {
    fprintf(stderr, "duumbi panic: %s\n", msg);
    exit(1);
}

/* ── Print ─────────────────────────────────────────────────────────── */

void duumbi_print_i64(int64_t val) {
    printf("%lld\n", (long long)val);
}

void duumbi_print_f64(double val) {
    printf("%.15g\n", val);
}

void duumbi_print_bool(int8_t val) {
    printf("%s\n", val ? "true" : "false");
}

void duumbi_print_string(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    if (s == NULL) {
        printf("(null)\n");
        return;
    }
    printf("%.*s\n", (int)s->len, s->data);
}

/* ── Heap allocation ───────────────────────────────────────────────── */

void *duumbi_alloc(uint64_t size) {
    void *ptr = malloc((size_t)size);
    if (ptr == NULL) {
        duumbi_panic("out of memory");
    }
    return ptr;
}

void duumbi_dealloc(void *ptr) {
    free(ptr);
}

/* ── String ────────────────────────────────────────────────────────── */

void *duumbi_string_new(const char *data, uint64_t len) {
    DuumbiString *s = (DuumbiString *)duumbi_alloc(sizeof(DuumbiString) + len + 1);
    s->len = len;
    memcpy(s->data, data, (size_t)len);
    s->data[len] = '\0';
    return s;
}

void duumbi_string_free(void *ptr) {
    duumbi_dealloc(ptr);
}

uint64_t duumbi_string_len(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    return s->len;
}

void *duumbi_string_concat(void *a, void *b) {
    DuumbiString *sa = (DuumbiString *)a;
    DuumbiString *sb = (DuumbiString *)b;
    uint64_t new_len = sa->len + sb->len;
    DuumbiString *result = (DuumbiString *)duumbi_alloc(sizeof(DuumbiString) + new_len + 1);
    result->len = new_len;
    memcpy(result->data, sa->data, (size_t)sa->len);
    memcpy(result->data + sa->len, sb->data, (size_t)sb->len);
    result->data[new_len] = '\0';
    return result;
}

int8_t duumbi_string_equals(void *a, void *b) {
    DuumbiString *sa = (DuumbiString *)a;
    DuumbiString *sb = (DuumbiString *)b;
    if (sa->len != sb->len) return 0;
    return memcmp(sa->data, sb->data, (size_t)sa->len) == 0 ? 1 : 0;
}

int64_t duumbi_string_compare(void *a, void *b) {
    DuumbiString *sa = (DuumbiString *)a;
    DuumbiString *sb = (DuumbiString *)b;
    uint64_t min_len = sa->len < sb->len ? sa->len : sb->len;
    int cmp = memcmp(sa->data, sb->data, (size_t)min_len);
    if (cmp != 0) return (int64_t)cmp;
    if (sa->len < sb->len) return -1;
    if (sa->len > sb->len) return 1;
    return 0;
}

void *duumbi_string_slice(void *ptr, uint64_t start, uint64_t end) {
    DuumbiString *s = (DuumbiString *)ptr;
    if (start > s->len) start = s->len;
    if (end > s->len) end = s->len;
    if (start > end) start = end;
    uint64_t slice_len = end - start;
    return duumbi_string_new(s->data + start, slice_len);
}

int8_t duumbi_string_contains(void *haystack, void *needle) {
    DuumbiString *h = (DuumbiString *)haystack;
    DuumbiString *n = (DuumbiString *)needle;
    if (n->len == 0) return 1;
    if (n->len > h->len) return 0;
    /* Simple search — O(n*m), sufficient for Phase 9a-1 */
    for (uint64_t i = 0; i <= h->len - n->len; i++) {
        if (memcmp(h->data + i, n->data, (size_t)n->len) == 0) {
            return 1;
        }
    }
    return 0;
}

int64_t duumbi_string_find(void *haystack, void *needle) {
    DuumbiString *h = (DuumbiString *)haystack;
    DuumbiString *n = (DuumbiString *)needle;
    if (n->len == 0) return 0;
    if (n->len > h->len) return -1;
    for (uint64_t i = 0; i <= h->len - n->len; i++) {
        if (memcmp(h->data + i, n->data, (size_t)n->len) == 0) {
            return (int64_t)i;
        }
    }
    return -1;
}

void *duumbi_string_from_i64(int64_t val) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)val);
    return duumbi_string_new(buf, (uint64_t)len);
}

/* ── Array ─────────────────────────────────────────────────────────── */
/*
 * Simplified API: all elements are stored as int64_t (8 bytes).
 * This works for i64, f64 (via bitcast), bool, and pointers (String/Array/Struct).
 * duumbi_array_push returns the (possibly reallocated) array pointer.
 */

#define ARRAY_INITIAL_CAPACITY 4

void *duumbi_array_new(uint64_t elem_size) {
    (void)elem_size; /* reserved for future use; currently always 8 */
    DuumbiArray *arr = (DuumbiArray *)duumbi_alloc(
        sizeof(DuumbiArray) + sizeof(int64_t) * ARRAY_INITIAL_CAPACITY);
    arr->len = 0;
    arr->capacity = ARRAY_INITIAL_CAPACITY;
    arr->elem_size = sizeof(int64_t);
    return arr;
}

static DuumbiArray *array_grow(DuumbiArray *arr) {
    uint64_t new_cap = arr->capacity * 2;
    DuumbiArray *new_arr = (DuumbiArray *)realloc(
        arr, sizeof(DuumbiArray) + arr->elem_size * new_cap);
    if (new_arr == NULL) {
        duumbi_panic("out of memory on array grow");
    }
    new_arr->capacity = new_cap;
    return new_arr;
}

void *duumbi_array_push(void *arr_ptr, int64_t elem) {
    DuumbiArray *arr = (DuumbiArray *)arr_ptr;
    if (arr->len >= arr->capacity) {
        arr = array_grow(arr);
    }
    int64_t *data = (int64_t *)arr->data;
    data[arr->len] = elem;
    arr->len++;
    return arr;  /* return (possibly reallocated) pointer */
}

int64_t duumbi_array_get(void *arr, uint64_t index) {
    DuumbiArray *a = (DuumbiArray *)arr;
    if (index >= a->len) {
        duumbi_panic("array index out of bounds");
    }
    int64_t *data = (int64_t *)a->data;
    return data[index];
}

void duumbi_array_set(void *arr, uint64_t index, int64_t elem) {
    DuumbiArray *a = (DuumbiArray *)arr;
    if (index >= a->len) {
        duumbi_panic("array index out of bounds");
    }
    int64_t *data = (int64_t *)a->data;
    data[index] = elem;
}

uint64_t duumbi_array_len(void *arr) {
    DuumbiArray *a = (DuumbiArray *)arr;
    return a->len;
}

void duumbi_array_free(void *arr) {
    duumbi_dealloc(arr);
}

/* ── Struct ────────────────────────────────────────────────────────── */

void *duumbi_struct_new(uint64_t total_size) {
    void *s = duumbi_alloc(total_size);
    memset(s, 0, (size_t)total_size);
    return s;
}

int64_t duumbi_struct_field_get(void *s, uint64_t offset) {
    int64_t value;
    memcpy(&value, (char *)s + offset, sizeof(int64_t));
    return value;
}

void duumbi_struct_field_set(void *s, uint64_t offset, int64_t value) {
    memcpy((char *)s + offset, &value, sizeof(int64_t));
}

void duumbi_struct_free(void *s) {
    duumbi_dealloc(s);
}

/* ── Result (tagged union: {i8 discriminant, i64 payload}) ────────── */
/*
 * Layout: DuumbiResult = { int8_t tag, int64_t payload }
 * Tag: 1 = Ok, 0 = Err
 * Payload: i64-sized value (integer, float bitcast, or pointer)
 */

typedef struct {
    int8_t  tag;       /* 1 = Ok, 0 = Err */
    int64_t payload;
} DuumbiResult;

void *duumbi_result_new_ok(int64_t payload) {
    DuumbiResult *r = (DuumbiResult *)duumbi_alloc(sizeof(DuumbiResult));
    r->tag = 1;
    r->payload = payload;
    return r;
}

void *duumbi_result_new_err(int64_t payload) {
    DuumbiResult *r = (DuumbiResult *)duumbi_alloc(sizeof(DuumbiResult));
    r->tag = 0;
    r->payload = payload;
    return r;
}

int8_t duumbi_result_is_ok(void *ptr) {
    DuumbiResult *r = (DuumbiResult *)ptr;
    return r->tag;
}

int64_t duumbi_result_unwrap(void *ptr) {
    DuumbiResult *r = (DuumbiResult *)ptr;
    if (r->tag != 1) {
        duumbi_panic("called Result::unwrap() on an Err value");
    }
    return r->payload;
}

int64_t duumbi_result_unwrap_err(void *ptr) {
    DuumbiResult *r = (DuumbiResult *)ptr;
    if (r->tag != 0) {
        duumbi_panic("called Result::unwrap_err() on an Ok value");
    }
    return r->payload;
}

void duumbi_result_free(void *ptr) {
    duumbi_dealloc(ptr);
}

/* ── Option (tagged union: {i8 discriminant, i64 payload}) ────────── */
/*
 * Layout: DuumbiOption = { int8_t tag, int64_t payload }
 * Tag: 1 = Some, 0 = None
 * Payload: i64-sized value (only meaningful when tag == 1)
 */

typedef struct {
    int8_t  tag;       /* 1 = Some, 0 = None */
    int64_t payload;
} DuumbiOption;

void *duumbi_option_new_some(int64_t payload) {
    DuumbiOption *o = (DuumbiOption *)duumbi_alloc(sizeof(DuumbiOption));
    o->tag = 1;
    o->payload = payload;
    return o;
}

void *duumbi_option_new_none(void) {
    DuumbiOption *o = (DuumbiOption *)duumbi_alloc(sizeof(DuumbiOption));
    o->tag = 0;
    o->payload = 0;
    return o;
}

int8_t duumbi_option_is_some(void *ptr) {
    DuumbiOption *o = (DuumbiOption *)ptr;
    return o->tag;
}

int64_t duumbi_option_unwrap(void *ptr) {
    DuumbiOption *o = (DuumbiOption *)ptr;
    if (o->tag != 1) {
        duumbi_panic("called Option::unwrap() on a None value");
    }
    return o->payload;
}

void duumbi_option_free(void *ptr) {
    duumbi_dealloc(ptr);
}
