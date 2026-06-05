#ifndef DUUMBI_RUNTIME_H
#define DUUMBI_RUNTIME_H

#include <stdint.h>

/* ── Panic ─────────────────────────────────────────────────────────── */

/** Print message to stderr and exit(1). */
void duumbi_panic(const char *msg);

/* ── Trace telemetry ───────────────────────────────────────────────── */

void duumbi_trace_init(void);
void duumbi_trace_function_enter(int64_t function_id);
void duumbi_trace_function_exit(int64_t function_id);
void duumbi_trace_block_enter(int64_t block_id);
void duumbi_trace_block_exit(int64_t block_id);
void duumbi_trace_panic(const char *msg);

/* ── Print ─────────────────────────────────────────────────────────── */

void duumbi_print_i64(int64_t val);
void duumbi_print_f64(double val);
void duumbi_print_bool(int8_t val);
void duumbi_print_string(void *ptr);
void *duumbi_read_line(void);
void *duumbi_print_ln(void *ptr);
void *duumbi_file_read(void *path_ptr, int64_t max_bytes);
void *duumbi_file_write(void *path_ptr, void *contents_ptr);
void *duumbi_file_exists(void *path_ptr);
void *duumbi_list_dir(void *path_ptr);
void *duumbi_path_join(void *left_ptr, void *right_ptr);

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
void    *duumbi_array_push(void *arr, int64_t elem);
int64_t  duumbi_array_get(void *arr, uint64_t index);
void     duumbi_array_set(void *arr, uint64_t index, int64_t elem);
uint64_t duumbi_array_len(void *arr);
void     duumbi_array_free(void *arr);

/* ── Struct ────────────────────────────────────────────────────────── */
/*
 * Struct: flat contiguous allocation, field offsets computed at compile time.
 */

void    *duumbi_struct_new(uint64_t total_size);
int64_t  duumbi_struct_field_get(void *s, uint64_t offset);
void     duumbi_struct_field_set(void *s, uint64_t offset, int64_t value);
void     duumbi_struct_free(void *s);

/* ── Result / Option ──────────────────────────────────────────────── */

void    *duumbi_result_new_ok(int64_t payload);
void    *duumbi_result_new_err(int64_t payload);
int8_t   duumbi_result_is_ok(void *ptr);
int64_t  duumbi_result_unwrap(void *ptr);
int64_t  duumbi_result_unwrap_err(void *ptr);
void     duumbi_result_free(void *ptr);

void    *duumbi_option_new_some(int64_t payload);
void    *duumbi_option_new_none(void);
int8_t   duumbi_option_is_some(void *ptr);
int64_t  duumbi_option_unwrap(void *ptr);
void     duumbi_option_free(void *ptr);

/* ── JSON ─────────────────────────────────────────────────────────── */

void    *duumbi_json_parse(void *input);
void    *duumbi_json_stringify(void *value);
void    *duumbi_json_get_field(void *value, void *key);
void    *duumbi_json_array_len(void *value);
void    *duumbi_json_array_get(void *value, int64_t index);
void     duumbi_json_free(void *value);

/* ── TCP ──────────────────────────────────────────────────────────── */

void    *duumbi_tcp_connect(void *host, int64_t port, int64_t timeout_ms);
void    *duumbi_tcp_listen(void *host, int64_t port, int64_t timeout_ms);
void    *duumbi_tcp_accept(void *listener, int64_t timeout_ms);
void    *duumbi_tcp_read(void *socket, int64_t max_bytes, int64_t timeout_ms);
void    *duumbi_tcp_write(void *socket, void *data, int64_t timeout_ms);
void    *duumbi_tcp_close(void *socket);
void    *duumbi_tcp_listener_close(void *listener);
void     duumbi_tcp_socket_free(void *socket);
void     duumbi_tcp_listener_free(void *listener);

#endif /* DUUMBI_RUNTIME_H */
