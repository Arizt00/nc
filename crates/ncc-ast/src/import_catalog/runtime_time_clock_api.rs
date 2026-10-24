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

pub(super) fn runtime_time_clock_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.time".to_string(),
            module_path: "runtime.time".to_string(),
            functions: vec![
                function("now", &[], "int"),
                function("nowMs", &[], "int"),
                function("timestamp", &[], "int"),
                function("monotonicMs", &[], "int"),
                function("monotonicNs", &[], "int"),
                function("sleep", &["int"], "void"),
                function("sleepUntilMs", &["int"], "bool"),
                function("formatTimeInto", &["int", "ptr<byte>", "u64"], "int"),
                function("dateInto", &["ptr<byte>", "u64"], "int"),
                // Compatibility surface. Returned text is released with timeRelease.
                function("formatTime", &["int"], "str"),
                function("date", &[], "str"),
                function("timeRelease", &["str"], "bool"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.clock".to_string(),
            module_path: "runtime.clock".to_string(),
            functions: vec![
                function("monotonic", &[], "u64"),
                function("elapsed", &["u64"], "u64"),
                function("measure", &["fn() -> bool"], "u64"),
                function("clockResolution", &[], "u64"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.sleep".to_string(),
            module_path: "runtime.sleep".to_string(),
            functions: vec![
                function("sleepMs", &["int"], "void"),
                function("sleepSec", &["int"], "void"),
                function("yieldRuntime", &[], "void"),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_clock_surface_is_typed_manual_and_import_free_ready() {
        let families = runtime_time_clock_api_family_specs();
        assert_eq!(families.len(), 3);
        assert!(families[0].functions.iter().any(|function| {
            function.name == "formatTimeInto"
                && function.arg_types == ["int", "ptr<byte>", "u64"]
                && function.return_ty == "int"
        }));
        assert!(families[1].functions.iter().any(|function| {
            function.name == "measure"
                && function.arg_types == ["fn() -> bool"]
                && function.return_ty == "u64"
        }));
        assert!(families[2].functions.iter().any(|function| {
            function.name == "sleepMs"
                && function.arg_types == ["int"]
                && function.return_ty == "void"
        }));
    }
}
