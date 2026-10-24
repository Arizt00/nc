pub(crate) const JSON_RUNTIME_FUNCTIONS: &[&str] = &[
    "jsonParse",
    "jsonValidate",
    "jsonParseFile",
    "jsonStringify",
    "jsonStringifyPretty",
    "jsonSaveFile",
    "jsonClone",
    "jsonRelease",
    "jsonKind",
    "jsonIsNull",
    "jsonEquals",
    "jsonGetString",
    "jsonGetBool",
    "jsonGetInt",
    "jsonGetFloat",
    "jsonGetNumberText",
    "jsonGetStringOr",
    "jsonStringView",
    "jsonGetBoolOr",
    "jsonGetIntOr",
    "jsonGetFloatOr",
    "jsonArrayLen",
    "jsonArrayGet",
    "jsonArrayBorrow",
    "jsonArrayPush",
    "jsonArrayInsert",
    "jsonArrayRemove",
    "jsonObjectLen",
    "jsonObjectHas",
    "jsonObjectGet",
    "jsonObjectBorrow",
    "jsonObjectKeyAt",
    "jsonObjectSet",
    "jsonObjectRemove",
    "jsonPointer",
    "jsonNewNull",
    "jsonNewBool",
    "jsonNewInt",
    "jsonNewFloat",
    "jsonNewString",
    "jsonNewArray",
    "jsonNewObject",
    "jsonErrorCode",
    "jsonErrorName",
    "jsonErrorMessage",
    "jsonErrorLine",
    "jsonErrorColumn",
    "jsonErrorPath",
    "jsonClearError",
];

pub(crate) fn signature(name: &str) -> Option<(&'static [&'static str], &'static str)> {
    match name {
        "jsonParse" | "jsonParseFile" => Some((&["ptr"], "i64")),
        "jsonValidate" => Some((&["ptr"], "i1")),
        "jsonStringify" => Some((&["i64"], "ptr")),
        "jsonStringifyPretty" => Some((&["i64", "i64"], "ptr")),
        "jsonSaveFile" => Some((&["i64", "ptr", "i64"], "i1")),
        "jsonClone" | "jsonKind" | "jsonGetInt" | "jsonArrayLen" | "jsonObjectLen" => {
            Some((&["i64"], "i64"))
        }
        "jsonRelease" | "jsonIsNull" => Some((&["i64"], "i1")),
        "jsonEquals" => Some((&["i64", "i64"], "i1")),
        "jsonGetString" | "jsonGetNumberText" => Some((&["i64"], "ptr")),
        "jsonGetBool" => Some((&["i64"], "i1")),
        "jsonGetFloat" => Some((&["i64"], "double")),
        "jsonGetStringOr" | "jsonStringView" => Some((&["i64", "ptr"], "ptr")),
        "jsonGetBoolOr" => Some((&["i64", "i1"], "i1")),
        "jsonGetIntOr" => Some((&["i64", "i64"], "i64")),
        "jsonGetFloatOr" => Some((&["i64", "double"], "double")),
        "jsonArrayGet" | "jsonArrayBorrow" | "jsonArrayRemove" => Some((&["i64", "i64"], "i64")),
        "jsonArrayPush" => Some((&["i64", "i64"], "i1")),
        "jsonArrayInsert" => Some((&["i64", "i64", "i64"], "i1")),
        "jsonObjectHas" => Some((&["i64", "ptr"], "i1")),
        "jsonObjectGet" | "jsonObjectBorrow" | "jsonObjectRemove" | "jsonPointer" => {
            Some((&["i64", "ptr"], "i64"))
        }
        "jsonObjectKeyAt" => Some((&["i64", "i64"], "ptr")),
        "jsonObjectSet" => Some((&["i64", "ptr", "i64"], "i1")),
        "jsonNewNull" | "jsonNewArray" | "jsonNewObject" | "jsonErrorCode" | "jsonErrorLine"
        | "jsonErrorColumn" => Some((&[], "i64")),
        "jsonNewBool" => Some((&["i1"], "i64")),
        "jsonNewInt" => Some((&["i64"], "i64")),
        "jsonNewFloat" => Some((&["double"], "i64")),
        "jsonNewString" => Some((&["ptr"], "i64")),
        "jsonErrorName" | "jsonErrorMessage" | "jsonErrorPath" => Some((&[], "ptr")),
        "jsonClearError" => Some((&[], "i1")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_json_runtime_symbol_has_one_llvm_signature() {
        let mut unique = HashSet::new();
        for name in JSON_RUNTIME_FUNCTIONS {
            assert!(unique.insert(*name), "duplicate JSON runtime symbol {name}");
            let (params, result) =
                signature(name).unwrap_or_else(|| panic!("missing LLVM JSON signature for {name}"));
            assert!(params.iter().all(|ty| !ty.is_empty()));
            assert!(!result.is_empty());
        }
        assert_eq!(unique.len(), 49);
    }
}
