#include <jni.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern bool @@NATIVE_PREFIX@@_init(void);
extern bool @@NATIVE_PREFIX@@_add(const char *);
extern bool @@NATIVE_PREFIX@@_remove(const char *);
extern bool @@NATIVE_PREFIX@@_addPattern(const char *);
extern bool @@NATIVE_PREFIX@@_removePattern(const char *);
extern bool @@NATIVE_PREFIX@@_addWhitelist(const char *);
extern bool @@NATIVE_PREFIX@@_removeWhitelist(const char *);
extern bool @@NATIVE_PREFIX@@_setUrgencyWindow(int32_t);
extern bool @@NATIVE_PREFIX@@_setWhitelistMode(bool);
extern bool @@NATIVE_PREFIX@@_clearHistory(void);
extern bool @@NATIVE_PREFIX@@_clear(void);
extern bool @@NATIVE_PREFIX@@_shouldBlock(const char *);
extern bool @@NATIVE_PREFIX@@_shouldBlockAt(const char *, int32_t);
extern bool @@NATIVE_PREFIX@@_callSeen(bool);
extern char *@@NATIVE_PREFIX@@_list(void);
extern char *@@NATIVE_PREFIX@@_history(void);
extern char *@@NATIVE_PREFIX@@_status(void);
extern char *@@NATIVE_PREFIX@@_snapshot(void);
extern bool @@NATIVE_PREFIX@@_restore(const char *);
@@CONTACTS_EXTERN@@
@@BMUI1_EXTERN@@
extern bool nccTextFormatRuntime_releaseRaw(char *);

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

extern int ncc_rt_mobile_install_provider(const NcMobileProviderV1 *);

static pthread_mutex_t nc_core_lock = PTHREAD_MUTEX_INITIALIZER;
static char nc_incoming_number[256] = "";
static char nc_call_state[32] = "IDLE";

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

static jstring nc_string(JNIEnv *env, const char *value) {
    return (*env)->NewStringUTF(env, value ? value : "");
}

static jstring nc_dynamic_response(JNIEnv *env, const char *prefix, char *value) {
    size_t prefix_len = strlen(prefix);
    size_t value_len = value ? strlen(value) : 0;
    char *response = (char *)malloc(prefix_len + value_len + 1);
    jstring result;
    if (!response) {
        if (value) nccTextFormatRuntime_releaseRaw(value);
        return nc_string(env, "ERR|OUT_OF_MEMORY");
    }
    memcpy(response, prefix, prefix_len);
    if (value_len) memcpy(response + prefix_len, value, value_len);
    response[prefix_len + value_len] = 0;
    result = nc_string(env, response);
    free(response);
    if (value) nccTextFormatRuntime_releaseRaw(value);
    return result;
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

static jstring nc_command(JNIEnv *env, const char *request) {
    char action[64];
    const char *argument;
    char *dynamic = NULL;
    bool ok = false;
    split_command(request, action, sizeof(action), &argument);

    if (strcmp(action, "PING") == 0) return nc_string(env, "OK|PONG");
    if (strcmp(action, "AYUDA") == 0) {
        return nc_string(env, "OK|AGREGAR ELIMINAR BLOQUEAR_POR_PATRON DESBLOQUEAR_PATRON AGREGAR_WHITELIST ELIMINAR_WHITELIST AJUSTAR_TIEMPO MODO_WHITELIST@@CONTACTS_HELP@@@@BMUI1_HELP@@ HISTORIAL LIMPIAR_HISTORIAL LISTAR ESTADO LIMPIAR APAGAR_SERVIDOR");
    }
    if (strcmp(action, "AGREGAR") == 0) {
        ok = @@NATIVE_PREFIX@@_add(argument);
        return nc_string(env, ok ? "OK|ADDED" : "ERR|DUPLICATE_OR_EMPTY");
    }
    if (strcmp(action, "ELIMINAR") == 0) {
        ok = @@NATIVE_PREFIX@@_remove(argument);
        return nc_string(env, ok ? "OK|REMOVED" : "ERR|NOT_FOUND");
    }
    if (strcmp(action, "BLOQUEAR_POR_PATRON") == 0) {
        ok = @@NATIVE_PREFIX@@_addPattern(argument);
        return nc_string(env, ok ? "OK|PATTERN_ADDED" : "ERR|DUPLICATE_OR_EMPTY");
    }
    if (strcmp(action, "DESBLOQUEAR_PATRON") == 0) {
        ok = @@NATIVE_PREFIX@@_removePattern(argument);
        return nc_string(env, ok ? "OK|PATTERN_REMOVED" : "ERR|NOT_FOUND");
    }
    if (strcmp(action, "AGREGAR_WHITELIST") == 0) {
        ok = @@NATIVE_PREFIX@@_addWhitelist(argument);
        return nc_string(env, ok ? "OK|WHITELIST_ADDED" : "ERR|DUPLICATE_OR_EMPTY");
    }
    if (strcmp(action, "ELIMINAR_WHITELIST") == 0) {
        ok = @@NATIVE_PREFIX@@_removeWhitelist(argument);
        return nc_string(env, ok ? "OK|WHITELIST_REMOVED" : "ERR|NOT_FOUND");
    }
    if (strcmp(action, "AJUSTAR_TIEMPO") == 0) {
        int32_t seconds = 0;
        char response[64];
        if (!parse_positive_seconds(argument, &seconds)) {
            return nc_string(env, "ERR|WINDOW_RANGE|1..86400");
        }
        ok = @@NATIVE_PREFIX@@_setUrgencyWindow(seconds);
        if (!ok) return nc_string(env, "ERR|WINDOW_REJECTED");
        snprintf(response, sizeof(response), "OK|WINDOW|%d", seconds);
        return nc_string(env, response);
    }
    if (strcmp(action, "MODO_WHITELIST") == 0) {
        if (strcmp(argument, "ON") == 0 || strcmp(argument, "on") == 0) {
            @@NATIVE_PREFIX@@_setWhitelistMode(true);
            return nc_string(env, "OK|MODE|WHITELIST_URGENCY");
        }
        if (strcmp(argument, "OFF") == 0 || strcmp(argument, "off") == 0) {
            @@NATIVE_PREFIX@@_setWhitelistMode(false);
            return nc_string(env, "OK|MODE|BLOCKLIST");
        }
        return nc_string(env, "ERR|MODE_EXPECTED|ON_OR_OFF");
    }
@@CONTACTS_COMMAND@@
@@BMUI1_COMMAND@@
    if (strcmp(action, "LIMPIAR_HISTORIAL") == 0) {
        ok = @@NATIVE_PREFIX@@_clearHistory();
        return nc_string(env, ok ? "OK|HISTORY_CLEARED" : "ERR|CLEAR_FAILED");
    }
    if (strcmp(action, "LIMPIAR") == 0) {
        ok = @@NATIVE_PREFIX@@_clear();
        return nc_string(env, ok ? "OK|CLEARED" : "ERR|CLEAR_FAILED");
    }
    if (strcmp(action, "LISTAR") == 0) {
        dynamic = @@NATIVE_PREFIX@@_list();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "HISTORIAL") == 0) {
        dynamic = @@NATIVE_PREFIX@@_history();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "ESTADO") == 0) {
        dynamic = @@NATIVE_PREFIX@@_status();
        return nc_dynamic_response(env, "OK|", dynamic);
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
    bool core_ok = @@NATIVE_PREFIX@@_init();
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

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_shouldBlock(
        JNIEnv *env, jclass type, jstring number) {
    (void)type;
    if (!number) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, number, NULL);
    if (!utf8) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    snprintf(nc_incoming_number, sizeof(nc_incoming_number), "%s", utf8);
    snprintf(nc_call_state, sizeof(nc_call_state), "%s", "INCOMING");
    bool blocked = @@NATIVE_PREFIX@@_shouldBlock(utf8);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, number, utf8);
    return blocked ? JNI_TRUE : JNI_FALSE;
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
    bool blocked = @@NATIVE_PREFIX@@_shouldBlockAt(utf8, (int32_t)now_seconds);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, number, utf8);
    return blocked ? JNI_TRUE : JNI_FALSE;
}

@@CONTACTS_JNI@@
@@BMUI1_JNI@@

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_restore(
        JNIEnv *env, jclass type, jstring snapshot) {
    (void)type;
    if (!snapshot) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, snapshot, NULL);
    if (!utf8) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    bool restored = @@NATIVE_PREFIX@@_restore(utf8);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, snapshot, utf8);
    return restored ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jstring JNICALL Java_dev_nc_blockmaster_NativeBridge_snapshot(
        JNIEnv *env, jclass type) {
    (void)type;
    pthread_mutex_lock(&nc_core_lock);
    char *snapshot = @@NATIVE_PREFIX@@_snapshot();
    jstring result = nc_string(env, snapshot);
    if (snapshot && snapshot[0]) nccTextFormatRuntime_releaseRaw(snapshot);
    pthread_mutex_unlock(&nc_core_lock);
    return result;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_callSeen(
        JNIEnv *env, jclass type, jboolean blocked) {
    (void)env;
    (void)type;
    pthread_mutex_lock(&nc_core_lock);
    bool result = @@NATIVE_PREFIX@@_callSeen(blocked == JNI_TRUE);
    snprintf(nc_call_state, sizeof(nc_call_state), "%s", "IDLE");
    pthread_mutex_unlock(&nc_core_lock);
    return result ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jstring JNICALL Java_dev_nc_blockmaster_NativeBridge_abiReport(
        JNIEnv *env, jclass type) {
    (void)type;
    return nc_string(env, "nC/aarch64-v8a + NcMobileProviderV2 + JNI 1.6");
}

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    (void)vm;
    (void)reserved;
    return JNI_VERSION_1_6;
}
