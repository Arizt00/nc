/* Hosted runtime ABI used by native .n executables.
 *
 * The file is deliberately header-free so Clang can build it for ELF,
 * Mach-O and PE targets. Result<T,E>/Option<T> use one tagged 64-bit word:
 * bit 0 is the positive tag and bits 1..63 hold the signed scalar or pointer
 * payload. Heap-backed handles are consumed by wait/join.
 */

typedef unsigned char nc_u8;
typedef unsigned int nc_u32;
typedef unsigned long long nc_u64;
typedef long long nc_i64;
typedef __UINTPTR_TYPE__ nc_uptr;

extern void *malloc(nc_u64);
extern void *realloc(void *, nc_u64);
extern void free(void *);
extern nc_u64 strlen(const char *);
extern int snprintf(char *, nc_u64, const char *, ...);
extern char *getenv(const char *);
extern long long strtoll(const char *, char **, int);
extern double strtod(const char *, char **);
extern int fflush(void *);

static char *nc_copy_string(const char *value) {
    nc_u64 bytes = strlen(value) + 1ULL;
    char *copy = (char *)malloc(bytes);
    nc_u64 index;
    if (!copy) return (char *)0;
    for (index = 0; index < bytes; index += 1ULL) copy[index] = value[index];
    return copy;
}

static char *nc_string_empty(void) {
    char *out = (char *)malloc(1ULL);
    if (out) out[0] = 0;
    return out;
}

static int nc_string_equal(const char *left, const char *right) {
    while (*left && *left == *right) {
        left += 1;
        right += 1;
    }
    return *left == *right;
}

static int nc_ascii_space(char value) {
    return value == ' ' || value == '\t' || value == '\n' || value == '\r' ||
           value == '\f' || value == '\v';
}


#if defined(_WIN32)

typedef struct NcFileTime {
    nc_u32 low;
    nc_u32 high;
} NcFileTime;

typedef struct NcWin32FindDataA {
    nc_u32 attributes;
    NcFileTime creation_time;
    NcFileTime access_time;
    NcFileTime write_time;
    nc_u32 size_high;
    nc_u32 size_low;
    nc_u32 reserved0;
    nc_u32 reserved1;
    char file_name[260];
    char alternate_file_name[14];
} NcWin32FindDataA;

extern void *CreateFileA(const char *, nc_u32, nc_u32, void *, nc_u32, nc_u32, void *);
extern int WriteFile(void *, const void *, nc_u32, nc_u32 *, void *);
extern int ReadFile(void *, void *, nc_u32, nc_u32 *, void *);
extern int FlushFileBuffers(void *);
extern int GetFileSizeEx(void *, nc_i64 *);
extern int CloseHandle(void *);
extern nc_u32 GetFileAttributesA(const char *);
extern int DeleteFileA(const char *);
extern int MoveFileExA(const char *, const char *, nc_u32);
extern int CreateDirectoryA(const char *, void *);
extern int RemoveDirectoryA(const char *);
extern nc_u32 GetCurrentDirectoryA(nc_u32, char *);
extern int SetCurrentDirectoryA(const char *);
extern void *FindFirstFileA(const char *, NcWin32FindDataA *);
extern int FindNextFileA(void *, NcWin32FindDataA *);
extern int FindClose(void *);
extern void *GetStdHandle(nc_u32);
extern int GetConsoleMode(void *, nc_u32 *);
extern int SetConsoleMode(void *, nc_u32);

#define NC_INVALID_HANDLE ((void *)(nc_i64)-1)
#define NC_GENERIC_READ 0x80000000U
#define NC_GENERIC_WRITE 0x40000000U
#define NC_SHARE_READ 0x00000001U
#define NC_OPEN_EXISTING 3U
#define NC_CREATE_ALWAYS 2U
#define NC_OPEN_ALWAYS 4U
#define NC_FILE_NORMAL 0x00000080U
#define NC_MOVE_REPLACE_EXISTING 0x00000001U
#define NC_MOVE_WRITE_THROUGH 0x00000008U
#define NC_INVALID_ATTRIBUTES 0xffffffffU
#define NC_STD_INPUT_HANDLE ((nc_u32)-10)
#define NC_STD_OUTPUT_HANDLE ((nc_u32)-11)
#define NC_STD_ERROR_HANDLE ((nc_u32)-12)
#else

extern int open(const char *, int, ...);
extern nc_i64 write(int, const void *, nc_u64);
extern nc_i64 read(int, void *, nc_u64);
extern nc_i64 lseek(int, nc_i64, int);
extern int fsync(int);
extern int close(int);
extern int unlink(const char *);
extern int rename(const char *, const char *);
extern int mkdir(const char *, unsigned int);
extern int rmdir(const char *);
extern char *getcwd(char *, nc_u64);
extern int chdir(const char *);
extern int usleep(nc_u32);
extern int isatty(int);
extern void *opendir(const char *);
extern int closedir(void *);

#if defined(__APPLE__)
typedef struct NcDirent {
    nc_u64 inode;
    nc_u64 seek_offset;
    unsigned short record_length;
    unsigned short name_length;
    nc_u8 type;
    char name[1024];
} NcDirent;
#else
typedef struct NcDirent {
    nc_u64 inode;
    nc_i64 offset;
    unsigned short record_length;
    nc_u8 type;
    char name[256];
} NcDirent;
#endif

extern NcDirent *readdir(void *);

#if defined(__APPLE__)
#define NC_O_CREAT 0x0200
#define NC_O_TRUNC 0x0400
#else
#define NC_O_CREAT 0x0040
#define NC_O_TRUNC 0x0200
#endif
#define NC_O_RDONLY 0
#define NC_O_WRONLY 1
#define NC_SEEK_SET 0
#define NC_SEEK_END 2

#endif

/* Generic network is materialized by ncc-ir/network_llvm.rs. Mobile providers
 * consume the same stable ABI instead of owning a second socket runtime. */
extern nc_u64 ncc_rt_network_client_open(const char *, nc_i64);
extern int ncc_rt_network_close(nc_u64);
extern int ncc_rt_mobile_network_is_wifi_connected(void);

typedef struct NcMobileProviderV1 {
    nc_u64 abi_version;
    const char *(*telephony_get_incoming_number)(void);
    const char *(*telephony_get_state)(void);
    int (*telephony_is_available)(void);
    int (*telephony_dial)(const char *);
    int (*telephony_answer)(void);
    int (*telephony_hang_up)(void);
    int (*tts_connect)(const char *);
    int (*tts_speak)(const char *);
    int (*tts_stop)(void);
    int (*tts_is_speaking)(void);
    int (*tts_disconnect)(void);
    int (*network_request)(const char *, nc_i64 *, const char **);
    int (*permission_request)(nc_u64);
} NcMobileProviderV1;

typedef struct NcMobileProviderV2 {
    NcMobileProviderV1 v1;
    int (*telephony_adb_connect)(nc_i64);
    int (*network_is_wifi_connected)(void);
} NcMobileProviderV2;

static const NcMobileProviderV1 *nc_mobile_active_provider(void);

typedef struct NcMobileResponse {
    nc_i64 status;
    char *body;
} NcMobileResponse;

static const NcMobileProviderV1 *nc_mobile_provider = (const NcMobileProviderV1 *)0;

#if defined(__GNUC__) && !defined(_WIN32)
extern const NcMobileProviderV1 *nc_mobile_provider_v1(void) __attribute__((weak));
#endif

int ncc_rt_mobile_install_provider(const NcMobileProviderV1 *provider) {
    if (!provider || (provider->abi_version != 1ULL && provider->abi_version != 2ULL)) return 0;
    nc_mobile_provider = provider;
    return 1;
}

nc_i64 ncc_rt_mobile_adb_connect(nc_i64 port) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    if (provider && provider->abi_version >= 2ULL) {
        const NcMobileProviderV2 *provider_v2 = (const NcMobileProviderV2 *)provider;
        if (provider_v2->telephony_adb_connect) return provider_v2->telephony_adb_connect(port);
    }
    {
        nc_u64 probe = ncc_rt_network_client_open("127.0.0.1", port);
        if (!probe) return 1;
        ncc_rt_network_close(probe);
        return 0;
    }
}

int ncc_rt_mobile_network_is_wifi_connected_provider(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    if (provider && provider->abi_version >= 2ULL) {
        const NcMobileProviderV2 *provider_v2 = (const NcMobileProviderV2 *)provider;
        if (provider_v2->network_is_wifi_connected) {
            return provider_v2->network_is_wifi_connected() != 0;
        }
    }
    return ncc_rt_mobile_network_is_wifi_connected();
}

static const NcMobileProviderV1 *nc_mobile_active_provider(void) {
#if defined(__GNUC__) && !defined(_WIN32)
    if (!nc_mobile_provider && nc_mobile_provider_v1) {
        const NcMobileProviderV1 *provider = nc_mobile_provider_v1();
        if (provider && (provider->abi_version == 1ULL || provider->abi_version == 2ULL)) {
            nc_mobile_provider = provider;
        }
    }
#endif
    return nc_mobile_provider;
}

static int nc_env_enabled(const char *name) {
    const char *value = getenv(name);
    if (!value) return 0;
    return value[0] == '1' || value[0] == 'y' || value[0] == 'Y' ||
           value[0] == 't' || value[0] == 'T';
}

static nc_i64 nc_parse_i64(const char *value, nc_i64 fallback) {
    nc_i64 out = 0;
    int negative = 0;
    if (!value || !value[0]) return fallback;
    if (*value == '-') {
        negative = 1;
        value += 1;
    }
    if (*value < '0' || *value > '9') return fallback;
    while (*value >= '0' && *value <= '9') {
        out = out * 10 + (nc_i64)(*value - '0');
        value += 1;
    }
    return negative ? -out : out;
}

static const char *nc_mobile_env_or(const char *name, const char *fallback) {
    const char *value = getenv(name);
    return value ? value : fallback;
}

char *ncc_rt_mobile_get_incoming_number(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    const char *value = provider && provider->telephony_get_incoming_number
        ? provider->telephony_get_incoming_number()
        : nc_mobile_env_or("NCC_MOBILE_INCOMING_NUMBER", "");
    return nc_copy_string(value ? value : "");
}

char *ncc_rt_mobile_get_state(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    const char *value = provider && provider->telephony_get_state
        ? provider->telephony_get_state()
        : nc_mobile_env_or("NCC_MOBILE_CALL_STATE", "IDLE");
    return nc_copy_string(value ? value : "IDLE");
}

int ncc_rt_mobile_telephony_available(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    if (provider && provider->telephony_is_available) {
        return provider->telephony_is_available() != 0;
    }
    return getenv("NCC_MOBILE_CALL_STATE") != (char *)0;
}

int ncc_rt_mobile_dial(const char *number) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    return provider && provider->telephony_dial && number
        ? provider->telephony_dial(number) != 0 : 0;
}

int ncc_rt_mobile_answer(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    return provider && provider->telephony_answer ? provider->telephony_answer() != 0 : 0;
}

int ncc_rt_mobile_hang_up(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    return provider && provider->telephony_hang_up ? provider->telephony_hang_up() != 0 : 0;
}

int ncc_rt_mobile_tts_connect(const char *language) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    if (provider && provider->tts_connect) return provider->tts_connect(language) != 0;
    return language && nc_env_enabled("NCC_MOBILE_TTS_READY");
}

int ncc_rt_mobile_tts_speak(const char *message) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    if (provider && provider->tts_speak) return provider->tts_speak(message) != 0;
    return message && nc_env_enabled("NCC_MOBILE_TTS_READY");
}

int ncc_rt_mobile_tts_stop(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    return provider && provider->tts_stop ? provider->tts_stop() != 0
                                         : nc_env_enabled("NCC_MOBILE_TTS_READY");
}

int ncc_rt_mobile_tts_is_speaking(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    return provider && provider->tts_is_speaking ? provider->tts_is_speaking() != 0 : 0;
}

int ncc_rt_mobile_tts_disconnect(void) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    return provider && provider->tts_disconnect ? provider->tts_disconnect() != 0
                                               : nc_env_enabled("NCC_MOBILE_TTS_READY");
}

nc_u64 ncc_rt_mobile_network_request(const char *url) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    NcMobileResponse *response = (NcMobileResponse *)malloc((nc_u64)sizeof(NcMobileResponse));
    const char *body = "{}";
    nc_i64 status = 0;
    if (!response || !url) {
        free(response);
        return 0;
    }
    if (provider && provider->network_request) {
        if (!provider->network_request(url, &status, &body)) {
            status = 0;
            body = "{}";
        }
    } else {
        status = nc_parse_i64(getenv("NCC_MOBILE_HTTP_STATUS"), 0);
        body = nc_mobile_env_or("NCC_MOBILE_HTTP_JSON", "{}");
    }
    response->status = status;
    response->body = nc_copy_string(body ? body : "{}");
    if (!response->body) {
        free(response);
        return 0;
    }
    return (nc_u64)(void *)response;
}

nc_i64 ncc_rt_mobile_response_status(nc_u64 handle) {
    NcMobileResponse *response = (NcMobileResponse *)(void *)handle;
    return response ? response->status : 0;
}

static char *nc_mobile_json_value(const char *json, const char *key) {
    nc_u64 key_len = strlen(key);
    const char *cursor = json;
    while (cursor && *cursor) {
        const char *candidate;
        nc_u64 index = 0;
        while (*cursor && *cursor != '"') cursor += 1;
        if (!*cursor) break;
        candidate = ++cursor;
        while (index < key_len && candidate[index] == key[index]) index += 1ULL;
        cursor = candidate + index;
        if (index != key_len || *cursor != '"') continue;
        cursor += 1;
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' || *cursor == '\n') cursor += 1;
        if (*cursor != ':') continue;
        cursor += 1;
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' || *cursor == '\n') cursor += 1;
        if (*cursor == '"') {
            const char *start = ++cursor;
            nc_u64 length = 0;
            char *out;
            while (cursor[length] && cursor[length] != '"') length += 1ULL;
            out = (char *)malloc(length + 1ULL);
            if (!out) return (char *)0;
            for (index = 0; index < length; index += 1ULL) out[index] = start[index];
            out[length] = 0;
            return out;
        } else {
            const char *start = cursor;
            nc_u64 length = 0;
            char *out;
            while (cursor[length] && cursor[length] != ',' && cursor[length] != '}') length += 1ULL;
            while (length && (start[length - 1ULL] == ' ' || start[length - 1ULL] == '\t')) length -= 1ULL;
            out = (char *)malloc(length + 1ULL);
            if (!out) return (char *)0;
            for (index = 0; index < length; index += 1ULL) out[index] = start[index];
            out[length] = 0;
            return out;
        }
    }
    return nc_copy_string("");
}

char *ncc_rt_mobile_response_json(nc_u64 handle, const char *key) {
    NcMobileResponse *response = (NcMobileResponse *)(void *)handle;
    if (!response || !response->body || !key) return nc_copy_string("");
    return nc_mobile_json_value(response->body, key);
}

char *ncc_rt_mobile_response_body(nc_u64 handle) {
    NcMobileResponse *response = (NcMobileResponse *)(void *)handle;
    return nc_copy_string(response && response->body ? response->body : "");
}

int ncc_rt_mobile_response_release(nc_u64 handle) {
    NcMobileResponse *response = (NcMobileResponse *)(void *)handle;
    if (!response) return 0;
    free(response->body);
    free(response);
    return 1;
}

int ncc_rt_mobile_permission_request(nc_u64 mask) {
    const NcMobileProviderV1 *provider = nc_mobile_active_provider();
    if (provider && provider->permission_request) {
        return provider->permission_request(mask) != 0;
    }
    if (nc_env_enabled("NCC_MOBILE_ALLOW_PERMISSIONS")) return 1;
    nc_u64 granted = (nc_u64)nc_parse_i64(getenv("NCC_MOBILE_PERMISSION_MASK"), 0);
    return (granted & mask) == mask;
}

static nc_u64 nc_mobile_permission_bit(const char *name) {
    if (!name) return 0;
    if (nc_string_equal(name, "READ_PHONE_STATE") ||
        nc_string_equal(name, "READ_CALL_LOG") ||
        nc_string_equal(name, "CALL_PHONE")) return 1ULL;
    if (nc_string_equal(name, "INTERNET") ||
        nc_string_equal(name, "ACCESS_NETWORK_STATE") ||
        nc_string_equal(name, "ACCESS_WIFI_STATE")) return 2ULL;
    if (nc_string_equal(name, "RECORD_AUDIO")) return 4ULL;
    if (nc_string_equal(name, "READ_EXTERNAL_STORAGE") ||
        nc_string_equal(name, "WRITE_EXTERNAL_STORAGE")) return 8ULL;
    if (nc_string_equal(name, "ACCESS_FINE_LOCATION") ||
        nc_string_equal(name, "ACCESS_COARSE_LOCATION")) return 16ULL;
    return 0;
}

int ncc_rt_mobile_permission_request_names(char **names, nc_u64 count) {
    nc_u64 index;
    nc_u64 mask = 0;
    if (!names && count != 0) return 0;
    for (index = 0; index < count; index += 1ULL) {
        nc_u64 bit = nc_mobile_permission_bit(names[index]);
        if (bit == 0) return 0;
        mask |= bit;
    }
    return ncc_rt_mobile_permission_request(mask);
}

int ncc_rt_mobile_permission_bootstrap(void) {
    return ncc_rt_mobile_permission_request(1ULL | 2ULL);
}

int ncc_rt_mobile_server_bootstrap(void) {
    return nc_mobile_active_provider() != (const NcMobileProviderV1 *)0;
}
