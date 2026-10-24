use std::fmt::{Display, Formatter};

pub const DIAGNOSTIC_ABI_MAGIC: u32 = u32::from_le_bytes(*b"NCDI");
pub const DIAGNOSTIC_ABI_VERSION: u16 = 2;
pub const DIAGNOSTIC_RECORD_BYTES: usize = 128;
pub const NDIAG_MAGIC: [u8; 8] = *b"NDIAG\0\0\0";
pub const NDIAG_VERSION: u16 = 1;
pub const NDIAG_HEADER_BYTES: usize = 32;
pub const NDIAG_FRAME_MAGIC: u32 = u32::from_le_bytes(*b"NDFR");
pub const NDIAG_FRAME_HEADER_BYTES: usize = 16;
pub const DIAGNOSTIC_REGISTRY_SCHEMA: &str = "nc.diagnostic.registry.v1";
pub const DIAGNOSTIC_REGISTRY_VERSION: u16 = 1;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }

    pub fn canonical(&self, default_phase: &str) -> CanonicalDiagnostic {
        CanonicalDiagnostic::from_legacy(&self.message, default_phase, self.line, self.column)
    }

    pub fn stage0_registered(
        code: &str,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        assert!(
            diagnostic_registered(code),
            "diagnostic code '{code}' is absent from diagnostics/registry.json"
        );
        Self::new(format!("{code}: {}", message.into()), line, column)
    }

    pub fn render_source(&self, default_phase: &str, file: &str, source: &str) -> String {
        self.canonical(default_phase).with_file(file).render(source)
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Note = 0,
    Help = 1,
    Warning = 2,
    Error = 3,
    Fatal = 4,
}

impl DiagnosticSeverity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Help => "help",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Note),
            1 => Some(Self::Help),
            2 => Some(Self::Warning),
            3 => Some(Self::Error),
            4 => Some(Self::Fatal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagnosticDomain {
    Primitive = 0,
    Basal = 1,
    Machine = 2,
    Lexer = 3,
    Parser = 4,
    Sema = 5,
    Ir = 6,
    Mct = 7,
    Linker = 8,
    Runtime = 9,
    Kernel = 10,
    HighLevel = 11,
    Tooling = 12,
    Ghor = 13,
    Unknown = 255,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticDefinition {
    pub code: &'static str,
    pub phase: &'static str,
    pub summary: &'static str,
    pub help: &'static str,
}

include!(concat!(
    env!("OUT_DIR"),
    "/diagnostic_registry_generated.rs"
));

pub fn diagnostic_definition(code: &str) -> Option<&'static DiagnosticDefinition> {
    DIAGNOSTIC_REGISTRY
        .binary_search_by_key(&code, |entry| entry.code)
        .ok()
        .map(|index| &DIAGNOSTIC_REGISTRY[index])
}

pub fn diagnostic_registered(code: &str) -> bool {
    diagnostic_definition(code).is_some()
}

pub fn registered_diagnostic(severity: DiagnosticSeverity, code: &str) -> CanonicalDiagnostic {
    let definition = diagnostic_definition(code).unwrap_or_else(|| {
        panic!("diagnostic code '{code}' is absent from diagnostics/registry.json")
    });
    CanonicalDiagnostic::new(
        severity,
        definition.code,
        definition.phase,
        definition.summary,
    )
    .with_help(definition.help)
}

pub fn registered_error_text(code: &str, found: impl Into<String>) -> String {
    registered_diagnostic(DiagnosticSeverity::Error, code)
        .with_found(found)
        .render("")
}

pub fn stage0_registered_diagnostic(
    severity: DiagnosticSeverity,
    code: &str,
    message: impl Into<String>,
) -> CanonicalDiagnostic {
    let definition = diagnostic_definition(code).unwrap_or_else(|| {
        panic!("diagnostic code '{code}' is absent from diagnostics/registry.json")
    });
    CanonicalDiagnostic::new(severity, definition.code, definition.phase, message)
        .with_help(definition.help)
}

impl DiagnosticDomain {
    pub fn from_phase(phase: &str) -> Self {
        let root = phase.split(['.', ':']).next().unwrap_or(phase);
        match root.to_ascii_lowercase().as_str() {
            "primitive" => Self::Primitive,
            "basal" => Self::Basal,
            "machine" => Self::Machine,
            "lexer" => Self::Lexer,
            "parser" => Self::Parser,
            "sema" | "semantic" => Self::Sema,
            "ir" | "nir" => Self::Ir,
            "mct" | "llvm" | "backend" => Self::Mct,
            "link" | "linker" => Self::Linker,
            "runtime" => Self::Runtime,
            "kernel" => Self::Kernel,
            "high" | "highlevel" | "high-level" => Self::HighLevel,
            "ghor" | "package" => Self::Ghor,
            "tooling" | "ide" | "lsp" | "nide" => Self::Tooling,
            _ => Self::Unknown,
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Primitive),
            1 => Some(Self::Basal),
            2 => Some(Self::Machine),
            3 => Some(Self::Lexer),
            4 => Some(Self::Parser),
            5 => Some(Self::Sema),
            6 => Some(Self::Ir),
            7 => Some(Self::Mct),
            8 => Some(Self::Linker),
            9 => Some(Self::Runtime),
            10 => Some(Self::Kernel),
            11 => Some(Self::HighLevel),
            12 => Some(Self::Tooling),
            13 => Some(Self::Ghor),
            255 => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceSpan {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl SourceSpan {
    pub fn point(line: usize, column: usize) -> Self {
        let line = saturating_u32(line);
        let column = saturating_u32(column);
        Self {
            line,
            column,
            end_line: line,
            end_column: column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDiagnostic {
    pub severity: DiagnosticSeverity,
    pub domain: DiagnosticDomain,
    pub code: String,
    pub phase: String,
    pub message: String,
    pub file: String,
    pub span: SourceSpan,
    pub expected: String,
    pub found: String,
    pub help: String,
    pub flags: u32,
    pub cause_index: u32,
    pub cause_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticRenderStyle {
    Plain,
    Ansi,
}

impl CanonicalDiagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        phase: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let phase = phase.into();
        Self {
            severity,
            domain: DiagnosticDomain::from_phase(&phase),
            code: code.into(),
            phase,
            message: message.into(),
            file: String::new(),
            span: SourceSpan::default(),
            expected: String::new(),
            found: String::new(),
            help: String::new(),
            flags: 0,
            cause_index: u32::MAX,
            cause_count: 0,
        }
    }

    pub fn from_legacy(message: &str, default_phase: &str, line: usize, column: usize) -> Self {
        if let Some(mut diagnostic) = parse_rendered_canonical(message) {
            if line != 0 || column != 0 {
                diagnostic.span = SourceSpan::point(line, column);
            }
            return diagnostic;
        }
        let (embedded_line, embedded_column, normalized) = strip_position_prefix(message);
        let line = if line == 0 {
            embedded_line.unwrap_or(0)
        } else {
            line
        };
        let column = if column == 0 {
            embedded_column.unwrap_or(0)
        } else {
            column
        };
        let (code, body) = split_code(normalized)
            .map(|(code, rest)| (code.to_string(), rest.to_string()))
            .unwrap_or_else(|| {
                (
                    fallback_code(default_phase, normalized).to_string(),
                    normalized.trim().to_string(),
                )
            });
        let explicit_phase = legacy_field(normalized, "phase");
        let phase = explicit_phase
            .or_else(|| default_phase.contains('.').then_some(default_phase))
            .or_else(|| diagnostic_definition(&code).map(|definition| definition.phase))
            .unwrap_or(default_phase)
            .trim()
            .to_string();
        let reason = legacy_field(normalized, "reason")
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .unwrap_or_else(|| body.trim())
            .to_string();
        let expected = legacy_field(normalized, "expected")
            .map(clean_legacy_value)
            .unwrap_or_default();
        let found = legacy_field(normalized, "found")
            .map(clean_legacy_value)
            .unwrap_or_default();
        let help = default_help(&code).to_string();
        Self {
            severity: DiagnosticSeverity::Error,
            domain: DiagnosticDomain::from_phase(&phase),
            code,
            phase,
            message: reason,
            file: String::new(),
            span: SourceSpan::point(line, column),
            expected,
            found,
            help,
            flags: 0,
            cause_index: u32::MAX,
            cause_count: 0,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = file.into();
        self
    }

    pub fn with_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_domain(mut self, domain: DiagnosticDomain) -> Self {
        self.domain = domain;
        self
    }

    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = phase.into();
        self.domain = DiagnosticDomain::from_phase(&self.phase);
        self
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = span;
        self
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = expected.into();
        self
    }

    pub fn with_found(mut self, found: impl Into<String>) -> Self {
        self.found = found.into();
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = help.into();
        self
    }

    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_causes(mut self, cause_index: u32, cause_count: u32) -> Self {
        self.cause_index = cause_index;
        self.cause_count = cause_count;
        self
    }

    pub fn stable_id(&self) -> u64 {
        stable_diagnostic_id(&self.code)
    }

    pub fn render(&self, source: &str) -> String {
        self.render_with_style(source, DiagnosticRenderStyle::Plain)
    }

    pub fn render_with_style(&self, source: &str, style: DiagnosticRenderStyle) -> String {
        let file = if self.file.is_empty() {
            "<source>"
        } else {
            &self.file
        };
        let mut out = String::new();
        match style {
            DiagnosticRenderStyle::Plain => out.push_str(&format!(
                "{}[{}] {}\n --> {}:{}:{}",
                self.severity.label(),
                self.code,
                self.phase,
                file,
                self.span.line,
                self.span.column
            )),
            DiagnosticRenderStyle::Ansi => {
                out.push_str(ansi_severity(self.severity));
                out.push_str(self.severity.label());
                out.push('[');
                out.push_str(&self.code);
                out.push_str("]\x1b[0m \x1b[1m");
                out.push_str(&self.phase);
                out.push_str("\x1b[0m\n \x1b[36m-->\x1b[0m ");
                out.push_str(file);
                out.push(':');
                out.push_str(&self.span.line.to_string());
                out.push(':');
                out.push_str(&self.span.column.to_string());
            }
        }
        if self.span.line > 0 {
            if let Some(line_text) = source
                .lines()
                .nth(self.span.line.saturating_sub(1) as usize)
            {
                let line = self.span.line.to_string();
                out.push('\n');
                out.push_str(&line);
                out.push_str(" | ");
                out.push_str(line_text);
                out.push('\n');
                out.push_str(&" ".repeat(line.len()));
                out.push_str(" | ");
                out.push_str(&" ".repeat(self.span.column.saturating_sub(1) as usize));
                if style == DiagnosticRenderStyle::Ansi {
                    out.push_str(ansi_severity(self.severity));
                    out.push('^');
                    out.push_str("\x1b[0m");
                } else {
                    out.push('^');
                }
            }
        }
        if !self.message.is_empty() {
            out.push_str(if style == DiagnosticRenderStyle::Ansi {
                "\n \x1b[36m=\x1b[0m "
            } else {
                "\n = "
            });
            out.push_str(&self.message);
        }
        if !self.found.is_empty() {
            out.push_str(if style == DiagnosticRenderStyle::Ansi {
                "\n \x1b[31mfound:\x1b[0m "
            } else {
                "\n found: "
            });
            out.push_str(&self.found);
        }
        if !self.expected.is_empty() {
            out.push_str(if style == DiagnosticRenderStyle::Ansi {
                "\n \x1b[32mexpected:\x1b[0m "
            } else {
                "\n expected: "
            });
            out.push_str(&self.expected);
        }
        if !self.help.is_empty() {
            out.push_str(if style == DiagnosticRenderStyle::Ansi {
                "\n \x1b[32mhelp:\x1b[0m "
            } else {
                "\n help: "
            });
            out.push_str(&self.help);
        }
        out
    }

    pub fn machine_packet(
        &self,
    ) -> Result<MachineDiagnosticPacket, MachineDiagnosticEncodingError> {
        MachineDiagnosticPacket::from_diagnostic(self)
    }
}

fn parse_rendered_canonical(message: &str) -> Option<CanonicalDiagnostic> {
    let mut lines = message.lines();
    let header = lines.next()?.trim();
    let (severity, tail) = if let Some(tail) = header.strip_prefix("error[") {
        (DiagnosticSeverity::Error, tail)
    } else if let Some(tail) = header.strip_prefix("warning[") {
        (DiagnosticSeverity::Warning, tail)
    } else if let Some(tail) = header.strip_prefix("note[") {
        (DiagnosticSeverity::Note, tail)
    } else {
        return None;
    };
    let code_end = tail.find(']')?;
    let code = tail[..code_end].trim();
    if code.is_empty() {
        return None;
    }
    let phase = tail[code_end + 1..].trim();
    if phase.is_empty() {
        return None;
    }

    let mut file = String::new();
    let mut span = SourceSpan::default();
    let mut detail = String::new();
    let mut help = String::new();
    let mut expected = String::new();
    let mut found = String::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(location) = trimmed.strip_prefix("-->") {
            let mut parts = location.trim().rsplitn(3, ':');
            let column = parts.next().and_then(|value| value.parse::<usize>().ok());
            let row = parts.next().and_then(|value| value.parse::<usize>().ok());
            let path = parts.next();
            if let (Some(row), Some(column), Some(path)) = (row, column, path) {
                file = path.to_string();
                span = SourceSpan::point(row, column);
            }
        } else if let Some(value) = trimmed.strip_prefix('=') {
            let value = value.trim();
            if let Some(value) = value.strip_prefix("expected:") {
                expected = value.trim().to_string();
            } else if let Some(value) = value.strip_prefix("found:") {
                found = value.trim().to_string();
            } else if !value.is_empty() {
                if !detail.is_empty() {
                    detail.push(' ');
                }
                detail.push_str(value);
            }
        } else if let Some(value) = trimmed.strip_prefix("help:") {
            help = value.trim().to_string();
        }
    }
    if detail.is_empty() {
        detail = diagnostic_definition(code)
            .map(|definition| definition.summary.to_string())
            .unwrap_or_else(|| code.to_string());
    }
    if help.is_empty() {
        help = default_help(code).to_string();
    }
    Some(CanonicalDiagnostic {
        severity,
        domain: DiagnosticDomain::from_phase(phase),
        code: code.to_string(),
        phase: phase.to_string(),
        message: detail,
        file,
        span,
        expected,
        found,
        help,
        flags: 0,
        cause_index: u32::MAX,
        cause_count: 0,
    })
}

fn ansi_severity(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Note => "\x1b[36m",
        DiagnosticSeverity::Help => "\x1b[32m",
        DiagnosticSeverity::Warning => "\x1b[33m",
        DiagnosticSeverity::Error => "\x1b[31m",
        DiagnosticSeverity::Fatal => "\x1b[1;31m",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct DiagnosticTextRef {
    pub offset: u32,
    pub bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct MachineDiagnosticRecord {
    pub magic: u32,
    pub abi_version: u16,
    pub record_bytes: u16,
    pub severity: u8,
    pub domain: u8,
    pub reserved0: u16,
    pub flags: u32,
    pub stable_id: u64,
    pub code: DiagnosticTextRef,
    pub message: DiagnosticTextRef,
    pub file: DiagnosticTextRef,
    pub phase: DiagnosticTextRef,
    pub expected: DiagnosticTextRef,
    pub found: DiagnosticTextRef,
    pub help: DiagnosticTextRef,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub cause_index: u32,
    pub cause_count: u32,
    pub reserved: [u8; 24],
}

impl MachineDiagnosticRecord {
    pub fn encode_le(&self) -> [u8; DIAGNOSTIC_RECORD_BYTES] {
        let mut out = [0_u8; DIAGNOSTIC_RECORD_BYTES];
        put_u32(&mut out, 0, self.magic);
        put_u16(&mut out, 4, self.abi_version);
        put_u16(&mut out, 6, self.record_bytes);
        out[8] = self.severity;
        out[9] = self.domain;
        put_u16(&mut out, 10, self.reserved0);
        put_u32(&mut out, 12, self.flags);
        put_u64(&mut out, 16, self.stable_id);
        put_text_ref(&mut out, 24, self.code);
        put_text_ref(&mut out, 32, self.message);
        put_text_ref(&mut out, 40, self.file);
        put_text_ref(&mut out, 48, self.phase);
        put_text_ref(&mut out, 56, self.expected);
        put_text_ref(&mut out, 64, self.found);
        put_text_ref(&mut out, 72, self.help);
        put_u32(&mut out, 80, self.line);
        put_u32(&mut out, 84, self.column);
        put_u32(&mut out, 88, self.end_line);
        put_u32(&mut out, 92, self.end_column);
        put_u32(&mut out, 96, self.cause_index);
        put_u32(&mut out, 100, self.cause_count);
        out[104..128].copy_from_slice(&self.reserved);
        out
    }

    pub fn decode_le(bytes: &[u8]) -> Result<Self, DiagnosticTransportError> {
        if bytes.len() < DIAGNOSTIC_RECORD_BYTES {
            return Err(DiagnosticTransportError::TruncatedRecord);
        }
        let mut reserved = [0_u8; 24];
        reserved.copy_from_slice(&bytes[104..128]);
        Ok(Self {
            magic: read_u32(bytes, 0)?,
            abi_version: read_u16(bytes, 4)?,
            record_bytes: read_u16(bytes, 6)?,
            severity: bytes[8],
            domain: bytes[9],
            reserved0: read_u16(bytes, 10)?,
            flags: read_u32(bytes, 12)?,
            stable_id: read_u64(bytes, 16)?,
            code: read_text_ref(bytes, 24)?,
            message: read_text_ref(bytes, 32)?,
            file: read_text_ref(bytes, 40)?,
            phase: read_text_ref(bytes, 48)?,
            expected: read_text_ref(bytes, 56)?,
            found: read_text_ref(bytes, 64)?,
            help: read_text_ref(bytes, 72)?,
            line: read_u32(bytes, 80)?,
            column: read_u32(bytes, 84)?,
            end_line: read_u32(bytes, 88)?,
            end_column: read_u32(bytes, 92)?,
            cause_index: read_u32(bytes, 96)?,
            cause_count: read_u32(bytes, 100)?,
            reserved,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineDiagnosticPacket {
    pub record: MachineDiagnosticRecord,
    pub arena: Vec<u8>,
}

impl MachineDiagnosticPacket {
    pub fn from_diagnostic(
        diagnostic: &CanonicalDiagnostic,
    ) -> Result<Self, MachineDiagnosticEncodingError> {
        let mut arena = Vec::new();
        let code = push_text(&mut arena, &diagnostic.code)?;
        let message = push_text(&mut arena, &diagnostic.message)?;
        let file = push_text(&mut arena, &diagnostic.file)?;
        let phase = push_text(&mut arena, &diagnostic.phase)?;
        let expected = push_text(&mut arena, &diagnostic.expected)?;
        let found = push_text(&mut arena, &diagnostic.found)?;
        let help = push_text(&mut arena, &diagnostic.help)?;
        Ok(Self {
            record: MachineDiagnosticRecord {
                magic: DIAGNOSTIC_ABI_MAGIC,
                abi_version: DIAGNOSTIC_ABI_VERSION,
                record_bytes: DIAGNOSTIC_RECORD_BYTES as u16,
                severity: diagnostic.severity as u8,
                domain: diagnostic.domain as u8,
                reserved0: 0,
                flags: diagnostic.flags,
                stable_id: diagnostic.stable_id(),
                code,
                message,
                file,
                phase,
                expected,
                found,
                help,
                line: diagnostic.span.line,
                column: diagnostic.span.column,
                end_line: diagnostic.span.end_line,
                end_column: diagnostic.span.end_column,
                cause_index: diagnostic.cause_index,
                cause_count: diagnostic.cause_count,
                reserved: [0; 24],
            },
            arena,
        })
    }

    pub fn encode_le(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(DIAGNOSTIC_RECORD_BYTES + self.arena.len());
        out.extend_from_slice(&self.record.encode_le());
        out.extend_from_slice(&self.arena);
        out
    }

    pub fn text(&self, text_ref: DiagnosticTextRef) -> Option<&str> {
        let start = text_ref.offset as usize;
        let end = start.checked_add(text_ref.bytes as usize)?;
        std::str::from_utf8(self.arena.get(start..end)?).ok()
    }

    pub fn to_diagnostic(&self) -> Result<CanonicalDiagnostic, DiagnosticTransportError> {
        if self.record.magic != DIAGNOSTIC_ABI_MAGIC {
            return Err(DiagnosticTransportError::InvalidRecordMagic);
        }
        if self.record.abi_version != DIAGNOSTIC_ABI_VERSION {
            return Err(DiagnosticTransportError::UnsupportedRecordVersion);
        }
        if usize::from(self.record.record_bytes) != DIAGNOSTIC_RECORD_BYTES {
            return Err(DiagnosticTransportError::InvalidRecordSize);
        }
        let severity = DiagnosticSeverity::from_u8(self.record.severity)
            .ok_or(DiagnosticTransportError::InvalidSeverity)?;
        let domain = DiagnosticDomain::from_u8(self.record.domain)
            .ok_or(DiagnosticTransportError::InvalidDomain)?;
        let text = |text_ref| {
            self.text(text_ref)
                .map(str::to_string)
                .ok_or(DiagnosticTransportError::InvalidTextReference)
        };
        let diagnostic = CanonicalDiagnostic {
            severity,
            domain,
            code: text(self.record.code)?,
            message: text(self.record.message)?,
            file: text(self.record.file)?,
            phase: text(self.record.phase)?,
            expected: text(self.record.expected)?,
            found: text(self.record.found)?,
            help: text(self.record.help)?,
            span: SourceSpan {
                line: self.record.line,
                column: self.record.column,
                end_line: self.record.end_line,
                end_column: self.record.end_column,
            },
            flags: self.record.flags,
            cause_index: self.record.cause_index,
            cause_count: self.record.cause_count,
        };
        if diagnostic.stable_id() != self.record.stable_id {
            return Err(DiagnosticTransportError::StableIdMismatch);
        }
        Ok(diagnostic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTransportError {
    Encoding(MachineDiagnosticEncodingError),
    TooManyDiagnostics,
    PayloadTooLarge,
    TruncatedHeader,
    InvalidFileMagic,
    UnsupportedFileVersion,
    InvalidHeaderSize,
    InvalidFileRecordSize,
    InvalidFrameHeaderSize,
    UnsupportedFileFlags,
    PayloadLengthMismatch,
    PayloadChecksumMismatch,
    TruncatedFrame,
    InvalidFrameMagic,
    InvalidFrameSize,
    InvalidFrameAlignment,
    FrameChecksumMismatch,
    NonZeroPadding,
    FrameCountMismatch,
    TruncatedRecord,
    InvalidRecordMagic,
    UnsupportedRecordVersion,
    InvalidRecordSize,
    InvalidSeverity,
    InvalidDomain,
    InvalidTextReference,
    StableIdMismatch,
    IntegerOverflow,
}

impl Display for DiagnosticTransportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Encoding(error) => return Display::fmt(error, f),
            Self::TooManyDiagnostics => "diagnostic count exceeds u32",
            Self::PayloadTooLarge => "diagnostic stream payload exceeds u32 bytes",
            Self::TruncatedHeader => "truncated .ndiag header",
            Self::InvalidFileMagic => "invalid .ndiag magic",
            Self::UnsupportedFileVersion => "unsupported .ndiag version",
            Self::InvalidHeaderSize => "invalid .ndiag header size",
            Self::InvalidFileRecordSize => "invalid .ndiag record size",
            Self::InvalidFrameHeaderSize => "invalid .ndiag frame-header size",
            Self::UnsupportedFileFlags => "unsupported .ndiag file flags",
            Self::PayloadLengthMismatch => "declared .ndiag payload length does not match input",
            Self::PayloadChecksumMismatch => "invalid .ndiag payload checksum",
            Self::TruncatedFrame => "truncated .ndiag frame",
            Self::InvalidFrameMagic => "invalid .ndiag frame magic",
            Self::InvalidFrameSize => "invalid .ndiag frame size",
            Self::InvalidFrameAlignment => "unaligned .ndiag frame",
            Self::FrameChecksumMismatch => "invalid .ndiag frame checksum",
            Self::NonZeroPadding => "non-zero .ndiag frame padding",
            Self::FrameCountMismatch => "declared .ndiag frame count does not match payload",
            Self::TruncatedRecord => "truncated NCDI record",
            Self::InvalidRecordMagic => "invalid NCDI record magic",
            Self::UnsupportedRecordVersion => "unsupported NCDI record version",
            Self::InvalidRecordSize => "invalid NCDI record size",
            Self::InvalidSeverity => "invalid NCDI severity",
            Self::InvalidDomain => "invalid NCDI domain",
            Self::InvalidTextReference => "invalid NCDI UTF-8 arena reference",
            Self::StableIdMismatch => "NCDI stable id does not match its code",
            Self::IntegerOverflow => "integer overflow while decoding .ndiag",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DiagnosticTransportError {}

impl From<MachineDiagnosticEncodingError> for DiagnosticTransportError {
    fn from(error: MachineDiagnosticEncodingError) -> Self {
        Self::Encoding(error)
    }
}

pub fn encode_ndiag(
    diagnostics: &[CanonicalDiagnostic],
) -> Result<Vec<u8>, DiagnosticTransportError> {
    let frame_count = u32::try_from(diagnostics.len())
        .map_err(|_| DiagnosticTransportError::TooManyDiagnostics)?;
    let mut payload = Vec::new();
    for diagnostic in diagnostics {
        let packet = diagnostic.machine_packet()?;
        let arena_bytes = u32::try_from(packet.arena.len())
            .map_err(|_| DiagnosticTransportError::PayloadTooLarge)?;
        let packet_bytes = DIAGNOSTIC_RECORD_BYTES
            .checked_add(packet.arena.len())
            .ok_or(DiagnosticTransportError::IntegerOverflow)?;
        let unaligned_frame_bytes = NDIAG_FRAME_HEADER_BYTES
            .checked_add(packet_bytes)
            .ok_or(DiagnosticTransportError::IntegerOverflow)?;
        let frame_bytes = align_up_8(unaligned_frame_bytes)?;
        let frame_bytes_u32 =
            u32::try_from(frame_bytes).map_err(|_| DiagnosticTransportError::PayloadTooLarge)?;
        let record = packet.record.encode_le();
        let checksum = fnv1a32_parts(&[&record, &packet.arena]);

        payload.extend_from_slice(&NDIAG_FRAME_MAGIC.to_le_bytes());
        payload.extend_from_slice(&frame_bytes_u32.to_le_bytes());
        payload.extend_from_slice(&arena_bytes.to_le_bytes());
        payload.extend_from_slice(&checksum.to_le_bytes());
        payload.extend_from_slice(&record);
        payload.extend_from_slice(&packet.arena);
        payload.resize(payload.len() + (frame_bytes - unaligned_frame_bytes), 0);
    }

    let payload_bytes =
        u32::try_from(payload.len()).map_err(|_| DiagnosticTransportError::PayloadTooLarge)?;
    let mut out = Vec::with_capacity(NDIAG_HEADER_BYTES + payload.len());
    out.extend_from_slice(&NDIAG_MAGIC);
    out.extend_from_slice(&NDIAG_VERSION.to_le_bytes());
    out.extend_from_slice(&(NDIAG_HEADER_BYTES as u16).to_le_bytes());
    out.extend_from_slice(&(DIAGNOSTIC_RECORD_BYTES as u16).to_le_bytes());
    out.extend_from_slice(&(NDIAG_FRAME_HEADER_BYTES as u16).to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&frame_count.to_le_bytes());
    out.extend_from_slice(&payload_bytes.to_le_bytes());
    out.extend_from_slice(&fnv1a32(&payload).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

pub fn decode_ndiag(bytes: &[u8]) -> Result<Vec<CanonicalDiagnostic>, DiagnosticTransportError> {
    if bytes.len() < NDIAG_HEADER_BYTES {
        return Err(DiagnosticTransportError::TruncatedHeader);
    }
    if bytes[..8] != NDIAG_MAGIC {
        return Err(DiagnosticTransportError::InvalidFileMagic);
    }
    if read_u16(bytes, 8)? != NDIAG_VERSION {
        return Err(DiagnosticTransportError::UnsupportedFileVersion);
    }
    if usize::from(read_u16(bytes, 10)?) != NDIAG_HEADER_BYTES {
        return Err(DiagnosticTransportError::InvalidHeaderSize);
    }
    if usize::from(read_u16(bytes, 12)?) != DIAGNOSTIC_RECORD_BYTES {
        return Err(DiagnosticTransportError::InvalidFileRecordSize);
    }
    if usize::from(read_u16(bytes, 14)?) != NDIAG_FRAME_HEADER_BYTES {
        return Err(DiagnosticTransportError::InvalidFrameHeaderSize);
    }
    if read_u32(bytes, 16)? != 0 {
        return Err(DiagnosticTransportError::UnsupportedFileFlags);
    }
    let frame_count = read_u32(bytes, 20)? as usize;
    let payload_bytes = read_u32(bytes, 24)? as usize;
    let expected_len = NDIAG_HEADER_BYTES
        .checked_add(payload_bytes)
        .ok_or(DiagnosticTransportError::IntegerOverflow)?;
    if bytes.len() != expected_len {
        return Err(DiagnosticTransportError::PayloadLengthMismatch);
    }
    let payload = &bytes[NDIAG_HEADER_BYTES..];
    if fnv1a32(payload) != read_u32(bytes, 28)? {
        return Err(DiagnosticTransportError::PayloadChecksumMismatch);
    }
    let maximum_frames = payload.len() / (NDIAG_FRAME_HEADER_BYTES + DIAGNOSTIC_RECORD_BYTES);
    if frame_count > maximum_frames {
        return Err(DiagnosticTransportError::FrameCountMismatch);
    }

    let mut diagnostics = Vec::with_capacity(frame_count);
    let mut cursor = 0_usize;
    while cursor < payload.len() {
        let remaining = &payload[cursor..];
        if remaining.len() < NDIAG_FRAME_HEADER_BYTES {
            return Err(DiagnosticTransportError::TruncatedFrame);
        }
        if read_u32(remaining, 0)? != NDIAG_FRAME_MAGIC {
            return Err(DiagnosticTransportError::InvalidFrameMagic);
        }
        let frame_bytes = read_u32(remaining, 4)? as usize;
        let arena_bytes = read_u32(remaining, 8)? as usize;
        if frame_bytes < NDIAG_FRAME_HEADER_BYTES + DIAGNOSTIC_RECORD_BYTES {
            return Err(DiagnosticTransportError::InvalidFrameSize);
        }
        if frame_bytes % 8 != 0 {
            return Err(DiagnosticTransportError::InvalidFrameAlignment);
        }
        let frame_end = cursor
            .checked_add(frame_bytes)
            .ok_or(DiagnosticTransportError::IntegerOverflow)?;
        if frame_end > payload.len() {
            return Err(DiagnosticTransportError::TruncatedFrame);
        }
        let packet_bytes = DIAGNOSTIC_RECORD_BYTES
            .checked_add(arena_bytes)
            .ok_or(DiagnosticTransportError::IntegerOverflow)?;
        let used_bytes = NDIAG_FRAME_HEADER_BYTES
            .checked_add(packet_bytes)
            .ok_or(DiagnosticTransportError::IntegerOverflow)?;
        if used_bytes > frame_bytes {
            return Err(DiagnosticTransportError::InvalidFrameSize);
        }
        let record_start = NDIAG_FRAME_HEADER_BYTES;
        let arena_start = record_start + DIAGNOSTIC_RECORD_BYTES;
        let arena_end = arena_start + arena_bytes;
        let record_bytes = &remaining[record_start..arena_start];
        let arena = &remaining[arena_start..arena_end];
        if fnv1a32_parts(&[record_bytes, arena]) != read_u32(remaining, 12)? {
            return Err(DiagnosticTransportError::FrameChecksumMismatch);
        }
        if remaining[arena_end..frame_bytes]
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(DiagnosticTransportError::NonZeroPadding);
        }
        let packet = MachineDiagnosticPacket {
            record: MachineDiagnosticRecord::decode_le(record_bytes)?,
            arena: arena.to_vec(),
        };
        diagnostics.push(packet.to_diagnostic()?);
        cursor = frame_end;
    }
    if diagnostics.len() != frame_count {
        return Err(DiagnosticTransportError::FrameCountMismatch);
    }
    Ok(diagnostics)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineDiagnosticEncodingError {
    ArenaTooLarge,
    TextTooLarge,
}

impl Display for MachineDiagnosticEncodingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArenaTooLarge => f.write_str("diagnostic arena exceeds u32 addressability"),
            Self::TextTooLarge => f.write_str("diagnostic text exceeds u32 length"),
        }
    }
}

impl std::error::Error for MachineDiagnosticEncodingError {}

pub fn stable_diagnostic_id(code: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in code.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn split_code(message: &str) -> Option<(&str, &str)> {
    let trimmed = message.trim();
    let delimiter = trimmed.find([':', ' ', '\n']).unwrap_or(trimmed.len());
    let token = &trimmed[..delimiter];
    let candidate = token
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(token);
    let valid = (candidate.starts_with("E_")
        || candidate.starts_with("W_")
        || candidate.starts_with("NCC-"))
        && candidate
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-');
    valid.then(|| {
        let rest = trimmed[delimiter..]
            .trim_start_matches([':', ' ', '\n'])
            .trim();
        (candidate, rest)
    })
}

fn strip_position_prefix(message: &str) -> (Option<usize>, Option<usize>, &str) {
    let trimmed = message.trim();
    let Some(header_end) = trimmed.find(']') else {
        return (None, None, trimmed);
    };
    let Some(header) = trimmed
        .strip_prefix('[')
        .map(|text| &text[..header_end - 1])
    else {
        return (None, None, trimmed);
    };
    let Some((line, column)) = header.split_once(':') else {
        return (None, None, trimmed);
    };
    let (Ok(line), Ok(column)) = (line.parse(), column.parse()) else {
        return (None, None, trimmed);
    };
    (Some(line), Some(column), trimmed[header_end + 1..].trim())
}

fn legacy_field<'a>(message: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=");
    let start = message.find(&needle)? + needle.len();
    let tail = &message[start..];
    let end = tail
        .char_indices()
        .find_map(|(index, _)| {
            let suffix = &tail[index..];
            (suffix.starts_with(" phase=")
                || suffix.starts_with(" reason=")
                || suffix.starts_with(" status=")
                || suffix.starts_with(" expected=")
                || suffix.starts_with(" found="))
            .then_some(index)
        })
        .unwrap_or(tail.len());
    Some(&tail[..end])
}

fn clean_legacy_value(value: &str) -> String {
    value.trim().trim_matches(['\'', '"', '(', ')']).to_string()
}

fn fallback_code(phase: &str, message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    match phase.split(['.', ':']).next().unwrap_or(phase) {
        "lexer" if lower.contains("invalid string escape") => "E_LEXER_ESCAPE_001",
        "lexer" if lower.contains("unterminated string") => "E_LEXER_STRING_UNTERMINATED_001",
        "lexer" if lower.contains("unterminated block comment") => {
            "E_LEXER_COMMENT_UNTERMINATED_001"
        }
        "lexer" if lower.contains("unexpected character") => "E_LEXER_CHARACTER_001",
        "lexer" => "E_LEXER_TOKEN_001",
        "parser" if lower.contains("expected semicolon") => "E_PARSER_EXPECTED_SEMICOLON_001",
        "parser" if lower.contains("invalid expression") => "E_PARSER_EXPRESSION_001",
        "parser" => "E_PARSER_SYNTAX_001",
        "sema" | "semantic" => "E_SEMA_CONTRACT_001",
        "machine" => "E_MACHINE_ENCODING_001",
        "mct" | "llvm" | "backend" => "E_MCT_LOWERING_001",
        "runtime" => "E_RUNTIME_EXECUTION_001",
        "ghor" | "package" => "E_GHOR_OPERATION_001",
        _ => "E_DIAGNOSTIC_UNCLASSIFIED_001",
    }
}

fn default_help(code: &str) -> &'static str {
    diagnostic_definition(code)
        .map(|definition| definition.help)
        .filter(|help| !help.is_empty())
        .unwrap_or("inspect the reported phase, source span and material cause")
}

fn push_text(
    arena: &mut Vec<u8>,
    text: &str,
) -> Result<DiagnosticTextRef, MachineDiagnosticEncodingError> {
    let offset =
        u32::try_from(arena.len()).map_err(|_| MachineDiagnosticEncodingError::ArenaTooLarge)?;
    let bytes =
        u32::try_from(text.len()).map_err(|_| MachineDiagnosticEncodingError::TextTooLarge)?;
    arena.extend_from_slice(text.as_bytes());
    Ok(DiagnosticTextRef { offset, bytes })
}

fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn put_u16(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], offset: usize, value: u64) {
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_text_ref(out: &mut [u8], offset: usize, text_ref: DiagnosticTextRef) {
    put_u32(out, offset, text_ref.offset);
    put_u32(out, offset + 4, text_ref.bytes);
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DiagnosticTransportError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or(DiagnosticTransportError::TruncatedRecord)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DiagnosticTransportError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or(DiagnosticTransportError::TruncatedRecord)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, DiagnosticTransportError> {
    let raw = bytes
        .get(offset..offset + 8)
        .ok_or(DiagnosticTransportError::TruncatedRecord)?;
    Ok(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

fn read_text_ref(
    bytes: &[u8],
    offset: usize,
) -> Result<DiagnosticTextRef, DiagnosticTransportError> {
    Ok(DiagnosticTextRef {
        offset: read_u32(bytes, offset)?,
        bytes: read_u32(bytes, offset + 4)?,
    })
}

fn align_up_8(value: usize) -> Result<usize, DiagnosticTransportError> {
    value
        .checked_add(7)
        .map(|value| value & !7)
        .ok_or(DiagnosticTransportError::IntegerOverflow)
}

fn fnv1a32(bytes: &[u8]) -> u32 {
    fnv1a32_parts(&[bytes])
}

fn fnv1a32_parts(parts: &[&[u8]]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for bytes in parts {
        for byte in *bytes {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(0x01000193);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_type_contract_becomes_a_typed_diagnostic() {
        let legacy = Diagnostic::new(
            "E_TYPE_CONTRACT_SECTION_001: phase=parser.type reason=invalid type contract section status=rejected",
            5,
            13,
        );
        let diagnostic = legacy.canonical("parser");
        assert_eq!(diagnostic.code, "E_TYPE_CONTRACT_SECTION_001");
        assert_eq!(diagnostic.phase, "parser.type");
        assert_eq!(diagnostic.domain, DiagnosticDomain::Parser);
        assert_eq!(diagnostic.message, "invalid type contract section");
        assert_eq!(diagnostic.span, SourceSpan::point(5, 13));
    }

    #[test]
    fn canonical_registry_is_sorted_unique_and_builds_diagnostics() {
        assert_eq!(DIAGNOSTIC_REGISTRY.len(), 2045);
        assert!(DIAGNOSTIC_REGISTRY
            .windows(2)
            .all(|pair| pair[0].code < pair[1].code));
        let diagnostic = registered_diagnostic(
            DiagnosticSeverity::Error,
            codes::E_TYPE_CONTRACT_SECTION_001,
        );
        assert_eq!(diagnostic.phase, "parser.type");
        assert_eq!(diagnostic.message, "invalid type contract section");
        assert!(!diagnostic.help.is_empty());
    }

    #[test]
    fn human_renderer_preserves_code_phase_source_and_caret() {
        let rendered = Diagnostic::new(
            "E_TYPE_CONTRACT_SECTION_001: phase=parser.type reason=invalid type contract section status=rejected",
            5,
            13,
        )
        .render_source(
            "parser",
            "gestor.n",
            "line1\nline2\nline3\nline4\nnombre >> str;\n",
        );
        assert!(rendered
            .starts_with("error[E_TYPE_CONTRACT_SECTION_001] parser.type\n --> gestor.n:5:13"));
        assert!(rendered.contains("5 | nombre >> str;"));
        assert!(rendered.contains("  |             ^"));
    }

    #[test]
    fn canonical_renderer_is_idempotent_across_cli_boundaries() {
        let mut diagnostic =
            registered_diagnostic(DiagnosticSeverity::Error, codes::E_MACHINE_ABI_001);
        diagnostic.message = "RISC-V32 profile requires abi: ilp32".to_string();
        let rendered = diagnostic
            .with_file("C:\\work\\program.nm")
            .with_span(SourceSpan::point(7, 13))
            .with_help("select the target ABI")
            .render("");
        let reparsed = CanonicalDiagnostic::from_legacy(&rendered, "cli", 0, 0);
        assert_eq!(reparsed.code, codes::E_MACHINE_ABI_001);
        assert_eq!(reparsed.phase, "machine");
        assert_eq!(reparsed.file, "C:\\work\\program.nm");
        assert_eq!(reparsed.span, SourceSpan::point(7, 13));
        assert_eq!(reparsed.message, "RISC-V32 profile requires abi: ilp32");
        assert_eq!(reparsed.help, "select the target ABI");
    }

    #[test]
    fn ansi_renderer_only_adds_terminal_style() {
        let diagnostic = CanonicalDiagnostic::new(
            DiagnosticSeverity::Error,
            "E_TYPE_CONTRACT_SECTION_001",
            "parser.type",
            "invalid type contract section",
        )
        .with_file("gestor.n")
        .with_span(SourceSpan::point(5, 13))
        .with_expected("type declaration")
        .with_found("field declaration")
        .with_help("declare a type");
        let source = "line1\nline2\nline3\nline4\nnombre >> str;\n";
        let plain = diagnostic.render(source);
        let ansi = diagnostic.render_with_style(source, DiagnosticRenderStyle::Ansi);
        assert!(ansi.contains("\x1b[31merror[E_TYPE_CONTRACT_SECTION_001]"));
        assert_eq!(strip_ansi(&ansi), plain);
    }

    #[test]
    fn machine_record_is_fixed_width_little_endian_and_arena_relative() {
        let diagnostic = CanonicalDiagnostic::from_legacy(
            "E_TYPE_CONTRACT_SECTION_001: phase=parser.type reason=invalid type contract section status=rejected",
            "parser",
            5,
            13,
        )
        .with_file("gestor.n")
        .with_expected("type declaration")
        .with_found("field declaration");
        let packet = diagnostic.machine_packet().expect("encode machine packet");
        let bytes = packet.record.encode_le();
        assert_eq!(bytes.len(), DIAGNOSTIC_RECORD_BYTES);
        assert_eq!(&bytes[0..4], b"NCDI");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2);
        assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 128);
        assert_eq!(bytes[8], DiagnosticSeverity::Error as u8);
        assert_eq!(bytes[9], DiagnosticDomain::Parser as u8);
        assert_eq!(
            packet.text(packet.record.code),
            Some("E_TYPE_CONTRACT_SECTION_001")
        );
        assert_eq!(packet.text(packet.record.phase), Some("parser.type"));
        assert_eq!(packet.text(packet.record.file), Some("gestor.n"));
        assert_eq!(packet.encode_le().len(), 128 + packet.arena.len());
    }

    #[test]
    fn uncoded_parser_errors_receive_stable_specific_codes() {
        let semicolon = Diagnostic::new("Expected Semicolon", 2, 1).canonical("parser");
        let expression = Diagnostic::new("Invalid expression", 8, 4).canonical("parser");
        assert_eq!(semicolon.code, "E_PARSER_EXPECTED_SEMICOLON_001");
        assert_eq!(expression.code, "E_PARSER_EXPRESSION_001");
        assert_eq!(semicolon.stable_id(), stable_diagnostic_id(&semicolon.code));
    }

    #[test]
    fn embedded_legacy_position_is_preserved_during_sema_bridge() {
        let diagnostic = CanonicalDiagnostic::from_legacy(
            "[9:4] E_SEMA_SAMPLE_001: incompatible values",
            "sema",
            0,
            0,
        );
        assert_eq!(diagnostic.code, "E_SEMA_SAMPLE_001");
        assert_eq!(diagnostic.span, SourceSpan::point(9, 4));
        assert_eq!(diagnostic.message, "incompatible values");
    }

    #[test]
    fn bracketed_cli_code_is_preserved_during_legacy_bridge() {
        let diagnostic = CanonicalDiagnostic::from_legacy(
            "[NCC-CLI-0001] nfmt syntax: nfmt <file.n>\nHint: run `ncc nfmt --help`",
            "tooling.nfmt",
            0,
            0,
        );
        assert_eq!(diagnostic.code, "NCC-CLI-0001");
        assert_eq!(diagnostic.phase, "tooling.nfmt");
        assert_eq!(diagnostic.domain, DiagnosticDomain::Tooling);
        assert_eq!(
            diagnostic.message,
            "nfmt syntax: nfmt <file.n>\nHint: run `ncc nfmt --help`"
        );
    }

    #[test]
    fn ndiag_roundtrip_preserves_multiple_canonical_records() {
        let diagnostics = vec![
            CanonicalDiagnostic::new(
                DiagnosticSeverity::Error,
                "E_TYPE_CONTRACT_SECTION_001",
                "parser.type",
                "invalid type contract section",
            )
            .with_file("gestor.n")
            .with_span(SourceSpan::point(5, 13))
            .with_expected("type declaration")
            .with_found("field declaration"),
            CanonicalDiagnostic::new(
                DiagnosticSeverity::Warning,
                "W_MACHINE_ALIGNMENT_001",
                "machine.encoder",
                "unaligned operand",
            )
            .with_file("boot.nm")
            .with_span(SourceSpan {
                line: 9,
                column: 4,
                end_line: 9,
                end_column: 12,
            }),
        ];
        let encoded = encode_ndiag(&diagnostics).expect("encode .ndiag");
        assert_eq!(&encoded[..8], &NDIAG_MAGIC);
        assert_eq!(encoded.len() % 8, 0);
        assert_eq!(decode_ndiag(&encoded).expect("decode .ndiag"), diagnostics);
        let empty = encode_ndiag(&[]).expect("encode empty .ndiag");
        assert!(decode_ndiag(&empty)
            .expect("decode empty .ndiag")
            .is_empty());
    }

    #[test]
    fn ndiag_rejects_global_and_frame_corruption() {
        let diagnostic = CanonicalDiagnostic::new(
            DiagnosticSeverity::Error,
            "E_TEST_001",
            "sema.test",
            "broken",
        );
        let encoded = encode_ndiag(&[diagnostic]).expect("encode .ndiag");

        let mut global_corruption = encoded.clone();
        *global_corruption.last_mut().expect("payload byte") ^= 1;
        assert_eq!(
            decode_ndiag(&global_corruption),
            Err(DiagnosticTransportError::PayloadChecksumMismatch)
        );

        let mut frame_corruption = encoded;
        frame_corruption[28..32].fill(0);
        frame_corruption[NDIAG_HEADER_BYTES + NDIAG_FRAME_HEADER_BYTES + 24] ^= 1;
        let payload_checksum = fnv1a32(&frame_corruption[NDIAG_HEADER_BYTES..]);
        frame_corruption[28..32].copy_from_slice(&payload_checksum.to_le_bytes());
        assert_eq!(
            decode_ndiag(&frame_corruption),
            Err(DiagnosticTransportError::FrameChecksumMismatch)
        );
    }

    #[test]
    fn ndiag_rejects_truncation_and_invalid_text_references() {
        assert_eq!(
            decode_ndiag(b"NDIAG"),
            Err(DiagnosticTransportError::TruncatedHeader)
        );

        let diagnostic = CanonicalDiagnostic::new(
            DiagnosticSeverity::Error,
            "E_TEST_002",
            "parser.test",
            "broken",
        );
        let mut encoded = encode_ndiag(&[diagnostic]).expect("encode .ndiag");
        let record_offset = NDIAG_HEADER_BYTES + NDIAG_FRAME_HEADER_BYTES;
        encoded[record_offset + 24..record_offset + 28].copy_from_slice(&u32::MAX.to_le_bytes());
        let arena_bytes =
            read_u32(&encoded, NDIAG_HEADER_BYTES + 8).expect("encoded frame arena size") as usize;
        let packet_end = record_offset + DIAGNOSTIC_RECORD_BYTES + arena_bytes;
        let frame_checksum = fnv1a32(&encoded[record_offset..packet_end]);
        encoded[NDIAG_HEADER_BYTES + 12..NDIAG_HEADER_BYTES + 16]
            .copy_from_slice(&frame_checksum.to_le_bytes());
        let payload_checksum = fnv1a32(&encoded[NDIAG_HEADER_BYTES..]);
        encoded[28..32].copy_from_slice(&payload_checksum.to_le_bytes());
        assert_eq!(
            decode_ndiag(&encoded),
            Err(DiagnosticTransportError::InvalidTextReference)
        );
    }

    fn strip_ansi(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'[') {
                index += 2;
                while index < bytes.len() {
                    let byte = bytes[index];
                    index += 1;
                    if byte.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(bytes[index]);
                index += 1;
            }
        }
        String::from_utf8(out).expect("ANSI stripping keeps UTF-8")
    }
}
