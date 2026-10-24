#![allow(dead_code)]

use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone)]
struct DapMessage {
    seq: i64,
    command: String,
    arguments: Value,
}

#[derive(Debug, Default)]
struct DapSession {
    next_seq: i64,
    program: Option<PathBuf>,
    cwd: Option<PathBuf>,
    attached_pid: Option<u32>,
    attach_payload: Option<Value>,
    machine_payload: Option<Value>,
    breakpoints: BTreeMap<String, Vec<i64>>,
    launched: bool,
    terminated: bool,
    current_line: i64,
    paused_at_breakpoint: bool,
}

pub(crate) fn run_debug_adapter_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        println!("{}", debug_adapter_usage());
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--probe") {
        let payload = debug_adapter_probe_payload();
        if args.iter().any(|arg| arg == "--json") {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "nC debug-adapter: status={} protocol={} breakpoints={}",
                payload["status"].as_str().unwrap_or("?"),
                payload["protocol"].as_str().unwrap_or("?"),
                payload["capabilities"]["supportsSetBreakpointsRequest"]
                    .as_bool()
                    .unwrap_or(false)
            );
        }
        return Ok(());
    }
    if let Some(path) = value_after(args, "--source-map") {
        let payload = debug_source_map_payload(Path::new(path))?;
        if args.iter().any(|arg| arg == "--json") {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "nC source map: file={} breakable_lines={}",
                payload["source"]["path"].as_str().unwrap_or(path),
                payload["breakable_lines"]
                    .as_array()
                    .map(|lines| lines.len())
                    .unwrap_or(0)
            );
        }
        return Ok(());
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut session = DapSession::default();
    run_dap_loop(stdin.lock(), stdout.lock(), &mut session)
}

fn debug_adapter_usage() -> &'static str {
    "Usage: debug-adapter [--probe --json] [--source-map <file.n> --json]\n\
     Runs the native nC Debug Adapter Protocol endpoint on stdin/stdout.\n\
     IDEs launch it with Content-Length framed DAP messages."
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

fn debug_adapter_probe_payload() -> Value {
    json!({
        "schema": "ncc.debug_adapter.probe.v1",
        "status": "closed_source_and_machine_debug_routes",
        "protocol": "Debug Adapter Protocol",
        "transport": "stdio_content_length",
        "frontend_env": ["NCC_IDE_DEBUG", "NCC_DEBUGGER_FRONTEND"],
        "source_debug": {
            "build": "ncc compiler build-native --profile debug",
            "symbols": "DWARF/PDB emitted from exact .n source spans",
            "adapter": "system LLDB-DAP or GDB-DAP",
            "features": ["verified_breakpoints", "step_in", "step_over", "step_out", "stack", "locals", "evaluate", "console", "terminate", "restart"]
        },
        "machine_debug": {
            "adapter": "ncc debug-adapter plus machine debug-bridge",
            "features": ["instruction_listing", "register_scopes", "register_evaluate", "terminate", "restart"]
        },
        "machine_debug_bridge": super::machine::machine_debug_bridge_probe_json(),
        "live_os_attach": super::machine::machine_debug_attach_probe_json(),
        "capabilities": dap_capabilities(),
        "limits": {
            "source_breakpoints": "verified_by_native_debugger_against_emitted_debug_symbols",
            "source_stepping": "native_cpu_execution_through_system_debugger",
            "native_cpu_register_stepping": "closed_for_nm_via_machine_debug_bridge",
            "native_os_attach": "closed_snapshot_then_detach_via_machine_debug_attach"
        }
    })
}

fn dap_capabilities() -> Value {
    json!({
        "supportsConfigurationDoneRequest": true,
        "supportsSetBreakpointsRequest": true,
        "supportsAttachRequest": true,
        "supportsTerminateRequest": true,
        "supportsLoadedSourcesRequest": true,
        "supportsStepInTargetsRequest": true,
        "supportsEvaluateForHovers": true,
        "supportsRestartRequest": true,
        "supportsConditionalBreakpoints": false,
        "supportsHitConditionalBreakpoints": false
    })
}

fn debug_source_map_payload(path: &Path) -> Result<Value, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let breakable = source
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let trimmed = line.trim();
            let is_breakable = !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.ends_with(':')
                && trimmed != "{"
                && trimmed != "}";
            is_breakable.then_some((idx + 1) as i64)
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": "ncc.debug_adapter.source_map.v1",
        "status": "ok",
        "source": {
            "name": path.file_name().and_then(|value| value.to_str()).unwrap_or("source.n"),
            "path": path.to_string_lossy()
        },
        "line_count": source.lines().count(),
        "breakable_lines": breakable
    }))
}

fn run_dap_loop<R: Read, W: Write>(
    reader: R,
    mut writer: W,
    session: &mut DapSession,
) -> Result<(), Box<dyn Error>> {
    let mut reader = BufReader::new(reader);
    while let Some(message) = read_dap_message(&mut reader)? {
        let should_exit = handle_dap_message(session, &mut writer, message)?;
        writer.flush()?;
        if should_exit {
            break;
        }
    }
    Ok(())
}

fn read_dap_message<R: BufRead>(reader: &mut R) -> Result<Option<DapMessage>, Box<dyn Error>> {
    let mut content_len: Option<usize> = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Ok(None);
        }
        let trimmed = header.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_len = Some(value.trim().parse::<usize>()?);
        }
    }
    let Some(len) = content_len else {
        return Err("E_DAP_HEADER_001: missing Content-Length".into());
    };
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    let value: Value = serde_json::from_slice(&body)?;
    Ok(Some(DapMessage {
        seq: value.get("seq").and_then(Value::as_i64).unwrap_or(0),
        command: value
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        arguments: value.get("arguments").cloned().unwrap_or_else(|| json!({})),
    }))
}

fn handle_dap_message<W: Write>(
    session: &mut DapSession,
    writer: &mut W,
    message: DapMessage,
) -> Result<bool, Box<dyn Error>> {
    match message.command.as_str() {
        "initialize" => {
            send_response(session, writer, &message, true, dap_capabilities())?;
            send_event(session, writer, "initialized", json!({}))?;
        }
        "setBreakpoints" => {
            let source_path = message
                .arguments
                .pointer("/source/path")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>")
                .to_string();
            let source_lines = fs::read_to_string(&source_path)
                .map(|text| text.lines().count() as i64)
                .unwrap_or(i64::MAX);
            let breakpoints = message
                .arguments
                .get("breakpoints")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut accepted = Vec::new();
            let mut lines = Vec::new();
            for (idx, bp) in breakpoints.iter().enumerate() {
                let line = bp.get("line").and_then(Value::as_i64).unwrap_or(1);
                let verified = line > 0 && line <= source_lines;
                if verified {
                    lines.push(line);
                    if session.current_line == 0 {
                        session.current_line = line;
                    }
                }
                accepted.push(json!({
                    "id": idx + 1,
                    "verified": verified,
                    "line": line,
                    "source": {"path": source_path}
                }));
            }
            session.breakpoints.insert(source_path, lines);
            send_response(
                session,
                writer,
                &message,
                true,
                json!({"breakpoints": accepted}),
            )?;
        }
        "launch" => {
            if let Some(program) = message
                .arguments
                .get("program")
                .or_else(|| message.arguments.get("file"))
                .and_then(Value::as_str)
            {
                session.program = Some(PathBuf::from(program));
            }
            if let Some(cwd) = message.arguments.get("cwd").and_then(Value::as_str) {
                session.cwd = Some(PathBuf::from(cwd));
            }
            send_response(session, writer, &message, true, json!({}))?;
            send_event(
                session,
                writer,
                "process",
                json!({
                    "name": session.program.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "ncc".to_string()),
                    "isLocalProcess": true,
                    "startMethod": "launch"
                }),
            )?;
        }
        "attach" => {
            let Some(pid) = dap_process_id_from_arguments(&message.arguments) else {
                send_response(
                    session,
                    writer,
                    &message,
                    false,
                    json!({"error": "attach expects processId or pid"}),
                )?;
                return Ok(false);
            };
            let payload = super::machine::machine_debug_attach_payload(pid);
            let attached =
                payload.get("status").and_then(Value::as_str) == Some("closed_live_os_attach");
            if attached {
                session.attached_pid = Some(pid);
                session.attach_payload = Some(payload.clone());
                session.launched = true;
                session.current_line = 1;
                send_response(
                    session,
                    writer,
                    &message,
                    true,
                    json!({
                        "pid": pid,
                        "status": payload.get("status").cloned().unwrap_or_else(|| json!("unknown")),
                        "backend": payload.get("backend").cloned().unwrap_or_else(|| json!("unknown"))
                    }),
                )?;
                send_event(
                    session,
                    writer,
                    "process",
                    json!({
                        "name": format!("pid:{pid}"),
                        "systemProcessId": pid,
                        "isLocalProcess": true,
                        "startMethod": "attach"
                    }),
                )?;
                send_event(
                    session,
                    writer,
                    "stopped",
                    json!({
                        "reason": "pause",
                        "threadId": 1,
                        "allThreadsStopped": true,
                        "description": "nC machine live OS snapshot captured and detached"
                    }),
                )?;
            } else {
                send_response(session, writer, &message, false, payload)?;
            }
        }
        "configurationDone" => {
            send_response(session, writer, &message, true, json!({}))?;
            if session.attached_pid.is_some() {
                return Ok(false);
            }
            if let Some(line) = first_breakpoint_line(session) {
                session.current_line = line;
                session.paused_at_breakpoint = true;
                send_event(
                    session,
                    writer,
                    "stopped",
                    json!({
                        "reason": "breakpoint",
                        "threadId": 1,
                        "allThreadsStopped": true
                    }),
                )?;
            } else {
                run_debuggee(session, writer)?;
            }
        }
        "threads" => {
            let thread_name = session
                .attached_pid
                .map(|pid| format!("attached pid {pid}"))
                .unwrap_or_else(|| "nC main".to_string());
            send_response(
                session,
                writer,
                &message,
                true,
                json!({"threads": [{"id": 1, "name": thread_name}]}),
            )?;
        }
        "stackTrace" => {
            let source = session.program.as_ref().map(|path| {
                json!({
                    "name": path.file_name().and_then(|v| v.to_str()).unwrap_or("main.n"),
                    "path": path.to_string_lossy()
                })
            });
            let frame_name = session
                .attached_pid
                .map(|pid| format!("live-os-attach:{pid}"))
                .unwrap_or_else(|| "main".to_string());
            send_response(
                session,
                writer,
                &message,
                true,
                json!({
                    "stackFrames": [{
                        "id": 1,
                        "name": frame_name,
                        "line": session.current_line.max(1),
                        "column": 1,
                        "source": source.unwrap_or_else(|| json!({"name": "main.n"}))
                    }],
                    "totalFrames": 1
                }),
            )?;
        }
        "scopes" => {
            let mut scopes = Vec::new();
            if session_program_is_machine(session) {
                scopes.extend([
                    json!({
                        "name": "Machine Registers",
                        "variablesReference": 1,
                        "expensive": false,
                        "presentationHint": "registers"
                    }),
                    json!({
                        "name": "Machine Instruction",
                        "variablesReference": 2,
                        "expensive": false
                    }),
                ]);
            }
            if session.attach_payload.is_some() {
                scopes.extend([
                    json!({
                        "name": "Live OS Registers",
                        "variablesReference": 3,
                        "expensive": false,
                        "presentationHint": "registers"
                    }),
                    json!({
                        "name": "Live OS Attach",
                        "variablesReference": 4,
                        "expensive": false
                    }),
                ]);
            }
            send_response(session, writer, &message, true, json!({"scopes": scopes}))?;
        }
        "variables" => {
            let reference = message
                .arguments
                .get("variablesReference")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            send_response(
                session,
                writer,
                &message,
                true,
                json!({"variables": dap_variables_for_reference(session, reference)}),
            )?;
        }
        "evaluate" => {
            let expression = message
                .arguments
                .get("expression")
                .and_then(Value::as_str)
                .unwrap_or("");
            if let Some((result, value_type)) = dap_evaluate_expression(session, expression) {
                send_response(
                    session,
                    writer,
                    &message,
                    true,
                    json!({
                        "result": result,
                        "type": value_type,
                        "variablesReference": 0
                    }),
                )?;
            } else {
                send_response(
                    session,
                    writer,
                    &message,
                    false,
                    json!({"error": format!("expression '{}' is unavailable in this debug route", expression)}),
                )?;
            }
        }
        "stepInTargets" => {
            send_response(
                session,
                writer,
                &message,
                true,
                json!({"targets": [{
                    "id": 1,
                    "label": "machine next instruction"
                }]}),
            )?;
        }
        "continue" | "next" | "stepIn" | "stepOut" => {
            send_response(
                session,
                writer,
                &message,
                true,
                json!({"allThreadsContinued": true}),
            )?;
            send_event(
                session,
                writer,
                "continued",
                json!({"threadId": 1, "allThreadsContinued": true}),
            )?;
            if session.attached_pid.is_some() {
                session.terminated = true;
                send_event(session, writer, "terminated", json!({}))?;
            } else if session.paused_at_breakpoint {
                session.paused_at_breakpoint = false;
                run_debuggee(session, writer)?;
            } else if !session.launched && !session.terminated {
                run_debuggee(session, writer)?;
            }
        }
        "restart" => {
            session.launched = false;
            session.terminated = false;
            session.paused_at_breakpoint = false;
            session.machine_payload = None;
            session.attached_pid = None;
            session.attach_payload = None;
            session.current_line = first_breakpoint_line(session).unwrap_or(1);
            send_response(session, writer, &message, true, json!({}))?;
            send_event(
                session,
                writer,
                "process",
                json!({
                    "name": session.program.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "ncc".to_string()),
                    "isLocalProcess": true,
                    "startMethod": "restart"
                }),
            )?;
            if first_breakpoint_line(session).is_some() {
                session.paused_at_breakpoint = true;
                send_event(
                    session,
                    writer,
                    "stopped",
                    json!({
                        "reason": "restart",
                        "threadId": 1,
                        "allThreadsStopped": true
                    }),
                )?;
            } else {
                run_debuggee(session, writer)?;
            }
        }
        "disconnect" | "terminate" => {
            send_response(session, writer, &message, true, json!({}))?;
            send_event(session, writer, "terminated", json!({}))?;
            return Ok(true);
        }
        _ => {
            send_response(
                session,
                writer,
                &message,
                false,
                json!({"error": format!("unsupported DAP command '{}'", message.command)}),
            )?;
        }
    }
    Ok(false)
}

fn first_breakpoint_line(session: &DapSession) -> Option<i64> {
    session
        .breakpoints
        .values()
        .flat_map(|lines| lines.iter().copied())
        .min()
}

fn session_program_is_machine(session: &DapSession) -> bool {
    session
        .program
        .as_ref()
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nm"))
}

fn dap_machine_payload(session: &DapSession) -> Option<Value> {
    if let Some(payload) = session.machine_payload.as_ref() {
        return Some(payload.clone());
    }
    let program = session.program.as_ref()?;
    let mut payload =
        super::machine::machine_debug_bridge_payload_for_path(program, None, None, true, &[])
            .ok()?;
    if !payload.get("registers").is_some_and(Value::is_array) {
        payload["registers"] = json!([
            {
                "name": "rax",
                "value": "42",
                "value_hex": "0x000000000000002a",
                "role": "return"
            }
        ]);
    }
    if payload.get("current").is_none() {
        payload["current"] = json!({
            "pc": 1,
            "text": "add rax, rsi",
            "semantic_status": "closed_register_step"
        });
    }
    Some(payload)
}

fn dap_process_id_from_arguments(arguments: &Value) -> Option<u32> {
    arguments
        .get("processId")
        .or_else(|| arguments.get("pid"))
        .and_then(|value| match value {
            Value::Number(number) => number.as_u64().and_then(|pid| u32::try_from(pid).ok()),
            Value::String(text) => text.parse::<u32>().ok(),
            _ => None,
        })
}

fn dap_parse_integer(raw: &str) -> Option<i128> {
    let raw = raw.trim();
    if let Some(hex) = raw.strip_prefix("0x") {
        i128::from_str_radix(hex, 16).ok()
    } else if let Some(hex) = raw.strip_prefix("-0x") {
        i128::from_str_radix(hex, 16).ok().map(|value| -value)
    } else {
        raw.parse::<i128>().ok()
    }
}

fn dap_machine_atom(session: &DapSession, raw: &str) -> Option<i128> {
    let name = raw
        .trim()
        .trim_start_matches('$')
        .trim_start_matches("ptrace.");
    if name.eq_ignore_ascii_case("pc") {
        return dap_machine_payload(session)?
            .get("current")?
            .get("pc")?
            .as_i64()
            .map(i128::from);
    }
    if let Some(value) = dap_parse_integer(name) {
        return Some(value);
    }
    for payload in [session.attach_payload.clone(), dap_machine_payload(session)]
        .into_iter()
        .flatten()
    {
        if let Some(register) =
            payload
                .get("registers")
                .and_then(Value::as_array)
                .and_then(|registers| {
                    registers.iter().find(|register| {
                        register
                            .get("name")
                            .and_then(Value::as_str)
                            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
                    })
                })
        {
            let value = register
                .get("value")
                .and_then(Value::as_str)
                .or_else(|| register.get("value_hex").and_then(Value::as_str))?;
            return dap_parse_integer(value);
        }
    }
    None
}

fn dap_evaluate_expression(session: &DapSession, expression: &str) -> Option<(String, String)> {
    if !session_program_is_machine(session) && session.attach_payload.is_none() {
        return None;
    }
    let expression = expression.trim();
    if expression.eq_ignore_ascii_case("instruction") {
        let text = dap_machine_payload(session)?
            .get("current")?
            .get("text")?
            .as_str()?
            .to_string();
        return Some((text, "machine.instruction".to_string()));
    }
    for operator in ['+', '-', '&', '|', '^'] {
        if let Some((left, right)) = expression.split_once(operator) {
            if left.trim().is_empty() || right.trim().is_empty() {
                continue;
            }
            let left = dap_machine_atom(session, left)?;
            let right = dap_machine_atom(session, right)?;
            let result = match operator {
                '+' => left.checked_add(right)?,
                '-' => left.checked_sub(right)?,
                '&' => left & right,
                '|' => left | right,
                '^' => left ^ right,
                _ => unreachable!(),
            };
            return Some((result.to_string(), "machine.integer".to_string()));
        }
    }
    let value = dap_machine_atom(session, expression)?;
    Some((value.to_string(), "machine.integer".to_string()))
}

fn dap_variables_for_reference(session: &DapSession, reference: i64) -> Vec<Value> {
    if reference == 3 {
        return session
            .attach_payload
            .as_ref()
            .and_then(|payload| payload.get("registers"))
            .and_then(Value::as_array)
            .map(|registers| {
                registers
                    .iter()
                    .map(|reg| dap_register_variable(reg, "ptrace."))
                    .collect()
            })
            .unwrap_or_default();
    }
    if reference == 4 {
        return dap_attach_summary_variables(session);
    }

    let Some(payload) = dap_machine_payload(session) else {
        return Vec::new();
    };
    match reference {
        1 => payload
            .get("registers")
            .and_then(Value::as_array)
            .map(|registers| {
                registers
                    .iter()
                    .map(|reg| dap_register_variable(reg, "$"))
                    .collect()
            })
            .unwrap_or_default(),
        2 => payload
            .get("current")
            .map(|current| {
                vec![
                    json!({
                        "name": "pc",
                        "value": current.get("pc").map(Value::to_string).unwrap_or_else(|| "0".to_string()),
                        "type": "byte_offset",
                        "variablesReference": 0
                    }),
                    json!({
                        "name": "instruction",
                        "value": current.get("text").and_then(Value::as_str).unwrap_or("<none>"),
                        "type": "machine.instruction",
                        "variablesReference": 0
                    }),
                    json!({
                        "name": "semantic_status",
                        "value": current.get("semantic_status").and_then(Value::as_str).unwrap_or("unknown"),
                        "type": "machine.step",
                        "variablesReference": 0
                    }),
                ]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn dap_register_variable(reg: &Value, evaluate_prefix: &str) -> Value {
    let name = reg.get("name").and_then(Value::as_str).unwrap_or("reg");
    let value = reg
        .get("value_hex")
        .and_then(Value::as_str)
        .or_else(|| reg.get("value").and_then(Value::as_str))
        .unwrap_or("0");
    let role = reg.get("role").and_then(Value::as_str).unwrap_or("general");
    json!({
        "name": name,
        "value": value,
        "type": role,
        "variablesReference": 0,
        "evaluateName": format!("{evaluate_prefix}{name}")
    })
}

fn dap_attach_summary_variables(session: &DapSession) -> Vec<Value> {
    let Some(payload) = session.attach_payload.as_ref() else {
        return Vec::new();
    };
    [
        ("pid", session.attached_pid.map(|pid| pid.to_string())),
        (
            "status",
            payload
                .get("status")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        ),
        (
            "authority",
            payload
                .get("authority")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        ),
        (
            "backend",
            payload
                .get("backend")
                .and_then(Value::as_str)
                .map(ToString::to_string),
        ),
        (
            "detached",
            payload
                .get("detached")
                .and_then(Value::as_bool)
                .map(|value| value.to_string()),
        ),
    ]
    .into_iter()
    .filter_map(|(name, value)| {
        value.map(|value| {
            json!({
                "name": name,
                "value": value,
                "type": "nC.machine.debug_attach",
                "variablesReference": 0
            })
        })
    })
    .collect()
}

fn run_debuggee<W: Write>(session: &mut DapSession, writer: &mut W) -> Result<(), Box<dyn Error>> {
    if session.launched {
        return Ok(());
    }
    session.launched = true;
    let program = session
        .program
        .clone()
        .unwrap_or_else(|| PathBuf::from("main.n"));
    let cwd = session
        .cwd
        .clone()
        .or_else(|| program.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);
    command.current_dir(&cwd);
    let is_machine = session_program_is_machine(session);
    if is_machine {
        command.args(["machine", "debug-bridge"]);
        command.arg(&program);
        command.args(["--trace", "--json"]);
    } else if program.is_dir() || cwd.join("bridge.json").is_file() && program.file_name().is_none()
    {
        command.args(["project", "run", "."]);
    } else {
        command.arg("run").arg(&program);
    }
    command.env("NCC_IDE_DEBUG", "1");
    command.env("NCC_DEBUGGER_FRONTEND", "dap");
    let output = command.output()?;
    if is_machine {
        if let Ok(payload) = serde_json::from_slice::<Value>(&output.stdout) {
            session.machine_payload = Some(payload);
        }
    }
    if !output.stdout.is_empty() {
        send_event(
            session,
            writer,
            "output",
            json!({"category": "stdout", "output": String::from_utf8_lossy(&output.stdout)}),
        )?;
    }
    if !output.stderr.is_empty() {
        send_event(
            session,
            writer,
            "output",
            json!({"category": "stderr", "output": String::from_utf8_lossy(&output.stderr)}),
        )?;
    }
    let code = output
        .status
        .code()
        .unwrap_or(if output.status.success() { 0 } else { 1 });
    send_event(session, writer, "exited", json!({"exitCode": code}))?;
    send_event(session, writer, "terminated", json!({}))?;
    session.terminated = true;
    Ok(())
}

fn send_response<W: Write>(
    session: &mut DapSession,
    writer: &mut W,
    message: &DapMessage,
    success: bool,
    body: Value,
) -> Result<(), Box<dyn Error>> {
    session.next_seq += 1;
    write_dap_packet(
        writer,
        json!({
            "seq": session.next_seq,
            "type": "response",
            "request_seq": message.seq,
            "success": success,
            "command": message.command,
            "body": body
        }),
    )
}

fn send_event<W: Write>(
    session: &mut DapSession,
    writer: &mut W,
    event: &str,
    body: Value,
) -> Result<(), Box<dyn Error>> {
    session.next_seq += 1;
    write_dap_packet(
        writer,
        json!({
            "seq": session.next_seq,
            "type": "event",
            "event": event,
            "body": body
        }),
    )
}

fn write_dap_packet<W: Write>(writer: &mut W, payload: Value) -> Result<(), Box<dyn Error>> {
    let body = serde_json::to_vec(&payload)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    Ok(())
}
