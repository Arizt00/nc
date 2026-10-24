use serde::Serialize;
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub type JsonHandle = u64;
pub const JSON_INVALID_HANDLE: JsonHandle = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum JsonKind {
    Invalid = 0,
    Null = 1,
    Bool = 2,
    Number = 3,
    String = 4,
    Array = 5,
    Object = 6,
}

impl JsonKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::Null => "null",
            Self::Bool => "bool",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum JsonErrorCode {
    None = 0,
    Parse = 1,
    Io = 2,
    InvalidHandle = 3,
    TypeMismatch = 4,
    MissingKey = 5,
    IndexOutOfBounds = 6,
    InvalidPointer = 7,
    NonFiniteNumber = 8,
    LimitExceeded = 9,
    Serialize = 10,
}

impl JsonErrorCode {
    pub fn stable_code(self) -> &'static str {
        match self {
            Self::None => "OK",
            Self::Parse => "E_RUNTIME_JSON_PARSE_001",
            Self::Io => "E_RUNTIME_JSON_IO_001",
            Self::InvalidHandle => "E_RUNTIME_JSON_HANDLE_001",
            Self::TypeMismatch => "E_RUNTIME_JSON_TYPE_001",
            Self::MissingKey => "E_RUNTIME_JSON_KEY_001",
            Self::IndexOutOfBounds => "E_RUNTIME_JSON_INDEX_001",
            Self::InvalidPointer => "E_RUNTIME_JSON_POINTER_001",
            Self::NonFiniteNumber => "E_RUNTIME_JSON_NUMBER_001",
            Self::LimitExceeded => "E_RUNTIME_JSON_LIMIT_001",
            Self::Serialize => "E_RUNTIME_JSON_SERIALIZE_001",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub code: JsonErrorCode,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub path: String,
}

impl JsonError {
    fn new(code: JsonErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            line: 0,
            column: 0,
            path: String::new(),
        }
    }

    fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    fn from_parse(error: serde_json::Error) -> Self {
        Self {
            code: JsonErrorCode::Parse,
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
            path: String::new(),
        }
    }

    pub fn stable_code(&self) -> &'static str {
        self.code.stable_code()
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.stable_code(), self.message)?;
        if self.line != 0 || self.column != 0 {
            write!(f, " at {}:{}", self.line, self.column)?;
        }
        if !self.path.is_empty() {
            write!(f, " ({})", self.path)?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonError {}

pub type JsonResult<T> = Result<T, JsonError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLimits {
    pub max_input_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_handles: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024 * 1024,
            max_depth: 128,
            max_nodes: 1_000_000,
            max_handles: 65_536,
        }
    }
}

/// Owned JSON arena used by the nC runtime bridge.
///
/// Handles never expose Rust pointers. Child traversal returns an owned clone,
/// so mutation and release cannot invalidate another live handle. Handle zero
/// is permanently invalid and every failure is retained as structured state.
#[derive(Debug, Clone)]
pub struct JsonRuntime {
    values: HashMap<JsonHandle, Value>,
    next_handle: JsonHandle,
    last_error: Option<JsonError>,
    limits: JsonLimits,
}

impl Default for JsonRuntime {
    fn default() -> Self {
        Self::new(JsonLimits::default())
    }
}

impl JsonRuntime {
    pub fn new(limits: JsonLimits) -> Self {
        Self {
            values: HashMap::new(),
            next_handle: 1,
            last_error: None,
            limits,
        }
    }

    pub fn limits(&self) -> JsonLimits {
        self.limits
    }

    pub fn active_handles(&self) -> usize {
        self.values.len()
    }

    pub fn last_error(&self) -> Option<&JsonError> {
        self.last_error.as_ref()
    }

    pub fn last_error_code(&self) -> JsonErrorCode {
        self.last_error
            .as_ref()
            .map(|error| error.code)
            .unwrap_or(JsonErrorCode::None)
    }

    pub fn last_error_message(&self) -> &str {
        self.last_error
            .as_ref()
            .map(|error| error.message.as_str())
            .unwrap_or("")
    }

    pub fn last_error_line(&self) -> usize {
        self.last_error.as_ref().map_or(0, |error| error.line)
    }

    pub fn last_error_column(&self) -> usize {
        self.last_error.as_ref().map_or(0, |error| error.column)
    }

    pub fn last_error_path(&self) -> &str {
        self.last_error
            .as_ref()
            .map(|error| error.path.as_str())
            .unwrap_or("")
    }

    pub fn clear_error(&mut self) {
        self.last_error = None;
    }

    pub fn parse(&mut self, text: &str) -> JsonResult<JsonHandle> {
        let parsed = self.parse_value(text);
        match parsed {
            Ok(value) => self.insert(value),
            Err(error) => self.fail(error),
        }
    }

    pub fn validate(&mut self, text: &str) -> JsonResult<()> {
        match self.parse_value(text) {
            Ok(_) => self.success(()),
            Err(error) => self.fail(error),
        }
    }

    pub fn parse_file(&mut self, path: impl AsRef<Path>) -> JsonResult<JsonHandle> {
        let path = path.as_ref();
        let result = (|| {
            let metadata = std::fs::metadata(path).map_err(|error| {
                JsonError::new(
                    JsonErrorCode::Io,
                    format!("cannot inspect JSON file {}: {error}", path.display()),
                )
                .at_path(path.display().to_string())
            })?;
            let declared_len = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
            self.check_input_len(declared_len)?;
            let mut file = File::open(path).map_err(|error| {
                JsonError::new(
                    JsonErrorCode::Io,
                    format!("cannot open JSON file {}: {error}", path.display()),
                )
                .at_path(path.display().to_string())
            })?;
            let mut text = String::with_capacity(declared_len.min(self.limits.max_input_bytes));
            file.read_to_string(&mut text).map_err(|error| {
                JsonError::new(
                    JsonErrorCode::Io,
                    format!("cannot read UTF-8 JSON file {}: {error}", path.display()),
                )
                .at_path(path.display().to_string())
            })?;
            self.parse_value(&text)
                .map_err(|error| error.at_path(path.display().to_string()))
        })();
        match result {
            Ok(value) => self.insert(value),
            Err(error) => self.fail(error),
        }
    }

    pub fn stringify(&mut self, handle: JsonHandle) -> JsonResult<String> {
        self.stringify_indent(handle, 0)
    }

    pub fn stringify_indent(&mut self, handle: JsonHandle, indent: usize) -> JsonResult<String> {
        let result = (|| {
            let value = self.value(handle)?;
            if indent == 0 {
                serde_json::to_string(value)
                    .map_err(|error| JsonError::new(JsonErrorCode::Serialize, error.to_string()))
            } else {
                let indent = indent.clamp(1, 8);
                let spaces = vec![b' '; indent];
                let formatter = serde_json::ser::PrettyFormatter::with_indent(&spaces);
                let mut bytes = Vec::new();
                let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
                value
                    .serialize(&mut serializer)
                    .map_err(|error| JsonError::new(JsonErrorCode::Serialize, error.to_string()))?;
                String::from_utf8(bytes)
                    .map_err(|error| JsonError::new(JsonErrorCode::Serialize, error.to_string()))
            }
        })();
        self.record(result)
    }

    pub fn save_file(
        &mut self,
        handle: JsonHandle,
        path: impl AsRef<Path>,
        indent: usize,
    ) -> JsonResult<()> {
        let path = path.as_ref();
        let rendered = self.stringify_indent(handle, indent)?;
        let result = write_atomic(path, rendered.as_bytes()).map_err(|error| {
            JsonError::new(
                JsonErrorCode::Io,
                format!("cannot save JSON file {}: {error}", path.display()),
            )
            .at_path(path.display().to_string())
        });
        self.record(result)
    }

    pub fn release(&mut self, handle: JsonHandle) -> JsonResult<()> {
        if handle == JSON_INVALID_HANDLE || self.values.remove(&handle).is_none() {
            return self.fail(invalid_handle_error(handle));
        }
        self.success(())
    }

    pub fn clone_handle(&mut self, handle: JsonHandle) -> JsonResult<JsonHandle> {
        let value = match self.value(handle) {
            Ok(value) => value.clone(),
            Err(error) => return self.fail(error),
        };
        self.insert(value)
    }

    pub fn kind(&mut self, handle: JsonHandle) -> JsonResult<JsonKind> {
        let result = self.value(handle).map(value_kind);
        self.record(result)
    }

    pub fn is_null(&mut self, handle: JsonHandle) -> JsonResult<bool> {
        let result = self.value(handle).map(Value::is_null);
        self.record(result)
    }

    pub fn equals(&mut self, left: JsonHandle, right: JsonHandle) -> JsonResult<bool> {
        let result = (|| Ok(self.value(left)? == self.value(right)?))();
        self.record(result)
    }

    pub fn as_string(&mut self, handle: JsonHandle) -> JsonResult<String> {
        let result = self.value(handle).and_then(|value| match value {
            Value::String(value) => Ok(value.clone()),
            other => Err(type_error("string", value_kind(other))),
        });
        self.record(result)
    }

    pub fn as_bool(&mut self, handle: JsonHandle) -> JsonResult<bool> {
        let result = self.value(handle).and_then(|value| {
            value
                .as_bool()
                .ok_or_else(|| type_error("bool", value_kind(value)))
        });
        self.record(result)
    }

    pub fn as_i64(&mut self, handle: JsonHandle) -> JsonResult<i64> {
        let result = self.value(handle).and_then(|value| {
            value
                .as_i64()
                .ok_or_else(|| type_error("integer number", value_kind(value)))
        });
        self.record(result)
    }

    pub fn as_f64(&mut self, handle: JsonHandle) -> JsonResult<f64> {
        let result = self.value(handle).and_then(|value| {
            value
                .as_f64()
                .ok_or_else(|| type_error("number", value_kind(value)))
        });
        self.record(result)
    }

    pub fn as_number_text(&mut self, handle: JsonHandle) -> JsonResult<String> {
        let result = self.value(handle).and_then(|value| match value {
            Value::Number(value) => Ok(value.to_string()),
            other => Err(type_error("number", value_kind(other))),
        });
        self.record(result)
    }

    pub fn array_len(&mut self, handle: JsonHandle) -> JsonResult<usize> {
        let result = self.array(handle).map(Vec::len);
        self.record(result)
    }

    pub fn array_get(&mut self, handle: JsonHandle, index: usize) -> JsonResult<JsonHandle> {
        let value = match self.array(handle).and_then(|array| {
            array.get(index).cloned().ok_or_else(|| {
                JsonError::new(
                    JsonErrorCode::IndexOutOfBounds,
                    format!("array index {index} is out of bounds (len {})", array.len()),
                )
                .at_path(format!("/{index}"))
            })
        }) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        self.insert(value)
    }

    pub fn array_push(&mut self, handle: JsonHandle, value: JsonHandle) -> JsonResult<()> {
        let child = match self.value(value) {
            Ok(value) => value.clone(),
            Err(error) => return self.fail(error),
        };
        let result = (|| {
            let mut next = self.array(handle)?.clone();
            next.push(child);
            validate_tree_limits(&Value::Array(next.clone()), self.limits)?;
            *self.array_mut(handle)? = next;
            Ok(())
        })();
        self.record(result)
    }

    pub fn array_insert(
        &mut self,
        handle: JsonHandle,
        index: usize,
        value: JsonHandle,
    ) -> JsonResult<()> {
        let child = match self.value(value) {
            Ok(value) => value.clone(),
            Err(error) => return self.fail(error),
        };
        let result = (|| {
            let mut next = self.array(handle)?.clone();
            if index > next.len() {
                return Err(JsonError::new(
                    JsonErrorCode::IndexOutOfBounds,
                    format!("array insertion index {index} exceeds len {}", next.len()),
                ));
            }
            next.insert(index, child);
            validate_tree_limits(&Value::Array(next.clone()), self.limits)?;
            *self.array_mut(handle)? = next;
            Ok(())
        })();
        self.record(result)
    }

    pub fn array_remove(&mut self, handle: JsonHandle, index: usize) -> JsonResult<JsonHandle> {
        let value = match self.array_mut(handle).and_then(|array| {
            if index >= array.len() {
                Err(JsonError::new(
                    JsonErrorCode::IndexOutOfBounds,
                    format!("array index {index} is out of bounds (len {})", array.len()),
                ))
            } else {
                Ok(array.remove(index))
            }
        }) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        self.insert(value)
    }

    pub fn object_len(&mut self, handle: JsonHandle) -> JsonResult<usize> {
        let result = self.object(handle).map(Map::len);
        self.record(result)
    }

    pub fn object_has(&mut self, handle: JsonHandle, key: &str) -> JsonResult<bool> {
        let result = self.object(handle).map(|object| object.contains_key(key));
        self.record(result)
    }

    pub fn object_get(&mut self, handle: JsonHandle, key: &str) -> JsonResult<JsonHandle> {
        let value = match self.object(handle).and_then(|object| {
            object.get(key).cloned().ok_or_else(|| {
                JsonError::new(
                    JsonErrorCode::MissingKey,
                    format!("JSON object does not contain key {key:?}"),
                )
                .at_path(format!("/{}", escape_pointer_token(key)))
            })
        }) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        self.insert(value)
    }

    pub fn object_key_at(&mut self, handle: JsonHandle, index: usize) -> JsonResult<String> {
        let result = self.object(handle).and_then(|object| {
            object.keys().nth(index).cloned().ok_or_else(|| {
                JsonError::new(
                    JsonErrorCode::IndexOutOfBounds,
                    format!(
                        "object key index {index} is out of bounds (len {})",
                        object.len()
                    ),
                )
            })
        });
        self.record(result)
    }

    pub fn object_set(
        &mut self,
        handle: JsonHandle,
        key: impl Into<String>,
        value: JsonHandle,
    ) -> JsonResult<()> {
        let child = match self.value(value) {
            Ok(value) => value.clone(),
            Err(error) => return self.fail(error),
        };
        let key = key.into();
        let result = (|| {
            let mut next = self.object(handle)?.clone();
            next.insert(key, child);
            validate_tree_limits(&Value::Object(next.clone()), self.limits)?;
            *self.object_mut(handle)? = next;
            Ok(())
        })();
        self.record(result)
    }

    pub fn object_remove(&mut self, handle: JsonHandle, key: &str) -> JsonResult<JsonHandle> {
        let value = match self.object_mut(handle).and_then(|object| {
            object.remove(key).ok_or_else(|| {
                JsonError::new(
                    JsonErrorCode::MissingKey,
                    format!("JSON object does not contain key {key:?}"),
                )
            })
        }) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        self.insert(value)
    }

    /// RFC 6901 JSON Pointer traversal. Empty pointer addresses the root.
    pub fn pointer(&mut self, handle: JsonHandle, pointer: &str) -> JsonResult<JsonHandle> {
        let root = match self.value(handle) {
            Ok(value) => value,
            Err(error) => return self.fail(error),
        };
        let value = match resolve_pointer(root, pointer) {
            Ok(value) => value.clone(),
            Err(error) => return self.fail(error),
        };
        self.insert(value)
    }

    pub fn new_null(&mut self) -> JsonResult<JsonHandle> {
        self.insert(Value::Null)
    }

    pub fn new_bool(&mut self, value: bool) -> JsonResult<JsonHandle> {
        self.insert(Value::Bool(value))
    }

    pub fn new_i64(&mut self, value: i64) -> JsonResult<JsonHandle> {
        self.insert(Value::Number(Number::from(value)))
    }

    pub fn new_f64(&mut self, value: f64) -> JsonResult<JsonHandle> {
        let Some(number) = Number::from_f64(value) else {
            return self.fail(JsonError::new(
                JsonErrorCode::NonFiniteNumber,
                "JSON numbers cannot be NaN or infinite",
            ));
        };
        self.insert(Value::Number(number))
    }

    pub fn new_string(&mut self, value: impl Into<String>) -> JsonResult<JsonHandle> {
        self.insert(Value::String(value.into()))
    }

    pub fn new_array(&mut self) -> JsonResult<JsonHandle> {
        self.insert(Value::Array(Vec::new()))
    }

    pub fn new_object(&mut self) -> JsonResult<JsonHandle> {
        self.insert(Value::Object(Map::new()))
    }

    fn parse_value(&self, text: &str) -> JsonResult<Value> {
        self.check_input_len(text.len())?;
        let value: Value = serde_json::from_str(text).map_err(JsonError::from_parse)?;
        validate_tree_limits(&value, self.limits)?;
        Ok(value)
    }

    fn check_input_len(&self, len: usize) -> JsonResult<()> {
        if len > self.limits.max_input_bytes {
            return Err(JsonError::new(
                JsonErrorCode::LimitExceeded,
                format!(
                    "JSON input has {len} bytes; limit is {}",
                    self.limits.max_input_bytes
                ),
            ));
        }
        Ok(())
    }

    fn insert(&mut self, value: Value) -> JsonResult<JsonHandle> {
        if self.values.len() >= self.limits.max_handles {
            return self.fail(JsonError::new(
                JsonErrorCode::LimitExceeded,
                format!("JSON handle limit {} reached", self.limits.max_handles),
            ));
        }
        validate_tree_limits(&value, self.limits).map_err(|error| {
            self.last_error = Some(error.clone());
            error
        })?;
        let start = self.next_handle.max(1);
        let mut handle = start;
        loop {
            if !self.values.contains_key(&handle) {
                break;
            }
            handle = handle.wrapping_add(1).max(1);
            if handle == start {
                return self.fail(JsonError::new(
                    JsonErrorCode::LimitExceeded,
                    "JSON handle space exhausted",
                ));
            }
        }
        self.values.insert(handle, value);
        self.next_handle = handle.wrapping_add(1).max(1);
        self.success(handle)
    }

    fn value(&self, handle: JsonHandle) -> JsonResult<&Value> {
        self.values
            .get(&handle)
            .ok_or_else(|| invalid_handle_error(handle))
    }

    fn array(&self, handle: JsonHandle) -> JsonResult<&Vec<Value>> {
        let value = self.value(handle)?;
        value
            .as_array()
            .ok_or_else(|| type_error("array", value_kind(value)))
    }

    fn array_mut(&mut self, handle: JsonHandle) -> JsonResult<&mut Vec<Value>> {
        let value = self
            .values
            .get_mut(&handle)
            .ok_or_else(|| invalid_handle_error(handle))?;
        let kind = value_kind(value);
        value
            .as_array_mut()
            .ok_or_else(|| type_error("array", kind))
    }

    fn object(&self, handle: JsonHandle) -> JsonResult<&Map<String, Value>> {
        let value = self.value(handle)?;
        value
            .as_object()
            .ok_or_else(|| type_error("object", value_kind(value)))
    }

    fn object_mut(&mut self, handle: JsonHandle) -> JsonResult<&mut Map<String, Value>> {
        let value = self
            .values
            .get_mut(&handle)
            .ok_or_else(|| invalid_handle_error(handle))?;
        let kind = value_kind(value);
        value
            .as_object_mut()
            .ok_or_else(|| type_error("object", kind))
    }

    fn success<T>(&mut self, value: T) -> JsonResult<T> {
        self.last_error = None;
        Ok(value)
    }

    fn fail<T>(&mut self, error: JsonError) -> JsonResult<T> {
        self.last_error = Some(error.clone());
        Err(error)
    }

    fn record<T>(&mut self, result: JsonResult<T>) -> JsonResult<T> {
        match result {
            Ok(value) => self.success(value),
            Err(error) => self.fail(error),
        }
    }
}

fn value_kind(value: &Value) -> JsonKind {
    match value {
        Value::Null => JsonKind::Null,
        Value::Bool(_) => JsonKind::Bool,
        Value::Number(_) => JsonKind::Number,
        Value::String(_) => JsonKind::String,
        Value::Array(_) => JsonKind::Array,
        Value::Object(_) => JsonKind::Object,
    }
}

fn invalid_handle_error(handle: JsonHandle) -> JsonError {
    JsonError::new(
        JsonErrorCode::InvalidHandle,
        format!("unknown or released JSON handle {handle}"),
    )
}

fn type_error(expected: &str, actual: JsonKind) -> JsonError {
    JsonError::new(
        JsonErrorCode::TypeMismatch,
        format!("expected JSON {expected}, got {}", actual.as_str()),
    )
}

fn validate_tree_limits(value: &Value, limits: JsonLimits) -> JsonResult<()> {
    let mut stack = vec![(value, 1usize)];
    let mut nodes = 0usize;
    while let Some((value, depth)) = stack.pop() {
        nodes = nodes.saturating_add(1);
        if nodes > limits.max_nodes {
            return Err(JsonError::new(
                JsonErrorCode::LimitExceeded,
                format!("JSON node limit {} exceeded", limits.max_nodes),
            ));
        }
        if depth > limits.max_depth {
            return Err(JsonError::new(
                JsonErrorCode::LimitExceeded,
                format!("JSON nesting depth limit {} exceeded", limits.max_depth),
            ));
        }
        match value {
            Value::Array(values) => {
                stack.extend(values.iter().map(|value| (value, depth.saturating_add(1))));
            }
            Value::Object(values) => {
                stack.extend(
                    values
                        .values()
                        .map(|value| (value, depth.saturating_add(1))),
                );
            }
            _ => {}
        }
    }
    Ok(())
}

fn resolve_pointer<'a>(root: &'a Value, pointer: &str) -> JsonResult<&'a Value> {
    if pointer.is_empty() {
        return Ok(root);
    }
    if !pointer.starts_with('/') {
        return Err(JsonError::new(
            JsonErrorCode::InvalidPointer,
            "RFC 6901 JSON Pointer must be empty or start with '/'",
        )
        .at_path(pointer));
    }
    let mut current = root;
    let mut traversed = String::new();
    for encoded in pointer[1..].split('/') {
        let token = decode_pointer_token(encoded).map_err(|error| error.at_path(pointer))?;
        traversed.push('/');
        traversed.push_str(encoded);
        current = match current {
            Value::Object(object) => object.get(&token).ok_or_else(|| {
                JsonError::new(
                    JsonErrorCode::MissingKey,
                    format!("JSON object does not contain key {token:?}"),
                )
                .at_path(traversed.clone())
            })?,
            Value::Array(array) => {
                if token.is_empty()
                    || token == "-"
                    || (token.len() > 1 && token.starts_with('0'))
                    || !token.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(JsonError::new(
                        JsonErrorCode::InvalidPointer,
                        format!("invalid JSON array index {token:?}"),
                    )
                    .at_path(traversed));
                }
                let index = token.parse::<usize>().map_err(|_| {
                    JsonError::new(
                        JsonErrorCode::InvalidPointer,
                        format!("JSON array index {token:?} does not fit usize"),
                    )
                    .at_path(traversed.clone())
                })?;
                array.get(index).ok_or_else(|| {
                    JsonError::new(
                        JsonErrorCode::IndexOutOfBounds,
                        format!("array index {index} is out of bounds (len {})", array.len()),
                    )
                    .at_path(traversed.clone())
                })?
            }
            other => {
                return Err(type_error(
                    "object or array during pointer traversal",
                    value_kind(other),
                )
                .at_path(traversed))
            }
        };
    }
    Ok(current)
}

fn decode_pointer_token(token: &str) -> JsonResult<String> {
    let mut decoded = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(character) = chars.next() {
        if character != '~' {
            decoded.push(character);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            Some(other) => {
                return Err(JsonError::new(
                    JsonErrorCode::InvalidPointer,
                    format!("invalid JSON Pointer escape ~{other}"),
                ))
            }
            None => {
                return Err(JsonError::new(
                    JsonErrorCode::InvalidPointer,
                    "incomplete JSON Pointer escape '~'",
                ))
            }
        }
    }
    Ok(decoded)
}

fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("json");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temp_path = PathBuf::from(parent);
    temp_path.push(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce));

    let write_result = (|| {
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        temp.write_all(bytes)?;
        temp.flush()?;
        temp.sync_all()?;
        drop(temp);

        match std::fs::rename(&temp_path, path) {
            Ok(()) => Ok(()),
            Err(first) if path.exists() => {
                std::fs::remove_file(path)?;
                std::fs::rename(&temp_path, path).map_err(|_| first)
            }
            Err(error) => Err(error),
        }
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_traverse_and_preserve_typed_values() {
        let mut runtime = JsonRuntime::default();
        let root = runtime
            .parse(r#"{"name":"Oxígeno","atomic":8,"stable":true,"ions":[-2,0]}"#)
            .expect("valid JSON");
        assert_eq!(runtime.kind(root).unwrap(), JsonKind::Object);
        assert!(runtime.object_has(root, "name").unwrap());
        assert_eq!(runtime.object_len(root).unwrap(), 4);

        let name = runtime.object_get(root, "name").unwrap();
        assert_eq!(runtime.as_string(name).unwrap(), "Oxígeno");
        let atomic = runtime.pointer(root, "/atomic").unwrap();
        assert_eq!(runtime.as_i64(atomic).unwrap(), 8);
        let ion = runtime.pointer(root, "/ions/0").unwrap();
        assert_eq!(runtime.as_i64(ion).unwrap(), -2);
        assert_eq!(runtime.last_error_code(), JsonErrorCode::None);
    }

    #[test]
    fn parse_error_retains_stable_code_and_location() {
        let mut runtime = JsonRuntime::default();
        let error = runtime.parse("{\n  \"x\": ]\n}").unwrap_err();
        assert_eq!(error.code, JsonErrorCode::Parse);
        assert_eq!(error.stable_code(), "E_RUNTIME_JSON_PARSE_001");
        assert!(error.line >= 2);
        assert!(error.column > 0);
        assert_eq!(runtime.last_error_code(), JsonErrorCode::Parse);
        assert!(runtime.last_error_message().contains("expected"));
    }

    #[test]
    fn object_array_build_mutate_remove_and_stringify() {
        let mut runtime = JsonRuntime::default();
        let object = runtime.new_object().unwrap();
        let array = runtime.new_array().unwrap();
        let one = runtime.new_i64(1).unwrap();
        let two = runtime.new_i64(2).unwrap();
        runtime.array_push(array, one).unwrap();
        runtime.array_push(array, two).unwrap();
        runtime.object_set(object, "values", array).unwrap();

        let child = runtime.object_get(object, "values").unwrap();
        assert_eq!(runtime.array_len(child).unwrap(), 2);
        let removed = runtime.array_remove(child, 0).unwrap();
        assert_eq!(runtime.as_i64(removed).unwrap(), 1);
        // Traversal is owned: mutating child never mutates its parent.
        assert_eq!(runtime.stringify(object).unwrap(), r#"{"values":[1,2]}"#);
        assert_eq!(runtime.stringify_indent(child, 4).unwrap(), "[\n    2\n]");
    }

    #[test]
    fn pointer_decodes_rfc6901_tokens_and_rejects_bad_indices() {
        let mut runtime = JsonRuntime::default();
        let root = runtime.parse(r#"{"a/b":{"~key":["ok"]}}"#).unwrap();
        let leaf = runtime.pointer(root, "/a~1b/~0key/0").unwrap();
        assert_eq!(runtime.as_string(leaf).unwrap(), "ok");

        let error = runtime.pointer(root, "/a~1b/~0key/01").unwrap_err();
        assert_eq!(error.code, JsonErrorCode::InvalidPointer);
        assert_eq!(error.path, "/a~1b/~0key/01");
    }

    #[test]
    fn invalid_handles_types_keys_and_indices_are_structured() {
        let mut runtime = JsonRuntime::default();
        assert_eq!(
            runtime.kind(999).unwrap_err().code,
            JsonErrorCode::InvalidHandle
        );
        let object = runtime.parse(r#"{"x":1}"#).unwrap();
        assert_eq!(
            runtime.as_bool(object).unwrap_err().code,
            JsonErrorCode::TypeMismatch
        );
        assert_eq!(
            runtime.object_get(object, "missing").unwrap_err().code,
            JsonErrorCode::MissingKey
        );
        let array = runtime.parse("[]").unwrap();
        assert_eq!(
            runtime.array_get(array, 0).unwrap_err().code,
            JsonErrorCode::IndexOutOfBounds
        );
    }

    #[test]
    fn finite_number_and_resource_limits_are_enforced() {
        let mut runtime = JsonRuntime::default();
        assert_eq!(
            runtime.new_f64(f64::NAN).unwrap_err().code,
            JsonErrorCode::NonFiniteNumber
        );

        let mut limited = JsonRuntime::new(JsonLimits {
            max_input_bytes: 8,
            max_depth: 2,
            max_nodes: 3,
            max_handles: 1,
        });
        assert_eq!(
            limited.parse(r#"{"long":1}"#).unwrap_err().code,
            JsonErrorCode::LimitExceeded
        );
        limited.parse("null").unwrap();
        assert_eq!(
            limited.new_null().unwrap_err().code,
            JsonErrorCode::LimitExceeded
        );

        let mut node_limited = JsonRuntime::new(JsonLimits {
            max_input_bytes: 128,
            max_depth: 2,
            max_nodes: 2,
            max_handles: 8,
        });
        let array = node_limited.new_array().unwrap();
        let one = node_limited.new_i64(1).unwrap();
        node_limited.array_push(array, one).unwrap();
        let two = node_limited.new_i64(2).unwrap();
        assert_eq!(
            node_limited.array_push(array, two).unwrap_err().code,
            JsonErrorCode::LimitExceeded
        );
        assert_eq!(node_limited.array_len(array).unwrap(), 1);
    }

    #[test]
    fn file_roundtrip_is_utf8_pretty_and_atomic_at_api_boundary() {
        let mut runtime = JsonRuntime::default();
        let root = runtime.parse(r#"{"element":"Curio","number":96}"#).unwrap();
        let path = std::env::temp_dir().join(format!(
            "ncc_runtime_json_{}_{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        runtime.save_file(root, &path, 2).unwrap();
        let rendered = std::fs::read_to_string(&path).unwrap();
        assert!(rendered.contains("\n  \"element\""));
        let loaded = runtime.parse_file(&path).unwrap();
        assert!(runtime.equals(root, loaded).unwrap());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn release_is_explicit_and_clone_survives() {
        let mut runtime = JsonRuntime::default();
        let original = runtime.parse("[1]").unwrap();
        let cloned = runtime.clone_handle(original).unwrap();
        runtime.release(original).unwrap();
        assert_eq!(runtime.kind(cloned).unwrap(), JsonKind::Array);
        assert_eq!(
            runtime.kind(original).unwrap_err().code,
            JsonErrorCode::InvalidHandle
        );
    }
}
