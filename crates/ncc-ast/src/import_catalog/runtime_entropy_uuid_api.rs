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

pub(super) fn runtime_entropy_uuid_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.entropy".to_string(),
            module_path: "runtime.entropy".to_string(),
            functions: vec![
                function("entropyFill", &["ptr<byte>", "u64"], "bool"),
                function("entropyAvailable", &[], "bool"),
                // Compatibility surface. New code owns its buffer and uses entropyFill.
                function(
                    "entropyBytes",
                    &["u64"],
                    "Result<memRegion<byte>, EntropyError>",
                ),
                function(
                    "entropyRelease",
                    &["Result<memRegion<byte>, EntropyError>"],
                    "bool",
                ),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.uuid".to_string(),
            module_path: "runtime.uuid".to_string(),
            functions: vec![
                function(
                    "uuidFromBytesInto",
                    &["ptr<byte>", "ptr<byte>", "u64"],
                    "int",
                ),
                function("uuidV4Into", &["ptr<byte>", "u64"], "int"),
                function("isUuid", &["str"], "bool"),
                // Compatibility surface. The returned string must be released explicitly.
                function("uuidV4", &[], "str"),
                function("uuidFromBytes", &["array<byte,16>"], "str"),
                function("uuidRelease", &["str"], "bool"),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_entropy_uuid_surface_is_typed_and_caller_owned() {
        let families = runtime_entropy_uuid_api_family_specs();
        assert_eq!(families.len(), 2);
        let entropy = &families[0];
        let uuid = &families[1];
        assert!(entropy.functions.iter().any(|function| {
            function.name == "entropyFill"
                && function.arg_types == ["ptr<byte>", "u64"]
                && function.return_ty == "bool"
        }));
        assert!(uuid.functions.iter().any(|function| {
            function.name == "uuidV4Into"
                && function.arg_types == ["ptr<byte>", "u64"]
                && function.return_ty == "int"
        }));
    }
}
