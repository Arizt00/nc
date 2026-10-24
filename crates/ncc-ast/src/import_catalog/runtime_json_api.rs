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

pub(super) fn runtime_json_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.json".to_string(),
        module_path: "runtime.json".to_string(),
        functions: vec![
            function("jsonParse", &["str"], "int"),
            function("jsonValidate", &["str"], "bool"),
            function("jsonParseFile", &["str"], "int"),
            function("jsonStringify", &["int"], "str"),
            function("jsonStringifyPretty", &["int", "int"], "str"),
            function("jsonSaveFile", &["int", "str", "int"], "bool"),
            function("jsonClone", &["int"], "int"),
            function("jsonRelease", &["int"], "bool"),
            function("jsonKind", &["int"], "int"),
            function("jsonIsNull", &["int"], "bool"),
            function("jsonEquals", &["int", "int"], "bool"),
            function("jsonGetString", &["int"], "str"),
            function("jsonGetBool", &["int"], "bool"),
            function("jsonGetInt", &["int"], "int"),
            function("jsonGetFloat", &["int"], "ft"),
            function("jsonGetNumberText", &["int"], "str"),
            function("jsonGetStringOr", &["int", "str"], "str"),
            function("jsonStringView", &["int", "str"], "str"),
            function("jsonGetBoolOr", &["int", "bool"], "bool"),
            function("jsonGetIntOr", &["int", "int"], "int"),
            function("jsonGetFloatOr", &["int", "ft"], "ft"),
            function("jsonArrayLen", &["int"], "int"),
            function("jsonArrayGet", &["int", "int"], "int"),
            function("jsonArrayBorrow", &["int", "int"], "int"),
            function("jsonArrayPush", &["int", "int"], "bool"),
            function("jsonArrayInsert", &["int", "int", "int"], "bool"),
            function("jsonArrayRemove", &["int", "int"], "int"),
            function("jsonObjectLen", &["int"], "int"),
            function("jsonObjectHas", &["int", "str"], "bool"),
            function("jsonObjectGet", &["int", "str"], "int"),
            function("jsonObjectBorrow", &["int", "str"], "int"),
            function("jsonObjectKeyAt", &["int", "int"], "str"),
            function("jsonObjectSet", &["int", "str", "int"], "bool"),
            function("jsonObjectRemove", &["int", "str"], "int"),
            function("jsonPointer", &["int", "str"], "int"),
            function("jsonNewNull", &[], "int"),
            function("jsonNewBool", &["bool"], "int"),
            function("jsonNewInt", &["int"], "int"),
            function("jsonNewFloat", &["ft"], "int"),
            function("jsonNewString", &["str"], "int"),
            function("jsonNewArray", &[], "int"),
            function("jsonNewObject", &[], "int"),
            function("jsonErrorCode", &[], "int"),
            function("jsonErrorName", &[], "str"),
            function("jsonErrorMessage", &[], "str"),
            function("jsonErrorLine", &[], "int"),
            function("jsonErrorColumn", &[], "int"),
            function("jsonErrorPath", &[], "str"),
            function("jsonClearError", &[], "bool"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn json_api_names_are_unique_and_signatures_are_typed() {
        let family = runtime_json_api_family_spec();
        assert_eq!(family.import_path, "nC.runtime.json");
        assert_eq!(family.module_path, "runtime.json");
        assert_eq!(family.functions.len(), 49);
        let mut names = HashSet::new();
        for function in family.functions {
            assert!(names.insert(function.name.clone()), "duplicate JSON symbol");
            assert_eq!(function.arity_min, function.arg_types.len());
            assert!(!function.variadic);
            assert!(!function.return_ty.is_empty());
        }
    }
}
