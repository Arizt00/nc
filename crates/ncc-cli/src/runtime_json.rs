use ncc_runtime::{JsonHandle, JsonRuntime, RuntimeValue, JSON_INVALID_HANDLE};

type BridgeResult = Result<RuntimeValue, String>;

pub(crate) fn eval_json_call(
    full_path: &str,
    args: &[RuntimeValue],
    runtime: &mut JsonRuntime,
) -> Option<BridgeResult> {
    let name = full_path
        .strip_prefix("runtime.json.")
        .unwrap_or(full_path)
        .to_ascii_lowercase();

    Some(match name.as_str() {
        "jsonparse" => string_arg(args, 0, "jsonParse")
            .map(|text| handle_value(runtime.parse(text).unwrap_or(JSON_INVALID_HANDLE))),
        "jsonvalidate" => string_arg(args, 0, "jsonValidate")
            .map(|text| RuntimeValue::Bool(runtime.validate(text).is_ok())),
        "jsonparsefile" => string_arg(args, 0, "jsonParseFile")
            .map(|path| handle_value(runtime.parse_file(path).unwrap_or(JSON_INVALID_HANDLE))),
        "jsonstringify" => handle_arg(args, 0, "jsonStringify")
            .map(|handle| RuntimeValue::String(runtime.stringify(handle).unwrap_or_default())),
        "jsonstringifypretty" => (|| {
            let handle = handle_arg(args, 0, "jsonStringifyPretty")?;
            let indent = index_arg(args, 1, "jsonStringifyPretty")?;
            Ok(RuntimeValue::String(
                runtime.stringify_indent(handle, indent).unwrap_or_default(),
            ))
        })(),
        "jsonsavefile" => (|| {
            let handle = handle_arg(args, 0, "jsonSaveFile")?;
            let path = string_arg(args, 1, "jsonSaveFile")?;
            let indent = index_arg(args, 2, "jsonSaveFile")?;
            Ok(RuntimeValue::Bool(
                runtime.save_file(handle, path, indent).is_ok(),
            ))
        })(),
        "jsonclone" => handle_arg(args, 0, "jsonClone").map(|handle| {
            handle_value(runtime.clone_handle(handle).unwrap_or(JSON_INVALID_HANDLE))
        }),
        "jsonrelease" => handle_arg(args, 0, "jsonRelease")
            .map(|handle| RuntimeValue::Bool(runtime.release(handle).is_ok())),
        "jsonkind" => handle_arg(args, 0, "jsonKind")
            .map(|handle| int_value(runtime.kind(handle).map(|kind| kind as i64).unwrap_or(0))),
        "jsonisnull" => handle_arg(args, 0, "jsonIsNull")
            .map(|handle| RuntimeValue::Bool(runtime.is_null(handle).unwrap_or(false))),
        "jsonequals" => (|| {
            let left = handle_arg(args, 0, "jsonEquals")?;
            let right = handle_arg(args, 1, "jsonEquals")?;
            Ok(RuntimeValue::Bool(
                runtime.equals(left, right).unwrap_or(false),
            ))
        })(),
        "jsongetstring" => handle_arg(args, 0, "jsonGetString")
            .map(|handle| RuntimeValue::String(runtime.as_string(handle).unwrap_or_default())),
        "jsongetbool" => handle_arg(args, 0, "jsonGetBool")
            .map(|handle| RuntimeValue::Bool(runtime.as_bool(handle).unwrap_or(false))),
        "jsongetint" => handle_arg(args, 0, "jsonGetInt")
            .map(|handle| int_value(runtime.as_i64(handle).unwrap_or(0))),
        "jsongetfloat" => handle_arg(args, 0, "jsonGetFloat")
            .map(|handle| float_value(runtime.as_f64(handle).unwrap_or(0.0))),
        "jsongetnumbertext" => handle_arg(args, 0, "jsonGetNumberText")
            .map(|handle| RuntimeValue::String(runtime.as_number_text(handle).unwrap_or_default())),
        "jsongetstringor" => (|| {
            let handle = handle_arg(args, 0, "jsonGetStringOr")?;
            let fallback = string_arg(args, 1, "jsonGetStringOr")?;
            Ok(RuntimeValue::String(
                runtime
                    .as_string(handle)
                    .unwrap_or_else(|_| fallback.to_string()),
            ))
        })(),
        "jsongetboolor" => (|| {
            let handle = handle_arg(args, 0, "jsonGetBoolOr")?;
            let fallback = bool_arg(args, 1, "jsonGetBoolOr")?;
            Ok(RuntimeValue::Bool(
                runtime.as_bool(handle).unwrap_or(fallback),
            ))
        })(),
        "jsongetintor" => (|| {
            let handle = handle_arg(args, 0, "jsonGetIntOr")?;
            let fallback = int_arg(args, 1, "jsonGetIntOr")?;
            Ok(int_value(runtime.as_i64(handle).unwrap_or(fallback)))
        })(),
        "jsongetfloator" => (|| {
            let handle = handle_arg(args, 0, "jsonGetFloatOr")?;
            let fallback = float_arg(args, 1, "jsonGetFloatOr")?;
            Ok(float_value(runtime.as_f64(handle).unwrap_or(fallback)))
        })(),
        "jsonarraylen" => handle_arg(args, 0, "jsonArrayLen")
            .map(|handle| int_value(runtime.array_len(handle).unwrap_or(0) as i64)),
        "jsonarrayget" => (|| {
            let handle = handle_arg(args, 0, "jsonArrayGet")?;
            let index = index_arg(args, 1, "jsonArrayGet")?;
            Ok(handle_value(
                runtime
                    .array_get(handle, index)
                    .unwrap_or(JSON_INVALID_HANDLE),
            ))
        })(),
        "jsonarraypush" => (|| {
            let array = handle_arg(args, 0, "jsonArrayPush")?;
            let value = handle_arg(args, 1, "jsonArrayPush")?;
            Ok(RuntimeValue::Bool(runtime.array_push(array, value).is_ok()))
        })(),
        "jsonarrayinsert" => (|| {
            let array = handle_arg(args, 0, "jsonArrayInsert")?;
            let index = index_arg(args, 1, "jsonArrayInsert")?;
            let value = handle_arg(args, 2, "jsonArrayInsert")?;
            Ok(RuntimeValue::Bool(
                runtime.array_insert(array, index, value).is_ok(),
            ))
        })(),
        "jsonarrayremove" => (|| {
            let array = handle_arg(args, 0, "jsonArrayRemove")?;
            let index = index_arg(args, 1, "jsonArrayRemove")?;
            Ok(handle_value(
                runtime
                    .array_remove(array, index)
                    .unwrap_or(JSON_INVALID_HANDLE),
            ))
        })(),
        "jsonobjectlen" => handle_arg(args, 0, "jsonObjectLen")
            .map(|handle| int_value(runtime.object_len(handle).unwrap_or(0) as i64)),
        "jsonobjecthas" => (|| {
            let object = handle_arg(args, 0, "jsonObjectHas")?;
            let key = string_arg(args, 1, "jsonObjectHas")?;
            Ok(RuntimeValue::Bool(
                runtime.object_has(object, key).unwrap_or(false),
            ))
        })(),
        "jsonobjectget" => (|| {
            let object = handle_arg(args, 0, "jsonObjectGet")?;
            let key = string_arg(args, 1, "jsonObjectGet")?;
            Ok(handle_value(
                runtime
                    .object_get(object, key)
                    .unwrap_or(JSON_INVALID_HANDLE),
            ))
        })(),
        "jsonobjectkeyat" => (|| {
            let object = handle_arg(args, 0, "jsonObjectKeyAt")?;
            let index = index_arg(args, 1, "jsonObjectKeyAt")?;
            Ok(RuntimeValue::String(
                runtime.object_key_at(object, index).unwrap_or_default(),
            ))
        })(),
        "jsonobjectset" => (|| {
            let object = handle_arg(args, 0, "jsonObjectSet")?;
            let key = string_arg(args, 1, "jsonObjectSet")?;
            let value = handle_arg(args, 2, "jsonObjectSet")?;
            Ok(RuntimeValue::Bool(
                runtime.object_set(object, key, value).is_ok(),
            ))
        })(),
        "jsonobjectremove" => (|| {
            let object = handle_arg(args, 0, "jsonObjectRemove")?;
            let key = string_arg(args, 1, "jsonObjectRemove")?;
            Ok(handle_value(
                runtime
                    .object_remove(object, key)
                    .unwrap_or(JSON_INVALID_HANDLE),
            ))
        })(),
        "jsonpointer" => (|| {
            let root = handle_arg(args, 0, "jsonPointer")?;
            let pointer = string_arg(args, 1, "jsonPointer")?;
            Ok(handle_value(
                runtime
                    .pointer(root, pointer)
                    .unwrap_or(JSON_INVALID_HANDLE),
            ))
        })(),
        "jsonnewnull" => Ok(handle_value(
            runtime.new_null().unwrap_or(JSON_INVALID_HANDLE),
        )),
        "jsonnewbool" => bool_arg(args, 0, "jsonNewBool")
            .map(|value| handle_value(runtime.new_bool(value).unwrap_or(JSON_INVALID_HANDLE))),
        "jsonnewint" => int_arg(args, 0, "jsonNewInt")
            .map(|value| handle_value(runtime.new_i64(value).unwrap_or(JSON_INVALID_HANDLE))),
        "jsonnewfloat" => float_arg(args, 0, "jsonNewFloat")
            .map(|value| handle_value(runtime.new_f64(value).unwrap_or(JSON_INVALID_HANDLE))),
        "jsonnewstring" => string_arg(args, 0, "jsonNewString")
            .map(|value| handle_value(runtime.new_string(value).unwrap_or(JSON_INVALID_HANDLE))),
        "jsonnewarray" => Ok(handle_value(
            runtime.new_array().unwrap_or(JSON_INVALID_HANDLE),
        )),
        "jsonnewobject" => Ok(handle_value(
            runtime.new_object().unwrap_or(JSON_INVALID_HANDLE),
        )),
        "jsonerrorcode" => Ok(int_value(runtime.last_error_code() as i64)),
        "jsonerrorname" => Ok(RuntimeValue::String(
            runtime.last_error_code().stable_code().to_string(),
        )),
        "jsonerrormessage" => Ok(RuntimeValue::String(
            runtime.last_error_message().to_string(),
        )),
        "jsonerrorline" => Ok(int_value(runtime.last_error_line() as i64)),
        "jsonerrorcolumn" => Ok(int_value(runtime.last_error_column() as i64)),
        "jsonerrorpath" => Ok(RuntimeValue::String(runtime.last_error_path().to_string())),
        "jsonclearerror" => {
            runtime.clear_error();
            Ok(RuntimeValue::Bool(true))
        }
        _ => return None,
    })
}

fn handle_value(value: JsonHandle) -> RuntimeValue {
    RuntimeValue::Number(value.to_string())
}

fn int_value(value: i64) -> RuntimeValue {
    RuntimeValue::Number(value.to_string())
}

fn float_value(value: f64) -> RuntimeValue {
    RuntimeValue::Number(value.to_string())
}

fn string_arg<'a>(args: &'a [RuntimeValue], index: usize, call: &str) -> Result<&'a str, String> {
    match args.get(index) {
        Some(RuntimeValue::String(value)) => Ok(value),
        _ => Err(format!(
            "E_RUNTIME_JSON_ABI_001: {call} argument {} must be str",
            index + 1
        )),
    }
}

fn bool_arg(args: &[RuntimeValue], index: usize, call: &str) -> Result<bool, String> {
    match args.get(index) {
        Some(RuntimeValue::Bool(value)) => Ok(*value),
        _ => Err(format!(
            "E_RUNTIME_JSON_ABI_001: {call} argument {} must be bool",
            index + 1
        )),
    }
}

fn int_arg(args: &[RuntimeValue], index: usize, call: &str) -> Result<i64, String> {
    match args.get(index) {
        Some(RuntimeValue::Number(value)) => value.parse::<i64>().map_err(|_| {
            format!(
                "E_RUNTIME_JSON_ABI_001: {call} argument {} must be int",
                index + 1
            )
        }),
        _ => Err(format!(
            "E_RUNTIME_JSON_ABI_001: {call} argument {} must be int",
            index + 1
        )),
    }
}

fn float_arg(args: &[RuntimeValue], index: usize, call: &str) -> Result<f64, String> {
    match args.get(index) {
        Some(RuntimeValue::Number(value)) => value.parse::<f64>().map_err(|_| {
            format!(
                "E_RUNTIME_JSON_ABI_001: {call} argument {} must be ft",
                index + 1
            )
        }),
        _ => Err(format!(
            "E_RUNTIME_JSON_ABI_001: {call} argument {} must be ft",
            index + 1
        )),
    }
}

fn handle_arg(args: &[RuntimeValue], index: usize, call: &str) -> Result<JsonHandle, String> {
    let value = int_arg(args, index, call)?;
    JsonHandle::try_from(value).map_err(|_| {
        format!(
            "E_RUNTIME_JSON_ABI_001: {call} argument {} must be a non-negative handle",
            index + 1
        )
    })
}

fn index_arg(args: &[RuntimeValue], index: usize, call: &str) -> Result<usize, String> {
    let value = int_arg(args, index, call)?;
    usize::try_from(value).map_err(|_| {
        format!(
            "E_RUNTIME_JSON_ABI_001: {call} argument {} must be a non-negative index",
            index + 1
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncc_runtime::JsonErrorCode;

    fn call(runtime: &mut JsonRuntime, name: &str, args: &[RuntimeValue]) -> RuntimeValue {
        eval_json_call(name, args, runtime)
            .unwrap_or_else(|| panic!("{name} must be recognized"))
            .unwrap_or_else(|error| panic!("{name} failed: {error}"))
    }

    fn int(value: i64) -> RuntimeValue {
        RuntimeValue::Number(value.to_string())
    }

    fn handle(value: &RuntimeValue) -> i64 {
        match value {
            RuntimeValue::Number(value) => value.parse().unwrap(),
            other => panic!("expected handle number, got {other:?}"),
        }
    }

    #[test]
    fn bridge_parses_traverses_and_reports_errors_without_aborting() {
        let mut runtime = JsonRuntime::default();
        let root = call(
            &mut runtime,
            "runtime.json.jsonParse",
            &[RuntimeValue::String(r#"{"items":[8]}"#.into())],
        );
        let items = call(
            &mut runtime,
            "runtime.json.jsonObjectGet",
            &[root.clone(), RuntimeValue::String("items".into())],
        );
        let value = call(&mut runtime, "runtime.json.jsonArrayGet", &[items, int(0)]);
        assert_eq!(
            handle(&call(&mut runtime, "runtime.json.jsonGetInt", &[value])),
            8
        );

        let missing = call(
            &mut runtime,
            "runtime.json.jsonObjectGet",
            &[root, RuntimeValue::String("missing".into())],
        );
        assert_eq!(handle(&missing), 0);
        assert_eq!(
            handle(&call(&mut runtime, "runtime.json.jsonErrorCode", &[])),
            JsonErrorCode::MissingKey as i64
        );
        assert!(matches!(
            call(&mut runtime, "runtime.json.jsonErrorName", &[]),
            RuntimeValue::String(text) if text == "E_RUNTIME_JSON_KEY_001"
        ));
        let message = call(&mut runtime, "runtime.json.jsonErrorMessage", &[]);
        assert!(matches!(message, RuntimeValue::String(text) if text.contains("missing")));
    }

    #[test]
    fn bridge_rejects_abi_type_mismatch() {
        let mut runtime = JsonRuntime::default();
        let error = eval_json_call(
            "runtime.json.jsonParse",
            &[RuntimeValue::Bool(true)],
            &mut runtime,
        )
        .unwrap()
        .unwrap_err();
        assert!(error.contains("E_RUNTIME_JSON_ABI_001"));
    }
}
