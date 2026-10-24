use super::super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DEFAULT_MIN_API: u32 = 29;
const DEFAULT_COMPILE_API: &str = "36.1";
const DEFAULT_NDK_VERSION: &str = "29.0.14206865";
const DEFAULT_BUILD_TOOLS: &str = "36.1.0";
const DEFAULT_PACKAGE_ID: &str = "com.aristote.uncall";
const PROVIDER_PACKAGE_ID: &str = "dev.nc.blockmaster";
const UNCALL_VERSION_CODE: &str = "26080901";
const UNCALL_VERSION_NAME: &str = "1.6.0";

const ANDROID_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/AndroidManifest.xml"
));
const ANDROID_STORE_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/AndroidManifest.store.xml"
));
const JNI_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/jni/nc_android_jni.c"
));
const URGENCY_JNI_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/jni/nc_android_urgency_jni.c"
));
const BUILD_CONFIG_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/BuildConfig.java.in"
));
const CONTROLLER_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/blockmaster_controller.n"
));
const PROTOCOL_TEST_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/protocol_conformance_test.n"
));
const UNCHANGED_PYTHON_LAUNCHER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/android/blockmaster/run_unchanged_python.ps1"
));

const MOBILE_TOOL_PROJECT_SOURCES: &[(&str, &str)] = &[
    ("build/main.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/android/blockmaster/build_mobile_tools.n"))),
    ("build/src/mobile43_bundle.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_bundle.n"))),
    ("linker/main.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/android/blockmaster/mobile_linker.n"))),
    ("controller/src/mobile43_codec.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_codec.n"))),
    ("controller/src/mobile43_transport.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_transport.n"))),
    ("controller/src/mobile43_client.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_client.n"))),
    ("controller/src/mobile43_cli.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_cli.n"))),
    ("protocol/src/mobile43_codec.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_codec.n"))),
    ("protocol/src/mobile43_transport.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_transport.n"))),
    ("protocol/src/mobile43_client.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_client.n"))),
    ("protocol/src/mobile43_peer.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_peer.n"))),
    ("protocol/src/mobile43_conformance.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/mobile43_conformance.n"))),
    ("build/src/tool_cli.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/tool_cli.n"))),
    ("build/src/release_io.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/release_io.n"))),
    ("build/src/process_capture.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/process_capture.n"))),
    ("linker/src/tool_cli.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/tool_cli.n"))),
    ("linker/src/release_io.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/release_io.n"))),
    ("linker/src/process_capture.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/process_capture.n"))),
    ("protocol/src/tool_cli.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/tool_cli.n"))),
    ("protocol/src/release_io.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/release_io.n"))),
    ("protocol/src/process_capture.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/process_capture.n"))),
    ("build/src/cert_json.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/cert_json.n"))),
    ("protocol/src/cert_json.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/verification/cert_json.n"))),
    ("build/src/sha256.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/sha256.n"))),
    ("controller/src/sha256.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/sha256.n"))),
    ("protocol/src/sha256.n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/ncc/nc/src/sha256.n"))),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AndroidDistribution {
    Development,
    StoreRelease,
}

impl AndroidDistribution {
    fn parse(value: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            "development" | "dev" => Ok(Self::Development),
            "store-release" | "store" | "play" => Ok(Self::StoreRelease),
            _ => Err(format!(
                "unknown Android distribution '{value}'; expected development or store-release"
            )
            .into()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::StoreRelease => "store-release",
        }
    }

    fn remote_control_enabled(self) -> bool {
        matches!(self, Self::Development)
    }

    fn legacy_usb_control_enabled(self) -> bool {
        matches!(self, Self::Development)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AndroidNativeProfile {
    Blocklist,
    BlocklistContacts,
    BlocklistContactsBmui1,
    Urgency,
}

impl AndroidNativeProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::Blocklist => "blocklist",
            Self::BlocklistContacts => "blocklist-contacts",
            Self::BlocklistContactsBmui1 => "blocklist-contacts-bmui1",
            Self::Urgency => "whitelist-urgency",
        }
    }

    fn jni_source(self) -> &'static str {
        match self {
            Self::Blocklist | Self::BlocklistContacts | Self::BlocklistContactsBmui1 => JNI_SOURCE,
            Self::Urgency => URGENCY_JNI_SOURCE,
        }
    }

    fn supports_contacts(self) -> bool {
        matches!(self, Self::BlocklistContacts | Self::BlocklistContactsBmui1)
    }

    fn supports_bmui1(self) -> bool {
        matches!(self, Self::BlocklistContactsBmui1)
    }

    fn render_jni(self, prefix: &str, package_id: &str) -> String {
        let (contacts_extern, contacts_command, contacts_jni, contacts_help) = if self
            .supports_contacts()
        {
            (
                r#"extern bool @@NATIVE_PREFIX@@_setContactsOnlyMode(bool);
extern bool @@NATIVE_PREFIX@@_shouldBlockContactAt(const char *, int32_t, bool);
extern char *@@NATIVE_PREFIX@@_lastDecision(void);"#,
                r#"    if (strcmp(action, "MODO_CONTACTOS") == 0) {
        if (strcmp(argument, "ON") == 0 || strcmp(argument, "on") == 0) {
            @@NATIVE_PREFIX@@_setContactsOnlyMode(true);
            return nc_string(env, "OK|MODE|CONTACTS_ONLY");
        }
        if (strcmp(argument, "OFF") == 0 || strcmp(argument, "off") == 0) {
            @@NATIVE_PREFIX@@_setContactsOnlyMode(false);
            return nc_string(env, "OK|MODE|CONTACTS_AND_URGENCY");
        }
        return nc_string(env, "ERR|MODE_EXPECTED|ON_OR_OFF");
    }
    if (strcmp(action, "ULTIMA_DECISION") == 0) {
        dynamic = @@NATIVE_PREFIX@@_lastDecision();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
"#,
                r#"JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_shouldBlockContactAt(
        JNIEnv *env, jclass type, jstring number, jint now_seconds, jboolean is_contact) {
    (void)type;
    if (!number) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, number, NULL);
    if (!utf8) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    snprintf(nc_incoming_number, sizeof(nc_incoming_number), "%s", utf8);
    snprintf(nc_call_state, sizeof(nc_call_state), "%s", "INCOMING");
    bool blocked = @@NATIVE_PREFIX@@_shouldBlockContactAt(
            utf8, (int32_t)now_seconds, is_contact == JNI_TRUE);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, number, utf8);
    return blocked ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jstring JNICALL Java_dev_nc_blockmaster_NativeBridge_lastDecision(
        JNIEnv *env, jclass type) {
    (void)type;
    pthread_mutex_lock(&nc_core_lock);
    char *decision = @@NATIVE_PREFIX@@_lastDecision();
    jstring result = nc_dynamic_response(env, "", decision);
    pthread_mutex_unlock(&nc_core_lock);
    return result;
}
"#,
                " MODO_CONTACTOS ULTIMA_DECISION",
            )
        } else {
            ("", "", "", "")
        };
        let (bmui1_extern, bmui1_command, bmui1_jni, bmui1_help) = if self.supports_bmui1() {
            (
                r#"extern bool @@NATIVE_PREFIX@@_addAllowPattern(const char *);
extern bool @@NATIVE_PREFIX@@_removeAllowPattern(const char *);
extern bool @@NATIVE_PREFIX@@_setProtectionActive(bool);
extern bool @@NATIVE_PREFIX@@_setBlockHidden(bool);
extern bool @@NATIVE_PREFIX@@_setAprendizajeAdaptativo(bool);
extern bool @@NATIVE_PREFIX@@_reiniciarAprendizajeAdaptativo(void);
extern bool @@NATIVE_PREFIX@@_registrarFeedbackAdaptativo(
        const char *, bool, bool, bool, int32_t);
extern char *@@NATIVE_PREFIX@@_activity(void);
extern char *@@NATIVE_PREFIX@@_estadoAprendizajeAdaptativo(void);
extern bool @@NATIVE_PREFIX@@_beginSignedSpamBase(
        const char *, const char *, int32_t, int32_t, int32_t);
extern bool @@NATIVE_PREFIX@@_addSignedSpamIndicator(
        const char *, const char *, int32_t, const char *, const char *);
extern bool @@NATIVE_PREFIX@@_commitSignedSpamBase(const char *);
extern bool @@NATIVE_PREFIX@@_invalidateSignedSpamBase(const char *);
extern char *@@NATIVE_PREFIX@@_signedSpamStatus(void);
extern char *@@NATIVE_PREFIX@@_signedSpamVerdict(const char *);
extern char *@@NATIVE_PREFIX@@_assistantLocal(const char *);
extern char *@@NATIVE_PREFIX@@_uiContract(void);
extern char *@@NATIVE_PREFIX@@_versionInfo(void);
extern char *@@NATIVE_PREFIX@@_updatePolicy(void);"#,
                r#"    if (strcmp(action, "PERMITIR_POR_PATRON") == 0) {
        ok = @@NATIVE_PREFIX@@_addAllowPattern(argument);
        return nc_string(env, ok ? "OK|ALLOW_PATTERN_ADDED" : "ERR|DUPLICATE_OR_EMPTY");
    }
    if (strcmp(action, "QUITAR_PERMISO_PATRON") == 0) {
        ok = @@NATIVE_PREFIX@@_removeAllowPattern(argument);
        return nc_string(env, ok ? "OK|ALLOW_PATTERN_REMOVED" : "ERR|NOT_FOUND");
    }
    if (strcmp(action, "MODO_PROTECCION") == 0) {
        if (strcmp(argument, "ON") == 0 || strcmp(argument, "on") == 0) {
            @@NATIVE_PREFIX@@_setProtectionActive(true);
            return nc_string(env, "OK|PROTECTION|ACTIVE");
        }
        if (strcmp(argument, "OFF") == 0 || strcmp(argument, "off") == 0) {
            @@NATIVE_PREFIX@@_setProtectionActive(false);
            return nc_string(env, "OK|PROTECTION|PAUSED");
        }
        return nc_string(env, "ERR|MODE_EXPECTED|ON_OR_OFF");
    }
    if (strcmp(action, "BLOQUEAR_OCULTOS") == 0) {
        if (strcmp(argument, "ON") == 0 || strcmp(argument, "on") == 0) {
            @@NATIVE_PREFIX@@_setBlockHidden(true);
            return nc_string(env, "OK|BLOCK_HIDDEN|ON");
        }
        if (strcmp(argument, "OFF") == 0 || strcmp(argument, "off") == 0) {
            @@NATIVE_PREFIX@@_setBlockHidden(false);
            return nc_string(env, "OK|BLOCK_HIDDEN|OFF");
        }
        return nc_string(env, "ERR|MODE_EXPECTED|ON_OR_OFF");
    }
    if (strcmp(action, "ACTIVIDAD") == 0) {
        dynamic = @@NATIVE_PREFIX@@_activity();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "APRENDIZAJE_ADAPTATIVO") == 0) {
        if (strcmp(argument, "ON") == 0 || strcmp(argument, "on") == 0) {
            @@NATIVE_PREFIX@@_setAprendizajeAdaptativo(true);
            return nc_string(env, "OK|ADAPTIVE_LEARNING|ON|MODE|SHADOW");
        }
        if (strcmp(argument, "OFF") == 0 || strcmp(argument, "off") == 0) {
            @@NATIVE_PREFIX@@_setAprendizajeAdaptativo(false);
            return nc_string(env, "OK|ADAPTIVE_LEARNING|OFF");
        }
        return nc_string(env, "ERR|MODE_EXPECTED|ON_OR_OFF");
    }
    if (strcmp(action, "IA_ESTADO") == 0) {
        dynamic = @@NATIVE_PREFIX@@_estadoAprendizajeAdaptativo();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "IA_REINICIAR") == 0) {
        @@NATIVE_PREFIX@@_reiniciarAprendizajeAdaptativo();
        return nc_string(env, "OK|ADAPTIVE_LEARNING_RESET");
    }
    if (strcmp(action, "SPAM_ESTADO") == 0) {
        dynamic = @@NATIVE_PREFIX@@_signedSpamStatus();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "SPAM_CONSULTAR") == 0) {
        dynamic = @@NATIVE_PREFIX@@_signedSpamVerdict(argument);
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "ASISTENTE") == 0) {
        dynamic = @@NATIVE_PREFIX@@_assistantLocal(argument);
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "IA_FEEDBACK") == 0) {
        char label[8] = {0};
        char number[256] = {0};
        int is_contact = 0;
        int repeated = 0;
        int now_seconds = 0;
        bool expected_block;
        if (sscanf(argument, "%7[^|]|%d|%d|%d|%255s", label, &is_contact,
                &repeated, &now_seconds, number) != 5 ||
                (is_contact != 0 && is_contact != 1) ||
                (repeated != 0 && repeated != 1) || now_seconds < 1) {
            return nc_string(env, "ERR|AI_FEEDBACK_FORMAT");
        }
        if (strcmp(label, "BLOCK") == 0) expected_block = true;
        else if (strcmp(label, "ALLOW") == 0) expected_block = false;
        else return nc_string(env, "ERR|AI_FEEDBACK_LABEL");
        if (strcmp(number, "-") == 0) number[0] = 0;
        ok = @@NATIVE_PREFIX@@_registrarFeedbackAdaptativo(number,
                is_contact != 0, repeated != 0, expected_block, now_seconds);
        return nc_string(env, ok ? "OK|AI_FEEDBACK_RECORDED"
                : "ERR|ADAPTIVE_LEARNING_DISABLED");
    }
    if (strcmp(action, "CONTRATO_UI") == 0) {
        dynamic = @@NATIVE_PREFIX@@_uiContract();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "VERSION") == 0) {
        dynamic = @@NATIVE_PREFIX@@_versionInfo();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
    if (strcmp(action, "POLITICA_ACTUALIZACION") == 0) {
        dynamic = @@NATIVE_PREFIX@@_updatePolicy();
        return nc_dynamic_response(env, "OK|", dynamic);
    }
"#,
                r#"JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_beginSignedSpamBase(
        JNIEnv *env, jclass type, jstring version, jstring key_id, jint issued,
        jint expires, jint count) {
    (void)type;
    if (!version || !key_id) return JNI_FALSE;
    const char *version_utf8 = (*env)->GetStringUTFChars(env, version, NULL);
    if (!version_utf8) return JNI_FALSE;
    const char *key_utf8 = (*env)->GetStringUTFChars(env, key_id, NULL);
    if (!key_utf8) {
        (*env)->ReleaseStringUTFChars(env, version, version_utf8);
        return JNI_FALSE;
    }
    pthread_mutex_lock(&nc_core_lock);
    bool result = @@NATIVE_PREFIX@@_beginSignedSpamBase(version_utf8, key_utf8,
            (int32_t)issued, (int32_t)expires, (int32_t)count);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, key_id, key_utf8);
    (*env)->ReleaseStringUTFChars(env, version, version_utf8);
    return result ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_addSignedSpamIndicator(
        JNIEnv *env, jclass type, jstring indicator_type, jstring value, jint risk,
        jstring label, jstring action) {
    (void)type;
    if (!indicator_type || !value || !label || !action) return JNI_FALSE;
    const char *type_utf8 = (*env)->GetStringUTFChars(env, indicator_type, NULL);
    if (!type_utf8) return JNI_FALSE;
    const char *value_utf8 = (*env)->GetStringUTFChars(env, value, NULL);
    if (!value_utf8) {
        (*env)->ReleaseStringUTFChars(env, indicator_type, type_utf8);
        return JNI_FALSE;
    }
    const char *label_utf8 = (*env)->GetStringUTFChars(env, label, NULL);
    if (!label_utf8) {
        (*env)->ReleaseStringUTFChars(env, value, value_utf8);
        (*env)->ReleaseStringUTFChars(env, indicator_type, type_utf8);
        return JNI_FALSE;
    }
    const char *action_utf8 = (*env)->GetStringUTFChars(env, action, NULL);
    if (!action_utf8) {
        (*env)->ReleaseStringUTFChars(env, label, label_utf8);
        (*env)->ReleaseStringUTFChars(env, value, value_utf8);
        (*env)->ReleaseStringUTFChars(env, indicator_type, type_utf8);
        return JNI_FALSE;
    }
    pthread_mutex_lock(&nc_core_lock);
    bool result = @@NATIVE_PREFIX@@_addSignedSpamIndicator(type_utf8, value_utf8,
            (int32_t)risk, label_utf8, action_utf8);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, action, action_utf8);
    (*env)->ReleaseStringUTFChars(env, label, label_utf8);
    (*env)->ReleaseStringUTFChars(env, value, value_utf8);
    (*env)->ReleaseStringUTFChars(env, indicator_type, type_utf8);
    return result ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_commitSignedSpamBase(
        JNIEnv *env, jclass type, jstring digest) {
    (void)type;
    if (!digest) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, digest, NULL);
    if (!utf8) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    bool result = @@NATIVE_PREFIX@@_commitSignedSpamBase(utf8);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, digest, utf8);
    return result ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jboolean JNICALL Java_dev_nc_blockmaster_NativeBridge_invalidateSignedSpamBase(
        JNIEnv *env, jclass type, jstring reason) {
    (void)type;
    if (!reason) return JNI_FALSE;
    const char *utf8 = (*env)->GetStringUTFChars(env, reason, NULL);
    if (!utf8) return JNI_FALSE;
    pthread_mutex_lock(&nc_core_lock);
    bool result = @@NATIVE_PREFIX@@_invalidateSignedSpamBase(utf8);
    pthread_mutex_unlock(&nc_core_lock);
    (*env)->ReleaseStringUTFChars(env, reason, utf8);
    return result ? JNI_TRUE : JNI_FALSE;
}
"#,
                " PERMITIR_POR_PATRON QUITAR_PERMISO_PATRON MODO_PROTECCION BLOQUEAR_OCULTOS ACTIVIDAD APRENDIZAJE_ADAPTATIVO IA_ESTADO IA_REINICIAR IA_FEEDBACK SPAM_ESTADO SPAM_CONSULTAR ASISTENTE CONTRATO_UI VERSION POLITICA_ACTUALIZACION",
            )
        } else {
            ("", "", "", "")
        };
        self.jni_source()
            .replace("@@CONTACTS_EXTERN@@", contacts_extern)
            .replace("@@CONTACTS_COMMAND@@", contacts_command)
            .replace("@@CONTACTS_JNI@@", contacts_jni)
            .replace("@@CONTACTS_HELP@@", contacts_help)
            .replace("@@BMUI1_EXTERN@@", bmui1_extern)
            .replace("@@BMUI1_COMMAND@@", bmui1_command)
            .replace("@@BMUI1_JNI@@", bmui1_jni)
            .replace("@@BMUI1_HELP@@", bmui1_help)
            .replace("@@NATIVE_PREFIX@@", prefix)
            .replace(
                "Java_dev_nc_blockmaster_",
                &format!("Java_{}_", jni_package_prefix(package_id)),
            )
    }
}

const JAVA_SOURCES: &[(&str, &str)] = &[
    (
        "ActivityStore.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/ActivityStore.java"
        )),
    ),
    (
        "BlocklistStore.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/BlocklistStore.java"
        )),
    ),
    (
        "BlockMasterService.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/BlockMasterService.java"
        )),
    ),
    (
        "ContactLookup.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/ContactLookup.java"
        )),
    ),
    (
        "FrameCodec.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/FrameCodec.java"
        )),
    ),
    (
        "MainActivity.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/MainActivity.java"
        )),
    ),
    (
        "NativeBridge.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/NativeBridge.java"
        )),
    ),
    (
        "NuinuiWebSearch.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/NuinuiWebSearch.java"
        )),
    ),
    (
        "NcCallScreeningService.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/NcCallScreeningService.java"
        )),
    ),
    (
        "ProtocolSecurity.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/ProtocolSecurity.java"
        )),
    ),
    (
        "ProtocolServer.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/ProtocolServer.java"
        )),
    ),
    (
        "RuleProfileStore.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/RuleProfileStore.java"
        )),
    ),
    (
        "SecurityVault.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/SecurityVault.java"
        )),
    ),
    (
        "SignedSpamDatabase.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/SignedSpamDatabase.java"
        )),
    ),
    (
        "Translations.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/src/dev/nc/blockmaster/Translations.java"
        )),
    ),
];

const RESOURCE_FILES: &[(&str, &str)] = &[
    (
        "values/strings.xml",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/values/strings.xml"
        )),
    ),
    (
        "values/styles.xml",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/values/styles.xml"
        )),
    ),
    (
        "xml/network_security_config.xml",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/xml/network_security_config.xml"
        )),
    ),
];

const BINARY_RESOURCE_FILES: &[(&str, &[u8])] = &[
    (
        "drawable/uncall_icon.png",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/drawable/uncall_icon.png"
        )),
    ),
    (
        "drawable/nav_home.png",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/drawable/nav_home.png"
        )),
    ),
    (
        "drawable/nav_activity.png",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/drawable/nav_activity.png"
        )),
    ),
    (
        "drawable/nav_rules.png",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/android/blockmaster/res/drawable/nav_rules.png"
        )),
    ),
];

#[derive(Debug, Clone)]
struct AndroidToolchain {
    sdk: PathBuf,
    ndk: PathBuf,
    android_jar: PathBuf,
    clang: PathBuf,
    llvm_readelf: PathBuf,
    llvm_nm: PathBuf,
    aapt2: PathBuf,
    aapt: PathBuf,
    d8: PathBuf,
    zipalign: PathBuf,
    apksigner: PathBuf,
    adb: PathBuf,
    javac: PathBuf,
    java: PathBuf,
    jar: PathBuf,
    jarsigner: PathBuf,
    keytool: PathBuf,
}

#[derive(Debug, Clone)]
struct AndroidBuildOptions {
    source: PathBuf,
    out_dir: PathBuf,
    sdk: Option<PathBuf>,
    ndk: Option<PathBuf>,
    min_api: u32,
    compile_api: String,
    build_tools: String,
    native_prefix: Option<String>,
    package_id: String,
    distribution: AndroidDistribution,
    trusted_signing_certs: Option<String>,
    bundletool: Option<PathBuf>,
    keystore: Option<PathBuf>,
    key_alias: String,
    store_password: String,
    key_password: String,
    install: bool,
    json: bool,
}

pub(crate) fn run_android_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty()
        || args
            .iter()
            .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        println!("{}", android_help());
        return Ok(());
    }
    match args[0].as_str() {
        "doctor" => android_doctor(&args[1..]),
        "build" => android_build(&args[1..]),
        "install" => android_install(&args[1..]),
        other => Err(format!("unknown android command '{other}'\n{}", android_help()).into()),
    }
}

fn android_help() -> &'static str {
    "Usage: ncc android <doctor|build|install> [args]\n\
     ncc android doctor [--sdk <path>] [--ndk <path>] [--json]\n\
     ncc android build <file.n> [--out-dir <dir>] [--package-id <reverse.dns.id>] [--distribution <development|store-release>] [--trusted-signing-certs <sha256[;sha256...]>] [--bundletool <bundletool.jar>] [--api <29>] [--compile-api <36.1>] [--build-tools <36.1.0>] [--native-prefix <container>] [--keystore <file>] [--key-alias <alias>] [--ks-pass <password>] [--key-pass <password>] [--install] [--json]\n\
     ncc android install <signed.apk> [--package-id <reverse.dns.id>] [--sdk <path>] [--json]\n\
     Development builds an installable APK. Store-release also requires bundletool and emits a signed, validated AAB with network control removed. For store-release, provide an existing keystore with NCC_ANDROID_KEYSTORE, NCC_ANDROID_KEYSTORE_PASSWORD and NCC_ANDROID_KEY_PASSWORD; command-line passwords are rejected."
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

fn validate_package_id(package_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let segments = package_id.split('.').collect::<Vec<_>>();
    if segments.len() < 3 {
        return Err("Android package ID must contain at least three reverse-DNS segments".into());
    }
    for segment in segments {
        let mut chars = segment.chars();
        if !matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
            || !chars.all(|value| value.is_ascii_lowercase() || value.is_ascii_digit())
        {
            return Err(format!(
                "invalid Android package ID '{package_id}'; segments must start with a lowercase ASCII letter and contain only lowercase letters or digits"
            )
            .into());
        }
    }
    Ok(())
}

fn jni_package_prefix(package_id: &str) -> String {
    package_id.replace('.', "_")
}

fn package_source_dir(root: &Path, package_id: &str) -> PathBuf {
    package_id
        .split('.')
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

fn activity_name(package_id: &str) -> String {
    format!("{package_id}.MainActivity")
}

fn positional(args: &[String]) -> Option<String> {
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--sdk"
                | "--ndk"
                | "--out-dir"
                | "--api"
                | "--compile-api"
                | "--build-tools"
                | "--native-prefix"
                | "--package-id"
                | "--distribution"
                | "--trusted-signing-certs"
                | "--bundletool"
                | "--keystore"
                | "--key-alias"
                | "--ks-pass"
                | "--key-pass"
        ) {
            skip = true;
        } else if !arg.starts_with('-') {
            return Some(arg.clone());
        }
    }
    None
}

fn android_doctor(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sdk = option_value(args, "--sdk").map(PathBuf::from);
    let ndk = option_value(args, "--ndk").map(PathBuf::from);
    let tools = discover_toolchain(
        sdk.as_deref(),
        ndk.as_deref(),
        DEFAULT_COMPILE_API,
        DEFAULT_BUILD_TOOLS,
    )?;
    let payload = toolchain_report(&tools);
    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "OK: Android SDK={} NDK={} clang={} aapt2={} d8={} apksigner={}",
            tools.sdk.display(),
            tools.ndk.display(),
            tools.clang.display(),
            tools.aapt2.display(),
            tools.d8.display(),
            tools.apksigner.display()
        );
    }
    Ok(())
}

fn parse_build_options(args: &[String]) -> Result<AndroidBuildOptions, Box<dyn std::error::Error>> {
    let source = positional(args).ok_or("android build requires a .n source file")?;
    let source = fs::canonicalize(PathBuf::from(source))?;
    if source.extension().and_then(|value| value.to_str()) != Some("n") {
        return Err("android build accepts a normal high-level .n source".into());
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("android");
    let out_dir = option_value(args, "--out-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            source
                .parent()
                .unwrap_or(Path::new("."))
                .join(".ncc")
                .join("android")
                .join(stem)
        });
    let out_dir = absolute_path(out_dir)?;
    let min_api = option_value(args, "--api")
        .unwrap_or_else(|| DEFAULT_MIN_API.to_string())
        .parse()?;
    if min_api < 29 {
        return Err("CallScreeningService build requires --api 29 or newer".into());
    }
    let distribution = AndroidDistribution::parse(
        &option_value(args, "--distribution").unwrap_or_else(|| "development".into()),
    )?;
    let package_id =
        option_value(args, "--package-id").unwrap_or_else(|| DEFAULT_PACKAGE_ID.to_string());
    validate_package_id(&package_id)?;
    if distribution == AndroidDistribution::StoreRelease && has_flag(args, "--install") {
        return Err("store-release emits a publishing AAB and cannot be installed directly; build development for device tests".into());
    }
    let keystore = option_value(args, "--keystore")
        .map(PathBuf::from)
        .or_else(|| env::var_os("NCC_ANDROID_KEYSTORE").map(PathBuf::from))
        .map(absolute_path)
        .transpose()?;
    if distribution == AndroidDistribution::StoreRelease {
        if has_flag(args, "--ks-pass") || has_flag(args, "--key-pass") {
            return Err("store-release rejects signing passwords on the command line; use NCC_ANDROID_KEYSTORE_PASSWORD and NCC_ANDROID_KEY_PASSWORD".into());
        }
        let signing_store = keystore.as_ref().ok_or(
            "store-release requires an existing signing identity through --keystore or NCC_ANDROID_KEYSTORE",
        )?;
        if !signing_store.is_file() {
            return Err(format!(
                "store-release signing identity was not found at {}",
                signing_store.display()
            )
            .into());
        }
    }
    let (store_password, key_password) = resolve_signing_passwords(
        distribution,
        option_value(args, "--ks-pass"),
        option_value(args, "--key-pass"),
        env::var("NCC_ANDROID_KEYSTORE_PASSWORD").ok(),
        env::var("NCC_ANDROID_KEY_PASSWORD").ok(),
    )?;
    Ok(AndroidBuildOptions {
        source,
        out_dir,
        sdk: option_value(args, "--sdk").map(PathBuf::from),
        ndk: option_value(args, "--ndk").map(PathBuf::from),
        min_api,
        compile_api: option_value(args, "--compile-api")
            .unwrap_or_else(|| DEFAULT_COMPILE_API.into()),
        build_tools: option_value(args, "--build-tools")
            .unwrap_or_else(|| DEFAULT_BUILD_TOOLS.into()),
        native_prefix: option_value(args, "--native-prefix"),
        package_id,
        distribution,
        trusted_signing_certs: option_value(args, "--trusted-signing-certs")
            .or_else(|| env::var("NCC_ANDROID_TRUSTED_SIGNING_CERTS_SHA256").ok()),
        bundletool: option_value(args, "--bundletool")
            .map(PathBuf::from)
            .or_else(|| env::var_os("NCC_ANDROID_BUNDLETOOL").map(PathBuf::from))
            .map(absolute_path)
            .transpose()?,
        keystore,
        key_alias: option_value(args, "--key-alias").unwrap_or_else(|| "ncc-android-local".into()),
        store_password,
        key_password,
        install: has_flag(args, "--install"),
        json: has_flag(args, "--json"),
    })
}

fn resolve_signing_passwords(
    distribution: AndroidDistribution,
    cli_store_password: Option<String>,
    cli_key_password: Option<String>,
    environment_store_password: Option<String>,
    environment_key_password: Option<String>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    if distribution == AndroidDistribution::StoreRelease {
        let store_password = environment_store_password
            .filter(|value| !value.is_empty())
            .ok_or("store-release requires NCC_ANDROID_KEYSTORE_PASSWORD")?;
        let key_password = environment_key_password
            .filter(|value| !value.is_empty())
            .ok_or("store-release requires NCC_ANDROID_KEY_PASSWORD")?;
        return Ok((store_password, key_password));
    }
    Ok((
        cli_store_password
            .or(environment_store_password)
            .unwrap_or_else(|| "android".into()),
        cli_key_password
            .or(environment_key_password)
            .unwrap_or_else(|| "android".into()),
    ))
}

fn android_build(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_build_options(args)?;
    let tools = discover_toolchain(
        options.sdk.as_deref(),
        options.ndk.as_deref(),
        &options.compile_api,
        &options.build_tools,
    )?;
    let source = fs::read_to_string(&options.source)?;
    let dirs = AndroidBuildDirs::create(&options.out_dir)?;
    let profile = ncc_ir::BackendLoweringProfileIR::nc_aarch64_android(options.min_api);
    let mut ast = load_ast_from_path(&options.source.to_string_lossy())?;
    super::compiler::compiler_native_attach_required_nc_text_runtime(
        &mut ast,
        &profile.target_spec.target_triple,
    )?;
    super::compiler::compiler_native_attach_nc_parse_runtime(&mut ast)?;
    super::compiler::compiler_native_attach_nc_unicode_runtime(
        &mut ast,
        &profile.target_spec.target_triple,
    )?;
    sema_check_with_warnings(&ast)?;

    let ir = ncc_ir::lower_with_backend_profile_and_source(
        ast,
        profile,
        options.source.display().to_string(),
        &source,
    );
    let pass_pipeline = ncc_ir::mct_llvm_pass_pipeline(ncc_ir::MctLlvmOptimizationLevelIR::O3);
    let attributes = ncc_ir::mct_llvm_target_attributes_for_spec(&ir.target_spec);
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
        &ir,
        pass_pipeline,
        super::mct_json::mct_llvm_debug_config_for_path(&options.source.to_string_lossy()),
        attributes,
    );
    if module.emitted_function_count == 0 || module.rejected_function_count != 0 {
        return Err(format!(
            "Android MCT lowering rejected the source: emitted={} rejected={} diagnostics={:?}",
            module.emitted_function_count, module.rejected_function_count, module.diagnostics
        )
        .into());
    }
    let prefix = options
        .native_prefix
        .clone()
        .unwrap_or_else(|| infer_native_prefix(&module.llvm_ir).unwrap_or_default());
    if prefix.is_empty() {
        return Err(
            "cannot infer Android native container; pass --native-prefix <container>".into(),
        );
    }
    let native_profile = validate_native_exports(&module.llvm_ir, &prefix)?;

    let stem = options
        .source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("android");
    let ll_path = dirs.lowering.join(format!("{stem}.android-arm64.ll"));
    let nm_path = dirs.lowering.join(format!("{stem}.nm"));
    let core_object = dirs.obj.join(format!("{stem}.android-arm64.o"));
    let runtime_source = dirs.lowering.join("ncc_android_runtime.c");
    let runtime_object = dirs.obj.join("ncc_android_runtime.o");
    let jni_source = dirs.lowering.join("nc_android_jni.c");
    let jni_object = dirs.obj.join("nc_android_jni.o");
    let shared_object = dirs.native_lib.join("libncblockmaster.so");
    fs::write(&ll_path, &module.llvm_ir)?;
    fs::write(&nm_path, android_machine_bridge(stem, &prefix))?;
    super::machine::check_machine_source_path(&nm_path.to_string_lossy())?;
    fs::write(
        &runtime_source,
        super::compiler::COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE,
    )?;
    fs::write(
        &jni_source,
        native_profile.render_jni(&prefix, &options.package_id),
    )?;

    let target = format!("aarch64-linux-android{}", options.min_api);
    run_checked(
        &tools.clang,
        [
            OsString::from(format!("--target={target}")),
            "-O3".into(),
            "-fPIC".into(),
            "-fvisibility=hidden".into(),
            "-ffunction-sections".into(),
            "-fdata-sections".into(),
            "-c".into(),
            ll_path.as_os_str().into(),
            "-o".into(),
            core_object.as_os_str().into(),
        ],
        None,
        "compile nC LLVM to ARM64 object",
    )?;
    run_checked(
        &tools.clang,
        [
            OsString::from(format!("--target={target}")),
            "-O3".into(),
            "-fPIC".into(),
            "-fvisibility=hidden".into(),
            "-fstack-protector-strong".into(),
            "-D_FORTIFY_SOURCE=2".into(),
            "-ffunction-sections".into(),
            "-fdata-sections".into(),
            "-std=c11".into(),
            "-c".into(),
            runtime_source.as_os_str().into(),
            "-o".into(),
            runtime_object.as_os_str().into(),
        ],
        None,
        "compile Android hosted runtime",
    )?;
    run_checked(
        &tools.clang,
        [
            OsString::from(format!("--target={target}")),
            "-O3".into(),
            "-fPIC".into(),
            "-fvisibility=hidden".into(),
            "-fstack-protector-strong".into(),
            "-D_FORTIFY_SOURCE=2".into(),
            "-ffunction-sections".into(),
            "-fdata-sections".into(),
            "-std=c11".into(),
            "-c".into(),
            jni_source.as_os_str().into(),
            "-o".into(),
            jni_object.as_os_str().into(),
        ],
        None,
        "compile JNI provider",
    )?;
    run_checked(
        &tools.clang,
        [
            OsString::from(format!("--target={target}")),
            "-shared".into(),
            "-Wl,--no-undefined".into(),
            "-Wl,-z,relro,-z,now".into(),
            "-Wl,-z,noexecstack".into(),
            "-Wl,--build-id=sha1".into(),
            "-Wl,--gc-sections".into(),
            "-Wl,--strip-all".into(),
            core_object.as_os_str().into(),
            runtime_object.as_os_str().into(),
            jni_object.as_os_str().into(),
            "-llog".into(),
            "-landroid".into(),
            "-lm".into(),
            "-o".into(),
            shared_object.as_os_str().into(),
        ],
        None,
        "link arm64-v8a shared library",
    )?;
    let readelf = run_checked(
        &tools.llvm_readelf,
        [
            "-h".into(),
            "-d".into(),
            "-s".into(),
            shared_object.as_os_str().into(),
        ],
        None,
        "verify ARM64 ELF shared library",
    )?;
    let readelf_text = String::from_utf8_lossy(&readelf.stdout);
    if !readelf_text.contains("AArch64") || !readelf_text.contains("DYN") {
        return Err("generated native library is not an AArch64 ELF shared object".into());
    }
    let nm = run_checked(
        &tools.llvm_nm,
        [
            "-D".into(),
            "--defined-only".into(),
            shared_object.as_os_str().into(),
        ],
        None,
        "verify JNI exports",
    )?;
    let nm_text = String::from_utf8_lossy(&nm.stdout);
    let jni_prefix = format!(
        "Java_{}_NativeBridge_",
        jni_package_prefix(&options.package_id)
    );
    for required in [
        "JNI_OnLoad".to_string(),
        format!("{jni_prefix}command"),
        format!("{jni_prefix}shouldBlock"),
    ] {
        if !nm_text.contains(&required) {
            return Err(format!("Android .so misses required export '{required}'").into());
        }
    }
    if native_profile.supports_contacts() {
        for required in [
            format!("{jni_prefix}shouldBlockContactAt"),
            format!("{jni_prefix}lastDecision"),
        ] {
            if !nm_text.contains(&required) {
                return Err(format!("Android contacts .so misses JNI export '{required}'").into());
            }
        }
    }

    let keystore = options
        .keystore
        .clone()
        .unwrap_or_else(|| dirs.keys.join("blockmaster-signing.p12"));
    generate_signing_identity(&tools.keytool, &keystore, &options)?;
    let signing_cert_sha256 = signing_certificate_sha256(&tools.keytool, &keystore, &options)?;
    let trusted_signing_certs = resolve_trusted_signers(&options, &signing_cert_sha256)?;
    let build_id = crate::content_cache::sha256_bytes(
        format!(
            "{}|{}|{}|{}|{}",
            UNCALL_VERSION_NAME,
            options.distribution.as_str(),
            options.package_id,
            trusted_signing_certs,
            crate::content_cache::sha256_bytes(source.as_bytes())
        )
        .as_bytes(),
    );
    write_android_sources(
        &dirs,
        native_profile,
        options.distribution,
        &options.package_id,
        &trusted_signing_certs,
        &build_id,
    )?;
    validate_distribution_surface(&dirs, options.distribution, &options.package_id)?;

    let resources_zip = dirs.build.join("resources.zip");
    run_checked(
        &tools.aapt2,
        [
            "compile".into(),
            "--dir".into(),
            dirs.res.as_os_str().into(),
            "-o".into(),
            resources_zip.as_os_str().into(),
        ],
        None,
        "compile Android resources",
    )?;
    let unsigned_apk = dirs.build.join("blockmaster-unsigned.apk");
    run_checked(
        &tools.aapt2,
        [
            "link".into(),
            "-o".into(),
            unsigned_apk.as_os_str().into(),
            "-I".into(),
            tools.android_jar.as_os_str().into(),
            "--manifest".into(),
            dirs.manifest.as_os_str().into(),
            "--min-sdk-version".into(),
            options.min_api.to_string().into(),
            "--target-sdk-version".into(),
            compile_api_integer(&options.compile_api).into(),
            "--version-code".into(),
            UNCALL_VERSION_CODE.into(),
            "--version-name".into(),
            UNCALL_VERSION_NAME.into(),
            "--auto-add-overlay".into(),
            "-A".into(),
            dirs.assets.as_os_str().into(),
            "-R".into(),
            resources_zip.as_os_str().into(),
        ],
        None,
        "link Android resources",
    )?;

    let java_files = collect_files(&dirs.java, "java")?;
    let mut javac_args = vec![
        "-encoding".into(),
        "UTF-8".into(),
        "-source".into(),
        "8".into(),
        "-target".into(),
        "8".into(),
        "-classpath".into(),
        tools.android_jar.as_os_str().into(),
        "-d".into(),
        dirs.classes.as_os_str().into(),
    ];
    javac_args.extend(java_files.iter().map(|path| path.as_os_str().into()));
    run_checked(
        &tools.javac,
        javac_args,
        None,
        "compile Android Java provider",
    )?;
    let class_files = collect_files(&dirs.classes, "class")?;
    let mut d8_args = vec![
        "--release".into(),
        "--lib".into(),
        tools.android_jar.as_os_str().into(),
        "--min-api".into(),
        options.min_api.to_string().into(),
        "--output".into(),
        dirs.dex.as_os_str().into(),
    ];
    d8_args.extend(class_files.iter().map(|path| path.as_os_str().into()));
    run_checked(&tools.d8, d8_args, None, "convert Java bytecode to DEX")?;

    let stage_dex = dirs.package_stage.join("classes.dex");
    fs::copy(dirs.dex.join("classes.dex"), &stage_dex)?;
    let stage_lib = dirs.package_stage.join("lib").join("arm64-v8a");
    fs::create_dir_all(&stage_lib)?;
    fs::copy(&shared_object, stage_lib.join("libncblockmaster.so"))?;
    run_checked(
        &tools.aapt,
        [
            "add".into(),
            unsigned_apk.as_os_str().into(),
            "classes.dex".into(),
            "lib/arm64-v8a/libncblockmaster.so".into(),
        ],
        Some(&dirs.package_stage),
        "embed DEX and arm64-v8a library",
    )?;

    let aligned_apk = dirs.build.join("blockmaster-aligned.apk");
    run_checked(
        &tools.zipalign,
        [
            "-f".into(),
            "-p".into(),
            "4".into(),
            unsigned_apk.as_os_str().into(),
            aligned_apk.as_os_str().into(),
        ],
        None,
        "align APK",
    )?;
    let signed_apk = dirs.bin.join("BlockMaster-arm64-v8a-signed.apk");
    run_checked(
        &tools.apksigner,
        [
            "sign".into(),
            "--ks".into(),
            keystore.as_os_str().into(),
            "--ks-key-alias".into(),
            options.key_alias.clone().into(),
            "--ks-pass".into(),
            format!("pass:{}", options.store_password).into(),
            "--key-pass".into(),
            format!("pass:{}", options.key_password).into(),
            "--out".into(),
            signed_apk.as_os_str().into(),
            aligned_apk.as_os_str().into(),
        ],
        None,
        "sign APK",
    )?;
    run_checked(
        &tools.apksigner,
        [
            "verify".into(),
            "--verbose".into(),
            "--print-certs".into(),
            signed_apk.as_os_str().into(),
        ],
        None,
        "verify signed APK",
    )?;
    run_checked(
        &tools.zipalign,
        [
            "-c".into(),
            "-p".into(),
            "4".into(),
            signed_apk.as_os_str().into(),
        ],
        None,
        "verify APK alignment",
    )?;
    let signed_aab = if options.distribution == AndroidDistribution::StoreRelease {
        Some(build_store_bundle(
            &tools,
            &options,
            &dirs,
            &resources_zip,
            &shared_object,
            &keystore,
        )?)
    } else {
        None
    };

    let mobile_tool_root = dirs.bin.join("pc-tools");
    let controller_path = mobile_tool_root.join("controller/main.n");
    let protocol_test_path = mobile_tool_root.join("protocol/main.n");
    let mobile_tool_paths = MOBILE_TOOL_PROJECT_SOURCES
        .iter()
        .map(|(relative, _)| mobile_tool_root.join(relative))
        .collect::<Vec<_>>();
    let unchanged_python_launcher = dirs.bin.join("run_unchanged_python.ps1");
    let mut artifacts = vec![
        &ll_path,
        &nm_path,
        &core_object,
        &runtime_object,
        &jni_object,
        &shared_object,
        &signed_apk,
    ];
    if options.distribution.remote_control_enabled() {
        fs::create_dir_all(controller_path.parent().ok_or("missing controller project directory")?)?;
        fs::create_dir_all(protocol_test_path.parent().ok_or("missing protocol project directory")?)?;
        fs::write(&controller_path, CONTROLLER_SOURCE)?;
        fs::write(&protocol_test_path, PROTOCOL_TEST_SOURCE)?;
        for ((_, source), path) in MOBILE_TOOL_PROJECT_SOURCES.iter().zip(&mobile_tool_paths) {
            fs::create_dir_all(path.parent().ok_or("missing mobile tool source directory")?)?;
            fs::write(path, source)?;
        }
        artifacts.extend(mobile_tool_paths.iter());
        fs::write(&unchanged_python_launcher, UNCHANGED_PYTHON_LAUNCHER)?;
        artifacts.extend([
            &controller_path,
            &protocol_test_path,
            &unchanged_python_launcher,
        ]);
    } else {
        for source in &mobile_tool_paths {
            if source.exists() {
                fs::remove_file(source)?;
            }
        }
        for development_artifact in [
            &controller_path,
            &protocol_test_path,
            &unchanged_python_launcher,
        ] {
            if development_artifact.exists() {
                fs::remove_file(development_artifact)?;
            }
        }
    }
    if let Some(bundle) = signed_aab.as_ref() {
        artifacts.push(bundle);
    }
    let protocols = if options.distribution.remote_control_enabled() {
        json!({
            "legacy": {"transport": "plain TCP over loopback/authorized ADB forwarding", "port": 8080, "bind": "loopback", "framing": ["connection-idle"]},
            "secure": {"transport": "TLS 1.2/1.3 with a per-installation non-exportable key plus HMAC-SHA256", "port": 8443, "framing": ["LINE", "FRAME"], "anti_replay": "session nonce plus strict sequence", "reconnect": "new authenticated session"},
            "raw": {"status": "disabled"}
        })
    } else {
        json!({
            "network": {"status": "disabled", "reason": "store-release local-only privacy profile"},
            "screening": {"transport": "Android CallScreeningService", "data_residency": "on-device"}
        })
    };
    let manifest = json!({
        "schema": "nc.android.blockmaster.build.v2",
        "status": "passed",
        "source": options.source,
        "source_sha256": crate::content_cache::sha256_bytes(source.as_bytes()),
        "target": format!("aarch64-linux-android{}", options.min_api),
        "abi": "arm64-v8a",
        "native_prefix": prefix,
        "native_profile": native_profile.as_str(),
        "distribution": options.distribution.as_str(),
        "version_code": UNCALL_VERSION_CODE,
        "version_name": UNCALL_VERSION_NAME,
        "upload_signing_certificate_sha256": signing_cert_sha256,
        "trusted_distribution_certificates_sha256": trusted_signing_certs.split(';').collect::<Vec<_>>(),
        "package": options.package_id,
        "activity": activity_name(&options.package_id),
        "pipeline": ["nC", "IR O3", "LLVM/MCT", "ELF object", "arm64-v8a .so", "JNI provider", "CallScreeningService", "DEX", "aligned APK", "APK signature", if signed_aab.is_some() { "validated signed AAB" } else { "development APK" }],
        "protocols": protocols,
        "store_release": {
            "aab": signed_aab,
            "permissions": if options.distribution == AndroidDistribution::StoreRelease { vec!["android.permission.READ_CONTACTS"] } else { vec!["android.permission.INTERNET", "android.permission.ACCESS_NETWORK_STATE", "android.permission.FOREGROUND_SERVICE", "android.permission.FOREGROUND_SERVICE_SPECIAL_USE", "android.permission.POST_NOTIFICATIONS", "android.permission.READ_CONTACTS"] },
            "bundled_tls_private_key": false,
            "runtime_private_key": if options.distribution.remote_control_enabled() { "AndroidKeyStore per installation" } else { "not applicable; network disabled" }
        },
        "artifacts": artifacts.iter().map(|path| json!({"path": path, "bytes": fs::metadata(path).map(|m| m.len()).unwrap_or(0), "sha256": sha256_file(path).unwrap_or_default()})).collect::<Vec<_>>(),
        "physical_install": options.install,
    });
    let manifest_path = dirs.run.join("blockmaster-android-build.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    if options.install {
        install_apk(&tools.adb, &signed_apk, &options.package_id)?;
    }
    if options.json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!(
            "OK: Android APK={} AAB={} so={} nm={} manifest={}",
            signed_apk.display(),
            signed_aab
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "not-requested".into()),
            shared_object.display(),
            nm_path.display(),
            manifest_path.display()
        );
    }
    Ok(())
}

fn android_install(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let apk = positional(args).ok_or("android install requires a signed .apk")?;
    let package_id =
        option_value(args, "--package-id").unwrap_or_else(|| DEFAULT_PACKAGE_ID.to_string());
    validate_package_id(&package_id)?;
    let sdk = option_value(args, "--sdk").map(PathBuf::from);
    let tools = discover_toolchain(
        sdk.as_deref(),
        None,
        DEFAULT_COMPILE_API,
        DEFAULT_BUILD_TOOLS,
    )?;
    install_apk(&tools.adb, Path::new(&apk), &package_id)?;
    if has_flag(args, "--json") {
        println!(
            "{}",
            json!({"status":"passed", "apk":apk, "package":package_id, "activity":activity_name(&package_id)})
        );
    } else {
        println!("OK: installed and launched {apk}");
    }
    Ok(())
}

struct AndroidBuildDirs {
    code: PathBuf,
    lowering: PathBuf,
    obj: PathBuf,
    build: PathBuf,
    bin: PathBuf,
    run: PathBuf,
    keys: PathBuf,
    java: PathBuf,
    res: PathBuf,
    assets: PathBuf,
    classes: PathBuf,
    dex: PathBuf,
    native_lib: PathBuf,
    package_stage: PathBuf,
    manifest: PathBuf,
}

impl AndroidBuildDirs {
    fn create(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let dirs = Self {
            code: root.join("code"),
            lowering: root.join("lowering"),
            obj: root.join("obj"),
            build: root.join("build"),
            bin: root.join("bin"),
            run: root.join("run"),
            keys: root.join("keys"),
            java: root.join("src"),
            res: root.join("res"),
            assets: root.join("assets"),
            classes: root.join("classes"),
            dex: root.join("dex"),
            native_lib: root.join("native").join("lib").join("arm64-v8a"),
            package_stage: root.join("package"),
            manifest: root.join("AndroidManifest.xml"),
        };
        for dir in [
            &dirs.code,
            &dirs.lowering,
            &dirs.obj,
            &dirs.build,
            &dirs.bin,
            &dirs.run,
            &dirs.keys,
            &dirs.java,
            &dirs.res,
            &dirs.assets,
            &dirs.classes,
            &dirs.dex,
            &dirs.native_lib,
            &dirs.package_stage,
        ] {
            fs::create_dir_all(dir)?;
        }
        Ok(dirs)
    }
}

fn write_android_sources(
    dirs: &AndroidBuildDirs,
    native_profile: AndroidNativeProfile,
    distribution: AndroidDistribution,
    package_id: &str,
    trusted_signing_certs: &str,
    build_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = match distribution {
        AndroidDistribution::Development => ANDROID_MANIFEST,
        AndroidDistribution::StoreRelease => ANDROID_STORE_MANIFEST,
    };
    fs::write(
        &dirs.manifest,
        manifest.replace(PROVIDER_PACKAGE_ID, package_id),
    )?;
    let package_dir = package_source_dir(&dirs.java, package_id);
    fs::create_dir_all(&package_dir)?;
    for (name, source) in JAVA_SOURCES {
        fs::write(
            package_dir.join(name),
            source.replace(PROVIDER_PACKAGE_ID, package_id),
        )?;
    }
    let build_config = BUILD_CONFIG_TEMPLATE
        .replace(PROVIDER_PACKAGE_ID, package_id)
        .replace("@@BUILD_ID@@", &build_id[..16])
        .replace(
            "@@TRUSTED_SIGNING_CERTS_SHA256@@",
            &java_escape(trusted_signing_certs),
        )
        .replace("@@DISTRIBUTION@@", distribution.as_str())
        .replace(
            "@@REMOTE_CONTROL_ENABLED@@",
            if distribution.remote_control_enabled() {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "@@LEGACY_USB_CONTROL_ENABLED@@",
            if distribution.legacy_usb_control_enabled() {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "@@NATIVE_CONTACTS@@",
            if native_profile.supports_contacts() {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "@@NATIVE_BMUI1@@",
            if native_profile.supports_bmui1() {
                "true"
            } else {
                "false"
            },
        );
    fs::write(package_dir.join("BuildConfig.java"), build_config)?;
    for (relative, contents) in RESOURCE_FILES {
        let path = dirs.res.join(relative);
        if distribution == AndroidDistribution::StoreRelease
            && *relative == "xml/network_security_config.xml"
        {
            if path.exists() {
                fs::remove_file(path)?;
            }
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    for (relative, contents) in BINARY_RESOURCE_FILES {
        let path = dirs.res.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    Ok(())
}

fn java_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn validate_distribution_surface(
    dirs: &AndroidBuildDirs,
    distribution: AndroidDistribution,
    package_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if distribution != AndroidDistribution::StoreRelease {
        return Ok(());
    }
    let manifest = fs::read_to_string(&dirs.manifest)?;
    for forbidden in [
        "android.permission.INTERNET",
        "android.permission.ACCESS_NETWORK_STATE",
        "android.permission.FOREGROUND_SERVICE",
        "android.permission.FOREGROUND_SERVICE_SPECIAL_USE",
        "android.permission.POST_NOTIFICATIONS",
        "android.permission.READ_CALL_LOG",
        "BlockMasterService",
    ] {
        if manifest.contains(forbidden) {
            return Err(
                format!("store-release manifest contains forbidden surface '{forbidden}'").into(),
            );
        }
    }
    let permission_count = manifest.matches("<uses-permission ").count();
    if permission_count != 1 || !manifest.contains("android.permission.READ_CONTACTS") {
        return Err("store-release manifest must declare only READ_CONTACTS".into());
    }
    for extension in ["p12", "pfx", "jks", "keystore", "key"] {
        if !collect_files(&dirs.assets, extension)?.is_empty() {
            return Err(format!(
                "store-release assets contain forbidden private-key material '*.{extension}'"
            )
            .into());
        }
    }
    let build_config =
        fs::read_to_string(package_source_dir(&dirs.java, package_id).join("BuildConfig.java"))?;
    if build_config.contains("TLS_STORE_PASSWORD")
        || !build_config.contains("REMOTE_CONTROL_ENABLED = false")
        || !build_config.contains("LEGACY_USB_CONTROL_ENABLED = false")
    {
        return Err(
            "store-release BuildConfig did not close remote control or contains a TLS secret"
                .into(),
        );
    }
    Ok(())
}

fn resolve_trusted_signers(
    options: &AndroidBuildOptions,
    upload_signer: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let configured = match (&options.trusted_signing_certs, options.distribution) {
        (Some(value), _) => value.as_str(),
        (None, AndroidDistribution::Development) => upload_signer,
        (None, AndroidDistribution::StoreRelease) => {
            return Err("store-release requires --trusted-signing-certs with the SHA-256 app-signing certificate fingerprint from Play Console; the upload certificate is not the distribution identity".into())
        }
    };
    normalize_trusted_signers(configured)
}

fn normalize_trusted_signers(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut signers = value
        .split(|character: char| {
            character == ';' || character == ',' || character.is_ascii_whitespace()
        })
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| candidate.replace(':', "").to_ascii_lowercase())
        .collect::<Vec<_>>();
    for signer in &signers {
        if signer.len() != 64 || !signer.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "invalid trusted signing certificate SHA-256 '{signer}'; expected 64 hexadecimal characters"
            )
            .into());
        }
    }
    signers.sort();
    signers.dedup();
    if signers.is_empty() {
        return Err("trusted signing certificate list is empty".into());
    }
    Ok(signers.join(";"))
}

fn infer_native_prefix(llvm: &str) -> Option<String> {
    for suffix in ["_init(", "_cargarConfiguracion("] {
        for line in llvm.lines() {
            if line.contains("define") && line.contains(suffix) {
                let begin = line.find('@')? + 1;
                let end = line[begin..].find(suffix)? + begin;
                return Some(line[begin..end].trim_matches('"').to_string());
            }
        }
    }
    None
}

fn has_native_exports(llvm: &str, prefix: &str, suffixes: &[&str]) -> bool {
    suffixes
        .iter()
        .all(|suffix| llvm.contains(&format!("@{prefix}_{suffix}(")))
}

fn validate_native_exports(
    llvm: &str,
    prefix: &str,
) -> Result<AndroidNativeProfile, Box<dyn std::error::Error>> {
    let blocklist = [
        "init",
        "add",
        "remove",
        "addPattern",
        "removePattern",
        "clear",
        "shouldBlock",
        "callSeen",
        "list",
        "status",
        "snapshot",
        "restore",
    ];
    if has_native_exports(llvm, prefix, &blocklist) {
        let contacts = [
            "setContactsOnlyMode",
            "shouldBlockContactAt",
            "lastDecision",
        ];
        if has_native_exports(llvm, prefix, &contacts) {
            let bmui1 = [
                "addAllowPattern",
                "removeAllowPattern",
                "setProtectionActive",
                "setBlockHidden",
                "activity",
                "uiContract",
                "versionInfo",
                "updatePolicy",
            ];
            if has_native_exports(llvm, prefix, &bmui1) {
                return Ok(AndroidNativeProfile::BlocklistContactsBmui1);
            }
            return Ok(AndroidNativeProfile::BlocklistContacts);
        }
        return Ok(AndroidNativeProfile::Blocklist);
    }
    let urgency = [
        "cargarConfiguracion",
        "limpiarHistorialAntiguo",
        "registrarLlamadaEntrante",
        "verificarUrgenciaYPermitir",
        "procesarComandoPC",
        "guardarWhitelist",
    ];
    if has_native_exports(llvm, prefix, &urgency) {
        return Ok(AndroidNativeProfile::Urgency);
    }
    let missing_blocklist = blocklist
        .iter()
        .filter(|suffix| !llvm.contains(&format!("@{prefix}_{suffix}(")))
        .copied()
        .collect::<Vec<_>>();
    let missing_urgency = urgency
        .iter()
        .filter(|suffix| !llvm.contains(&format!("@{prefix}_{suffix}(")))
        .copied()
        .collect::<Vec<_>>();
    Err(format!(
        "nC Android core does not implement a supported provider profile; blocklist missing={missing_blocklist:?}, urgency missing={missing_urgency:?}"
    )
    .into())
}

fn android_machine_bridge(stem: &str, prefix: &str) -> String {
    let name = stem.replace(|ch: char| !ch.is_ascii_alphanumeric(), "_");
    format!("{name} :: machine {{\n    target:\n        arch: aarch64;\n        abi: aapcs64;\n        object: elf64;\n        endian: little;\n        pointerWidth: 64;\n        profile: userland;\n\n    symbol:\n        {prefix}_android_bridge:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        {prefix}_android_bridge;\n\n    proc:\n        {prefix}_android_bridge:\n            abi: aapcs64;\n\n    instr:\n        ret;\n\n    verify:\n        target;\n        abi;\n        symbols;\n        encoding;\n\n    seal:\n        artifact: elf64;\n        require: verified;\n}}\n")
}

fn compile_api_integer(value: &str) -> String {
    value.split('.').next().unwrap_or(value).to_string()
}

fn discover_toolchain(
    sdk_override: Option<&Path>,
    ndk_override: Option<&Path>,
    compile_api: &str,
    build_tools: &str,
) -> Result<AndroidToolchain, Box<dyn std::error::Error>> {
    let sdk = sdk_override
        .map(PathBuf::from)
        .or_else(|| env::var_os("ANDROID_SDK_ROOT").map(PathBuf::from))
        .or_else(|| env::var_os("ANDROID_HOME").map(PathBuf::from))
        .or_else(default_sdk_path)
        .ok_or("Android SDK not found; set ANDROID_SDK_ROOT or pass --sdk")?;
    let ndk = ndk_override
        .map(PathBuf::from)
        .unwrap_or_else(|| sdk.join("ndk").join(DEFAULT_NDK_VERSION));
    let host_tag = if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "linux-x86_64"
    };
    let ndk_bin = ndk
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join(host_tag)
        .join("bin");
    let build_bin = sdk.join("build-tools").join(build_tools);
    let platform = sdk.join("platforms").join(format!("android-{compile_api}"));
    let platform_tools = sdk.join("platform-tools");
    let tools = AndroidToolchain {
        sdk,
        ndk,
        android_jar: platform.join("android.jar"),
        clang: executable(&ndk_bin, "clang"),
        llvm_readelf: executable(&ndk_bin, "llvm-readelf"),
        llvm_nm: executable(&ndk_bin, "llvm-nm"),
        aapt2: executable(&build_bin, "aapt2"),
        aapt: executable(&build_bin, "aapt"),
        d8: executable(&build_bin, "d8"),
        zipalign: executable(&build_bin, "zipalign"),
        apksigner: executable(&build_bin, "apksigner"),
        adb: executable(&platform_tools, "adb"),
        javac: find_java_tool("javac")?,
        java: find_java_tool("java")?,
        jar: find_java_tool("jar")?,
        jarsigner: find_java_tool("jarsigner")?,
        keytool: find_java_tool("keytool")?,
    };
    for (label, path) in [
        ("NDK", &tools.ndk),
        ("android.jar", &tools.android_jar),
        ("clang", &tools.clang),
        ("llvm-readelf", &tools.llvm_readelf),
        ("llvm-nm", &tools.llvm_nm),
        ("aapt2", &tools.aapt2),
        ("aapt", &tools.aapt),
        ("d8", &tools.d8),
        ("zipalign", &tools.zipalign),
        ("apksigner", &tools.apksigner),
        ("adb", &tools.adb),
        ("javac", &tools.javac),
        ("java", &tools.java),
        ("jar", &tools.jar),
        ("jarsigner", &tools.jarsigner),
        ("keytool", &tools.keytool),
    ] {
        if !path.exists() {
            return Err(format!(
                "Android toolchain component '{label}' not found at {}",
                path.display()
            )
            .into());
        }
    }
    Ok(tools)
}

fn default_sdk_path() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Android").join("Sdk"))
    } else {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join("Android").join("Sdk"))
    }
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn executable(dir: &Path, name: &str) -> PathBuf {
    if cfg!(windows) {
        for extension in ["exe", "bat", "cmd"] {
            let path = dir.join(format!("{name}.{extension}"));
            if path.exists() {
                return path;
            }
        }
    }
    dir.join(name)
}

fn find_java_tool(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(home) = env::var_os("JAVA_HOME") {
        let path = executable(&PathBuf::from(home).join("bin"), name);
        if path.exists() {
            return Ok(path);
        }
    }
    let probe = if cfg!(windows) { "where.exe" } else { "which" };
    let output = Command::new(probe).arg(name).output()?;
    if output.status.success() {
        if let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next() {
            return Ok(PathBuf::from(line.trim()));
        }
    }
    Err(format!("Java tool '{name}' not found; set JAVA_HOME").into())
}

fn toolchain_report(tools: &AndroidToolchain) -> serde_json::Value {
    json!({"schema":"nc.android.doctor.v1", "status":"passed", "sdk":tools.sdk, "ndk":tools.ndk, "android_jar":tools.android_jar, "clang":tools.clang, "llvm_readelf":tools.llvm_readelf, "aapt2":tools.aapt2, "d8":tools.d8, "zipalign":tools.zipalign, "apksigner":tools.apksigner, "adb":tools.adb, "javac":tools.javac, "java":tools.java, "jar":tools.jar, "jarsigner":tools.jarsigner, "keytool":tools.keytool})
}

fn run_checked<I>(
    tool: &Path,
    args: I,
    cwd: Option<&Path>,
    stage: &str,
) -> Result<Output, Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    let mut command = Command::new(tool);
    command.args(&args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .map_err(|error| format!("{stage}: failed to launch {}: {error}", tool.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{stage} failed ({}):\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output)
}

fn collect_files(root: &Path, extension: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    fn visit(root: &Path, extension: &str, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() {
                visit(&path, extension, out)?;
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, extension, &mut files)?;
    files.sort();
    Ok(files)
}

fn generate_signing_identity(
    keytool: &Path,
    store: &Path,
    options: &AndroidBuildOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    if store.exists() {
        return Ok(());
    }
    if let Some(parent) = store.parent() {
        fs::create_dir_all(parent)?;
    }
    run_checked(
        keytool,
        [
            "-genkeypair".into(),
            "-storetype".into(),
            "PKCS12".into(),
            "-keystore".into(),
            store.as_os_str().into(),
            "-storepass".into(),
            options.store_password.clone().into(),
            "-keypass".into(),
            options.key_password.clone().into(),
            "-alias".into(),
            options.key_alias.clone().into(),
            "-keyalg".into(),
            "RSA".into(),
            "-keysize".into(),
            "3072".into(),
            "-sigalg".into(),
            "SHA256withRSA".into(),
            "-validity".into(),
            "10000".into(),
            "-dname".into(),
            "CN=nC Android local signing,O=nC,C=ES".into(),
        ],
        None,
        "generate APK signing identity",
    )?;
    Ok(())
}

fn signing_certificate_sha256(
    keytool: &Path,
    store: &Path,
    options: &AndroidBuildOptions,
) -> Result<String, Box<dyn std::error::Error>> {
    let certificate = run_checked(
        keytool,
        [
            "-exportcert".into(),
            "-storetype".into(),
            "PKCS12".into(),
            "-keystore".into(),
            store.as_os_str().into(),
            "-storepass".into(),
            options.store_password.clone().into(),
            "-alias".into(),
            options.key_alias.clone().into(),
        ],
        None,
        "export APK signing certificate for runtime pinning",
    )?;
    if certificate.stdout.is_empty() {
        return Err("APK signing certificate export was empty".into());
    }
    Ok(crate::content_cache::sha256_bytes(&certificate.stdout))
}

fn build_store_bundle(
    tools: &AndroidToolchain,
    options: &AndroidBuildOptions,
    dirs: &AndroidBuildDirs,
    resources_zip: &Path,
    shared_object: &Path,
    keystore: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let bundletool = options.bundletool.as_ref().ok_or(
        "store-release requires --bundletool <bundletool-all.jar> or NCC_ANDROID_BUNDLETOOL",
    )?;
    if !bundletool.is_file() {
        return Err(format!("bundletool JAR not found at {}", bundletool.display()).into());
    }

    let proto_apk = dirs.build.join("uncall-proto-resources.apk");
    run_checked(
        &tools.aapt2,
        [
            "link".into(),
            "--proto-format".into(),
            "-o".into(),
            proto_apk.as_os_str().into(),
            "-I".into(),
            tools.android_jar.as_os_str().into(),
            "--manifest".into(),
            dirs.manifest.as_os_str().into(),
            "--min-sdk-version".into(),
            options.min_api.to_string().into(),
            "--target-sdk-version".into(),
            compile_api_integer(&options.compile_api).into(),
            "--version-code".into(),
            UNCALL_VERSION_CODE.into(),
            "--version-name".into(),
            UNCALL_VERSION_NAME.into(),
            "--auto-add-overlay".into(),
            "-A".into(),
            dirs.assets.as_os_str().into(),
            "-R".into(),
            resources_zip.as_os_str().into(),
        ],
        None,
        "link Android resources in App Bundle protobuf format",
    )?;

    let module = dirs.build.join("aab-base-module");
    if module.exists() {
        fs::remove_dir_all(&module)?;
    }
    fs::create_dir_all(&module)?;
    run_checked(
        &tools.jar,
        ["xf".into(), proto_apk.as_os_str().into()],
        Some(&module),
        "extract protobuf base module",
    )?;
    let extracted_manifest = module.join("AndroidManifest.xml");
    let manifest_dir = module.join("manifest");
    fs::create_dir_all(&manifest_dir)?;
    if !extracted_manifest.is_file() {
        return Err("protobuf base module did not contain AndroidManifest.xml".into());
    }
    fs::rename(extracted_manifest, manifest_dir.join("AndroidManifest.xml"))?;
    if !module.join("resources.pb").is_file() {
        return Err("protobuf base module did not contain resources.pb".into());
    }
    let meta_inf = module.join("META-INF");
    if meta_inf.exists() {
        fs::remove_dir_all(meta_inf)?;
    }
    let dex = module.join("dex");
    fs::create_dir_all(&dex)?;
    fs::copy(dirs.dex.join("classes.dex"), dex.join("classes.dex"))?;
    let native = module.join("lib").join("arm64-v8a");
    fs::create_dir_all(&native)?;
    fs::copy(shared_object, native.join("libncblockmaster.so"))?;

    let base_zip = dirs.build.join("uncall-base.zip");
    if base_zip.exists() {
        fs::remove_file(&base_zip)?;
    }
    run_checked(
        &tools.jar,
        [
            "cMf".into(),
            base_zip.as_os_str().into(),
            "-C".into(),
            module.as_os_str().into(),
            ".".into(),
        ],
        None,
        "package App Bundle base module",
    )?;

    let unsigned_aab = dirs.build.join("UnCall-store-release-unsigned.aab");
    if unsigned_aab.exists() {
        fs::remove_file(&unsigned_aab)?;
    }
    run_checked(
        &tools.java,
        [
            "-jar".into(),
            bundletool.as_os_str().into(),
            "build-bundle".into(),
            format!("--modules={}", base_zip.display()).into(),
            format!("--output={}", unsigned_aab.display()).into(),
        ],
        None,
        "build Android App Bundle",
    )?;

    let signed_aab = dirs.bin.join("UnCall-store-release.aab");
    if signed_aab.exists() {
        fs::remove_file(&signed_aab)?;
    }
    run_checked(
        &tools.jarsigner,
        [
            "-keystore".into(),
            keystore.as_os_str().into(),
            "-storetype".into(),
            "PKCS12".into(),
            "-storepass".into(),
            options.store_password.clone().into(),
            "-keypass".into(),
            options.key_password.clone().into(),
            "-digestalg".into(),
            "SHA-256".into(),
            "-sigalg".into(),
            "SHA256withRSA".into(),
            "-signedjar".into(),
            signed_aab.as_os_str().into(),
            unsigned_aab.as_os_str().into(),
            options.key_alias.clone().into(),
        ],
        None,
        "sign Android App Bundle with upload key",
    )?;
    run_checked(
        &tools.jarsigner,
        [
            "-verify".into(),
            "-verbose".into(),
            "-certs".into(),
            signed_aab.as_os_str().into(),
        ],
        None,
        "verify Android App Bundle signature",
    )?;
    run_checked(
        &tools.java,
        [
            "-jar".into(),
            bundletool.as_os_str().into(),
            "validate".into(),
            format!("--bundle={}", signed_aab.display()).into(),
        ],
        None,
        "validate Android App Bundle",
    )?;
    Ok(signed_aab)
}

fn install_apk(adb: &Path, apk: &Path, package_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let devices = run_checked(adb, ["devices".into()], None, "query Android devices")?;
    let authorized = String::from_utf8_lossy(&devices.stdout)
        .lines()
        .skip(1)
        .filter(|line| line.trim_end().ends_with("\tdevice"))
        .count();
    if authorized != 1 {
        return Err(format!(
            "android install requires exactly one authorized adb device, found {authorized}"
        )
        .into());
    }
    run_checked(
        adb,
        ["install".into(), "-r".into(), apk.as_os_str().into()],
        None,
        "install signed APK",
    )?;
    run_checked(
        adb,
        [
            "shell".into(),
            "am".into(),
            "start".into(),
            "-n".into(),
            format!("{package_id}/{}", activity_name(package_id)).into(),
        ],
        None,
        "launch BlockMaster activity",
    )?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_pc_tools_are_native_nc_projects_without_python_payloads() {
        assert!(CONTROLLER_SOURCE.contains("ref main ->> run;"));
        assert!(PROTOCOL_TEST_SOURCE.contains("ref main ->> run;"));
        assert_eq!(MOBILE_TOOL_PROJECT_SOURCES.len(), 26);
        let mut names = std::collections::BTreeSet::new();
        for (name, source) in MOBILE_TOOL_PROJECT_SOURCES {
            assert!(name.ends_with(".n"), "non-native asset {name}");
            assert!(!source.trim().is_empty(), "empty source {name}");
            assert!(names.insert(name), "duplicate asset {name}");
        }
        assert!(names.contains(&"build/main.n"));
        assert!(names.contains(&"linker/main.n"));
        assert!(names.contains(&"controller/src/mobile43_cli.n"));
        assert!(names.contains(&"protocol/src/mobile43_conformance.n"));
    }

    #[test]
    fn definitive_uncall_package_id_is_valid_and_reaches_jni() {
        validate_package_id(DEFAULT_PACKAGE_ID).unwrap();
        assert_eq!(DEFAULT_PACKAGE_ID, "com.aristote.uncall");
        assert_eq!(
            activity_name(DEFAULT_PACKAGE_ID),
            "com.aristote.uncall.MainActivity"
        );
        assert!(validate_package_id("dev.nc").is_err());
        assert!(validate_package_id("Com.aristote.uncall").is_err());
        assert!(validate_package_id("com.aristote.un_call").is_err());
        let rendered = AndroidNativeProfile::Blocklist.render_jni("UnCall", DEFAULT_PACKAGE_ID);
        assert!(rendered.contains("Java_com_aristote_uncall_NativeBridge_command"));
        assert!(!rendered.contains("Java_dev_nc_blockmaster_"));
    }

    fn skip_java_space(bytes: &[u8], cursor: &mut usize) {
        while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
            *cursor += 1;
        }
    }

    fn java_string_expression(source: &str, cursor: &mut usize) -> Option<String> {
        let bytes = source.as_bytes();
        skip_java_space(bytes, cursor);
        let mut decoded = Vec::new();
        let mut found = false;
        loop {
            if *cursor >= bytes.len() || bytes[*cursor] != b'"' {
                break;
            }
            found = true;
            *cursor += 1;
            while *cursor < bytes.len() && bytes[*cursor] != b'"' {
                if bytes[*cursor] == b'\\' && *cursor + 1 < bytes.len() {
                    *cursor += 1;
                    decoded.push(match bytes[*cursor] {
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        escaped => escaped,
                    });
                } else {
                    decoded.push(bytes[*cursor]);
                }
                *cursor += 1;
            }
            if *cursor >= bytes.len() {
                return None;
            }
            *cursor += 1;
            skip_java_space(bytes, cursor);
            if *cursor < bytes.len() && bytes[*cursor] == b'+' {
                *cursor += 1;
                skip_java_space(bytes, cursor);
                continue;
            }
            break;
        }
        found.then(|| String::from_utf8(decoded).expect("Java UI strings must be UTF-8"))
    }

    fn java_localized_english_keys(source: &str) -> std::collections::BTreeSet<String> {
        let bytes = source.as_bytes();
        let mut keys = std::collections::BTreeSet::new();
        for marker in ["l(", "localized("] {
            let mut offset = 0;
            while let Some(found) = source[offset..].find(marker) {
                let mut cursor = offset + found + marker.len();
                if java_string_expression(source, &mut cursor).is_some() {
                    skip_java_space(bytes, &mut cursor);
                    if cursor < bytes.len() && bytes[cursor] == b',' {
                        cursor += 1;
                        if let Some(english) = java_string_expression(source, &mut cursor) {
                            if !english.is_empty() {
                                keys.insert(english);
                            }
                        }
                    }
                }
                offset += found + marker.len();
            }
        }
        keys
    }

    fn java_catalog_keys(source: &str) -> std::collections::BTreeSet<String> {
        let mut keys = std::collections::BTreeSet::new();
        let marker = "add(table,";
        let mut offset = 0;
        while let Some(found) = source[offset..].find(marker) {
            let mut cursor = offset + found + marker.len();
            if let Some(key) = java_string_expression(source, &mut cursor) {
                keys.insert(key);
            }
            offset = cursor.max(offset + found + marker.len());
        }
        keys
    }

    #[test]
    fn android_machine_bridge_is_aarch64_aapcs64_elf() {
        let source = android_machine_bridge("blockmaster", "blockMasterAndroid");
        assert!(source.contains("arch: aarch64"));
        assert!(source.contains("abi: aapcs64"));
        assert!(source.contains("object: elf64"));
        assert!(source.contains("blockMasterAndroid_android_bridge"));
    }

    #[test]
    fn android_lowering_profile_is_pic_aarch64_elf() {
        let profile = ncc_ir::BackendLoweringProfileIR::nc_aarch64_android(29);
        assert_eq!(profile.target_spec.target_triple, "aarch64-linux-android29");
        assert_eq!(profile.target_spec.pointer_width, 64);
        assert_eq!(
            profile.target_spec.object_format,
            ncc_ir::BackendObjectFormatIR::Elf64
        );
        assert_eq!(
            profile.target_spec.relocation_model,
            ncc_ir::BackendRelocationModelIR::Pic
        );
    }

    #[test]
    fn android_attaches_required_nc_text_runtime_before_lowering() {
        let source = r#"
mobileTextConsumer :: runtime {
    fn:
        public:
            join(left >> str, right >> str) -> str {
                return left + right;
            }
}
"#;
        let mut ast = ncc_parser::parse(ncc_lexer::lex(source).unwrap()).unwrap();
        assert!(!ast
            .containers
            .iter()
            .any(|container| container.name == "nccTextFormatRuntime"));

        let attached = super::super::compiler::compiler_native_attach_required_nc_text_runtime(
            &mut ast,
            "aarch64-linux-android29",
        )
        .unwrap();

        assert!(attached);
        assert!(ast
            .containers
            .iter()
            .any(|container| container.name == "nccTextFormatRuntime"));
    }

    #[test]
    fn embedded_android_surface_has_screening_local_compat_and_hardened_protocol() {
        assert!(ANDROID_MANIFEST.contains("android.telecom.CallScreeningService"));
        assert!(ANDROID_MANIFEST.contains("android.permission.READ_CONTACTS"));
        assert!(ANDROID_MANIFEST.contains("@drawable/uncall_icon"));
        assert!(RESOURCE_FILES
            .iter()
            .any(|(path, source)| *path == "values/strings.xml" && source.contains("UnCall")));
        assert!(BINARY_RESOURCE_FILES
            .iter()
            .any(|(path, bytes)| *path == "drawable/uncall_icon.png" && !bytes.is_empty()));
        for nav_icon in ["nav_home.png", "nav_activity.png", "nav_rules.png"] {
            assert!(BINARY_RESOURCE_FILES.iter().any(|(path, bytes)| {
                *path == format!("drawable/{nav_icon}") && !bytes.is_empty()
            }));
        }
        let contacts = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "ContactLookup.java")
            .unwrap()
            .1;
        assert!(contacts.contains("ContactsContract.PhoneLookup.CONTENT_FILTER_URI"));
        let server = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "ProtocolServer.java")
            .unwrap()
            .1;
        assert!(server.contains("LEGACY_PORT = 8080"));
        assert!(server.contains("SECURE_PORT = 8443"));
        assert!(server.contains("InetAddress.getLoopbackAddress()"));
        assert!(!server.contains("RAW_PORT = 8081"));
        assert!(server.contains("FrameCodec.Framing.LINE"));
        assert!(server.contains("FrameCodec.Framing.parse"));
        assert!(server.contains("ProtocolSecurity.hmac"));
        assert!(server.contains("value.startsWith(\"APRENDIZAJE_ADAPTATIVO \""));
        assert!(server.contains("value.startsWith(\"IA_FEEDBACK \""));
        assert!(server.contains("\"IA_REINICIAR\".equals(value)"));
        assert!(JNI_SOURCE.contains("nccTextFormatRuntime_releaseRaw"));
        assert!(!JNI_SOURCE.contains("ncc_rt_str_release"));
        let activity = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "MainActivity.java")
            .unwrap()
            .1;
        assert!(activity.contains("ui_theme_mode"));
        assert!(activity.contains("ui_language_mode"));
        assert!(activity.contains("showBottomDialog"));
        assert!(activity.contains("setOnScrollChangeListener"));
        assert!(activity.contains("reviewContactsPermission"));
        assert!(activity.contains("Actividad actualizada"));
        assert!(activity.contains("showLanguageDialog"));
        assert!(activity.contains("rebuildShellKeepingPage"));
        assert!(activity.contains("addGroupedActivity"));
        assert!(activity.contains("View.LAYOUT_DIRECTION_LTR"));
        assert!(activity.contains("View.TEXT_DIRECTION_RTL"));
        assert!(activity.contains("systemFontScale"));
        assert!(activity.contains("high_text_contrast_enabled"));
        assert!(activity.contains("Settings.Global.ANIMATOR_DURATION_SCALE"));
        assert!(activity.contains("setMinimumHeight(dp(48))"));
        assert!(activity.contains("setAccessibilityHeading(true)"));
        assert!(activity.contains("AccessibilityNodeInfo.AccessibilityAction.ACTION_EXPAND"));
        assert!(activity.contains("compactContent"));
        assert!(activity.contains("dp(760)"));
        assert!(activity.contains("IA_ESTADO"));
        assert!(activity.contains("APRENDIZAJE_ADAPTATIVO"));
        assert!(activity.contains("showAdaptiveConsentDialog"));
        assert!(activity.contains("setOnDismissListener"));
        assert!(activity.contains("enforceAdaptiveConsentOff"));
        assert!(activity.contains("showAdaptiveFeedbackDialog"));
        assert!(activity.contains("IA_FEEDBACK"));
        assert!(activity.contains("showLocalAssistantDialog"));
        assert!(activity.contains("ASISTENTE @welcome"));
        assert!(activity.contains("Confirm change"));
        assert!(activity.contains("Nuinui"));
        assert!(activity.contains("appendNuinuiRuleConfirmation"));
        assert!(activity.contains("nuinuiCallAnswer"));
        assert!(activity.contains("appendNuinuiWebConsent"));
        let nuinui_web = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "NuinuiWebSearch.java")
            .unwrap()
            .1;
        assert!(nuinui_web.contains("HttpsURLConnection"));
        assert!(nuinui_web.contains("wikipedia.org/w/api.php"));
        assert!(nuinui_web.contains("containsPhoneNumber"));
        assert!(nuinui_web.contains("setInstanceFollowRedirects(false)"));
        assert!(!nuinui_web.contains("http://"));
        let signed_spam = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "SignedSpamDatabase.java")
            .unwrap()
            .1;
        assert!(signed_spam.contains("SHA256withECDSA"));
        assert!(signed_spam.contains("EXPECTED_DIGEST"));
        assert!(signed_spam.contains("verifyAndInstall"));
        assert!(signed_spam.contains("UNSP1|2026.08.09.1"));
        assert!(!signed_spam.contains("PRIVATE KEY"));
        assert!(!signed_spam.contains("BEGIN EC PRIVATE"));
        let profiles = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "RuleProfileStore.java")
            .unwrap()
            .1;
        assert!(profiles.contains("SecurityVault.encrypt"));
        assert!(profiles.contains("SHA-256"));
        let activity_store = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "ActivityStore.java")
            .unwrap()
            .1;
        assert!(activity_store.contains("SecurityVault.encrypt"));
        assert!(activity_store.contains("MAX_RECORDS = 250"));
        let translations = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "Translations.java")
            .unwrap()
            .1;
        assert!(translations.contains("\"ar\""));
        assert!(translations.contains("\"zh\""));
        let vault = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "SecurityVault.java")
            .unwrap()
            .1;
        assert!(vault.contains("AES/GCM/NoPadding"));
        assert!(vault.contains("TRUSTED_SIGNING_CERTS_SHA256"));
        assert!(vault.contains("uncall.nc.v1.device.tls"));
        assert!(!vault.contains("BuildConfig.TLS_STORE_PASSWORD"));
        assert!(ANDROID_MANIFEST.contains("android:usesCleartextTraffic=\"false\""));
        assert!(ANDROID_STORE_MANIFEST.contains("android.permission.READ_CONTACTS"));
        assert!(ANDROID_STORE_MANIFEST.contains("android.permission.INTERNET"));
        assert!(ANDROID_STORE_MANIFEST.contains("android:networkSecurityConfig"));
        assert!(!ANDROID_STORE_MANIFEST.contains("FOREGROUND_SERVICE"));
        assert!(!ANDROID_STORE_MANIFEST.contains("BlockMasterService"));
        assert!(BUILD_CONFIG_TEMPLATE.contains("REMOTE_CONTROL_ENABLED"));
        assert!(!BUILD_CONFIG_TEMPLATE.contains("TLS_STORE_PASSWORD"));
    }

    #[test]
    fn every_visible_android_string_has_all_offline_language_rows() {
        let activity = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "MainActivity.java")
            .unwrap()
            .1;
        let translations = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "Translations.java")
            .unwrap()
            .1;
        let mut visible = java_localized_english_keys(activity);
        let service = JAVA_SOURCES
            .iter()
            .find(|(name, _)| *name == "BlockMasterService.java")
            .unwrap()
            .1;
        visible.extend(java_localized_english_keys(service));
        let catalog = java_catalog_keys(translations);
        assert!(
            visible.len() >= 150,
            "unexpectedly small visible UI catalog"
        );
        let missing = visible.difference(&catalog).collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing offline translation rows: {missing:#?}"
        );
        assert!(translations.contains("values.length != TRANSLATED_CODES.length"));
        assert!(translations.contains("static boolean isRtl"));
        assert!(translations.contains("static String relativeTime"));
    }

    #[test]
    fn trusted_store_signers_are_normalized_and_validated() {
        let first = "AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA:AA";
        let second = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert_eq!(
            normalize_trusted_signers(&format!("{second}; {first};{second}")).unwrap(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );
        assert!(normalize_trusted_signers("not-a-certificate").is_err());
    }

    #[test]
    fn store_signing_secrets_must_arrive_outside_the_command_line() {
        let store = resolve_signing_passwords(
            AndroidDistribution::StoreRelease,
            None,
            None,
            Some("store-secret".into()),
            Some("key-secret".into()),
        )
        .unwrap();
        assert_eq!(store, ("store-secret".into(), "key-secret".into()));
        assert!(resolve_signing_passwords(
            AndroidDistribution::StoreRelease,
            Some("visible".into()),
            Some("visible".into()),
            None,
            None,
        )
        .is_err());

        let development = resolve_signing_passwords(
            AndroidDistribution::Development,
            Some("local-store".into()),
            Some("local-key".into()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(development, ("local-store".into(), "local-key".into()));
    }

    #[test]
    fn android_provider_profile_is_selected_from_material_exports() {
        let urgency = [
            "cargarConfiguracion",
            "limpiarHistorialAntiguo",
            "registrarLlamadaEntrante",
            "verificarUrgenciaYPermitir",
            "procesarComandoPC",
            "guardarWhitelist",
        ]
        .into_iter()
        .map(|name| format!("define i1 @core_{name}() {{ ret i1 true }}"))
        .collect::<Vec<_>>()
        .join("\n");
        assert_eq!(infer_native_prefix(&urgency).as_deref(), Some("core"));
        assert_eq!(
            validate_native_exports(&urgency, "core").unwrap(),
            AndroidNativeProfile::Urgency
        );

        let mut blocklist = [
            "init",
            "add",
            "remove",
            "addPattern",
            "removePattern",
            "clear",
            "shouldBlock",
            "callSeen",
            "list",
            "status",
            "snapshot",
            "restore",
            "setContactsOnlyMode",
            "shouldBlockContactAt",
            "lastDecision",
        ]
        .into_iter()
        .map(|name| format!("define i1 @contacts_{name}() {{ ret i1 true }}"))
        .collect::<Vec<_>>()
        .join("\n");
        blocklist.push('\n');
        assert_eq!(
            validate_native_exports(&blocklist, "contacts").unwrap(),
            AndroidNativeProfile::BlocklistContacts
        );
        let rendered =
            AndroidNativeProfile::BlocklistContacts.render_jni("contacts", DEFAULT_PACKAGE_ID);
        assert!(rendered.contains("contacts_shouldBlockContactAt"));
        assert!(rendered.contains("contacts_lastDecision"));
        assert!(rendered.contains("MODO_CONTACTOS"));
        assert!(!rendered.contains("@@CONTACTS_"));
        assert!(!rendered.contains("@@BMUI1_"));

        let bmui1 = [
            "addAllowPattern",
            "removeAllowPattern",
            "setProtectionActive",
            "setBlockHidden",
            "activity",
            "uiContract",
            "versionInfo",
            "updatePolicy",
        ]
        .into_iter()
        .map(|name| format!("define i1 @contacts_{name}() {{ ret i1 true }}"))
        .collect::<Vec<_>>()
        .join("\n");
        blocklist.push_str(&bmui1);
        assert_eq!(
            validate_native_exports(&blocklist, "contacts").unwrap(),
            AndroidNativeProfile::BlocklistContactsBmui1
        );
        let rendered =
            AndroidNativeProfile::BlocklistContactsBmui1.render_jni("contacts", DEFAULT_PACKAGE_ID);
        assert!(rendered.contains("contacts_addAllowPattern"));
        assert!(rendered.contains("MODO_PROTECCION"));
        assert!(rendered.contains("ACTIVIDAD"));
        assert!(!rendered.contains("@@BMUI1_"));
    }
}
