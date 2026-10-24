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

pub(super) fn runtime_atomic_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.atomic".to_string(),
        module_path: "runtime.atomic".to_string(),
        functions: vec![
            // Compatibility calls are permanently seq_cst.
            function(
                "atomicCreate",
                &["int"],
                "Result<AtomicHandle, ThreadError>",
            ),
            function("atomicLoad", &["any"], "Result<int, ThreadError>"),
            function("atomicStore", &["any", "int"], "Result<bool, ThreadError>"),
            function("atomicAdd", &["any", "int"], "Result<int, ThreadError>"),
            function(
                "compareExchange",
                &["any", "int", "int"],
                "Result<bool, ThreadError>",
            ),
            function("atomicFence", &[], "void"),
            // Ordered C11 surface. Orders use canonical strings:
            // relaxed, acquire, release, acq_rel and seq_cst.
            function(
                "atomicLoadOrder",
                &["any", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicStoreOrder",
                &["any", "int", "str"],
                "Result<bool, ThreadError>",
            ),
            function(
                "atomicExchange",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicFetchAdd",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicFetchSub",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicFetchAnd",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicFetchOr",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicFetchXor",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicCompareExchangeStrong",
                &["any", "int", "int", "str", "str"],
                "Result<int, ThreadError>",
            ),
            function(
                "atomicCompareExchangeWeak",
                &["any", "int", "int", "str", "str"],
                "Result<int, ThreadError>",
            ),
            function("atomicThreadFence", &["str"], "void"),
            function("atomicCompilerFence", &["str"], "void"),
            function(
                "atomicWait",
                &["any", "int", "str"],
                "Result<int, ThreadError>",
            ),
            function("atomicNotifyOne", &["any"], "int"),
            function("atomicNotifyAll", &["any"], "int"),
            function("volatileLoad", &["any"], "Result<int, ThreadError>"),
            function(
                "volatileStore",
                &["any", "int"],
                "Result<bool, ThreadError>",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn c11_atomic_api_is_unique_and_fully_typed() {
        let family = runtime_atomic_api_family_spec();
        assert_eq!(family.functions.len(), 23);
        let mut names = HashSet::new();
        for function in family.functions {
            assert!(names.insert(function.name));
            assert_eq!(function.arity_min, function.arg_types.len());
            assert!(!function.variadic);
            assert!(!function.return_ty.is_empty());
        }
    }
}
