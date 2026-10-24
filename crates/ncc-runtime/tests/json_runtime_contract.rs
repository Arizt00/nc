use ncc_runtime::{JsonErrorCode, JsonKind, JsonLimits, JsonRuntime};

#[test]
fn public_json_runtime_roundtrip_and_typed_traversal() {
    let mut json = JsonRuntime::default();
    let root = json
        .parse(r#"{"symbol":"Cm","number":96,"synthetic":true,"uses":["research"]}"#)
        .expect("JSON parse");
    assert_eq!(json.kind(root).unwrap(), JsonKind::Object);
    let symbol = json.pointer(root, "/symbol").unwrap();
    let number = json.pointer(root, "/number").unwrap();
    let synthetic = json.pointer(root, "/synthetic").unwrap();
    assert_eq!(json.as_string(symbol).unwrap(), "Cm");
    assert_eq!(json.as_i64(number).unwrap(), 96);
    assert!(json.as_bool(synthetic).unwrap());

    let compact = json.stringify(root).unwrap();
    let reparsed = json.parse(&compact).unwrap();
    assert!(json.equals(root, reparsed).unwrap());
}

#[test]
fn public_json_runtime_failure_is_queryable_and_recoverable() {
    let mut json = JsonRuntime::default();
    assert_eq!(
        json.parse("{broken").unwrap_err().code,
        JsonErrorCode::Parse
    );
    assert_eq!(json.last_error_code(), JsonErrorCode::Parse);
    assert!(json.last_error_line() > 0);
    assert!(json.last_error_column() > 0);
    json.clear_error();
    assert_eq!(json.last_error_code(), JsonErrorCode::None);
}

#[test]
fn public_json_runtime_honours_explicit_resource_limits() {
    let mut json = JsonRuntime::new(JsonLimits {
        max_input_bytes: 16,
        max_depth: 2,
        max_nodes: 4,
        max_handles: 2,
    });
    assert_eq!(
        json.parse(r#"{"payload":"too long"}"#).unwrap_err().code,
        JsonErrorCode::LimitExceeded
    );
    let first = json.new_object().unwrap();
    let second = json.new_null().unwrap();
    assert_eq!(
        json.clone_handle(first).unwrap_err().code,
        JsonErrorCode::LimitExceeded
    );
    json.release(second).unwrap();
    assert!(json.clone_handle(first).is_ok());
}
