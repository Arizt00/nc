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

pub(super) fn runtime_sys_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.permission".to_string(),
            module_path: "runtime.permission".to_string(),
            functions: vec![
                function("request", &["array<any>"], "bool"),
                function("cargarPermisos", &[], "bool"),
                function("canRead", &["str"], "bool"),
                function("canWrite", &["str"], "bool"),
                function("canExecute", &["str"], "bool"),
                function("requirePermission", &["str"], "result"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.system".to_string(),
            module_path: "runtime.system".to_string(),
            functions: vec![
                function("cpu_load", &[], "ft64"),
                function("registry_write", &["str", "str"], "bool"),
                function("registry_read", &["str"], "str"),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sys_and_permission_calls_are_fully_typed() {
        let families = runtime_sys_api_family_specs();
        let family = families
            .iter()
            .find(|family| family.import_path == "nC.runtime.permission")
            .expect("permission family");
        let request = family
            .functions
            .iter()
            .find(|function| function.name == "request")
            .expect("permission request");
        assert_eq!(request.arg_types, ["array<any>"]);
        assert_eq!(request.return_ty, "bool");
        let bootstrap = family
            .functions
            .iter()
            .find(|function| function.name == "cargarPermisos")
            .expect("mobile permission bootstrap");
        assert!(bootstrap.arg_types.is_empty());
        assert_eq!(bootstrap.return_ty, "bool");

        let system = families
            .iter()
            .find(|family| family.import_path == "nC.runtime.system")
            .expect("system family");
        assert_eq!(system.functions.len(), 3);
        assert_eq!(system.functions[0].return_ty, "ft64");
        assert_eq!(system.functions[1].arg_types, ["str", "str"]);
        assert_eq!(system.functions[2].return_ty, "str");
    }
}
