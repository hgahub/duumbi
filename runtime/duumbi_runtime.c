#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <errno.h>
#include <sys/stat.h>
#include <sys/types.h>

#if defined(_WIN32)
#include <direct.h>
#include <io.h>
#include <windows.h>
#define DUUMBI_MKDIR(path) _mkdir(path)
#define DUUMBI_PATH_SEP '\\'
#define DUUMBI_REALPATH(path, resolved) _fullpath((resolved), (path), DUUMBI_PATH_BUFFER_LEN)
#else
#include <dirent.h>
#include <unistd.h>
#define DUUMBI_MKDIR(path) mkdir(path, 0777)
#define DUUMBI_PATH_SEP '/'
#define DUUMBI_REALPATH(path, resolved) realpath((path), (resolved))
#endif

#ifndef S_IFMT
#define S_IFMT _S_IFMT
#endif
#ifndef S_IFREG
#define S_IFREG _S_IFREG
#endif
#ifndef S_IFDIR
#define S_IFDIR _S_IFDIR
#endif
#ifndef S_ISREG
#define S_ISREG(mode) (((mode) & S_IFMT) == S_IFREG)
#endif
#ifndef S_ISDIR
#define S_ISDIR(mode) (((mode) & S_IFMT) == S_IFDIR)
#endif

#if defined(_MSC_VER)
#define DUUMBI_THREAD_LOCAL __declspec(thread)
#else
#define DUUMBI_THREAD_LOCAL _Thread_local
#endif

#define DUUMBI_TELEMETRY_DIR_ENV "DUUMBI_TELEMETRY_DIR"
#define DUUMBI_DEFAULT_TELEMETRY_DIR ".duumbi/telemetry"
#define DUUMBI_TRACE_SCHEMA_VERSION "duumbi.telemetry.trace.v1"
#define DUUMBI_CRASH_SCHEMA_VERSION "duumbi.telemetry.crash.v1"
#define DUUMBI_PATH_BUFFER_LEN 4096
#define DUUMBI_TRACE_STACK_LIMIT 1024
#define DUUMBI_WORKSPACE_ROOT_ENV "DUUMBI_WORKSPACE_ROOT"

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

/* ── Trace telemetry ───────────────────────────────────────────────── */

static DUUMBI_THREAD_LOCAL int duumbi_trace_active = 0;
static DUUMBI_THREAD_LOCAL int64_t duumbi_current_function_id = 0;
static DUUMBI_THREAD_LOCAL int64_t duumbi_current_block_id = 0;
static DUUMBI_THREAD_LOCAL int64_t duumbi_function_id_stack[DUUMBI_TRACE_STACK_LIMIT];
static DUUMBI_THREAD_LOCAL int64_t duumbi_block_id_stack[DUUMBI_TRACE_STACK_LIMIT];
static DUUMBI_THREAD_LOCAL size_t duumbi_function_id_stack_len = 0;
static DUUMBI_THREAD_LOCAL size_t duumbi_block_id_stack_len = 0;
static DUUMBI_THREAD_LOCAL size_t duumbi_function_id_stack_overflow = 0;
static DUUMBI_THREAD_LOCAL size_t duumbi_block_id_stack_overflow = 0;

static void duumbi_push_trace_id(int64_t *stack,
                                 size_t *len,
                                 size_t *overflow,
                                 int64_t trace_id) {
    if (*overflow > 0 || *len >= DUUMBI_TRACE_STACK_LIMIT) {
        (*overflow)++;
        return;
    }

    stack[*len] = trace_id;
    (*len)++;
}

static int64_t duumbi_pop_trace_id(int64_t *stack, size_t *len, size_t *overflow) {
    if (*overflow > 0) {
        (*overflow)--;
        return 0;
    }

    if (*len == 0) {
        return 0;
    }

    (*len)--;
    return stack[*len];
}

static const char *duumbi_telemetry_dir(void) {
    const char *dir = getenv(DUUMBI_TELEMETRY_DIR_ENV);
    if (dir != NULL && dir[0] != '\0') {
        return dir;
    }
    return DUUMBI_DEFAULT_TELEMETRY_DIR;
}

static int duumbi_is_path_sep(char ch) {
    return ch == '/' || ch == '\\';
}

static size_t duumbi_path_root_len(const char *path) {
#if defined(_WIN32)
    size_t len = strlen(path);
    if (len >= 2 &&
        ((path[0] >= 'A' && path[0] <= 'Z') || (path[0] >= 'a' && path[0] <= 'z')) &&
        path[1] == ':') {
        if (len >= 3 && duumbi_is_path_sep(path[2])) {
            return 3;
        }
        return 2;
    }

    if (len >= 2 && duumbi_is_path_sep(path[0]) && duumbi_is_path_sep(path[1])) {
        const char *cursor = path + 2;
        while (*cursor != '\0' && !duumbi_is_path_sep(*cursor)) {
            cursor++;
        }
        if (*cursor == '\0') {
            return len;
        }
        cursor++;
        while (*cursor != '\0' && !duumbi_is_path_sep(*cursor)) {
            cursor++;
        }
        if (*cursor == '\0') {
            return len;
        }
        return (size_t)(cursor - path + 1);
    }
#endif

    if (duumbi_is_path_sep(path[0])) {
        return 1;
    }
    return 0;
}

static int duumbi_mkdir_p(const char *dir) {
    char path[DUUMBI_PATH_BUFFER_LEN];
    size_t len = strlen(dir);
    if (len == 0 || len >= sizeof(path)) {
        return -1;
    }

    memcpy(path, dir, len + 1);
    size_t root_len = duumbi_path_root_len(path);
    if (root_len >= len) {
        return 0;
    }

    for (char *p = path + (root_len > 0 ? root_len : 1); *p != '\0'; p++) {
        if (duumbi_is_path_sep(*p)) {
            char saved = *p;
            *p = '\0';
            if (DUUMBI_MKDIR(path) != 0 && errno != EEXIST) {
                return -1;
            }
            *p = saved;
        }
    }

    if (DUUMBI_MKDIR(path) != 0 && errno != EEXIST) {
        return -1;
    }
    return 0;
}

static int duumbi_telemetry_path(char *buffer, size_t buffer_len, const char *file_name) {
    const char *dir = duumbi_telemetry_dir();
    if (duumbi_mkdir_p(dir) != 0) {
        return -1;
    }

    size_t dir_len = strlen(dir);
    const char *separator = "";
    if (dir_len > 0 && dir[dir_len - 1] != '/' && dir[dir_len - 1] != '\\') {
        separator = (const char[]){DUUMBI_PATH_SEP, '\0'};
    }

    int written = snprintf(buffer, buffer_len, "%s%s%s", dir, separator, file_name);
    if (written < 0 || (size_t)written >= buffer_len) {
        return -1;
    }
    return 0;
}

static FILE *duumbi_open_telemetry_file(const char *file_name) {
    char path[DUUMBI_PATH_BUFFER_LEN];
    if (duumbi_telemetry_path(path, sizeof(path), file_name) != 0) {
        fprintf(stderr, "duumbi telemetry warning: failed to resolve telemetry path\n");
        return NULL;
    }

    FILE *file = fopen(path, "a");
    if (file == NULL) {
        fprintf(stderr, "duumbi telemetry warning: failed to open %s\n", path);
    }
    return file;
}

static void duumbi_write_json_string(FILE *file, const char *value) {
    fputc('"', file);
    for (const unsigned char *p = (const unsigned char *)value; *p != '\0'; p++) {
        switch (*p) {
        case '"':
            fputs("\\\"", file);
            break;
        case '\\':
            fputs("\\\\", file);
            break;
        case '\n':
            fputs("\\n", file);
            break;
        case '\r':
            fputs("\\r", file);
            break;
        case '\t':
            fputs("\\t", file);
            break;
        default:
            if (*p < 0x20) {
                fprintf(file, "\\u%04x", *p);
            } else {
                fputc(*p, file);
            }
            break;
        }
    }
    fputc('"', file);
}

static void duumbi_write_trace_event(const char *event, int64_t trace_id) {
    if (!duumbi_trace_active) {
        return;
    }

    FILE *file = duumbi_open_telemetry_file("traces.jsonl");
    if (file == NULL) {
        return;
    }

    fprintf(file,
            "{\"schema_version\":\"%s\",\"event\":\"%s\",\"trace_id\":%lld,\"timestamp_ns\":0}\n",
            DUUMBI_TRACE_SCHEMA_VERSION,
            event,
            (long long)trace_id);
    fclose(file);
}

void duumbi_trace_init(void) {
    duumbi_trace_active = 1;
    duumbi_current_function_id = 0;
    duumbi_current_block_id = 0;
    duumbi_function_id_stack_len = 0;
    duumbi_block_id_stack_len = 0;
    duumbi_function_id_stack_overflow = 0;
    duumbi_block_id_stack_overflow = 0;
}

void duumbi_trace_function_enter(int64_t function_id) {
    duumbi_push_trace_id(duumbi_function_id_stack,
                         &duumbi_function_id_stack_len,
                         &duumbi_function_id_stack_overflow,
                         duumbi_current_function_id);
    duumbi_current_function_id = function_id;
    duumbi_write_trace_event("function_enter", function_id);
}

void duumbi_trace_function_exit(int64_t function_id) {
    duumbi_write_trace_event("function_exit", function_id);
    if (duumbi_current_function_id == function_id) {
        duumbi_current_function_id = duumbi_pop_trace_id(duumbi_function_id_stack,
                                                         &duumbi_function_id_stack_len,
                                                         &duumbi_function_id_stack_overflow);
    }
}

void duumbi_trace_block_enter(int64_t block_id) {
    duumbi_push_trace_id(duumbi_block_id_stack,
                         &duumbi_block_id_stack_len,
                         &duumbi_block_id_stack_overflow,
                         duumbi_current_block_id);
    duumbi_current_block_id = block_id;
    duumbi_write_trace_event("block_enter", block_id);
}

void duumbi_trace_block_exit(int64_t block_id) {
    duumbi_write_trace_event("block_exit", block_id);
    if (duumbi_current_block_id == block_id) {
        duumbi_current_block_id = duumbi_pop_trace_id(duumbi_block_id_stack,
                                                      &duumbi_block_id_stack_len,
                                                      &duumbi_block_id_stack_overflow);
    }
}

void duumbi_trace_panic(const char *msg) {
    if (!duumbi_trace_active) {
        return;
    }

    FILE *trace_file = duumbi_open_telemetry_file("traces.jsonl");
    if (trace_file != NULL) {
        fprintf(trace_file,
                "{\"schema_version\":\"%s\",\"event\":\"panic\",\"function_id\":%lld,\"block_id\":%lld,\"message\":",
                DUUMBI_TRACE_SCHEMA_VERSION,
                (long long)duumbi_current_function_id,
                (long long)duumbi_current_block_id);
        duumbi_write_json_string(trace_file, msg);
        fputs("}\n", trace_file);
        fclose(trace_file);
    }

    FILE *crash_file = duumbi_open_telemetry_file("crash_dump.jsonl");
    if (crash_file == NULL) {
        return;
    }

    fprintf(crash_file,
            "{\"schema_version\":\"%s\",\"event\":\"panic\",\"message\":",
            DUUMBI_CRASH_SCHEMA_VERSION);
    duumbi_write_json_string(crash_file, msg);
    fprintf(crash_file,
            ",\"function_id\":%lld,\"block_id\":%lld,\"trace_active\":true}\n",
            (long long)duumbi_current_function_id,
            (long long)duumbi_current_block_id);
    fclose(crash_file);
}

/* ── Panic ─────────────────────────────────────────────────────────── */

void duumbi_panic(const char *msg) {
    duumbi_trace_panic(msg);
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

/* ── Math (Phase 9A — link with -lm) ─────────────────────────────── */

double duumbi_sqrt(double x) {
    return sqrt(x);
}

double duumbi_pow(double base, double exp) {
    return pow(base, exp);
}

int64_t duumbi_powi64(int64_t base, int64_t exp) {
    if (exp < 0) return 0;  /* integer power of negative exponent → 0 */
    /* Use unsigned arithmetic to avoid signed overflow UB, then cast back. */
    uint64_t ubase = (uint64_t)base;
    uint64_t uresult = 1;
    uint64_t uexp = (uint64_t)exp;
    while (uexp > 0) {
        if (uexp & 1) uresult *= ubase;
        ubase *= ubase;
        uexp >>= 1;
    }
    return (int64_t)uresult;
}

double duumbi_fmod(double a, double b) {
    return fmod(a, b);
}

/* ── String utilities (Phase 9A stdlib) ──────────────────────────── */

void *duumbi_string_trim(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    uint64_t start = 0;
    while (start < s->len && (s->data[start] == ' ' || s->data[start] == '\t' ||
           s->data[start] == '\n' || s->data[start] == '\r')) {
        start++;
    }
    uint64_t end = s->len;
    while (end > start && (s->data[end - 1] == ' ' || s->data[end - 1] == '\t' ||
           s->data[end - 1] == '\n' || s->data[end - 1] == '\r')) {
        end--;
    }
    return duumbi_string_new(s->data + start, end - start);
}

void *duumbi_string_to_upper(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    DuumbiString *result = (DuumbiString *)duumbi_alloc(sizeof(DuumbiString) + s->len + 1);
    result->len = s->len;
    for (uint64_t i = 0; i < s->len; i++) {
        char c = s->data[i];
        result->data[i] = (c >= 'a' && c <= 'z') ? (char)(c - 32) : c;
    }
    result->data[s->len] = '\0';
    return result;
}

void *duumbi_string_to_lower(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    DuumbiString *result = (DuumbiString *)duumbi_alloc(sizeof(DuumbiString) + s->len + 1);
    result->len = s->len;
    for (uint64_t i = 0; i < s->len; i++) {
        char c = s->data[i];
        result->data[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;
    }
    result->data[s->len] = '\0';
    return result;
}

void *duumbi_string_replace(void *haystack, void *needle, void *replacement) {
    DuumbiString *h = (DuumbiString *)haystack;
    DuumbiString *n = (DuumbiString *)needle;
    DuumbiString *r = (DuumbiString *)replacement;

    if (n->len == 0 || h->len < n->len) {
        /* Empty needle or haystack too short: return a copy unchanged */
        return duumbi_string_new(h->data, h->len);
    }

    /* Find first occurrence only */
    uint64_t match_pos = h->len; /* sentinel: not found */
    for (uint64_t i = 0; i <= h->len - n->len; i++) {
        if (memcmp(h->data + i, n->data, (size_t)n->len) == 0) {
            match_pos = i;
            break;
        }
    }

    if (match_pos == h->len) {
        /* Not found: return a copy unchanged */
        return duumbi_string_new(h->data, h->len);
    }

    uint64_t new_len = h->len - n->len + r->len;
    DuumbiString *result = (DuumbiString *)duumbi_alloc(sizeof(DuumbiString) + new_len + 1);
    result->len = new_len;

    /* Copy prefix + replacement + suffix */
    memcpy(result->data, h->data, (size_t)match_pos);
    memcpy(result->data + match_pos, r->data, (size_t)r->len);
    memcpy(result->data + match_pos + r->len,
           h->data + match_pos + n->len,
           (size_t)(h->len - match_pos - n->len));
    result->data[new_len] = '\0';
    return result;
}

/* ── DUUMBI-378 text I/O and workspace-confined file APIs ─────────── */

static int duumbi_utf8_valid(const char *data, uint64_t len) {
    uint64_t i = 0;
    while (i < len) {
        unsigned char c = (unsigned char)data[i];
        if (c <= 0x7f) {
            i++;
        } else if ((c & 0xe0) == 0xc0) {
            if (i + 1 >= len) return 0;
            unsigned char c1 = (unsigned char)data[i + 1];
            if ((c1 & 0xc0) != 0x80 || c < 0xc2) return 0;
            i += 2;
        } else if ((c & 0xf0) == 0xe0) {
            if (i + 2 >= len) return 0;
            unsigned char c1 = (unsigned char)data[i + 1];
            unsigned char c2 = (unsigned char)data[i + 2];
            if ((c1 & 0xc0) != 0x80 || (c2 & 0xc0) != 0x80) return 0;
            if (c == 0xe0 && c1 < 0xa0) return 0;
            if (c == 0xed && c1 >= 0xa0) return 0;
            i += 3;
        } else if ((c & 0xf8) == 0xf0) {
            if (i + 3 >= len) return 0;
            unsigned char c1 = (unsigned char)data[i + 1];
            unsigned char c2 = (unsigned char)data[i + 2];
            unsigned char c3 = (unsigned char)data[i + 3];
            if ((c1 & 0xc0) != 0x80 || (c2 & 0xc0) != 0x80 || (c3 & 0xc0) != 0x80) return 0;
            if (c == 0xf0 && c1 < 0x90) return 0;
            if (c > 0xf4 || (c == 0xf4 && c1 > 0x8f)) return 0;
            i += 4;
        } else {
            return 0;
        }
    }
    return 1;
}

static void *duumbi_ok_i64(int64_t value) {
    return duumbi_result_new_ok(value);
}

static void *duumbi_ok_string(const char *data, uint64_t len) {
    return duumbi_result_new_ok((int64_t)(intptr_t)duumbi_string_new(data, len));
}

static void *duumbi_ok_bool(int8_t value) {
    return duumbi_result_new_ok((int64_t)value);
}

static void *duumbi_err_cstr(const char *message) {
    return duumbi_result_new_err((int64_t)(intptr_t)duumbi_string_new(message, strlen(message)));
}

static int duumbi_string_to_cstr(void *ptr, char *out, size_t out_len) {
    DuumbiString *s = (DuumbiString *)ptr;
    if (s == NULL || s->len == 0 || s->len >= out_len) {
        return -1;
    }
    for (uint64_t i = 0; i < s->len; i++) {
        if (s->data[i] == '\0') {
            return -1;
        }
    }
    memcpy(out, s->data, (size_t)s->len);
    out[s->len] = '\0';
    return 0;
}

static int duumbi_has_url_prefix(const char *path) {
    return strstr(path, "://") != NULL;
}

static int duumbi_normalize_relative_path(const char *input, char *out, size_t out_len) {
    if (input == NULL || input[0] == '\0') {
        return -1;
    }
    if (input[0] == '/' || input[0] == '\\' || input[0] == '~' || input[0] == '$' ||
        strchr(input, ':') != NULL || duumbi_has_url_prefix(input)) {
        return -1;
    }
    if (strchr(input, '\\') != NULL) {
        return -1;
    }

    out[0] = '\0';
    size_t out_used = 0;
    const char *cursor = input;
    while (*cursor != '\0') {
        while (*cursor == '/') {
            cursor++;
        }
        const char *start = cursor;
        while (*cursor != '\0' && *cursor != '/') {
            cursor++;
        }
        size_t len = (size_t)(cursor - start);
        if (len == 0) {
            continue;
        }
        if (len == 1 && start[0] == '.') {
            continue;
        }
        if (len == 2 && start[0] == '.' && start[1] == '.') {
            return -1;
        }
        if (out_used != 0) {
            if (out_used + 1 >= out_len) return -1;
            out[out_used++] = '/';
        }
        if (out_used + len >= out_len) return -1;
        memcpy(out + out_used, start, len);
        out_used += len;
        out[out_used] = '\0';
    }

    return out_used == 0 ? -1 : 0;
}

static int duumbi_join_workspace_path(const char *normalized, char *out, size_t out_len) {
    const char *root = getenv(DUUMBI_WORKSPACE_ROOT_ENV);
    if (root == NULL || root[0] == '\0') {
        return -1;
    }
    int written = snprintf(out, out_len, "%s%c%s", root, DUUMBI_PATH_SEP, normalized);
    if (written < 0 || (size_t)written >= out_len) {
        return -1;
    }
    return 0;
}

static int duumbi_workspace_root_available(void) {
    const char *root = getenv(DUUMBI_WORKSPACE_ROOT_ENV);
    return root != NULL && root[0] != '\0';
}

static int duumbi_canonical_workspace_root(char *out, size_t out_len) {
    const char *root = getenv(DUUMBI_WORKSPACE_ROOT_ENV);
    if (root == NULL || root[0] == '\0') {
        return -1;
    }
    char resolved[DUUMBI_PATH_BUFFER_LEN];
    if (DUUMBI_REALPATH(root, resolved) == NULL) {
        return -1;
    }
    size_t len = strlen(resolved);
    if (len == 0 || len >= out_len) {
        return -1;
    }
    memcpy(out, resolved, len + 1);
    return 0;
}

static int duumbi_path_inside_root(const char *root, const char *path) {
    size_t root_len = strlen(root);
    if (strncmp(root, path, root_len) != 0) {
        return 0;
    }
    return path[root_len] == '\0' || path[root_len] == '/' || path[root_len] == '\\';
}

static int duumbi_nearest_existing_parent_inside_root(const char *joined, const char *root) {
    char probe[DUUMBI_PATH_BUFFER_LEN];
    char resolved[DUUMBI_PATH_BUFFER_LEN];
    size_t joined_len = strlen(joined);
    if (joined_len == 0 || joined_len >= sizeof(probe)) {
        return 0;
    }
    memcpy(probe, joined, joined_len + 1);

    while (1) {
        char *slash = strrchr(probe, DUUMBI_PATH_SEP);
        if (slash == NULL) {
            return 0;
        }
        if (slash == probe) {
            probe[1] = '\0';
        } else {
            *slash = '\0';
        }

        if (DUUMBI_REALPATH(probe, resolved) != NULL) {
            return duumbi_path_inside_root(root, resolved);
        }
        if (slash == probe) {
            return 0;
        }
    }
}

static int duumbi_resolve_existing_workspace_path(void *path_ptr, char *out, size_t out_len) {
    char raw[DUUMBI_PATH_BUFFER_LEN];
    char normalized[DUUMBI_PATH_BUFFER_LEN];
    char joined[DUUMBI_PATH_BUFFER_LEN];
    char root[DUUMBI_PATH_BUFFER_LEN];
    char resolved[DUUMBI_PATH_BUFFER_LEN];

    if (duumbi_string_to_cstr(path_ptr, raw, sizeof(raw)) != 0 ||
        duumbi_normalize_relative_path(raw, normalized, sizeof(normalized)) != 0 ||
        duumbi_join_workspace_path(normalized, joined, sizeof(joined)) != 0 ||
        duumbi_canonical_workspace_root(root, sizeof(root)) != 0 ||
        DUUMBI_REALPATH(joined, resolved) == NULL ||
        !duumbi_path_inside_root(root, resolved)) {
        return -1;
    }

    size_t len = strlen(resolved);
    if (len >= out_len) {
        return -1;
    }
    memcpy(out, resolved, len + 1);
    return 0;
}

static int duumbi_resolve_write_workspace_path(void *path_ptr, char *out, size_t out_len) {
    char raw[DUUMBI_PATH_BUFFER_LEN];
    char normalized[DUUMBI_PATH_BUFFER_LEN];
    char joined[DUUMBI_PATH_BUFFER_LEN];
    char root[DUUMBI_PATH_BUFFER_LEN];
    char resolved[DUUMBI_PATH_BUFFER_LEN];

    if (duumbi_string_to_cstr(path_ptr, raw, sizeof(raw)) != 0 ||
        duumbi_normalize_relative_path(raw, normalized, sizeof(normalized)) != 0 ||
        duumbi_join_workspace_path(normalized, joined, sizeof(joined)) != 0 ||
        duumbi_canonical_workspace_root(root, sizeof(root)) != 0) {
        return -1;
    }

    if (DUUMBI_REALPATH(joined, resolved) != NULL && !duumbi_path_inside_root(root, resolved)) {
        return -1;
    }

    char parent[DUUMBI_PATH_BUFFER_LEN];
    size_t joined_len = strlen(joined);
    if (joined_len >= sizeof(parent)) {
        return -1;
    }
    memcpy(parent, joined, joined_len + 1);
    char *slash = strrchr(parent, DUUMBI_PATH_SEP);
    if (slash == NULL) {
        return -1;
    }
    *slash = '\0';
    if (DUUMBI_REALPATH(parent, resolved) == NULL || !duumbi_path_inside_root(root, resolved)) {
        return -1;
    }

    if (joined_len >= out_len) {
        return -1;
    }
    memcpy(out, joined, joined_len + 1);
    return 0;
}

void *duumbi_read_line(void) {
    size_t capacity = 128;
    size_t len = 0;
    char *buffer = (char *)malloc(capacity);
    if (buffer == NULL) {
        return duumbi_err_cstr("io_error: out of memory");
    }

    int ch;
    while ((ch = fgetc(stdin)) != EOF) {
        if (len + 1 >= capacity) {
            capacity *= 2;
            char *grown = (char *)realloc(buffer, capacity);
            if (grown == NULL) {
                free(buffer);
                return duumbi_err_cstr("io_error: out of memory");
            }
            buffer = grown;
        }
        buffer[len++] = (char)ch;
        if (ch == '\n') {
            break;
        }
    }

    if (ferror(stdin)) {
        free(buffer);
        return duumbi_err_cstr("io_error: failed to read stdin");
    }
    if (len == 0 && ch == EOF) {
        free(buffer);
        return duumbi_err_cstr("stdin_eof: end of input");
    }
    if (len > 0 && buffer[len - 1] == '\n') {
        len--;
        if (len > 0 && buffer[len - 1] == '\r') {
            len--;
        }
    }
    if (!duumbi_utf8_valid(buffer, (uint64_t)len)) {
        free(buffer);
        return duumbi_err_cstr("stdin_invalid_utf8: stdin line is not valid UTF-8");
    }

    void *result = duumbi_ok_string(buffer, (uint64_t)len);
    free(buffer);
    return result;
}

void *duumbi_print_ln(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    if (s == NULL) {
        return duumbi_err_cstr("io_error: print_ln received null string");
    }
    if ((s->len > 0 && fwrite(s->data, 1, (size_t)s->len, stdout) != s->len) ||
        fputc('\n', stdout) == EOF || fflush(stdout) != 0) {
        return duumbi_err_cstr("stdout_write_failed: failed to write stdout");
    }
    return duumbi_ok_i64(0);
}

void *duumbi_file_read(void *path_ptr, int64_t max_bytes) {
    if (max_bytes < 0) {
        return duumbi_err_cstr("byte_limit: max_bytes must be non-negative");
    }
    if (!duumbi_workspace_root_available()) {
        return duumbi_err_cstr("workspace_root_unavailable: DUUMBI_WORKSPACE_ROOT is not set");
    }

    char path[DUUMBI_PATH_BUFFER_LEN];
    if (duumbi_resolve_existing_workspace_path(path_ptr, path, sizeof(path)) != 0) {
        return duumbi_err_cstr("path_policy: path is outside the workspace or does not exist");
    }
    struct stat st;
    if (stat(path, &st) != 0) {
        return duumbi_err_cstr("io_error: failed to inspect file");
    }
    if (!S_ISREG(st.st_mode)) {
        return duumbi_err_cstr("not_file: path is not a file");
    }

    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return duumbi_err_cstr("io_error: failed to open file");
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        fclose(file);
        return duumbi_err_cstr("io_error: failed to inspect file");
    }
    long size = ftell(file);
    if (size < 0) {
        fclose(file);
        return duumbi_err_cstr("io_error: failed to inspect file");
    }
    if ((int64_t)size > max_bytes) {
        fclose(file);
        return duumbi_err_cstr("byte_limit: file exceeds max_bytes");
    }
    rewind(file);

    char *buffer = (char *)malloc((size_t)size + 1);
    if (buffer == NULL) {
        fclose(file);
        return duumbi_err_cstr("io_error: out of memory");
    }
    size_t read = fread(buffer, 1, (size_t)size, file);
    int read_error = ferror(file);
    fclose(file);
    if (read_error || read != (size_t)size) {
        free(buffer);
        return duumbi_err_cstr("io_error: failed to read file");
    }
    if (!duumbi_utf8_valid(buffer, (uint64_t)size)) {
        free(buffer);
        return duumbi_err_cstr("invalid_utf8: file is not valid UTF-8");
    }

    void *result = duumbi_ok_string(buffer, (uint64_t)size);
    free(buffer);
    return result;
}

void *duumbi_file_write(void *path_ptr, void *contents_ptr) {
    DuumbiString *contents = (DuumbiString *)contents_ptr;
    if (contents == NULL) {
        return duumbi_err_cstr("io_error: write_file received null contents");
    }
    if (!duumbi_utf8_valid(contents->data, contents->len)) {
        return duumbi_err_cstr("invalid_utf8: write_file contents are not valid UTF-8");
    }
    if (!duumbi_workspace_root_available()) {
        return duumbi_err_cstr("workspace_root_unavailable: DUUMBI_WORKSPACE_ROOT is not set");
    }

    char path[DUUMBI_PATH_BUFFER_LEN];
    if (duumbi_resolve_write_workspace_path(path_ptr, path, sizeof(path)) != 0) {
        return duumbi_err_cstr("path_policy: path is outside the workspace");
    }

    FILE *file = fopen(path, "wb");
    if (file == NULL) {
        return duumbi_err_cstr("io_error: failed to open file for writing");
    }
    size_t written = fwrite(contents->data, 1, (size_t)contents->len, file);
    int close_error = fclose(file);
    if (written != contents->len || close_error != 0) {
        return duumbi_err_cstr("io_error: failed to write file");
    }
    return duumbi_ok_i64(0);
}

void *duumbi_file_exists(void *path_ptr) {
    char raw[DUUMBI_PATH_BUFFER_LEN];
    char normalized[DUUMBI_PATH_BUFFER_LEN];
    char joined[DUUMBI_PATH_BUFFER_LEN];
    char root[DUUMBI_PATH_BUFFER_LEN];
    char resolved[DUUMBI_PATH_BUFFER_LEN];

    if (!duumbi_workspace_root_available()) {
        return duumbi_err_cstr("workspace_root_unavailable: DUUMBI_WORKSPACE_ROOT is not set");
    }
    if (duumbi_string_to_cstr(path_ptr, raw, sizeof(raw)) != 0 ||
        duumbi_normalize_relative_path(raw, normalized, sizeof(normalized)) != 0 ||
        duumbi_join_workspace_path(normalized, joined, sizeof(joined)) != 0 ||
        duumbi_canonical_workspace_root(root, sizeof(root)) != 0) {
        return duumbi_err_cstr("path_policy: path is outside the workspace");
    }
    if (DUUMBI_REALPATH(joined, resolved) == NULL) {
        if (!duumbi_nearest_existing_parent_inside_root(joined, root)) {
            return duumbi_err_cstr("path_policy: path is outside the workspace");
        }
        return duumbi_ok_bool(0);
    }
    if (!duumbi_path_inside_root(root, resolved)) {
        return duumbi_err_cstr("path_policy: path is outside the workspace");
    }
    return duumbi_ok_bool(1);
}

static int duumbi_compare_names(const void *a, const void *b) {
    const char *const *sa = (const char *const *)a;
    const char *const *sb = (const char *const *)b;
    return strcmp(*sa, *sb);
}

static int duumbi_collect_dir_name(char ***names,
                                   size_t *count,
                                   size_t *capacity,
                                   const char *name) {
    if (strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
        return 0;
    }
    size_t len = strlen(name);
    if (!duumbi_utf8_valid(name, (uint64_t)len)) {
        return -1;
    }
    if (*count == *capacity) {
        *capacity *= 2;
        char **grown = (char **)realloc(*names, *capacity * sizeof(char *));
        if (grown == NULL) {
            return -1;
        }
        *names = grown;
    }
    (*names)[*count] = (char *)malloc(len + 1);
    if ((*names)[*count] == NULL) {
        return -1;
    }
    memcpy((*names)[*count], name, len + 1);
    (*count)++;
    return 0;
}

static void duumbi_free_dir_names(char **names, size_t count) {
    for (size_t i = 0; i < count; i++) {
        free(names[i]);
    }
    free(names);
}

void *duumbi_list_dir(void *path_ptr) {
    if (!duumbi_workspace_root_available()) {
        return duumbi_err_cstr("workspace_root_unavailable: DUUMBI_WORKSPACE_ROOT is not set");
    }

    char path[DUUMBI_PATH_BUFFER_LEN];
    if (duumbi_resolve_existing_workspace_path(path_ptr, path, sizeof(path)) != 0) {
        return duumbi_err_cstr("path_policy: path is outside the workspace or does not exist");
    }
    struct stat st;
    if (stat(path, &st) != 0) {
        return duumbi_err_cstr("io_error: failed to inspect directory");
    }
    if (!S_ISDIR(st.st_mode)) {
        return duumbi_err_cstr("not_directory: path is not a directory");
    }

    size_t count = 0;
    size_t capacity = 8;
    char **names = (char **)calloc(capacity, sizeof(char *));
    if (names == NULL) {
        return duumbi_err_cstr("io_error: out of memory");
    }

#if defined(_WIN32)
    char pattern[DUUMBI_PATH_BUFFER_LEN];
    int written = snprintf(pattern, sizeof(pattern), "%s\\*", path);
    if (written < 0 || (size_t)written >= sizeof(pattern)) {
        free(names);
        return duumbi_err_cstr("path_policy: directory path is too long");
    }

    WIN32_FIND_DATAA data;
    HANDLE handle = FindFirstFileA(pattern, &data);
    if (handle == INVALID_HANDLE_VALUE) {
        free(names);
        return duumbi_err_cstr("io_error: failed to open directory");
    }
    do {
        if (duumbi_collect_dir_name(&names, &count, &capacity, data.cFileName) != 0) {
            FindClose(handle);
            duumbi_free_dir_names(names, count);
            return duumbi_err_cstr("invalid_utf8: failed to read directory entry");
        }
    } while (FindNextFileA(handle, &data) != 0);
    FindClose(handle);
#else
    DIR *dir = opendir(path);
    if (dir == NULL) {
        free(names);
        return duumbi_err_cstr("io_error: failed to open directory");
    }

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        if (duumbi_collect_dir_name(&names, &count, &capacity, entry->d_name) != 0) {
            closedir(dir);
            duumbi_free_dir_names(names, count);
            return duumbi_err_cstr("invalid_utf8: failed to read directory entry");
        }
    }
    closedir(dir);
#endif
    qsort(names, count, sizeof(char *), duumbi_compare_names);

    void *array = duumbi_array_new(8);
    for (size_t i = 0; i < count; i++) {
        void *name = duumbi_string_new(names[i], (uint64_t)strlen(names[i]));
        array = duumbi_array_push(array, (int64_t)(intptr_t)name);
    }
    duumbi_free_dir_names(names, count);
    return duumbi_result_new_ok((int64_t)(intptr_t)array);
}

void *duumbi_path_join(void *left_ptr, void *right_ptr) {
    char left[DUUMBI_PATH_BUFFER_LEN];
    char right[DUUMBI_PATH_BUFFER_LEN];
    char joined[DUUMBI_PATH_BUFFER_LEN];
    char normalized[DUUMBI_PATH_BUFFER_LEN];

    if (duumbi_string_to_cstr(left_ptr, left, sizeof(left)) != 0 ||
        duumbi_string_to_cstr(right_ptr, right, sizeof(right)) != 0) {
        return duumbi_err_cstr("path_policy: path_join components must be non-empty");
    }
    int written = snprintf(joined, sizeof(joined), "%s/%s", left, right);
    if (written < 0 || (size_t)written >= sizeof(joined) ||
        duumbi_normalize_relative_path(joined, normalized, sizeof(normalized)) != 0) {
        return duumbi_err_cstr("path_policy: path_join produced an invalid path");
    }
    return duumbi_ok_string(normalized, (uint64_t)strlen(normalized));
}
