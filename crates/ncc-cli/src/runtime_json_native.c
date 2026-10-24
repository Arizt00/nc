/* nC runtime JSON v1 -- freestanding PE/ELF/Mach-O bridge.
 *
 * This source is compiled only when emitted LLVM references a json* symbol.
 * It deliberately avoids a platform C header dependency: Phase12 resolves the
 * small CRT/OS surface below and keeps final loader imports explicit.
 */

typedef unsigned char u8;
typedef unsigned int u32;
typedef unsigned long long u64;
typedef long long i64;
typedef __SIZE_TYPE__ usize;

extern void *malloc(usize);
extern void *realloc(void *, usize);
extern void free(void *);
extern usize strlen(const char *);
#if defined(_WIN32)
extern int _snprintf(char *, usize, const char *, ...);

extern void *CreateFileA(const char *, u32, u32, void *, u32, u32, void *);
extern int ReadFile(void *, void *, u32, u32 *, void *);
extern int WriteFile(void *, const void *, u32, u32 *, void *);
extern int FlushFileBuffers(void *);
extern int CloseHandle(void *);
extern int GetFileSizeEx(void *, i64 *);
extern int MoveFileExA(const char *, const char *, u32);
extern int DeleteFileA(const char *);
extern u32 GetModuleFileNameA(void *, char *, u32);
#else
extern int snprintf(char *, usize, const char *, ...);
extern void *fopen(const char *, const char *);
extern int fseek(void *, long, int);
extern long ftell(void *);
extern usize fread(void *, usize, usize, void *);
extern usize fwrite(const void *, usize, usize, void *);
extern int fflush(void *);
extern int fclose(void *);
extern int fileno(void *);
extern int fsync(int);
extern int rename(const char *, const char *);
extern int remove(const char *);
#if defined(__APPLE__)
extern int _NSGetExecutablePath(char *, u32 *);
#else
extern long readlink(const char *, char *, usize);
#endif
#define _snprintf snprintf
#endif

#define NC_JSON_INVALID_HANDLE ((i64)0)
#define NC_JSON_NULL 1
#define NC_JSON_BOOL 2
#define NC_JSON_NUMBER 3
#define NC_JSON_STRING 4
#define NC_JSON_ARRAY 5
#define NC_JSON_OBJECT 6

#define NC_JSON_E_NONE 0
#define NC_JSON_E_PARSE 1
#define NC_JSON_E_IO 2
#define NC_JSON_E_HANDLE 3
#define NC_JSON_E_TYPE 4
#define NC_JSON_E_KEY 5
#define NC_JSON_E_INDEX 6
#define NC_JSON_E_POINTER 7
#define NC_JSON_E_NUMBER 8
#define NC_JSON_E_LIMIT 9
#define NC_JSON_E_SERIALIZE 10

#define NC_JSON_MAX_INPUT (16ULL * 1024ULL * 1024ULL)
#define NC_JSON_MAX_DEPTH 128
#define NC_JSON_MAX_NODES 1000000ULL
#define NC_JSON_MAX_HANDLES 65536ULL

#if defined(_WIN32)
#define GENERIC_READ 0x80000000U
#define GENERIC_WRITE 0x40000000U
#define FILE_SHARE_READ 0x00000001U
#define OPEN_EXISTING 3U
#define CREATE_ALWAYS 2U
#define FILE_ATTRIBUTE_NORMAL 0x00000080U
#define MOVEFILE_REPLACE_EXISTING 0x00000001U
#define MOVEFILE_WRITE_THROUGH 0x00000008U
#define INVALID_HANDLE_VALUE ((void *)(i64)-1)
#endif

typedef struct NcJsonNode NcJsonNode;
struct NcJsonNode {
    int kind;
    int boolean;
    char *text;
    NcJsonNode **items;
    char **keys;
    u64 len;
    u64 cap;
};

typedef struct NcJsonHandleEntry NcJsonHandleEntry;
struct NcJsonHandleEntry {
    i64 id;
    NcJsonNode *node;
    NcJsonHandleEntry *next;
};

typedef struct {
    int code;
    i64 line;
    i64 column;
    char message[512];
    char path[512];
} NcJsonError;

typedef struct {
    const char *text;
    u64 len;
    u64 pos;
    u64 nodes;
    i64 line;
    i64 column;
} NcJsonParser;

typedef struct {
    char *data;
    u64 len;
    u64 cap;
    int failed;
} NcJsonBuffer;

static NcJsonHandleEntry *g_json_handles = (NcJsonHandleEntry *)0;
static u64 g_json_handle_count = 0;
static i64 g_json_next_handle = 1;
static NcJsonError g_json_error = {0, 0, 0, {0}, {0}};
static const char g_json_empty[] = "";
#if defined(_WIN32)
int _fltused = 0;
#endif

static void nc_memcpy(void *destination, const void *source, u64 len) {
    u8 *dst = (u8 *)destination;
    const u8 *src = (const u8 *)source;
    u64 i;
    for (i = 0; i < len; ++i) dst[i] = src[i];
}

static int nc_streq(const char *left, const char *right) {
    if (!left || !right) return left == right;
    while (*left && *right && *left == *right) {
        ++left;
        ++right;
    }
    return *left == *right;
}

static int nc_text_order(const char *left, const char *right) {
    const u8 *a = (const u8 *)(left ? left : "");
    const u8 *b = (const u8 *)(right ? right : "");
    while (*a && *b && *a == *b) {
        ++a;
        ++b;
    }
    return *a < *b ? -1 : (*a > *b ? 1 : 0);
}

static int nc_utf8_valid(const char *text) {
    const u8 *cursor = (const u8 *)text;
    if (!cursor) return 0;
    while (*cursor) {
        u8 first = *cursor++;
        if (first <= 0x7f) continue;
        if (first >= 0xc2 && first <= 0xdf) {
            if (*cursor < 0x80 || *cursor > 0xbf) return 0;
            ++cursor;
            continue;
        }
        if (first >= 0xe0 && first <= 0xef) {
            u8 second;
            u8 third;
            if (!cursor[0] || !cursor[1]) return 0;
            second = cursor[0];
            third = cursor[1];
            cursor += 2;
            if (second < 0x80 || second > 0xbf || third < 0x80 || third > 0xbf) return 0;
            if (first == 0xe0 && second < 0xa0) return 0;
            if (first == 0xed && second > 0x9f) return 0;
            continue;
        }
        if (first >= 0xf0 && first <= 0xf4) {
            u8 second;
            u8 third;
            u8 fourth;
            if (!cursor[0] || !cursor[1] || !cursor[2]) return 0;
            second = cursor[0];
            third = cursor[1];
            fourth = cursor[2];
            cursor += 3;
            if (second < 0x80 || second > 0xbf || third < 0x80 || third > 0xbf ||
                fourth < 0x80 || fourth > 0xbf) return 0;
            if (first == 0xf0 && second < 0x90) return 0;
            if (first == 0xf4 && second > 0x8f) return 0;
            continue;
        }
        return 0;
    }
    return 1;
}

static char *nc_strdup_len(const char *source, u64 len) {
    char *copy = (char *)malloc(len + 1);
    if (!copy) return (char *)0;
    if (len) nc_memcpy(copy, source, len);
    copy[len] = 0;
    return copy;
}

static char *nc_strdup(const char *source) {
    if (!source) return (char *)0;
    return nc_strdup_len(source, strlen(source));
}

static void nc_copy_fixed(char *target, u64 capacity, const char *source) {
    u64 index = 0;
    if (!capacity) return;
    if (source) {
        while (source[index] && index + 1 < capacity) {
            target[index] = source[index];
            ++index;
        }
    }
    target[index] = 0;
}

/* Keep every diagnostic string as a direct image-relative reference.  Clang's
 * optimized switch otherwise materializes an array of absolute pointers in
 * .rdata; the internal PE lane deliberately avoids that extra writable-style
 * pointer table for this freestanding runtime object. */
__attribute__((noinline, optnone)) static const char *nc_error_name(int code) {
    if (code == NC_JSON_E_PARSE) return "E_RUNTIME_JSON_PARSE_001";
    if (code == NC_JSON_E_IO) return "E_RUNTIME_JSON_IO_001";
    if (code == NC_JSON_E_HANDLE) return "E_RUNTIME_JSON_HANDLE_001";
    if (code == NC_JSON_E_TYPE) return "E_RUNTIME_JSON_TYPE_001";
    if (code == NC_JSON_E_KEY) return "E_RUNTIME_JSON_KEY_001";
    if (code == NC_JSON_E_INDEX) return "E_RUNTIME_JSON_INDEX_001";
    if (code == NC_JSON_E_POINTER) return "E_RUNTIME_JSON_POINTER_001";
    if (code == NC_JSON_E_NUMBER) return "E_RUNTIME_JSON_NUMBER_001";
    if (code == NC_JSON_E_LIMIT) return "E_RUNTIME_JSON_LIMIT_001";
    if (code == NC_JSON_E_SERIALIZE) return "E_RUNTIME_JSON_SERIALIZE_001";
    return "OK";
}

static void nc_clear_error(void) {
    g_json_error.code = NC_JSON_E_NONE;
    g_json_error.line = 0;
    g_json_error.column = 0;
    g_json_error.message[0] = 0;
    g_json_error.path[0] = 0;
}

/* The internal PE linker preserves the C object's scalar relocations, while
 * this retained bridge intentionally does not depend on compiler-generated
 * aligned SIMD literal loads.  Keeping the cold diagnostic copier out of line
 * also makes error construction small and alignment-independent. */
__attribute__((noinline, optnone)) static void nc_set_error(
    int code,
    const char *message,
    i64 line,
    i64 column,
    const char *path
) {
    g_json_error.code = code;
    g_json_error.line = line;
    g_json_error.column = column;
    nc_copy_fixed(g_json_error.message, 512, message);
    nc_copy_fixed(g_json_error.path, 512, path);
}

static NcJsonNode *nc_node_new(int kind) {
    NcJsonNode *node = (NcJsonNode *)malloc((u64)sizeof(NcJsonNode));
    if (!node) {
        nc_set_error(NC_JSON_E_LIMIT, "out of memory creating JSON node", 0, 0, "");
        return (NcJsonNode *)0;
    }
    node->kind = kind;
    node->boolean = 0;
    node->text = (char *)0;
    node->items = (NcJsonNode **)0;
    node->keys = (char **)0;
    node->len = 0;
    node->cap = 0;
    return node;
}

static void nc_node_free(NcJsonNode *node) {
    u64 i;
    if (!node) return;
    if (node->text) free(node->text);
    for (i = 0; i < node->len; ++i) {
        nc_node_free(node->items[i]);
        if (node->keys && node->keys[i]) free(node->keys[i]);
    }
    if (node->items) free(node->items);
    if (node->keys) free(node->keys);
    free(node);
}

static int nc_node_reserve(NcJsonNode *node, u64 wanted) {
    u64 cap;
    NcJsonNode **items;
    char **keys;
    if (wanted <= node->cap) return 1;
    cap = node->cap ? node->cap * 2 : 4;
    while (cap < wanted) cap *= 2;
    if (cap > NC_JSON_MAX_NODES) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON container capacity limit exceeded", 0, 0, "");
        return 0;
    }
    items = (NcJsonNode **)realloc(node->items, cap * (u64)sizeof(NcJsonNode *));
    if (!items) {
        nc_set_error(NC_JSON_E_LIMIT, "out of memory growing JSON container", 0, 0, "");
        return 0;
    }
    node->items = items;
    if (node->kind == NC_JSON_OBJECT) {
        keys = (char **)realloc(node->keys, cap * (u64)sizeof(char *));
        if (!keys) {
            nc_set_error(NC_JSON_E_LIMIT, "out of memory growing JSON object", 0, 0, "");
            return 0;
        }
        node->keys = keys;
    }
    node->cap = cap;
    return 1;
}

static int nc_array_append_owned(NcJsonNode *array, NcJsonNode *value) {
    if (!nc_node_reserve(array, array->len + 1)) return 0;
    array->items[array->len++] = value;
    return 1;
}

static int nc_object_set_owned(NcJsonNode *object, char *key, NcJsonNode *value) {
    u64 i;
    u64 insertion = object->len;
    for (i = 0; i < object->len; ++i) {
        if (nc_streq(object->keys[i], key)) {
            free(key);
            nc_node_free(object->items[i]);
            object->items[i] = value;
            return 1;
        }
        if (insertion == object->len && nc_text_order(key, object->keys[i]) < 0) insertion = i;
    }
    if (!nc_node_reserve(object, object->len + 1)) return 0;
    for (i = object->len; i > insertion; --i) {
        object->keys[i] = object->keys[i - 1];
        object->items[i] = object->items[i - 1];
    }
    object->keys[insertion] = key;
    object->items[insertion] = value;
    ++object->len;
    return 1;
}

static NcJsonNode *nc_node_clone_depth(const NcJsonNode *source, int depth) {
    NcJsonNode *copy;
    u64 i;
    if (!source || depth > NC_JSON_MAX_DEPTH) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON clone depth limit exceeded", 0, 0, "");
        return (NcJsonNode *)0;
    }
    copy = nc_node_new(source->kind);
    if (!copy) return (NcJsonNode *)0;
    copy->boolean = source->boolean;
    if (source->text) {
        copy->text = nc_strdup(source->text);
        if (!copy->text) {
            nc_node_free(copy);
            nc_set_error(NC_JSON_E_LIMIT, "out of memory cloning JSON scalar", 0, 0, "");
            return (NcJsonNode *)0;
        }
    }
    for (i = 0; i < source->len; ++i) {
        NcJsonNode *child = nc_node_clone_depth(source->items[i], depth + 1);
        if (!child) {
            nc_node_free(copy);
            return (NcJsonNode *)0;
        }
        if (source->kind == NC_JSON_OBJECT) {
            char *key = nc_strdup(source->keys[i]);
            if (!key || !nc_object_set_owned(copy, key, child)) {
                if (key) free(key);
                nc_node_free(child);
                nc_node_free(copy);
                return (NcJsonNode *)0;
            }
        } else if (!nc_array_append_owned(copy, child)) {
            nc_node_free(child);
            nc_node_free(copy);
            return (NcJsonNode *)0;
        }
    }
    return copy;
}

static int nc_tree_limits(const NcJsonNode *node, int depth, u64 *count) {
    u64 i;
    if (!node || depth > NC_JSON_MAX_DEPTH) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON nesting depth limit exceeded", 0, 0, "");
        return 0;
    }
    ++*count;
    if (*count > NC_JSON_MAX_NODES) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON node limit exceeded", 0, 0, "");
        return 0;
    }
    for (i = 0; i < node->len; ++i) {
        if (!nc_tree_limits(node->items[i], depth + 1, count)) return 0;
    }
    return 1;
}

static NcJsonNode *nc_handle_lookup(i64 handle) {
    NcJsonHandleEntry *entry = g_json_handles;
    if (handle <= 0) {
        nc_set_error(NC_JSON_E_HANDLE, "invalid JSON handle", 0, 0, "");
        return (NcJsonNode *)0;
    }
    while (entry) {
        if (entry->id == handle) return entry->node;
        entry = entry->next;
    }
    nc_set_error(NC_JSON_E_HANDLE, "unknown or released JSON handle", 0, 0, "");
    return (NcJsonNode *)0;
}

static i64 nc_handle_expose(NcJsonNode *node) {
    NcJsonHandleEntry *entry;
    i64 candidate;
    if (!node) return NC_JSON_INVALID_HANDLE;
    if (g_json_handle_count >= NC_JSON_MAX_HANDLES) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON handle limit reached", 0, 0, "");
        nc_node_free(node);
        return NC_JSON_INVALID_HANDLE;
    }
    entry = (NcJsonHandleEntry *)malloc((u64)sizeof(NcJsonHandleEntry));
    if (!entry) {
        nc_set_error(NC_JSON_E_LIMIT, "out of memory registering JSON handle", 0, 0, "");
        nc_node_free(node);
        return NC_JSON_INVALID_HANDLE;
    }
    entry->node = node;
    candidate = g_json_next_handle;
    for (;;) {
        NcJsonHandleEntry *cursor = g_json_handles;
        int used = 0;
        while (cursor) {
            if (cursor->id == candidate) { used = 1; break; }
            cursor = cursor->next;
        }
        if (!used) break;
        ++candidate;
        if (candidate <= 0 || candidate > 0x7fffffffLL) candidate = 1;
    }
    entry->id = candidate;
    g_json_next_handle = candidate + 1;
    if (g_json_next_handle > 0x7fffffffLL) g_json_next_handle = 1;
    entry->next = g_json_handles;
    g_json_handles = entry;
    ++g_json_handle_count;
    nc_clear_error();
    return candidate;
}

static int nc_handle_release(i64 handle) {
    NcJsonHandleEntry **link = &g_json_handles;
    while (*link) {
        if ((*link)->id == handle) {
            NcJsonHandleEntry *entry = *link;
            *link = entry->next;
            nc_node_free(entry->node);
            free(entry);
            --g_json_handle_count;
            nc_clear_error();
            return 1;
        }
        link = &(*link)->next;
    }
    nc_set_error(NC_JSON_E_HANDLE, "unknown or released JSON handle", 0, 0, "");
    return 0;
}

static int nc_buffer_reserve(NcJsonBuffer *buffer, u64 extra) {
    u64 wanted;
    u64 cap;
    char *next;
    if (buffer->failed) return 0;
    wanted = buffer->len + extra + 1;
    if (wanted <= buffer->cap) return 1;
    cap = buffer->cap ? buffer->cap * 2 : 64;
    while (cap < wanted) cap *= 2;
    if (cap > NC_JSON_MAX_INPUT * 4ULL) {
        buffer->failed = 1;
        nc_set_error(NC_JSON_E_LIMIT, "JSON string buffer limit exceeded", 0, 0, "");
        return 0;
    }
    next = (char *)realloc(buffer->data, cap);
    if (!next) {
        buffer->failed = 1;
        nc_set_error(NC_JSON_E_LIMIT, "out of memory growing JSON string buffer", 0, 0, "");
        return 0;
    }
    buffer->data = next;
    buffer->cap = cap;
    return 1;
}

static int nc_buffer_byte(NcJsonBuffer *buffer, char byte) {
    if (!nc_buffer_reserve(buffer, 1)) return 0;
    buffer->data[buffer->len++] = byte;
    buffer->data[buffer->len] = 0;
    return 1;
}

static int nc_buffer_text_len(NcJsonBuffer *buffer, const char *text, u64 len) {
    if (!nc_buffer_reserve(buffer, len)) return 0;
    if (len) nc_memcpy(buffer->data + buffer->len, text, len);
    buffer->len += len;
    buffer->data[buffer->len] = 0;
    return 1;
}

static int nc_buffer_text(NcJsonBuffer *buffer, const char *text) {
    return nc_buffer_text_len(buffer, text, strlen(text));
}

static int nc_parser_peek(const NcJsonParser *parser) {
    return parser->pos < parser->len ? (u8)parser->text[parser->pos] : -1;
}

static int nc_parser_take(NcJsonParser *parser) {
    int byte;
    if (parser->pos >= parser->len) return -1;
    byte = (u8)parser->text[parser->pos++];
    if (byte == '\n') {
        ++parser->line;
        parser->column = 1;
    } else {
        ++parser->column;
    }
    return byte;
}

static void nc_parser_ws(NcJsonParser *parser) {
    int byte;
    while ((byte = nc_parser_peek(parser)) == ' ' || byte == '\t' || byte == '\r' || byte == '\n') {
        nc_parser_take(parser);
    }
}

static int nc_hex(int byte) {
    if (byte >= '0' && byte <= '9') return byte - '0';
    if (byte >= 'a' && byte <= 'f') return byte - 'a' + 10;
    if (byte >= 'A' && byte <= 'F') return byte - 'A' + 10;
    return -1;
}

static int nc_append_utf8(NcJsonBuffer *buffer, u32 codepoint) {
    if (codepoint <= 0x7fU) return nc_buffer_byte(buffer, (char)codepoint);
    if (codepoint <= 0x7ffU) {
        return nc_buffer_byte(buffer, (char)(0xc0U | (codepoint >> 6))) &&
               nc_buffer_byte(buffer, (char)(0x80U | (codepoint & 0x3fU)));
    }
    if (codepoint <= 0xffffU) {
        return nc_buffer_byte(buffer, (char)(0xe0U | (codepoint >> 12))) &&
               nc_buffer_byte(buffer, (char)(0x80U | ((codepoint >> 6) & 0x3fU))) &&
               nc_buffer_byte(buffer, (char)(0x80U | (codepoint & 0x3fU)));
    }
    return nc_buffer_byte(buffer, (char)(0xf0U | (codepoint >> 18))) &&
           nc_buffer_byte(buffer, (char)(0x80U | ((codepoint >> 12) & 0x3fU))) &&
           nc_buffer_byte(buffer, (char)(0x80U | ((codepoint >> 6) & 0x3fU))) &&
           nc_buffer_byte(buffer, (char)(0x80U | (codepoint & 0x3fU)));
}

static int nc_parse_hex4(NcJsonParser *parser, u32 *value) {
    int i;
    int digit;
    *value = 0;
    for (i = 0; i < 4; ++i) {
        digit = nc_hex(nc_parser_take(parser));
        if (digit < 0) return 0;
        *value = (*value << 4) | (u32)digit;
    }
    return 1;
}

static char *nc_parse_string_text(NcJsonParser *parser) {
    NcJsonBuffer out = {(char *)0, 0, 0, 0};
    int byte;
    if (nc_parser_take(parser) != '"') return (char *)0;
    while ((byte = nc_parser_take(parser)) >= 0) {
        if (byte == '"') {
            if (!out.data) return nc_strdup("");
            return out.data;
        }
        if (byte < 0x20) break;
        if (byte != '\\') {
            if (!nc_buffer_byte(&out, (char)byte)) break;
            continue;
        }
        byte = nc_parser_take(parser);
        switch (byte) {
            case '"': case '\\': case '/': if (!nc_buffer_byte(&out, (char)byte)) goto fail; break;
            case 'b': if (!nc_buffer_byte(&out, '\b')) goto fail; break;
            case 'f': if (!nc_buffer_byte(&out, '\f')) goto fail; break;
            case 'n': if (!nc_buffer_byte(&out, '\n')) goto fail; break;
            case 'r': if (!nc_buffer_byte(&out, '\r')) goto fail; break;
            case 't': if (!nc_buffer_byte(&out, '\t')) goto fail; break;
            case 'u': {
                u32 first;
                u32 codepoint;
                if (!nc_parse_hex4(parser, &first)) goto fail;
                codepoint = first;
                if (first >= 0xd800U && first <= 0xdbffU) {
                    u32 second;
                    if (nc_parser_take(parser) != '\\' || nc_parser_take(parser) != 'u' ||
                        !nc_parse_hex4(parser, &second) || second < 0xdc00U || second > 0xdfffU) goto fail;
                    codepoint = 0x10000U + ((first - 0xd800U) << 10) + (second - 0xdc00U);
                } else if (first >= 0xdc00U && first <= 0xdfffU) {
                    goto fail;
                }
                if (!nc_append_utf8(&out, codepoint)) goto fail;
                break;
            }
            default: goto fail;
        }
    }
fail:
    if (out.data) free(out.data);
    nc_set_error(NC_JSON_E_PARSE, "invalid or unterminated JSON string", parser->line, parser->column, "");
    return (char *)0;
}

static int nc_parser_literal(NcJsonParser *parser, const char *literal) {
    u64 i;
    for (i = 0; literal[i]; ++i) {
        if (nc_parser_take(parser) != literal[i]) return 0;
    }
    return 1;
}

static NcJsonNode *nc_parse_value(NcJsonParser *parser, int depth);

static NcJsonNode *nc_parse_array(NcJsonParser *parser, int depth) {
    NcJsonNode *array = nc_node_new(NC_JSON_ARRAY);
    if (!array) return (NcJsonNode *)0;
    nc_parser_take(parser);
    nc_parser_ws(parser);
    if (nc_parser_peek(parser) == ']') {
        nc_parser_take(parser);
        return array;
    }
    for (;;) {
        NcJsonNode *child = nc_parse_value(parser, depth + 1);
        if (!child || !nc_array_append_owned(array, child)) {
            if (child) nc_node_free(child);
            nc_node_free(array);
            return (NcJsonNode *)0;
        }
        nc_parser_ws(parser);
        if (nc_parser_peek(parser) == ']') {
            nc_parser_take(parser);
            return array;
        }
        if (nc_parser_take(parser) != ',') break;
        nc_parser_ws(parser);
    }
    nc_node_free(array);
    nc_set_error(NC_JSON_E_PARSE, "expected ',' or ']' in JSON array", parser->line, parser->column, "");
    return (NcJsonNode *)0;
}

static NcJsonNode *nc_parse_object(NcJsonParser *parser, int depth) {
    NcJsonNode *object = nc_node_new(NC_JSON_OBJECT);
    if (!object) return (NcJsonNode *)0;
    nc_parser_take(parser);
    nc_parser_ws(parser);
    if (nc_parser_peek(parser) == '}') {
        nc_parser_take(parser);
        return object;
    }
    for (;;) {
        char *key;
        NcJsonNode *child;
        if (nc_parser_peek(parser) != '"') break;
        key = nc_parse_string_text(parser);
        if (!key) {
            nc_node_free(object);
            return (NcJsonNode *)0;
        }
        nc_parser_ws(parser);
        if (nc_parser_take(parser) != ':') {
            free(key);
            break;
        }
        nc_parser_ws(parser);
        child = nc_parse_value(parser, depth + 1);
        if (!child || !nc_object_set_owned(object, key, child)) {
            if (child) nc_node_free(child);
            free(key);
            nc_node_free(object);
            return (NcJsonNode *)0;
        }
        nc_parser_ws(parser);
        if (nc_parser_peek(parser) == '}') {
            nc_parser_take(parser);
            return object;
        }
        if (nc_parser_take(parser) != ',') break;
        nc_parser_ws(parser);
    }
    nc_node_free(object);
    nc_set_error(NC_JSON_E_PARSE, "expected JSON object key, ',' or '}'", parser->line, parser->column, "");
    return (NcJsonNode *)0;
}

static NcJsonNode *nc_parse_number(NcJsonParser *parser) {
    u64 start = parser->pos;
    NcJsonNode *node;
    if (nc_parser_peek(parser) == '-') nc_parser_take(parser);
    if (nc_parser_peek(parser) == '0') {
        nc_parser_take(parser);
        if (nc_parser_peek(parser) >= '0' && nc_parser_peek(parser) <= '9') goto invalid;
    } else {
        if (nc_parser_peek(parser) < '1' || nc_parser_peek(parser) > '9') goto invalid;
        while (nc_parser_peek(parser) >= '0' && nc_parser_peek(parser) <= '9') nc_parser_take(parser);
    }
    if (nc_parser_peek(parser) == '.') {
        nc_parser_take(parser);
        if (nc_parser_peek(parser) < '0' || nc_parser_peek(parser) > '9') goto invalid;
        while (nc_parser_peek(parser) >= '0' && nc_parser_peek(parser) <= '9') nc_parser_take(parser);
    }
    if (nc_parser_peek(parser) == 'e' || nc_parser_peek(parser) == 'E') {
        nc_parser_take(parser);
        if (nc_parser_peek(parser) == '+' || nc_parser_peek(parser) == '-') nc_parser_take(parser);
        if (nc_parser_peek(parser) < '0' || nc_parser_peek(parser) > '9') goto invalid;
        while (nc_parser_peek(parser) >= '0' && nc_parser_peek(parser) <= '9') nc_parser_take(parser);
    }
    node = nc_node_new(NC_JSON_NUMBER);
    if (!node) return (NcJsonNode *)0;
    node->text = nc_strdup_len(parser->text + start, parser->pos - start);
    if (!node->text) {
        nc_node_free(node);
        nc_set_error(NC_JSON_E_LIMIT, "out of memory copying JSON number", parser->line, parser->column, "");
        return (NcJsonNode *)0;
    }
    return node;
invalid:
    nc_set_error(NC_JSON_E_PARSE, "invalid JSON number", parser->line, parser->column, "");
    return (NcJsonNode *)0;
}

static NcJsonNode *nc_parse_value(NcJsonParser *parser, int depth) {
    int byte;
    NcJsonNode *node;
    if (depth > NC_JSON_MAX_DEPTH || ++parser->nodes > NC_JSON_MAX_NODES) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON depth or node limit exceeded", parser->line, parser->column, "");
        return (NcJsonNode *)0;
    }
    nc_parser_ws(parser);
    byte = nc_parser_peek(parser);
    if (byte == '{') return nc_parse_object(parser, depth);
    if (byte == '[') return nc_parse_array(parser, depth);
    if (byte == '"') {
        node = nc_node_new(NC_JSON_STRING);
        if (!node) return (NcJsonNode *)0;
        node->text = nc_parse_string_text(parser);
        if (!node->text) {
            nc_node_free(node);
            return (NcJsonNode *)0;
        }
        return node;
    }
    if (byte == 'n') {
        if (!nc_parser_literal(parser, "null")) goto invalid_literal;
        return nc_node_new(NC_JSON_NULL);
    }
    if (byte == 't' || byte == 'f') {
        int truth = byte == 't';
        if (!nc_parser_literal(parser, truth ? "true" : "false")) goto invalid_literal;
        node = nc_node_new(NC_JSON_BOOL);
        if (node) node->boolean = truth;
        return node;
    }
    if (byte == '-' || (byte >= '0' && byte <= '9')) return nc_parse_number(parser);
invalid_literal:
    nc_set_error(NC_JSON_E_PARSE, "invalid JSON value", parser->line, parser->column, "");
    return (NcJsonNode *)0;
}

static NcJsonNode *nc_parse_document(const char *text) {
    NcJsonParser parser;
    NcJsonNode *root;
    if (!text) {
        nc_set_error(NC_JSON_E_PARSE, "JSON input is null", 0, 0, "");
        return (NcJsonNode *)0;
    }
    parser.text = text;
    parser.len = strlen(text);
    parser.pos = 0;
    parser.nodes = 0;
    parser.line = 1;
    parser.column = 1;
    if (parser.len > NC_JSON_MAX_INPUT) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON input byte limit exceeded", 0, 0, "");
        return (NcJsonNode *)0;
    }
    if (!nc_utf8_valid(text)) {
        nc_set_error(NC_JSON_E_PARSE, "JSON input is not valid UTF-8", 0, 0, "");
        return (NcJsonNode *)0;
    }
    root = nc_parse_value(&parser, 1);
    if (!root) return (NcJsonNode *)0;
    nc_parser_ws(&parser);
    if (parser.pos != parser.len) {
        nc_node_free(root);
        nc_set_error(NC_JSON_E_PARSE, "trailing bytes after JSON value", parser.line, parser.column, "");
        return (NcJsonNode *)0;
    }
    nc_clear_error();
    return root;
}

static int nc_serialize_string(NcJsonBuffer *out, const char *text) {
    static const char hex[] = "0123456789ABCDEF";
    const u8 *cursor = (const u8 *)text;
    if (!nc_buffer_byte(out, '"')) return 0;
    while (*cursor) {
        u8 byte = *cursor++;
        switch (byte) {
            case '"': if (!nc_buffer_text(out, "\\\"")) return 0; break;
            case '\\': if (!nc_buffer_text(out, "\\\\")) return 0; break;
            case '\b': if (!nc_buffer_text(out, "\\b")) return 0; break;
            case '\f': if (!nc_buffer_text(out, "\\f")) return 0; break;
            case '\n': if (!nc_buffer_text(out, "\\n")) return 0; break;
            case '\r': if (!nc_buffer_text(out, "\\r")) return 0; break;
            case '\t': if (!nc_buffer_text(out, "\\t")) return 0; break;
            default:
                if (byte < 0x20) {
                    char escaped[7];
                    escaped[0] = '\\'; escaped[1] = 'u'; escaped[2] = '0'; escaped[3] = '0';
                    escaped[4] = hex[(byte >> 4) & 15]; escaped[5] = hex[byte & 15]; escaped[6] = 0;
                    if (!nc_buffer_text_len(out, escaped, 6)) return 0;
                } else if (!nc_buffer_byte(out, (char)byte)) return 0;
        }
    }
    return nc_buffer_byte(out, '"');
}

static int nc_serialize_indent(NcJsonBuffer *out, int depth, int indent) {
    int count = depth * indent;
    while (count-- > 0) if (!nc_buffer_byte(out, ' ')) return 0;
    return 1;
}

static int nc_serialize_node(NcJsonBuffer *out, const NcJsonNode *node, int depth, int indent) {
    u64 i;
    if (!node || depth > NC_JSON_MAX_DEPTH) {
        nc_set_error(NC_JSON_E_SERIALIZE, "invalid JSON tree during serialization", 0, 0, "");
        return 0;
    }
    switch (node->kind) {
        case NC_JSON_NULL: return nc_buffer_text(out, "null");
        case NC_JSON_BOOL: return nc_buffer_text(out, node->boolean ? "true" : "false");
        case NC_JSON_NUMBER: return nc_buffer_text(out, node->text ? node->text : "0");
        case NC_JSON_STRING: return nc_serialize_string(out, node->text ? node->text : "");
        case NC_JSON_ARRAY:
            if (!nc_buffer_byte(out, '[')) return 0;
            for (i = 0; i < node->len; ++i) {
                if (i && !nc_buffer_byte(out, ',')) return 0;
                if (indent && (!nc_buffer_byte(out, '\n') || !nc_serialize_indent(out, depth + 1, indent))) return 0;
                if (!nc_serialize_node(out, node->items[i], depth + 1, indent)) return 0;
            }
            if (indent && node->len && (!nc_buffer_byte(out, '\n') || !nc_serialize_indent(out, depth, indent))) return 0;
            return nc_buffer_byte(out, ']');
        case NC_JSON_OBJECT:
            if (!nc_buffer_byte(out, '{')) return 0;
            for (i = 0; i < node->len; ++i) {
                if (i && !nc_buffer_byte(out, ',')) return 0;
                if (indent && (!nc_buffer_byte(out, '\n') || !nc_serialize_indent(out, depth + 1, indent))) return 0;
                if (!nc_serialize_string(out, node->keys[i]) || !nc_buffer_byte(out, ':')) return 0;
                if (indent && !nc_buffer_byte(out, ' ')) return 0;
                if (!nc_serialize_node(out, node->items[i], depth + 1, indent)) return 0;
            }
            if (indent && node->len && (!nc_buffer_byte(out, '\n') || !nc_serialize_indent(out, depth, indent))) return 0;
            return nc_buffer_byte(out, '}');
        default:
            nc_set_error(NC_JSON_E_SERIALIZE, "unknown JSON node kind", 0, 0, "");
            return 0;
    }
}

static char *nc_serialize(const NcJsonNode *node, int indent) {
    NcJsonBuffer out = {(char *)0, 0, 0, 0};
    if (indent < 0) indent = 0;
    if (indent > 8) indent = 8;
    if (!nc_serialize_node(&out, node, 0, indent)) {
        if (out.data) free(out.data);
        if (g_json_error.code == NC_JSON_E_NONE)
            nc_set_error(NC_JSON_E_SERIALIZE, "JSON serialization failed", 0, 0, "");
        return (char *)0;
    }
    if (!out.data) out.data = nc_strdup("");
    nc_clear_error();
    return out.data;
}

static NcJsonNode *nc_object_find(NcJsonNode *object, const char *key, u64 *index_out) {
    u64 i;
    if (!object || object->kind != NC_JSON_OBJECT) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON object", 0, 0, "");
        return (NcJsonNode *)0;
    }
    for (i = 0; i < object->len; ++i) {
        if (nc_streq(object->keys[i], key)) {
            if (index_out) *index_out = i;
            return object->items[i];
        }
    }
    nc_set_error(NC_JSON_E_KEY, "JSON object key not found", 0, 0, key ? key : "");
    return (NcJsonNode *)0;
}

static int nc_parse_i64_exact(const char *text, i64 *out) {
    const char *cursor = text;
    u64 magnitude = 0;
    u64 limit;
    int negative = 0;
    int digits = 0;
    if (!cursor) return 0;
    if (*cursor == '-') { negative = 1; ++cursor; }
    limit = negative ? 0x8000000000000000ULL : 0x7fffffffffffffffULL;
    while (*cursor >= '0' && *cursor <= '9') {
        u32 digit = (u32)(*cursor - '0');
        if (magnitude > (limit - digit) / 10ULL) return 0;
        magnitude = magnitude * 10ULL + digit;
        ++cursor;
        digits = 1;
    }
    if (!digits || *cursor) return 0;
    if (negative) {
        if (magnitude == 0x8000000000000000ULL) *out = (i64)0x8000000000000000ULL;
        else *out = -(i64)magnitude;
    } else *out = (i64)magnitude;
    return 1;
}

static int nc_parse_f64(const char *text, double *out) {
    const char *cursor = text;
    double value = 0.0;
    double scale = 1.0;
    int negative = 0;
    int exponent = 0;
    int exponent_negative = 0;
    int digits = 0;
    if (!cursor) return 0;
    if (*cursor == '-') { negative = 1; ++cursor; }
    while (*cursor >= '0' && *cursor <= '9') {
        value = value * 10.0 + (double)(*cursor - '0');
        ++cursor;
        digits = 1;
    }
    if (*cursor == '.') {
        ++cursor;
        while (*cursor >= '0' && *cursor <= '9') {
            scale *= 0.1;
            value += (double)(*cursor - '0') * scale;
            ++cursor;
            digits = 1;
        }
    }
    if (!digits) return 0;
    if (*cursor == 'e' || *cursor == 'E') {
        ++cursor;
        if (*cursor == '+' || *cursor == '-') { exponent_negative = *cursor == '-'; ++cursor; }
        if (*cursor < '0' || *cursor > '9') return 0;
        while (*cursor >= '0' && *cursor <= '9') {
            if (exponent < 10000) exponent = exponent * 10 + (*cursor - '0');
            ++cursor;
        }
    }
    if (*cursor) return 0;
    while (exponent-- > 0) value = exponent_negative ? value / 10.0 : value * 10.0;
    *out = negative ? -value : value;
    return 1;
}

static int nc_double_finite(double value) {
    union { double d; u64 bits; } raw;
    raw.d = value;
    return ((raw.bits >> 52) & 0x7ffULL) != 0x7ffULL;
}

static int nc_node_equal(const NcJsonNode *left, const NcJsonNode *right, int depth) {
    u64 i;
    if (!left || !right || left->kind != right->kind || depth > NC_JSON_MAX_DEPTH) return 0;
    if (left->kind == NC_JSON_BOOL && left->boolean != right->boolean) return 0;
    if ((left->kind == NC_JSON_STRING || left->kind == NC_JSON_NUMBER) &&
        !nc_streq(left->text, right->text)) return 0;
    if (left->len != right->len) return 0;
    if (left->kind == NC_JSON_OBJECT) {
        for (i = 0; i < left->len; ++i) {
            u64 right_index;
            int found = 0;
            for (right_index = 0; right_index < right->len; ++right_index) {
                if (nc_streq(left->keys[i], right->keys[right_index])) {
                    found = 1;
                    if (!nc_node_equal(
                            left->items[i], right->items[right_index], depth + 1)) return 0;
                    break;
                }
            }
            if (!found) return 0;
        }
        return 1;
    }
    for (i = 0; i < left->len; ++i) {
        if (!nc_node_equal(left->items[i], right->items[i], depth + 1)) return 0;
    }
    return 1;
}

static char *nc_pointer_decode(const char *encoded, u64 len) {
    NcJsonBuffer out = {(char *)0, 0, 0, 0};
    u64 i;
    for (i = 0; i < len; ++i) {
        if (encoded[i] != '~') {
            if (!nc_buffer_byte(&out, encoded[i])) goto fail;
        } else {
            if (++i >= len) goto invalid;
            if (encoded[i] == '0') { if (!nc_buffer_byte(&out, '~')) goto fail; }
            else if (encoded[i] == '1') { if (!nc_buffer_byte(&out, '/')) goto fail; }
            else goto invalid;
        }
    }
    if (!out.data) out.data = nc_strdup("");
    return out.data;
invalid:
    nc_set_error(NC_JSON_E_POINTER, "invalid JSON Pointer escape", 0, 0, encoded);
fail:
    if (out.data) free(out.data);
    return (char *)0;
}

static NcJsonNode *nc_pointer_resolve(NcJsonNode *root, const char *pointer) {
    const char *cursor = pointer;
    NcJsonNode *current = root;
    if (!pointer) {
        nc_set_error(NC_JSON_E_POINTER, "JSON Pointer is null", 0, 0, "");
        return (NcJsonNode *)0;
    }
    if (!*cursor) return root;
    if (*cursor != '/') {
        nc_set_error(NC_JSON_E_POINTER, "JSON Pointer must start with '/'", 0, 0, pointer);
        return (NcJsonNode *)0;
    }
    while (*cursor) {
        const char *start;
        u64 len;
        char *token;
        ++cursor;
        start = cursor;
        while (*cursor && *cursor != '/') ++cursor;
        len = (u64)(cursor - start);
        token = nc_pointer_decode(start, len);
        if (!token) return (NcJsonNode *)0;
        if (current->kind == NC_JSON_OBJECT) {
            current = nc_object_find(current, token, (u64 *)0);
            free(token);
            if (!current) return (NcJsonNode *)0;
        } else if (current->kind == NC_JSON_ARRAY) {
            u64 index = 0;
            u64 i;
            if (!len || (len > 1 && token[0] == '0') || token[0] == '-') {
                free(token);
                nc_set_error(NC_JSON_E_POINTER, "invalid JSON Pointer array index", 0, 0, pointer);
                return (NcJsonNode *)0;
            }
            for (i = 0; token[i]; ++i) {
                if (token[i] < '0' || token[i] > '9') {
                    free(token);
                    nc_set_error(NC_JSON_E_POINTER, "invalid JSON Pointer array index", 0, 0, pointer);
                    return (NcJsonNode *)0;
                }
                if (index > 0xfffffffffffffffULL) {
                    free(token);
                    nc_set_error(NC_JSON_E_POINTER, "JSON Pointer array index overflow", 0, 0, pointer);
                    return (NcJsonNode *)0;
                }
                index = index * 10ULL + (u64)(token[i] - '0');
            }
            free(token);
            if (index >= current->len) {
                nc_set_error(NC_JSON_E_INDEX, "JSON Pointer array index out of bounds", 0, 0, pointer);
                return (NcJsonNode *)0;
            }
            current = current->items[index];
        } else {
            free(token);
            nc_set_error(NC_JSON_E_TYPE, "JSON Pointer traversed a scalar", 0, 0, pointer);
            return (NcJsonNode *)0;
        }
    }
    return current;
}

static int nc_file_read_exact(const char *path, char **text_out) {
#if defined(_WIN32)
    void *file;
    i64 size = 0;
    char *text;
    u64 offset = 0;
    file = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, (void *)0, OPEN_EXISTING,
                       FILE_ATTRIBUTE_NORMAL, (void *)0);
    if (file == INVALID_HANDLE_VALUE) return 0;
    if (!GetFileSizeEx(file, &size) || size < 0 || (u64)size > NC_JSON_MAX_INPUT) {
        CloseHandle(file);
        return 0;
    }
    text = (char *)malloc((u64)size + 1);
    if (!text) { CloseHandle(file); return 0; }
    while (offset < (u64)size) {
        u32 chunk = (u32)(((u64)size - offset) > 0x7fffffffULL ? 0x7fffffffULL : (u64)size - offset);
        u32 read = 0;
        if (!ReadFile(file, text + offset, chunk, &read, (void *)0) || !read) {
            free(text); CloseHandle(file); return 0;
        }
        offset += read;
    }
    CloseHandle(file);
    if ((i64)offset != size) { free(text); return 0; }
    text[size] = 0;
    *text_out = text;
    return 1;
#else
    void *file = fopen(path, "rb");
    long size;
    char *text;
    u64 read_bytes;
    if (!file) return 0;
    if (fseek(file, 0, 2) != 0) { fclose(file); return 0; }
    size = ftell(file);
    if (size < 0 || (u64)size > NC_JSON_MAX_INPUT || fseek(file, 0, 0) != 0) {
        fclose(file);
        return 0;
    }
    text = (char *)malloc((u64)size + 1ULL);
    if (!text) { fclose(file); return 0; }
    read_bytes = fread(text, 1, (u64)size, file);
    if (fclose(file) != 0 || read_bytes != (u64)size) {
        free(text);
        return 0;
    }
    text[read_bytes] = 0;
    *text_out = text;
    return 1;
#endif
}

static int nc_exe_sibling_path(const char *relative, char *out, u64 cap) {
#if defined(_WIN32)
    u32 len;
#else
    u64 len;
#endif
    i64 slash = -1;
    u64 i;
    u64 rel_len;
    if (!relative || !out || cap < 4) return 0;
#if defined(_WIN32)
    len = GetModuleFileNameA((void *)0, out, (u32)cap);
    if (!len || (u64)len >= cap) return 0;
#elif defined(__APPLE__)
    {
        u32 requested = (u32)cap;
        if (_NSGetExecutablePath(out, &requested) != 0) return 0;
        len = strlen(out);
        if (!len || len >= cap) return 0;
    }
#else
    {
        long result = readlink("/proc/self/exe", out, cap - 1ULL);
        if (result <= 0 || (u64)result >= cap) return 0;
        len = (u64)result;
        out[len] = 0;
    }
#endif
    for (i = 0; i < len; ++i) if (out[i] == '\\' || out[i] == '/') slash = (i64)i;
    if (slash < 0) return 0;
    rel_len = strlen(relative);
    if ((u64)slash + 1 + rel_len >= cap) return 0;
    nc_memcpy(out + slash + 1, relative, rel_len + 1);
    return 1;
}

static int nc_path_is_relative(const char *path) {
    if (!path || !*path) return 0;
    if ((path[0] == '\\' && path[1] == '\\') || path[0] == '/') return 0;
    if (((path[0] >= 'A' && path[0] <= 'Z') || (path[0] >= 'a' && path[0] <= 'z')) && path[1] == ':') return 0;
    return 1;
}

static int nc_path_basename_is_settings(const char *path) {
    static const char expected[] = "settings.json";
    const char *base = path;
    u64 index = 0;
    if (!path) return 0;
    while (*path) {
        if (*path == '\\' || *path == '/') base = path + 1;
        ++path;
    }
    while (base[index] && expected[index]) {
        char left = base[index];
        char right = expected[index];
        if (left >= 'A' && left <= 'Z') left = (char)(left + ('a' - 'A'));
        if (right >= 'A' && right <= 'Z') right = (char)(right + ('a' - 'A'));
        if (left != right) return 0;
        ++index;
    }
    return base[index] == 0 && expected[index] == 0;
}

static int nc_file_write_atomic(const char *path, const char *text) {
    char *temp;
    u64 path_len = strlen(path);
    u64 text_len = strlen(text);
    void *file;
    int ok = 1;
    temp = (char *)malloc(path_len + 16);
    if (!temp) return 0;
    nc_memcpy(temp, path, path_len);
    nc_memcpy(temp + path_len, ".nctmp", 7);
#if defined(_WIN32)
    {
        u64 offset = 0;
        file = CreateFileA(temp, GENERIC_WRITE, 0, (void *)0, CREATE_ALWAYS,
                       FILE_ATTRIBUTE_NORMAL, (void *)0);
        if (file == INVALID_HANDLE_VALUE) { free(temp); return 0; }
        while (offset < text_len) {
            u32 chunk = (u32)((text_len - offset) > 0x7fffffffULL ? 0x7fffffffULL : text_len - offset);
            u32 written = 0;
            if (!WriteFile(file, text + offset, chunk, &written, (void *)0) || written != chunk) {
                ok = 0; break;
            }
            offset += written;
        }
        if (ok) ok = FlushFileBuffers(file);
        CloseHandle(file);
        if (ok) ok = MoveFileExA(temp, path, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH);
        if (!ok) DeleteFileA(temp);
    }
#else
    file = fopen(temp, "wb");
    if (!file) { free(temp); return 0; }
    if (text_len && fwrite(text, 1, text_len, file) != text_len) ok = 0;
    if (ok && fflush(file) != 0) ok = 0;
    if (ok && fsync(fileno(file)) != 0) ok = 0;
    if (fclose(file) != 0) ok = 0;
    if (ok && rename(temp, path) != 0) ok = 0;
    if (!ok) remove(temp);
#endif
    free(temp);
    return ok;
}

static int nc_read_with_exe_fallback(const char *path, char **text_out, char *resolved, u64 cap) {
    if (nc_file_read_exact(path, text_out)) {
        nc_copy_fixed(resolved, cap, path);
        return 1;
    }
    if (nc_path_is_relative(path) && nc_exe_sibling_path(path, resolved, cap) &&
        nc_file_read_exact(resolved, text_out)) return 1;
    return 0;
}

static int nc_write_with_exe_fallback(const char *path, const char *text, char *resolved, u64 cap) {
    if (nc_path_is_relative(path) && nc_path_basename_is_settings(path) &&
        nc_exe_sibling_path(path, resolved, cap) && nc_file_write_atomic(resolved, text)) return 1;
    if (nc_file_write_atomic(path, text)) {
        nc_copy_fixed(resolved, cap, path);
        return 1;
    }
    if (nc_path_is_relative(path) && nc_exe_sibling_path(path, resolved, cap) &&
        nc_file_write_atomic(resolved, text)) return 1;
    return 0;
}

/* Public ABI: 46 symbols, all using the Win64 C calling convention. */

i64 jsonParse(const char *text) {
    return nc_handle_expose(nc_parse_document(text));
}

int jsonValidate(const char *text) {
    NcJsonNode *root = nc_parse_document(text);
    if (!root) return 0;
    nc_node_free(root);
    nc_clear_error();
    return 1;
}

i64 jsonParseFile(const char *path) {
    char *text = (char *)0;
    char resolved[1024];
    NcJsonNode *root;
    if (!path || !*path) {
        nc_set_error(NC_JSON_E_IO, "JSON file path is empty", 0, 0, path ? path : "");
        return 0;
    }
    if (nc_file_read_exact(path, &text)) {
        root = nc_parse_document(text);
        free(text);
        if (root) return nc_handle_expose(root);
        /* A relative cwd file that is stale/corrupt does not mask the valid
         * deployment copy next to the executable. */
        if (!nc_path_is_relative(path) || !nc_exe_sibling_path(path, resolved, 1024) ||
            !nc_file_read_exact(resolved, &text)) {
            nc_copy_fixed(g_json_error.path, 512, path);
            return 0;
        }
        root = nc_parse_document(text);
        free(text);
        if (!root) {
            nc_copy_fixed(g_json_error.path, 512, resolved);
            return 0;
        }
        return nc_handle_expose(root);
    }
    if (nc_path_is_relative(path) && nc_exe_sibling_path(path, resolved, 1024) &&
        nc_file_read_exact(resolved, &text)) {
        root = nc_parse_document(text);
        free(text);
        if (!root) {
            nc_copy_fixed(g_json_error.path, 512, resolved);
            return 0;
        }
        return nc_handle_expose(root);
    }
    nc_set_error(NC_JSON_E_IO, "cannot read JSON file from cwd or executable directory", 0, 0, path);
    return 0;
}

char *jsonStringify(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    return node ? nc_serialize(node, 0) : (char *)g_json_empty;
}

char *jsonStringifyPretty(i64 handle, i64 indent) {
    NcJsonNode *node = nc_handle_lookup(handle);
    if (!node) return (char *)g_json_empty;
    if (indent < 0) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON indentation cannot be negative", 0, 0, "");
        return (char *)g_json_empty;
    }
    return nc_serialize(node, (int)indent);
}

int jsonSaveFile(i64 handle, const char *path, i64 indent) {
    NcJsonNode *node = nc_handle_lookup(handle);
    char *text;
    char resolved[1024];
    int ok;
    if (!node) return 0;
    if (!path || !*path || indent < 0) {
        nc_set_error(NC_JSON_E_IO, "invalid JSON save path or indentation", 0, 0, path ? path : "");
        return 0;
    }
    text = nc_serialize(node, (int)indent);
    if (!text) return 0;
    ok = nc_write_with_exe_fallback(path, text, resolved, 1024);
    free(text);
    if (!ok) {
        nc_set_error(NC_JSON_E_IO, "cannot save JSON file in cwd or executable directory", 0, 0, path);
        return 0;
    }
    nc_clear_error();
    return 1;
}

i64 jsonClone(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    if (!node) return 0;
    return nc_handle_expose(nc_node_clone_depth(node, 1));
}

int jsonRelease(i64 handle) { return nc_handle_release(handle); }

i64 jsonKind(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    if (!node) return 0;
    nc_clear_error();
    return node->kind;
}

int jsonIsNull(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    if (!node) return 0;
    nc_clear_error();
    return node->kind == NC_JSON_NULL;
}

int jsonEquals(i64 left_handle, i64 right_handle) {
    NcJsonNode *left = nc_handle_lookup(left_handle);
    NcJsonNode *right;
    if (!left) return 0;
    right = nc_handle_lookup(right_handle);
    if (!right) return 0;
    nc_clear_error();
    return nc_node_equal(left, right, 1);
}

char *jsonGetString(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    char *copy;
    if (!node) return (char *)g_json_empty;
    if (node->kind != NC_JSON_STRING) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON string", 0, 0, "");
        return (char *)g_json_empty;
    }
    copy = nc_strdup(node->text ? node->text : "");
    if (!copy) {
        nc_set_error(NC_JSON_E_LIMIT, "out of memory returning JSON string", 0, 0, "");
        return (char *)g_json_empty;
    }
    nc_clear_error();
    return copy;
}

int jsonGetBool(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    if (!node) return 0;
    if (node->kind != NC_JSON_BOOL) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON bool", 0, 0, "");
        return 0;
    }
    nc_clear_error();
    return node->boolean;
}

i64 jsonGetInt(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    i64 value;
    if (!node) return 0;
    if (node->kind != NC_JSON_NUMBER || !nc_parse_i64_exact(node->text, &value)) {
        nc_set_error(NC_JSON_E_TYPE, "expected exact signed integer JSON number", 0, 0, "");
        return 0;
    }
    nc_clear_error();
    return value;
}

double jsonGetFloat(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    double value = 0.0;
    if (!node) return 0.0;
    if (node->kind != NC_JSON_NUMBER || !nc_parse_f64(node->text, &value) || !nc_double_finite(value)) {
        nc_set_error(NC_JSON_E_TYPE, "expected finite JSON number", 0, 0, "");
        return 0.0;
    }
    nc_clear_error();
    return value;
}

char *jsonGetNumberText(i64 handle) {
    NcJsonNode *node = nc_handle_lookup(handle);
    char *copy;
    if (!node) return (char *)g_json_empty;
    if (node->kind != NC_JSON_NUMBER) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON number", 0, 0, "");
        return (char *)g_json_empty;
    }
    copy = nc_strdup(node->text);
    if (!copy) {
        nc_set_error(NC_JSON_E_LIMIT, "out of memory returning JSON number text", 0, 0, "");
        return (char *)g_json_empty;
    }
    nc_clear_error();
    return copy;
}

char *jsonGetStringOr(i64 handle, const char *fallback) {
    NcJsonNode *node = nc_handle_lookup(handle);
    char *copy;
    if (!node || node->kind != NC_JSON_STRING) {
        if (node) nc_set_error(NC_JSON_E_TYPE, "expected JSON string; fallback used", 0, 0, "");
        return (char *)(fallback ? fallback : g_json_empty);
    }
    copy = nc_strdup(node->text ? node->text : "");
    if (!copy) {
        nc_set_error(NC_JSON_E_LIMIT, "out of memory returning JSON string", 0, 0, "");
        return (char *)(fallback ? fallback : g_json_empty);
    }
    nc_clear_error();
    return copy;
}

int jsonGetBoolOr(i64 handle, int fallback) {
    NcJsonNode *node = nc_handle_lookup(handle);
    if (!node || node->kind != NC_JSON_BOOL) {
        if (node) nc_set_error(NC_JSON_E_TYPE, "expected JSON bool; fallback used", 0, 0, "");
        return fallback != 0;
    }
    nc_clear_error();
    return node->boolean;
}

i64 jsonGetIntOr(i64 handle, i64 fallback) {
    NcJsonNode *node = nc_handle_lookup(handle);
    i64 value;
    if (!node || node->kind != NC_JSON_NUMBER || !nc_parse_i64_exact(node->text, &value)) {
        if (node) nc_set_error(NC_JSON_E_TYPE, "expected exact integer; fallback used", 0, 0, "");
        return fallback;
    }
    nc_clear_error();
    return value;
}

double jsonGetFloatOr(i64 handle, double fallback) {
    NcJsonNode *node = nc_handle_lookup(handle);
    double value;
    if (!node || node->kind != NC_JSON_NUMBER || !nc_parse_f64(node->text, &value) || !nc_double_finite(value)) {
        if (node) nc_set_error(NC_JSON_E_TYPE, "expected finite number; fallback used", 0, 0, "");
        return fallback;
    }
    nc_clear_error();
    return value;
}

i64 jsonArrayLen(i64 handle) {
    NcJsonNode *array = nc_handle_lookup(handle);
    if (!array) return 0;
    if (array->kind != NC_JSON_ARRAY) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON array", 0, 0, "");
        return 0;
    }
    nc_clear_error();
    return (i64)array->len;
}

i64 jsonArrayGet(i64 handle, i64 index) {
    NcJsonNode *array = nc_handle_lookup(handle);
    NcJsonNode *copy;
    if (!array) return 0;
    if (array->kind != NC_JSON_ARRAY) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON array", 0, 0, "");
        return 0;
    }
    if (index < 0 || (u64)index >= array->len) {
        nc_set_error(NC_JSON_E_INDEX, "JSON array index out of bounds", 0, 0, "");
        return 0;
    }
    copy = nc_node_clone_depth(array->items[index], 1);
    return nc_handle_expose(copy);
}

int jsonArrayPush(i64 handle, i64 value_handle) {
    NcJsonNode *array = nc_handle_lookup(handle);
    NcJsonNode *value;
    NcJsonNode *copy;
    u64 count = 0;
    if (!array) return 0;
    value = nc_handle_lookup(value_handle);
    if (!value) return 0;
    if (array->kind != NC_JSON_ARRAY) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON array", 0, 0, "");
        return 0;
    }
    copy = nc_node_clone_depth(value, 2);
    if (!copy || !nc_array_append_owned(array, copy)) { if (copy) nc_node_free(copy); return 0; }
    if (!nc_tree_limits(array, 1, &count)) {
        --array->len; nc_node_free(copy); return 0;
    }
    nc_clear_error();
    return 1;
}

int jsonArrayInsert(i64 handle, i64 index, i64 value_handle) {
    NcJsonNode *array = nc_handle_lookup(handle);
    NcJsonNode *value;
    NcJsonNode *copy;
    u64 i;
    u64 count = 0;
    if (!array) return 0;
    value = nc_handle_lookup(value_handle);
    if (!value) return 0;
    if (array->kind != NC_JSON_ARRAY) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON array", 0, 0, ""); return 0;
    }
    if (index < 0 || (u64)index > array->len) {
        nc_set_error(NC_JSON_E_INDEX, "JSON array insertion index out of bounds", 0, 0, ""); return 0;
    }
    copy = nc_node_clone_depth(value, 2);
    if (!copy || !nc_node_reserve(array, array->len + 1)) { if (copy) nc_node_free(copy); return 0; }
    for (i = array->len; i > (u64)index; --i) array->items[i] = array->items[i - 1];
    array->items[index] = copy;
    ++array->len;
    if (!nc_tree_limits(array, 1, &count)) {
        for (i = (u64)index; i + 1 < array->len; ++i) array->items[i] = array->items[i + 1];
        --array->len; nc_node_free(copy); return 0;
    }
    nc_clear_error();
    return 1;
}

i64 jsonArrayRemove(i64 handle, i64 index) {
    NcJsonNode *array = nc_handle_lookup(handle);
    NcJsonNode *removed;
    u64 i;
    if (!array) return 0;
    if (array->kind != NC_JSON_ARRAY) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON array", 0, 0, ""); return 0;
    }
    if (index < 0 || (u64)index >= array->len) {
        nc_set_error(NC_JSON_E_INDEX, "JSON array index out of bounds", 0, 0, ""); return 0;
    }
    if (g_json_handle_count >= NC_JSON_MAX_HANDLES) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON handle limit reached", 0, 0, ""); return 0;
    }
    removed = array->items[index];
    for (i = (u64)index; i + 1 < array->len; ++i) array->items[i] = array->items[i + 1];
    --array->len;
    return nc_handle_expose(removed);
}

i64 jsonObjectLen(i64 handle) {
    NcJsonNode *object = nc_handle_lookup(handle);
    if (!object) return 0;
    if (object->kind != NC_JSON_OBJECT) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON object", 0, 0, ""); return 0;
    }
    nc_clear_error();
    return (i64)object->len;
}

int jsonObjectHas(i64 handle, const char *key) {
    NcJsonNode *object = nc_handle_lookup(handle);
    u64 i;
    if (!object) return 0;
    if (object->kind != NC_JSON_OBJECT) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON object", 0, 0, ""); return 0;
    }
    for (i = 0; i < object->len; ++i) if (nc_streq(object->keys[i], key)) { nc_clear_error(); return 1; }
    nc_clear_error();
    return 0;
}

i64 jsonObjectGet(i64 handle, const char *key) {
    NcJsonNode *object = nc_handle_lookup(handle);
    NcJsonNode *child;
    if (!object) return 0;
    child = nc_object_find(object, key, (u64 *)0);
    if (!child) return 0;
    return nc_handle_expose(nc_node_clone_depth(child, 1));
}

char *jsonObjectKeyAt(i64 handle, i64 index) {
    NcJsonNode *object = nc_handle_lookup(handle);
    char *copy;
    if (!object) return (char *)g_json_empty;
    if (object->kind != NC_JSON_OBJECT) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON object", 0, 0, ""); return (char *)g_json_empty;
    }
    if (index < 0 || (u64)index >= object->len) {
        nc_set_error(NC_JSON_E_INDEX, "JSON object key index out of bounds", 0, 0, ""); return (char *)g_json_empty;
    }
    copy = nc_strdup(object->keys[index]);
    if (!copy) { nc_set_error(NC_JSON_E_LIMIT, "out of memory returning JSON key", 0, 0, ""); return (char *)g_json_empty; }
    nc_clear_error();
    return copy;
}

int jsonObjectSet(i64 handle, const char *key, i64 value_handle) {
    NcJsonNode *object = nc_handle_lookup(handle);
    NcJsonNode *value;
    NcJsonNode *copy;
    char *key_copy;
    u64 object_count = 0;
    u64 value_count = 0;
    u64 replaced_count = 0;
    u64 i;
    if (!object) return 0;
    value = nc_handle_lookup(value_handle);
    if (!value) return 0;
    if (object->kind != NC_JSON_OBJECT) {
        nc_set_error(NC_JSON_E_TYPE, "expected JSON object", 0, 0, ""); return 0;
    }
    if (!key || !nc_utf8_valid(key)) {
        nc_set_error(NC_JSON_E_KEY, "JSON object key is null or invalid UTF-8", 0, 0, ""); return 0;
    }
    copy = nc_node_clone_depth(value, 2);
    key_copy = nc_strdup(key);
    if (!copy || !key_copy) { if (copy) nc_node_free(copy); if (key_copy) free(key_copy); return 0; }
    if (!nc_tree_limits(object, 1, &object_count) ||
        !nc_tree_limits(copy, 2, &value_count)) {
        free(key_copy); nc_node_free(copy); return 0;
    }
    for (i = 0; i < object->len; ++i) {
        if (nc_streq(object->keys[i], key)) {
            if (!nc_tree_limits(object->items[i], 2, &replaced_count)) {
                free(key_copy); nc_node_free(copy); return 0;
            }
            break;
        }
    }
    if (object_count - replaced_count + value_count > NC_JSON_MAX_NODES) {
        free(key_copy); nc_node_free(copy);
        nc_set_error(NC_JSON_E_LIMIT, "JSON node limit exceeded by object mutation", 0, 0, key);
        return 0;
    }
    if (!nc_object_set_owned(object, key_copy, copy)) { free(key_copy); nc_node_free(copy); return 0; }
    nc_clear_error();
    return 1;
}

i64 jsonObjectRemove(i64 handle, const char *key) {
    NcJsonNode *object = nc_handle_lookup(handle);
    u64 index;
    u64 i;
    NcJsonNode *removed;
    if (!object) return 0;
    if (!nc_object_find(object, key, &index)) return 0;
    if (g_json_handle_count >= NC_JSON_MAX_HANDLES) {
        nc_set_error(NC_JSON_E_LIMIT, "JSON handle limit reached", 0, 0, key ? key : ""); return 0;
    }
    removed = object->items[index];
    free(object->keys[index]);
    for (i = index; i + 1 < object->len; ++i) {
        object->keys[i] = object->keys[i + 1];
        object->items[i] = object->items[i + 1];
    }
    --object->len;
    return nc_handle_expose(removed);
}

i64 jsonPointer(i64 handle, const char *pointer) {
    NcJsonNode *root = nc_handle_lookup(handle);
    NcJsonNode *node;
    if (!root) return 0;
    node = nc_pointer_resolve(root, pointer);
    if (!node) return 0;
    return nc_handle_expose(nc_node_clone_depth(node, 1));
}

i64 jsonNewNull(void) { return nc_handle_expose(nc_node_new(NC_JSON_NULL)); }

i64 jsonNewBool(int value) {
    NcJsonNode *node = nc_node_new(NC_JSON_BOOL);
    if (node) node->boolean = value != 0;
    return nc_handle_expose(node);
}

i64 jsonNewInt(i64 value) {
    NcJsonNode *node = nc_node_new(NC_JSON_NUMBER);
    char buffer[64];
    int written;
    if (!node) return 0;
    written = _snprintf(buffer, 64, "%lld", value);
    if (written <= 0 || written >= 64) { nc_node_free(node); nc_set_error(NC_JSON_E_NUMBER, "cannot format JSON integer", 0, 0, ""); return 0; }
    node->text = nc_strdup(buffer);
    if (!node->text) { nc_node_free(node); return 0; }
    return nc_handle_expose(node);
}

i64 jsonNewFloat(double value) {
    NcJsonNode *node;
    char buffer[64];
    int written;
    if (!nc_double_finite(value)) {
        nc_set_error(NC_JSON_E_NUMBER, "JSON numbers cannot be NaN or infinite", 0, 0, ""); return 0;
    }
    node = nc_node_new(NC_JSON_NUMBER);
    if (!node) return 0;
    written = _snprintf(buffer, 64, "%.17g", value);
    if (written <= 0 || written >= 64) { nc_node_free(node); nc_set_error(NC_JSON_E_NUMBER, "cannot format JSON float", 0, 0, ""); return 0; }
    node->text = nc_strdup(buffer);
    if (!node->text) { nc_node_free(node); return 0; }
    return nc_handle_expose(node);
}

i64 jsonNewString(const char *value) {
    if (value && !nc_utf8_valid(value)) {
        nc_set_error(NC_JSON_E_PARSE, "JSON string is not valid UTF-8", 0, 0, ""); return 0;
    }
    NcJsonNode *node = nc_node_new(NC_JSON_STRING);
    if (!node) return 0;
    node->text = nc_strdup(value ? value : "");
    if (!node->text) { nc_node_free(node); return 0; }
    return nc_handle_expose(node);
}

i64 jsonNewArray(void) { return nc_handle_expose(nc_node_new(NC_JSON_ARRAY)); }
i64 jsonNewObject(void) { return nc_handle_expose(nc_node_new(NC_JSON_OBJECT)); }

i64 jsonErrorCode(void) { return (i64)g_json_error.code; }
char *jsonErrorName(void) { return (char *)nc_error_name(g_json_error.code); }
char *jsonErrorMessage(void) { return g_json_error.message; }
i64 jsonErrorLine(void) { return g_json_error.line; }
i64 jsonErrorColumn(void) { return g_json_error.column; }
char *jsonErrorPath(void) { return g_json_error.path; }
int jsonClearError(void) { nc_clear_error(); return 1; }
