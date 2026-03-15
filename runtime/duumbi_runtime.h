#ifndef DUUMBI_RUNTIME_H
#define DUUMBI_RUNTIME_H

#include <stdint.h>

/* ── Panic ─────────────────────────────────────────────────────────── */

/** Print message to stderr and exit(1). */
void duumbi_panic(const char *msg);

/* ── Print ─────────────────────────────────────────────────────────── */

void duumbi_print_i64(int64_t val);
void duumbi_print_f64(double val);
void duumbi_print_bool(int8_t val);
void duumbi_print_string(void *ptr);

/* ── Heap allocation ───────────────────────────────────────────────── */

void *duumbi_alloc(uint64_t size);
void  duumbi_dealloc(void *ptr);

/* ── String ────────────────────────────────────────────────────────── */
/*
 * String representation (DuumbiString):
 *   { uint64_t len; char data[]; }
 * data is always null-terminated for C interop but len tracks true length.
 */

void    *duumbi_string_new(const char *data, uint64_t len);
void     duumbi_string_free(void *ptr);
uint64_t duumbi_string_len(void *ptr);
void    *duumbi_string_concat(void *a, void *b);
int8_t   duumbi_string_equals(void *a, void *b);
int64_t  duumbi_string_compare(void *a, void *b);
void    *duumbi_string_slice(void *ptr, uint64_t start, uint64_t end);
int8_t   duumbi_string_contains(void *haystack, void *needle);
int64_t  duumbi_string_find(void *haystack, void *needle);
void    *duumbi_string_from_i64(int64_t val);

/* ── Array ─────────────────────────────────────────────────────────── */
/*
 * Array representation (DuumbiArray):
 *   { uint64_t len; uint64_t capacity; uint64_t elem_size; char data[]; }
 * Growth: double capacity on push when full (initial capacity 4).
 */

void    *duumbi_array_new(uint64_t elem_size);
void     duumbi_array_push(void *arr, void *elem);
void    *duumbi_array_get(void *arr, uint64_t index);
void     duumbi_array_set(void *arr, uint64_t index, void *elem);
uint64_t duumbi_array_len(void *arr);
void     duumbi_array_free(void *arr);

/* ── Struct ────────────────────────────────────────────────────────── */
/*
 * Struct: flat contiguous allocation, field offsets computed at compile time.
 */

void *duumbi_struct_new(uint64_t total_size);
void *duumbi_struct_field_get(void *s, uint64_t offset);
void  duumbi_struct_field_set(void *s, uint64_t offset, void *value, uint64_t size);
void  duumbi_struct_free(void *s);

#endif /* DUUMBI_RUNTIME_H */
