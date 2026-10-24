#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsolePrintKind {
    Std,
    Ln,
    Sp,
    Nl,
    Err,
    Wm,
    Inf,
    Ok,
    Trc,
    Dbg,
    Sta,
    Inspect,
    Mem,
    Site,
    Type,
    Raw,
    Stable,
    Addr,
    Phys,
    Dump,
    Peek,
    Mmio,
    Port,
    Sensory,
    St,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleBuiltin {
    Print,
    Println,
    PrintSp,
    PrintNl,
    PrintErr,
    PrintWm,
    PrintInf,
    PrintOk,
    PrintTrc,
    PrintDbg,
    PrintSta,
    PrintInspect,
    PrintMem,
    PrintSite,
    PrintType,
    PrintRaw,
    PrintStable,
    PrintAddr,
    PrintPhys,
    PrintDump,
    PrintPeek,
    PrintMmio,
    PrintPort,
    PrintSensory,
    PrintSt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleChannel {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleBuiltinSpec {
    pub channel: ConsoleChannel,
    pub newline: bool,
    pub ansi_prefix: Option<&'static str>,
}

impl ConsoleBuiltin {
    pub fn official_name(self) -> &'static str {
        match self {
            ConsoleBuiltin::Print => "print",
            ConsoleBuiltin::Println => "println",
            ConsoleBuiltin::PrintSp => "print.sp",
            ConsoleBuiltin::PrintNl => "print.nl",
            ConsoleBuiltin::PrintErr => "print.err",
            ConsoleBuiltin::PrintWm => "print.wm",
            ConsoleBuiltin::PrintInf => "print.inf",
            ConsoleBuiltin::PrintOk => "print.ok",
            ConsoleBuiltin::PrintTrc => "print.trc",
            ConsoleBuiltin::PrintDbg => "print.dbg",
            ConsoleBuiltin::PrintSta => "print.sta",
            ConsoleBuiltin::PrintInspect => "print.inspect",
            ConsoleBuiltin::PrintMem => "print.mem",
            ConsoleBuiltin::PrintSite => "print.site",
            ConsoleBuiltin::PrintType => "print.type",
            ConsoleBuiltin::PrintRaw => "print.raw",
            ConsoleBuiltin::PrintStable => "print.stable",
            ConsoleBuiltin::PrintAddr => "print.addr",
            ConsoleBuiltin::PrintPhys => "print.phys",
            ConsoleBuiltin::PrintDump => "print.dump",
            ConsoleBuiltin::PrintPeek => "print.peek",
            ConsoleBuiltin::PrintMmio => "print.mmio",
            ConsoleBuiltin::PrintPort => "print.port",
            ConsoleBuiltin::PrintSensory => "print.sensory",
            ConsoleBuiltin::PrintSt => "print.st",
        }
    }

    pub fn spec(self) -> ConsoleBuiltinSpec {
        match self {
            ConsoleBuiltin::Print => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: false,
                ansi_prefix: None,
            },
            ConsoleBuiltin::Println => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: None,
            },
            ConsoleBuiltin::PrintSp => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: false,
                ansi_prefix: None,
            },
            ConsoleBuiltin::PrintNl => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: None,
            },
            ConsoleBuiltin::PrintErr => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stderr,
                newline: true,
                ansi_prefix: Some("\x1b[31m"),
            },
            ConsoleBuiltin::PrintWm => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stderr,
                newline: true,
                ansi_prefix: Some("\x1b[33m"),
            },
            ConsoleBuiltin::PrintInf => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[94m"),
            },
            ConsoleBuiltin::PrintOk => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[92m"),
            },
            ConsoleBuiltin::PrintTrc => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[36m"),
            },
            ConsoleBuiltin::PrintDbg => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[35m"),
            },
            ConsoleBuiltin::PrintSta => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[32m"),
            },
            ConsoleBuiltin::PrintInspect => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[96m"),
            },
            ConsoleBuiltin::PrintMem => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[34m"),
            },
            ConsoleBuiltin::PrintSite => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[96m"),
            },
            ConsoleBuiltin::PrintType => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[95m"),
            },
            ConsoleBuiltin::PrintRaw => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[37m"),
            },
            ConsoleBuiltin::PrintStable => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[90m"),
            },
            ConsoleBuiltin::PrintAddr => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[91m"),
            },
            ConsoleBuiltin::PrintPhys => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[91m"),
            },
            ConsoleBuiltin::PrintDump => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[90m"),
            },
            ConsoleBuiltin::PrintPeek => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[90m"),
            },
            ConsoleBuiltin::PrintMmio => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[93m"),
            },
            ConsoleBuiltin::PrintPort => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[93m"),
            },
            ConsoleBuiltin::PrintSensory => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[97m"),
            },
            ConsoleBuiltin::PrintSt => ConsoleBuiltinSpec {
                channel: ConsoleChannel::Stdout,
                newline: true,
                ansi_prefix: Some("\x1b[37m"),
            },
        }
    }
}

pub fn resolve_console_builtin(path: &str) -> Option<ConsoleBuiltin> {
    match path {
        "print" => Some(ConsoleBuiltin::Print),
        "println" => Some(ConsoleBuiltin::Println),
        "print.sp" => Some(ConsoleBuiltin::PrintSp),
        "print.nl" => Some(ConsoleBuiltin::PrintNl),
        "print.err" => Some(ConsoleBuiltin::PrintErr),
        "print.wm" => Some(ConsoleBuiltin::PrintWm),
        "print.inf" => Some(ConsoleBuiltin::PrintInf),
        "print.ok" => Some(ConsoleBuiltin::PrintOk),
        "print.trc" => Some(ConsoleBuiltin::PrintTrc),
        "print.dbg" => Some(ConsoleBuiltin::PrintDbg),
        "print.sta" => Some(ConsoleBuiltin::PrintSta),
        "print.inspect" => Some(ConsoleBuiltin::PrintInspect),
        "print.mem" => Some(ConsoleBuiltin::PrintMem),
        "print.site" => Some(ConsoleBuiltin::PrintSite),
        "print.type" => Some(ConsoleBuiltin::PrintType),
        "print.raw" => Some(ConsoleBuiltin::PrintRaw),
        "print.stable" => Some(ConsoleBuiltin::PrintStable),
        "print.addr" => Some(ConsoleBuiltin::PrintAddr),
        "print.phys" => Some(ConsoleBuiltin::PrintPhys),
        "print.dump" => Some(ConsoleBuiltin::PrintDump),
        "print.peek" => Some(ConsoleBuiltin::PrintPeek),
        "print.mmio" => Some(ConsoleBuiltin::PrintMmio),
        "print.port" => Some(ConsoleBuiltin::PrintPort),
        "print.sensory" => Some(ConsoleBuiltin::PrintSensory),
        "print.st" => Some(ConsoleBuiltin::PrintSt),
        "print.state" => Some(ConsoleBuiltin::PrintSt),
        _ => None,
    }
}

pub fn console_builtin_from_print_kind(kind: ConsolePrintKind) -> ConsoleBuiltin {
    match kind {
        ConsolePrintKind::Std => ConsoleBuiltin::Print,
        ConsolePrintKind::Ln => ConsoleBuiltin::Println,
        ConsolePrintKind::Sp => ConsoleBuiltin::PrintSp,
        ConsolePrintKind::Nl => ConsoleBuiltin::PrintNl,
        ConsolePrintKind::Err => ConsoleBuiltin::PrintErr,
        ConsolePrintKind::Wm => ConsoleBuiltin::PrintWm,
        ConsolePrintKind::Inf => ConsoleBuiltin::PrintInf,
        ConsolePrintKind::Ok => ConsoleBuiltin::PrintOk,
        ConsolePrintKind::Trc => ConsoleBuiltin::PrintTrc,
        ConsolePrintKind::Dbg => ConsoleBuiltin::PrintDbg,
        ConsolePrintKind::Sta => ConsoleBuiltin::PrintSta,
        ConsolePrintKind::Inspect => ConsoleBuiltin::PrintInspect,
        ConsolePrintKind::Mem => ConsoleBuiltin::PrintMem,
        ConsolePrintKind::Site => ConsoleBuiltin::PrintSite,
        ConsolePrintKind::Type => ConsoleBuiltin::PrintType,
        ConsolePrintKind::Raw => ConsoleBuiltin::PrintRaw,
        ConsolePrintKind::Stable => ConsoleBuiltin::PrintStable,
        ConsolePrintKind::Addr => ConsoleBuiltin::PrintAddr,
        ConsolePrintKind::Phys => ConsoleBuiltin::PrintPhys,
        ConsolePrintKind::Dump => ConsoleBuiltin::PrintDump,
        ConsolePrintKind::Peek => ConsoleBuiltin::PrintPeek,
        ConsolePrintKind::Mmio => ConsoleBuiltin::PrintMmio,
        ConsolePrintKind::Port => ConsoleBuiltin::PrintPort,
        ConsolePrintKind::Sensory => ConsoleBuiltin::PrintSensory,
        ConsolePrintKind::St => ConsoleBuiltin::PrintSt,
    }
}

pub fn execute_console_builtin(builtin: ConsoleBuiltin, value: &str) {
    let spec = builtin.spec();
    let value = render_console_text_controls(value);
    let rendered = match (spec.ansi_prefix, color_enabled()) {
        (Some(prefix), true) => format!("{prefix}{value}\x1b[0m"),
        _ => value,
    };

    match (spec.channel, spec.newline) {
        (ConsoleChannel::Stdout, false) => print!("{}", rendered),
        (ConsoleChannel::Stdout, true) => println!("{}", rendered),
        (ConsoleChannel::Stderr, false) => eprint!("{}", rendered),
        (ConsoleChannel::Stderr, true) => eprintln!("{}", rendered),
    }
}

fn color_enabled() -> bool {
    let raw = std::env::var("NC_NO_COLOR").ok();
    !parse_no_color(raw.as_deref())
}

fn parse_no_color(raw: Option<&str>) -> bool {
    match raw {
        None => false,
        Some(v) => {
            let lowered = v.trim().to_ascii_lowercase();
            matches!(lowered.as_str(), "1" | "true" | "yes" | "on")
        }
    }
}

pub fn execute_console_builtin_path(path: &str, value: &str) -> Result<(), String> {
    let Some(builtin) = resolve_console_builtin(path) else {
        return Err(format!("invalid console builtin path '{}'", path));
    };
    execute_console_builtin(builtin, value);
    Ok(())
}

pub fn emit_print(kind: ConsolePrintKind, value: &str) {
    execute_console_builtin(console_builtin_from_print_kind(kind), value);
}

pub fn render_console_text_controls(value: &str) -> String {
    if value.trim() == "##" {
        "=".repeat(72)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        console_builtin_from_print_kind, execute_console_builtin_path, parse_no_color,
        render_console_text_controls, resolve_console_builtin, ConsoleBuiltin, ConsoleChannel,
        ConsolePrintKind,
    };

    #[test]
    fn resolve_console_builtin_paths() {
        assert_eq!(
            resolve_console_builtin("print"),
            Some(ConsoleBuiltin::Print)
        );
        assert_eq!(
            resolve_console_builtin("print.sp"),
            Some(ConsoleBuiltin::PrintSp)
        );
        assert_eq!(
            resolve_console_builtin("print.nl"),
            Some(ConsoleBuiltin::PrintNl)
        );
        assert_eq!(
            resolve_console_builtin("print.inf"),
            Some(ConsoleBuiltin::PrintInf)
        );
        assert_eq!(
            resolve_console_builtin("print.ok"),
            Some(ConsoleBuiltin::PrintOk)
        );
        assert_eq!(
            resolve_console_builtin("print.inspect"),
            Some(ConsoleBuiltin::PrintInspect)
        );
        assert_eq!(
            resolve_console_builtin("print.mem"),
            Some(ConsoleBuiltin::PrintMem)
        );
        assert_eq!(
            resolve_console_builtin("print.state"),
            Some(ConsoleBuiltin::PrintSt)
        );
        assert_eq!(
            resolve_console_builtin("print.sensory"),
            Some(ConsoleBuiltin::PrintSensory)
        );
        assert_eq!(resolve_console_builtin("print.unknown"), None);
    }

    #[test]
    fn builtin_spec_channels_are_stable() {
        assert_eq!(ConsoleBuiltin::Print.spec().channel, ConsoleChannel::Stdout);
        assert_eq!(
            ConsoleBuiltin::PrintErr.spec().channel,
            ConsoleChannel::Stderr
        );
        assert_eq!(
            ConsoleBuiltin::PrintWm.spec().channel,
            ConsoleChannel::Stderr
        );
        assert!(ConsoleBuiltin::Println.spec().newline);
    }

    #[test]
    fn print_kind_mapping_is_complete() {
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Std),
            ConsoleBuiltin::Print
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Ln),
            ConsoleBuiltin::Println
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Sp),
            ConsoleBuiltin::PrintSp
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Nl),
            ConsoleBuiltin::PrintNl
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Err),
            ConsoleBuiltin::PrintErr
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Inf),
            ConsoleBuiltin::PrintInf
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Ok),
            ConsoleBuiltin::PrintOk
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Inspect),
            ConsoleBuiltin::PrintInspect
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Site),
            ConsoleBuiltin::PrintSite
        );
        assert_eq!(
            console_builtin_from_print_kind(ConsolePrintKind::Sensory),
            ConsoleBuiltin::PrintSensory
        );
    }

    #[test]
    fn render_console_text_controls_expands_separator_marker() {
        assert_eq!(render_console_text_controls("##"), "=".repeat(72));
        assert_eq!(render_console_text_controls("  ##  "), "=".repeat(72));
        assert_eq!(render_console_text_controls("hola"), "hola");
    }

    #[test]
    fn execute_console_builtin_path_rejects_invalid_names() {
        let err = execute_console_builtin_path("print.bad", "x").expect_err("expected invalid");
        assert!(err.contains("invalid console builtin path"));
    }

    #[test]
    fn parse_no_color_truthy_values() {
        assert!(parse_no_color(Some("1")));
        assert!(parse_no_color(Some("true")));
        assert!(parse_no_color(Some("YES")));
        assert!(parse_no_color(Some("on")));
        assert!(!parse_no_color(Some("0")));
        assert!(!parse_no_color(Some("false")));
        assert!(!parse_no_color(None));
    }
}
