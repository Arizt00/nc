#include <jni.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern bool @@NATIVE_PREFIX@@_cargarConfiguracion(void);
extern bool @@NATIVE_PREFIX@@_limpiarHistorialAntiguo(void);
extern bool @@NATIVE_PREFIX@@_registrarLlamadaEntrante(const char *);
extern bool @@NATIVE_PREFIX@@_verificarUrgenciaYPermitir(const char *);
extern bool @@NATIVE_PREFIX@@_procesarComandoPC(const char *);
extern bool @@NATIVE_PREFIX@@_guardarWhitelist(void);

extern int ncc_rt_mobile_install_provider(const void *);
extern void ncc_rt_network_response_clear(void);
extern const char *ncc_rt_network_response_last(void);

typedef struct NcMobileProviderV1 {
    uint64_t abi_version;
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
    int (*network_request)(const char *, int64_t *, const char **);
    int (*permission_request)(uint64_t);
} NcMobileProviderV1;

typedef struct NcMobileProviderV2 {
    NcMobileProviderV1 v1;
    int (*telephony_adb_connect)(int64_t);
    int (*network_is_wifi_connected)(void);
} NcMobileProviderV2;

typedef struct NcUrgencyCall {
    char *number;
    int32_t timestamp;
} NcUrgencyCall;

static pthread_mutex_t nc_core_lock = PTHREAD_MUTEX_INITIALIZER;
static char nc_incoming_number[256] = "";
static char nc_call_state[32] = "IDLE";
static char **nc_whitelist = NULL;
static size_t nc_whitelist_len = 0;
static size_t nc_whitelist_cap = 0;
static NcUrgencyCall *nc_history = NULL;
static size_t nc_history_len = 0;
static size_t nc_history_cap = 0;
static int32_t nc_urgency_seconds = 60;

static const char *provider_number(void) { return nc_incoming_number; }
static const char *provider_state(void) { return nc_call_state; }
static int provider_true(void) { return 1; }
static int provider_false(void) { return 0; }
static int provider_string_true(const char *value) { return value != NULL; }
static int provider_request(const char *url, int64_t *status, const char **body) {
    (void)url;
    if (status) *status = 501;
    if (body) *body = "{\"error\":\"android provider has no HTTP adapter\"}";
    return 1;
}
static int provider_permission(uint64_t mask) { return mask != 0; }
static int provider_adb(int64_t port) { return port > 0 ? 0 : 1; }

static const NcMobileProviderV2 provider = {
    .v1 = {
        .abi_version = 2,
        .telephony_get_incoming_number = provider_number,
        .telephony_get_state = provider_state,
        .telephony_is_available = provider_true,
        .telephony_dial = provider_string_true,
        .telephony_answer = provider_false,
        .telephony_hang_up = provider_true,
        .tts_connect = provider_string_true,
        .tts_speak = provider_string_true,
        .tts_stop = provider_true,
        .tts_is_speaking = provider_false,
        .tts_disconnect = provider_true,
        .network_request = provider_request,
        .permission_request = provider_permission,
    },
    .telephony_adb_connect = provider_adb,
    .network_is_wifi_connected = provider_true,
};

static char *nc_copy(const char *value) {
    size_t length = value ? strlen(value) : 0;
    char *copy = (char *)malloc(length + 1);
    if (!copy) return NULL;
    if (length) memcpy(copy, value, length);
    copy[length] = 0;
    return copy;
}

static jstring nc_string(JNIEnv *env, const char *value) {
    return (*env)->NewStringUTF(env, value ? value : "");
}

static const char *skip_space(const char *value) {
    while (*value == ' ' || *value == '\t' || *value == '\r' || *value == '\n') value++;
    return value;
}

static void split_command(const char *request, char *action, size_t action_cap,
                          const char **argument) {
    const char *begin = skip_space(request);
    const char *end = begin;
    size_t length;
    while (*end && *end != ' ' && *end != '\t' && *end != '\r' && *end != '\n') end++;
    length = (size_t)(end - begin);
    if (length >= action_cap) length = action_cap - 1;
    memcpy(action, begin, length);
    action[length] = 0;
    for (size_t i = 0; i < length; i++) {
        if (action[i] >= 'a' && action[i] <= 'z') action[i] = (char)(action[i] - 'a' + 'A');
    }
    *argument = skip_space(end);
}

static bool ensure_capacity(void **items, size_t *capacity, size_t needed,
                            size_t item_size) {
    if (*capacity >= needed) return true;
    size_t next = *capacity ? *capacity * 2 : 8;
    while (next < needed) next *= 2;
    void *grown = realloc(*items, next * item_size);
    if (!grown) return false;
    *items = grown;
    *capacity = next;
    return true;
}

static int whitelist_index(const char *number) {
    for (size_t i = 0; i < nc_whitelist_len; i++) {
        if (strcmp(nc_whitelist[i], number) == 0) return (int)i;
    }
    return -1;
}

static bool whitelist_add(const char *number) {
    if (!number || !*number || whitelist_index(number) >= 0) return false;
    if (!ensure_capacity((void **)&nc_whitelist, &nc_whitelist_cap,
                         nc_whitelist_len + 1, sizeof(*nc_whitelist))) return false;
    char *copy = nc_copy(number);
    if (!copy) return false;
    nc_whitelist[nc_whitelist_len++] = copy;
    return true;
}

static bool whitelist_remove(const char *number) {
    int index = whitelist_index(number);
    if (index < 0) return false;
    free(nc_whitelist[index]);
    memmove(&nc_whitelist[index], &nc_whitelist[index + 1],
            (nc_whitelist_len - (size_t)index - 1) * sizeof(*nc_whitelist));
    nc_whitelist_len--;
    return true;
}

static void history_expire(int32_t now) {
    size_t write = 0;
    for (size_t read = 0; read < nc_history_len; read++) {
        int64_t age = (int64_t)now - nc_history[read].timestamp;
        if (age >= 0 && age <= nc_urgency_seconds) {
            if (write != read) nc_history[write] = nc_history[read];
            write++;
        } else {
            free(nc_history[read].number);
        }
    }
    nc_history_len = write;
}

static int history_index(const char *number) {
    for (size_t i = 0; i < nc_history_len; i++) {
        if (strcmp(nc_history[i].number, number) == 0) return (int)i;
    }
    return -1;
}

static bool history_record(const char *number, int32_t now) {
    history_expire(now);
    int index = history_index(number);
    if (index >= 0) {
        nc_history[index].timestamp = now;
        return true;
    }
    if (!ensure_capacity((void **)&nc_history, &nc_history_cap,
                         nc_history_len + 1, sizeof(*nc_history))) return false;
    char *copy = nc_copy(number);
    if (!copy) return false;
    nc_history[nc_history_len++] = (NcUrgencyCall){copy, now};
    return false;
}

static bool parse_positive_seconds(const char *value, int32_t *seconds) {
    char *end = NULL;
    long parsed;
    if (!value || !*value) return false;
    parsed = strtol(value, &end, 10);
    end = (char *)skip_space(end);
    if (*end || parsed < 1L || parsed > 86400L) return false;
    *seconds = (int32_t)parsed;
    return true;
}

static char *status_text(void) {
    size_t capacity = 96 + nc_whitelist_len * 260;
    char *value = (char *)malloc(capacity);
    if (!value) return NULL;
    int used = snprintf(value, capacity, "WHITELIST|%zu|HISTORIAL|%zu|VENTANA|%d",
                        nc_whitelist_len, nc_history_len, nc_urgency_seconds);
    for (size_t i = 0; i < nc_whitelist_len && used > 0 && (size_t)used < capacity; i++) {
        used += snprintf(value + used, capacity - (size_t)used, "|%s", nc_whitelist[i]);
    }
    return value;
}

static char *history_text(void) {
    size_t capacity = 32 + nc_history_len * 272;
    char *value = (char *)malloc(capacity);
    if (!value) return NULL;
    size_t used = 0;
    for (size_t i = 0; i < nc_history_len; i++) {
        int written = snprintf(value + used, capacity - used, "%s%s@%d",
                               i ? "," : "", nc_history[i].number, nc_history[i].timestamp);
        if (written < 0) break;
        used += (size_t)written;
    }
    value[used] = 0;
    return value;
}

static jstring nc_owned_response(JNIEnv *env, const char *prefix, char *value) {
    size_t prefix_len = strlen(prefix);
    size_t value_len = value ? strlen(value) : 0;
    char *response = (char *)malloc(prefix_len + value_len + 1);
    if (!response) {
        free(value);
        return nc_string(env, "ERR|OUT_OF_MEMORY");
    }
    memcpy(response, prefix, prefix_len);
    if (value_len) memcpy(response + prefix_len, value, value_len);
    response[prefix_len + value_len] = 0;
    jstring result = nc_string(env, response);
    free(response);
    free(value);
    return result;
}

static jstring nc_command(JNIEnv *env, const char *request) {
    char action[64];
    const char *argument;
    split_command(request, action, sizeof(action), &argument);

    ncc_rt_network_response_clear();
    (void)@@NATIVE_PREFIX@@_procesarComandoPC(request);

    if (strcmp(action, "PING") == 0) return nc_string(env, "OK|PONG");
    if (strcmp(action, "AYUDA") == 0) {
        return nc_string(env, "OK|AGREGAR_WHITELIST ELIMINAR_WHITELIST AJUSTAR_TIEMPO HISTORIAL LISTAR ESTADO LIMPIAR_HISTORIAL APAGAR_SERVIDOR");
    }
    if (strcmp(action, "AGREGAR_WHITELIST") == 0) {
        bool ok = whitelist_add(argument);
        (void)@@NATIVE_PREFIX@@_guardarWhitelist();
        return nc_string(env, ok ? "OK|WHITELIST_ADDED" : "ERR|DUPLICATE_OR_EMPTY");
    }
    if (strcmp(action, "ELIMINAR_WHITELIST") == 0) {
        bool ok = whitelist_remove(argument);
        (void)@@NATIVE_PREFIX@@_guardarWhitelist();
        return nc_string(env, ok ? "OK|WHITELIST_REMOVED" : "ERR|NOT_FOUND");
    }
    if (strcmp(action, "AJUSTAR_TIEMPO") == 0) {
        int32_t seconds = 0;
        char response[64];
        if (!parse_positive_seconds(argument, &seconds)) {
            return nc_string(env, "ERR|WINDOW_RANGE|1..86400");
        }
        nc_urgency_seconds = seconds;
        snprintf(response, sizeof(response), "OK|WINDOW|%d", seconds);
        return nc_string(env, response);
    }
    if (strcmp(action, "HISTORIAL") == 0) {
        const char *native = ncc_rt_network_response_last();
        if (native && *native) return nc_string(env, native);
        return nc_owned_response(env, "OK|", history_text());
    }
    if (strcmp(action, "LISTAR") == 0 || strcmp(action, "ESTADO") == 0) {
        const char *native = ncc_rt_network_response_last();
        if (native && *native) return nc_string(env, native);
        return nc_owned_response(env, "OK|", status_text());
    }
    if (strcmp(action, "LIMPIAR_HISTORIAL") == 0) {
        for (size_t i = 0; i < nc_history_len; i++) free(nc_history[i].number);
        nc_history_len = 0;
        return nc_string(env, "OK|HISTORY_CLEARED");
    }
    if (strcmp(action, "APAGAR_SERVIDOR") == 0) return nc_string(env, "OK|SERVER_STOPPING");
    return nc_string(env, "ERR|UNKNOWN_COMMAND");
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_init(
        JNIEnv *env, jclass type) {
    (void)env;
    (void)type;
    pthread_mutex_lock(&nc_core_lock);
    int provider_ok = ncc_rt_mobile_install_provider(&provider.v1);
    bool core_ok = @@NATIVE_PREFIX@@_cargarConfiguracion();
    (void)@@NATIVE_PREFIX@@_limpiarHistorialAntiguo();
    pthread_mutex_unlock(&nc_core_lock);
    return (jboolean)(provider_ok && core_ok);
}

JNIEXPORT jstring JNICALL Java_dev_nc_blockmaster_NativeBridge_command(
        JNIEnv *env, jclass type, jstring request) {
    (void)type;
    if (!request) return nc_string(env, "ERR|EMPTY_COMMAND");
    const char *utf8 = (*env)->GetStringUTFChars(env, request, NULL);
    if (!utf8) return NULL;
    pthread_mutex_lock(&nc_core_lock);
    jstring response = nc_command(env, utf8);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, request, utf8);
    return response;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_shouldBlockAt(
        JNIEnv *env, jclass type, jstring number, jint now_seconds) {
    (void)type;
    if (!number) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, number, NULL);
    if (!utf8) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    snprintf(nc_incoming_number, sizeof(nc_incoming_number), "%s", utf8);
    snprintf(nc_call_state, sizeof(nc_call_state), "%s", "INCOMING");
    bool native_allowed = @@NATIVE_PREFIX@@_verificarUrgenciaYPermitir(utf8);
    bool repeated = history_record(utf8, (int32_t)now_seconds);
    bool source_repeated = @@NATIVE_PREFIX@@_registrarLlamadaEntrante(utf8);
    bool allowed = whitelist_index(utf8) >= 0 || repeated || source_repeated || native_allowed;
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, number, utf8);
    return allowed ? JNI_FALSE : JNI_TRUE;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_shouldBlock(
        JNIEnv *env, jclass type, jstring number) {
    return Java_dev_nc_blockmaster_NativeBridge_shouldBlockAt(env, type, number, 0);
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_callSeen(
        JNIEnv *env, jclass type, jboolean blocked) {
    (void)env;
    (void)type;
    (void)blocked;
    pthread_mutex_lock(&nc_core_lock);
    snprintf(nc_call_state, sizeof(nc_call_state), "%s", "IDLE");
    pthread_mutex_unlock(&nc_core_lock);
    return JNI_TRUE;
}

JNIEXPORT jstring JNICALL Java_dev_nc_blockmaster_NativeBridge_snapshot(
        JNIEnv *env, jclass type) {
    (void)type;
    pthread_mutex_lock(&nc_core_lock);
    size_t capacity = 48 + nc_whitelist_len * 260;
    char *snapshot = (char *)malloc(capacity);
    if (!snapshot) {
        pthread_mutex_unlock(&nc_core_lock);
        return nc_string(env, "");
    }
    int used = snprintf(snapshot, capacity, "U1|%d", nc_urgency_seconds);
    for (size_t i = 0; i < nc_whitelist_len && used > 0 && (size_t)used < capacity; i++) {
        used += snprintf(snapshot + used, capacity - (size_t)used, "|%s", nc_whitelist[i]);
    }
    jstring result = nc_string(env, snapshot);
    free(snapshot);
    pthread_mutex_unlock(&nc_core_lock);
    return result;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_restore(
        JNIEnv *env, jclass type, jstring snapshot) {
    (void)type;
    if (!snapshot) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, snapshot, NULL);
    if (!utf8) return JNI_FALSE;
    char *copy = nc_copy(utf8);
    (*env)->ReleaseStringUTFChars(env, snapshot, utf8);
    if (!copy) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    char *save = NULL;
    char *part = strtok_r(copy, "|", &save);
    bool ok = part && strcmp(part, "U1") == 0;
    part = ok ? strtok_r(NULL, "|", &save) : NULL;
    int32_t seconds = 0;
    ok = part && parse_positive_seconds(part, &seconds);
    if (ok) {
        for (size_t i = 0; i < nc_whitelist_len; i++) free(nc_whitelist[i]);
        nc_whitelist_len = 0;
        nc_urgency_seconds = seconds;
        while ((part = strtok_r(NULL, "|", &save)) != NULL) {
            if (!whitelist_add(part)) { ok = false; break; }
        }
    }
    pthread_mutex_unlock(&nc_core_lock);
    free(copy);
    return ok ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jstring JNICALL Java_dev_nc_blockmaster_NativeBridge_abiReport(
        JNIEnv *env, jclass type) {
    (void)type;
    return nc_string(env, "nC/aarch64-v8a + urgency JNI + NcMobileProviderV2");
}

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    (void)vm;
    (void)reserved;
    return JNI_VERSION_1_6;
}
