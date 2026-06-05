#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <errno.h>
#include <ctype.h>
#include <sys/stat.h>
#include <sys/types.h>

#if defined(_WIN32)
#include <winsock2.h>
#include <ws2tcpip.h>
#include <direct.h>
#define DUUMBI_MKDIR(path) _mkdir(path)
#define DUUMBI_PATH_SEP '\\'
#else
#include <unistd.h>
#include <fcntl.h>
#include <poll.h>
#include <netdb.h>
#include <sys/socket.h>
#define DUUMBI_MKDIR(path) mkdir(path, 0777)
#define DUUMBI_PATH_SEP '/'
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
    DuumbiJson *value;
} DuumbiJsonObjectEntry;

struct DuumbiJson {
    DuumbiJsonKind kind;
    int            bool_value;
    double         number_value;
    char          *string_value;
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
        copy->string_value = duumbi_json_strndup(json->string_value, strlen(json->string_value));
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
                copy->object_entries[i].key =
                    duumbi_json_strndup(json->object_entries[i].key,
                                        strlen(json->object_entries[i].key));
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
    if (new_data == NULL) {
        return 0;
    }
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

static char *duumbi_json_parse_string_raw(DuumbiJsonParser *parser) {
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

static DuumbiJson *duumbi_json_parse_value(DuumbiJsonParser *parser);

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

static int duumbi_json_object_add(DuumbiJson *object, char *key, DuumbiJson *value) {
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
    object->object_entries[object->object_len].value = value;
    object->object_len++;
    return 1;
}

static DuumbiJson *duumbi_json_parse_array(DuumbiJsonParser *parser) {
    parser->cursor++;
    DuumbiJson *array = duumbi_json_new(DUUMBI_JSON_ARRAY);
    duumbi_json_skip_ws(parser);
    if (parser->cursor < parser->end && *parser->cursor == ']') {
        parser->cursor++;
        return array;
    }

    while (parser->cursor < parser->end) {
        DuumbiJson *item = duumbi_json_parse_value(parser);
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

static DuumbiJson *duumbi_json_parse_object(DuumbiJsonParser *parser) {
    parser->cursor++;
    DuumbiJson *object = duumbi_json_new(DUUMBI_JSON_OBJECT);
    duumbi_json_skip_ws(parser);
    if (parser->cursor < parser->end && *parser->cursor == '}') {
        parser->cursor++;
        return object;
    }

    while (parser->cursor < parser->end) {
        char *key = duumbi_json_parse_string_raw(parser);
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
        DuumbiJson *value = duumbi_json_parse_value(parser);
        if (value == NULL) {
            duumbi_dealloc(key);
            duumbi_json_free(object);
            return NULL;
        }
        if (!duumbi_json_object_add(object, key, value)) {
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

static DuumbiJson *duumbi_json_parse_value(DuumbiJsonParser *parser) {
    duumbi_json_skip_ws(parser);
    if (parser->cursor >= parser->end) {
        parser->error = "Unexpected end of JSON input";
        return NULL;
    }

    char ch = *parser->cursor;
    if (ch == '{') return duumbi_json_parse_object(parser);
    if (ch == '[') return duumbi_json_parse_array(parser);
    if (ch == '"') {
        char *s = duumbi_json_parse_string_raw(parser);
        if (s == NULL) return NULL;
        DuumbiJson *json = duumbi_json_new(DUUMBI_JSON_STRING);
        json->string_value = s;
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

static int duumbi_json_stringify_string(const char *s, DuumbiJsonBuffer *buf) {
    if (!duumbi_json_buffer_char(buf, '"')) return 0;
    for (const unsigned char *p = (const unsigned char *)s; *p != '\0'; p++) {
        switch (*p) {
            case '"': if (!duumbi_json_buffer_text(buf, "\\\"")) return 0; break;
            case '\\': if (!duumbi_json_buffer_text(buf, "\\\\")) return 0; break;
            case '\b': if (!duumbi_json_buffer_text(buf, "\\b")) return 0; break;
            case '\f': if (!duumbi_json_buffer_text(buf, "\\f")) return 0; break;
            case '\n': if (!duumbi_json_buffer_text(buf, "\\n")) return 0; break;
            case '\r': if (!duumbi_json_buffer_text(buf, "\\r")) return 0; break;
            case '\t': if (!duumbi_json_buffer_text(buf, "\\t")) return 0; break;
            default:
                if (*p < 0x20) {
                    char esc[7];
                    snprintf(esc, sizeof(esc), "\\u%04x", *p);
                    if (!duumbi_json_buffer_text(buf, esc)) return 0;
                } else if (!duumbi_json_buffer_char(buf, (char)*p)) {
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
            return duumbi_json_stringify_string(json->string_value, buf);
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
                if (!duumbi_json_stringify_string(json->object_entries[i].key, buf)) return 0;
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
    DuumbiJson *json = duumbi_json_parse_value(&parser);
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
        size_t entry_len = strlen(entry_key);
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
static int duumbi_socket_init(void) {
    static int initialized = 0;
    if (initialized) return 1;
    WSADATA data;
    if (WSAStartup(MAKEWORD(2, 2), &data) != 0) return 0;
    initialized = 1;
    return 1;
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
            i += 3;
        } else if ((c & 0xf8) == 0xf0) {
            if (i + 3 >= len || (data[i + 1] & 0xc0) != 0x80 ||
                (data[i + 2] & 0xc0) != 0x80 || (data[i + 3] & 0xc0) != 0x80) return 0;
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

    char port_buf[16];
    snprintf(port_buf, sizeof(port_buf), "%lld", (long long)port);
    struct addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_family = AF_UNSPEC;
    hints.ai_flags = AI_PASSIVE;
    struct addrinfo *res = NULL;
    if (getaddrinfo(host, port_buf, &hints, &res) != 0) {
        duumbi_dealloc(host);
        return duumbi_tcp_err("TCP listen failed: address resolution failed");
    }

    DuumbiSocketHandle bound = DUUMBI_INVALID_SOCKET;
    for (struct addrinfo *ai = res; ai != NULL; ai = ai->ai_next) {
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

    if (bound == DUUMBI_INVALID_SOCKET) return duumbi_tcp_err("TCP listen failed");
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
    if (n <= 0) {
        duumbi_dealloc(buf);
        return duumbi_tcp_err("TCP read failed: peer closed");
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
    int ready = duumbi_tcp_wait(socket_resource->handle, 1, timeout_ms);
    if (ready == 0) return duumbi_tcp_err("TCP write failed: timeout");
    if (ready < 0) return duumbi_tcp_err("TCP write failed");
    int n = (int)send(socket_resource->handle, data->data, (int)data->len, 0);
    if (n < 0) return duumbi_tcp_err("TCP write failed");
    return duumbi_tcp_ok_i64((int64_t)n);
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
