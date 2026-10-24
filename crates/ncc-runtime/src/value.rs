use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Human,
    Debug,
    Inspect,
    Stable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBinaryProvider {
    PortableExact,
    ExternalLibcPercentB,
}

impl RenderBinaryProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PortableExact => "portable-exact",
            Self::ExternalLibcPercentB => "external-libc-percent-b",
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimePointer {
    Null,
    Addr(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeClosureHandle {
    pub id: u64,
    pub param_count: usize,
    pub capture_count: usize,
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Void,
    Bool(bool),
    Number(String),
    Char(char),
    String(String),
    Pointer(RuntimePointer),
    Vector(Vec<RuntimeValue>),
    Record(HashMap<String, RuntimeValue>),
    MemRegionHandle(usize),
    Closure(RuntimeClosureHandle),
}

fn escape_runtime_string(raw: &str) -> String {
    raw.chars()
        .flat_map(|c| match c {
            '\\' => "\\\\".chars().collect::<Vec<char>>(),
            '"' => "\\\"".chars().collect::<Vec<char>>(),
            '\n' => "\\n".chars().collect::<Vec<char>>(),
            '\r' => "\\r".chars().collect::<Vec<char>>(),
            '\t' => "\\t".chars().collect::<Vec<char>>(),
            _ => vec![c],
        })
        .collect()
}

pub fn render_value(v: &RuntimeValue) -> String {
    render_value_with_mode(v, RenderMode::Human)
}

pub fn render_binary_i64(value: i64) -> String {
    render_binary_i64_with_provider(value, render_binary_provider())
}

pub fn render_binary_i64_with_provider(value: i64, provider: RenderBinaryProvider) -> String {
    match provider {
        RenderBinaryProvider::ExternalLibcPercentB => render_binary_i64_with_libc_percent_b(value)
            .unwrap_or_else(|| render_binary_i64_portable(value)),
        RenderBinaryProvider::PortableExact => render_binary_i64_portable(value),
    }
}

pub fn render_binary_provider() -> RenderBinaryProvider {
    match std::env::var("NCC_RENDER_BINARY_PROVIDER")
        .or_else(|_| std::env::var("NC_RENDER_BINARY_PROVIDER"))
        .unwrap_or_else(|_| "auto".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "portable" | "portable-exact" | "exact" => RenderBinaryProvider::PortableExact,
        "libc" | "libc-percent-b" | "external-libc-percent-b" | "%b" => {
            if libc_percent_b_supported() {
                RenderBinaryProvider::ExternalLibcPercentB
            } else {
                RenderBinaryProvider::PortableExact
            }
        }
        _ => {
            if libc_percent_b_supported() {
                RenderBinaryProvider::ExternalLibcPercentB
            } else {
                RenderBinaryProvider::PortableExact
            }
        }
    }
}

pub fn render_binary_i64_portable(value: i64) -> String {
    if value == 0 {
        return "0".to_string();
    }

    let negative = value < 0;
    let mut magnitude = signed_binary_magnitude(value);
    let mut bytes = Vec::with_capacity(65);

    while magnitude != 0 {
        bytes.push(if (magnitude & 1) == 1 { b'1' } else { b'0' });
        magnitude >>= 1;
    }
    if negative {
        bytes.push(b'-');
    }
    bytes.reverse();

    String::from_utf8(bytes).expect("binary render emits ASCII bytes only")
}

fn signed_binary_magnitude(value: i64) -> u64 {
    if value < 0 {
        value.wrapping_neg() as u64
    } else {
        value as u64
    }
}

fn libc_percent_b_supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(libc_percent_b_supported_uncached)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn libc_percent_b_supported_uncached() -> bool {
    let mut buf = [0 as libc::c_char; 8];
    let written = unsafe {
        libc::snprintf(
            buf.as_mut_ptr(),
            buf.len(),
            b"%llb\0".as_ptr().cast(),
            5 as libc::c_ulonglong,
        )
    };
    if written != 3 {
        return false;
    }
    let Ok(rendered) = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_str() else {
        return false;
    };
    rendered == "101"
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn libc_percent_b_supported_uncached() -> bool {
    false
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn render_binary_i64_with_libc_percent_b(value: i64) -> Option<String> {
    if !libc_percent_b_supported() {
        return None;
    }

    let negative = value < 0;
    let magnitude = signed_binary_magnitude(value);
    let mut buf = [0 as libc::c_char; 72];
    let written = unsafe {
        libc::snprintf(
            buf.as_mut_ptr(),
            buf.len(),
            b"%llb\0".as_ptr().cast(),
            magnitude as libc::c_ulonglong,
        )
    };
    if written <= 0 || written as usize >= buf.len() {
        return None;
    }

    let body = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
        .to_str()
        .ok()?;
    if body.is_empty()
        || !body
            .as_bytes()
            .iter()
            .all(|byte| matches!(*byte, b'0' | b'1'))
    {
        return None;
    }

    if negative {
        Some(format!("-{body}"))
    } else {
        Some(body.to_string())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn render_binary_i64_with_libc_percent_b(_value: i64) -> Option<String> {
    None
}

pub fn render_value_with_mode(v: &RuntimeValue, mode: RenderMode) -> String {
    match v {
        RuntimeValue::Void => match mode {
            RenderMode::Human | RenderMode::Stable => "none".to_string(),
            RenderMode::Debug => "void(none)".to_string(),
            RenderMode::Inspect => "void".to_string(),
        },
        RuntimeValue::Bool(b) => match mode {
            RenderMode::Human | RenderMode::Stable => b.to_string(),
            RenderMode::Debug => format!("bool({b})"),
            RenderMode::Inspect => format!("bool:{b}"),
        },
        RuntimeValue::Number(n) => match mode {
            RenderMode::Human | RenderMode::Stable => n.clone(),
            RenderMode::Debug => format!("num({n})"),
            RenderMode::Inspect => format!("num:{n}"),
        },
        RuntimeValue::Char(ch) => match mode {
            RenderMode::Human => ch.to_string(),
            RenderMode::Stable => format!("'{}'", escape_runtime_string(&ch.to_string())),
            RenderMode::Debug => format!("char('{}')", escape_runtime_string(&ch.to_string())),
            RenderMode::Inspect => format!("char:{}", u32::from(*ch)),
        },
        RuntimeValue::String(s) => {
            let escaped = escape_runtime_string(s);
            match mode {
                RenderMode::Human => s.clone(),
                RenderMode::Stable => format!("\"{}\"", escaped),
                RenderMode::Debug => format!("str(\"{}\")", escaped),
                RenderMode::Inspect => format!("str:\"{}\"", escaped),
            }
        }
        RuntimeValue::Pointer(RuntimePointer::Null) => match mode {
            RenderMode::Human | RenderMode::Stable => "null".to_string(),
            RenderMode::Debug => "ptr(null)".to_string(),
            RenderMode::Inspect => "ptr:null".to_string(),
        },
        RuntimeValue::Pointer(RuntimePointer::Addr(addr)) => match mode {
            RenderMode::Human => addr.clone(),
            RenderMode::Stable => format!("addr({addr})"),
            RenderMode::Debug => format!("ptr({addr})"),
            RenderMode::Inspect => format!("ptr:{addr}"),
        },
        RuntimeValue::Vector(items) => {
            let rendered: Vec<String> = items
                .iter()
                .map(|it| render_value_with_mode(it, mode))
                .collect();
            match mode {
                RenderMode::Human | RenderMode::Stable => format!("[{}]", rendered.join(", ")),
                RenderMode::Debug => format!("vec(len={} [{}])", items.len(), rendered.join(", ")),
                RenderMode::Inspect => {
                    format!(
                        "vector(len={}, value=[{}])",
                        items.len(),
                        rendered.join(", ")
                    )
                }
            }
        }
        RuntimeValue::Record(fields) => {
            let mut keys: Vec<&String> = fields.keys().collect();
            keys.sort();
            let rendered: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    let value = render_value_with_mode(fields.get(k).unwrap(), mode);
                    format!("{k}: {value}")
                })
                .collect();
            match mode {
                RenderMode::Human | RenderMode::Stable => format!("{{{}}}", rendered.join(", ")),
                RenderMode::Debug => {
                    format!("rec(len={} {{{}}})", fields.len(), rendered.join(", "))
                }
                RenderMode::Inspect => {
                    format!(
                        "record(len={}, value={{{}}})",
                        fields.len(),
                        rendered.join(", ")
                    )
                }
            }
        }
        RuntimeValue::MemRegionHandle(id) => match mode {
            RenderMode::Human | RenderMode::Inspect => format!("<memRegion#{}>", id),
            RenderMode::Stable => format!("memRegion#{}", id),
            RenderMode::Debug => format!("memRegion(id={id})"),
        },
        RuntimeValue::Closure(handle) => match mode {
            RenderMode::Human => format!("<closure#{}>", handle.id),
            RenderMode::Stable => format!(
                "closure#{}(params={},captures={})",
                handle.id, handle.param_count, handle.capture_count
            ),
            RenderMode::Debug => format!(
                "closure(id={}, params={}, captures={})",
                handle.id, handle.param_count, handle.capture_count
            ),
            RenderMode::Inspect => format!(
                "closure:id={},params={},captures={}",
                handle.id, handle.param_count, handle.capture_count
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        render_binary_i64, render_binary_i64_portable, render_binary_i64_with_provider,
        render_binary_provider, render_value, render_value_with_mode, RenderBinaryProvider,
        RenderMode, RuntimePointer, RuntimeValue,
    };
    use std::collections::HashMap;

    #[test]
    fn render_binary_emits_exact_ascii_digits() {
        for (value, expected) in [
            (0, "0"),
            (1, "1"),
            (5, "101"),
            (42, "101010"),
            (-5, "-101"),
            (
                i64::MIN,
                "-1000000000000000000000000000000000000000000000000000000000000000",
            ),
        ] {
            let rendered = render_binary_i64_portable(value);
            assert_eq!(rendered, expected);
            assert!(rendered.bytes().all(|b| matches!(b, b'-' | b'0' | b'1')));
        }
    }

    #[test]
    fn render_binary_provider_keeps_language_contract() {
        let provider = render_binary_provider();
        assert!(matches!(
            provider,
            RenderBinaryProvider::PortableExact | RenderBinaryProvider::ExternalLibcPercentB
        ));
        assert_eq!(render_binary_i64(42), "101010");
        assert_eq!(render_binary_i64(-5), "-101");
        assert_eq!(provider.as_str(), render_binary_provider().as_str());
    }

    #[test]
    fn render_binary_external_provider_falls_back_to_exact_when_needed() {
        assert_eq!(
            render_binary_i64_with_provider(42, RenderBinaryProvider::ExternalLibcPercentB),
            render_binary_i64_portable(42)
        );
    }

    #[test]
    fn render_scalars_and_null_pointer() {
        assert_eq!(render_value(&RuntimeValue::Bool(true)), "true");
        assert_eq!(render_value(&RuntimeValue::Number("42".into())), "42");
        assert_eq!(render_value(&RuntimeValue::Char('A')), "A");
        assert_eq!(
            render_value_with_mode(&RuntimeValue::Char('A'), RenderMode::Stable),
            "'A'"
        );
        assert_eq!(
            render_value_with_mode(&RuntimeValue::Char('A'), RenderMode::Debug),
            "char('A')"
        );
        assert_eq!(render_value(&RuntimeValue::String("hola".into())), "hola");
        assert_eq!(
            render_value(&RuntimeValue::Pointer(RuntimePointer::Null)),
            "null"
        );
    }

    #[test]
    fn render_vector_record_and_mem_region() {
        let vec_val = RuntimeValue::Vector(vec![
            RuntimeValue::Number("1".into()),
            RuntimeValue::Number("2".into()),
            RuntimeValue::Number("3".into()),
        ]);
        assert_eq!(render_value(&vec_val), "[1, 2, 3]");

        let mut fields = HashMap::new();
        fields.insert("b".to_string(), RuntimeValue::Number("2".into()));
        fields.insert("a".to_string(), RuntimeValue::Bool(true));
        let rec = RuntimeValue::Record(fields);
        assert_eq!(render_value(&rec), "{a: true, b: 2}");

        assert_eq!(
            render_value(&RuntimeValue::MemRegionHandle(9)),
            "<memRegion#9>"
        );
    }

    #[test]
    fn render_modes_are_distinct_and_stable() {
        let value = RuntimeValue::Vector(vec![
            RuntimeValue::String("hola".into()),
            RuntimeValue::Bool(true),
            RuntimeValue::Pointer(RuntimePointer::Addr("0x1000".into())),
        ]);

        assert_eq!(
            render_value_with_mode(&value, RenderMode::Human),
            "[hola, true, 0x1000]"
        );
        assert_eq!(
            render_value_with_mode(&value, RenderMode::Debug),
            "vec(len=3 [str(\"hola\"), bool(true), ptr(0x1000)])"
        );
        assert_eq!(
            render_value_with_mode(&value, RenderMode::Inspect),
            "vector(len=3, value=[str:\"hola\", bool:true, ptr:0x1000])"
        );
        assert_eq!(
            render_value_with_mode(&value, RenderMode::Stable),
            "[\"hola\", true, addr(0x1000)]"
        );
    }

    #[test]
    fn render_stable_record_order_is_deterministic() {
        let mut fields = HashMap::new();
        fields.insert("z".to_string(), RuntimeValue::Number("2".into()));
        fields.insert("a".to_string(), RuntimeValue::Number("1".into()));
        let rec = RuntimeValue::Record(fields);
        assert_eq!(
            render_value_with_mode(&rec, RenderMode::Stable),
            "{a: 1, z: 2}"
        );
    }

    #[test]
    fn render_debug_escapes_string_payload() {
        let raw = RuntimeValue::String("line1\n\"line2\"".into());
        assert_eq!(
            render_value_with_mode(&raw, RenderMode::Debug),
            "str(\"line1\\n\\\"line2\\\"\")"
        );
        assert_eq!(
            render_value_with_mode(&raw, RenderMode::Stable),
            "\"line1\\n\\\"line2\\\"\""
        );
    }
}
