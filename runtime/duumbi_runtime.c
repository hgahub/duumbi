#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <errno.h>
#include <ctype.h>
#include <time.h>
#include <sys/stat.h>
#include <sys/types.h>

#if defined(_WIN32)
#include <winsock2.h>
#include <ws2tcpip.h>
#include <direct.h>
#include <io.h>
#include <process.h>
#include <windows.h>
#define DUUMBI_MKDIR(path) _mkdir(path)
#define DUUMBI_PATH_SEP '\\'
#define DUUMBI_REALPATH(path, resolved) _fullpath((resolved), (path), DUUMBI_PATH_BUFFER_LEN)
#define DUUMBI_PROCESS_ID() _getpid()
#else
#include <dirent.h>
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>
#include <netdb.h>
#include <sys/socket.h>
#define DUUMBI_MKDIR(path) mkdir(path, 0777)
#define DUUMBI_PATH_SEP '/'
#define DUUMBI_REALPATH(path, resolved) realpath((path), (resolved))
#define DUUMBI_PROCESS_ID() getpid()
#endif

#include <curl/curl.h>

#define SQLITE_OMIT_LOAD_EXTENSION 1
#include "third_party/sqlite/sqlite3.c"

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
#define DUUMBI_STDIN_LINE_MAX 65536
#define DUUMBI_TRACE_STACK_LIMIT 1024
#define DUUMBI_WORKSPACE_ROOT_ENV "DUUMBI_WORKSPACE_ROOT"
#define DUUMBI_JSON_MAX_PARSE_DEPTH 512
#define DUUMBI_DB_BUSY_TIMEOUT_MS 1000
#define DUUMBI_DB_MAX_ROWS 10000
#define DUUMBI_DB_MAX_CELL_BYTES (8 * 1024 * 1024)
#define DUUMBI_DB_MAX_COLUMNS 256

#if defined(_WIN32)
#define DUUMBI_TEMP_WRITE_MODE "wb"
#else
#define DUUMBI_TEMP_WRITE_MODE "wbx"
#endif

/* ── Internal types ────────────────────────────────────────────────── */

/* Keep approved DUUMBI-380 runtime dependencies link-visible before public
 * HTTP/DB APIs are added. Cycle 1 uses this as a narrow feasibility proof. */
int64_t duumbi_dependency_probe(void) {
    curl_version_info_data *curl_info = curl_version_info(CURLVERSION_NOW);
    int64_t curl_version = curl_info != NULL ? (int64_t)curl_info->version_num : 0;
    return curl_version + (int64_t)sqlite3_libversion_number();
}

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

#define DUUMBI_CURL_INIT_UNINITIALIZED 0
#define DUUMBI_CURL_INIT_INITIALIZING 1
#define DUUMBI_CURL_INIT_READY 2
#define DUUMBI_CURL_INIT_FAILED 3

static volatile int duumbi_curl_init_state = DUUMBI_CURL_INIT_UNINITIALIZED;

static int duumbi_atomic_load_int(volatile int *value) {
#if defined(_WIN32)
    return (int)InterlockedCompareExchange((volatile LONG *)value, 0, 0);
#else
    __sync_synchronize();
    return *value;
#endif
}

static void duumbi_atomic_store_int(volatile int *value, int new_value) {
#if defined(_WIN32)
    InterlockedExchange((volatile LONG *)value, (LONG)new_value);
#else
    __sync_lock_test_and_set(value, new_value);
    __sync_synchronize();
#endif
}

static int duumbi_atomic_compare_exchange_int(
    volatile int *value,
    int expected,
    int desired
) {
#if defined(_WIN32)
    return InterlockedCompareExchange((volatile LONG *)value, (LONG)desired, (LONG)expected) ==
           (LONG)expected;
#else
    return __sync_bool_compare_and_swap(value, expected, desired);
#endif
}

static void duumbi_yield_while_initializing(void) {
#if defined(_WIN32)
    Sleep(0);
#else
    struct timespec delay;
    delay.tv_sec = 0;
    delay.tv_nsec = 1000000L;
    nanosleep(&delay, NULL);
#endif
}

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

/* ── JSON (owned recursive runtime tree) ──────────────────────────── */

typedef enum {
    DUUMBI_JSON_NULL,
    DUUMBI_JSON_BOOL,
    DUUMBI_JSON_NUMBER,
    DUUMBI_JSON_STRING,
    DUUMBI_JSON_ARRAY,
    DUUMBI_JSON_OBJECT
} DuumbiJsonKind;

typedef struct DuumbiJson DuumbiJson;

typedef struct {
    char       *key;
    size_t      key_len;
    DuumbiJson *value;
} DuumbiJsonObjectEntry;

struct DuumbiJson {
    DuumbiJsonKind kind;
    int            bool_value;
    double         number_value;
    char          *string_value;
    size_t         string_len;
    DuumbiJson   **array_items;
    uint64_t       array_len;
    uint64_t       array_cap;
    DuumbiJsonObjectEntry *object_entries;
    uint64_t       object_len;
    uint64_t       object_cap;
};

typedef struct {
    const char *cursor;
    const char *end;
    const char *error;
} DuumbiJsonParser;

typedef struct {
    char   *data;
    size_t  len;
    size_t  cap;
} DuumbiJsonBuffer;

static void *duumbi_json_err(const char *message) {
    void *err = duumbi_string_new(message, (uint64_t)strlen(message));
    return duumbi_result_new_err((int64_t)(intptr_t)err);
}

static void *duumbi_json_ok_ptr(void *ptr) {
    return duumbi_result_new_ok((int64_t)(intptr_t)ptr);
}

static void *duumbi_json_ok_i64(int64_t value) {
    return duumbi_result_new_ok(value);
}

static DuumbiJson *duumbi_json_new(DuumbiJsonKind kind) {
    DuumbiJson *json = (DuumbiJson *)duumbi_alloc(sizeof(DuumbiJson));
    memset(json, 0, sizeof(DuumbiJson));
    json->kind = kind;
    return json;
}

static char *duumbi_json_strndup(const char *data, size_t len) {
    char *out = (char *)duumbi_alloc((uint64_t)len + 1);
    memcpy(out, data, len);
    out[len] = '\0';
    return out;
}

void duumbi_json_free(void *value) {
    DuumbiJson *json = (DuumbiJson *)value;
    if (json == NULL) {
        return;
    }

    switch (json->kind) {
        case DUUMBI_JSON_STRING:
            duumbi_dealloc(json->string_value);
            break;
        case DUUMBI_JSON_ARRAY:
            for (uint64_t i = 0; i < json->array_len; i++) {
                duumbi_json_free(json->array_items[i]);
            }
            duumbi_dealloc(json->array_items);
            break;
        case DUUMBI_JSON_OBJECT:
            for (uint64_t i = 0; i < json->object_len; i++) {
                duumbi_dealloc(json->object_entries[i].key);
                duumbi_json_free(json->object_entries[i].value);
            }
            duumbi_dealloc(json->object_entries);
            break;
        default:
            break;
    }

    duumbi_dealloc(json);
}

static DuumbiJson *duumbi_json_clone(const DuumbiJson *json) {
    if (json == NULL) {
        return NULL;
    }

    DuumbiJson *copy = duumbi_json_new(json->kind);
    copy->bool_value = json->bool_value;
    copy->number_value = json->number_value;
    if (json->kind == DUUMBI_JSON_STRING) {
        copy->string_len = json->string_len;
        copy->string_value = duumbi_json_strndup(json->string_value, json->string_len);
    } else if (json->kind == DUUMBI_JSON_ARRAY) {
        copy->array_len = json->array_len;
        copy->array_cap = json->array_len;
        if (copy->array_len > 0) {
            copy->array_items = (DuumbiJson **)duumbi_alloc(
                sizeof(DuumbiJson *) * copy->array_len);
            for (uint64_t i = 0; i < copy->array_len; i++) {
                copy->array_items[i] = duumbi_json_clone(json->array_items[i]);
            }
        }
    } else if (json->kind == DUUMBI_JSON_OBJECT) {
        copy->object_len = json->object_len;
        copy->object_cap = json->object_len;
        if (copy->object_len > 0) {
            copy->object_entries = (DuumbiJsonObjectEntry *)duumbi_alloc(
                sizeof(DuumbiJsonObjectEntry) * copy->object_len);
            for (uint64_t i = 0; i < copy->object_len; i++) {
                copy->object_entries[i].key_len = json->object_entries[i].key_len;
                copy->object_entries[i].key = duumbi_json_strndup(
                    json->object_entries[i].key,
                    json->object_entries[i].key_len
                );
                copy->object_entries[i].value =
                    duumbi_json_clone(json->object_entries[i].value);
            }
        }
    }
    return copy;
}

static void duumbi_json_skip_ws(DuumbiJsonParser *parser) {
    while (parser->cursor < parser->end &&
           (*parser->cursor == ' ' || *parser->cursor == '\n' ||
            *parser->cursor == '\r' || *parser->cursor == '\t')) {
        parser->cursor++;
    }
}

static int duumbi_json_hex(char ch) {
    if (ch >= '0' && ch <= '9') return ch - '0';
    if (ch >= 'a' && ch <= 'f') return 10 + ch - 'a';
    if (ch >= 'A' && ch <= 'F') return 10 + ch - 'A';
    return -1;
}

static int duumbi_json_buffer_init(DuumbiJsonBuffer *buf) {
    buf->cap = 64;
    buf->len = 0;
    buf->data = (char *)duumbi_alloc((uint64_t)buf->cap);
    return 1;
}

static int duumbi_json_buffer_reserve(DuumbiJsonBuffer *buf, size_t extra) {
    if (buf->len + extra + 1 <= buf->cap) {
        return 1;
    }
    size_t new_cap = buf->cap;
    while (buf->len + extra + 1 > new_cap) {
        new_cap *= 2;
    }
    char *new_data = (char *)realloc(buf->data, new_cap);
    if (new_data == NULL) duumbi_panic("out of memory on JSON buffer grow");
    buf->data = new_data;
    buf->cap = new_cap;
    return 1;
}

static int duumbi_json_buffer_char(DuumbiJsonBuffer *buf, char ch) {
    if (!duumbi_json_buffer_reserve(buf, 1)) {
        return 0;
    }
    buf->data[buf->len++] = ch;
    buf->data[buf->len] = '\0';
    return 1;
}

static int duumbi_json_buffer_text(DuumbiJsonBuffer *buf, const char *text) {
    size_t len = strlen(text);
    if (!duumbi_json_buffer_reserve(buf, len)) {
        return 0;
    }
    memcpy(buf->data + buf->len, text, len);
    buf->len += len;
    buf->data[buf->len] = '\0';
    return 1;
}

static int duumbi_json_buffer_utf8(DuumbiJsonBuffer *buf, uint32_t codepoint) {
    if (codepoint <= 0x7f) {
        return duumbi_json_buffer_char(buf, (char)codepoint);
    }
    if (codepoint <= 0x7ff) {
        if (!duumbi_json_buffer_reserve(buf, 2)) return 0;
        buf->data[buf->len++] = (char)(0xc0 | (codepoint >> 6));
        buf->data[buf->len++] = (char)(0x80 | (codepoint & 0x3f));
    } else if (codepoint <= 0xffff) {
        if (codepoint >= 0xd800 && codepoint <= 0xdfff) return 0;
        if (!duumbi_json_buffer_reserve(buf, 3)) return 0;
        buf->data[buf->len++] = (char)(0xe0 | (codepoint >> 12));
        buf->data[buf->len++] = (char)(0x80 | ((codepoint >> 6) & 0x3f));
        buf->data[buf->len++] = (char)(0x80 | (codepoint & 0x3f));
    } else if (codepoint <= 0x10ffff) {
        if (!duumbi_json_buffer_reserve(buf, 4)) return 0;
        buf->data[buf->len++] = (char)(0xf0 | (codepoint >> 18));
        buf->data[buf->len++] = (char)(0x80 | ((codepoint >> 12) & 0x3f));
        buf->data[buf->len++] = (char)(0x80 | ((codepoint >> 6) & 0x3f));
        buf->data[buf->len++] = (char)(0x80 | (codepoint & 0x3f));
    } else {
        return 0;
    }
    buf->data[buf->len] = '\0';
    return 1;
}

static int duumbi_json_parse_hex4(DuumbiJsonParser *parser, uint32_t *out) {
    if (parser->end - parser->cursor < 4) {
        parser->error = "Short JSON unicode escape";
        return 0;
    }
    uint32_t code = 0;
    for (int i = 0; i < 4; i++) {
        int hex = duumbi_json_hex(parser->cursor[i]);
        if (hex < 0) {
            parser->error = "Invalid JSON unicode escape";
            return 0;
        }
        code = (code << 4) | (uint32_t)hex;
    }
    parser->cursor += 4;
    *out = code;
    return 1;
}

static char *duumbi_json_parse_string_raw(DuumbiJsonParser *parser, size_t *out_len) {
    if (parser->cursor >= parser->end || *parser->cursor != '"') {
        parser->error = "JSON string expected";
        return NULL;
    }
    parser->cursor++;

    DuumbiJsonBuffer buf;
    duumbi_json_buffer_init(&buf);
    while (parser->cursor < parser->end) {
        unsigned char ch = (unsigned char)*parser->cursor++;
        if (ch == '"') {
            char *result = duumbi_json_strndup(buf.data, buf.len);
            if (out_len != NULL) *out_len = buf.len;
            duumbi_dealloc(buf.data);
            return result;
        }
        if (ch < 0x20) {
            parser->error = "Invalid control character in JSON string";
            duumbi_dealloc(buf.data);
            return NULL;
        }
        if (ch != '\\') {
            duumbi_json_buffer_char(&buf, (char)ch);
            continue;
        }

        if (parser->cursor >= parser->end) {
            parser->error = "Unterminated JSON escape";
            duumbi_dealloc(buf.data);
            return NULL;
        }
        char esc = *parser->cursor++;
        switch (esc) {
            case '"': duumbi_json_buffer_char(&buf, '"'); break;
            case '\\': duumbi_json_buffer_char(&buf, '\\'); break;
            case '/': duumbi_json_buffer_char(&buf, '/'); break;
            case 'b': duumbi_json_buffer_char(&buf, '\b'); break;
            case 'f': duumbi_json_buffer_char(&buf, '\f'); break;
            case 'n': duumbi_json_buffer_char(&buf, '\n'); break;
            case 'r': duumbi_json_buffer_char(&buf, '\r'); break;
            case 't': duumbi_json_buffer_char(&buf, '\t'); break;
            case 'u': {
                uint32_t code = 0;
                if (!duumbi_json_parse_hex4(parser, &code)) {
                    duumbi_dealloc(buf.data);
                    return NULL;
                }
                if (code >= 0xd800 && code <= 0xdbff) {
                    if (parser->end - parser->cursor < 6 ||
                        parser->cursor[0] != '\\' || parser->cursor[1] != 'u') {
                        parser->error = "Missing JSON unicode low surrogate";
                        duumbi_dealloc(buf.data);
                        return NULL;
                    }
                    parser->cursor += 2;
                    uint32_t low = 0;
                    if (!duumbi_json_parse_hex4(parser, &low) || low < 0xdc00 || low > 0xdfff) {
                        parser->error = "Invalid JSON unicode low surrogate";
                        duumbi_dealloc(buf.data);
                        return NULL;
                    }
                    code = 0x10000 + (((code - 0xd800) << 10) | (low - 0xdc00));
                } else if (code >= 0xdc00 && code <= 0xdfff) {
                    parser->error = "Unexpected JSON unicode low surrogate";
                    duumbi_dealloc(buf.data);
                    return NULL;
                }
                if (!duumbi_json_buffer_utf8(&buf, code)) {
                    parser->error = "Invalid JSON unicode codepoint";
                    duumbi_dealloc(buf.data);
                    return NULL;
                }
                break;
            }
            default:
                parser->error = "Invalid JSON string escape";
                duumbi_dealloc(buf.data);
                return NULL;
        }
    }

    parser->error = "Unterminated JSON string";
    duumbi_dealloc(buf.data);
    return NULL;
}

static DuumbiJson *duumbi_json_parse_value(DuumbiJsonParser *parser, uint32_t depth);

static int duumbi_json_array_push_item(DuumbiJson *array, DuumbiJson *item) {
    if (array->array_len == array->array_cap) {
        uint64_t new_cap = array->array_cap == 0 ? 4 : array->array_cap * 2;
        DuumbiJson **new_items =
            (DuumbiJson **)realloc(array->array_items, sizeof(DuumbiJson *) * new_cap);
        if (new_items == NULL) {
            return 0;
        }
        array->array_items = new_items;
        array->array_cap = new_cap;
    }
    array->array_items[array->array_len++] = item;
    return 1;
}

static int duumbi_json_object_add(
    DuumbiJson *object,
    char *key,
    size_t key_len,
    DuumbiJson *value
) {
    if (object->object_len == object->object_cap) {
        uint64_t new_cap = object->object_cap == 0 ? 4 : object->object_cap * 2;
        DuumbiJsonObjectEntry *new_entries = (DuumbiJsonObjectEntry *)realloc(
            object->object_entries, sizeof(DuumbiJsonObjectEntry) * new_cap);
        if (new_entries == NULL) {
            return 0;
        }
        object->object_entries = new_entries;
        object->object_cap = new_cap;
    }
    object->object_entries[object->object_len].key = key;
    object->object_entries[object->object_len].key_len = key_len;
    object->object_entries[object->object_len].value = value;
    object->object_len++;
    return 1;
}

static DuumbiJson *duumbi_json_parse_array(DuumbiJsonParser *parser, uint32_t depth) {
    parser->cursor++;
    DuumbiJson *array = duumbi_json_new(DUUMBI_JSON_ARRAY);
    duumbi_json_skip_ws(parser);
    if (parser->cursor < parser->end && *parser->cursor == ']') {
        parser->cursor++;
        return array;
    }

    while (parser->cursor < parser->end) {
        DuumbiJson *item = duumbi_json_parse_value(parser, depth + 1);
        if (item == NULL) {
            duumbi_json_free(array);
            return NULL;
        }
        if (!duumbi_json_array_push_item(array, item)) {
            parser->error = "Out of memory while parsing JSON array";
            duumbi_json_free(item);
            duumbi_json_free(array);
            return NULL;
        }
        duumbi_json_skip_ws(parser);
        if (parser->cursor < parser->end && *parser->cursor == ',') {
            parser->cursor++;
            duumbi_json_skip_ws(parser);
            continue;
        }
        if (parser->cursor < parser->end && *parser->cursor == ']') {
            parser->cursor++;
            return array;
        }
        parser->error = "Expected ',' or ']' in JSON array";
        duumbi_json_free(array);
        return NULL;
    }

    parser->error = "Unterminated JSON array";
    duumbi_json_free(array);
    return NULL;
}

static DuumbiJson *duumbi_json_parse_object(DuumbiJsonParser *parser, uint32_t depth) {
    parser->cursor++;
    DuumbiJson *object = duumbi_json_new(DUUMBI_JSON_OBJECT);
    duumbi_json_skip_ws(parser);
    if (parser->cursor < parser->end && *parser->cursor == '}') {
        parser->cursor++;
        return object;
    }

    while (parser->cursor < parser->end) {
        size_t key_len = 0;
        char *key = duumbi_json_parse_string_raw(parser, &key_len);
        if (key == NULL) {
            duumbi_json_free(object);
            return NULL;
        }
        duumbi_json_skip_ws(parser);
        if (parser->cursor >= parser->end || *parser->cursor != ':') {
            parser->error = "Expected ':' in JSON object";
            duumbi_dealloc(key);
            duumbi_json_free(object);
            return NULL;
        }
        parser->cursor++;
        DuumbiJson *value = duumbi_json_parse_value(parser, depth + 1);
        if (value == NULL) {
            duumbi_dealloc(key);
            duumbi_json_free(object);
            return NULL;
        }
        if (!duumbi_json_object_add(object, key, key_len, value)) {
            parser->error = "Out of memory while parsing JSON object";
            duumbi_dealloc(key);
            duumbi_json_free(value);
            duumbi_json_free(object);
            return NULL;
        }
        duumbi_json_skip_ws(parser);
        if (parser->cursor < parser->end && *parser->cursor == ',') {
            parser->cursor++;
            duumbi_json_skip_ws(parser);
            continue;
        }
        if (parser->cursor < parser->end && *parser->cursor == '}') {
            parser->cursor++;
            return object;
        }
        parser->error = "Expected ',' or '}' in JSON object";
        duumbi_json_free(object);
        return NULL;
    }

    parser->error = "Unterminated JSON object";
    duumbi_json_free(object);
    return NULL;
}

static DuumbiJson *duumbi_json_parse_number(DuumbiJsonParser *parser) {
    const char *start = parser->cursor;
    if (parser->cursor < parser->end && *parser->cursor == '-') {
        parser->cursor++;
    }
    if (parser->cursor >= parser->end || !isdigit((unsigned char)*parser->cursor)) {
        parser->error = "Invalid JSON number";
        return NULL;
    }
    if (*parser->cursor == '0') {
        parser->cursor++;
    } else {
        while (parser->cursor < parser->end && isdigit((unsigned char)*parser->cursor)) {
            parser->cursor++;
        }
    }
    if (parser->cursor < parser->end && *parser->cursor == '.') {
        parser->cursor++;
        if (parser->cursor >= parser->end || !isdigit((unsigned char)*parser->cursor)) {
            parser->error = "Invalid JSON number";
            return NULL;
        }
        while (parser->cursor < parser->end && isdigit((unsigned char)*parser->cursor)) {
            parser->cursor++;
        }
    }
    if (parser->cursor < parser->end && (*parser->cursor == 'e' || *parser->cursor == 'E')) {
        parser->cursor++;
        if (parser->cursor < parser->end && (*parser->cursor == '+' || *parser->cursor == '-')) {
            parser->cursor++;
        }
        if (parser->cursor >= parser->end || !isdigit((unsigned char)*parser->cursor)) {
            parser->error = "Invalid JSON exponent";
            return NULL;
        }
        while (parser->cursor < parser->end && isdigit((unsigned char)*parser->cursor)) {
            parser->cursor++;
        }
    }

    size_t len = (size_t)(parser->cursor - start);
    char *tmp = duumbi_json_strndup(start, len);
    char *endptr = NULL;
    double number = strtod(tmp, &endptr);
    if (endptr != tmp + len) {
        parser->error = "Invalid JSON number";
        duumbi_dealloc(tmp);
        return NULL;
    }
    duumbi_dealloc(tmp);

    DuumbiJson *json = duumbi_json_new(DUUMBI_JSON_NUMBER);
    json->number_value = number;
    return json;
}

static int duumbi_json_consume_literal(DuumbiJsonParser *parser, const char *literal) {
    size_t len = strlen(literal);
    if ((size_t)(parser->end - parser->cursor) < len ||
        memcmp(parser->cursor, literal, len) != 0) {
        return 0;
    }
    parser->cursor += len;
    return 1;
}

static DuumbiJson *duumbi_json_parse_value(DuumbiJsonParser *parser, uint32_t depth) {
    duumbi_json_skip_ws(parser);
    if (parser->cursor >= parser->end) {
        parser->error = "Unexpected end of JSON input";
        return NULL;
    }
    if (depth > DUUMBI_JSON_MAX_PARSE_DEPTH) {
        parser->error = "JSON parse failed: maximum nesting depth exceeded";
        return NULL;
    }

    char ch = *parser->cursor;
    if (ch == '{') return duumbi_json_parse_object(parser, depth);
    if (ch == '[') return duumbi_json_parse_array(parser, depth);
    if (ch == '"') {
        size_t string_len = 0;
        char *s = duumbi_json_parse_string_raw(parser, &string_len);
        if (s == NULL) return NULL;
        DuumbiJson *json = duumbi_json_new(DUUMBI_JSON_STRING);
        json->string_value = s;
        json->string_len = string_len;
        return json;
    }
    if (ch == '-' || (ch >= '0' && ch <= '9')) {
        return duumbi_json_parse_number(parser);
    }
    if (duumbi_json_consume_literal(parser, "true")) {
        DuumbiJson *json = duumbi_json_new(DUUMBI_JSON_BOOL);
        json->bool_value = 1;
        return json;
    }
    if (duumbi_json_consume_literal(parser, "false")) {
        DuumbiJson *json = duumbi_json_new(DUUMBI_JSON_BOOL);
        json->bool_value = 0;
        return json;
    }
    if (duumbi_json_consume_literal(parser, "null")) {
        return duumbi_json_new(DUUMBI_JSON_NULL);
    }

    parser->error = "Invalid JSON value";
    return NULL;
}

static int duumbi_json_stringify_value(const DuumbiJson *json, DuumbiJsonBuffer *buf);

static int duumbi_json_stringify_string(const char *s, size_t len, DuumbiJsonBuffer *buf) {
    if (!duumbi_json_buffer_char(buf, '"')) return 0;
    for (size_t i = 0; i < len; i++) {
        unsigned char ch = (unsigned char)s[i];
        switch (ch) {
            case '"': if (!duumbi_json_buffer_text(buf, "\\\"")) return 0; break;
            case '\\': if (!duumbi_json_buffer_text(buf, "\\\\")) return 0; break;
            case '\b': if (!duumbi_json_buffer_text(buf, "\\b")) return 0; break;
            case '\f': if (!duumbi_json_buffer_text(buf, "\\f")) return 0; break;
            case '\n': if (!duumbi_json_buffer_text(buf, "\\n")) return 0; break;
            case '\r': if (!duumbi_json_buffer_text(buf, "\\r")) return 0; break;
            case '\t': if (!duumbi_json_buffer_text(buf, "\\t")) return 0; break;
            default:
                if (ch < 0x20) {
                    char esc[7];
                    snprintf(esc, sizeof(esc), "\\u%04x", ch);
                    if (!duumbi_json_buffer_text(buf, esc)) return 0;
                } else if (!duumbi_json_buffer_char(buf, (char)ch)) {
                    return 0;
                }
        }
    }
    return duumbi_json_buffer_char(buf, '"');
}

static int duumbi_json_stringify_value(const DuumbiJson *json, DuumbiJsonBuffer *buf) {
    char number_buf[64];
    switch (json->kind) {
        case DUUMBI_JSON_NULL:
            return duumbi_json_buffer_text(buf, "null");
        case DUUMBI_JSON_BOOL:
            return duumbi_json_buffer_text(buf, json->bool_value ? "true" : "false");
        case DUUMBI_JSON_NUMBER:
            snprintf(number_buf, sizeof(number_buf), "%.15g", json->number_value);
            return duumbi_json_buffer_text(buf, number_buf);
        case DUUMBI_JSON_STRING:
            return duumbi_json_stringify_string(json->string_value, json->string_len, buf);
        case DUUMBI_JSON_ARRAY:
            if (!duumbi_json_buffer_char(buf, '[')) return 0;
            for (uint64_t i = 0; i < json->array_len; i++) {
                if (i > 0 && !duumbi_json_buffer_char(buf, ',')) return 0;
                if (!duumbi_json_stringify_value(json->array_items[i], buf)) return 0;
            }
            return duumbi_json_buffer_char(buf, ']');
        case DUUMBI_JSON_OBJECT:
            if (!duumbi_json_buffer_char(buf, '{')) return 0;
            for (uint64_t i = 0; i < json->object_len; i++) {
                if (i > 0 && !duumbi_json_buffer_char(buf, ',')) return 0;
                if (!duumbi_json_stringify_string(
                    json->object_entries[i].key,
                    json->object_entries[i].key_len,
                    buf
                )) return 0;
                if (!duumbi_json_buffer_char(buf, ':')) return 0;
                if (!duumbi_json_stringify_value(json->object_entries[i].value, buf)) return 0;
            }
            return duumbi_json_buffer_char(buf, '}');
    }
    return 0;
}

void *duumbi_json_parse(void *input) {
    DuumbiString *s = (DuumbiString *)input;
    if (s == NULL) {
        return duumbi_json_err("JSON parse failed: input string is null");
    }

    DuumbiJsonParser parser = {s->data, s->data + s->len, NULL};
    DuumbiJson *json = duumbi_json_parse_value(&parser, 0);
    if (json == NULL) {
        return duumbi_json_err(parser.error != NULL ? parser.error : "JSON parse failed");
    }
    duumbi_json_skip_ws(&parser);
    if (parser.cursor != parser.end) {
        duumbi_json_free(json);
        return duumbi_json_err("JSON parse failed: trailing input");
    }
    return duumbi_json_ok_ptr(json);
}

void *duumbi_json_stringify(void *value) {
    DuumbiJson *json = (DuumbiJson *)value;
    if (json == NULL) {
        return duumbi_json_err("JSON stringify failed: value is null");
    }
    DuumbiJsonBuffer buf;
    duumbi_json_buffer_init(&buf);
    if (!duumbi_json_stringify_value(json, &buf)) {
        duumbi_dealloc(buf.data);
        return duumbi_json_err("JSON stringify failed");
    }
    void *out = duumbi_string_new(buf.data, (uint64_t)buf.len);
    duumbi_dealloc(buf.data);
    return duumbi_json_ok_ptr(out);
}

void *duumbi_json_get_field(void *value, void *key) {
    DuumbiJson *json = (DuumbiJson *)value;
    DuumbiString *field = (DuumbiString *)key;
    if (json == NULL) {
        return duumbi_json_err("JSON get_field failed: value is null");
    }
    if (field == NULL) {
        return duumbi_json_err("JSON get_field failed: key is null");
    }
    if (json->kind != DUUMBI_JSON_OBJECT) {
        return duumbi_json_err("JSON get_field failed: expected object");
    }
    for (uint64_t i = 0; i < json->object_len; i++) {
        const char *entry_key = json->object_entries[i].key;
        size_t entry_len = json->object_entries[i].key_len;
        if (field->len == entry_len && memcmp(field->data, entry_key, entry_len) == 0) {
            return duumbi_json_ok_ptr(duumbi_json_clone(json->object_entries[i].value));
        }
    }
    return duumbi_json_err("JSON get_field failed: missing field");
}

void *duumbi_json_array_len(void *value) {
    DuumbiJson *json = (DuumbiJson *)value;
    if (json == NULL) {
        return duumbi_json_err("JSON array_len failed: value is null");
    }
    if (json->kind != DUUMBI_JSON_ARRAY) {
        return duumbi_json_err("JSON array_len failed: expected array");
    }
    if (json->array_len > (uint64_t)INT64_MAX) {
        return duumbi_json_err("JSON array_len failed: array too large");
    }
    return duumbi_json_ok_i64((int64_t)json->array_len);
}

void *duumbi_json_array_get(void *value, int64_t index) {
    DuumbiJson *json = (DuumbiJson *)value;
    if (json == NULL) {
        return duumbi_json_err("JSON array_get failed: value is null");
    }
    if (json->kind != DUUMBI_JSON_ARRAY) {
        return duumbi_json_err("JSON array_get failed: expected array");
    }
    if (index < 0 || (uint64_t)index >= json->array_len) {
        return duumbi_json_err("JSON array_get failed: index out of bounds");
    }
    return duumbi_json_ok_ptr(duumbi_json_clone(json->array_items[index]));
}

/* ── TCP (opaque socket and listener resources) ───────────────────── */

#if defined(_WIN32)
typedef SOCKET DuumbiSocketHandle;
#define DUUMBI_INVALID_SOCKET INVALID_SOCKET
#define DUUMBI_SOCKET_ERROR SOCKET_ERROR
static int duumbi_socket_close_handle(DuumbiSocketHandle handle) {
    return closesocket(handle);
}
static int duumbi_socket_last_error(void) {
    return WSAGetLastError();
}
static int duumbi_socket_would_block(int err) {
    return err == WSAEWOULDBLOCK || err == WSAEINPROGRESS || err == WSAEALREADY;
}
static BOOL CALLBACK duumbi_socket_init_once(
    PINIT_ONCE init_once,
    PVOID parameter,
    PVOID *context
) {
    (void)init_once;
    (void)parameter;
    (void)context;
    WSADATA data;
    return WSAStartup(MAKEWORD(2, 2), &data) == 0;
}
static int duumbi_socket_init(void) {
    static INIT_ONCE init_once = INIT_ONCE_STATIC_INIT;
    return InitOnceExecuteOnce(&init_once, duumbi_socket_init_once, NULL, NULL) != 0;
}
static int duumbi_socket_set_nonblocking(DuumbiSocketHandle handle) {
    u_long mode = 1;
    return ioctlsocket(handle, FIONBIO, &mode) == 0;
}
#else
typedef int DuumbiSocketHandle;
#define DUUMBI_INVALID_SOCKET (-1)
#define DUUMBI_SOCKET_ERROR (-1)
static int duumbi_socket_close_handle(DuumbiSocketHandle handle) {
    return close(handle);
}
static int duumbi_socket_last_error(void) {
    return errno;
}
static int duumbi_socket_would_block(int err) {
    return err == EINPROGRESS || err == EWOULDBLOCK || err == EAGAIN;
}
static int duumbi_socket_init(void) {
    return 1;
}
static int duumbi_socket_set_nonblocking(DuumbiSocketHandle handle) {
    int flags = fcntl(handle, F_GETFL, 0);
    if (flags < 0) return 0;
    return fcntl(handle, F_SETFL, flags | O_NONBLOCK) == 0;
}
#endif

typedef struct {
    DuumbiSocketHandle handle;
    int closed;
} DuumbiTcpSocket;

typedef struct {
    DuumbiSocketHandle handle;
    int closed;
} DuumbiTcpListener;

static void *duumbi_tcp_err(const char *message) {
    void *err = duumbi_string_new(message, (uint64_t)strlen(message));
    return duumbi_result_new_err((int64_t)(intptr_t)err);
}

static void *duumbi_tcp_ok_ptr(void *ptr) {
    return duumbi_result_new_ok((int64_t)(intptr_t)ptr);
}

static void *duumbi_tcp_ok_i64(int64_t value) {
    return duumbi_result_new_ok(value);
}

static char *duumbi_tcp_string_to_c(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    if (s == NULL) {
        return NULL;
    }
    return duumbi_json_strndup(s->data, (size_t)s->len);
}

static int duumbi_tcp_validate_common(int64_t timeout_ms) {
    return timeout_ms > 0 && timeout_ms <= INT32_MAX;
}

static int duumbi_tcp_validate_port(int64_t port) {
    return port > 0 && port <= 65535;
}

static uint64_t duumbi_tcp_now_ms(void) {
#if defined(_WIN32)
    return (uint64_t)GetTickCount64();
#else
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
        return 0;
    }
    return ((uint64_t)ts.tv_sec * 1000) + ((uint64_t)ts.tv_nsec / 1000000);
#endif
}

static int64_t duumbi_tcp_remaining_timeout(uint64_t start_ms, int64_t timeout_ms) {
    uint64_t now_ms = duumbi_tcp_now_ms();
    uint64_t elapsed_ms = now_ms >= start_ms ? now_ms - start_ms : 0;
    if (elapsed_ms >= (uint64_t)timeout_ms) return 0;
    return timeout_ms - (int64_t)elapsed_ms;
}

static int duumbi_tcp_wait(DuumbiSocketHandle handle, int for_write, int64_t timeout_ms) {
#if defined(_WIN32)
    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(handle, &fds);
    struct timeval tv;
    tv.tv_sec = (long)(timeout_ms / 1000);
    tv.tv_usec = (long)((timeout_ms % 1000) * 1000);
    int ready = select(0, for_write ? NULL : &fds, for_write ? &fds : NULL, NULL, &tv);
    return ready > 0 ? 1 : ready == 0 ? 0 : -1;
#else
    struct pollfd pfd;
    pfd.fd = handle;
    pfd.events = for_write ? POLLOUT : POLLIN;
    pfd.revents = 0;
    int ready = poll(&pfd, 1, (int)timeout_ms);
    if (ready <= 0) return ready;
    if (pfd.revents & (for_write ? POLLOUT : POLLIN)) return 1;
    if (pfd.revents & (POLLERR | POLLHUP | POLLNVAL)) return -1;
    return 0;
#endif
}

static int duumbi_tcp_check_connect(DuumbiSocketHandle handle) {
    int err = 0;
#if defined(_WIN32)
    int len = sizeof(err);
    if (getsockopt(handle, SOL_SOCKET, SO_ERROR, (char *)&err, &len) != 0) return 0;
#else
    socklen_t len = sizeof(err);
    if (getsockopt(handle, SOL_SOCKET, SO_ERROR, &err, &len) != 0) return 0;
#endif
    return err == 0;
}

static int duumbi_tcp_valid_utf8(const unsigned char *data, size_t len) {
    size_t i = 0;
    while (i < len) {
        unsigned char c = data[i];
        if (c <= 0x7f) {
            i++;
        } else if ((c & 0xe0) == 0xc0) {
            if (i + 1 >= len || (data[i + 1] & 0xc0) != 0x80 || c < 0xc2) return 0;
            i += 2;
        } else if ((c & 0xf0) == 0xe0) {
            if (i + 2 >= len || (data[i + 1] & 0xc0) != 0x80 ||
                (data[i + 2] & 0xc0) != 0x80) return 0;
            if (c == 0xe0 && data[i + 1] < 0xa0) return 0;
            if (c == 0xed && data[i + 1] >= 0xa0) return 0;
            i += 3;
        } else if ((c & 0xf8) == 0xf0) {
            if (i + 3 >= len || (data[i + 1] & 0xc0) != 0x80 ||
                (data[i + 2] & 0xc0) != 0x80 || (data[i + 3] & 0xc0) != 0x80) return 0;
            if (c < 0xf0 || c > 0xf4) return 0;
            if (c == 0xf0 && data[i + 1] < 0x90) return 0;
            if (c == 0xf4 && data[i + 1] > 0x8f) return 0;
            i += 4;
        } else {
            return 0;
        }
    }
    return 1;
}

void duumbi_tcp_socket_free(void *socket) {
    DuumbiTcpSocket *s = (DuumbiTcpSocket *)socket;
    if (s == NULL) return;
    if (!s->closed && s->handle != DUUMBI_INVALID_SOCKET) {
        duumbi_socket_close_handle(s->handle);
        s->closed = 1;
    }
    duumbi_dealloc(s);
}

void duumbi_tcp_listener_free(void *listener) {
    DuumbiTcpListener *l = (DuumbiTcpListener *)listener;
    if (l == NULL) return;
    if (!l->closed && l->handle != DUUMBI_INVALID_SOCKET) {
        duumbi_socket_close_handle(l->handle);
        l->closed = 1;
    }
    duumbi_dealloc(l);
}

void *duumbi_tcp_connect(void *host_ptr, int64_t port, int64_t timeout_ms) {
    if (!duumbi_socket_init()) return duumbi_tcp_err("TCP connect failed: socket init failed");
    if (!duumbi_tcp_validate_port(port)) return duumbi_tcp_err("TCP connect failed: invalid port");
    if (!duumbi_tcp_validate_common(timeout_ms)) {
        return duumbi_tcp_err("TCP connect failed: invalid timeout_ms");
    }
    char *host = duumbi_tcp_string_to_c(host_ptr);
    if (host == NULL) return duumbi_tcp_err("TCP connect failed: host is null");

    char port_buf[16];
    snprintf(port_buf, sizeof(port_buf), "%lld", (long long)port);
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_family = AF_UNSPEC;
    hints.ai_flags = AI_NUMERICHOST;
    struct addrinfo *res = NULL;
    if (getaddrinfo(host, port_buf, &hints, &res) != 0) {
        duumbi_dealloc(host);
        return duumbi_tcp_err("TCP connect failed: address resolution failed");
    }

    DuumbiSocketHandle connected = DUUMBI_INVALID_SOCKET;
    for (struct addrinfo *ai = res; ai != NULL; ai = ai->ai_next) {
        DuumbiSocketHandle handle = socket(ai->ai_family, ai->ai_socktype, ai->ai_protocol);
        if (handle == DUUMBI_INVALID_SOCKET) continue;
        if (!duumbi_socket_set_nonblocking(handle)) {
            duumbi_socket_close_handle(handle);
            continue;
        }
        int rc = connect(handle, ai->ai_addr, (int)ai->ai_addrlen);
        if (rc == 0) {
            connected = handle;
            break;
        }
        int err = duumbi_socket_last_error();
        if (duumbi_socket_would_block(err)) {
            int ready = duumbi_tcp_wait(handle, 1, timeout_ms);
            if (ready > 0 && duumbi_tcp_check_connect(handle)) {
                connected = handle;
                break;
            }
        }
        duumbi_socket_close_handle(handle);
    }
    freeaddrinfo(res);
    duumbi_dealloc(host);

    if (connected == DUUMBI_INVALID_SOCKET) {
        return duumbi_tcp_err("TCP connect failed");
    }
    DuumbiTcpSocket *socket_resource = (DuumbiTcpSocket *)duumbi_alloc(sizeof(DuumbiTcpSocket));
    socket_resource->handle = connected;
    socket_resource->closed = 0;
    return duumbi_tcp_ok_ptr(socket_resource);
}

void *duumbi_tcp_listen(void *host_ptr, int64_t port, int64_t timeout_ms) {
    if (!duumbi_socket_init()) return duumbi_tcp_err("TCP listen failed: socket init failed");
    if (!duumbi_tcp_validate_port(port)) return duumbi_tcp_err("TCP listen failed: invalid port");
    if (!duumbi_tcp_validate_common(timeout_ms)) {
        return duumbi_tcp_err("TCP listen failed: invalid timeout_ms");
    }
    char *host = duumbi_tcp_string_to_c(host_ptr);
    if (host == NULL) return duumbi_tcp_err("TCP listen failed: host is null");
    uint64_t start_ms = duumbi_tcp_now_ms();

    char port_buf[16];
    snprintf(port_buf, sizeof(port_buf), "%lld", (long long)port);
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_family = AF_UNSPEC;
    hints.ai_flags = AI_PASSIVE | AI_NUMERICHOST;
    struct addrinfo *res = NULL;
    if (getaddrinfo(host, port_buf, &hints, &res) != 0) {
        duumbi_dealloc(host);
        return duumbi_tcp_err("TCP listen failed: address resolution failed");
    }

    DuumbiSocketHandle bound = DUUMBI_INVALID_SOCKET;
    int timed_out = 0;
    for (struct addrinfo *ai = res; ai != NULL; ai = ai->ai_next) {
        if (duumbi_tcp_remaining_timeout(start_ms, timeout_ms) <= 0) {
            timed_out = 1;
            break;
        }
        DuumbiSocketHandle handle = socket(ai->ai_family, ai->ai_socktype, ai->ai_protocol);
        if (handle == DUUMBI_INVALID_SOCKET) continue;
        int yes = 1;
        setsockopt(handle, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes, sizeof(yes));
        if (bind(handle, ai->ai_addr, (int)ai->ai_addrlen) == 0 &&
            listen(handle, 16) == 0 &&
            duumbi_socket_set_nonblocking(handle)) {
            bound = handle;
            break;
        }
        duumbi_socket_close_handle(handle);
    }
    freeaddrinfo(res);
    duumbi_dealloc(host);

    if (bound == DUUMBI_INVALID_SOCKET) {
        return duumbi_tcp_err(timed_out ? "TCP listen failed: timeout" : "TCP listen failed");
    }
    DuumbiTcpListener *listener = (DuumbiTcpListener *)duumbi_alloc(sizeof(DuumbiTcpListener));
    listener->handle = bound;
    listener->closed = 0;
    return duumbi_tcp_ok_ptr(listener);
}

void *duumbi_tcp_accept(void *listener_ptr, int64_t timeout_ms) {
    DuumbiTcpListener *listener = (DuumbiTcpListener *)listener_ptr;
    if (listener == NULL || listener->closed) {
        return duumbi_tcp_err("TCP accept failed: listener is closed");
    }
    if (!duumbi_tcp_validate_common(timeout_ms)) {
        return duumbi_tcp_err("TCP accept failed: invalid timeout_ms");
    }
    int ready = duumbi_tcp_wait(listener->handle, 0, timeout_ms);
    if (ready == 0) return duumbi_tcp_err("TCP accept failed: timeout");
    if (ready < 0) return duumbi_tcp_err("TCP accept failed");
    DuumbiSocketHandle accepted = accept(listener->handle, NULL, NULL);
    if (accepted == DUUMBI_INVALID_SOCKET) return duumbi_tcp_err("TCP accept failed");
    duumbi_socket_set_nonblocking(accepted);
    DuumbiTcpSocket *socket_resource = (DuumbiTcpSocket *)duumbi_alloc(sizeof(DuumbiTcpSocket));
    socket_resource->handle = accepted;
    socket_resource->closed = 0;
    return duumbi_tcp_ok_ptr(socket_resource);
}

void *duumbi_tcp_read(void *socket_ptr, int64_t max_bytes, int64_t timeout_ms) {
    DuumbiTcpSocket *socket_resource = (DuumbiTcpSocket *)socket_ptr;
    if (socket_resource == NULL || socket_resource->closed) {
        return duumbi_tcp_err("TCP read failed: socket is closed");
    }
    if (max_bytes <= 0 || max_bytes > INT32_MAX) return duumbi_tcp_err("TCP read failed: invalid max_bytes");
    if (!duumbi_tcp_validate_common(timeout_ms)) return duumbi_tcp_err("TCP read failed: invalid timeout_ms");
    int ready = duumbi_tcp_wait(socket_resource->handle, 0, timeout_ms);
    if (ready == 0) return duumbi_tcp_err("TCP read failed: timeout");
    if (ready < 0) return duumbi_tcp_err("TCP read failed");
    char *buf = (char *)duumbi_alloc((uint64_t)max_bytes);
    int n = (int)recv(socket_resource->handle, buf, (int)max_bytes, 0);
    if (n == 0) {
        duumbi_dealloc(buf);
        return duumbi_tcp_err("TCP read failed: peer closed");
    }
    if (n < 0) {
        duumbi_dealloc(buf);
        return duumbi_tcp_err("TCP read failed: socket error");
    }
    if (!duumbi_tcp_valid_utf8((const unsigned char *)buf, (size_t)n)) {
        duumbi_dealloc(buf);
        return duumbi_tcp_err("TCP read failed: bytes are not valid UTF-8");
    }
    void *out = duumbi_string_new(buf, (uint64_t)n);
    duumbi_dealloc(buf);
    return duumbi_tcp_ok_ptr(out);
}

void *duumbi_tcp_write(void *socket_ptr, void *data_ptr, int64_t timeout_ms) {
    DuumbiTcpSocket *socket_resource = (DuumbiTcpSocket *)socket_ptr;
    DuumbiString *data = (DuumbiString *)data_ptr;
    if (socket_resource == NULL || socket_resource->closed) {
        return duumbi_tcp_err("TCP write failed: socket is closed");
    }
    if (data == NULL) return duumbi_tcp_err("TCP write failed: data is null");
    if (!duumbi_tcp_validate_common(timeout_ms)) return duumbi_tcp_err("TCP write failed: invalid timeout_ms");
    if (data->len == 0) return duumbi_tcp_ok_i64(0);
    if (data->len > (uint64_t)INT64_MAX) {
        return duumbi_tcp_err("TCP write failed: data is too large");
    }

    uint64_t start_ms = duumbi_tcp_now_ms();
    uint64_t sent = 0;
    while (sent < data->len) {
        int64_t remaining_ms = duumbi_tcp_remaining_timeout(start_ms, timeout_ms);
        if (remaining_ms <= 0) return duumbi_tcp_err("TCP write failed: timeout");

        int ready = duumbi_tcp_wait(socket_resource->handle, 1, remaining_ms);
        if (ready == 0) return duumbi_tcp_err("TCP write failed: timeout");
        if (ready < 0) return duumbi_tcp_err("TCP write failed");

        uint64_t remaining_bytes = data->len - sent;
        int chunk = remaining_bytes > (uint64_t)INT32_MAX
            ? INT32_MAX
            : (int)remaining_bytes;
#if defined(_WIN32)
        int n = send(socket_resource->handle, data->data + sent, chunk, 0);
#else
        ssize_t n = send(socket_resource->handle, data->data + sent, (size_t)chunk, 0);
#endif
        if (n > 0) {
            sent += (uint64_t)n;
            continue;
        }
        if (n == 0) return duumbi_tcp_err("TCP write failed: socket accepted zero bytes");

        int err = duumbi_socket_last_error();
        if (duumbi_socket_would_block(err)) continue;
        return duumbi_tcp_err("TCP write failed");
    }

    return duumbi_tcp_ok_i64((int64_t)sent);
}

void *duumbi_tcp_close(void *socket_ptr) {
    DuumbiTcpSocket *socket_resource = (DuumbiTcpSocket *)socket_ptr;
    if (socket_resource == NULL || socket_resource->closed) {
        return duumbi_tcp_err("TCP close failed: socket is already closed");
    }
    if (duumbi_socket_close_handle(socket_resource->handle) != 0) {
        return duumbi_tcp_err("TCP close failed");
    }
    socket_resource->closed = 1;
    return duumbi_tcp_ok_i64(0);
}

void *duumbi_tcp_listener_close(void *listener_ptr) {
    DuumbiTcpListener *listener = (DuumbiTcpListener *)listener_ptr;
    if (listener == NULL || listener->closed) {
        return duumbi_tcp_err("TCP listener close failed: listener is already closed");
    }
    if (duumbi_socket_close_handle(listener->handle) != 0) {
        return duumbi_tcp_err("TCP listener close failed");
    }
    listener->closed = 1;
    return duumbi_tcp_ok_i64(0);
}

/* ── HTTP server (DUUMBI-381) ─────────────────────────────────────── */

#define DUUMBI_HTTP_MAX_REQUEST_BYTES 8192

typedef struct {
    char *key;
    char *value;
} DuumbiHttpHeader;

typedef struct {
    char *method;
    char *path;
    int64_t status;
    DuumbiHttpHeader *headers;
    uint64_t header_len;
    char *body;
    uint64_t body_len;
} DuumbiHttpRoute;

typedef struct {
    DuumbiSocketHandle handle;
    int closed;
    int started;
    DuumbiHttpRoute *routes;
    uint64_t route_len;
    uint64_t route_cap;
} DuumbiHttpServer;

static void *duumbi_server_err(const char *message) {
    void *err = duumbi_string_new(message, (uint64_t)strlen(message));
    return duumbi_result_new_err((int64_t)(intptr_t)err);
}

static char *duumbi_server_string_to_c(void *ptr) {
    return duumbi_tcp_string_to_c(ptr);
}

static int duumbi_http_method_valid(const char *method) {
    if (method == NULL || method[0] == '\0') return 0;
    for (const unsigned char *p = (const unsigned char *)method; *p != '\0'; p++) {
        if (!isalnum(*p) && *p != '!' && *p != '#' && *p != '$' && *p != '%' &&
            *p != '&' && *p != '\'' && *p != '*' && *p != '+' && *p != '-' &&
            *p != '.' && *p != '^' && *p != '_' && *p != '`' && *p != '|' &&
            *p != '~') {
            return 0;
        }
    }
    return 1;
}

static int duumbi_http_header_text_valid(const char *text) {
    if (text == NULL || text[0] == '\0') return 0;
    for (const unsigned char *p = (const unsigned char *)text; *p != '\0'; p++) {
        if (*p == '\r' || *p == '\n') return 0;
    }
    return 1;
}

static void duumbi_http_route_free(DuumbiHttpRoute *route) {
    if (route == NULL) return;
    duumbi_dealloc(route->method);
    duumbi_dealloc(route->path);
    duumbi_dealloc(route->body);
    for (uint64_t i = 0; i < route->header_len; i++) {
        duumbi_dealloc(route->headers[i].key);
        duumbi_dealloc(route->headers[i].value);
    }
    duumbi_dealloc(route->headers);
    memset(route, 0, sizeof(*route));
}

static int duumbi_http_copy_headers(void *headers_ptr, DuumbiHttpHeader **out, uint64_t *out_len) {
    DuumbiJson *headers = (DuumbiJson *)headers_ptr;
    if (headers == NULL || headers->kind != DUUMBI_JSON_OBJECT) return 0;
    *out = NULL;
    *out_len = headers->object_len;
    if (headers->object_len == 0) return 1;

    DuumbiHttpHeader *copied =
        (DuumbiHttpHeader *)duumbi_alloc(sizeof(DuumbiHttpHeader) * headers->object_len);
    memset(copied, 0, sizeof(DuumbiHttpHeader) * headers->object_len);
    for (uint64_t i = 0; i < headers->object_len; i++) {
        DuumbiJsonObjectEntry *entry = &headers->object_entries[i];
        if (entry->value == NULL || entry->value->kind != DUUMBI_JSON_STRING) {
            for (uint64_t j = 0; j < i; j++) {
                duumbi_dealloc(copied[j].key);
                duumbi_dealloc(copied[j].value);
            }
            duumbi_dealloc(copied);
            return 0;
        }
        copied[i].key = duumbi_json_strndup(entry->key, entry->key_len);
        copied[i].value = duumbi_json_strndup(entry->value->string_value, entry->value->string_len);
        if (!duumbi_http_header_text_valid(copied[i].key) ||
            !duumbi_http_header_text_valid(copied[i].value)) {
            for (uint64_t j = 0; j <= i; j++) {
                duumbi_dealloc(copied[j].key);
                duumbi_dealloc(copied[j].value);
            }
            duumbi_dealloc(copied);
            return 0;
        }
    }
    *out = copied;
    return 1;
}

void *duumbi_server_new(void *host_ptr, int64_t port, int64_t timeout_ms) {
    if (!duumbi_socket_init()) return duumbi_server_err("server_new failed: socket init failed");
    if (!duumbi_tcp_validate_port(port)) return duumbi_server_err("server_new failed: invalid port");
    if (!duumbi_tcp_validate_common(timeout_ms)) {
        return duumbi_server_err("server_new failed: invalid timeout_ms");
    }
    char *host = duumbi_server_string_to_c(host_ptr);
    if (host == NULL) return duumbi_server_err("server_new failed: host is null");
    uint64_t start_ms = duumbi_tcp_now_ms();

    char port_buf[16];
    snprintf(port_buf, sizeof(port_buf), "%lld", (long long)port);
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_family = AF_UNSPEC;
    hints.ai_flags = AI_PASSIVE | AI_NUMERICHOST;
    struct addrinfo *res = NULL;
    if (getaddrinfo(host, port_buf, &hints, &res) != 0) {
        duumbi_dealloc(host);
        return duumbi_server_err("server_new failed: address resolution failed");
    }

    DuumbiSocketHandle bound = DUUMBI_INVALID_SOCKET;
    int timed_out = 0;
    for (struct addrinfo *ai = res; ai != NULL; ai = ai->ai_next) {
        if (duumbi_tcp_remaining_timeout(start_ms, timeout_ms) <= 0) {
            timed_out = 1;
            break;
        }
        DuumbiSocketHandle handle = socket(ai->ai_family, ai->ai_socktype, ai->ai_protocol);
        if (handle == DUUMBI_INVALID_SOCKET) continue;
        int yes = 1;
        setsockopt(handle, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes, sizeof(yes));
        if (bind(handle, ai->ai_addr, (int)ai->ai_addrlen) == 0 &&
            listen(handle, 16) == 0 &&
            duumbi_socket_set_nonblocking(handle)) {
            bound = handle;
            break;
        }
        duumbi_socket_close_handle(handle);
    }
    freeaddrinfo(res);
    duumbi_dealloc(host);

    if (bound == DUUMBI_INVALID_SOCKET) {
        return duumbi_server_err(timed_out ? "server_new failed: timeout" : "server_new failed");
    }
    DuumbiHttpServer *server = (DuumbiHttpServer *)duumbi_alloc(sizeof(DuumbiHttpServer));
    memset(server, 0, sizeof(*server));
    server->handle = bound;
    return duumbi_tcp_ok_ptr(server);
}

void *duumbi_route_add_static(
    void *server_ptr,
    void *method_ptr,
    void *path_ptr,
    int64_t status,
    void *headers_ptr,
    void *body_ptr
) {
    DuumbiHttpServer *server = (DuumbiHttpServer *)server_ptr;
    if (server == NULL || server->closed) {
        return duumbi_server_err("route_add_static failed: server is closed");
    }
    if (server->started) return duumbi_server_err("route_add_static failed: server already started");
    if (status < 100 || status > 599) return duumbi_server_err("route_add_static failed: invalid status");

    char *method = duumbi_server_string_to_c(method_ptr);
    char *path = duumbi_server_string_to_c(path_ptr);
    char *body = duumbi_server_string_to_c(body_ptr);
    if (method == NULL || path == NULL || body == NULL) {
        duumbi_dealloc(method);
        duumbi_dealloc(path);
        duumbi_dealloc(body);
        return duumbi_server_err("route_add_static failed: null string");
    }
    if (!duumbi_http_method_valid(method) || path[0] != '/') {
        duumbi_dealloc(method);
        duumbi_dealloc(path);
        duumbi_dealloc(body);
        return duumbi_server_err("route_add_static failed: invalid route");
    }
    for (uint64_t i = 0; i < server->route_len; i++) {
        if (strcmp(server->routes[i].method, method) == 0 &&
            strcmp(server->routes[i].path, path) == 0) {
            duumbi_dealloc(method);
            duumbi_dealloc(path);
            duumbi_dealloc(body);
            return duumbi_server_err("route_add_static failed: duplicate route");
        }
    }

    DuumbiHttpHeader *headers = NULL;
    uint64_t header_len = 0;
    if (!duumbi_http_copy_headers(headers_ptr, &headers, &header_len)) {
        duumbi_dealloc(method);
        duumbi_dealloc(path);
        duumbi_dealloc(body);
        return duumbi_server_err("route_add_static failed: invalid headers");
    }

    if (server->route_len == server->route_cap) {
        uint64_t new_cap = server->route_cap == 0 ? 4 : server->route_cap * 2;
        DuumbiHttpRoute *new_routes =
            (DuumbiHttpRoute *)realloc(server->routes, sizeof(DuumbiHttpRoute) * new_cap);
        if (new_routes == NULL) {
            duumbi_dealloc(method);
            duumbi_dealloc(path);
            duumbi_dealloc(body);
            for (uint64_t i = 0; i < header_len; i++) {
                duumbi_dealloc(headers[i].key);
                duumbi_dealloc(headers[i].value);
            }
            duumbi_dealloc(headers);
            return duumbi_server_err("route_add_static failed: out of memory");
        }
        server->routes = new_routes;
        server->route_cap = new_cap;
    }

    DuumbiHttpRoute *route = &server->routes[server->route_len++];
    memset(route, 0, sizeof(*route));
    route->method = method;
    route->path = path;
    route->status = status;
    route->headers = headers;
    route->header_len = header_len;
    route->body = body;
    route->body_len = strlen(body);
    return duumbi_tcp_ok_i64(0);
}

static const char *duumbi_http_status_text(int64_t status) {
    switch (status) {
    case 200: return "OK";
    case 201: return "Created";
    case 204: return "No Content";
    case 400: return "Bad Request";
    case 404: return "Not Found";
    case 500: return "Internal Server Error";
    default: return "Unknown";
    }
}

static int duumbi_http_send_all(DuumbiSocketHandle handle, const char *data, size_t len) {
    size_t sent = 0;
    while (sent < len) {
#if defined(_WIN32)
        int n = send(handle, data + sent, (int)(len - sent), 0);
#else
        ssize_t n = send(handle, data + sent, len - sent, 0);
#endif
#if !defined(_WIN32)
        if (n < 0 && errno == EINTR) continue;
#endif
        if (n <= 0) return 0;
        sent += (size_t)n;
    }
    return 1;
}

static int duumbi_http_write_response(DuumbiSocketHandle client, DuumbiHttpRoute *route) {
    char head[2048];
    size_t len = 0;
    if (route == NULL) {
        len = (size_t)snprintf(
            head,
            sizeof(head),
            "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found"
        );
        return len < sizeof(head) && duumbi_http_send_all(client, head, len);
    }
    len = (size_t)snprintf(
        head,
        sizeof(head),
        "HTTP/1.1 %lld %s\r\nContent-Length: %llu\r\nConnection: close\r\n",
        (long long)route->status,
        duumbi_http_status_text(route->status),
        (unsigned long long)route->body_len
    );
    if (len >= sizeof(head)) return 0;
    for (uint64_t i = 0; i < route->header_len; i++) {
        int written = snprintf(
            head + len,
            sizeof(head) - len,
            "%s: %s\r\n",
            route->headers[i].key,
            route->headers[i].value
        );
        if (written < 0 || (size_t)written >= sizeof(head) - len) return 0;
        len += (size_t)written;
    }
    if (len + 2 >= sizeof(head)) return 0;
    memcpy(head + len, "\r\n", 3);
    len += 2;
    return duumbi_http_send_all(client, head, len) &&
           duumbi_http_send_all(client, route->body, (size_t)route->body_len);
}

static DuumbiHttpRoute *duumbi_http_find_route(DuumbiHttpServer *server, const char *method, const char *path) {
    for (uint64_t i = 0; i < server->route_len; i++) {
        if (strcmp(server->routes[i].method, method) == 0 &&
            strcmp(server->routes[i].path, path) == 0) {
            return &server->routes[i];
        }
    }
    return NULL;
}

static int duumbi_http_handle_client(
    DuumbiHttpServer *server,
    DuumbiSocketHandle client,
    int64_t timeout_ms
) {
    int ready = duumbi_tcp_wait(client, 0, timeout_ms);
    if (ready <= 0) return 0;

    char request[DUUMBI_HTTP_MAX_REQUEST_BYTES + 1];
#if defined(_WIN32)
    int n = recv(client, request, DUUMBI_HTTP_MAX_REQUEST_BYTES, 0);
#else
    ssize_t n = recv(client, request, DUUMBI_HTTP_MAX_REQUEST_BYTES, 0);
#endif
    if (n <= 0) return 0;
    request[n] = '\0';
    char method[32];
    char path[1024];
    if (sscanf(request, "%31s %1023s", method, path) != 2) {
        DuumbiHttpRoute *missing = NULL;
        return duumbi_http_write_response(client, missing);
    }
    return duumbi_http_write_response(client, duumbi_http_find_route(server, method, path));
}

void *duumbi_server_start(void *server_ptr, int64_t max_requests, int64_t timeout_ms) {
    DuumbiHttpServer *server = (DuumbiHttpServer *)server_ptr;
    if (server == NULL || server->closed) {
        return duumbi_server_err("server_start failed: server is closed");
    }
    if (max_requests <= 0) return duumbi_server_err("server_start failed: invalid max_requests");
    if (!duumbi_tcp_validate_common(timeout_ms)) {
        return duumbi_server_err("server_start failed: invalid timeout_ms");
    }
    server->started = 1;
    uint64_t start_ms = duumbi_tcp_now_ms();
    int64_t handled = 0;

    while (handled < max_requests) {
        int64_t remaining = duumbi_tcp_remaining_timeout(start_ms, timeout_ms);
        if (remaining <= 0) {
            return handled > 0
                ? duumbi_tcp_ok_i64(handled)
                : duumbi_server_err("server_start failed: timeout");
        }
        int ready = duumbi_tcp_wait(server->handle, 0, remaining);
        if (ready == 0) {
            return handled > 0
                ? duumbi_tcp_ok_i64(handled)
                : duumbi_server_err("server_start failed: timeout");
        }
        if (ready < 0) return duumbi_server_err("server_start failed");
        DuumbiSocketHandle client = accept(server->handle, NULL, NULL);
        if (client == DUUMBI_INVALID_SOCKET) return duumbi_server_err("server_start failed: accept");
        int64_t read_timeout = duumbi_tcp_remaining_timeout(start_ms, timeout_ms);
        if (read_timeout <= 0) read_timeout = 1;
        (void)duumbi_http_handle_client(server, client, read_timeout);
        duumbi_socket_close_handle(client);
        handled++;
    }
    return duumbi_tcp_ok_i64(handled);
}

void *duumbi_server_close(void *server_ptr) {
    DuumbiHttpServer *server = (DuumbiHttpServer *)server_ptr;
    if (server == NULL) return duumbi_tcp_ok_i64(0);
    if (!server->closed && server->handle != DUUMBI_INVALID_SOCKET) {
        duumbi_socket_close_handle(server->handle);
        server->closed = 1;
    }
    return duumbi_tcp_ok_i64(0);
}

void duumbi_server_free(void *server_ptr) {
    DuumbiHttpServer *server = (DuumbiHttpServer *)server_ptr;
    if (server == NULL) return;
    (void)duumbi_server_close(server);
    for (uint64_t i = 0; i < server->route_len; i++) {
        duumbi_http_route_free(&server->routes[i]);
    }
    duumbi_dealloc(server->routes);
    duumbi_dealloc(server);
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

    char *target_name = strrchr(joined, DUUMBI_PATH_SEP);
    if (target_name == NULL || target_name[1] == '\0') {
        return -1;
    }
    target_name++;

    char parent[DUUMBI_PATH_BUFFER_LEN];
    size_t parent_len = (size_t)((target_name - 1) - joined);
    if (parent_len == 0 || parent_len >= sizeof(parent)) {
        return -1;
    }
    memcpy(parent, joined, parent_len);
    parent[parent_len] = '\0';

    if (DUUMBI_REALPATH(parent, resolved) == NULL || !duumbi_path_inside_root(root, resolved)) {
        return -1;
    }

    int written = snprintf(out, out_len, "%s%c%s", resolved, DUUMBI_PATH_SEP, target_name);
    if (written < 0 || (size_t)written >= out_len) {
        return -1;
    }
    return 0;
}

static int duumbi_temp_write_path(const char *path, char *out, size_t out_len) {
    int written = snprintf(out, out_len, "%s.tmp.%ld", path, (long)DUUMBI_PROCESS_ID());
    if (written < 0 || (size_t)written >= out_len) {
        return -1;
    }
    return 0;
}

static int duumbi_replace_file(const char *tmp_path, const char *target_path) {
#if defined(_WIN32)
    return MoveFileExA(tmp_path, target_path, MOVEFILE_REPLACE_EXISTING) ? 0 : -1;
#else
    return rename(tmp_path, target_path) == 0 ? 0 : -1;
#endif
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
        if (len >= DUUMBI_STDIN_LINE_MAX) {
            free(buffer);
            return duumbi_err_cstr("stdin_line_too_long: stdin line exceeds 65536 bytes");
        }
        if (len + 1 >= capacity) {
            size_t max_capacity = DUUMBI_STDIN_LINE_MAX + 1;
            size_t next_capacity = capacity * 2;
            if (next_capacity < capacity || next_capacity > max_capacity) {
                next_capacity = max_capacity;
            }
            if (next_capacity <= capacity) {
                free(buffer);
                return duumbi_err_cstr("stdin_line_too_long: stdin line exceeds 65536 bytes");
            }
            char *grown = (char *)realloc(buffer, next_capacity);
            if (grown == NULL) {
                free(buffer);
                return duumbi_err_cstr("io_error: out of memory");
            }
            capacity = next_capacity;
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

    char tmp_path[DUUMBI_PATH_BUFFER_LEN];
    if (duumbi_temp_write_path(path, tmp_path, sizeof(tmp_path)) != 0) {
        return duumbi_err_cstr("path_policy: temporary file path is too long");
    }

    remove(tmp_path);
    FILE *file = fopen(tmp_path, DUUMBI_TEMP_WRITE_MODE);
    if (file == NULL) {
        return duumbi_err_cstr("io_error: failed to open file for writing");
    }
    if ((uint64_t)(size_t)contents->len != contents->len) {
        fclose(file);
        remove(tmp_path);
        return duumbi_err_cstr("io_error: file contents are too large to write");
    }
    size_t written = fwrite(contents->data, 1, (size_t)contents->len, file);
    int write_error = ferror(file);
    int close_error = fclose(file);
    if (written != (size_t)contents->len || write_error || close_error != 0) {
        remove(tmp_path);
        return duumbi_err_cstr("io_error: failed to write file");
    }
    if (duumbi_replace_file(tmp_path, path) != 0) {
        remove(tmp_path);
        return duumbi_err_cstr("io_error: failed to replace file");
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
    struct stat st;
    if (stat(resolved, &st) != 0) {
        return duumbi_ok_bool(0);
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

/* ── HTTP (libcurl-backed response resources) ─────────────────────── */

#define DUUMBI_HTTP_MAX_BODY_BYTES (1024 * 1024)
#define DUUMBI_HTTP_MAX_HEADER_VALUE_BYTES 65536

typedef struct {
    int64_t     status;
    char       *body;
    size_t      body_len;
    DuumbiJson *headers;
    int         closed;
} DuumbiHttpResponse;

typedef struct {
    char  *data;
    size_t len;
    size_t cap;
    int    overflow;
} DuumbiHttpBuffer;

typedef struct {
    DuumbiHttpBuffer body;
    DuumbiJson      *headers;
    int              header_error;
} DuumbiHttpCapture;

static void *duumbi_http_err(const char *message) {
    return duumbi_result_new_err((int64_t)(intptr_t)duumbi_string_new(message, strlen(message)));
}

static void *duumbi_http_ok_ptr(void *ptr) {
    return duumbi_result_new_ok((int64_t)(intptr_t)ptr);
}

static void *duumbi_http_ok_i64(int64_t value) {
    return duumbi_result_new_ok(value);
}

static const char *duumbi_http_curl_error_class(CURLcode rc) {
    switch (rc) {
        case CURLE_OPERATION_TIMEDOUT:
            return "http_timeout";
        case CURLE_PEER_FAILED_VERIFICATION:
        case CURLE_SSL_CACERT_BADFILE:
        case CURLE_SSL_CONNECT_ERROR:
            return "http_tls";
        default:
            return "http_transport";
    }
}

static int duumbi_http_validate_timeout(int64_t timeout_ms) {
    return timeout_ms > 0 && timeout_ms <= INT32_MAX;
}

static const char *duumbi_http_ca_bundle_path(void) {
    const char *path = getenv("CURL_CA_BUNDLE");
    if (path != NULL && path[0] != '\0') {
        return path;
    }
    path = getenv("SSL_CERT_FILE");
    if (path != NULL && path[0] != '\0') {
        return path;
    }
    return NULL;
}

static long duumbi_http_connect_timeout_ms(int64_t timeout_ms) {
    int64_t connect_timeout = timeout_ms / 3;
    if (connect_timeout < 1) {
        connect_timeout = 1;
    }
    return (long)connect_timeout;
}

static void duumbi_http_global_cleanup(void) {
    if (duumbi_atomic_load_int(&duumbi_curl_init_state) == DUUMBI_CURL_INIT_READY) {
        curl_global_cleanup();
    }
}

static int duumbi_http_global_init(void) {
    for (;;) {
        int state = duumbi_atomic_load_int(&duumbi_curl_init_state);
        if (state == DUUMBI_CURL_INIT_READY) {
            return 1;
        }
        if (state == DUUMBI_CURL_INIT_FAILED) {
            return 0;
        }
        if (state == DUUMBI_CURL_INIT_UNINITIALIZED &&
            duumbi_atomic_compare_exchange_int(
                &duumbi_curl_init_state,
                DUUMBI_CURL_INIT_UNINITIALIZED,
                DUUMBI_CURL_INIT_INITIALIZING
            )) {
            CURLcode rc = curl_global_init(CURL_GLOBAL_DEFAULT);
            if (rc != CURLE_OK) {
                duumbi_atomic_store_int(&duumbi_curl_init_state, DUUMBI_CURL_INIT_FAILED);
                return 0;
            }
            if (atexit(duumbi_http_global_cleanup) != 0) {
                curl_global_cleanup();
                duumbi_atomic_store_int(&duumbi_curl_init_state, DUUMBI_CURL_INIT_FAILED);
                return 0;
            }
            duumbi_atomic_store_int(&duumbi_curl_init_state, DUUMBI_CURL_INIT_READY);
            return 1;
        }
        duumbi_yield_while_initializing();
    }
}

static int duumbi_http_has_scheme(const char *url) {
    return strncmp(url, "http://", 7) == 0 || strncmp(url, "https://", 8) == 0;
}

static void duumbi_http_buffer_free(DuumbiHttpBuffer *buffer) {
    if (buffer == NULL) return;
    duumbi_dealloc(buffer->data);
    buffer->data = NULL;
    buffer->len = 0;
    buffer->cap = 0;
}

static int duumbi_http_buffer_append(DuumbiHttpBuffer *buffer, const char *data, size_t len) {
    if (len == 0) return 1;
    if (buffer->len > DUUMBI_HTTP_MAX_BODY_BYTES ||
        len > DUUMBI_HTTP_MAX_BODY_BYTES - buffer->len) {
        buffer->overflow = 1;
        return 0;
    }
    size_t needed = buffer->len + len + 1;
    if (needed > buffer->cap) {
        size_t new_cap = buffer->cap == 0 ? 4096 : buffer->cap;
        while (new_cap < needed) {
            if (new_cap > DUUMBI_HTTP_MAX_BODY_BYTES / 2) {
                new_cap = DUUMBI_HTTP_MAX_BODY_BYTES + 1;
                break;
            }
            new_cap *= 2;
        }
        char *new_data = (char *)realloc(buffer->data, new_cap);
        if (new_data == NULL) {
            buffer->overflow = 1;
            return 0;
        }
        buffer->data = new_data;
        buffer->cap = new_cap;
    }
    memcpy(buffer->data + buffer->len, data, len);
    buffer->len += len;
    buffer->data[buffer->len] = '\0';
    return 1;
}

static size_t duumbi_http_write_cb(char *ptr, size_t size, size_t nmemb, void *userdata) {
    size_t len = size * nmemb;
    DuumbiHttpCapture *capture = (DuumbiHttpCapture *)userdata;
    if (!duumbi_http_buffer_append(&capture->body, ptr, len)) {
        return 0;
    }
    return len;
}

static char duumbi_http_lower_ascii(char ch) {
    return (char)tolower((unsigned char)ch);
}

static int duumbi_http_header_name_valid(const char *name, size_t len) {
    if (len == 0) return 0;
    for (size_t i = 0; i < len; i++) {
        unsigned char ch = (unsigned char)name[i];
        if (ch <= 32 || ch >= 127) return 0;
        switch (ch) {
            case '(':
            case ')':
            case '<':
            case '>':
            case '@':
            case ',':
            case ';':
            case ':':
            case '\\':
            case '"':
            case '/':
            case '[':
            case ']':
            case '?':
            case '=':
            case '{':
            case '}':
                return 0;
            default:
                break;
        }
    }
    return 1;
}

static char *duumbi_http_lower_header_name(const char *name, size_t len) {
    char *out = (char *)duumbi_alloc((uint64_t)len + 1);
    for (size_t i = 0; i < len; i++) {
        out[i] = duumbi_http_lower_ascii(name[i]);
    }
    out[len] = '\0';
    return out;
}

static DuumbiJson *duumbi_http_json_string(const char *value, size_t len) {
    DuumbiJson *json = duumbi_json_new(DUUMBI_JSON_STRING);
    json->string_value = duumbi_json_strndup(value, len);
    json->string_len = len;
    return json;
}

static int duumbi_http_object_set_string(
    DuumbiJson *object,
    char *key,
    size_t key_len,
    const char *value,
    size_t value_len
) {
    for (uint64_t i = 0; i < object->object_len; i++) {
        DuumbiJsonObjectEntry *entry = &object->object_entries[i];
        if (entry->key_len == key_len && memcmp(entry->key, key, key_len) == 0) {
            DuumbiJson *existing = entry->value;
            if (existing == NULL || existing->kind != DUUMBI_JSON_STRING) {
                duumbi_dealloc(key);
                return 0;
            }
            if (existing->string_len > DUUMBI_HTTP_MAX_HEADER_VALUE_BYTES ||
                value_len > DUUMBI_HTTP_MAX_HEADER_VALUE_BYTES ||
                existing->string_len + value_len + 2 > DUUMBI_HTTP_MAX_HEADER_VALUE_BYTES) {
                duumbi_dealloc(key);
                return 0;
            }
            size_t joined_len = existing->string_len + 2 + value_len;
            char *joined = (char *)duumbi_alloc((uint64_t)joined_len + 1);
            memcpy(joined, existing->string_value, existing->string_len);
            memcpy(joined + existing->string_len, ", ", 2);
            memcpy(joined + existing->string_len + 2, value, value_len);
            joined[joined_len] = '\0';
            duumbi_dealloc(existing->string_value);
            existing->string_value = joined;
            existing->string_len = joined_len;
            duumbi_dealloc(key);
            return 1;
        }
    }

    DuumbiJson *json_value = duumbi_http_json_string(value, value_len);
    if (!duumbi_json_object_add(object, key, key_len, json_value)) {
        duumbi_json_free(json_value);
        duumbi_dealloc(key);
        return 0;
    }
    return 1;
}

static size_t duumbi_http_header_cb(char *ptr, size_t size, size_t nmemb, void *userdata) {
    size_t len = size * nmemb;
    DuumbiHttpCapture *capture = (DuumbiHttpCapture *)userdata;
    char *colon = memchr(ptr, ':', len);
    if (colon == NULL) {
        return len;
    }

    char *name_start = ptr;
    char *name_end = colon;
    while (name_end > name_start && isspace((unsigned char)name_end[-1])) name_end--;
    char *value_start = colon + 1;
    char *value_end = ptr + len;
    while (value_start < value_end && isspace((unsigned char)*value_start)) value_start++;
    while (value_end > value_start && isspace((unsigned char)value_end[-1])) value_end--;

    size_t name_len = (size_t)(name_end - name_start);
    size_t value_len = (size_t)(value_end - value_start);
    if (!duumbi_http_header_name_valid(name_start, name_len) ||
        value_len > DUUMBI_HTTP_MAX_HEADER_VALUE_BYTES) {
        capture->header_error = 1;
        return 0;
    }

    char *lower_name = duumbi_http_lower_header_name(name_start, name_len);
    if (!duumbi_http_object_set_string(
            capture->headers,
            lower_name,
            name_len,
            value_start,
            value_len
        )) {
        capture->header_error = 1;
        return 0;
    }
    return len;
}

static struct curl_slist *duumbi_http_build_request_headers(void *headers_ptr, char **error) {
    DuumbiJson *headers = (DuumbiJson *)headers_ptr;
    if (headers == NULL || headers->kind != DUUMBI_JSON_OBJECT) {
        *error = "http_headers: request headers must be a json object";
        return NULL;
    }

    struct curl_slist *list = NULL;
    for (uint64_t i = 0; i < headers->object_len; i++) {
        DuumbiJsonObjectEntry *entry = &headers->object_entries[i];
        if (!duumbi_http_header_name_valid(entry->key, entry->key_len)) {
            curl_slist_free_all(list);
            *error = "http_headers: header name is invalid";
            return NULL;
        }
        if (entry->value == NULL || entry->value->kind != DUUMBI_JSON_STRING) {
            curl_slist_free_all(list);
            *error = "http_headers: header values must be strings";
            return NULL;
        }
        size_t value_len = entry->value->string_len;
        if (value_len > DUUMBI_HTTP_MAX_HEADER_VALUE_BYTES) {
            curl_slist_free_all(list);
            *error = "http_headers: header value is too large";
            return NULL;
        }
        size_t line_len = entry->key_len + 2 + value_len;
        char *line = (char *)duumbi_alloc((uint64_t)line_len + 1);
        memcpy(line, entry->key, entry->key_len);
        memcpy(line + entry->key_len, ": ", 2);
        memcpy(line + entry->key_len + 2, entry->value->string_value, value_len);
        line[line_len] = '\0';
        struct curl_slist *updated = curl_slist_append(list, line);
        duumbi_dealloc(line);
        if (updated == NULL) {
            curl_slist_free_all(list);
            *error = "http_headers: failed to build request headers";
            return NULL;
        }
        list = updated;
    }
    return list;
}

static void duumbi_http_response_release_contents(DuumbiHttpResponse *response) {
    if (response == NULL) return;
    duumbi_dealloc(response->body);
    response->body = NULL;
    response->body_len = 0;
    duumbi_json_free(response->headers);
    response->headers = NULL;
}

static void *duumbi_http_request(
    const char *method,
    void *url_ptr,
    void *headers_ptr,
    void *body_ptr,
    int64_t timeout_ms
) {
    DuumbiString *url = (DuumbiString *)url_ptr;
    DuumbiString *body = (DuumbiString *)body_ptr;
    if (url == NULL) return duumbi_http_err("http_url: url is null");
    if (!duumbi_http_validate_timeout(timeout_ms)) {
        return duumbi_http_err("http_timeout: invalid timeout_ms");
    }

    char *url_c = duumbi_json_strndup(url->data, (size_t)url->len);
    if (!duumbi_http_has_scheme(url_c)) {
        duumbi_dealloc(url_c);
        return duumbi_http_err("http_url: unsupported URL scheme");
    }

    char *header_error = NULL;
    struct curl_slist *request_headers =
        duumbi_http_build_request_headers(headers_ptr, &header_error);
    if (header_error != NULL) {
        duumbi_dealloc(url_c);
        return duumbi_http_err(header_error);
    }

    if (!duumbi_http_global_init()) {
        curl_slist_free_all(request_headers);
        duumbi_dealloc(url_c);
        return duumbi_http_err("http_transport: curl global init failed");
    }

    CURL *curl = curl_easy_init();
    if (curl == NULL) {
        curl_slist_free_all(request_headers);
        duumbi_dealloc(url_c);
        return duumbi_http_err("http_transport: curl init failed");
    }

    DuumbiHttpCapture capture;
    memset(&capture, 0, sizeof(capture));
    capture.headers = duumbi_json_new(DUUMBI_JSON_OBJECT);

    char error_buffer[CURL_ERROR_SIZE];
    error_buffer[0] = '\0';
    curl_easy_setopt(curl, CURLOPT_URL, url_c);
    curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, method);
    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, request_headers);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT_MS, (long)timeout_ms);
    curl_easy_setopt(curl, CURLOPT_CONNECTTIMEOUT_MS, duumbi_http_connect_timeout_ms(timeout_ms));
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, duumbi_http_write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &capture);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, duumbi_http_header_cb);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, &capture);
    curl_easy_setopt(curl, CURLOPT_ERRORBUFFER, error_buffer);
    curl_easy_setopt(curl, CURLOPT_NOSIGNAL, 1L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 2L);
    const char *ca_bundle = duumbi_http_ca_bundle_path();
    if (ca_bundle != NULL) {
        curl_easy_setopt(curl, CURLOPT_CAINFO, ca_bundle);
    }
    if (body != NULL) {
        const char *body_data = (const char *)body->data;
        curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body_data);
        curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE_LARGE, (curl_off_t)body->len);
    }

    CURLcode rc = curl_easy_perform(curl);
    long status = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    curl_easy_cleanup(curl);
    curl_slist_free_all(request_headers);
    duumbi_dealloc(url_c);

    if (rc != CURLE_OK) {
        duumbi_json_free(capture.headers);
        duumbi_http_buffer_free(&capture.body);
        if (capture.body.overflow) {
            return duumbi_http_err("http_body_limit: response body exceeds limit");
        }
        if (capture.header_error) {
            return duumbi_http_err("http_header: failed to materialize response headers");
        }
        const char *msg = error_buffer[0] != '\0' ? error_buffer : curl_easy_strerror(rc);
        const char *error_class = duumbi_http_curl_error_class(rc);
        char full[256];
        int written = snprintf(full, sizeof(full), "%s: %s", error_class, msg);
        if (written < 0 || (size_t)written >= sizeof(full)) {
            int fallback_written = snprintf(full, sizeof(full), "%s: request failed", error_class);
            if (fallback_written < 0 || (size_t)fallback_written >= sizeof(full)) {
                return duumbi_http_err("http_transport: request failed");
            }
        }
        return duumbi_http_err(full);
    }

    DuumbiHttpResponse *response =
        (DuumbiHttpResponse *)duumbi_alloc(sizeof(DuumbiHttpResponse));
    response->status = (int64_t)status;
    response->body = capture.body.data;
    response->body_len = capture.body.len;
    response->headers = capture.headers;
    response->closed = 0;
    return duumbi_http_ok_ptr(response);
}

void *duumbi_http_get(void *url, void *headers, int64_t timeout_ms) {
    return duumbi_http_request("GET", url, headers, NULL, timeout_ms);
}

void *duumbi_http_post(void *url, void *headers, void *body, int64_t timeout_ms) {
    if (body == NULL) return duumbi_http_err("http_body: request body is null");
    return duumbi_http_request("POST", url, headers, body, timeout_ms);
}

void *duumbi_http_put(void *url, void *headers, void *body, int64_t timeout_ms) {
    if (body == NULL) return duumbi_http_err("http_body: request body is null");
    return duumbi_http_request("PUT", url, headers, body, timeout_ms);
}

void *duumbi_http_delete(void *url, void *headers, int64_t timeout_ms) {
    return duumbi_http_request("DELETE", url, headers, NULL, timeout_ms);
}

void *duumbi_http_status(void *response) {
    DuumbiHttpResponse *http_response = (DuumbiHttpResponse *)response;
    if (http_response == NULL || http_response->closed) {
        return duumbi_http_err("http_status failed: response is closed");
    }
    return duumbi_http_ok_i64(http_response->status);
}

void *duumbi_http_body(void *response) {
    DuumbiHttpResponse *http_response = (DuumbiHttpResponse *)response;
    if (http_response == NULL || http_response->closed) {
        return duumbi_http_err("http_body failed: response is closed");
    }
    const char *body = http_response->body != NULL ? http_response->body : "";
    return duumbi_http_ok_ptr(duumbi_string_new(body, http_response->body_len));
}

void *duumbi_http_headers(void *response) {
    DuumbiHttpResponse *http_response = (DuumbiHttpResponse *)response;
    if (http_response == NULL || http_response->closed) {
        return duumbi_http_err("http_headers failed: response is closed");
    }
    return duumbi_http_ok_ptr(duumbi_json_clone(http_response->headers));
}

void *duumbi_http_response_close(void *response) {
    DuumbiHttpResponse *http_response = (DuumbiHttpResponse *)response;
    if (http_response == NULL || http_response->closed) {
        return duumbi_http_err("http_response_close failed: response is already closed");
    }
    duumbi_http_response_release_contents(http_response);
    http_response->closed = 1;
    return duumbi_http_ok_i64(0);
}

void duumbi_http_response_free(void *response) {
    DuumbiHttpResponse *http_response = (DuumbiHttpResponse *)response;
    if (http_response == NULL) return;
    duumbi_http_response_release_contents(http_response);
    http_response->closed = 1;
    duumbi_dealloc(http_response);
}

/* ── SQLite runtime (DUUMBI-380) ───────────────────────────────────── */

typedef struct {
    sqlite3 *db;
    int      closed;
} DuumbiDbConnection;

typedef struct {
    uint64_t row_count;
    uint64_t column_count;
    char   **column_names;
    char   **values;
    uint8_t *is_null;
    int      closed;
} DuumbiDbRows;

static void *duumbi_db_err(const char *message) {
    return duumbi_err_cstr(message);
}

static void *duumbi_db_ok_ptr(void *ptr) {
    return duumbi_result_new_ok((int64_t)(intptr_t)ptr);
}

static int duumbi_db_connection_open(DuumbiDbConnection *conn) {
    return conn != NULL && !conn->closed && conn->db != NULL;
}

static char *duumbi_db_string_copy(void *ptr) {
    DuumbiString *s = (DuumbiString *)ptr;
    if (s == NULL || s->len == 0) {
        return NULL;
    }
    for (uint64_t i = 0; i < s->len; i++) {
        if (s->data[i] == '\0') {
            return NULL;
        }
    }
    return duumbi_json_strndup(s->data, (size_t)s->len);
}

static int duumbi_db_path(void *path_ptr, char *out, size_t out_len) {
    DuumbiString *path = (DuumbiString *)path_ptr;
    if (path == NULL || path->len == 0) {
        return -1;
    }
    if (path->len == strlen(":memory:") &&
        memcmp(path->data, ":memory:", strlen(":memory:")) == 0) {
        if (out_len <= strlen(":memory:")) {
            return -1;
        }
        memcpy(out, ":memory:", strlen(":memory:") + 1);
        return 0;
    }
    if (!duumbi_workspace_root_available()) {
        return -1;
    }
    return duumbi_resolve_write_workspace_path(path_ptr, out, out_len);
}

static int duumbi_db_bind_params(sqlite3_stmt *stmt, void *params_ptr) {
    DuumbiArray *params = (DuumbiArray *)params_ptr;
    if (stmt == NULL || params == NULL || params->elem_size != sizeof(int64_t)) {
        return -1;
    }

    int expected = sqlite3_bind_parameter_count(stmt);
    if (expected < 0 || params->len != (uint64_t)expected) {
        return -1;
    }

    for (uint64_t i = 0; i < params->len; i++) {
        int64_t raw = duumbi_array_get(params, i);
        DuumbiString *param = (DuumbiString *)(intptr_t)raw;
        if (param == NULL || param->len > (uint64_t)INT32_MAX) {
            return -1;
        }
        int rc = sqlite3_bind_text(
            stmt,
            (int)i + 1,
            param->data,
            (int)param->len,
            SQLITE_TRANSIENT
        );
        if (rc != SQLITE_OK) {
            return -1;
        }
    }
    return 0;
}

static void duumbi_db_rows_release_contents(DuumbiDbRows *rows) {
    if (rows == NULL) {
        return;
    }
    if (rows->column_names != NULL) {
        for (uint64_t i = 0; i < rows->column_count; i++) {
            duumbi_dealloc(rows->column_names[i]);
        }
        duumbi_dealloc(rows->column_names);
        rows->column_names = NULL;
    }
    if (rows->values != NULL) {
        uint64_t total = rows->row_count * rows->column_count;
        for (uint64_t i = 0; i < total; i++) {
            duumbi_dealloc(rows->values[i]);
        }
        duumbi_dealloc(rows->values);
        rows->values = NULL;
    }
    duumbi_dealloc(rows->is_null);
    rows->is_null = NULL;
    rows->row_count = 0;
    rows->column_count = 0;
}

static void duumbi_db_rows_dispose(DuumbiDbRows *rows) {
    if (rows == NULL || rows->closed) {
        return;
    }
    duumbi_db_rows_release_contents(rows);
    rows->closed = 1;
}

static int duumbi_db_rows_init_columns(DuumbiDbRows *rows, sqlite3_stmt *stmt, int column_count) {
    if (column_count < 0 || column_count > DUUMBI_DB_MAX_COLUMNS) {
        return -1;
    }
    rows->column_count = (uint64_t)column_count;
    if (column_count == 0) {
        return 0;
    }
    rows->column_names = (char **)duumbi_alloc(sizeof(char *) * (uint64_t)column_count);
    memset(rows->column_names, 0, sizeof(char *) * (uint64_t)column_count);
    for (int i = 0; i < column_count; i++) {
        const char *name = sqlite3_column_name(stmt, i);
        if (name == NULL || !duumbi_utf8_valid(name, (uint64_t)strlen(name))) {
            return -1;
        }
        rows->column_names[i] = duumbi_json_strndup(name, strlen(name));
    }
    return 0;
}

static int duumbi_db_rows_append_row(
    DuumbiDbRows *rows,
    sqlite3_stmt *stmt,
    uint64_t *cell_bytes
) {
    if (rows->row_count >= DUUMBI_DB_MAX_ROWS ||
        rows->column_count > 0 &&
            rows->row_count > UINT64_MAX / rows->column_count - 1) {
        return -1;
    }

    uint64_t old_cells = rows->row_count * rows->column_count;
    uint64_t new_row_count = rows->row_count + 1;
    uint64_t new_cells = new_row_count * rows->column_count;

    char **new_values = NULL;
    uint8_t *new_is_null = NULL;
    if (new_cells > 0) {
        new_values = (char **)realloc(rows->values, sizeof(char *) * new_cells);
        if (new_values == NULL) {
            return -1;
        }
        rows->values = new_values;
        new_is_null = (uint8_t *)realloc(rows->is_null, sizeof(uint8_t) * new_cells);
        if (new_is_null == NULL) {
            return -1;
        }
        rows->is_null = new_is_null;
        for (uint64_t i = old_cells; i < new_cells; i++) {
            rows->values[i] = NULL;
            rows->is_null[i] = 0;
        }
    }

    int column_count = (int)rows->column_count;
    for (int col = 0; col < column_count; col++) {
        uint64_t cell = old_cells + (uint64_t)col;
        int kind = sqlite3_column_type(stmt, col);
        if (kind == SQLITE_NULL) {
            rows->is_null[cell] = 1;
            continue;
        }
        if (kind == SQLITE_BLOB) {
            return -1;
        }

        const unsigned char *text = sqlite3_column_text(stmt, col);
        int bytes = sqlite3_column_bytes(stmt, col);
        if (text == NULL || bytes < 0 ||
            !duumbi_utf8_valid((const char *)text, (uint64_t)bytes)) {
            return -1;
        }
        if (*cell_bytes > DUUMBI_DB_MAX_CELL_BYTES ||
            (uint64_t)bytes > DUUMBI_DB_MAX_CELL_BYTES - *cell_bytes) {
            return -1;
        }
        rows->values[cell] = duumbi_json_strndup((const char *)text, (size_t)bytes);
        *cell_bytes += (uint64_t)bytes;
    }

    rows->row_count = new_row_count;
    return 0;
}

void *duumbi_db_open(void *path) {
    char db_path[DUUMBI_PATH_BUFFER_LEN];
    if (duumbi_db_path(path, db_path, sizeof(db_path)) != 0) {
        return duumbi_db_err("db_path: path is outside the workspace or unsupported");
    }

    sqlite3 *db = NULL;
    int rc = sqlite3_open_v2(
        db_path,
        &db,
        SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        NULL
    );
    if (rc != SQLITE_OK) {
        if (db != NULL) {
            sqlite3_close(db);
        }
        return duumbi_db_err("db_open: failed to open SQLite database");
    }
    sqlite3_busy_timeout(db, DUUMBI_DB_BUSY_TIMEOUT_MS);

    DuumbiDbConnection *conn =
        (DuumbiDbConnection *)duumbi_alloc(sizeof(DuumbiDbConnection));
    conn->db = db;
    conn->closed = 0;
    return duumbi_db_ok_ptr(conn);
}

void *duumbi_db_execute(void *conn_ptr, void *sql_ptr, void *params_ptr) {
    DuumbiDbConnection *conn = (DuumbiDbConnection *)conn_ptr;
    if (!duumbi_db_connection_open(conn)) {
        return duumbi_db_err("db_resource: connection is closed");
    }

    DuumbiString *sql = (DuumbiString *)sql_ptr;
    if (sql == NULL || sql->len == 0 || sql->len > (uint64_t)INT32_MAX) {
        return duumbi_db_err("db_sql: SQL must be a non-empty string");
    }

    sqlite3_stmt *stmt = NULL;
    int rc = sqlite3_prepare_v2(conn->db, sql->data, (int)sql->len, &stmt, NULL);
    if (rc != SQLITE_OK || stmt == NULL) {
        return duumbi_db_err("db_sql: failed to prepare statement");
    }
    if (duumbi_db_bind_params(stmt, params_ptr) != 0) {
        sqlite3_finalize(stmt);
        return duumbi_db_err("db_params: parameter count or value is invalid");
    }

    while ((rc = sqlite3_step(stmt)) == SQLITE_ROW) {
        sqlite3_finalize(stmt);
        return duumbi_db_err("db_sql: query produced rows; use db_query");
    }
    if (rc != SQLITE_DONE) {
        sqlite3_finalize(stmt);
        return duumbi_db_err(rc == SQLITE_BUSY ? "db_busy: SQLite statement is busy"
                                               : "db_sql: failed to execute statement");
    }
    sqlite3_finalize(stmt);
    return duumbi_ok_i64((int64_t)sqlite3_changes(conn->db));
}

void *duumbi_db_query(void *conn_ptr, void *sql_ptr, void *params_ptr) {
    DuumbiDbConnection *conn = (DuumbiDbConnection *)conn_ptr;
    if (!duumbi_db_connection_open(conn)) {
        return duumbi_db_err("db_resource: connection is closed");
    }

    DuumbiString *sql = (DuumbiString *)sql_ptr;
    if (sql == NULL || sql->len == 0 || sql->len > (uint64_t)INT32_MAX) {
        return duumbi_db_err("db_sql: SQL must be a non-empty string");
    }

    sqlite3_stmt *stmt = NULL;
    int rc = sqlite3_prepare_v2(conn->db, sql->data, (int)sql->len, &stmt, NULL);
    if (rc != SQLITE_OK || stmt == NULL) {
        return duumbi_db_err("db_sql: failed to prepare query");
    }
    if (duumbi_db_bind_params(stmt, params_ptr) != 0) {
        sqlite3_finalize(stmt);
        return duumbi_db_err("db_params: parameter count or value is invalid");
    }

    DuumbiDbRows *rows = (DuumbiDbRows *)duumbi_alloc(sizeof(DuumbiDbRows));
    memset(rows, 0, sizeof(DuumbiDbRows));
    if (duumbi_db_rows_init_columns(rows, stmt, sqlite3_column_count(stmt)) != 0) {
        sqlite3_finalize(stmt);
        duumbi_db_rows_release_contents(rows);
        duumbi_dealloc(rows);
        return duumbi_db_err("db_rows_limit: query column metadata is unsupported");
    }

    uint64_t cell_bytes = 0;
    while ((rc = sqlite3_step(stmt)) == SQLITE_ROW) {
        if (duumbi_db_rows_append_row(rows, stmt, &cell_bytes) != 0) {
            sqlite3_finalize(stmt);
            duumbi_db_rows_release_contents(rows);
            duumbi_dealloc(rows);
            return duumbi_db_err("db_rows_limit: query result exceeds v1 limits");
        }
    }
    if (rc != SQLITE_DONE) {
        sqlite3_finalize(stmt);
        duumbi_db_rows_release_contents(rows);
        duumbi_dealloc(rows);
        return duumbi_db_err(rc == SQLITE_BUSY ? "db_busy: SQLite query is busy"
                                               : "db_sql: failed to execute query");
    }

    sqlite3_finalize(stmt);
    return duumbi_db_ok_ptr(rows);
}

void *duumbi_db_rows_len(void *rows_ptr) {
    DuumbiDbRows *rows = (DuumbiDbRows *)rows_ptr;
    if (rows == NULL || rows->closed) {
        return duumbi_db_err("db_resource: rows are closed");
    }
    if (rows->row_count > (uint64_t)INT64_MAX) {
        return duumbi_db_err("db_rows_limit: row count exceeds i64");
    }
    return duumbi_ok_i64((int64_t)rows->row_count);
}

void *duumbi_db_row_get(void *rows_ptr, int64_t row_index, void *column_ptr) {
    DuumbiDbRows *rows = (DuumbiDbRows *)rows_ptr;
    if (rows == NULL || rows->closed) {
        return duumbi_db_err("db_resource: rows are closed");
    }
    if (row_index < 0 || (uint64_t)row_index >= rows->row_count) {
        return duumbi_db_err("db_row: row index out of bounds");
    }
    char *column = duumbi_db_string_copy(column_ptr);
    if (column == NULL) {
        return duumbi_db_err("db_row: column name is invalid");
    }

    uint64_t column_index = UINT64_MAX;
    for (uint64_t i = 0; i < rows->column_count; i++) {
        if (strcmp(rows->column_names[i], column) == 0) {
            column_index = i;
            break;
        }
    }
    duumbi_dealloc(column);
    if (column_index == UINT64_MAX) {
        return duumbi_db_err("db_row: column is missing");
    }

    uint64_t cell = (uint64_t)row_index * rows->column_count + column_index;
    if (rows->is_null[cell]) {
        return duumbi_db_err("db_null: SQLite NULL is not representable as v1 string");
    }
    char *value = rows->values[cell];
    if (value == NULL) {
        return duumbi_db_err("db_row: value is unavailable");
    }
    return duumbi_result_new_ok(
        (int64_t)(intptr_t)duumbi_string_new(value, (uint64_t)strlen(value))
    );
}

void *duumbi_db_close(void *conn_ptr) {
    DuumbiDbConnection *conn = (DuumbiDbConnection *)conn_ptr;
    if (conn == NULL || conn->closed) {
        return duumbi_db_err("db_resource: connection is already closed");
    }
    if (conn->db != NULL) {
        int rc = sqlite3_close(conn->db);
        if (rc != SQLITE_OK) {
            return duumbi_db_err("db_busy: connection still has active statements");
        }
    }
    conn->db = NULL;
    conn->closed = 1;
    return duumbi_ok_i64(0);
}

void *duumbi_db_rows_close(void *rows_ptr) {
    DuumbiDbRows *rows = (DuumbiDbRows *)rows_ptr;
    if (rows == NULL || rows->closed) {
        return duumbi_db_err("db_resource: rows are already closed");
    }
    duumbi_db_rows_dispose(rows);
    return duumbi_ok_i64(0);
}

void duumbi_db_connection_free(void *conn_ptr) {
    DuumbiDbConnection *conn = (DuumbiDbConnection *)conn_ptr;
    if (conn == NULL) {
        return;
    }
    if (!conn->closed && conn->db != NULL) {
        sqlite3_close(conn->db);
        conn->db = NULL;
        conn->closed = 1;
    }
    duumbi_dealloc(conn);
}

void duumbi_db_rows_free(void *rows_ptr) {
    DuumbiDbRows *rows = (DuumbiDbRows *)rows_ptr;
    if (rows == NULL) {
        return;
    }
    duumbi_db_rows_dispose(rows);
    duumbi_dealloc(rows);
}
