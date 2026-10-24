use super::{RuntimeApiFamilySpec, RuntimeApiFunctionSpec};

fn function(name: &str, args: &[&str], return_ty: &str) -> RuntimeApiFunctionSpec {
    RuntimeApiFunctionSpec {
        name: name.to_string(),
        arg_types: args.iter().map(|arg| (*arg).to_string()).collect(),
        arity_min: args.len(),
        variadic: false,
        return_ty: return_ty.to_string(),
    }
}

pub(super) fn runtime_mobile_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.telephony".to_string(),
            module_path: "runtime.telephony".to_string(),
            functions: vec![
                function("getIncomingNumber", &[], "str"),
                function("getState", &[], "str"),
                function("isAvailable", &[], "bool"),
                function("dial", &["str"], "bool"),
                function("answer", &[], "bool"),
                function("hangUp", &[], "bool"),
                function("rejectCall", &[], "bool"),
                function("adb_connect", &["int"], "int"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.tts".to_string(),
            module_path: "runtime.tts".to_string(),
            functions: vec![
                function("connect", &["str"], "bool"),
                function("speak", &["str"], "bool"),
                function("stop", &[], "bool"),
                function("isSpeaking", &[], "bool"),
                function("disconnect", &[], "bool"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.mobileNetwork".to_string(),
            module_path: "runtime.mobileNetwork".to_string(),
            functions: vec![
                function("request", &["str"], "MobileResponse"),
                function("responseStatus", &["MobileResponse"], "int"),
                function("responseJson", &["MobileResponse", "str"], "str"),
                function("responseBody", &["MobileResponse"], "str"),
                function("responseRelease", &["MobileResponse"], "bool"),
                function("isWiFiConnected", &[], "bool"),
                function("conectarServidor", &[], "bool"),
                function("server_open", &["int"], "NetworkServer"),
                function("serverReceive", &["NetworkServer"], "str"),
                function("serverSend", &["NetworkServer", "str"], "int"),
                function("serverClose", &["NetworkServer"], "bool"),
                function("client_open", &["str", "int"], "NetworkClient"),
                function("clientSend", &["NetworkClient", "str"], "int"),
                function("clientReceive", &["NetworkClient"], "str"),
                function("clientClose", &["NetworkClient"], "bool"),
                function("send_bytes", &["str"], "int"),
                function("receive_bytes", &[], "str"),
                function("sendBytesRaw", &["addr", "int"], "int"),
                function("receiveBytesInto", &["addr", "int"], "int"),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn mobile_api_is_unique_and_fully_typed() {
        let families = runtime_mobile_api_family_specs();
        assert_eq!(families.len(), 3);
        for family in families {
            let mut names = HashSet::new();
            for function in family.functions {
                assert!(names.insert(function.name));
                assert_eq!(function.arity_min, function.arg_types.len());
                assert!(!function.variadic);
                assert!(!function.return_ty.is_empty());
            }
        }
    }
}
