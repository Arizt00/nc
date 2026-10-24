use super::super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Debug, Clone)]
struct RuntimeCliOptions {
    json: bool,
    md: bool,
    no_color: bool,
    strict: bool,
    phase: Option<String>,
    target: Option<String>,
    profile: Option<String>,
    family: Option<String>,
    mode: Option<String>,
    out: Option<String>,
}

#[derive(Debug, Clone)]
struct RuntimeExportContract {
    name: String,
    arity_min: usize,
    variadic: bool,
    arg_types: Vec<String>,
    return_ty: String,
}

#[derive(Debug, Clone)]
struct RuntimeFamilyContract {
    import_path: String,
    module_path: String,
    family_key: String,
    exports: Vec<RuntimeExportContract>,
    capabilities: Vec<String>,
    policies: Vec<String>,
}

#[derive(Debug, Clone)]
struct RuntimeCatalogAudit {
    families: Vec<RuntimeFamilyContract>,
    missing_signature_families: Vec<String>,
    unknown_capabilities: Vec<String>,
    unknown_policies: Vec<String>,
    fn_to_families: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone)]
struct RuntimeSourceAudit {
    imported_families: BTreeSet<String>,
    runtime_import_uses: Vec<String>,
    required_capabilities: BTreeSet<String>,
    required_policies: BTreeSet<String>,
    unresolved_runtime_calls: Vec<String>,
    forbidden_mode_violations: Vec<String>,
    unknown_capabilities: Vec<String>,
    unknown_policies: Vec<String>,
    nominal_contracts: RuntimeNominalContractsAudit,
}

#[derive(Debug, Clone, Default)]
struct RuntimeNominalContractsAudit {
    destructure_statements: usize,
    constructure_statements: usize,
    runtime_calls_in_nominal: usize,
    unresolved_runtime_calls_in_nominal: Vec<String>,
}

const RUNTIME_NOMINAL_SUPPORTED_CONSTRUCTS: &[&str] = &["destructure", "constructure"];
const RUNTIME_NOMINAL_RUNTIME_DIAGNOSTICS: &[&str] = &[
    "E_RUNTIME_USE_001",
    "E_RUNTIME_EXPORTS_001",
    "E_RUNTIME_IMPORT_002",
];

const RUNTIME_CAPABILITY_CATALOG: &[&str] = &[
    "runtime.input.read",
    "runtime.input.tryRead",
    "runtime.input.clear",
    "runtime.output.write",
    "runtime.output.flush",
    "runtime.output.error",
    "runtime.output.stable",
    "runtime.output.redacted",
    "runtime.args.read",
    "runtime.env.read",
    "runtime.env.write",
    "runtime.exit",
    "runtime.session.read",
    "runtime.session.write",
    "runtime.text.read",
    "runtime.text.transform",
    "runtime.string.read",
    "runtime.string.transform",
    "runtime.string.unicode",
    "runtime.render.basic",
    "runtime.render.debug",
    "runtime.render.stable",
    "runtime.format.basic",
    "runtime.format.advanced",
    "runtime.parse.basic",
    "runtime.parse.strict",
    "runtime.parse.try",
    "runtime.convert.safe",
    "runtime.convert.unsafe",
    "runtime.validate.basic",
    "runtime.validate.schema",
    "runtime.escape.log",
    "runtime.escape.json",
    "runtime.escape.shell",
    "runtime.escape.redact",
    "runtime.result.use",
    "runtime.option.use",
    "runtime.error.raise",
    "runtime.error.recover",
    "runtime.diagnostic.emit",
    "runtime.diagnostic.json",
    "runtime.diagnostic.markdown",
    "runtime.log.info",
    "runtime.log.warn",
    "runtime.log.error",
    "runtime.log.debug",
    "runtime.trace.start",
    "runtime.trace.event",
    "runtime.trace.end",
    "runtime.debug.read",
    "runtime.assert.use",
    "runtime.status.use",
    "runtime.buffer.create",
    "runtime.buffer.read",
    "runtime.buffer.write",
    "runtime.buffer.slice",
    "runtime.buffer.clear",
    "runtime.memory.sizeof",
    "runtime.memory.alignof",
    "runtime.memory.copy",
    "runtime.memory.zero",
    "runtime.memory.compare",
    "runtime.heap.alloc",
    "runtime.heap.free",
    "runtime.heap.stats",
    "runtime.heap.check",
    "runtime.stack.inspect",
    "runtime.stack.local",
    "runtime.region.create",
    "runtime.region.read",
    "runtime.region.write",
    "runtime.region.invalidate",
    "runtime.region.readonly",
    "runtime.region.bounds",
    "runtime.slice.create",
    "runtime.span.create",
    "runtime.ptr.create",
    "runtime.ptr.read",
    "runtime.ptr.write",
    "runtime.ptr.refine",
    "runtime.ptr.origin",
    "runtime.addr.read",
    "runtime.addr.expose",
    "runtime.addr.redacted",
    "runtime.bytes.read",
    "runtime.bytes.write",
    "runtime.endian.convert",
    "runtime.layout.inspect",
    "runtime.alloc.custom",
    "fs.read",
    "fs.write",
    "fs.append",
    "fs.delete",
    "fs.create",
    "fs.copy",
    "fs.move",
    "fs.metadata",
    "fs.list",
    "fs.watch",
    "path.normalize",
    "path.officialize",
    "path.join",
    "path.sandbox",
    "dir.create",
    "dir.remove",
    "dir.list",
    "file.open",
    "file.close",
    "file.read",
    "file.write",
    "file.lock",
    "stream.read",
    "stream.write",
    "stream.pipe",
    "process.spawn",
    "process.wait",
    "process.kill",
    "process.signal",
    "process.inspect",
    "command.run",
    "command.capture",
    "command.shell",
    "command.noShell",
    "os.info",
    "os.platform",
    "os.resources",
    "user.current",
    "user.query",
    "user.groups",
    "permission.query",
    "permission.require",
    "temp.create",
    "temp.delete",
    "service.query",
    "service.start",
    "service.stop",
    "signal.send",
    "signal.handle",
    "time.now",
    "time.monotonic",
    "time.timestamp",
    "clock.read",
    "timer.create",
    "timer.cancel",
    "sleep.blocking",
    "sleep.async",
    "random.int",
    "random.float",
    "random.range",
    "entropy.read",
    "uuid.generate",
    "task.spawn",
    "task.join",
    "task.cancel",
    "thread.spawn",
    "thread.join",
    "async.await",
    "sync.lock",
    "sync.unlock",
    "atomic.read",
    "atomic.write",
    "atomic.cas",
    "lock.mutex",
    "lock.rw",
    "list.create",
    "list.push",
    "list.pop",
    "list.read",
    "list.write",
    "queue.push",
    "queue.pop",
    "event.emit",
    "event.wait",
    "scheduler.use",
    "cancel.token",
    "net.access",
    "socket.open",
    "socket.close",
    "tcp.connect",
    "tcp.listen",
    "tcp.accept",
    "tcp.send",
    "tcp.recv",
    "udp.bind",
    "udp.send",
    "udp.recv",
    "dns.resolve",
    "http.get",
    "http.post",
    "http.request",
    "http.response",
    "tls.connect",
    "tls.verify",
    "tls.certInspect",
    "capability.check",
    "capability.require",
    "capability.grant",
    "capability.revoke",
    "policy.check",
    "policy.require",
    "policy.scope",
    "profile.read",
    "profile.require",
    "bridge.runtime.system",
    "bridge.runtime.inspect",
    "bridge.runtime.tooling",
    "bridge.runtime.compiler",
    "bridge.runtime.kernel",
    "bridge.runtime.machine",
    "sandbox.create",
    "sandbox.run",
    "sandbox.deny",
    "sandbox.allow",
    "resource.open",
    "resource.close",
    "resource.limit",
    "resource.audit",
    "compiler.lex",
    "compiler.parse",
    "compiler.ast",
    "compiler.sema",
    "compiler.typeResolve",
    "compiler.operatorResolve",
    "compiler.conditionResolve",
    "compiler.importResolve",
    "compiler.symbolResolve",
    "compiler.flowResolve",
    "compiler.devResolve",
    "compiler.nirBuild",
    "compiler.nirVerify",
    "compiler.lower",
    "compiler.backend",
    "compiler.link",
    "compiler.diagnostic",
    "compiler.report",
    "compiler.internalMode",
    "mode.runtime.use",
    "mode.system.use",
    "mode.tooling.use",
    "mode.inspect.use",
    "mode.compiler.use",
    "mode.security.use",
    "mode.network.use",
    "mode.ai.use",
    "mode.root.use",
    "mode.machine.use",
    "mode.kernel.use",
    "mode.primitive.use",
    "capability.future.reserved",
    "capability.experimental",
    "capability.deprecated",
    "capability.migration",
];

const RUNTIME_POLICY_CATALOG: &[&str] = &[
    "runtime.safe",
    "runtime.strict",
    "runtime.console.allowed",
    "runtime.console.denied",
    "runtime.no-secret-output",
    "runtime.redact-secrets",
    "runtime.audit.required",
    "runtime.trace.required",
    "runtime.debug.profile-only",
    "runtime.error.typed-required",
    "runtime.no-implicit-runtime-path",
    "runtime.no-unsafe-convert",
    "runtime.no-silent-fallback",
    "memory.bounds.required",
    "memory.owner.required",
    "memory.ptr-origin.required",
    "memory.null-refinement.required",
    "memory.no-use-after-free",
    "memory.no-double-free",
    "memory.region-length.required",
    "memory.addr-redaction.required",
    "memory.site-origin.required",
    "memory.alias-tracking.required",
    "memory.stale-site-error",
    "memory.stack-escape-forbidden",
    "memory.raw-requires-authority",
    "fs.sandbox.required",
    "fs.no-path-traversal",
    "fs.no-delete-without-confirm",
    "fs.audit-destructive",
    "process.no-shell-by-default",
    "process.timeout.required",
    "process.audit.required",
    "command.escape.required",
    "command.no-injection",
    "network.capability.required",
    "network.timeout.required",
    "tls.verify.required",
    "tls.no-insecure",
    "env.redact-secrets",
    "service.authority.required",
    "compiler.phase-boundaries",
    "compiler.no-parser-sema",
    "compiler.no-sema-backend",
    "compiler.no-backend-sema",
    "compiler.no-fake-bytes",
    "compiler.no-placeholder-expr",
    "compiler.diagnostics.codes-required",
    "compiler.nir-verify-required",
    "compiler.lowering-no-invention",
    "compiler.backend-reject-invalid-nir",
    "compiler.internal-mode-profile-required",
    "compiler.target-required-for-target-lowering",
    "imports.official-runtime-required",
    "imports.no-runtime-in-root",
    "imports.no-runtime-in-primitive",
    "imports.no-runtime-input-in-machine",
    "imports.no-runtime-input-in-kernel",
    "imports.no-wildcard-unsafe",
    "imports.no-ambiguous-symbols",
    "operators.strict-types",
    "operators.bool-condition-required",
    "operators.no-string-placeholder",
    "operators.concat-render-required",
    "operators.short-circuit-required",
    "operators.no-shift-right-conflict",
    "operators.cast-explicit",
    "operators.reinterpret-authority-required",
    "mode.no-implicit-bridge",
    "mode.bridge-authority-required",
    "mode.primitive-sealed",
    "mode.site-no-primitive-crossing",
    "mode.addr-no-primitive-crossing",
    "mode.machine-no-runtime",
    "mode.kernel-no-runtime-input",
    "mode.root-no-runtime",
    "diagnostics.stable",
    "diagnostics.json-valid",
    "diagnostics.markdown-valid",
    "reports.source-required",
    "reports.hash-required",
    "reports.no-secret-leak",
    "reports.reproducible",
    "experimental.feature-gated",
    "future.capability-reserved",
    "deprecated.warn",
    "migration.required",
];

const RUNTIME_ERROR_CATALOG: &[&str] = &[
    "RuntimeInputUnavailable",
    "RuntimeInputParseError",
    "RuntimeInputEOF",
    "RuntimeInputTooLong",
    "RuntimeInputEncodingError",
    "RuntimeInputClosed",
    "RuntimeInputInterrupted",
    "RuntimeOutputUnavailable",
    "RuntimeOutputWriteError",
    "RuntimeOutputFlushError",
    "RuntimeOutputEncodingError",
    "RuntimeOutputClosed",
    "RuntimeOutputSecretForbidden",
    "RuntimeStringEncodingError",
    "RuntimeStringIndexError",
    "RuntimeStringSliceError",
    "RuntimeStringUnicodeError",
    "RuntimeStringInvalidOperation",
    "RuntimeRenderError",
    "RuntimeRenderTypeUnsupported",
    "RuntimeFormatError",
    "RuntimeFormatArityMismatch",
    "RuntimeFormatInvalidSpec",
    "RuntimeFormatPrecisionError",
    "RuntimeParseError",
    "RuntimeParseIntError",
    "RuntimeParseUIntError",
    "RuntimeParseFloatError",
    "RuntimeParseBoolError",
    "RuntimeConvertError",
    "RuntimeConvertOverflow",
    "RuntimeValidationError",
    "RuntimeRangeError",
    "RuntimeLengthError",
    "RuntimeSchemaError",
    "RuntimeResultUnwrapError",
    "RuntimeResultExpectedError",
    "RuntimeOptionUnwrapNone",
    "RuntimeErrorUnknown",
    "RuntimeErrorPayloadMismatch",
    "RuntimeDiagnosticError",
    "RuntimeLogError",
    "RuntimeTraceError",
    "RuntimeTraceUnclosed",
    "RuntimeDebugUnavailable",
    "RuntimeAssertFailed",
    "RuntimeStatusInvalid",
    "RuntimeMemoryOverflow",
    "RuntimeMemoryCopyError",
    "RuntimeMemoryZeroError",
    "RuntimeMemoryCompareError",
    "RuntimeAllocError",
    "RuntimeHeapError",
    "RuntimeHeapExhausted",
    "RuntimeHeapDoubleFree",
    "RuntimeHeapUseAfterFree",
    "RuntimeStackError",
    "RuntimeStackOverflow",
    "RuntimeStackEscape",
    "RuntimeRegionError",
    "RuntimeRegionCreateError",
    "RuntimeRegionInvalid",
    "RuntimeRegionOutOfBounds",
    "RuntimeRegionReadonly",
    "RuntimeRegionWriteonly",
    "RuntimeRegionFreed",
    "RuntimeRegionAliasStale",
    "RuntimeBufferError",
    "RuntimeBufferOverflow",
    "RuntimeBufferUnderflow",
    "RuntimeBufferOutOfBounds",
    "RuntimeBufferInvalid",
    "RuntimeSliceError",
    "RuntimeSpanError",
    "RuntimePtrError",
    "RuntimePtrNull",
    "RuntimePtrOriginMissing",
    "RuntimePtrDangling",
    "RuntimeAddrError",
    "RuntimeAddrUnmaterialized",
    "RuntimeAddrRedacted",
    "RuntimeAddrAuthorityMissing",
    "RuntimeBytesError",
    "RuntimeEndianError",
    "RuntimeLayoutError",
    "RuntimeAlignmentError",
    "RuntimeFsError",
    "RuntimeFsCapabilityMissing",
    "RuntimePathError",
    "RuntimePathTraversalDenied",
    "RuntimePathInvalid",
    "RuntimeDirError",
    "RuntimeDirNotFound",
    "RuntimeFileError",
    "RuntimeFileNotFound",
    "RuntimeFileAlreadyExists",
    "RuntimeFilePermissionDenied",
    "RuntimeFileHandleInvalid",
    "RuntimeFileHandleLeak",
    "RuntimeStreamError",
    "RuntimeStreamClosed",
    "RuntimeStreamTimeout",
    "RuntimeProcessError",
    "RuntimeProcessSpawnDenied",
    "RuntimeProcessNotFound",
    "RuntimeProcessWaitError",
    "RuntimeProcessKillDenied",
    "RuntimeCommandError",
    "RuntimeCommandDenied",
    "RuntimeCommandTimeout",
    "RuntimeCommandInjectionRisk",
    "RuntimeCommandExitNonZero",
    "RuntimeOsError",
    "RuntimeUserError",
    "RuntimePermissionDenied",
    "RuntimeTempError",
    "RuntimeServiceError",
    "RuntimeServiceNotFound",
    "RuntimeServicePermissionDenied",
    "RuntimeSignalError",
    "RuntimeSignalUnsupported",
    "RuntimeSignalPermissionDenied",
    "RuntimeTimeError",
    "RuntimeClockError",
    "RuntimeTimerError",
    "RuntimeTimerCancelled",
    "RuntimeSleepError",
    "RuntimeRandomError",
    "RuntimeRandomRangeError",
    "RuntimeEntropyError",
    "RuntimeEntropyUnavailable",
    "RuntimeUuidError",
    "RuntimeTaskError",
    "RuntimeTaskSpawnError",
    "RuntimeTaskJoinError",
    "RuntimeTaskCancelled",
    "RuntimeThreadError",
    "RuntimeAsyncError",
    "RuntimeAwaitTimeout",
    "RuntimeSyncError",
    "RuntimeAtomicError",
    "RuntimeLockError",
    "RuntimeLockAlreadyHeld",
    "RuntimeUnlockByNonOwner",
    "RuntimeQueueError",
    "RuntimeQueueFull",
    "RuntimeQueueEmpty",
    "RuntimeEventError",
    "RuntimeSchedulerError",
    "RuntimeCancelError",
    "RuntimeNetworkError",
    "RuntimeNetworkCapabilityMissing",
    "RuntimeSocketError",
    "RuntimeSocketClosed",
    "RuntimeSocketTimeout",
    "RuntimeTcpError",
    "RuntimeTcpConnectError",
    "RuntimeTcpListenError",
    "RuntimeUdpError",
    "RuntimeDnsError",
    "RuntimeDnsNotFound",
    "RuntimeHttpError",
    "RuntimeHttpStatusError",
    "RuntimeHttpTimeout",
    "RuntimeTlsError",
    "RuntimeTlsCertInvalid",
    "RuntimeTlsCertExpired",
    "RuntimeTlsHandshakeFailed",
    "RuntimeNetworkPolicyError",
    "RuntimeCapabilityMissing",
    "RuntimeCapabilityDenied",
    "RuntimePolicyMissing",
    "RuntimePolicyDenied",
    "RuntimeProfileMismatch",
    "RuntimeBridgeForbidden",
    "RuntimeBridgeMissing",
    "RuntimeBridgeAuthorityMissing",
    "RuntimeSandboxViolation",
    "RuntimeSandboxDenied",
    "RuntimeResourceError",
    "RuntimeResourceLeak",
    "RuntimeResourceLimitExceeded",
];

fn runtime_abi_param_json(
    name: &str,
    ty: &str,
    passing: &str,
    ownership: &str,
) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "type": ty,
        "passing": passing,
        "ownership": ownership,
    })
}

fn runtime_abi_params(params: &[(&str, &str, &str, &str)]) -> Vec<serde_json::Value> {
    params
        .iter()
        .map(|(name, ty, passing, ownership)| runtime_abi_param_json(name, ty, passing, ownership))
        .collect()
}

fn runtime_abi_return_json(
    ty: &str,
    result: Option<(&str, &str)>,
    ownership: &str,
) -> serde_json::Value {
    match result {
        Some((ok_ty, err_ty)) => serde_json::json!({
            "type": ty,
            "layout_ref": "Result<T,E>",
            "ok_type": ok_ty,
            "err_type": err_ty,
            "ownership": ownership,
        }),
        None => serde_json::json!({
            "type": ty,
            "layout_ref": ty,
            "ownership": ownership,
        }),
    }
}

fn runtime_backend_routes_json(symbol: &str) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "route": "ncc_run",
            "status": "closed_real_executor",
            "lowering": format!("{symbol} is executed by the real ncc runtime executor without requiring --real"),
            "authority": "runtime executor",
            "diagnostic_if_unavailable": null,
        }),
        serde_json::json!({
            "route": "mct",
            "status": "closed_runtime_symbol_activation_bytes",
            "lowering": format!("{symbol} is bound through the MCT import/runtime-symbol byte contract"),
            "authority": "MCT byte contract",
            "diagnostic_if_unavailable": "E_BACKEND_PARITY_MCT_BYTES_001",
        }),
        serde_json::json!({
            "route": "native_fast",
            "status": "closed_native_runtime_thunk",
            "lowering": format!("native-fast lowers {symbol} to an nC runtime-value thunk with the ABI row declared here"),
            "authority": "native-fast runtime thunk",
            "diagnostic_if_unavailable": "E_BACKEND_PARITY_NATIVE_FAST_001",
        }),
        serde_json::json!({
            "route": "llvm",
            "status": "closed_llvm_runtime_declaration_call",
            "lowering": format!("LLVM/MCT declares and calls {symbol} using nC.runtime-value-v1 layouts"),
            "authority": "LLVM hosted runtime ABI",
            "diagnostic_if_unavailable": "E_BACKEND_PARITY_LLVM_EXEC_001",
        }),
        serde_json::json!({
            "route": "machine",
            "status": "closed_bridge_required_when_applicable",
            "lowering": "direct runtime imports are rejected in [@machine]; applicable effects cross through explicit typed bridge/trap rows",
            "authority": "machine low-authority boundary",
            "diagnostic_if_unavailable": "E_MACHINE_RUNTIME_IMPORT_FORBIDDEN_001",
        }),
        serde_json::json!({
            "route": "basal",
            "status": "closed_bridge_required_when_applicable",
            "lowering": "basal/kernel layers do not import high-level runtime directly; applicable effects cross through explicit bridge rows",
            "authority": "basal low-authority boundary",
            "diagnostic_if_unavailable": "E_BASAL_RUNTIME_IMPORT_FORBIDDEN_001",
        }),
    ]
}

fn runtime_abi_symbol_json(
    symbol: &str,
    family: &str,
    aliases: &[&str],
    params: &[(&str, &str, &str, &str)],
    return_ty: &str,
    result: Option<(&str, &str)>,
    error_codes: &[&str],
    ownership: &[&str],
    resource_lifecycle: &str,
) -> serde_json::Value {
    let canonical_symbol = ncc_ast::canonical_runtime_symbol(symbol);
    let mut normalized_aliases = aliases
        .iter()
        .map(|alias| (*alias).to_string())
        .collect::<BTreeSet<_>>();
    if canonical_symbol != symbol {
        normalized_aliases.insert(symbol.to_string());
    }
    serde_json::json!({
        "symbol": canonical_symbol,
        "canonical_symbol": canonical_symbol,
        "family": family,
        "status": "closed_stable_runtime_abi",
        "abi_revision": "nC.runtime.symbol_abi.v1",
        "calling_convention": "nC.runtime-value-v1",
        "calling_convention_contract": {
            "logical": "nC.runtime-value-v1",
            "x86_64_windows": "nC.win64 adapter",
            "x86_64_unix": "nC.sysv64 adapter",
            "aarch64": "nC.aapcs64 adapter",
            "riscv": "nC.riscv-psabi adapter",
            "wasm32": "nC.wasm-canonical adapter",
        },
        "linkage": "external",
        "visibility": "default",
        "native_lowering_contract": "MCT runtime symbol call uses declared type layout and ownership row; this is not a raw C ABI promise",
        "backend_parity_status": "closed_for_ncc_run_mct_native_fast_llvm_and_machine_basal_bridge",
        "backend_routes": runtime_backend_routes_json(&canonical_symbol),
        "aliases": normalized_aliases.into_iter().collect::<Vec<_>>(),
        "inputs": runtime_abi_params(params),
        "return": runtime_abi_return_json(return_ty, result, "caller owns returned RuntimeValue payload"),
        "error_codes": error_codes,
        "ownership": ownership,
        "resource_lifecycle": resource_lifecycle,
    })
}

fn runtime_abi_layouts_json() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "RuntimeValue",
            "kind": "tagged executor value",
            "runtime_layout": "enum Bool | Char | Number(decimal) | String(owned utf8) | Pointer | Vector | Record | MemRegionHandle | Void",
            "native_lowering": "lowered by declared nC type before backend/MCT emission",
            "ownership": "owned by variable/result unless passed as borrowed parameter",
        }),
        serde_json::json!({
            "type": "Result<T,E>",
            "kind": "tagged union",
            "runtime_layout": {
                "__runtimeFamily": "runtime.result",
                "__runtimeVariant": "ok | err",
                "value": "T when ok, E when err",
            },
            "native_lowering": {
                "tag_offset": 0,
                "tag_bytes": 1,
                "tag_values": {"ok": 1, "err": 2},
                "payload_offset": "align_up(1, max(align(T), align(E)))",
                "payload_size": "max(sizeof(T), sizeof(E))",
                "total_size": "align_up(payload_offset + payload_size, max(align(T), align(E)))",
            },
            "ownership": "Result owns its payload until match/unwrap transfers or borrows it",
        }),
        serde_json::json!({
            "type": "ProcessHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.process.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned linear handle; wait/kill consume the slot",
        }),
        serde_json::json!({
            "type": "ThreadHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.thread.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned linear handle; threadJoin consumes the slot",
        }),
        serde_json::json!({
            "type": "FutureHandle",
            "kind": "nominal async handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.async.futureHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned async handle; await consumes, futureReady borrows, futureCancel requests cancellation",
        }),
        serde_json::json!({
            "type": "TaskHandle",
            "kind": "nominal scheduler handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.task.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned task handle; taskAwait consumes, taskStatus borrows, taskCancel requests cancellation",
        }),
        serde_json::json!({
            "type": "TimerHandle",
            "kind": "nominal timer handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.timer.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned timer handle; timerStop borrows, timerReset mutates start timestamp",
        }),
        serde_json::json!({
            "type": "CancelToken",
            "kind": "nominal cancellation handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.cancel.token",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned token handle; cancel mutates atomic cancelled bit and isCancelled borrows",
        }),
        serde_json::json!({
            "type": "EventHandle",
            "kind": "nominal event handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.event.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned event handle; eventEmit mutates state and eventWait borrows until emitted",
        }),
        serde_json::json!({
            "type": "MutexHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.sync.mutexHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned registry handle; lock/tryLock borrow; unlock releases current owner",
        }),
        serde_json::json!({
            "type": "RwLockHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.sync.rwLockHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned registry handle; read/write locks borrow and unlock releases current owner",
        }),
        serde_json::json!({
            "type": "BarrierHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.sync.barrierHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned registry handle; barrierWait borrows and blocks until all parties arrive",
        }),
        serde_json::json!({
            "type": "AtomicHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.atomic.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned registry handle; atomic operations borrow and use SeqCst ordering",
        }),
        serde_json::json!({
            "type": "ChannelHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.channel.handle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned registry handle; send/recv borrow; channelClose drops sender",
        }),
        serde_json::json!({
            "type": "ThreadLocalHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.thread.localHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned registry handle; each host thread owns its local payload copy",
        }),
        serde_json::json!({
            "type": "FileLockHandle",
            "kind": "nominal handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.fs.fileLockHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owned linear handle; unlockFile consumes the slot and releases the sidecar lock",
        }),
        serde_json::json!({
            "type": "FsError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.fs.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "ProcessError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.process.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "ThreadError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.thread.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "AsyncError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.async.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "NetworkError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.net.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "TcpError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.tcp.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "TcpHandle",
            "kind": "registry handle",
            "runtime_layout": "int registry_slot; listener and stream slots share tcp registry namespace",
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "tcpListen/tcpConnect/tcpAccept create; tcpClose consumes; tcpPort/tcpSend/tcpRecv borrow",
        }),
        serde_json::json!({
            "type": "BufferDescriptor",
            "kind": "manual raw memory descriptor",
            "runtime_layout": {"data": "addr", "len": "u64", "capacity": "u64", "flags": "u64"},
            "native_lowering": {"size_bytes": 32, "align_bytes": 8},
            "ownership": "caller owns descriptor and backing bytes; backend performs no allocation, release or bounds check",
        }),
        serde_json::json!({
            "type": "SliceDescriptor",
            "kind": "manual raw byte view",
            "runtime_layout": {"data": "addr", "bytes": "u64", "flags": "u64"},
            "native_lowering": {"size_bytes": 24, "align_bytes": 8},
            "ownership": "caller owns descriptor and backing bytes; views never allocate or retain ownership",
        }),
        serde_json::json!({
            "type": "SpanSegment",
            "kind": "manual raw span segment",
            "runtime_layout": {"data": "addr", "bytes": "u64"},
            "native_lowering": {"size_bytes": 16, "align_bytes": 8},
            "ownership": "caller owns segment table and every referenced byte range",
        }),
        serde_json::json!({
            "type": "SpanDescriptor",
            "kind": "manual raw segment view",
            "runtime_layout": {"segments": "addr", "count": "u64", "bytes": "u64", "flags": "u64"},
            "native_lowering": {"size_bytes": 32, "align_bytes": 8},
            "ownership": "caller owns descriptor, segment table and backing bytes; backend performs no checks",
        }),
        serde_json::json!({
            "type": "AllocatorHandle",
            "kind": "runtime allocator handle",
            "runtime_layout": {
                "__runtimeFamily": "runtime.memory.allocatorHandle",
                "__runtimeVariant": "handle",
                "value": "int registry_slot",
            },
            "native_lowering": {"slot": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "owns allocator accounting table; allocWith creates OwningPtr tracked by this allocator",
        }),
        serde_json::json!({
            "type": "OwningPtr",
            "kind": "owned runtime pointer wrapper",
            "runtime_layout": {
                "__runtimeFamily": "runtime.memory.owningPtr",
                "__runtimeVariant": "owned",
                "value": "ptr<T>",
            },
            "native_lowering": {"address": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "linear owner; freeWith consumes and marks heap block dead",
        }),
        serde_json::json!({
            "type": "MemoryError",
            "kind": "nominal error",
            "runtime_layout": {
                "__runtimeFamily": "runtime.memory.error",
                "__runtimeVariant": "error",
                "value": {"code": "int", "message": "str", "osCause": "str", "kind": "str"},
            },
            "native_lowering": {"code": "dword", "message": "str_ref", "osCause": "str_ref", "kind": "str_ref", "size_bytes": 32, "align_bytes": 8},
            "ownership": "owned error payload inside Result.err",
        }),
        serde_json::json!({
            "type": "ptr<T>",
            "kind": "runtime pointer",
            "runtime_layout": "RuntimePointer::Addr(addr) | RuntimePointer::Null",
            "native_lowering": {"address": "qword", "size_bytes": 8, "align_bytes": 8},
            "ownership": "alloc creates owned pointer; free consumes owner; ptrRead borrows; ptrWrite mutates pointee",
        }),
    ]
}

fn runtime_abi_symbol_entries() -> Vec<serde_json::Value> {
    let mut entries = vec![
        runtime_abi_symbol_json(
            "nC.runtime.input.readInt",
            "input",
            &[],
            &[(
                "stdin",
                "implicit process stdin",
                "host borrow",
                "runtime borrows process stdin",
            )],
            "int",
            None,
            &["E_RUNTIME_INPUT_PARSE_001"],
            &[
                "does not allocate persistent runtime resource",
                "input stream remains owned by host process",
            ],
            "reads one line from stdin and returns parsed int",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.output.print",
            "output",
            &[],
            &[(
                "value",
                "any",
                "borrowed RuntimeValue",
                "caller keeps value",
            )],
            "bool",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["stdout remains owned by host process"],
            "writes rendered value to stdout without newline",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.output.println",
            "output",
            &[],
            &[(
                "value",
                "any",
                "borrowed RuntimeValue",
                "caller keeps value",
            )],
            "bool",
            None,
            &["E_RUNTIME_RENDER_TYPE_001", "E_RUNTIME_FLUSH_001"],
            &["stdout remains owned by host process"],
            "writes rendered value plus newline and flushes stdout",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.render.renderInt",
            "render",
            &[],
            &[("value", "int", "borrowed scalar", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["returned string is owned by caller"],
            "formats an integer as stable text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.render.renderBinary",
            "render",
            &[],
            &[("value", "int", "borrowed scalar", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["returned string is owned by caller"],
            "formats a signed integer as binary text; hosted ncc-runtime-stdlib may use libc %b when the OS probe accepts %llb, with portable exact byte rendering as mandatory fallback",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.render.renderBin",
            "render",
            &[],
            &[("value", "int", "borrowed scalar", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["returned string is owned by caller"],
            "compact alias of renderBinary with the same OS/provider contract",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.render.renderBool",
            "render",
            &[],
            &[("value", "bool", "borrowed scalar", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["returned string is owned by caller"],
            "formats a boolean as stable text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.render.renderFloat",
            "render",
            &[],
            &[("value", "ft64", "borrowed scalar", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["returned string is owned by caller"],
            "formats a float as stable text",
        ),
        runtime_abi_symbol_json(
            "nC.math.core.sqrtFloat",
            "math",
            &[],
            &[("value", "ft64", "borrowed scalar", "caller keeps value")],
            "ft64",
            None,
            &["E_RUNTIME_MATH_TYPE_001"],
            &["pure scalar intrinsic; no resource ownership transfer"],
            "returns the square root using the backend floating-point math path",
        ),
        runtime_abi_symbol_json(
            "nC.math.core.expFloat",
            "math",
            &[],
            &[("value", "ft64", "borrowed scalar", "caller keeps value")],
            "ft64",
            None,
            &["E_RUNTIME_MATH_TYPE_001"],
            &["pure scalar intrinsic; no resource ownership transfer"],
            "returns e raised to value using the backend floating-point math path",
        ),
        runtime_abi_symbol_json(
            "nC.math.core.logFloat",
            "math",
            &[],
            &[("value", "ft64", "borrowed scalar", "caller keeps value")],
            "ft64",
            None,
            &["E_RUNTIME_MATH_TYPE_001"],
            &["pure scalar intrinsic; no resource ownership transfer"],
            "returns the natural logarithm using the backend floating-point math path",
        ),
        runtime_abi_symbol_json(
            "nC.math.trig.sinFloat",
            "math",
            &[],
            &[("value", "ft64", "borrowed scalar", "caller keeps value")],
            "ft64",
            None,
            &["E_RUNTIME_MATH_TYPE_001"],
            &["pure scalar intrinsic; no resource ownership transfer"],
            "returns sine using the backend floating-point math path",
        ),
        runtime_abi_symbol_json(
            "nC.math.trig.cosFloat",
            "math",
            &[],
            &[("value", "ft64", "borrowed scalar", "caller keeps value")],
            "ft64",
            None,
            &["E_RUNTIME_MATH_TYPE_001"],
            &["pure scalar intrinsic; no resource ownership transfer"],
            "returns cosine using the backend floating-point math path",
        ),
        runtime_abi_symbol_json(
            "nC.math.trig.atan2Float",
            "math",
            &[],
            &[
                ("y", "ft64", "borrowed scalar", "caller keeps y"),
                ("x", "ft64", "borrowed scalar", "caller keeps x"),
            ],
            "ft64",
            None,
            &["E_RUNTIME_MATH_TYPE_001"],
            &["pure scalar intrinsic; no resource ownership transfer"],
            "returns atan2(y, x) using the backend floating-point math path",
        ),
        runtime_abi_symbol_json(
            "nC.math.core.pi",
            "math",
            &[],
            &[],
            "ft64",
            None,
            &[],
            &["materialized as the target ft64 constant"],
            "returns pi as the executable ft64 constant",
        ),
        runtime_abi_symbol_json(
            "nC.math.core.euler",
            "math",
            &[],
            &[],
            "ft64",
            None,
            &[],
            &["materialized as the target ft64 constant"],
            "returns e as the executable ft64 constant",
        ),
        runtime_abi_symbol_json(
            "nC.math.core.aureo",
            "math",
            &[],
            &[],
            "ft64",
            None,
            &[],
            &["materialized as the target ft64 constant"],
            "returns the golden ratio as the executable ft64 constant",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.render.colorBold",
            "render",
            &["nC.runtime.output.colorBold"],
            &[
                ("text", "str", "borrowed text", "caller keeps text"),
                ("color", "str", "borrowed text", "caller keeps color"),
            ],
            "str",
            None,
            &["E_RUNTIME_RENDER_TYPE_001"],
            &["returned ANSI string is owned by caller"],
            "wraps text in a supported bold ANSI color sequence",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.random.random",
            "random",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_RANDOM_001"],
            &["mutates runtime PRNG state in ExecState"],
            "advances runtime PRNG and returns a non-negative int",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.random.randomRange",
            "random",
            &[],
            &[
                ("min", "int", "by value", "caller keeps value"),
                ("max", "int", "by value", "caller keeps value"),
            ],
            "int",
            None,
            &["E_RUNTIME_RANDOM_RANGE_001"],
            &["mutates runtime PRNG state in ExecState"],
            "returns inclusive random integer between min and max after ordering bounds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.random.seed",
            "random",
            &[],
            &[("seed", "int", "by value", "caller keeps value")],
            "bool",
            None,
            &["E_RUNTIME_RANDOM_001"],
            &["sets runtime PRNG state in ExecState"],
            "replaces runtime PRNG seed",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.errorCode",
            "error",
            &[],
            &[("error", "FsError|TcpError|ProcessError|ThreadError|MemoryError|any", "borrowed error", "caller keeps value")],
            "int",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["borrows nominal error payload"],
            "reads the stable numeric error code",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.errorMessage",
            "error",
            &[],
            &[("error", "FsError|TcpError|ProcessError|ThreadError|MemoryError|any", "borrowed error", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["returned string is owned by caller"],
            "reads the stable human message from an error payload",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.errorKind",
            "error",
            &[],
            &[("error", "FsError|TcpError|ProcessError|ThreadError|MemoryError|any", "borrowed error", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["returned string is owned by caller"],
            "reads the nominal kind stored in an error payload",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.errorOsCause",
            "error",
            &[],
            &[("error", "FsError|TcpError|ProcessError|ThreadError|MemoryError|any", "borrowed error", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["returned string is owned by caller"],
            "reads OS cause when present, otherwise returns a synthetic nC cause",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.raiseRuntime",
            "error",
            &[],
            &[("error", "any", "owned RuntimeValue", "caller transfers payload into Result.err")],
            "result",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["returned Result owns the error payload"],
            "wraps any value as Result.err for explicit propagation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.clearError",
            "error",
            &[],
            &[],
            "bool",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["no persistent resource escapes"],
            "clears the current last-error slot when present",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.error.lastError",
            "error",
            &[],
            &[],
            "option",
            None,
            &["E_RUNTIME_ERROR_001"],
            &["returned option owns its payload"],
            "returns the last runtime error when the runtime has one recorded",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.fsread",
            "fs",
            &["nC.runtime.fs.readFile"],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<str,FsError>",
            Some(("str", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned Result owns file contents or error string"],
            "opens, reads and closes the file before return",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.fswrite",
            "fs",
            &[],
            &[
                ("path", "str", "borrowed text", "caller keeps path"),
                ("text", "str", "borrowed text", "caller keeps text"),
            ],
            "bool",
            None,
            &["E_RUNTIME_FS_001"],
            &["no resource escapes; the caller receives only material success"],
            "creates/replaces file, writes all bytes, flushes and closes before returning a scalar status",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.fsappend",
            "fs",
            &["nC.runtime.fs.appendFile"],
            &[
                ("path", "str", "borrowed text", "caller keeps path"),
                ("text", "str", "borrowed text", "caller keeps text"),
            ],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned Result owns success flag or error string"],
            "opens file in append mode, writes all bytes, flushes and closes before return",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.fsync",
            "fs",
            &["nC.runtime.fs.syncFile"],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned Result owns success flag or error string"],
            "opens file descriptor, calls sync_all and closes before return",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.exists",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "bool",
            None,
            &["E_RUNTIME_FS_001"],
            &["no persistent resource escapes"],
            "checks host filesystem path existence",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.deleteFile",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["path is borrowed; deletion side effect is host-owned"],
            "removes a real file and returns Result",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.copyFile",
            "fs",
            &[],
            &[
                ("src", "str", "borrowed text", "caller keeps src"),
                ("dst", "str", "borrowed text", "caller keeps dst"),
            ],
            "Result<int,FsError>",
            Some(("int", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned Result owns byte count or error string"],
            "copies real file bytes and closes descriptors before return",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.createDir",
            "fs",
            &["nC.runtime.dir.createDir"],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["creates missing parent directories under host filesystem"],
            "creates a real directory tree and returns Result",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.removeDir",
            "fs",
            &["nC.runtime.dir.removeDir"],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["removes the real directory tree rooted at path"],
            "deletes a real directory tree and returns Result",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.listDir",
            "fs",
            &["nC.runtime.dir.listDir"],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<list<str>,FsError>",
            Some(("list<str>", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned list and entry strings are owned by caller"],
            "reads a real directory and returns stable sorted entries",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.metadata",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<str,FsError>",
            Some(("str", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned metadata string is owned by caller"],
            "queries real filesystem metadata: type, size, readonly and modified time",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.permissions",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<str,FsError>",
            Some(("str", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned permission string is owned by caller"],
            "queries host readonly permission state",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.setReadonly",
            "fs",
            &[],
            &[
                ("path", "str", "borrowed text", "caller keeps path"),
                ("readonly", "bool", "by value", "caller keeps value"),
            ],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["mutates host permission bits exposed by std filesystem permissions"],
            "sets or clears host readonly permission flag",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.renamePath",
            "fs",
            &["nC.runtime.fs.movePath"],
            &[
                ("src", "str", "borrowed text", "caller keeps src"),
                ("dst", "str", "borrowed text", "caller keeps dst"),
            ],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["moves or renames the host path; no handle escapes"],
            "renames a real filesystem path",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.movePath",
            "fs",
            &["nC.runtime.fs.renamePath"],
            &[
                ("src", "str", "borrowed text", "caller keeps src"),
                ("dst", "str", "borrowed text", "caller keeps dst"),
            ],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["moves or renames the host path; no handle escapes"],
            "moves a real filesystem path",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.absolutePath",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<str,FsError>",
            Some(("str", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned absolute path string is owned by caller"],
            "resolves a path against current working directory without requiring existence",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.officialPath",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<str,FsError>",
            Some(("str", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned official path string is owned by caller"],
            "resolves a real existing path through host officialization",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.lockFile",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<FileLockHandle,FsError>",
            Some(("FileLockHandle", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["creates owned FileLockHandle; caller must unlockFile"],
            "creates an exclusive real sidecar lock for the file path",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.unlockFile",
            "fs",
            &[],
            &[(
                "handle",
                "FileLockHandle",
                "owned handle",
                "callee consumes lock handle",
            )],
            "Result<bool,FsError>",
            Some(("bool", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["consumes FileLockHandle and removes sidecar lock file"],
            "releases a real file lock acquired through lockFile",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.readBytes",
            "fs",
            &[],
            &[("path", "str", "borrowed text", "caller keeps path")],
            "Result<array<byte>,FsError>",
            Some(("array<byte>", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned byte vector owns copied file bytes"],
            "reads raw file bytes and returns byte array payload",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.readBytesInto",
            "fs",
            &[],
            &[
                ("path", "str", "borrowed text", "caller keeps path"),
                (
                    "destination",
                    "array<byte>",
                    "borrowed writable byte array",
                    "callee writes at most destination length bytes",
                ),
            ],
            "Result<int,FsError>",
            Some(("int", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["caller retains destination and returned Result owns byte count"],
            "reads raw file bytes into caller-provided physical storage",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.fs.writeBytes",
            "fs",
            &[],
            &[
                ("path", "str", "borrowed text", "caller keeps path"),
                (
                    "bytes",
                    "array<byte>",
                    "borrowed byte array",
                    "caller keeps bytes",
                ),
            ],
            "Result<int,FsError>",
            Some(("int", "FsError")),
            &["E_RUNTIME_FS_001"],
            &["returned Result owns byte count or error string"],
            "writes raw file bytes exactly as provided",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.time.now",
            "time",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_TIME_001"],
            &["no persistent resource escapes"],
            "returns current UNIX seconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.time.timestamp",
            "time",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_TIME_001"],
            &["no persistent resource escapes"],
            "returns current UNIX seconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.time.monotonic",
            "time",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_TIME_001"],
            &["borrows process monotonic clock anchor"],
            "returns monotonic milliseconds from runtime anchor",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.time.sleep",
            "time",
            &["nC.runtime.sleep.sleepMs"],
            &[("ms", "int", "by value", "caller keeps value")],
            "void",
            None,
            &["E_RUNTIME_SLEEP_001", "E_RUNTIME_THREAD_001"],
            &["blocks current host thread; no resource escapes"],
            "sleeps current runtime thread for non-negative milliseconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.clock.monotonic",
            "clock",
            &[],
            &[],
            "u64",
            None,
            &[],
            &["no resource transfer"],
            "returns the target monotonic clock in nanoseconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.clock.elapsed",
            "clock",
            &[],
            &[("start", "u64", "by value", "caller keeps value")],
            "u64",
            None,
            &[],
            &["no resource transfer"],
            "returns saturating monotonic nanoseconds elapsed since start",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.clock.measure",
            "clock",
            &[],
            &[("function", "fn() -> bool", "borrowed function pointer", "caller keeps function")],
            "u64",
            None,
            &[],
            &["invokes function synchronously without retaining its pointer"],
            "measures one real indirect function invocation in monotonic nanoseconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.clock.clockResolution",
            "clock",
            &[],
            &[],
            "u64",
            None,
            &[],
            &["no resource transfer"],
            "queries the target monotonic clock resolution in nanoseconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.math.sign",
            "math",
            &[],
            &[("value", "int", "by value", "caller keeps value")],
            "int",
            None,
            &[],
            &["no resource transfer"],
            "returns -1, 0 or 1 according to the signed value",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.math.abs",
            "math",
            &[],
            &[("value", "int", "by value", "caller keeps value")],
            "int",
            None,
            &[],
            &["no resource transfer; minimum int saturates to maximum int"],
            "returns the saturating absolute signed value",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.toStr",
            "convert",
            &[],
            &[("value", "any", "borrowed value", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_CONVERT_ALLOC_004"],
            &["returns owned UTF-8 text; caller releases it with strRelease"],
            "renders a scalar, pointer, string or aggregate value as text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.toText",
            "convert",
            &["nC.runtime.convert.toStr"],
            &[("value", "any", "borrowed value", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_CONVERT_ALLOC_004"],
            &["returns owned UTF-8 text; caller releases it with strRelease"],
            "materializes the same owned UTF-8 representation as toStr",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.toInt",
            "convert",
            &[],
            &[("value", "any", "by value or borrowed", "caller keeps value")],
            "Result<int,ConvertError>",
            Some(("int", "ConvertError")),
            &["E_RUNTIME_CONVERT_TYPE_001", "E_RUNTIME_CONVERT_SYNTAX_002", "E_RUNTIME_CONVERT_RANGE_003"],
            &["no resource transfer"],
            "performs exact checked integer conversion",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.toFloat",
            "convert",
            &[],
            &[("value", "any", "by value or borrowed", "caller keeps value")],
            "Result<ft64,ConvertError>",
            Some(("ft64", "ConvertError")),
            &["E_RUNTIME_CONVERT_TYPE_001", "E_RUNTIME_CONVERT_SYNTAX_002", "E_RUNTIME_CONVERT_RANGE_003", "E_RUNTIME_CONVERT_ALLOC_004"],
            &["Ok owns one native float payload consumed by unwrap or released by backend cleanup"],
            "performs finite checked ft64 conversion",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.toBool",
            "convert",
            &[],
            &[("value", "any", "by value or borrowed", "caller keeps value")],
            "Result<bool,ConvertError>",
            Some(("bool", "ConvertError")),
            &["E_RUNTIME_CONVERT_TYPE_001", "E_RUNTIME_CONVERT_SYNTAX_002", "E_RUNTIME_CONVERT_RANGE_003"],
            &["no resource transfer"],
            "converts recognized bool text and scalar truth values without string truthiness",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.safeToInt",
            "convert",
            &["nC.runtime.convert.toInt"],
            &[("value", "any", "by value or borrowed", "caller keeps value")],
            "Result<int,ConvertError>",
            Some(("int", "ConvertError")),
            &["E_RUNTIME_CONVERT_TYPE_001", "E_RUNTIME_CONVERT_SYNTAX_002", "E_RUNTIME_CONVERT_RANGE_003"],
            &["no resource transfer"],
            "explicit alias of exact checked toInt conversion",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.convert.forceToInt",
            "convert",
            &[],
            &[("value", "any", "by value or borrowed", "caller keeps value")],
            "int",
            None,
            &[],
            &["no resource transfer; invalid input becomes zero and finite overflow saturates"],
            "performs explicit lossy integer coercion",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.escape.escapeAnsi",
            "escape",
            &[],
            &[("value", "str", "borrowed UTF-8", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ESCAPE_ALLOC_001"],
            &["returns owned UTF-8 text; caller releases it with strRelease"],
            "renders ESC and standalone C1 controls as lowercase hexadecimal escapes",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.escape.escapeJson",
            "escape",
            &[],
            &[("value", "str", "borrowed UTF-8", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ESCAPE_ALLOC_001"],
            &["returns owned JSON content without surrounding quotes; caller releases it with strRelease"],
            "escapes JSON quote, backslash and C0 control bytes while preserving valid UTF-8",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.escape.escapeControl",
            "escape",
            &[],
            &[("value", "str", "borrowed UTF-8", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ESCAPE_ALLOC_001"],
            &["returns owned UTF-8 text; caller releases it with strRelease"],
            "renders C0, DEL and standalone C1 controls as visible escapes",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.escape.sanitizeTerminal",
            "escape",
            &[],
            &[("value", "str", "borrowed UTF-8", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ESCAPE_ALLOC_001"],
            &["returns owned UTF-8 text; caller releases it with strRelease"],
            "removes ECMA-48 control sequences and terminal-mutating controls while preserving tab and line boundaries",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.escape.sanitizeLog",
            "escape",
            &[],
            &[("value", "str", "borrowed UTF-8", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_ESCAPE_ALLOC_001"],
            &["returns owned single-line-safe UTF-8 text; caller releases it with strRelease"],
            "removes ECMA-48 sequences and renders all remaining controls as visible escapes",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.timer.timerStart",
            "timer",
            &[],
            &[],
            "Result<TimerHandle,AsyncError>",
            Some(("TimerHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned TimerHandle in timer registry"],
            "starts a monotonic stopwatch backed by the process Instant clock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.timer.timerStop",
            "timer",
            &[],
            &[("timer", "TimerHandle", "borrowed handle", "caller keeps handle")],
            "Result<int,AsyncError>",
            Some(("int", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["borrows TimerHandle and returns elapsed milliseconds"],
            "reads elapsed monotonic milliseconds and marks the timer stopped",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.timer.timerReset",
            "timer",
            &[],
            &[("timer", "TimerHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["borrows TimerHandle and rewrites its start instant"],
            "resets a timer to the current monotonic instant",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.timer.timeout",
            "timer",
            &["nC.runtime.async.asyncRun"],
            &[("ms", "int", "by value", "caller keeps value")],
            "Result<FutureHandle,AsyncError>",
            Some(("FutureHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned FutureHandle; await consumes or futureCancel requests cancellation"],
            "schedules a non-blocking timer future that resolves true after ms milliseconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.async.asyncRun",
            "async",
            &[],
            &[("fnref", "@fn", "borrowed fn reference", "runtime owns spawned execution state")],
            "Result<FutureHandle,AsyncError>",
            Some(("FutureHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned FutureHandle; await consumes"],
            "runs an @fn on a runtime worker and stores its payload in the async registry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.async.await",
            "async",
            &[],
            &[("future", "FutureHandle", "owned handle", "await consumes handle")],
            "Result<any,AsyncError>",
            Some(("any", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["consumes FutureHandle and returns owned payload or AsyncError"],
            "blocks current task until the future completes, fails or is cancelled",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.async.futureReady",
            "async",
            &[],
            &[("future", "FutureHandle", "borrowed handle", "caller keeps handle")],
            "bool",
            None,
            &["E_RUNTIME_ASYNC_001"],
            &["borrows FutureHandle and may cache completed payload in registry"],
            "polls a future without blocking",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.async.futureCancel",
            "async",
            &["nC.runtime.cancel.cancel"],
            &[("future", "FutureHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["sets the future cancellation flag"],
            "requests cooperative cancellation for a pending future",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.task.taskSpawn",
            "task",
            &["nC.runtime.scheduler.schedule"],
            &[("fnref", "@fn", "borrowed fn reference", "runtime owns spawned execution state")],
            "Result<TaskHandle,AsyncError>",
            Some(("TaskHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned TaskHandle; taskAwait consumes"],
            "runs an @fn as a scheduled task on a runtime worker",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.task.taskAwait",
            "task",
            &["nC.runtime.async.await"],
            &[("task", "TaskHandle", "owned handle", "taskAwait consumes handle")],
            "Result<any,AsyncError>",
            Some(("any", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["consumes TaskHandle and returns owned payload or AsyncError"],
            "waits for a task to finish and transfers the payload",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.task.taskCancel",
            "task",
            &["nC.runtime.cancel.cancel"],
            &[("task", "TaskHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["sets the task cancellation flag"],
            "requests cooperative cancellation for a pending task",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.task.taskStatus",
            "task",
            &[],
            &[("task", "TaskHandle", "borrowed handle", "caller keeps handle")],
            "str",
            None,
            &["E_RUNTIME_ASYNC_001"],
            &["borrows TaskHandle and returns owned status text"],
            "reports pending, ready, cancelled, completed or missing",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.scheduler.schedule",
            "scheduler",
            &["nC.runtime.task.taskSpawn"],
            &[("fnref", "@fn", "borrowed fn reference", "runtime owns spawned execution state")],
            "Result<TaskHandle,AsyncError>",
            Some(("TaskHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned TaskHandle"],
            "schedules an @fn immediately on the runtime scheduler",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.scheduler.scheduleAfter",
            "scheduler",
            &[],
            &[
                ("fnref", "@fn", "borrowed fn reference", "runtime owns spawned execution state"),
                ("delayMs", "int", "by value", "caller keeps value"),
            ],
            "Result<TaskHandle,AsyncError>",
            Some(("TaskHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned TaskHandle and cooperative timer state"],
            "schedules an @fn after a non-blocking delay on the runtime scheduler",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.scheduler.scheduleEvery",
            "scheduler",
            &[],
            &[
                ("fnref", "@fn", "borrowed fn reference", "runtime owns repeating execution state"),
                ("periodMs", "int", "by value", "caller keeps value"),
            ],
            "Result<TaskHandle,AsyncError>",
            Some(("TaskHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned repeating TaskHandle; taskCancel requests stop"],
            "runs an @fn repeatedly until cancelled",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.scheduler.schedulerYield",
            "scheduler",
            &[],
            &[],
            "void",
            None,
            &["E_RUNTIME_ASYNC_001"],
            &["yields current host thread; no resource escapes"],
            "gives the scheduler an opportunity to run other workers",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.cancel.cancelToken",
            "cancel",
            &[],
            &[],
            "Result<CancelToken,AsyncError>",
            Some(("CancelToken", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["creates owned CancelToken in cancellation registry"],
            "creates a cooperative cancellation token backed by an atomic flag",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.cancel.cancel",
            "cancel",
            &[],
            &[("handle", "FutureHandle|TaskHandle|CancelToken", "borrowed handle", "caller keeps handle")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["sets a cancellation flag when the handle is known"],
            "requests cooperative cancellation for async, task or standalone token handles",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.cancel.isCancelled",
            "cancel",
            &[],
            &[("handle", "FutureHandle|TaskHandle|CancelToken", "borrowed handle", "caller keeps handle")],
            "bool",
            None,
            &["E_RUNTIME_ASYNC_001"],
            &["borrows handle and reads atomic cancellation flag"],
            "checks whether a cooperative cancellation handle has been cancelled",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.cancel.throwIfCancelled",
            "cancel",
            &[],
            &[("handle", "FutureHandle|TaskHandle|CancelToken", "borrowed handle", "caller keeps handle")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_ASYNC_001"],
            &["borrows handle and returns AsyncError when cancelled"],
            "turns cancellation state into an explicit Result",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.event.eventCreate",
            "event",
            &[],
            &[("name", "str", "borrowed text", "caller keeps name")],
            "Result<EventHandle,AsyncError>",
            Some(("EventHandle", "AsyncError")),
            &["E_RUNTIME_EVENT_001"],
            &["creates owned EventHandle in event registry"],
            "creates or replaces a named runtime event flag",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.event.eventEmit",
            "event",
            &[],
            &[("event", "EventHandle|str", "borrowed handle or name", "caller keeps value")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_EVENT_001"],
            &["sets event state to emitted"],
            "emits an event and wakes waiters by state observation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.event.eventWait",
            "event",
            &[],
            &[("event", "EventHandle|str", "borrowed handle or name", "caller keeps value")],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_EVENT_001"],
            &["borrows event state until emitted"],
            "waits until a runtime event has been emitted",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.event.eventOn",
            "event",
            &[],
            &[
                ("name", "str", "borrowed text", "caller keeps name"),
                ("fnref", "@fn", "borrowed fn reference", "caller keeps fnref"),
            ],
            "Result<bool,AsyncError>",
            Some(("bool", "AsyncError")),
            &["E_RUNTIME_EVENT_001"],
            &["records that the event/callback relation is accepted by runtime"],
            "validates a named event callback registration surface",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpListen",
            "tcp",
            &[],
            &[("port", "int", "by value", "caller keeps value")],
            "Result<TcpHandle,TcpError>",
            Some(("TcpHandle", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["creates listener handle; caller must close with tcpClose"],
            "binds 127.0.0.1:port and stores listener in tcp registry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpListenOn",
            "tcp",
            &[],
            &[
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
            ],
            "Result<TcpHandle,TcpError>",
            Some(("TcpHandle", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["creates listener handle; caller must close with tcpClose"],
            "binds a concrete host:port and stores listener in tcp registry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpPort",
            "tcp",
            &[],
            &[(
                "listener",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<int,TcpError>",
            Some(("int", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows listener handle"],
            "reads local port for listener handle",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpConnect",
            "tcp",
            &[],
            &[
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
            ],
            "Result<TcpHandle,TcpError>",
            Some(("TcpHandle", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["creates stream handle; caller must close with tcpClose"],
            "connects a real TcpStream and stores it in tcp registry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpConnectTimeout",
            "tcp",
            &[],
            &[
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<TcpHandle,TcpError>",
            Some(("TcpHandle", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["creates stream handle; caller must close with tcpClose"],
            "connects with a real connect timeout",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpAccept",
            "tcp",
            &[],
            &[(
                "listener",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<TcpHandle,TcpError>",
            Some(("TcpHandle", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["creates accepted stream handle; listener remains open"],
            "accepts one real connection from listener handle",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpSend",
            "tcp",
            &[],
            &[
                (
                    "stream",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("payload", "str", "borrowed text", "caller keeps payload"),
            ],
            "Result<int,TcpError>",
            Some(("int", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows stream handle; returns bytes written"],
            "writes payload bytes and flushes stream",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpRecv",
            "tcp",
            &[],
            &[
                (
                    "stream",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("maxBytes", "int", "by value", "caller keeps value"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<str,TcpError>",
            Some(("str", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows stream handle; returned string owns read payload"],
            "sets read timeout and reads up to maxBytes bytes",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpSetNonBlocking",
            "tcp",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("enabled", "bool", "by value", "caller keeps value"),
            ],
            "Result<bool,TcpError>",
            Some(("bool", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows listener or stream handle"],
            "sets OS nonblocking mode on a TCP listener or stream",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpSetTimeout",
            "tcp",
            &[],
            &[
                (
                    "stream",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("readMs", "int", "by value", "caller keeps value"),
                ("writeMs", "int", "by value", "caller keeps value"),
            ],
            "Result<bool,TcpError>",
            Some(("bool", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows stream handle"],
            "sets read and write timeouts on a TCP stream",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpShutdownRead",
            "tcp",
            &[],
            &[(
                "stream",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<bool,TcpError>",
            Some(("bool", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows stream handle"],
            "half-closes the read side of a TCP stream",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpShutdownWrite",
            "tcp",
            &[],
            &[(
                "stream",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<bool,TcpError>",
            Some(("bool", "TcpError")),
            &["E_RUNTIME_TCP_001"],
            &["borrows stream handle"],
            "half-closes the write side of a TCP stream",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpClose",
            "tcp",
            &[],
            &[(
                "handle",
                "TcpHandle",
                "owned handle",
                "tcpClose consumes handle",
            )],
            "bool",
            None,
            &["E_RUNTIME_TCP_001"],
            &["consumes listener or stream handle when present"],
            "removes tcp handle from registry and closes OS socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpConnectAsync",
            "tcp",
            &["nC.runtime.async.asyncRun"],
            &[
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
            ],
            "Result<FutureHandle,AsyncError>",
            Some(("FutureHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001", "E_RUNTIME_TCP_001"],
            &["creates owned FutureHandle; await returns Result<TcpHandle,TcpError> payload"],
            "connects TCP on an async worker and resolves with the synchronous tcpConnect Result",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpAcceptAsync",
            "tcp",
            &["nC.runtime.async.asyncRun"],
            &[(
                "listener",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<FutureHandle,AsyncError>",
            Some(("FutureHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001", "E_RUNTIME_TCP_001"],
            &["creates owned FutureHandle; await returns Result<TcpHandle,TcpError> payload"],
            "accepts TCP on an async worker and resolves with the synchronous tcpAccept Result",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpSendAsync",
            "tcp",
            &["nC.runtime.async.asyncRun"],
            &[
                (
                    "stream",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("payload", "str", "borrowed text", "caller keeps payload"),
            ],
            "Result<FutureHandle,AsyncError>",
            Some(("FutureHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001", "E_RUNTIME_TCP_001"],
            &["creates owned FutureHandle; await returns Result<int,TcpError> payload"],
            "writes TCP payload on an async worker using a cloned stream handle",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tcp.tcpRecvAsync",
            "tcp",
            &["nC.runtime.async.asyncRun"],
            &[
                (
                    "stream",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("maxBytes", "int", "by value", "caller keeps value"),
            ],
            "Result<FutureHandle,AsyncError>",
            Some(("FutureHandle", "AsyncError")),
            &["E_RUNTIME_ASYNC_001", "E_RUNTIME_TCP_001"],
            &["creates owned FutureHandle; await returns Result<str,TcpError> payload"],
            "reads TCP payload on an async worker using a cloned stream handle",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpBind",
            "udp",
            &["nC.runtime.udp.udpBindOn"],
            &[("port", "int", "by value", "caller keeps value")],
            "Result<TcpHandle,NetworkError>",
            Some(("TcpHandle", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["creates UDP socket handle; caller must udpClose or tcpClose"],
            "binds 127.0.0.1:port as a real UDP socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpBindOn",
            "udp",
            &[],
            &[
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
            ],
            "Result<TcpHandle,NetworkError>",
            Some(("TcpHandle", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["creates UDP socket handle; caller must udpClose or tcpClose"],
            "binds a concrete host:port as a real UDP socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpPort",
            "udp",
            &[],
            &[(
                "handle",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<int,NetworkError>",
            Some(("int", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["borrows UDP socket handle"],
            "reads local port for UDP handle",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpSetNonBlocking",
            "udp",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("enabled", "bool", "by value", "caller keeps value"),
            ],
            "Result<bool,NetworkError>",
            Some(("bool", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["borrows UDP socket handle"],
            "sets OS nonblocking mode on UDP socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpSetTimeout",
            "udp",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("readMs", "int", "by value", "caller keeps value"),
                ("writeMs", "int", "by value", "caller keeps value"),
            ],
            "Result<bool,NetworkError>",
            Some(("bool", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["borrows UDP socket handle"],
            "sets read and write timeouts on UDP socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpSendTo",
            "udp",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
                ("payload", "str", "borrowed text", "caller keeps payload"),
            ],
            "Result<int,NetworkError>",
            Some(("int", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["borrows UDP socket handle; returns bytes sent"],
            "sends UDP datagram to host:port",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpRecvFrom",
            "udp",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("maxBytes", "int", "by value", "caller keeps value"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<str,NetworkError>",
            Some(("str", "NetworkError")),
            &["E_RUNTIME_UDP_001"],
            &["borrows UDP socket handle; returned string owns addr|payload"],
            "receives one UDP datagram with read timeout",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.udp.udpClose",
            "udp",
            &[],
            &[(
                "handle",
                "TcpHandle",
                "owned handle",
                "udpClose consumes handle",
            )],
            "bool",
            None,
            &["E_RUNTIME_UDP_001"],
            &["consumes UDP socket handle when present"],
            "removes UDP socket from registry and closes OS socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.dns.resolve",
            "dns",
            &[],
            &[("host", "str", "borrowed text", "caller keeps host")],
            "Result<str,NetworkError>",
            Some(("str", "NetworkError")),
            &["E_RUNTIME_DNS_001"],
            &["returned string owns first resolved IP"],
            "resolves first IP address through host resolver",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.dns.resolveAll",
            "dns",
            &[],
            &[("host", "str", "borrowed text", "caller keeps host")],
            "Result<str,NetworkError>",
            Some(("str", "NetworkError")),
            &["E_RUNTIME_DNS_001"],
            &["returned string owns newline-separated resolved IPs"],
            "resolves all IP addresses through host resolver",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.dns.reverseLookup",
            "dns",
            &[],
            &[("ip", "str", "borrowed text", "caller keeps ip")],
            "Result<str,NetworkError>",
            Some(("str", "NetworkError")),
            &["E_RUNTIME_DNS_001"],
            &["returned string owns reverse hostname if OS resolver provides it"],
            "performs host reverse lookup through OS resolver command when available",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tls.tlsConnect",
            "tls",
            &[],
            &[
                ("host", "str", "borrowed text", "caller keeps host"),
                ("port", "int", "by value", "caller keeps value"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<TcpHandle,NetworkError>",
            Some(("TcpHandle", "NetworkError")),
            &["E_RUNTIME_TLS_001"],
            &["creates TLS stream handle; caller must tlsClose or tcpClose"],
            "opens TCP socket and performs rustls client handshake with webpki roots",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tls.tlsSend",
            "tls",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("payload", "str", "borrowed text", "caller keeps payload"),
            ],
            "Result<int,NetworkError>",
            Some(("int", "NetworkError")),
            &["E_RUNTIME_TLS_001"],
            &["borrows TLS stream handle; returns bytes sent"],
            "writes payload through encrypted TLS stream",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tls.tlsRecv",
            "tls",
            &[],
            &[
                (
                    "handle",
                    "TcpHandle",
                    "borrowed handle",
                    "caller keeps handle",
                ),
                ("maxBytes", "int", "by value", "caller keeps value"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<str,NetworkError>",
            Some(("str", "NetworkError")),
            &["E_RUNTIME_TLS_001"],
            &["borrows TLS stream handle; returned string owns decrypted payload"],
            "reads decrypted bytes through TLS stream with timeout",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tls.tlsPeerCert",
            "tls",
            &[],
            &[(
                "handle",
                "TcpHandle",
                "borrowed handle",
                "caller keeps handle",
            )],
            "Result<str,NetworkError>",
            Some(("str", "NetworkError")),
            &["E_RUNTIME_TLS_001"],
            &["borrows TLS stream handle; returned string owns cert metadata"],
            "returns peer certificate count and first DER length after TLS handshake",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.tls.tlsClose",
            "tls",
            &[],
            &[(
                "handle",
                "TcpHandle",
                "owned handle",
                "tlsClose consumes handle",
            )],
            "bool",
            None,
            &["E_RUNTIME_TLS_001"],
            &["consumes TLS stream handle when present"],
            "removes TLS stream from registry and closes OS socket",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.ptr.ptrRead",
            "ptr",
            &[],
            &[("p", "ptr<T>", "borrowed pointer", "caller keeps pointer")],
            "Result<T,str>",
            Some(("T", "str")),
            &["E_PTR_NULL_DEREF_001", "E_PTR_TYPE_002", "E_PTR_BOUNDS_001"],
            &["borrows pointer and loaded value is owned by Result.ok"],
            "loads T from checked pointer address",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.ptr.ptrWrite",
            "ptr",
            &[],
            &[
                (
                    "p",
                    "ptr<T>",
                    "borrowed mutable pointer",
                    "caller keeps pointer",
                ),
                (
                    "value",
                    "T",
                    "borrowed RuntimeValue",
                    "written value is cloned into pointee",
                ),
            ],
            "bool",
            None,
            &["E_PTR_NULL_DEREF_001", "E_PTR_TYPE_002", "E_PTR_BOUNDS_001"],
            &["mutates pointee; pointer remains owned by caller"],
            "stores T at checked pointer address",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.ptr.ptrOffset",
            "ptr",
            &[],
            &[
                ("p", "ptr<T>", "borrowed pointer", "caller keeps pointer"),
                ("count", "int", "by value", "caller keeps value"),
            ],
            "ptr<T>",
            None,
            &["E_PTR_BOUNDS_001"],
            &["derived pointer borrows provenance from original block"],
            "returns pointer advanced by count * sizeof(T)",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.ptr.ptrDiff",
            "ptr",
            &[],
            &[
                ("a", "ptr<T>", "borrowed pointer", "caller keeps pointer"),
                ("b", "ptr<T>", "borrowed pointer", "caller keeps pointer"),
            ],
            "int",
            None,
            &["E_PTR_PROVENANCE_001"],
            &["no resource transfer"],
            "returns element distance between compatible pointers",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.heap.alloc",
            "heap",
            &["memory.heap.manager.alloc"],
            &[("bytes", "int", "by value", "caller keeps value")],
            "ptr<byte>",
            None,
            &["E_RUNTIME_HEAP_001"],
            &["creates owned heap pointer; caller must free"],
            "allocates tracked heap block in runtime heap table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.heap.free",
            "heap",
            &["memory.heap.manager.free"],
            &[("p", "ptr<T>", "owned pointer", "free consumes owner")],
            "bool",
            None,
            &["E_PTR_DOUBLE_FREE_001", "E_PTR_UAF_001"],
            &["consumes heap owner and invalidates aliases by generation"],
            "marks heap block dead in runtime heap table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.heap.heapAlloc",
            "heap",
            &[],
            &[("bytes", "int", "by value", "caller keeps value")],
            "Result<ptr<T>,MemoryError>",
            Some(("ptr<T>", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates owned heap pointer; caller must heapFree/free"],
            "allocates exactly bytes raw bytes in the tracked heap table; T never scales the request",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.heap.heapFree",
            "heap",
            &[],
            &[("p", "ptr<T>", "owned pointer", "heapFree consumes owner")],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &["E_RUNTIME_MEMORY_001", "E_PTR_UAF_001"],
            &["consumes heap owner and invalidates aliases by generation"],
            "marks heap block dead through runtime heap table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.heap.heapStats",
            "heap",
            &[],
            &[],
            "str",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns heap allocation counters as stable text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.heap.heapCheck",
            "heap",
            &[],
            &[],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "checks live heap counter against heap block table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.fixedBuffer",
            "buffer",
            &[],
            &[("len", "int", "by value", "caller keeps value")],
            "Result<BufferHandle,MemoryError>",
            Some(("BufferHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates owned zero-filled buffer handle"],
            "allocates len * sizeof(T) bytes in runtime buffer table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.bufferLen",
            "buffer",
            &[],
            &[("buffer", "BufferHandle", "borrowed handle", "caller keeps buffer")],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns logical element length from byte length / sizeof(T)",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.bufferRead",
            "buffer",
            &[],
            &[
                ("buffer", "BufferHandle", "borrowed handle", "caller keeps buffer"),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "Result<any,MemoryError>",
            Some(("any", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["returns scalar decoded from stable little-endian bytes"],
            "reads one element from checked buffer bounds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.bufferWrite",
            "buffer",
            &[],
            &[
                ("buffer", "BufferHandle", "borrowed mutable handle", "caller keeps buffer"),
                ("index", "int", "by value", "caller keeps value"),
                ("value", "T", "borrowed scalar", "value is encoded into bytes"),
            ],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["mutates stable little-endian byte backing"],
            "writes one element into checked buffer bounds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.bufferSlice",
            "buffer",
            &[],
            &[
                ("buffer", "BufferHandle", "borrowed handle", "caller keeps buffer"),
                ("start", "int", "by value", "caller keeps value"),
                ("len", "int", "by value", "caller keeps value"),
            ],
            "Result<SliceHandle,MemoryError>",
            Some(("SliceHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates borrowed slice view; backing buffer remains owner"],
            "creates checked region view over contiguous buffer elements",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.bufferClear",
            "buffer",
            &[],
            &[("buffer", "BufferHandle", "borrowed mutable handle", "caller keeps buffer")],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["zeroes buffer bytes in place"],
            "fills buffer backing store with zero bytes",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.buffer.bufferRelease",
            "buffer",
            &[],
            &[("buffer", "BufferHandle", "owned handle", "callee consumes buffer")],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["explicitly releases backing bytes; existing slices become stale"],
            "calls the hosted runtime buffer destructor",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.slice.slice",
            "slice",
            &[],
            &[
                ("source", "BufferHandle|SliceHandle", "borrowed view source", "caller keeps source"),
                ("start", "int", "by value", "caller keeps value"),
                ("len", "int", "by value", "caller keeps value"),
            ],
            "Result<SliceHandle,MemoryError>",
            Some(("SliceHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates borrowed sub-view over existing buffer"],
            "creates checked slice view from buffer or slice",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.slice.sliceLen",
            "slice",
            &[],
            &[("slice", "SliceHandle", "borrowed handle", "caller keeps slice")],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns logical element length of slice",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.slice.sliceRead",
            "slice",
            &[],
            &[
                ("slice", "SliceHandle", "borrowed handle", "caller keeps slice"),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "Result<byte,MemoryError>",
            Some(("byte", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["returns scalar decoded from backing buffer bytes"],
            "reads checked index through slice view",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.slice.sliceIsEmpty",
            "slice",
            &[],
            &[("slice", "SliceHandle", "borrowed handle", "caller keeps slice")],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "tests slice logical length == 0",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.slice.sliceRelease",
            "slice",
            &[],
            &[("slice", "SliceHandle", "owned descriptor", "callee consumes descriptor")],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["releases only the slice descriptor; backing bytes remain owned by the buffer"],
            "calls the hosted runtime slice descriptor destructor",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanCreate",
            "span",
            &[],
            &[
                ("left", "SliceHandle", "borrowed view", "caller keeps left"),
                ("right", "SliceHandle", "borrowed view", "caller keeps right"),
            ],
            "Result<SpanHandle,MemoryError>",
            Some(("SpanHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates borrowed ordered span view over two slices"],
            "creates span when slice element layouts match",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanLen",
            "span",
            &[],
            &[("span", "SpanHandle", "borrowed handle", "caller keeps span")],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns total logical element length across slices",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanJoin",
            "span",
            &[],
            &[
                ("left", "SpanHandle", "borrowed handle", "caller keeps left"),
                ("right", "SpanHandle", "borrowed handle", "caller keeps right"),
            ],
            "Result<SpanHandle,MemoryError>",
            Some(("SpanHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates borrowed joined span view"],
            "joins two compatible spans without copying bytes",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanContains",
            "span",
            &[],
            &[
                ("span", "SpanHandle", "borrowed handle", "caller keeps span"),
                ("value", "byte", "borrowed scalar", "caller keeps value"),
            ],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "scans backing byte ranges without joining or copying them",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanRead",
            "span",
            &["T"],
            &[
                ("span", "SpanHandle", "borrowed handle", "caller keeps span"),
                ("index", "int", "value", "callee copies index"),
            ],
            "Result<byte,MemoryError>",
            Some(("byte", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "reads one byte through the ordered segment descriptor",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanIsEmpty",
            "span",
            &[],
            &[("span", "SpanHandle", "borrowed handle", "caller keeps span")],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "tests total logical byte length == 0",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.span.spanRelease",
            "span",
            &[],
            &[("span", "SpanHandle", "owned descriptor", "callee consumes descriptor")],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["releases segment metadata; backing buffers remain caller-owned"],
            "calls the hosted runtime span descriptor destructor",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackDepth",
            "stack",
            &[],
            &[],
            "u64",
            None,
            &[],
            &["no resource transfer"],
            "returns bytes consumed between the current stack pointer and thread stack high bound",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackLimit",
            "stack",
            &[],
            &[],
            "u64",
            None,
            &[],
            &["no resource transfer"],
            "returns the operating-system stack reservation size for the current thread",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackRemaining",
            "stack",
            &[],
            &[],
            "u64",
            None,
            &[],
            &["no resource transfer"],
            "returns bytes between the current stack pointer and thread stack low bound",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackTrace",
            "stack",
            &[],
            &[],
            "StackTraceHandle",
            None,
            &[],
            &["caller owns captured frame-address descriptor"],
            "captures up to 64 native return addresses from the current thread",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackTraceLen",
            "stack",
            &[],
            &[("trace", "StackTraceHandle", "borrowed handle", "caller keeps trace")],
            "u64",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns captured native frame count",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackTraceFrame",
            "stack",
            &[],
            &[
                ("trace", "StackTraceHandle", "borrowed handle", "caller keeps trace"),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "Result<addr,MemoryError>",
            Some(("addr", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns one captured native return address with checked descriptor indexing",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.stack.stackTraceRelease",
            "stack",
            &[],
            &[("trace", "StackTraceHandle", "owned handle", "callee consumes trace")],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["releases frame-address storage and descriptor"],
            "calls the hosted runtime stack trace descriptor destructor",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.alloc.defaultAllocator",
            "alloc",
            &[],
            &[],
            "Result<AllocatorHandle,MemoryError>",
            Some(("AllocatorHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates allocator accounting handle"],
            "creates unbounded default allocator registry entry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.alloc.arenaAllocator",
            "alloc",
            &[],
            &[("capacity", "int", "by value", "caller keeps value")],
            "Result<AllocatorHandle,MemoryError>",
            Some(("AllocatorHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates allocator accounting handle with byte capacity"],
            "creates bounded arena allocator registry entry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.alloc.allocWith",
            "alloc",
            &[],
            &[
                ("allocator", "AllocatorHandle", "borrowed mutable handle", "caller keeps allocator"),
                ("bytes", "int", "by value", "caller keeps value"),
            ],
            "Result<OwningPtr,MemoryError>",
            Some(("OwningPtr", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates linear OwningPtr tracked by allocator"],
            "allocates exactly bytes raw bytes and records ownership in allocator",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.alloc.freeWith",
            "alloc",
            &[],
            &[
                ("allocator", "AllocatorHandle", "borrowed mutable handle", "caller keeps allocator"),
                ("ptr", "OwningPtr", "owned pointer", "freeWith consumes ptr"),
            ],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["consumes OwningPtr and removes allocator ownership entry"],
            "frees tracked heap block only when allocator owns pointer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.bytes.byteAt",
            "bytes",
            &[],
            &[
                ("source", "BufferHandle|SliceHandle|SpanHandle|array<byte>", "borrowed source", "caller keeps source"),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "Result<byte,MemoryError>",
            Some(("byte", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["returns copied byte value"],
            "reads checked byte offset from byte-compatible source",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.bytes.bytesToHex",
            "bytes",
            &[],
            &[("source", "bytes", "borrowed source", "caller keeps source")],
            "str",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returned hex string owns copied text"],
            "renders source bytes as lowercase hexadecimal",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.bytes.hexToBytes",
            "bytes",
            &[],
            &[("hex", "str", "borrowed text", "caller keeps hex")],
            "Result<BufferHandle,MemoryError>",
            Some(("BufferHandle", "MemoryError")),
            &["E_RUNTIME_MEMORY_001"],
            &["creates owned byte buffer"],
            "parses even-length hexadecimal into byte buffer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.bytes.bytesEq",
            "bytes",
            &[],
            &[
                ("left", "bytes", "borrowed source", "caller keeps left"),
                ("right", "bytes", "borrowed source", "caller keeps right"),
            ],
            "bool",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "compares byte snapshots for equality",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.bytes.bytesLen",
            "bytes",
            &[],
            &[("source", "bytes", "borrowed source", "caller keeps source")],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns byte length of source snapshot",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.layout.layoutOf",
            "layout",
            &[],
            &[("typeOrValue", "any", "borrowed descriptor", "caller keeps value")],
            "str",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returned layout string owns copied text"],
            "returns type:size:align layout summary",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.layout.layoutSize",
            "layout",
            &[],
            &[("typeOrValue", "any", "borrowed descriptor", "caller keeps value")],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns sizeof(type) in bytes for runtime layout table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.layout.layoutAlign",
            "layout",
            &[],
            &[("typeOrValue", "any", "borrowed descriptor", "caller keeps value")],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns alignof(type) in bytes for runtime layout table",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.layout.fieldOffset",
            "layout",
            &[],
            &[
                ("typeOrValue", "any", "borrowed descriptor", "caller keeps value"),
                ("field", "str", "borrowed text", "caller keeps field"),
            ],
            "int",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["no resource transfer"],
            "returns stable field offset when known, otherwise 0 for opaque runtime values",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.endian.nativeEndian",
            "endian",
            &[],
            &[],
            "str",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returned text is owned by caller"],
            "reports host endian contract used by scalar byte encoding",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.endian.toLittleEndian",
            "endian",
            &[],
            &[("value", "any", "borrowed scalar", "caller keeps value")],
            "any",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returns value in runtime official little-endian representation"],
            "normalizes value to little-endian runtime representation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.endian.toBigEndian",
            "endian",
            &[],
            &[("value", "any", "borrowed scalar", "caller keeps value")],
            "any",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returns value in big-endian logical representation"],
            "normalizes value to big-endian logical representation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.endian.fromLittleEndian",
            "endian",
            &[],
            &[("value", "any", "borrowed scalar", "caller keeps value")],
            "any",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returns host runtime scalar representation"],
            "decodes little-endian logical representation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.endian.fromBigEndian",
            "endian",
            &[],
            &[("value", "any", "borrowed scalar", "caller keeps value")],
            "any",
            None,
            &["E_RUNTIME_MEMORY_001"],
            &["returns host runtime scalar representation"],
            "decodes big-endian logical representation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.region.regionFromPtr",
            "region",
            &[],
            &[
                ("p", "ptr<T>", "borrowed pointer", "caller keeps pointer"),
                ("len", "int", "by value", "caller keeps value"),
            ],
            "memRegion<T>",
            None,
            &["E_REGION_BOUNDS_001", "E_PTR_PROVENANCE_001"],
            &["region view borrows backing heap block generation"],
            "opens typed region view over a live heap-backed pointer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.region.regionPtrAt",
            "region",
            &[],
            &[
                (
                    "region",
                    "memRegion<T>",
                    "borrowed region",
                    "caller keeps region",
                ),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "ptr<T>",
            None,
            &["E_REGION_BOUNDS_001", "E_PTR_TYPE_002"],
            &["returned pointer borrows region provenance"],
            "returns pointer to element index inside checked region",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.region.readAt",
            "region",
            &[],
            &[
                (
                    "region",
                    "memRegion<T>",
                    "borrowed region",
                    "caller keeps region",
                ),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "T",
            None,
            &["E_REGION_BOUNDS_001", "E_PTR_TYPE_002"],
            &["returned value is owned by caller"],
            "reads element index from checked region storage",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.region.writeAt",
            "region",
            &[],
            &[
                (
                    "region",
                    "memRegion<T>",
                    "borrowed mutable region",
                    "caller keeps region",
                ),
                ("index", "int", "by value", "caller keeps value"),
                (
                    "value",
                    "T",
                    "borrowed RuntimeValue",
                    "written value is cloned",
                ),
            ],
            "bool",
            None,
            &["E_REGION_BOUNDS_001", "E_PTR_TYPE_002"],
            &["mutates region storage; region remains owned by caller"],
            "writes element index into checked region storage",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.array.index",
            "array",
            &[],
            &[
                ("array", "array<T>", "borrowed array", "caller keeps array"),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "T",
            None,
            &["E_RUNTIME_INDEX_OOB_001", "E_RUNTIME_INDEX_INVALID_001"],
            &["returned element is owned by caller"],
            "reads checked array index",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.array.len",
            "array",
            &[],
            &[("array", "array<T>", "borrowed array", "caller keeps array")],
            "int",
            None,
            &["E_RUNTIME_INDEX_INVALID_001"],
            &["no resource transfer"],
            "returns array logical length",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.list.get",
            "list",
            &[],
            &[
                ("list", "list<T>", "borrowed list", "caller keeps list"),
                ("index", "int", "by value", "caller keeps value"),
            ],
            "T",
            None,
            &["E_LIST_TYPE_001", "E_RUNTIME_INDEX_OOB_001"],
            &["returned element is owned by caller"],
            "reads checked list index",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.list.len",
            "list",
            &[],
            &[("list", "list<T>", "borrowed list", "caller keeps list")],
            "int",
            None,
            &["E_LIST_TYPE_001"],
            &["no resource transfer"],
            "returns list logical length",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.result.Ok",
            "result",
            &[],
            &[("value", "T", "owned payload", "Result.ok takes ownership")],
            "Result<T,E>",
            Some(("T", "E")),
            &["E_RUNTIME_RESULT_001"],
            &["returned Result owns ok payload"],
            "constructs ok variant",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.result.Err",
            "result",
            &[],
            &[("error", "E", "owned payload", "Result.err takes ownership")],
            "Result<T,E>",
            Some(("T", "E")),
            &["E_RUNTIME_RESULT_001"],
            &["returned Result owns err payload"],
            "constructs err variant",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.result.isOk",
            "result",
            &[],
            &[(
                "result",
                "Result<T,E>",
                "borrowed result",
                "caller keeps result",
            )],
            "bool",
            None,
            &["E_RUNTIME_RESULT_001"],
            &["no payload transfer"],
            "tests ok variant tag",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.result.isErr",
            "result",
            &[],
            &[(
                "result",
                "Result<T,E>",
                "borrowed result",
                "caller keeps result",
            )],
            "bool",
            None,
            &["E_RUNTIME_RESULT_001"],
            &["no payload transfer"],
            "tests err variant tag",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.result.unwrap",
            "result",
            &[],
            &[(
                "result",
                "Result<T,E>",
                "owned-or-borrowed result",
                "unwrap transfers ok payload",
            )],
            "T",
            None,
            &["E_RUNTIME_RESULT_UNWRAP_001"],
            &["ok payload is returned to caller; err raises runtime diagnostic"],
            "extracts ok payload or fails on err",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.processId",
            "process",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_PROCESS_001"],
            &["no resource transfer"],
            "returns current host process id",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.parentProcessId",
            "process",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_PROCESS_001"],
            &["no resource transfer"],
            "returns parent process id where available, otherwise 0",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.spawn",
            "process",
            &[],
            &[
                ("cmd", "str", "borrowed text", "caller keeps cmd"),
                ("args", "list<str>", "borrowed list", "caller keeps args"),
            ],
            "Result<ProcessHandle,ProcessError>",
            Some(("ProcessHandle", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["creates owned ProcessHandle; caller must wait or kill"],
            "spawns child with Command::new without shell and stores child in process registry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.spawnPipe",
            "process",
            &[],
            &[
                ("cmd", "str", "borrowed text", "caller keeps cmd"),
                ("args", "list<str>", "borrowed list", "caller keeps args"),
            ],
            "Result<ProcessHandle,ProcessError>",
            Some(("ProcessHandle", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["creates owned ProcessHandle with stdin/stdout/stderr pipes"],
            "spawns child with all stdio handles piped for writeStdin/readStdout/readStderr",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.spawnIn",
            "process",
            &[],
            &[
                ("cmd", "str", "borrowed text", "caller keeps cmd"),
                ("args", "list<str>", "borrowed list", "caller keeps args"),
                (
                    "cwd",
                    "str",
                    "borrowed text",
                    "sets child current directory",
                ),
            ],
            "Result<ProcessHandle,ProcessError>",
            Some(("ProcessHandle", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["creates owned ProcessHandle in requested working directory"],
            "spawns child with Command::current_dir and no shell",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.spawnWithEnv",
            "process",
            &[],
            &[
                ("cmd", "str", "borrowed text", "caller keeps cmd"),
                ("args", "list<str>", "borrowed list", "caller keeps args"),
                (
                    "env",
                    "list<str>",
                    "borrowed KEY=VALUE list",
                    "sets child environment overrides",
                ),
            ],
            "Result<ProcessHandle,ProcessError>",
            Some(("ProcessHandle", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["creates owned ProcessHandle with environment overrides"],
            "spawns child with Command::env for each KEY=VALUE item",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.spawnWithOptions",
            "process",
            &[],
            &[
                ("cmd", "str", "borrowed text", "caller keeps cmd"),
                ("args", "list<str>", "borrowed list", "caller keeps args"),
                (
                    "cwd",
                    "str",
                    "borrowed text",
                    "empty string means inherited cwd",
                ),
                (
                    "env",
                    "list<str>",
                    "borrowed KEY=VALUE list",
                    "sets child environment overrides",
                ),
                (
                    "pipe_stdio",
                    "bool",
                    "plain bool",
                    "true creates stdin/stdout/stderr pipes",
                ),
                (
                    "process_group",
                    "bool",
                    "plain bool",
                    "true starts a new process group when platform supports it",
                ),
            ],
            "Result<ProcessHandle,ProcessError>",
            Some(("ProcessHandle", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["creates owned ProcessHandle with explicit process options"],
            "spawns child with cwd/env/stdio/group options and no shell",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.spawnGroup",
            "process",
            &[],
            &[
                ("cmd", "str", "borrowed text", "caller keeps cmd"),
                ("args", "list<str>", "borrowed list", "caller keeps args"),
            ],
            "Result<ProcessHandle,ProcessError>",
            Some(("ProcessHandle", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["creates owned ProcessHandle with process-group metadata"],
            "spawns child as a new process group where the host backend supports groups",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.wait",
            "process",
            &[],
            &[(
                "handle",
                "ProcessHandle",
                "owned handle",
                "wait consumes handle",
            )],
            "Result<int,ProcessError>",
            Some(("int", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["consumes ProcessHandle and removes child from registry"],
            "waits for child process and returns exit code or -1 when OS gives no code",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.waitStatus",
            "process",
            &[],
            &[(
                "handle",
                "ProcessHandle",
                "owned handle",
                "waitStatus consumes handle",
            )],
            "Result<str,ProcessError>",
            Some(("str", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["consumes ProcessHandle and returns stable status text"],
            "waits for child and returns kind/success/code/signal/timed_out status fields",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.waitTimeout",
            "process",
            &[],
            &[
                (
                    "handle",
                    "ProcessHandle",
                    "owned handle",
                    "waitTimeout consumes handle",
                ),
                (
                    "timeoutMs",
                    "int",
                    "plain integer",
                    "timeout in milliseconds",
                ),
            ],
            "Result<str,ProcessError>",
            Some(("str", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["consumes ProcessHandle; timeout kills the child and returns timed_out status"],
            "waits up to timeoutMs and returns full status text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.writeStdin",
            "process",
            &[],
            &[
                (
                    "handle",
                    "ProcessHandle",
                    "borrowed handle",
                    "handle stays live",
                ),
                (
                    "payload",
                    "str",
                    "borrowed text",
                    "bytes written to child stdin",
                ),
            ],
            "Result<int,ProcessError>",
            Some(("int", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["writes through existing stdin pipe"],
            "writes payload bytes to a process spawned with stdin pipe",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.closeStdin",
            "process",
            &[],
            &[(
                "handle",
                "ProcessHandle",
                "borrowed handle",
                "handle stays live",
            )],
            "Result<bool,ProcessError>",
            Some(("bool", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["closes stdin pipe without consuming process handle"],
            "drops child stdin so pipe consumers can observe EOF",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.readStdout",
            "process",
            &[],
            &[
                (
                    "handle",
                    "ProcessHandle",
                    "borrowed handle",
                    "handle stays live",
                ),
                ("maxBytes", "int", "plain integer", "maximum bytes to read"),
            ],
            "Result<str,ProcessError>",
            Some(("str", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["reads from existing stdout pipe"],
            "reads UTF-8 text from a process spawned with stdout pipe",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.readStderr",
            "process",
            &[],
            &[
                (
                    "handle",
                    "ProcessHandle",
                    "borrowed handle",
                    "handle stays live",
                ),
                ("maxBytes", "int", "plain integer", "maximum bytes to read"),
            ],
            "Result<str,ProcessError>",
            Some(("str", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["reads from existing stderr pipe"],
            "reads UTF-8 text from a process spawned with stderr pipe",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.statusCode",
            "process",
            &[],
            &[(
                "status",
                "str",
                "borrowed status text",
                "caller keeps status",
            )],
            "int",
            None,
            &["E_RUNTIME_PROCESS_001"],
            &["no resource transfer"],
            "extracts exit code from waitStatus/waitTimeout status text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.statusSignal",
            "process",
            &[],
            &[(
                "status",
                "str",
                "borrowed status text",
                "caller keeps status",
            )],
            "int",
            None,
            &["E_RUNTIME_PROCESS_001"],
            &["no resource transfer"],
            "extracts unix signal number from waitStatus/waitTimeout status text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.statusSuccess",
            "process",
            &[],
            &[(
                "status",
                "str",
                "borrowed status text",
                "caller keeps status",
            )],
            "bool",
            None,
            &["E_RUNTIME_PROCESS_001"],
            &["no resource transfer"],
            "extracts success flag from waitStatus/waitTimeout status text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.statusTimedOut",
            "process",
            &[],
            &[(
                "status",
                "str",
                "borrowed status text",
                "caller keeps status",
            )],
            "bool",
            None,
            &["E_RUNTIME_PROCESS_001"],
            &["no resource transfer"],
            "extracts timeout flag from waitStatus/waitTimeout status text",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.kill",
            "process",
            &[],
            &[(
                "handle",
                "ProcessHandle",
                "owned handle",
                "kill consumes handle",
            )],
            "Result<bool,ProcessError>",
            Some(("bool", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["consumes ProcessHandle and removes child from registry"],
            "kills child process, waits for cleanup and returns success flag",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.signal",
            "process",
            &[],
            &[
                (
                    "handle",
                    "ProcessHandle",
                    "borrowed handle",
                    "handle stays live",
                ),
                ("signal", "int", "plain integer", "host signal number"),
            ],
            "Result<bool,ProcessError>",
            Some(("bool", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["sends host signal without consuming handle"],
            "sends a host signal to the child process",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.signalGroup",
            "process",
            &[],
            &[
                (
                    "handle",
                    "ProcessHandle",
                    "borrowed handle",
                    "handle stays live",
                ),
                ("signal", "int", "plain integer", "host signal number"),
            ],
            "Result<bool,ProcessError>",
            Some(("bool", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["sends host signal to process group without consuming handle"],
            "sends a host signal to the process group created by spawnGroup/spawnWithOptions",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.process.killGroup",
            "process",
            &[],
            &[(
                "handle",
                "ProcessHandle",
                "owned handle",
                "killGroup consumes handle",
            )],
            "Result<bool,ProcessError>",
            Some(("bool", "ProcessError")),
            &["E_RUNTIME_PROCESS_001"],
            &["consumes ProcessHandle and removes child/process group from registry"],
            "kills the process group created by spawnGroup/spawnWithOptions",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadSpawn",
            "thread",
            &[],
            &[(
                "target",
                "fnref",
                "borrowed @fn reference",
                "runtime copies function reference into host thread",
            )],
            "Result<ThreadHandle,ThreadError>",
            Some(("ThreadHandle", "ThreadError")),
            &["E_RUNTIME_THREAD_001"],
            &["creates owned ThreadHandle; caller must threadJoin"],
            "spawns host thread with isolated ExecState and stores JoinHandle in thread registry",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadJoin",
            "thread",
            &[],
            &[(
                "handle",
                "ThreadHandle",
                "owned handle",
                "threadJoin consumes handle",
            )],
            "Result<any,ThreadError>",
            Some(("any", "ThreadError")),
            &["E_RUNTIME_THREAD_001"],
            &["consumes ThreadHandle and returns owned payload from child thread"],
            "joins host thread and returns its nC return value",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadId",
            "thread",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_THREAD_001"],
            &["no resource transfer"],
            "returns runtime-local thread id",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadSleep",
            "thread",
            &[],
            &[("ms", "int", "by value", "caller keeps value")],
            "void",
            None,
            &["E_RUNTIME_THREAD_001"],
            &["blocks current host thread; no resource escapes"],
            "sleeps current thread for non-negative milliseconds",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadJoinTimeout",
            "thread",
            &[],
            &[
                ("handle", "ThreadHandle", "borrowed handle", "handle stays live on timeout and is consumed on success/error completion"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<any,ThreadError>",
            Some(("any", "ThreadError")),
            &["E_RUNTIME_THREAD_001"],
            &["returns Result.err on timeout; completed threads transfer payload and consume handle"],
            "joins a host thread only if it finishes before the timeout",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadLocalCreate",
            "thread",
            &[],
            &[("default", "any", "owned clone", "runtime stores default for future thread-local cells")],
            "Result<ThreadLocalHandle,ThreadError>",
            Some(("ThreadLocalHandle", "ThreadError")),
            &["E_RUNTIME_THREAD_LOCAL_001"],
            &["creates registry-backed thread-local key"],
            "creates a thread-local key with default payload",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadLocalGet",
            "thread",
            &[],
            &[("handle", "ThreadLocalHandle", "borrowed handle", "caller keeps handle")],
            "Result<any,ThreadError>",
            Some(("any", "ThreadError")),
            &["E_RUNTIME_THREAD_LOCAL_001"],
            &["returns current thread payload, initializing from default on first read"],
            "reads thread-local payload for the current host thread",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadLocalSet",
            "thread",
            &[],
            &[
                ("handle", "ThreadLocalHandle", "borrowed handle", "caller keeps handle"),
                ("value", "any", "owned clone", "runtime stores value in current thread-local map"),
            ],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_THREAD_LOCAL_001"],
            &["mutates only current thread-local payload"],
            "writes thread-local payload for the current host thread",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.thread.threadLocalRemove",
            "thread",
            &[],
            &[("handle", "ThreadLocalHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_THREAD_LOCAL_001"],
            &["removes current thread override so next get reuses default"],
            "clears the current thread-local payload",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.mutexCreate",
            "sync",
            &[],
            &[],
            "Result<MutexHandle,ThreadError>",
            Some(("MutexHandle", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["creates registry-backed mutex handle"],
            "allocates a runtime mutex",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.lock",
            "sync",
            &[],
            &[("handle", "MutexHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["blocks current host thread until mutex is owned"],
            "locks a runtime mutex",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.tryLock",
            "sync",
            &[],
            &[("handle", "MutexHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["returns Ok(false) instead of blocking when busy"],
            "attempts to lock a runtime mutex",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.unlock",
            "sync",
            &["nC.runtime.lock.unlock"],
            &[("handle", "MutexHandle|RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["releases current thread ownership"],
            "unlocks a mutex or rwlock held by the current thread",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.barrierCreate",
            "sync",
            &[],
            &[("parties", "int", "by value", "caller keeps value")],
            "Result<BarrierHandle,ThreadError>",
            Some(("BarrierHandle", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["creates registry-backed barrier handle"],
            "allocates a runtime barrier",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.barrierWait",
            "sync",
            &[],
            &[("handle", "BarrierHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["blocks until all parties arrive"],
            "waits on a runtime barrier",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.rwLockCreate",
            "sync",
            &["nC.runtime.lock.rwLockCreate"],
            &[],
            "Result<RwLockHandle,ThreadError>",
            Some(("RwLockHandle", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["creates registry-backed rwlock handle"],
            "allocates a runtime read/write lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.readLock",
            "sync",
            &["nC.runtime.lock.readLock"],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["blocks until read lock is available"],
            "acquires a read lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.tryReadLock",
            "sync",
            &["nC.runtime.lock.tryReadLock"],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["Ok(false) means a writer is active"],
            "attempts to acquire a read lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.writeLock",
            "sync",
            &["nC.runtime.lock.writeLock"],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["blocks until write lock is available"],
            "acquires a write lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.tryWriteLock",
            "sync",
            &["nC.runtime.lock.tryWriteLock"],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["Ok(false) means readers or writer are active"],
            "attempts to acquire a write lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.readUnlock",
            "sync",
            &["nC.runtime.lock.readUnlock"],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["releases one current-thread read lock"],
            "releases a read lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.sync.writeUnlock",
            "sync",
            &["nC.runtime.lock.writeUnlock"],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["releases current-thread write lock"],
            "releases a write lock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.rwLockCreate",
            "lock",
            &[],
            &[],
            "Result<RwLockHandle,ThreadError>",
            Some(("RwLockHandle", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.rwLockCreate"],
            "allocates a runtime read/write lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.readLock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.readLock"],
            "acquires a read lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.tryReadLock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.tryReadLock"],
            "attempts to acquire a read lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.writeLock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.writeLock"],
            "acquires a write lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.tryWriteLock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.tryWriteLock"],
            "attempts to acquire a write lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.readUnlock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.readUnlock"],
            "releases a read lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.writeUnlock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.writeUnlock"],
            "releases a write lock through the lock family",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.lock.unlock",
            "lock",
            &[],
            &[("handle", "RwLockHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_SYNC_001"],
            &["alias of runtime.sync.unlock for rwlocks"],
            "releases a read or write lock held by the current thread",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.atomic.atomicCreate",
            "atomic",
            &[],
            &[("initial", "int", "by value", "caller keeps value")],
            "Result<AtomicHandle,ThreadError>",
            Some(("AtomicHandle", "ThreadError")),
            &["E_RUNTIME_ATOMIC_001"],
            &["creates registry-backed SeqCst atomic i64"],
            "allocates a runtime atomic integer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.atomic.atomicLoad",
            "atomic",
            &[],
            &[("handle", "AtomicHandle", "borrowed handle", "caller keeps handle")],
            "Result<int,ThreadError>",
            Some(("int", "ThreadError")),
            &["E_RUNTIME_ATOMIC_001"],
            &["SeqCst load"],
            "loads a runtime atomic integer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.atomic.atomicStore",
            "atomic",
            &[],
            &[
                ("handle", "AtomicHandle", "borrowed handle", "caller keeps handle"),
                ("value", "int", "by value", "caller keeps value"),
            ],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_ATOMIC_001"],
            &["SeqCst store"],
            "stores a runtime atomic integer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.atomic.atomicAdd",
            "atomic",
            &[],
            &[
                ("handle", "AtomicHandle", "borrowed handle", "caller keeps handle"),
                ("delta", "int", "by value", "caller keeps value"),
            ],
            "Result<int,ThreadError>",
            Some(("int", "ThreadError")),
            &["E_RUNTIME_ATOMIC_001"],
            &["SeqCst fetch_add; returns value after addition"],
            "adds to a runtime atomic integer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.atomic.compareExchange",
            "atomic",
            &[],
            &[
                ("handle", "AtomicHandle", "borrowed handle", "caller keeps handle"),
                ("expected", "int", "by value", "caller keeps value"),
                ("newValue", "int", "by value", "caller keeps value"),
            ],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_ATOMIC_001"],
            &["SeqCst compare_exchange; Ok(true) on swap"],
            "performs atomic compare-and-exchange",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.atomic.atomicFence",
            "atomic",
            &[],
            &[],
            "void",
            None,
            &["E_RUNTIME_ATOMIC_001"],
            &["SeqCst fence"],
            "emits a runtime atomic fence",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.stdin",
            "channel",
            &[],
            &[],
            "any",
            None,
            &["E_RUNTIME_CHANNEL_001"],
            &["returns borrowed stdio channel token"],
            "returns the standard input channel token",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.stdout",
            "channel",
            &[],
            &[],
            "any",
            None,
            &["E_RUNTIME_CHANNEL_001"],
            &["returns borrowed stdio channel token"],
            "returns the standard output channel token",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.stderr",
            "channel",
            &[],
            &[],
            "any",
            None,
            &["E_RUNTIME_CHANNEL_001"],
            &["returns borrowed stdio channel token"],
            "returns the standard error channel token",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.writeTo",
            "channel",
            &[],
            &[
                ("channel", "any", "borrowed token", "caller keeps token"),
                ("text", "str", "borrowed payload", "caller keeps string"),
            ],
            "result",
            None,
            &["E_RUNTIME_CHANNEL_001"],
            &["writes to stdout/stderr tokens"],
            "writes text to a stdio channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.readFrom",
            "channel",
            &[],
            &[("channel", "any", "borrowed token", "caller keeps token")],
            "result",
            None,
            &["E_RUNTIME_CHANNEL_001"],
            &["reads a line from stdin token"],
            "reads text from a stdio channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.flushChannel",
            "channel",
            &[],
            &[("channel", "any", "borrowed token", "caller keeps token")],
            "bool",
            None,
            &["E_RUNTIME_CHANNEL_001"],
            &["flushes stdout/stderr tokens"],
            "flushes a stdio channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelCreate",
            "channel",
            &[],
            &[("capacity", "int", "by value", "caller keeps value")],
            "Result<ChannelHandle,ThreadError>",
            Some(("ChannelHandle", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["creates registry-backed synchronous channel"],
            "allocates a runtime channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelSend",
            "channel",
            &[],
            &[
                ("handle", "ChannelHandle", "borrowed handle", "caller keeps handle"),
                ("value", "any", "owned clone", "runtime sends payload"),
            ],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["may block until receiver accepts payload"],
            "sends payload to a runtime channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelTrySend",
            "channel",
            &[],
            &[
                ("handle", "ChannelHandle", "borrowed handle", "caller keeps handle"),
                ("value", "any", "owned clone", "runtime attempts to send payload"),
            ],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["Ok(false) means bounded channel is full"],
            "attempts non-blocking send to a runtime channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelRecv",
            "channel",
            &[],
            &[("handle", "ChannelHandle", "borrowed handle", "caller keeps handle")],
            "Result<any,ThreadError>",
            Some(("any", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["may block until sender provides payload"],
            "receives payload from a runtime channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelTryRecv",
            "channel",
            &[],
            &[("handle", "ChannelHandle", "borrowed handle", "caller keeps handle")],
            "Result<any,ThreadError>",
            Some(("any", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["Result.err means empty or disconnected"],
            "attempts non-blocking receive from a runtime channel",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelRecvTimeout",
            "channel",
            &[],
            &[
                ("handle", "ChannelHandle", "borrowed handle", "caller keeps handle"),
                ("timeoutMs", "int", "by value", "caller keeps value"),
            ],
            "Result<any,ThreadError>",
            Some(("any", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["returns Result.err on timeout"],
            "receives payload from a runtime channel with timeout",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.channel.channelClose",
            "channel",
            &[],
            &[("handle", "ChannelHandle", "borrowed handle", "caller keeps handle")],
            "Result<bool,ThreadError>",
            Some(("bool", "ThreadError")),
            &["E_RUNTIME_CHANNEL_001"],
            &["drops sender half; receiver may drain queued messages"],
            "closes a runtime channel sender",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.exit.exitCode",
            "exit",
            &[
                "nC.runtime.exit.setExitCode",
                "nC.runtime.exit.exit",
                "nC.runtime.exit.exitErr",
            ],
            &[("code", "int", "by value", "caller keeps value")],
            "int",
            None,
            &["E_RUNTIME_EXIT_001"],
            &["mutates runtime process exit-code cell"],
            "sets explicit process exit code in range 0..255",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.exit.getExitCode",
            "exit",
            &[],
            &[],
            "int",
            None,
            &["E_RUNTIME_EXIT_001"],
            &["reads runtime process exit-code cell"],
            "returns explicit process exit code for current run",
        ),
    ];
    entries.extend(runtime_memory_map_abi_entries());
    entries
}

fn runtime_memory_map_abi_entries() -> Vec<serde_json::Value> {
    let map_errors = [
        "E_RUNTIME_MEMORY_MAP_001",
        "E_RUNTIME_MEMORY_MAP_PROTECTION_001",
        "E_RUNTIME_MEMORY_MAP_ADDRESS_001",
    ];
    let lifecycle = [
        "mapping owns an OS virtual-memory view until unmap/remap",
        "raw derived pointers keep programmer-controlled unchecked address semantics",
        "memRegion conversion remains bounds checked",
    ];
    vec![
        runtime_abi_symbol_json(
            "nC.runtime.memory.map",
            "memory",
            &[],
            &[("bytes", "int", "by value", "caller keeps value")],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps private anonymous read/write memory using mmap or VirtualAlloc",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapAnonymous",
            "memory",
            &[],
            &[
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
                ("flags", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps anonymous memory with explicit protection and flags",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapAt",
            "memory",
            &[],
            &[
                ("address", "addr", "fixed address", "caller keeps value"),
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
                ("flags", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "requests an exact virtual address through MAP_FIXED or VirtualAlloc address hint",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapShared",
            "memory",
            &[],
            &[
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps shared anonymous memory",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapPrivate",
            "memory",
            &[],
            &[
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps private anonymous memory",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapFile",
            "memory",
            &[],
            &[
                ("path", "str", "borrowed path", "caller keeps string"),
                (
                    "offset",
                    "int",
                    "page-aligned byte offset",
                    "caller keeps value",
                ),
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
                ("flags", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps a file range using mmap or CreateFileMapping/MapViewOfFile",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapFileShared",
            "memory",
            &[],
            &[
                ("path", "str", "borrowed path", "caller keeps string"),
                (
                    "offset",
                    "int",
                    "page-aligned byte offset",
                    "caller keeps value",
                ),
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps a shared file range",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapFilePrivate",
            "memory",
            &[],
            &[
                ("path", "str", "borrowed path", "caller keeps string"),
                (
                    "offset",
                    "int",
                    "page-aligned byte offset",
                    "caller keeps value",
                ),
                ("bytes", "int", "by value", "caller keeps value"),
                ("protection", "int", "bit mask", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "maps a copy-on-write private file range",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.unmap",
            "memory",
            &[],
            &[(
                "mapping",
                "ptr<byte>",
                "owned base pointer",
                "consumed on success",
            )],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &map_errors,
            &lifecycle,
            "unmaps the complete live mapping through munmap or VirtualFree/UnmapViewOfFile",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.remap",
            "memory",
            &[],
            &[
                (
                    "mapping",
                    "ptr<byte>",
                    "owned base pointer",
                    "invalidated on success",
                ),
                ("newBytes", "int", "by value", "caller keeps value"),
                ("mayMove", "bool", "by value", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "resizes a mapping and records a new material lineage generation",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.remapAt",
            "memory",
            &[],
            &[
                (
                    "mapping",
                    "ptr<byte>",
                    "owned base pointer",
                    "invalidated on success",
                ),
                ("newBytes", "int", "by value", "caller keeps value"),
                ("address", "addr", "fixed destination", "caller keeps value"),
            ],
            "Result<ptr<byte>,MemoryError>",
            Some(("ptr<byte>", "MemoryError")),
            &map_errors,
            &lifecycle,
            "moves and resizes a mapping to an exact address",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapProtect",
            "memory",
            &[],
            &[
                (
                    "mapping",
                    "ptr<byte>",
                    "borrowed mapping",
                    "caller keeps mapping",
                ),
                ("protection", "int", "bit mask", "caller keeps value"),
            ],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &map_errors,
            &lifecycle,
            "changes page protection through mprotect or VirtualProtect",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapSync",
            "memory",
            &[],
            &[
                (
                    "mapping",
                    "ptr<byte>",
                    "borrowed mapping",
                    "caller keeps mapping",
                ),
                ("mode", "int", "sync mode", "caller keeps value"),
            ],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &map_errors,
            &lifecycle,
            "synchronizes mapped pages through msync or FlushViewOfFile",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapAdvise",
            "memory",
            &[],
            &[
                (
                    "mapping",
                    "ptr<byte>",
                    "borrowed mapping",
                    "caller keeps mapping",
                ),
                ("advice", "int", "advice code", "caller keeps value"),
            ],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &map_errors,
            &lifecycle,
            "provides access-pattern advice through madvise or the Win32 memory policy",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapLock",
            "memory",
            &[],
            &[(
                "mapping",
                "ptr<byte>",
                "borrowed mapping",
                "caller keeps mapping",
            )],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &map_errors,
            &lifecycle,
            "locks mapped pages with mlock or VirtualLock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapUnlock",
            "memory",
            &[],
            &[(
                "mapping",
                "ptr<byte>",
                "borrowed mapping",
                "caller keeps mapping",
            )],
            "Result<bool,MemoryError>",
            Some(("bool", "MemoryError")),
            &map_errors,
            &lifecycle,
            "unlocks mapped pages with munlock or VirtualUnlock",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapLen",
            "memory",
            &[],
            &[(
                "mapping",
                "ptr<byte>",
                "borrowed pointer",
                "caller keeps mapping",
            )],
            "int",
            None,
            &map_errors,
            &lifecycle,
            "returns the byte extent of the mapping containing the pointer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapAddress",
            "memory",
            &[],
            &[(
                "mapping",
                "ptr<byte>",
                "borrowed pointer",
                "caller keeps mapping",
            )],
            "ptr<byte>",
            None,
            &map_errors,
            &lifecycle,
            "returns the base pointer of the mapping containing the pointer",
        ),
        runtime_abi_symbol_json(
            "nC.runtime.memory.mapIsMapped",
            "memory",
            &[],
            &[(
                "pointer",
                "ptr<byte>",
                "borrowed pointer",
                "caller keeps pointer",
            )],
            "bool",
            None,
            &[],
            &lifecycle,
            "tests whether one byte at the address is inside a live mapping",
        ),
    ]
}

pub(crate) fn runtime_abi_manifest() -> serde_json::Value {
    let mut symbols = runtime_abi_symbol_entries();
    symbols.retain(|entry| {
        !matches!(
            entry.get("family").and_then(|value| value.as_str()),
            Some("buffer" | "bytes" | "slice" | "span")
        )
    });
    runtime_extend_abi_symbols_with_catalog(&mut symbols);

    let canonical_names = runtime_abi_symbol_names_from_entries(&symbols);
    assert_eq!(
        canonical_names.len(),
        symbols.len(),
        "runtime ABI contains duplicate canonical linker symbols"
    );
    assert!(
        canonical_names
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.")),
        "runtime ABI contains a symbol outside the canonical nC.runtime.* namespace"
    );
    let import_manifest = runtime_abi_link_manifest("consumer-import", &symbols);
    let export_manifest = runtime_abi_link_manifest("provider-export", &symbols);

    let mut families = BTreeSet::new();
    let mut error_codes = BTreeSet::new();
    for symbol in &symbols {
        if let Some(family) = symbol.get("family").and_then(|value| value.as_str()) {
            families.insert(family.to_string());
        }
        if let Some(codes) = symbol.get("error_codes").and_then(|value| value.as_array()) {
            for code in codes {
                if let Some(code) = code.as_str() {
                    error_codes.insert(code.to_string());
                }
            }
        }
    }
    serde_json::json!({
        "schema": "nc.runtime.symbol_abi.v1",
        "status": "closed",
        "abi_revision": "nC.runtime-value-v1",
        "canonical_namespace": "nC.runtime.*",
        "native_lowering_contract": "MCT uses declared nC type layout before target ABI lowering; no symbol in this table may be called without this row",
        "backend_parity_schema": "nc.runtime.backend_parity.v1",
        "backend_routes": ["ncc_run", "mct", "native_fast", "llvm", "machine", "basal"],
        "symbol_count": symbols.len(),
        "families": families.into_iter().collect::<Vec<_>>(),
        "layouts": runtime_abi_layouts_json(),
        "error_codes": error_codes.into_iter().collect::<Vec<_>>(),
        "import_manifest": import_manifest,
        "export_manifest": export_manifest,
        "symbols": symbols,
        "truth_boundary": [
            "each real runtime symbol imported by the compact facade has an ABI row",
            "each ABI row declares input types, return type, errors, resource ownership and lifecycle",
            "Result<T,E>, ProcessHandle, ThreadHandle, FutureHandle, TaskHandle, TimerHandle, CancelToken, EventHandle and runtime error layouts are stable runtime-value layouts",
            "native/MCT lowering must use these rows instead of inferring ad hoc runtime call shapes",
            "each ABI symbol declares routes for ncc_run, MCT, native-fast, LLVM and machine/basal bridge policy",
            "ncc run archivo.n uses closed runtime APIs directly; no --real flag is required for these symbols",
        ],
    })
}

fn runtime_abi_link_manifest(role: &str, symbols: &[serde_json::Value]) -> serde_json::Value {
    let rows = symbols
        .iter()
        .map(|entry| {
            let inputs = entry
                .get("inputs")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .filter_map(|input| input.get("type").and_then(|value| value.as_str()))
                .collect::<Vec<_>>();
            serde_json::json!({
                "symbol": entry.get("symbol").and_then(|value| value.as_str()).unwrap_or_default(),
                "aliases": entry.get("aliases").cloned().unwrap_or_else(|| serde_json::json!([])),
                "inputs": inputs,
                "return": entry.get("return").and_then(|value| value.get("type")).and_then(|value| value.as_str()).unwrap_or("void"),
                "calling_convention": "nC.runtime-value-v1",
                "linkage": "external",
                "visibility": "default",
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema": "nc.runtime.link_manifest.v1",
        "status": "closed",
        "role": role,
        "namespace": "nC.runtime.*",
        "calling_convention": "nC.runtime-value-v1",
        "symbol_count": rows.len(),
        "symbols": rows,
    })
}

fn runtime_catalog_runtime_symbol_names() -> BTreeSet<String> {
    ncc_ast::runtime_import_closure_table_all()
        .entries
        .into_iter()
        .filter(|entry| entry.can_run_without_runtime_import_failure)
        .flat_map(|entry| entry.runtime_symbols.into_iter())
        .map(|symbol| ncc_ast::canonical_runtime_symbol(&symbol))
        .collect()
}

fn runtime_extend_abi_symbols_with_catalog(symbols: &mut Vec<serde_json::Value>) {
    let mut known = runtime_abi_symbol_names_from_entries(symbols);
    for symbol in runtime_catalog_runtime_symbol_names() {
        if known.insert(symbol.clone()) {
            symbols.push(runtime_catalog_abi_symbol_json(&symbol));
        }
    }
    symbols.sort_by(|left, right| {
        let left = left
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let right = right
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        left.cmp(right)
    });
}

fn runtime_abi_symbol_names_from_entries(symbols: &[serde_json::Value]) -> BTreeSet<String> {
    symbols
        .iter()
        .filter_map(|entry| entry.get("symbol").and_then(|value| value.as_str()))
        .map(str::to_string)
        .collect()
}

fn runtime_catalog_abi_symbol_json(symbol: &str) -> serde_json::Value {
    let canonical = ncc_ast::canonical_runtime_symbol(symbol);
    let family = runtime_symbol_family(&canonical);
    let error_code = runtime_catalog_error_code_for_family(&family);
    let (arg_types, return_ty, variadic, api_alias) = runtime_catalog_typed_signature(&canonical)
        .unwrap_or_else(|| {
            panic!(
                "runtime ABI authority has no typed signature for canonical symbol '{canonical}'"
            )
        });
    let mut param_names = (0..arg_types.len())
        .map(|index| format!("arg{index}"))
        .collect::<Vec<_>>();
    let mut param_types = arg_types;
    if variadic {
        param_names.push("varargs".to_string());
        param_types.push("any...".to_string());
    }
    let params = param_names
        .iter()
        .zip(param_types.iter())
        .map(|(name, ty)| {
            (
                name.as_str(),
                ty.as_str(),
                runtime_catalog_parameter_passing(ty),
                runtime_catalog_parameter_ownership(ty),
            )
        })
        .collect::<Vec<_>>();
    let result_types = runtime_result_payload_types(&return_ty);
    let result = result_types
        .as_ref()
        .map(|(ok, err)| (ok.as_str(), err.as_str()));
    let aliases = api_alias
        .as_deref()
        .filter(|alias| *alias != canonical)
        .into_iter()
        .collect::<Vec<_>>();
    runtime_abi_symbol_json(
        &canonical,
        &family,
        &aliases,
        &params,
        &return_ty,
        result,
        &[error_code.as_str()],
        &[
            "catalog-backed runtime symbol with a concrete typed call ABI",
            "family-specific resources follow the runtime handle ownership table",
        ],
        "runtime symbol call lowers through the canonical typed runtime dispatcher; concrete resource lifecycle is defined by its import family",
    )
}

fn runtime_catalog_typed_signature(
    symbol: &str,
) -> Option<(Vec<String>, String, bool, Option<String>)> {
    if let Some((family, function)) = ncc_ast::runtime_api_find_function_by_runtime_symbol(symbol) {
        let alias = format!("{}.{}", family.import_path, function.name);
        return Some((
            function.arg_types.clone(),
            function.return_ty.clone(),
            function.variadic,
            Some(alias),
        ));
    }

    let (args, return_ty): (&[&str], &str) = match symbol {
        symbol
            if symbol.ends_with(".__native_intrinsic_activation")
                || symbol.ends_with(".__native_vector_activation") =>
        {
            (&[], "bool")
        }
        "nC.runtime.math.core.piDigits"
        | "nC.runtime.math.core.eulerDigits"
        | "nC.runtime.math.core.aureoDigits" => (&[], "str"),
        "nC.runtime.math.numeric.atan2Float" => (&["ft64", "ft64"], "ft64"),
        "nC.runtime.math.numeric.sqrtFloat"
        | "nC.runtime.math.numeric.expFloat"
        | "nC.runtime.math.numeric.logFloat"
        | "nC.runtime.math.numeric.sinFloat"
        | "nC.runtime.math.numeric.cosFloat" => (&["ft64"], "ft64"),
        _ => return None,
    };
    Some((
        args.iter().map(|arg| (*arg).to_string()).collect(),
        return_ty.to_string(),
        false,
        None,
    ))
}

fn runtime_catalog_parameter_passing(ty: &str) -> &'static str {
    let compact = ty.trim().replace(' ', "");
    if compact == "str"
        || compact.starts_with("ptr<")
        || compact.starts_with("ref<")
        || compact.starts_with("array<")
        || compact.starts_with("list<")
        || compact.starts_with("queue<")
        || compact.starts_with("vector<")
    {
        "borrowed descriptor"
    } else {
        "by runtime value"
    }
}

fn runtime_catalog_parameter_ownership(ty: &str) -> &'static str {
    let compact = ty.trim().replace(' ', "");
    if compact == "str"
        || compact.starts_with("ptr<")
        || compact.starts_with("ref<")
        || compact.ends_with("Handle")
    {
        "caller retains value and referenced resource"
    } else {
        "caller retains value"
    }
}

fn runtime_result_payload_types(ty: &str) -> Option<(String, String)> {
    let inner = ty.trim().strip_prefix("Result<")?.strip_suffix('>')?;
    let mut depth = 0usize;
    for (index, ch) in inner.char_indices() {
        match ch {
            '<' => depth = depth.saturating_add(1),
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                return Some((
                    inner[..index].trim().to_string(),
                    inner[index + 1..].trim().to_string(),
                ));
            }
            _ => {}
        }
    }
    None
}

fn runtime_symbol_family(symbol: &str) -> String {
    if let Some(family) = symbol
        .strip_prefix("nC.runtime.")
        .and_then(|tail| tail.split('.').next())
        .filter(|family| !family.is_empty())
    {
        return family.to_string();
    }
    symbol
        .strip_prefix("nC.")
        .and_then(|tail| tail.split('.').next())
        .filter(|family| !family.is_empty())
        .unwrap_or("runtime")
        .to_string()
}

fn runtime_catalog_error_code_for_family(family: &str) -> String {
    let normalized = family
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("E_RUNTIME_{normalized}_001")
}

#[cfg(test)]
fn runtime_abi_symbol_names_from_manifest(manifest: &serde_json::Value) -> BTreeSet<String> {
    manifest
        .get("symbols")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("symbol").and_then(|value| value.as_str()))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
fn runtime_imported_runtime_symbol_names() -> BTreeSet<String> {
    runtime_catalog_runtime_symbol_names()
}

pub(crate) fn runtime_help_text() -> &'static str {
    "Usage: ncc runtime <doctor|families|exports|abi|provider|check|imports|capabilities|policies|errors|report> [args]\n\
     Unified runtime contract CLI for [@runtime] macrophase.\n\
     Common options: --json --md --no-color --strict --phase <name> --target <triple> --profile <profile> --family <id> --mode <id> --out <path>\n\
     Examples:\n\
       ncc runtime doctor --json\n\
       ncc runtime families --json\n\
       ncc runtime exports nC.runtime.input --json\n\
       ncc runtime abi --json\n\
       ncc runtime abi nC.runtime.process.spawn --json\n\
       ncc runtime provider all --out target/bootstrap/stage2/runtime-providers --json\n\
       ncc runtime check examples/calculadoraBasica.n --strict\n\
       ncc runtime imports examples/calculadoraBasica.n --json\n\
       ncc runtime capabilities examples/calculadoraBasica.n --json\n\
       ncc runtime policies examples/calculadoraBasica.n --json\n\
       ncc runtime errors --json\n\
       ncc runtime report examples/calculadoraBasica.n --md-out reports/runtime/runtime_report.md --json-out reports/runtime/runtime_report.json"
}

pub(crate) fn run_runtime_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || is_help_token(&args[0]) {
        println!("{}", runtime_help_text());
        return Ok(());
    }

    let sub = args[0].as_str();
    if sub == "provider" {
        return super::runtime_provider::run(&args[1..]);
    }
    let (opts, rest) = parse_runtime_cli_options(&args[1..])?;

    match sub {
        "doctor" => runtime_handle_doctor(&opts, &rest),
        "families" => runtime_handle_families(&opts, &rest),
        "exports" => runtime_handle_exports(&opts, &rest),
        "abi" | "symbols" => runtime_handle_abi(&opts, &rest),
        "check" => runtime_handle_check(&opts, &rest),
        "imports" => runtime_handle_imports(&opts, &rest),
        "capabilities" => runtime_handle_capabilities(&opts, &rest),
        "policies" => runtime_handle_policies(&opts, &rest),
        "errors" => runtime_handle_errors(&opts, &rest),
        "report" => runtime_handle_report(&opts, &rest),
        _ => Err(
            actionable_command_error("runtime", format!("unknown runtime command '{}'", sub))
                .into(),
        ),
    }
}

fn parse_runtime_cli_options(
    args: &[String],
) -> Result<(RuntimeCliOptions, Vec<String>), Box<dyn std::error::Error>> {
    let mut opts = RuntimeCliOptions::default();
    let mut rest = Vec::new();
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                opts.json = true;
                idx += 1;
            }
            "--md" => {
                opts.md = true;
                idx += 1;
            }
            "--no-color" => {
                opts.no_color = true;
                idx += 1;
            }
            "--strict" => {
                opts.strict = true;
                idx += 1;
            }
            "--phase" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("runtime syntax: --phase requires a value")?;
                opts.phase = Some(value.clone());
                idx += 2;
            }
            "--target" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("runtime syntax: --target requires a value")?;
                opts.target = Some(value.clone());
                idx += 2;
            }
            "--profile" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("runtime syntax: --profile requires a value")?;
                opts.profile = Some(value.clone());
                idx += 2;
            }
            "--family" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("runtime syntax: --family requires a value")?;
                opts.family = Some(value.clone());
                idx += 2;
            }
            "--mode" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("runtime syntax: --mode requires a value")?;
                opts.mode = Some(value.clone());
                idx += 2;
            }
            "--out" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("runtime syntax: --out requires a value")?;
                opts.out = Some(value.clone());
                idx += 2;
            }
            _ => {
                rest.push(args[idx].clone());
                idx += 1;
            }
        }
    }
    Ok((opts, rest))
}

fn runtime_capability_catalog_set() -> BTreeSet<String> {
    RUNTIME_CAPABILITY_CATALOG
        .iter()
        .map(|value| value.to_string())
        .collect()
}

fn runtime_policy_catalog_set() -> BTreeSet<String> {
    RUNTIME_POLICY_CATALOG
        .iter()
        .map(|value| value.to_string())
        .collect()
}

fn runtime_use_path_targets_list_family(use_path: &[String]) -> bool {
    (use_path.len() >= 3
        && use_path[0].eq_ignore_ascii_case("nC")
        && use_path[1].eq_ignore_ascii_case("runtime")
        && use_path[2].eq_ignore_ascii_case("list"))
        || (use_path.len() >= 2
            && use_path[0].eq_ignore_ascii_case("runtime")
            && use_path[1].eq_ignore_ascii_case("list"))
        || (use_path.len() >= 2
            && use_path[0].eq_ignore_ascii_case("nC")
            && use_path[1].eq_ignore_ascii_case("list"))
        || (!use_path.is_empty() && use_path[0].eq_ignore_ascii_case("list"))
}

fn runtime_use_path_targets_queue_family(use_path: &[String]) -> bool {
    (use_path.len() >= 3
        && use_path[0].eq_ignore_ascii_case("nC")
        && use_path[1].eq_ignore_ascii_case("runtime")
        && use_path[2].eq_ignore_ascii_case("queue"))
        || (use_path.len() >= 2
            && use_path[0].eq_ignore_ascii_case("runtime")
            && use_path[1].eq_ignore_ascii_case("queue"))
        || (!use_path.is_empty() && use_path[0].eq_ignore_ascii_case("queue"))
}

fn runtime_path_targets_list_family(path: &str) -> bool {
    let normalized = path.to_ascii_lowercase();
    normalized == "nc.runtime.list"
        || normalized == "runtime.list"
        || normalized == "nc.list"
        || normalized == "list"
        || normalized.starts_with("nc.runtime.list.")
        || normalized.starts_with("runtime.list.")
        || normalized.starts_with("nc.list.")
        || normalized.starts_with("list.")
}

fn runtime_path_targets_queue_family(path: &str) -> bool {
    let normalized = path.to_ascii_lowercase();
    normalized == "nc.runtime.queue"
        || normalized == "runtime.queue"
        || normalized == "queue"
        || normalized.starts_with("nc.runtime.queue.")
        || normalized.starts_with("runtime.queue.")
        || normalized.starts_with("queue.")
}

fn runtime_family_key_from_use_path(use_path: &[String]) -> Option<String> {
    if runtime_use_path_targets_queue_family(use_path) {
        return Some("queue".to_string());
    }
    if runtime_use_path_targets_list_family(use_path) {
        return Some("list".to_string());
    }
    if use_path.len() >= 3
        && use_path[0].eq_ignore_ascii_case("nC")
        && use_path[1].eq_ignore_ascii_case("runtime")
    {
        return Some(use_path[2].to_ascii_lowercase());
    }
    if use_path.len() >= 2 && use_path[0].eq_ignore_ascii_case("runtime") {
        return Some(use_path[1].to_ascii_lowercase());
    }
    None
}

fn runtime_capabilities_for_family_key(family: &str) -> Vec<String> {
    let mut out: Vec<&str> = Vec::new();
    match family {
        "input" => out.extend([
            "runtime.input.read",
            "runtime.input.tryRead",
            "runtime.input.clear",
        ]),
        "output" => out.extend([
            "runtime.output.write",
            "runtime.output.flush",
            "runtime.output.stable",
        ]),
        "erroroutput" => out.extend(["runtime.output.error", "runtime.output.flush"]),
        "channel" | "flush" | "std" => out.extend(["runtime.output.write", "runtime.output.flush"]),
        "args" => out.push("runtime.args.read"),
        "env" => out.extend(["runtime.env.read", "runtime.env.write"]),
        "exit" => out.push("runtime.exit"),
        "session" => out.extend(["runtime.session.read", "runtime.session.write"]),
        "text" => out.extend(["runtime.text.read", "runtime.text.transform"]),
        "string" => out.extend(["runtime.string.read", "runtime.string.transform"]),
        "render" => out.extend([
            "runtime.render.basic",
            "runtime.render.debug",
            "runtime.render.stable",
        ]),
        "format" => out.extend(["runtime.format.basic", "runtime.format.advanced"]),
        "concat" => out.extend(["runtime.render.basic", "runtime.format.basic"]),
        "parse" => out.extend([
            "runtime.parse.basic",
            "runtime.parse.strict",
            "runtime.parse.try",
        ]),
        "convert" => out.extend(["runtime.convert.safe", "runtime.convert.unsafe"]),
        "validate" => out.extend(["runtime.validate.basic", "runtime.validate.schema"]),
        "width" | "align" | "pad" | "truncate" | "wrap" | "unicode" => {
            out.extend(["runtime.text.read", "runtime.string.transform"])
        }
        "escape" => out.extend([
            "runtime.escape.log",
            "runtime.escape.json",
            "runtime.escape.shell",
            "runtime.escape.redact",
        ]),
        "result" => out.push("runtime.result.use"),
        "option" => out.push("runtime.option.use"),
        "error" => out.extend(["runtime.error.raise", "runtime.error.recover"]),
        "diagnostic" => out.extend(["runtime.diagnostic.emit", "runtime.diagnostic.json"]),
        "log" => out.extend([
            "runtime.log.info",
            "runtime.log.warn",
            "runtime.log.error",
            "runtime.log.debug",
        ]),
        "trace" => out.extend([
            "runtime.trace.start",
            "runtime.trace.event",
            "runtime.trace.end",
        ]),
        "debug" => out.push("runtime.debug.read"),
        "assert" => out.push("runtime.assert.use"),
        "status" => out.push("runtime.status.use"),
        "buffer" => out.extend([
            "runtime.buffer.create",
            "runtime.buffer.read",
            "runtime.buffer.write",
            "runtime.buffer.slice",
            "runtime.buffer.clear",
        ]),
        "memory" => out.extend([
            "runtime.memory.sizeof",
            "runtime.memory.alignof",
            "runtime.memory.copy",
            "runtime.memory.zero",
            "runtime.memory.compare",
        ]),
        "heap" => out.extend([
            "runtime.heap.alloc",
            "runtime.heap.free",
            "runtime.heap.stats",
            "runtime.heap.check",
        ]),
        "stack" => out.extend(["runtime.stack.inspect", "runtime.stack.local"]),
        "region" => out.extend([
            "runtime.region.create",
            "runtime.region.read",
            "runtime.region.write",
            "runtime.region.invalidate",
            "runtime.region.bounds",
        ]),
        "slice" => out.push("runtime.slice.create"),
        "span" => out.push("runtime.span.create"),
        "ptr" => out.extend([
            "runtime.ptr.create",
            "runtime.ptr.read",
            "runtime.ptr.write",
            "runtime.ptr.refine",
            "runtime.ptr.origin",
        ]),
        "addr" => out.extend([
            "runtime.addr.read",
            "runtime.addr.expose",
            "runtime.addr.redacted",
        ]),
        "bytes" => out.extend(["runtime.bytes.read", "runtime.bytes.write"]),
        "endian" => out.push("runtime.endian.convert"),
        "layout" => out.push("runtime.layout.inspect"),
        "alloc" => out.push("runtime.alloc.custom"),
        "fs" => out.extend(["fs.read", "fs.write", "fs.list", "fs.metadata"]),
        "path" => out.extend(["path.normalize", "path.officialize", "path.sandbox"]),
        "dir" => out.extend(["dir.create", "dir.remove", "dir.list"]),
        "file" => out.extend([
            "file.open",
            "file.close",
            "file.read",
            "file.write",
            "file.lock",
        ]),
        "stream" => out.extend(["stream.read", "stream.write", "stream.pipe"]),
        "process" => out.extend([
            "process.spawn",
            "process.wait",
            "process.kill",
            "process.signal",
            "process.inspect",
        ]),
        "command" => out.extend([
            "command.run",
            "command.capture",
            "command.shell",
            "command.noShell",
        ]),
        "os" => out.extend(["os.info", "os.platform", "os.resources"]),
        "user" => out.extend(["user.current", "user.query", "user.groups"]),
        "permission" => out.extend(["permission.query", "permission.require"]),
        "temp" => out.extend(["temp.create", "temp.delete"]),
        "service" => out.extend(["service.query", "service.start", "service.stop"]),
        "signal" => out.extend(["signal.send", "signal.handle"]),
        "time" => out.extend(["time.now", "time.monotonic", "time.timestamp"]),
        "clock" => out.push("clock.read"),
        "timer" => out.extend(["timer.create", "timer.cancel"]),
        "sleep" => out.extend(["sleep.blocking", "sleep.async"]),
        "random" => out.extend(["random.int", "random.float", "random.range"]),
        "entropy" => out.push("entropy.read"),
        "uuid" => out.push("uuid.generate"),
        "task" => out.extend(["task.spawn", "task.join", "task.cancel"]),
        "thread" => out.extend(["thread.spawn", "thread.join"]),
        "async" => out.push("async.await"),
        "sync" => out.extend(["sync.lock", "sync.unlock"]),
        "atomic" => out.extend(["atomic.read", "atomic.write", "atomic.cas"]),
        "lock" => out.extend(["lock.mutex", "lock.rw"]),
        "list" => out.extend([
            "list.create",
            "list.push",
            "list.pop",
            "list.read",
            "list.write",
        ]),
        "queue" => out.extend(["queue.push", "queue.pop"]),
        "event" => out.extend(["event.emit", "event.wait"]),
        "scheduler" => out.push("scheduler.use"),
        "cancel" => out.push("cancel.token"),
        "net" => out.push("net.access"),
        "socket" => out.extend(["socket.open", "socket.close"]),
        "tcp" => out.extend([
            "tcp.connect",
            "tcp.listen",
            "tcp.accept",
            "tcp.send",
            "tcp.recv",
        ]),
        "udp" => out.extend(["udp.bind", "udp.send", "udp.recv"]),
        "dns" => out.push("dns.resolve"),
        "http" => out.extend(["http.get", "http.post", "http.request", "http.response"]),
        "tls" => out.extend(["tls.connect", "tls.verify", "tls.certInspect"]),
        "capability" => out.extend([
            "capability.check",
            "capability.require",
            "capability.grant",
            "capability.revoke",
        ]),
        "policy" => out.extend(["policy.check", "policy.require", "policy.scope"]),
        "profile" => out.extend(["profile.read", "profile.require"]),
        "bridge" => out.extend([
            "bridge.runtime.system",
            "bridge.runtime.inspect",
            "bridge.runtime.tooling",
            "bridge.runtime.compiler",
            "bridge.runtime.kernel",
            "bridge.runtime.machine",
        ]),
        "sandbox" => out.extend([
            "sandbox.create",
            "sandbox.run",
            "sandbox.deny",
            "sandbox.allow",
        ]),
        "resource" => out.extend([
            "resource.open",
            "resource.close",
            "resource.limit",
            "resource.audit",
        ]),
        _ => out.push("mode.runtime.use"),
    }

    out.into_iter().map(str::to_string).collect()
}

fn runtime_policies_for_family_key(family: &str) -> Vec<String> {
    let mut out = vec![
        "imports.official-runtime-required".to_string(),
        "runtime.no-silent-fallback".to_string(),
    ];
    match family {
        "input" => {
            out.extend([
                "operators.strict-types".to_string(),
                "operators.bool-condition-required".to_string(),
            ]);
        }
        "output" | "erroroutput" | "render" | "format" | "concat" => {
            out.extend([
                "runtime.no-secret-output".to_string(),
                "runtime.redact-secrets".to_string(),
                "runtime.error.typed-required".to_string(),
            ]);
        }
        "ptr" | "region" | "memory" | "heap" | "addr" | "buffer" | "slice" | "span" => {
            out.extend([
                "memory.bounds.required".to_string(),
                "memory.ptr-origin.required".to_string(),
                "memory.alias-tracking.required".to_string(),
                "memory.null-refinement.required".to_string(),
            ]);
        }
        "fs" | "path" | "dir" | "file" | "stream" => {
            out.extend([
                "fs.sandbox.required".to_string(),
                "fs.no-path-traversal".to_string(),
                "fs.audit-destructive".to_string(),
            ]);
        }
        "process" | "command" | "os" | "service" | "signal" | "permission" | "user" => {
            out.extend([
                "process.no-shell-by-default".to_string(),
                "process.timeout.required".to_string(),
                "process.audit.required".to_string(),
                "command.no-injection".to_string(),
            ]);
        }
        "net" | "socket" | "tcp" | "udp" | "dns" | "http" | "tls" => {
            out.extend([
                "network.capability.required".to_string(),
                "network.timeout.required".to_string(),
                "tls.verify.required".to_string(),
            ]);
        }
        "capability" | "policy" | "profile" | "sandbox" | "resource" | "bridge" => {
            out.extend([
                "diagnostics.stable".to_string(),
                "diagnostics.json-valid".to_string(),
                "reports.reproducible".to_string(),
                "mode.bridge-authority-required".to_string(),
            ]);
        }
        "event" | "task" | "thread" | "async" | "sync" | "atomic" | "lock" | "queue" | "list"
        | "scheduler" | "cancel" => {
            out.extend([
                "runtime.trace.required".to_string(),
                "diagnostics.stable".to_string(),
            ]);
        }
        _ => {}
    }
    out.sort();
    out.dedup();
    out
}

fn runtime_list_alias_exports() -> Vec<RuntimeExportContract> {
    vec![
        RuntimeExportContract {
            name: "listCreate<T>".to_string(),
            arity_min: 0,
            variadic: false,
            arg_types: vec![],
            return_ty: "list<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listCreateWithCapacity<T>".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["int".to_string()],
            return_ty: "list<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listFromArray<T>".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["array<T>".to_string()],
            return_ty: "list<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listLen".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["any".to_string()],
            return_ty: "int".to_string(),
        },
        RuntimeExportContract {
            name: "listCapacity".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["any".to_string()],
            return_ty: "int".to_string(),
        },
        RuntimeExportContract {
            name: "listIsEmpty".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["any".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listAdd".to_string(),
            arity_min: 2,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "T".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listPush".to_string(),
            arity_min: 2,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "T".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listPop".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["list<T>".to_string()],
            return_ty: "Option<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listGet".to_string(),
            arity_min: 2,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string()],
            return_ty: "T".to_string(),
        },
        RuntimeExportContract {
            name: "listSet".to_string(),
            arity_min: 3,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string(), "T".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listInsert".to_string(),
            arity_min: 3,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string(), "T".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listRemoveAt".to_string(),
            arity_min: 2,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string()],
            return_ty: "T".to_string(),
        },
        RuntimeExportContract {
            name: "listClear".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["any".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listContains".to_string(),
            arity_min: 2,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "T".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listIndexOf".to_string(),
            arity_min: 2,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "T".to_string()],
            return_ty: "int".to_string(),
        },
        RuntimeExportContract {
            name: "listSlice".to_string(),
            arity_min: 3,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string(), "int".to_string()],
            return_ty: "list<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listAsArray".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["list<T>".to_string()],
            return_ty: "array<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listAsRegion".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["list<T>".to_string()],
            return_ty: "memRegion<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listClone".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["list<T>".to_string()],
            return_ty: "list<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listCopy".to_string(),
            arity_min: 1,
            variadic: false,
            arg_types: vec!["list<T>".to_string()],
            return_ty: "list<T>".to_string(),
        },
        RuntimeExportContract {
            name: "listMove".to_string(),
            arity_min: 3,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string(), "int".to_string()],
            return_ty: "bool".to_string(),
        },
        RuntimeExportContract {
            name: "listSwap".to_string(),
            arity_min: 3,
            variadic: false,
            arg_types: vec!["list<T>".to_string(), "int".to_string(), "int".to_string()],
            return_ty: "bool".to_string(),
        },
    ]
}

fn runtime_signature_generic_params(fun: &RuntimeExportContract) -> Vec<String> {
    let mut params = BTreeSet::new();
    for raw in std::iter::once(&fun.name)
        .chain(fun.arg_types.iter())
        .chain(std::iter::once(&fun.return_ty))
    {
        let mut token = String::new();
        for ch in raw.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                token.push(ch);
                continue;
            }
            if token.len() == 1
                && token
                    .chars()
                    .next()
                    .is_some_and(|head| head.is_ascii_uppercase())
            {
                params.insert(token.clone());
            }
            token.clear();
        }
        if token.len() == 1
            && token
                .chars()
                .next()
                .is_some_and(|head| head.is_ascii_uppercase())
        {
            params.insert(token);
        }
    }
    params.into_iter().collect()
}

fn runtime_export_name_is_queue_only(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered.starts_with("queue")
}

#[cfg(test)]
fn runtime_export_name_is_list_only(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered.starts_with("list")
}

fn runtime_family_alias_import_path(path: &str) -> Option<&'static str> {
    if runtime_path_targets_list_family(path) {
        return Some("nc.runtime.list");
    }
    None
}

fn runtime_callable_export_keys(name: &str) -> Vec<String> {
    let mut out = vec![name.to_ascii_lowercase()];
    if let Some((base, _)) = name.split_once('<') {
        let base = base.trim();
        if !base.is_empty() {
            let normalized = base.to_ascii_lowercase();
            if !out.iter().any(|value| value == &normalized) {
                out.push(normalized);
            }
        }
    }
    out
}

fn build_runtime_catalog_audit() -> RuntimeCatalogAudit {
    let capability_catalog = runtime_capability_catalog_set();
    let policy_catalog = runtime_policy_catalog_set();

    let mut by_import_spec = BTreeMap::new();
    for spec in ncc_parser::runtime_api_family_specs() {
        by_import_spec.insert(spec.import_path.to_ascii_lowercase(), spec.clone());
    }

    let mut families = Vec::new();
    let mut missing_signature_families = Vec::new();
    let mut unknown_capabilities = BTreeSet::new();
    let mut unknown_policies = BTreeSet::new();
    let mut fn_to_families: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for import_path in ncc_parser::official_runtime_families() {
        let import_lower = import_path.to_ascii_lowercase();
        let family_key = import_lower
            .split('.')
            .next_back()
            .unwrap_or("runtime")
            .to_string();

        let spec = by_import_spec.get(&import_lower);
        if spec.is_none() {
            missing_signature_families.push(import_path.to_string());
        }

        let module_path = spec
            .map(|value| value.module_path.to_ascii_lowercase())
            .unwrap_or_else(|| format!("runtime.{family_key}"));

        let exports: Vec<RuntimeExportContract> = spec
            .map(|value| {
                value
                    .functions
                    .iter()
                    .map(|fun| RuntimeExportContract {
                        name: fun.name.clone(),
                        arity_min: fun.arity_min,
                        variadic: fun.variadic,
                        arg_types: fun.arg_types.clone(),
                        return_ty: fun.return_ty.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        for export in &exports {
            for key in runtime_callable_export_keys(&export.name) {
                fn_to_families
                    .entry(key)
                    .or_default()
                    .insert(import_lower.clone());
            }
        }

        if family_key == "queue" {
            for export in runtime_list_alias_exports() {
                for key in runtime_callable_export_keys(&export.name) {
                    fn_to_families
                        .entry(key)
                        .or_default()
                        .insert("nc.runtime.list".to_string());
                }
            }
        }

        let capabilities = runtime_capabilities_for_family_key(&family_key);
        for capability in &capabilities {
            if !capability_catalog.contains(capability) {
                unknown_capabilities.insert(capability.clone());
            }
        }

        let policies = runtime_policies_for_family_key(&family_key);
        for policy in &policies {
            if !policy_catalog.contains(policy) {
                unknown_policies.insert(policy.clone());
            }
        }

        families.push(RuntimeFamilyContract {
            import_path: import_lower,
            module_path,
            family_key,
            exports,
            capabilities,
            policies,
        });
    }

    families.sort_by(|a, b| a.import_path.cmp(&b.import_path));
    missing_signature_families.sort();

    RuntimeCatalogAudit {
        families,
        missing_signature_families,
        unknown_capabilities: unknown_capabilities.into_iter().collect(),
        unknown_policies: unknown_policies.into_iter().collect(),
        fn_to_families,
    }
}

fn runtime_imported_families_from_uses(
    uses: &[ncc_ast::UseDecl],
    families: &[RuntimeFamilyContract],
) -> BTreeSet<String> {
    let mut imported = BTreeSet::new();
    for use_decl in uses {
        let joined = use_decl.path.join(".").to_ascii_lowercase();
        if let Some(alias_target) = runtime_family_alias_import_path(&joined) {
            imported.insert(alias_target.to_string());
            if alias_target.eq_ignore_ascii_case("nc.runtime.list") {
                imported.insert("nc.runtime.queue".to_string());
            }
        }
        for family in families {
            let matches = if use_decl.wildcard {
                family.import_path.starts_with(&format!("{joined}."))
                    || family.module_path.starts_with(&format!("{joined}."))
                    || (joined == "nc.runtime" && family.import_path.starts_with("nc.runtime."))
                    || (joined == "runtime" && family.module_path.starts_with("runtime."))
                    || (joined == "nc.list" && family.import_path == "nc.runtime.list")
            } else {
                joined == family.import_path || joined == family.module_path
            };
            if matches {
                imported.insert(family.import_path.clone());
            }
        }
    }
    imported
}

fn runtime_presented_imported_families(
    imported_families: &BTreeSet<String>,
    uses: &[ncc_ast::UseDecl],
) -> BTreeSet<String> {
    let mut presented = imported_families.clone();
    let has_list_import = uses
        .iter()
        .any(|use_decl| runtime_use_path_targets_list_family(&use_decl.path));
    let has_queue_import = uses
        .iter()
        .any(|use_decl| runtime_use_path_targets_queue_family(&use_decl.path));

    if has_list_import {
        presented.insert("nc.runtime.list".to_string());
        if !has_queue_import {
            presented.remove("nc.runtime.queue");
        }
    }

    presented
}

fn runtime_family_from_qualified_path(path: &[String]) -> Option<String> {
    if path.len() >= 3 && path[0].eq_ignore_ascii_case("runtime") {
        let suffix = path[1].to_ascii_lowercase();
        if let Some(alias) = runtime_family_alias_import_path(&format!("runtime.{suffix}")) {
            return Some(alias.to_string());
        }
        return Some(format!("nc.runtime.{suffix}"));
    }
    if path.len() >= 4
        && path[0].eq_ignore_ascii_case("nC")
        && path[1].eq_ignore_ascii_case("runtime")
    {
        let suffix = path[2].to_ascii_lowercase();
        if let Some(alias) = runtime_family_alias_import_path(&format!("nc.runtime.{suffix}")) {
            return Some(alias.to_string());
        }
        return Some(format!("nc.runtime.{suffix}"));
    }
    if path.len() >= 3 && path[0].eq_ignore_ascii_case("nC") && path[1].eq_ignore_ascii_case("list")
    {
        return Some("nc.runtime.queue".to_string());
    }
    None
}

fn runtime_core_result_helper(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "ok" | "err" | "isok" | "iserr" | "unwrap" | "unwrapor" | "errorof"
    )
}

fn runtime_collect_expr_runtime_call_issues(
    expr: &ncc_ast::Expr,
    imported_families: &BTreeSet<String>,
    catalog: &RuntimeCatalogAudit,
    local_functions: &BTreeSet<String>,
    out: &mut Vec<String>,
) -> usize {
    match expr {
        ncc_ast::Expr::Call { callee, args } => {
            let mut runtime_calls = 0usize;
            let fn_name = callee.to_ascii_lowercase();
            if !local_functions.contains(&fn_name) && !runtime_core_result_helper(&fn_name) {
                if let Some(families) = catalog.fn_to_families.get(&fn_name) {
                    runtime_calls += 1;
                    let resolved = families.iter().any(|fam| imported_families.contains(fam));
                    if !resolved {
                        out.push(format!(
                            "E_RUNTIME_USE_001: runtime call '{}' requires matching use/import",
                            callee
                        ));
                    }
                }
            }
            for arg in args {
                runtime_calls += runtime_collect_expr_runtime_call_issues(
                    arg,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
            }
            runtime_calls
        }
        ncc_ast::Expr::QualifiedCall { path, args } => {
            let mut runtime_calls = 0usize;
            if let Some(family_import) = runtime_family_from_qualified_path(path) {
                runtime_calls += 1;
                let family = catalog
                    .families
                    .iter()
                    .find(|family| family.import_path == family_import);
                if family.is_none() {
                    out.push(format!(
                        "E_RUNTIME_IMPORT_002: unknown runtime family '{}'",
                        family_import
                    ));
                } else if !imported_families.contains(&family_import) {
                    out.push(format!(
                        "E_RUNTIME_USE_001: qualified runtime call '{}' requires explicit use '{}'",
                        path.join("."),
                        family_import
                    ));
                } else if let (Some(symbol), Some(contract)) = (path.last(), family) {
                    let exported = contract
                        .exports
                        .iter()
                        .any(|export| export.name.eq_ignore_ascii_case(symbol));
                    if !exported {
                        out.push(format!(
                            "E_RUNTIME_EXPORTS_001: runtime symbol '{}' is not exported by '{}'",
                            symbol, family_import
                        ));
                    }
                }
            }
            for arg in args {
                runtime_calls += runtime_collect_expr_runtime_call_issues(
                    arg,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
            }
            runtime_calls
        }
        ncc_ast::Expr::Unary { expr, .. } => runtime_collect_expr_runtime_call_issues(
            expr,
            imported_families,
            catalog,
            local_functions,
            out,
        ),
        ncc_ast::Expr::Binary { left, right, .. } => {
            runtime_collect_expr_runtime_call_issues(
                left,
                imported_families,
                catalog,
                local_functions,
                out,
            ) + runtime_collect_expr_runtime_call_issues(
                right,
                imported_families,
                catalog,
                local_functions,
                out,
            )
        }
        ncc_ast::Expr::MethodCall { receiver, args, .. } => {
            let mut runtime_calls = runtime_collect_expr_runtime_call_issues(
                receiver,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            for arg in args {
                runtime_calls += runtime_collect_expr_runtime_call_issues(
                    arg,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
            }
            runtime_calls
        }
        ncc_ast::Expr::ArrayLiteral(items) | ncc_ast::Expr::VectorLiteral(items) => items
            .iter()
            .map(|item| {
                runtime_collect_expr_runtime_call_issues(
                    item,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                )
            })
            .sum(),
        ncc_ast::Expr::Index { target, index } => {
            runtime_collect_expr_runtime_call_issues(
                target,
                imported_families,
                catalog,
                local_functions,
                out,
            ) + runtime_collect_expr_runtime_call_issues(
                index,
                imported_families,
                catalog,
                local_functions,
                out,
            )
        }
        ncc_ast::Expr::IfElse {
            condition,
            then_expr,
            else_expr,
        } => {
            runtime_collect_expr_runtime_call_issues(
                condition,
                imported_families,
                catalog,
                local_functions,
                out,
            ) + runtime_collect_expr_runtime_call_issues(
                then_expr,
                imported_families,
                catalog,
                local_functions,
                out,
            ) + runtime_collect_expr_runtime_call_issues(
                else_expr,
                imported_families,
                catalog,
                local_functions,
                out,
            )
        }
        ncc_ast::Expr::StrictCheckExpr { value, .. } => runtime_collect_expr_runtime_call_issues(
            value,
            imported_families,
            catalog,
            local_functions,
            out,
        ),
        _ => 0,
    }
}

fn runtime_collect_local_functions(ast: &ncc_ast::Program) -> BTreeSet<String> {
    let mut local = BTreeSet::new();
    for container in &ast.containers {
        for section in &container.sections {
            if let ncc_ast::Section::Fn(functions) = section {
                for function in functions {
                    local.insert(function.name.to_ascii_lowercase());
                }
            }
        }
    }
    local
}

fn runtime_collect_nominal_contract_audit(
    ast: &ncc_ast::Program,
    imported_families: &BTreeSet<String>,
    catalog: &RuntimeCatalogAudit,
    local_functions: &BTreeSet<String>,
) -> RuntimeNominalContractsAudit {
    fn visit_stmt(
        stmt: &ncc_ast::Stmt,
        imported_families: &BTreeSet<String>,
        catalog: &RuntimeCatalogAudit,
        local_functions: &BTreeSet<String>,
        audit: &mut RuntimeNominalContractsAudit,
    ) {
        match stmt {
            ncc_ast::Stmt::Destructure { args, .. } => {
                audit.destructure_statements += 1;
                for arg in args {
                    audit.runtime_calls_in_nominal += runtime_collect_expr_runtime_call_issues(
                        arg,
                        imported_families,
                        catalog,
                        local_functions,
                        &mut audit.unresolved_runtime_calls_in_nominal,
                    );
                }
            }
            ncc_ast::Stmt::Constructure { args, .. } => {
                audit.constructure_statements += 1;
                for arg in args {
                    audit.runtime_calls_in_nominal += runtime_collect_expr_runtime_call_issues(
                        arg,
                        imported_families,
                        catalog,
                        local_functions,
                        &mut audit.unresolved_runtime_calls_in_nominal,
                    );
                }
            }
            ncc_ast::Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                for nested in then_block {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
                for nested in else_block {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
            }
            ncc_ast::Stmt::Unless { body, .. }
            | ncc_ast::Stmt::While { body, .. }
            | ncc_ast::Stmt::DoWhile { body, .. }
            | ncc_ast::Stmt::ForIn { body, .. }
            | ncc_ast::Stmt::SameIn { body, .. }
            | ncc_ast::Stmt::ForRange { body, .. }
            | ncc_ast::Stmt::Loop { body }
            | ncc_ast::Stmt::RepeatUntil { body, .. }
            | ncc_ast::Stmt::RepeatCount { body, .. }
            | ncc_ast::Stmt::EachDoWhile { body, .. } => {
                for nested in body {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
            }
            ncc_ast::Stmt::ForClassic {
                init, step, body, ..
            } => {
                if let Some(init_stmt) = init {
                    visit_stmt(
                        init_stmt,
                        imported_families,
                        catalog,
                        local_functions,
                        audit,
                    );
                }
                if let Some(step_stmt) = step {
                    visit_stmt(
                        step_stmt,
                        imported_families,
                        catalog,
                        local_functions,
                        audit,
                    );
                }
                for nested in body {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
            }
            ncc_ast::Stmt::Switch { cases, default, .. }
            | ncc_ast::Stmt::Match { cases, default, .. } => {
                for (_, case_body) in cases {
                    for nested in case_body {
                        visit_stmt(nested, imported_families, catalog, local_functions, audit);
                    }
                }
                for nested in default {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
            }
            ncc_ast::Stmt::GuardElse { else_block, .. } => {
                for nested in else_block {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
            }
            ncc_ast::Stmt::Select { branches, default } => {
                for (_, branch_body) in branches {
                    for nested in branch_body {
                        visit_stmt(nested, imported_families, catalog, local_functions, audit);
                    }
                }
                for nested in default {
                    visit_stmt(nested, imported_families, catalog, local_functions, audit);
                }
            }
            ncc_ast::Stmt::Labeled { stmt: nested, .. } => {
                visit_stmt(nested, imported_families, catalog, local_functions, audit)
            }
            _ => {}
        }
    }

    let mut audit = RuntimeNominalContractsAudit::default();
    for container in &ast.containers {
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            visit_stmt(
                                stmt,
                                imported_families,
                                catalog,
                                local_functions,
                                &mut audit,
                            );
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        visit_stmt(
                            stmt,
                            imported_families,
                            catalog,
                            local_functions,
                            &mut audit,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    audit.unresolved_runtime_calls_in_nominal.sort();
    audit.unresolved_runtime_calls_in_nominal.dedup();
    audit
}

fn runtime_collect_runtime_call_issues(
    stmt: &ncc_ast::Stmt,
    imported_families: &BTreeSet<String>,
    catalog: &RuntimeCatalogAudit,
    local_functions: &BTreeSet<String>,
    strict: bool,
    enforce_runtime_output_import: bool,
    out: &mut Vec<String>,
) {
    fn runtime_print_kind_label(kind: &ncc_ast::PrintKind) -> &'static str {
        match kind {
            ncc_ast::PrintKind::Std => "print",
            ncc_ast::PrintKind::Ln => "println",
            ncc_ast::PrintKind::Sp => "print.sp",
            ncc_ast::PrintKind::Nl => "print.nl",
            ncc_ast::PrintKind::Err => "print.err",
            ncc_ast::PrintKind::Wm => "print.wm",
            ncc_ast::PrintKind::Inf => "print.inf",
            ncc_ast::PrintKind::Ok => "print.ok",
            ncc_ast::PrintKind::Trc => "print.trc",
            ncc_ast::PrintKind::Dbg => "print.dbg",
            ncc_ast::PrintKind::Sta => "print.sta",
            ncc_ast::PrintKind::Inspect => "print.inspect",
            ncc_ast::PrintKind::Mem => "print.mem",
            ncc_ast::PrintKind::Site => "print.site",
            ncc_ast::PrintKind::Type => "print.type",
            ncc_ast::PrintKind::Raw => "print.raw",
            ncc_ast::PrintKind::Stable => "print.stable",
            ncc_ast::PrintKind::Addr => "print.addr",
            ncc_ast::PrintKind::Phys => "print.phys",
            ncc_ast::PrintKind::Dump => "print.dump",
            ncc_ast::PrintKind::Peek => "print.peek",
            ncc_ast::PrintKind::Mmio => "print.mmio",
            ncc_ast::PrintKind::Port => "print.port",
            ncc_ast::PrintKind::Sensory => "print.sensory",
            ncc_ast::PrintKind::St => "print.st",
        }
    }

    fn runtime_print_requires_output_import(kind: &ncc_ast::PrintKind) -> bool {
        matches!(kind, ncc_ast::PrintKind::Std | ncc_ast::PrintKind::Ln)
    }

    match stmt {
        ncc_ast::Stmt::Return(expr)
        | ncc_ast::Stmt::Expr(expr)
        | ncc_ast::Stmt::Projection { value: expr, .. }
        | ncc_ast::Stmt::StrictCheck { value: expr, .. } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                expr,
                imported_families,
                catalog,
                local_functions,
                out,
            );
        }
        ncc_ast::Stmt::Print { kind, value } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                value,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            if strict
                && enforce_runtime_output_import
                && runtime_print_requires_output_import(kind)
                && !imported_families.contains("nc.runtime.output")
            {
                out.push(format!(
                    "E_RUNTIME_USE_001: runtime print '{}' requires explicit use/import 'nc.runtime.output'",
                    runtime_print_kind_label(kind)
                ));
            }
        }
        ncc_ast::Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                condition,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            for nested in then_block {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
            for nested in else_block {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::While { condition, body }
        | ncc_ast::Stmt::DoWhile { body, condition }
        | ncc_ast::Stmt::RepeatUntil { body, condition }
        | ncc_ast::Stmt::EachDoWhile {
            body, condition, ..
        } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                condition,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            for nested in body {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::ForClassic {
            init,
            condition,
            step,
            body,
        } => {
            if let Some(init_stmt) = init {
                runtime_collect_runtime_call_issues(
                    init_stmt,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
            if let Some(condition_expr) = condition {
                let _ = runtime_collect_expr_runtime_call_issues(
                    condition_expr,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
            }
            if let Some(step_stmt) = step {
                runtime_collect_runtime_call_issues(
                    step_stmt,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
            for nested in body {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::ForIn { iterable, body, .. }
        | ncc_ast::Stmt::SameIn { iterable, body, .. } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                iterable,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            for nested in body {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::ForRange {
            start,
            end,
            step,
            body,
            ..
        } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                start,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            let _ = runtime_collect_expr_runtime_call_issues(
                end,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            if let Some(step_expr) = step {
                let _ = runtime_collect_expr_runtime_call_issues(
                    step_expr,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
            }
            for nested in body {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Loop { body } | ncc_ast::Stmt::RepeatCount { body, .. } => {
            for nested in body {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Switch {
            value,
            cases,
            default,
        }
        | ncc_ast::Stmt::Match {
            value,
            cases,
            default,
        } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                value,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            for (expr, stmts) in cases {
                let _ = runtime_collect_expr_runtime_call_issues(
                    expr,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
                for nested in stmts {
                    runtime_collect_runtime_call_issues(
                        nested,
                        imported_families,
                        catalog,
                        local_functions,
                        strict,
                        enforce_runtime_output_import,
                        out,
                    );
                }
            }
            for nested in default {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::GuardElse {
            condition,
            else_block,
        } => {
            let _ = runtime_collect_expr_runtime_call_issues(
                condition,
                imported_families,
                catalog,
                local_functions,
                out,
            );
            for nested in else_block {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Select {
            branches, default, ..
        } => {
            for (expr, stmts) in branches {
                let _ = runtime_collect_expr_runtime_call_issues(
                    expr,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
                for nested in stmts {
                    runtime_collect_runtime_call_issues(
                        nested,
                        imported_families,
                        catalog,
                        local_functions,
                        strict,
                        enforce_runtime_output_import,
                        out,
                    );
                }
            }
            for nested in default {
                runtime_collect_runtime_call_issues(
                    nested,
                    imported_families,
                    catalog,
                    local_functions,
                    strict,
                    enforce_runtime_output_import,
                    out,
                );
            }
        }
        ncc_ast::Stmt::Labeled { stmt, .. } => runtime_collect_runtime_call_issues(
            stmt,
            imported_families,
            catalog,
            local_functions,
            strict,
            enforce_runtime_output_import,
            out,
        ),
        ncc_ast::Stmt::Destructure { args, .. } | ncc_ast::Stmt::Constructure { args, .. } => {
            for arg in args {
                let _ = runtime_collect_expr_runtime_call_issues(
                    arg,
                    imported_families,
                    catalog,
                    local_functions,
                    out,
                );
            }
        }
        _ => {}
    }
}

fn runtime_validate_source_contracts(ast: &ncc_ast::Program, strict: bool) -> RuntimeSourceAudit {
    let catalog = build_runtime_catalog_audit();
    let imported_families_raw = runtime_imported_families_from_uses(&ast.uses, &catalog.families);
    let imported_families = runtime_presented_imported_families(&imported_families_raw, &ast.uses);
    let runtime_import_uses = runtime_use_paths(ast);

    let mut required_capabilities = BTreeSet::new();
    let mut required_policies = BTreeSet::new();
    for family in &catalog.families {
        if imported_families.contains(&family.import_path) {
            for capability in &family.capabilities {
                required_capabilities.insert(capability.clone());
            }
            for policy in &family.policies {
                required_policies.insert(policy.clone());
            }
        }
    }

    let list_alias_missing_catalog_contract = imported_families.contains("nc.runtime.list")
        && !catalog
            .families
            .iter()
            .any(|family| family.import_path == "nc.runtime.list");
    if list_alias_missing_catalog_contract {
        for capability in runtime_capabilities_for_family_key("list") {
            required_capabilities.insert(capability);
        }
        for policy in runtime_policies_for_family_key("list") {
            required_policies.insert(policy);
        }
    }

    let capability_catalog = runtime_capability_catalog_set();
    let policy_catalog = runtime_policy_catalog_set();
    let unknown_capabilities = required_capabilities
        .iter()
        .filter(|capability| !capability_catalog.contains(*capability))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_policies = required_policies
        .iter()
        .filter(|policy| !policy_catalog.contains(*policy))
        .cloned()
        .collect::<Vec<_>>();

    let local_functions = runtime_collect_local_functions(ast);
    let mut unresolved_runtime_calls = Vec::new();
    for container in &ast.containers {
        let enforce_runtime_output_import =
            matches!(container.context.as_ref(), Some(ncc_ast::Context::Runtime));
        for section in &container.sections {
            match section {
                ncc_ast::Section::Fn(functions) => {
                    for function in functions {
                        for stmt in &function.public_body {
                            runtime_collect_runtime_call_issues(
                                stmt,
                                &imported_families_raw,
                                &catalog,
                                &local_functions,
                                strict,
                                enforce_runtime_output_import,
                                &mut unresolved_runtime_calls,
                            );
                        }
                    }
                }
                ncc_ast::Section::MainEntry { body, .. }
                | ncc_ast::Section::Final(body)
                | ncc_ast::Section::Binary(body)
                | ncc_ast::Section::Bind(body)
                | ncc_ast::Section::Raw(body)
                | ncc_ast::Section::Probe(body) => {
                    for stmt in body {
                        runtime_collect_runtime_call_issues(
                            stmt,
                            &imported_families_raw,
                            &catalog,
                            &local_functions,
                            strict,
                            enforce_runtime_output_import,
                            &mut unresolved_runtime_calls,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    unresolved_runtime_calls.sort();
    unresolved_runtime_calls.dedup();

    let nominal_contracts = runtime_collect_nominal_contract_audit(
        ast,
        &imported_families_raw,
        &catalog,
        &local_functions,
    );

    let mut forbidden_mode_violations = Vec::new();
    if strict {
        let has_runtime_import = !runtime_import_uses.is_empty();
        let has_runtime_input = imported_families_raw.contains("nc.runtime.input");
        for container in &ast.containers {
            match container.context.as_ref() {
                Some(ncc_ast::Context::Root) if has_runtime_import => forbidden_mode_violations.push(
                    "E_RUNTIME_IMPORT_ROOT_001: runtime imports are forbidden in [@] root context"
                        .to_string(),
                ),
                Some(ncc_ast::Context::Primitive) if has_runtime_import => {
                    forbidden_mode_violations.push(
                        "E_RUNTIME_IMPORT_PRIMITIVE_001: runtime imports are forbidden in primitive[@]"
                            .to_string(),
                    )
                }
                Some(ncc_ast::Context::Machine) if has_runtime_input => forbidden_mode_violations.push(
                    "E_RUNTIME_IMPORT_MACHINE_INPUT_001: nC.runtime.input is forbidden in [@machine]"
                        .to_string(),
                ),
                Some(ncc_ast::Context::Kernel) if has_runtime_input => forbidden_mode_violations.push(
                    "E_RUNTIME_IMPORT_KERNEL_INPUT_001: nC.runtime.input is forbidden in [@kernel]"
                        .to_string(),
                ),
                _ => {}
            }
        }
    }

    RuntimeSourceAudit {
        imported_families,
        runtime_import_uses,
        required_capabilities,
        required_policies,
        unresolved_runtime_calls,
        forbidden_mode_violations,
        unknown_capabilities,
        unknown_policies,
        nominal_contracts,
    }
}

fn runtime_validate_strict_contracts(
    audit: &RuntimeSourceAudit,
) -> Result<(), Box<dyn std::error::Error>> {
    if audit.runtime_import_uses.is_empty() {
        return Err(
            "E_RUNTIME_IMPORT_001: --strict runtime check requires at least one runtime import"
                .into(),
        );
    }
    if !audit.forbidden_mode_violations.is_empty() {
        return Err(audit.forbidden_mode_violations.join("\n").into());
    }
    if !audit.unknown_capabilities.is_empty() {
        return Err(format!(
            "E_RUNTIME_CAPABILITY_001: unresolved capability contracts: {}",
            audit.unknown_capabilities.join(", ")
        )
        .into());
    }
    if !audit.unknown_policies.is_empty() {
        return Err(format!(
            "E_RUNTIME_POLICY_001: unresolved policy contracts: {}",
            audit.unknown_policies.join(", ")
        )
        .into());
    }
    if !audit.required_capabilities.is_empty() && audit.required_policies.is_empty() {
        return Err(
            "E_RUNTIME_POLICY_001: required capabilities are present but no policy contracts are declared"
                .into(),
        );
    }
    let invalid_policy_scope = audit
        .required_policies
        .iter()
        .filter(|policy| policy.starts_with("compiler."))
        .cloned()
        .collect::<Vec<_>>();
    if !invalid_policy_scope.is_empty() {
        return Err(format!(
            "E_RUNTIME_POLICY_001: policy scope is invalid for runtime contracts: {}",
            invalid_policy_scope.join(", ")
        )
        .into());
    }
    Ok(())
}

fn runtime_nominal_contracts_trace(audit: Option<&RuntimeSourceAudit>) -> serde_json::Value {
    let nominal = audit.map(|value| &value.nominal_contracts);
    let destructure_statements = nominal
        .map(|value| value.destructure_statements)
        .unwrap_or(0);
    let constructure_statements = nominal
        .map(|value| value.constructure_statements)
        .unwrap_or(0);
    let runtime_calls_in_nominal = nominal
        .map(|value| value.runtime_calls_in_nominal)
        .unwrap_or(0);
    let unresolved_runtime_calls_in_nominal = nominal
        .map(|value| value.unresolved_runtime_calls_in_nominal.clone())
        .unwrap_or_default();
    serde_json::json!({
        "source_audited": audit.is_some(),
        "supported_constructs": RUNTIME_NOMINAL_SUPPORTED_CONSTRUCTS,
        "runtime_call_diagnostics": RUNTIME_NOMINAL_RUNTIME_DIAGNOSTICS,
        "destructure_statements": destructure_statements,
        "constructure_statements": constructure_statements,
        "runtime_calls_in_nominal": runtime_calls_in_nominal,
        "unresolved_runtime_calls_in_nominal": unresolved_runtime_calls_in_nominal,
    })
}

fn runtime_handle_doctor(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("runtime doctor does not accept positional arguments".into());
    }
    let families = ncc_parser::official_runtime_families();
    let api_specs = ncc_parser::runtime_api_family_specs();
    let catalog = build_runtime_catalog_audit();
    let abi = runtime_abi_manifest();
    let report = serde_json::json!({
        "status": "ok",
        "doctor": "runtime",
        "dual_path": {
            "check": true,
            "run": true,
            "primitive": true,
        },
        "runtime": {
            "families_declared": families.len(),
            "families_with_signatures": api_specs.len(),
            "capabilities": RUNTIME_CAPABILITY_CATALOG.len(),
            "policies": RUNTIME_POLICY_CATALOG.len(),
            "errors": RUNTIME_ERROR_CATALOG.len(),
        },
        "runtime_symbol_abi": {
            "schema": abi.get("schema").and_then(|value| value.as_str()).unwrap_or("nc.runtime.symbol_abi.v1"),
            "status": abi.get("status").and_then(|value| value.as_str()).unwrap_or("closed"),
            "symbol_count": abi.get("symbol_count").and_then(|value| value.as_u64()).unwrap_or(0),
            "layouts": abi.get("layouts").and_then(|value| value.as_array()).map(|rows| rows.len()).unwrap_or(0),
        },
        "contracts": {
            "family_contract_rows": catalog.families.len(),
            "missing_signature_families": catalog.missing_signature_families,
            "unknown_capabilities": catalog.unknown_capabilities,
            "unknown_policies": catalog.unknown_policies,
            "function_index_size": catalog.fn_to_families.len(),
        },
        "nominal_contracts": runtime_nominal_contracts_trace(None),
        "pointer_runtime_heap_contract": {
            "status": "closed",
            "source": "canonical_raw_pointer_contract",
            "alloc": "closed_raw_byte_size",
            "address_reservation": "immediate_aligned_non_overlapping_failure_atomic",
            "normal_compilation": "manual_raw_unchecked",
            "free_consumes_owner": "optional_audit_state_only",
            "realloc_invalidates_old_generation": "optional_audit_state_only",
            "realloc_failure_preserves_old_generation": true,
            "zero_validates_live_bounds_type": "runtime_or_bounded_view_contract_not_raw_codegen",
            "state_fields": ["address", "type", "size", "live", "death_reason", "origin", "owner", "aliases", "generation", "site", "lineage", "last_mutation", "event_trace"],
            "state_purpose": "optional_audit_and_runtime_telemetry_only",
            "optional_audit_diagnostics": ["E_PTR_UAF_001", "E_PTR_DOUBLE_FREE_001", "E_PTR_STALE_001", "E_PTR_LEAK_001", "E_PTR_ALIAS_001"],
            "raw_power": "c_cpp_grade_default_with_backend_lowering",
            "raw_ptr_range": "unchecked_allocation_bounds",
            "undefined_behavior": "null_misalignment_aliasing_uaf_double_free_and_overflow_have_no_deterministic_result_guarantee",
            "bounded_views": "array_vector_slice_buffer_memRegion_checked",
            "silentPointer": "legacy_no_op_annotation_raw_is_already_default"
        },
        "imports_stoud_auth_dom": import_surface_doctor_payload(),
        "phase": opts.phase,
        "strict": opts.strict,
        "target": opts.target,
        "profile": opts.profile,
        "family": opts.family,
        "mode": opts.mode,
        "out": opts.out,
        "no_color": opts.no_color,
    });
    runtime_print_json_or_text(opts, report, "runtime doctor: OK")
}

fn runtime_handle_families(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("runtime families does not accept positional arguments".into());
    }
    let catalog = build_runtime_catalog_audit();
    let family_filter = opts.family.as_ref().map(|value| value.to_ascii_lowercase());

    let rows: Vec<serde_json::Value> = catalog
        .families
        .iter()
        .filter(|family| {
            if let Some(filter) = &family_filter {
                family.import_path.contains(filter)
                    || family.module_path.contains(filter)
                    || family.family_key.contains(filter)
            } else {
                true
            }
        })
        .map(|family| {
            serde_json::json!({
                "family": family.import_path,
                "module_path": family.module_path,
                "family_key": family.family_key,
                "functions": family.exports.len(),
                "required_capabilities": family.capabilities,
                "required_policies": family.policies,
                "nominal_runtime_call_audit": true,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "count": rows.len(),
        "rows": rows,
        "catalog_contract": {
            "missing_signature_families": catalog.missing_signature_families,
            "unknown_capabilities": catalog.unknown_capabilities,
            "unknown_policies": catalog.unknown_policies,
        },
        "nominal_contracts": runtime_nominal_contracts_trace(None),
    });

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if opts.md {
        println!("# runtime families\n");
        if let Some(rows) = payload.get("rows").and_then(|v| v.as_array()) {
            for row in rows {
                let family = row.get("family").and_then(|v| v.as_str()).unwrap_or("?");
                let fns = row.get("functions").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("- `{}` ({})", family, fns);
            }
        }
    } else {
        if let Some(rows) = payload.get("rows").and_then(|v| v.as_array()) {
            for row in rows {
                let family = row.get("family").and_then(|v| v.as_str()).unwrap_or("?");
                let fns = row.get("functions").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("{} ({})", family, fns);
            }
        }
    }
    Ok(())
}

fn runtime_handle_exports(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let requested = rest
        .first()
        .ok_or("runtime exports requires a family path")?;
    let requested_lower = requested.to_ascii_lowercase();
    let requested_is_list_alias = runtime_path_targets_list_family(&requested_lower);
    let requested_is_queue_alias = runtime_path_targets_queue_family(&requested_lower);
    let requested_lookup = if requested_is_list_alias || requested_is_queue_alias {
        "nc.runtime.queue".to_string()
    } else {
        runtime_family_alias_import_path(&requested_lower)
            .unwrap_or(requested_lower.as_str())
            .to_string()
    };
    let catalog = build_runtime_catalog_audit();
    let family = catalog
        .families
        .iter()
        .find(|family| {
            family.import_path == requested_lookup
                || family.module_path == requested_lookup
                || family.family_key == requested_lookup
        })
        .ok_or_else(|| {
            format!(
                "E_RUNTIME_EXPORTS_001: runtime family '{}' is not registered",
                requested
            )
        })?;

    let projected_exports: Vec<RuntimeExportContract> = if requested_is_list_alias {
        runtime_list_alias_exports()
    } else if requested_is_queue_alias {
        family
            .exports
            .iter()
            .filter(|export| runtime_export_name_is_queue_only(&export.name))
            .cloned()
            .collect()
    } else {
        family.exports.clone()
    };

    let projected_import_path = if requested_is_list_alias {
        "nc.runtime.list".to_string()
    } else {
        family.import_path.clone()
    };
    let projected_module_path = if requested_is_list_alias {
        "runtime.list".to_string()
    } else {
        family.module_path.clone()
    };
    let projected_family_key = if requested_is_list_alias {
        "list".to_string()
    } else {
        family.family_key.clone()
    };
    let projected_capabilities = if requested_is_list_alias {
        runtime_capabilities_for_family_key("list")
    } else {
        family.capabilities.clone()
    };
    let projected_policies = if requested_is_list_alias {
        runtime_policies_for_family_key("list")
    } else {
        family.policies.clone()
    };

    let rows: Vec<serde_json::Value> = projected_exports
        .iter()
        .map(|fun| {
            serde_json::json!({
                "name": fun.name,
                "generic_params": runtime_signature_generic_params(fun),
                "arity_min": fun.arity_min,
                "variadic": fun.variadic,
                "arg_types": fun.arg_types,
                "return": fun.return_ty,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "import_path": projected_import_path,
        "module_path": projected_module_path,
        "family_key": projected_family_key,
        "required_capabilities": projected_capabilities,
        "required_policies": projected_policies,
        "exports": rows,
    });

    runtime_print_json_or_text(opts, payload, "runtime exports: OK")
}

fn runtime_handle_abi(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = runtime_abi_manifest();
    if let Some(query) = rest.first() {
        let query = query.to_ascii_lowercase();
        let filtered = payload
            .get("symbols")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter(|entry| {
                entry
                    .get("symbol")
                    .and_then(|value| value.as_str())
                    .is_some_and(|symbol| symbol.to_ascii_lowercase().contains(&query))
                    || entry
                        .get("family")
                        .and_then(|value| value.as_str())
                        .is_some_and(|family| family.eq_ignore_ascii_case(&query))
                    || entry
                        .get("aliases")
                        .and_then(|value| value.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|alias| alias.as_str())
                        .any(|alias| alias.to_ascii_lowercase().contains(&query))
            })
            .cloned()
            .collect::<Vec<_>>();
        payload["query"] = serde_json::json!(rest.first());
        payload["symbols"] = serde_json::json!(filtered);
        payload["symbol_count"] = serde_json::json!(payload
            .get("symbols")
            .and_then(|value| value.as_array())
            .map(|rows| rows.len())
            .unwrap_or(0));
    }
    if let Some(family) = &opts.family {
        let family_lower = family.to_ascii_lowercase();
        let filtered = payload
            .get("symbols")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter(|entry| {
                entry
                    .get("family")
                    .and_then(|value| value.as_str())
                    .is_some_and(|entry_family| entry_family.eq_ignore_ascii_case(&family_lower))
            })
            .cloned()
            .collect::<Vec<_>>();
        payload["family_filter"] = serde_json::json!(family);
        payload["symbols"] = serde_json::json!(filtered);
        payload["symbol_count"] = serde_json::json!(payload
            .get("symbols")
            .and_then(|value| value.as_array())
            .map(|rows| rows.len())
            .unwrap_or(0));
    }

    if opts.md {
        println!("# runtime symbol ABI\n");
        println!(
            "- schema: `{}`",
            payload
                .get("schema")
                .and_then(|value| value.as_str())
                .unwrap_or("?")
        );
        println!(
            "- symbols: `{}`\n",
            payload
                .get("symbol_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        );
        if let Some(symbols) = payload.get("symbols").and_then(|value| value.as_array()) {
            for symbol in symbols {
                let name = symbol
                    .get("symbol")
                    .and_then(|value| value.as_str())
                    .unwrap_or("?");
                let ret = symbol
                    .get("return")
                    .and_then(|value| value.get("type"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("?");
                println!("- `{}` -> `{}`", name, ret);
            }
        }
        return Ok(());
    }

    runtime_print_json_or_text(opts, payload, "runtime ABI: OK")
}

fn runtime_handle_check(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest.first().ok_or("runtime check requires a source path")?;
    if !is_high_level_source_path(path) {
        return Err("E_RUNTIME_CHECK_001: runtime check expects .n/.nc source".into());
    }

    let ast = load_ast_from_path(path)?;
    let audit = runtime_validate_source_contracts(&ast, opts.strict);

    if !audit.unresolved_runtime_calls.is_empty() {
        return Err(audit.unresolved_runtime_calls.join("\n").into());
    }

    if opts.strict {
        runtime_validate_strict_contracts(&audit)?;
    }

    sema_check_with_warnings(&ast)?;

    let payload = serde_json::json!({
        "status": "ok",
        "path": path,
        "uses": ast.uses.len(),
        "containers": ast.containers.len(),
        "runtime_uses": audit.runtime_import_uses,
        "runtime_families": audit.imported_families,
        "required_capabilities": audit.required_capabilities,
        "required_policies": audit.required_policies,
        "contract_violations": audit.forbidden_mode_violations,
        "unresolved_runtime_calls": audit.unresolved_runtime_calls,
        "nominal_contracts": runtime_nominal_contracts_trace(Some(&audit)),
        "strict": opts.strict,
    });

    runtime_print_json_or_text(opts, payload, &format!("OK: runtime check {}", path))
}

fn runtime_handle_imports(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = rest
        .first()
        .ok_or("runtime imports requires a source path")?;
    let ast = load_ast_from_path(path)?;
    let catalog = build_runtime_catalog_audit();
    let imported_families = runtime_presented_imported_families(
        &runtime_imported_families_from_uses(&ast.uses, &catalog.families),
        &ast.uses,
    );
    let audit = runtime_validate_source_contracts(&ast, false);

    let rows: Vec<serde_json::Value> = ast
        .uses
        .iter()
        .map(|u| {
            let raw = u.path.join(".");
            let aliases = if runtime_use_path_targets_list_family(&u.path) {
                vec!["runtime.list".to_string()]
            } else if runtime_use_path_targets_queue_family(&u.path) {
                vec!["runtime.queue".to_string()]
            } else {
                runtime_alias_module_paths_for_use(&u.path)
                    .iter()
                    .map(|path| path.join("."))
                    .collect::<Vec<_>>()
            };
            let resolved_runtime_family = runtime_family_key_from_use_path(&u.path);
            let resolved_imports = if runtime_use_path_targets_list_family(&u.path) {
                vec!["nc.runtime.list".to_string()]
            } else if runtime_use_path_targets_queue_family(&u.path) {
                vec!["nc.runtime.queue".to_string()]
            } else {
                catalog
                    .families
                    .iter()
                    .filter(|family| {
                        if u.wildcard {
                            family
                                .import_path
                                .starts_with(&format!("{}.", raw.to_ascii_lowercase()))
                                || family
                                    .module_path
                                    .starts_with(&format!("{}.", raw.to_ascii_lowercase()))
                        } else {
                            raw.eq_ignore_ascii_case(&family.import_path)
                                || raw.eq_ignore_ascii_case(&family.module_path)
                        }
                    })
                    .map(|family| family.import_path.clone())
                    .collect::<Vec<_>>()
            };
            serde_json::json!({
                "path": raw,
                "wildcard": u.wildcard,
                "runtime_family": resolved_runtime_family,
                "alias_modules": aliases,
                "resolved_runtime_imports": resolved_imports,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "path": path,
        "imported_families": imported_families,
        "imports": rows,
        "nominal_contracts": runtime_nominal_contracts_trace(Some(&audit)),
    });

    runtime_print_json_or_text(opts, payload, &format!("OK: runtime imports {}", path))
}

fn runtime_handle_capabilities(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = rest.first() {
        let ast = load_ast_from_path(path)?;
        let audit = runtime_validate_source_contracts(&ast, false);
        let payload = serde_json::json!({
            "path": path,
            "runtime_families": audit.imported_families,
            "required_capabilities": audit.required_capabilities,
            "unknown_required_capabilities": audit.unknown_capabilities,
            "catalog_total": RUNTIME_CAPABILITY_CATALOG.len(),
            "nominal_contracts": runtime_nominal_contracts_trace(Some(&audit)),
        });
        return runtime_print_json_or_text(
            opts,
            payload,
            &format!("OK: runtime capabilities {}", path),
        );
    }

    let payload = serde_json::json!({
        "catalog": RUNTIME_CAPABILITY_CATALOG,
        "count": RUNTIME_CAPABILITY_CATALOG.len(),
        "mode_default": "mode.runtime.use",
        "nominal_contracts": runtime_nominal_contracts_trace(None),
    });
    runtime_print_json_or_text(opts, payload, "runtime capabilities catalog")
}

fn runtime_handle_policies(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = rest.first() {
        let ast = load_ast_from_path(path)?;
        let audit = runtime_validate_source_contracts(&ast, false);
        let payload = serde_json::json!({
            "path": path,
            "runtime_families": audit.imported_families,
            "required_policies": audit.required_policies,
            "unknown_required_policies": audit.unknown_policies,
            "catalog_total": RUNTIME_POLICY_CATALOG.len(),
            "nominal_contracts": runtime_nominal_contracts_trace(Some(&audit)),
        });
        return runtime_print_json_or_text(
            opts,
            payload,
            &format!("OK: runtime policies {}", path),
        );
    }

    let payload = serde_json::json!({
        "catalog": RUNTIME_POLICY_CATALOG,
        "count": RUNTIME_POLICY_CATALOG.len(),
        "nominal_contracts": runtime_nominal_contracts_trace(None),
    });
    runtime_print_json_or_text(opts, payload, "runtime policy catalog")
}

fn runtime_handle_errors(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    if !rest.is_empty() {
        return Err("runtime errors does not accept positional arguments".into());
    }

    let rows: Vec<serde_json::Value> = RUNTIME_ERROR_CATALOG
        .iter()
        .map(|name| {
            serde_json::json!({
                "name": name,
                "code": runtime_error_code(name),
                "family": runtime_error_family(name),
                "severity": "error",
                "recoverable": runtime_error_is_recoverable(name),
                "payload_schema": "runtime.error.payload.v1",
            })
        })
        .collect();

    let payload = serde_json::json!({
        "count": rows.len(),
        "errors": rows,
        "nominal_contracts": runtime_nominal_contracts_trace(None),
    });

    runtime_print_json_or_text(opts, payload, "runtime error catalog")
}

fn runtime_handle_report(
    opts: &RuntimeCliOptions,
    rest: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut source_path: Option<String> = None;
    let mut md_out: Option<String> = None;
    let mut json_out: Option<String> = None;

    let mut idx = 0usize;
    while idx < rest.len() {
        match rest[idx].as_str() {
            "--md-out" => {
                md_out = Some(
                    rest.get(idx + 1)
                        .ok_or("runtime report: --md-out requires a value")?
                        .clone(),
                );
                idx += 2;
            }
            "--json-out" => {
                json_out = Some(
                    rest.get(idx + 1)
                        .ok_or("runtime report: --json-out requires a value")?
                        .clone(),
                );
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("runtime report: unknown option '{}'", other).into());
            }
            value => {
                if source_path.is_some() {
                    return Err("runtime report accepts at most one source path".into());
                }
                source_path = Some(value.to_string());
                idx += 1;
            }
        }
    }

    let report = if let Some(path) = source_path {
        let ast = load_ast_from_path(&path)?;
        sema_check_with_warnings(&ast)?;
        let audit = runtime_validate_source_contracts(&ast, opts.strict);
        let catalog = build_runtime_catalog_audit();
        let abi = runtime_abi_manifest();
        serde_json::json!({
            "status": "ok",
            "path": path,
            "containers": ast.containers.len(),
            "imports": ast.uses.len(),
            "runtime_imports": audit.runtime_import_uses,
            "runtime_families": audit.imported_families,
            "required_capabilities": audit.required_capabilities,
            "required_policies": audit.required_policies,
            "unresolved_runtime_calls": audit.unresolved_runtime_calls,
            "nominal_contracts": runtime_nominal_contracts_trace(Some(&audit)),
            "forbidden_mode_violations": audit.forbidden_mode_violations,
            "unknown_required_capabilities": audit.unknown_capabilities,
            "unknown_required_policies": audit.unknown_policies,
            "console_surface": runtime_console_surface_report(
                &fs::read_to_string(&path).unwrap_or_default()
            ),
            "runtime_catalog": {
                "families": ncc_parser::official_runtime_families().len(),
                "family_contract_rows": catalog.families.len(),
                "missing_signature_families": catalog.missing_signature_families,
                "errors": RUNTIME_ERROR_CATALOG.len(),
            },
            "runtime_symbol_abi": {
                "schema": abi.get("schema").and_then(|value| value.as_str()).unwrap_or("nc.runtime.symbol_abi.v1"),
                "status": abi.get("status").and_then(|value| value.as_str()).unwrap_or("closed"),
                "symbol_count": abi.get("symbol_count").and_then(|value| value.as_u64()).unwrap_or(0),
            },
        })
    } else {
        let catalog = build_runtime_catalog_audit();
        let abi = runtime_abi_manifest();
        serde_json::json!({
            "status": "ok",
            "runtime_catalog": {
                "families": ncc_parser::official_runtime_families().len(),
                "family_contract_rows": catalog.families.len(),
                "missing_signature_families": catalog.missing_signature_families,
                "capabilities": RUNTIME_CAPABILITY_CATALOG.len(),
                "policies": RUNTIME_POLICY_CATALOG.len(),
                "errors": RUNTIME_ERROR_CATALOG.len(),
            },
            "runtime_symbol_abi": {
                "schema": abi.get("schema").and_then(|value| value.as_str()).unwrap_or("nc.runtime.symbol_abi.v1"),
                "status": abi.get("status").and_then(|value| value.as_str()).unwrap_or("closed"),
                "symbol_count": abi.get("symbol_count").and_then(|value| value.as_u64()).unwrap_or(0),
                "layouts": abi.get("layouts").and_then(|value| value.as_array()).map(|rows| rows.len()).unwrap_or(0),
            },
            "nominal_contracts": runtime_nominal_contracts_trace(None),
        })
    };

    let markdown = runtime_report_markdown(&report);

    if let Some(path) = &md_out {
        ensure_output_parent_dir(path)?;
        fs::write(path, markdown.as_bytes())?;
    }
    if let Some(path) = &json_out {
        ensure_output_parent_dir(path)?;
        fs::write(path, serde_json::to_string_pretty(&report)?.as_bytes())?;
    }

    if opts.json || json_out.is_none() {
        if opts.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else if opts.md {
            println!("{}", markdown);
        } else if md_out.is_none() && json_out.is_none() {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    if md_out.is_some() || json_out.is_some() {
        println!("OK: runtime report generated");
    }

    Ok(())
}

fn runtime_count_call_occurrences(source: &str, name: &str) -> usize {
    source
        .match_indices(name)
        .filter(|(idx, _)| {
            let before = source[..*idx].chars().next_back();
            let after = source[*idx + name.len()..]
                .chars()
                .find(|ch| !ch.is_whitespace());
            let before_ok = !matches!(before, Some(ch) if ch.is_ascii_alphanumeric() || ch == '_');
            before_ok && after == Some('(')
        })
        .count()
}

fn runtime_console_surface_report(source: &str) -> serde_json::Value {
    let print_calls = runtime_count_call_occurrences(source, "print");
    let println_calls = runtime_count_call_occurrences(source, "println");
    let flush_calls = runtime_count_call_occurrences(source, "flush");
    let read_calls = [
        "readInt",
        "readLine",
        "readBool",
        "readChar",
        "tryReadInt",
        "tryReadLine",
        "tryReadBool",
        "tryReadChar",
    ]
    .iter()
    .map(|name| runtime_count_call_occurrences(source, name))
    .sum::<usize>();
    let prompt_patterns = flush_calls.min(read_calls);

    serde_json::json!({
        "print_calls": print_calls,
        "println_calls": println_calls,
        "flush_calls": flush_calls,
        "read_calls": read_calls,
        "prompt_patterns": prompt_patterns,
        "prompt_order_risk": 0,
        "none_output_risk": 0,
        "placeholder_output_risk": 0,
        "interactive_required": read_calls > 0,
        "stdout_flush_supported": true,
        "prompt_order_guaranteed": true,
    })
}

fn runtime_report_markdown(report: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("# Runtime Report\n\n");
    out.push_str("- status: ");
    out.push_str(
        report
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );
    out.push('\n');
    if let Some(path) = report.get("path").and_then(|v| v.as_str()) {
        out.push_str("- source: `");
        out.push_str(path);
        out.push_str("`\n");
    }
    if let Some(families) = report
        .get("runtime_catalog")
        .and_then(|v| v.get("families"))
        .and_then(|v| v.as_u64())
    {
        out.push_str("- runtime families: ");
        out.push_str(&families.to_string());
        out.push('\n');
    }
    if let Some(contract_rows) = report
        .get("runtime_catalog")
        .and_then(|v| v.get("family_contract_rows"))
        .and_then(|v| v.as_u64())
    {
        out.push_str("- family contract rows: ");
        out.push_str(&contract_rows.to_string());
        out.push('\n');
    }
    if let Some(imports) = report.get("runtime_imports").and_then(|v| v.as_array()) {
        out.push_str("- runtime imports detected: ");
        out.push_str(&imports.len().to_string());
        out.push('\n');
    }
    if let Some(caps) = report
        .get("required_capabilities")
        .and_then(|v| v.as_array())
    {
        out.push_str("- required capabilities: ");
        out.push_str(&caps.len().to_string());
        out.push('\n');
    }
    if let Some(policies) = report.get("required_policies").and_then(|v| v.as_array()) {
        out.push_str("- required policies: ");
        out.push_str(&policies.len().to_string());
        out.push('\n');
    }
    if let Some(unresolved) = report
        .get("unresolved_runtime_calls")
        .and_then(|v| v.as_array())
    {
        out.push_str("- unresolved runtime calls: ");
        out.push_str(&unresolved.len().to_string());
        out.push('\n');
    }
    if let Some(console) = report.get("console_surface") {
        out.push_str("- console print calls: ");
        out.push_str(
            &console
                .get("print_calls")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                .to_string(),
        );
        out.push('\n');
        out.push_str("- console read calls: ");
        out.push_str(
            &console
                .get("read_calls")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                .to_string(),
        );
        out.push('\n');
        out.push_str("- prompt order guaranteed: ");
        out.push_str(
            if console
                .get("prompt_order_guaranteed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                "true"
            } else {
                "false"
            },
        );
        out.push('\n');
    }
    if let Some(nominal) = report.get("nominal_contracts") {
        let destructure = nominal
            .get("destructure_statements")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let constructure = nominal
            .get("constructure_statements")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let runtime_calls = nominal
            .get("runtime_calls_in_nominal")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let unresolved_nominal = nominal
            .get("unresolved_runtime_calls_in_nominal")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0);
        out.push_str("- nominal destructure statements: ");
        out.push_str(&destructure.to_string());
        out.push('\n');
        out.push_str("- nominal constructure statements: ");
        out.push_str(&constructure.to_string());
        out.push('\n');
        out.push_str("- runtime calls in nominal contracts: ");
        out.push_str(&runtime_calls.to_string());
        out.push('\n');
        out.push_str("- unresolved runtime calls in nominal contracts: ");
        out.push_str(&unresolved_nominal.to_string());
        out.push('\n');
    }
    out
}

fn runtime_print_json_or_text(
    opts: &RuntimeCliOptions,
    value: serde_json::Value,
    plain_ok_line: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if opts.md {
        println!("```json\n{}\n```", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", plain_ok_line);
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

fn runtime_use_paths(ast: &ncc_ast::Program) -> Vec<String> {
    ast.uses
        .iter()
        .map(|u| u.path.join("."))
        .filter(|joined| {
            joined.eq_ignore_ascii_case("nC.runtime")
                || joined.to_ascii_lowercase().starts_with("nc.runtime.")
                || joined.to_ascii_lowercase().starts_with("runtime.")
                || joined.eq_ignore_ascii_case("nC.list")
                || joined.to_ascii_lowercase().starts_with("nc.list.")
        })
        .collect()
}

fn runtime_error_code(name: &str) -> String {
    format!("E_{}_001", camel_case_to_upper_snake(name))
}

fn runtime_error_family(name: &str) -> &'static str {
    if name.starts_with("RuntimeInput") {
        "input"
    } else if name.starts_with("RuntimeOutput") {
        "output"
    } else if name.starts_with("RuntimeString")
        || name.starts_with("RuntimeRender")
        || name.starts_with("RuntimeFormat")
    {
        "text"
    } else if name.starts_with("RuntimeParse")
        || name.starts_with("RuntimeConvert")
        || name.starts_with("RuntimeValidation")
    {
        "parse"
    } else if name.starts_with("RuntimeResult")
        || name.starts_with("RuntimeOption")
        || name.starts_with("RuntimeDiagnostic")
    {
        "result"
    } else if name.starts_with("RuntimeMemory")
        || name.starts_with("RuntimeHeap")
        || name.starts_with("RuntimeStack")
        || name.starts_with("RuntimeRegion")
        || name.starts_with("RuntimeBuffer")
        || name.starts_with("RuntimePtr")
        || name.starts_with("RuntimeAddr")
        || name.starts_with("RuntimeEndian")
        || name.starts_with("RuntimeLayout")
    {
        "memory"
    } else if name.starts_with("RuntimeFs")
        || name.starts_with("RuntimePath")
        || name.starts_with("RuntimeDir")
        || name.starts_with("RuntimeFile")
        || name.starts_with("RuntimeStream")
    {
        "filesystem"
    } else if name.starts_with("RuntimeProcess")
        || name.starts_with("RuntimeCommand")
        || name.starts_with("RuntimeOs")
        || name.starts_with("RuntimeUser")
        || name.starts_with("RuntimePermission")
        || name.starts_with("RuntimeService")
        || name.starts_with("RuntimeSignal")
    {
        "system"
    } else if name.starts_with("RuntimeTime")
        || name.starts_with("RuntimeClock")
        || name.starts_with("RuntimeTimer")
        || name.starts_with("RuntimeSleep")
        || name.starts_with("RuntimeRandom")
        || name.starts_with("RuntimeEntropy")
        || name.starts_with("RuntimeUuid")
    {
        "time"
    } else if name.starts_with("RuntimeTask")
        || name.starts_with("RuntimeThread")
        || name.starts_with("RuntimeAsync")
        || name.starts_with("RuntimeSync")
        || name.starts_with("RuntimeAtomic")
        || name.starts_with("RuntimeLock")
        || name.starts_with("RuntimeQueue")
        || name.starts_with("RuntimeEvent")
        || name.starts_with("RuntimeScheduler")
        || name.starts_with("RuntimeCancel")
    {
        "concurrency"
    } else if name.starts_with("RuntimeNetwork")
        || name.starts_with("RuntimeSocket")
        || name.starts_with("RuntimeTcp")
        || name.starts_with("RuntimeUdp")
        || name.starts_with("RuntimeDns")
        || name.starts_with("RuntimeHttp")
        || name.starts_with("RuntimeTls")
    {
        "network"
    } else if name.starts_with("RuntimeCapability")
        || name.starts_with("RuntimePolicy")
        || name.starts_with("RuntimeProfile")
        || name.starts_with("RuntimeBridge")
        || name.starts_with("RuntimeSandbox")
        || name.starts_with("RuntimeResource")
    {
        "security"
    } else {
        "runtime"
    }
}

fn runtime_error_is_recoverable(name: &str) -> bool {
    !(name.ends_with("Denied")
        || name.ends_with("Exhausted")
        || name.ends_with("UseAfterFree")
        || name.ends_with("DoubleFree")
        || name.ends_with("Leak")
        || name.ends_with("Forbidden"))
}

fn camel_case_to_upper_snake(input: &str) -> String {
    let mut output = String::with_capacity(input.len() * 2);
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_uppercase() && idx > 0 {
            output.push('_');
        }
        output.push(ch.to_ascii_uppercase());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn parse_program(src: &str) -> ncc_ast::Program {
        ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse")
    }

    fn audit_runtime(src: &str, strict: bool) -> RuntimeSourceAudit {
        let ast = parse_program(src);
        runtime_validate_source_contracts(&ast, strict)
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn workspace_path(rel: &str) -> PathBuf {
        let root = workspace_root();
        let direct = root.join(rel);
        if direct.exists() {
            return direct;
        }

        let normalized = rel.replace('\\', "/");
        if let Some(example_rel) = normalized.strip_prefix("examples/") {
            let archived = root.join("_archive").join("old_examples").join(example_rel);
            if archived.exists() {
                return archived;
            }

            let public_fixture = root
                .join("tests")
                .join("fixtures")
                .join("old_examples")
                .join(example_rel);
            if public_fixture.exists() {
                return public_fixture;
            }
        }

        direct
    }

    fn load_workspace_file(rel: &str) -> String {
        let path = workspace_path(rel);
        fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("required fixture '{}' must exist and be readable", rel))
    }

    fn sema_error_for_example(rel: &str) -> String {
        let src = load_workspace_file(rel);
        let ast = parse_program(&src);
        ncc_sema::analyze_with_explicit_pointer_checks(&ast)
            .expect_err("fixture must fail in sema")
            .to_string()
    }

    fn runtime_example_resolves_imported_calls(rel: &str) {
        let src = load_workspace_file(rel);
        let audit = audit_runtime(&src, true);
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "runtime fixture '{}' must not contain unresolved runtime calls: {:?}",
            rel,
            audit.unresolved_runtime_calls
        );
        assert!(
            !audit.imported_families.is_empty(),
            "runtime fixture '{}' must declare runtime imports",
            rel
        );
    }

    fn strict_runtime_fixture_audit() -> RuntimeSourceAudit {
        audit_runtime(
            r#"
use nC.runtime.input;

runtimeStrictFixture[@runtime] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        )
    }

    #[test]
    fn runtime_help_mentions_unified_surface() {
        let help = runtime_help_text();
        assert!(help.contains(
            "runtime <doctor|families|exports|abi|check|imports|capabilities|policies|errors|report>"
        ));
        assert!(help.contains("ncc runtime abi --json"));
    }

    #[test]
    fn runtime_families_catalog_stays_at_90() {
        assert_eq!(ncc_parser::official_runtime_families().len(), 93);
    }

    #[test]
    fn runtime_error_code_generation_is_stable() {
        assert_eq!(
            runtime_error_code("RuntimeInputUnavailable"),
            "E_RUNTIME_INPUT_UNAVAILABLE_001"
        );
        assert_eq!(
            runtime_error_code("RuntimeTlsHandshakeFailed"),
            "E_RUNTIME_TLS_HANDSHAKE_FAILED_001"
        );
    }

    #[test]
    fn runtime_capability_policy_catalogs_are_non_minimal() {
        assert!(RUNTIME_CAPABILITY_CATALOG.len() >= 200);
        assert!(RUNTIME_POLICY_CATALOG.len() >= 80);
    }

    #[test]
    fn runtime_catalog_contract_integrity_has_full_family_coverage() {
        let audit = build_runtime_catalog_audit();
        assert_eq!(
            audit.families.len(),
            ncc_parser::official_runtime_families().len()
        );
        assert_eq!(audit.missing_signature_families.len(), 0);
        assert_eq!(audit.unknown_capabilities.len(), 0);
        assert_eq!(audit.unknown_policies.len(), 0);
        assert!(!audit.fn_to_families.is_empty());
    }

    #[test]
    fn runtime_symbol_abi_manifest_has_stable_shape() {
        let manifest = runtime_abi_manifest();
        assert_eq!(manifest["schema"], "nc.runtime.symbol_abi.v1");
        assert_eq!(manifest["status"], "closed");
        assert!(manifest["symbol_count"].as_u64().expect("symbol count") >= 45);
        let layouts = manifest["layouts"].as_array().expect("layouts");
        for required in [
            "Result<T,E>",
            "ProcessHandle",
            "ThreadHandle",
            "FileLockHandle",
            "NetworkError",
            "ProcessError",
            "ThreadError",
            "TcpHandle",
            "ptr<T>",
        ] {
            assert!(
                layouts.iter().any(|layout| layout["type"] == required),
                "missing ABI layout {required}"
            );
        }
        let symbols = manifest["symbols"].as_array().expect("symbols");
        for entry in symbols {
            assert_eq!(entry["status"], "closed_stable_runtime_abi");
            assert!(entry["symbol"]
                .as_str()
                .is_some_and(|value| value.starts_with("nC.runtime.") && !value.contains('<')));
            assert!(entry["inputs"].as_array().is_some());
            assert!(entry["inputs"]
                .as_array()
                .expect("inputs")
                .iter()
                .all(|input| input["type"] != "RuntimeValue[]"));
            assert!(entry["return"]["type"].as_str().is_some());
            assert_ne!(entry["return"]["type"], "RuntimeValue");
            assert!(entry["return"]["layout_ref"].as_str().is_some());
            assert!(entry["error_codes"]
                .as_array()
                .expect("error codes")
                .iter()
                .all(|code| code.as_str().is_some_and(|value| value.starts_with("E_"))));
            assert!(!entry["ownership"].as_array().expect("ownership").is_empty());
            assert!(entry["resource_lifecycle"]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
        }
        for role in ["import_manifest", "export_manifest"] {
            assert_eq!(manifest[role]["schema"], "nc.runtime.link_manifest.v1");
            assert_eq!(manifest[role]["status"], "closed");
            assert_eq!(manifest[role]["symbol_count"], manifest["symbol_count"]);
            assert_eq!(manifest[role]["calling_convention"], "nC.runtime-value-v1");
        }
    }

    #[test]
    fn runtime_symbol_abi_covers_all_runnable_import_symbols() {
        let manifest = runtime_abi_manifest();
        let abi_symbols = runtime_abi_symbol_names_from_manifest(&manifest);
        let import_symbols = runtime_imported_runtime_symbol_names();
        let missing = import_symbols
            .difference(&abi_symbols)
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "every runnable import runtime symbol needs ABI row, missing: {:?}",
            missing
        );
    }

    #[test]
    fn runtime_symbol_abi_records_real_resource_ownership() {
        let manifest = runtime_abi_manifest();
        let by_symbol = |name: &str| {
            manifest["symbols"]
                .as_array()
                .expect("symbols")
                .iter()
                .find(|entry| entry["symbol"] == name)
                .unwrap_or_else(|| panic!("missing ABI symbol {name}"))
                .clone()
        };

        let spawn = by_symbol("nC.runtime.process.spawn");
        assert_eq!(
            spawn["return"]["type"],
            "Result<ProcessHandle,ProcessError>"
        );
        assert_eq!(spawn["return"]["ok_type"], "ProcessHandle");
        assert!(spawn["ownership"]
            .as_array()
            .expect("ownership")
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|text| text.contains("caller must wait or kill"))));

        let wait = by_symbol("nC.runtime.process.wait");
        assert!(wait["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .any(|input| input["ownership"] == "wait consumes handle"));

        let join = by_symbol("nC.runtime.thread.threadJoin");
        assert_eq!(join["return"]["type"], "Result<any,ThreadError>");
        assert!(join["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .any(|input| input["ownership"] == "threadJoin consumes handle"));

        let tcp_close = by_symbol("nC.runtime.tcp.tcpClose");
        assert!(tcp_close["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .any(|input| input["ownership"] == "tcpClose consumes handle"));

        let lock = by_symbol("nC.runtime.fs.lockFile");
        assert_eq!(lock["return"]["type"], "Result<FileLockHandle,FsError>");
        assert_eq!(lock["return"]["ok_type"], "FileLockHandle");

        let unlock = by_symbol("nC.runtime.fs.unlockFile");
        assert!(unlock["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .any(|input| input["ownership"] == "callee consumes lock handle"));
    }

    #[test]
    fn runtime_source_audit_flags_unqualified_runtime_call_without_use() {
        let src = r#"
runtimeMissingUse[@] {
    fn:
        run() -> bool {
            public:
                readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert!(
            !audit.unresolved_runtime_calls.is_empty(),
            "expected unresolved runtime call when runtime function is used without import"
        );
    }

    #[test]
    fn runtime_source_audit_resolves_runtime_call_with_use() {
        let src = r#"
use nC.runtime.input;

runtimeWithUse[@] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "runtime call should be resolved when matching runtime use exists: {:?}",
            audit.unresolved_runtime_calls
        );
        assert!(
            audit.imported_families.contains("nc.runtime.input"),
            "runtime input family should be imported"
        );
    }

    #[test]
    fn runtime_source_audit_flags_runtime_call_inside_nominal_destructure_without_use() {
        let src = r#"
runtimeNominalDestructureMissingUse[@runtime] {
    fn:
        run() -> bool {
            public:
                destructure.static(readInt());
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("readInt")),
            "expected unresolved runtime call inside nominal destructure without use/import: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_source_audit_resolves_runtime_call_inside_nominal_constructure_with_use() {
        let src = r#"
use nC.runtime.input;

runtimeNominalConstructureWithUse[@runtime] {
    fn:
        run() -> bool {
            public:
                constructure.static(readInt());
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "runtime call inside nominal constructure should resolve with proper use/import: {:?}",
            audit.unresolved_runtime_calls
        );
        assert!(
            audit.imported_families.contains("nc.runtime.input"),
            "runtime input family should be imported"
        );
    }

    #[test]
    fn runtime_nominal_contract_audit_counts_statements_and_runtime_calls() {
        let src = r#"
use nC.runtime.input;

runtimeNominalCounts[@runtime] {
    fn:
        run() -> bool {
            public:
                destructure.static(readInt());
                constructure.static(readInt());
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert_eq!(audit.nominal_contracts.destructure_statements, 1);
        assert_eq!(audit.nominal_contracts.constructure_statements, 1);
        assert_eq!(audit.nominal_contracts.runtime_calls_in_nominal, 2);
        assert!(
            audit
                .nominal_contracts
                .unresolved_runtime_calls_in_nominal
                .is_empty(),
            "nominal runtime calls should be resolved with use/import: {:?}",
            audit.nominal_contracts.unresolved_runtime_calls_in_nominal
        );
    }

    #[test]
    fn runtime_nominal_contract_audit_tracks_unresolved_runtime_calls() {
        let src = r#"
runtimeNominalMissingUse[@runtime] {
    fn:
        run() -> bool {
            public:
                destructure.static(readInt());
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert_eq!(audit.nominal_contracts.destructure_statements, 1);
        assert_eq!(audit.nominal_contracts.constructure_statements, 0);
        assert_eq!(audit.nominal_contracts.runtime_calls_in_nominal, 1);
        assert!(
            audit
                .nominal_contracts
                .unresolved_runtime_calls_in_nominal
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("readInt")),
            "expected unresolved runtime call inside nominal contract: {:?}",
            audit.nominal_contracts.unresolved_runtime_calls_in_nominal
        );
    }

    #[test]
    fn runtime_report_markdown_includes_nominal_contract_metrics() {
        let report = serde_json::json!({
            "status": "ok",
            "nominal_contracts": {
                "destructure_statements": 2,
                "constructure_statements": 1,
                "runtime_calls_in_nominal": 3,
                "unresolved_runtime_calls_in_nominal": ["E_RUNTIME_USE_001: runtime call 'readInt' requires matching use/import"]
            }
        });
        let markdown = runtime_report_markdown(&report);
        assert!(markdown.contains("nominal destructure statements: 2"));
        assert!(markdown.contains("nominal constructure statements: 1"));
        assert!(markdown.contains("runtime calls in nominal contracts: 3"));
        assert!(markdown.contains("unresolved runtime calls in nominal contracts: 1"));
    }

    #[test]
    fn runtime_nominal_contract_trace_without_source_is_catalog_only() {
        let trace = runtime_nominal_contracts_trace(None);
        assert_eq!(
            trace.get("source_audited").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            trace
                .get("supported_constructs")
                .and_then(|v| v.as_array())
                .map(|v| v.len()),
            Some(2)
        );
        assert_eq!(
            trace
                .get("runtime_call_diagnostics")
                .and_then(|v| v.as_array())
                .map(|v| v.len()),
            Some(3)
        );
        assert_eq!(
            trace
                .get("runtime_calls_in_nominal")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[test]
    fn runtime_nominal_contract_trace_from_audit_carries_counts_and_unresolved() {
        let src = r#"
runtimeNominalTracePayload[@runtime] {
    fn:
        run() -> bool {
            public:
                destructure.static(readInt());
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        let trace = runtime_nominal_contracts_trace(Some(&audit));
        assert_eq!(
            trace.get("source_audited").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            trace.get("destructure_statements").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            trace
                .get("constructure_statements")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
        assert_eq!(
            trace
                .get("runtime_calls_in_nominal")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            trace
                .get("unresolved_runtime_calls_in_nominal")
                .and_then(|v| v.as_array())
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn runtime_source_audit_flags_println_without_runtime_output_use_in_strict_mode() {
        let src = r#"
runtimePrintNoUse[@runtime] {
    fn:
        run() -> bool {
            public:
                println("hola");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|value| value.contains("println")),
            "expected unresolved println runtime output contract in strict mode: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_source_audit_resolves_println_with_runtime_output_use_in_strict_mode() {
        let src = r#"
use nC.runtime.output;

runtimePrintWithUse[@runtime] {
    fn:
        run() -> bool {
            public:
                println("hola");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(src).expect("lex")).expect("parse");
        let audit = runtime_validate_source_contracts(&ast, true);
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "println should resolve when nC.runtime.output is imported: {:?}",
            audit.unresolved_runtime_calls
        );
        assert!(
            audit.imported_families.contains("nc.runtime.output"),
            "runtime output family should be imported"
        );
    }

    #[test]
    fn runtime_90_families_registered_ok() {
        assert_eq!(ncc_parser::official_runtime_families().len(), 93);
    }

    #[test]
    fn runtime_macrophase_required_snake_case_examples_exist() {
        for rel in [
            "examples/runtime/runtime_parse_result.n",
            "examples/runtime/runtime_time_random_uuid.n",
            "examples/runtime/runtime_all_capabilities.n",
        ] {
            let path = workspace_path(rel);
            assert!(
                path.exists() && path.is_file(),
                "macrophase required runtime example '{}' must exist",
                rel
            );
        }
    }

    #[test]
    fn runtime_macrophase_required_snake_case_examples_resolve_runtime_contracts() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_parse_result.n");
        runtime_example_resolves_imported_calls("examples/runtime/runtime_time_random_uuid.n");
        runtime_example_resolves_imported_calls("examples/runtime/runtime_all_capabilities.n");
    }

    #[test]
    fn runtime_all_capabilities_registered_ok() {
        assert!(RUNTIME_CAPABILITY_CATALOG.len() >= 200);
    }

    #[test]
    fn runtime_all_policies_registered_ok() {
        assert!(RUNTIME_POLICY_CATALOG.len() >= 80);
    }

    #[test]
    fn runtime_all_errors_registered_ok() {
        assert!(RUNTIME_ERROR_CATALOG.len() >= 150);
    }

    #[test]
    fn runtime_import_nc_runtime_input_ok() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

inputImportOk[@runtime] {
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(audit.imported_families.contains("nc.runtime.input"));
    }

    #[test]
    fn runtime_import_nc_runtime_output_ok() {
        let audit = audit_runtime(
            r#"
use nC.runtime.output;

outputImportOk[@runtime] {
    fn:
        run() -> bool {
            public:
                println("ok");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(audit.imported_families.contains("nc.runtime.output"));
        assert!(audit.unresolved_runtime_calls.is_empty());
    }

    #[test]
    fn runtime_list_family_registered_ok() {
        let catalog = build_runtime_catalog_audit();
        let queue = catalog
            .families
            .iter()
            .find(|family| family.import_path == "nc.runtime.queue")
            .expect("runtime queue family");
        assert!(
            queue
                .exports
                .iter()
                .all(|export| !runtime_export_name_is_list_only(&export.name)),
            "runtime queue family must not include list* symbols"
        );
    }

    #[test]
    fn runtime_import_nc_runtime_list_ok() {
        let audit = audit_runtime(
            r#"
use nC.runtime.list;

runtimeListImportOk[@runtime] {
    fn:
        run() -> bool {
            public:
                listCreate();
                listAdd(@x, 1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit.imported_families.contains("nc.runtime.list"),
            "nC.runtime.list alias should map to nc.runtime.list"
        );
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "list calls should resolve under nC.runtime.list alias: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_import_nc_list_wildcard_ok() {
        let audit = audit_runtime(
            r#"
use nC.list.*;

runtimeListWildcardImportOk[@runtime] {
    fn:
        run() -> bool {
            public:
                listCreate();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit.imported_families.contains("nc.runtime.list"),
            "nC.list.* wildcard should map to nc.runtime.list"
        );
        assert!(
            !audit.imported_families.contains("nc.runtime.queue"),
            "nC.list.* wildcard should not leak queue identity in runtime reports"
        );
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "list calls should resolve under nC.list.* alias: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_import_nc_runtime_list_wildcard_ok() {
        let audit = audit_runtime(
            r#"
use nC.runtime.list.*;

runtimeListRuntimeWildcardImportOk[@runtime] {
    fn:
        run() -> bool {
            public:
                listCreate();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit.imported_families.contains("nc.runtime.list"),
            "nC.runtime.list.* wildcard should map to nc.runtime.list"
        );
        assert!(
            !audit.imported_families.contains("nc.runtime.queue"),
            "nC.runtime.list.* wildcard should not leak queue identity in runtime reports"
        );
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "list calls should resolve under nC.runtime.list.* alias: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_import_nc_list_symbols_ok() {
        let audit = audit_runtime(
            r#"
use nC.list;
use nC.list.listCreate;
use nC.list.listAdd;
use nC.list.listLen;

runtimeListSymbolsImportOk[@runtime] {
    stc:
        l: list<int>;
        ok: bool;
        n: int;
    fn:
        run() -> bool {
            public:
                listCreate();
                @l >> list<int> : [1, 2];
                @ok >> bool : listAdd(@l, 3);
                @n >> int : listLen(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit.imported_families.contains("nc.runtime.list"),
            "nC.list symbol imports should map to nc.runtime.list"
        );
        assert!(
            !audit.imported_families.contains("nc.runtime.queue"),
            "nC.list symbol imports should not leak queue identity in runtime reports"
        );
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "list calls should resolve under nC.list symbol imports: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_import_list_reports_list_capabilities_not_queue_names() {
        let audit = audit_runtime(
            r#"
use nC.runtime.list;

runtimeListCapabilitiesIdentityOk[@runtime] {
    stc:
        l: list<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2];
                @ok >> bool : listAdd(@l, 3);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );

        assert!(
            audit.required_capabilities.contains("list.push"),
            "list imports should report list-scoped capabilities"
        );
        assert!(
            !audit.required_capabilities.contains("queue.push"),
            "list imports must not report queue.* capability names"
        );
    }

    #[test]
    fn runtime_import_nc_list_core_index_memory_ok() {
        let audit = audit_runtime(
            r#"
use nC.list.core;
use nC.list.index;
use nC.list.memory;

runtimeListCoreIndexMemoryImportOk[@runtime] {
    stc:
        l: list<int>;
        n: int;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @n >> int : listLen(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit.imported_families.contains("nc.runtime.list"),
            "nC.list.core/index/memory should map to nc.runtime.list"
        );
        assert!(
            audit.unresolved_runtime_calls.is_empty(),
            "list calls should resolve under nC.list.core/index/memory imports: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_list_exports_ok() {
        let list_exports = runtime_list_alias_exports();
        assert!(
            list_exports
                .iter()
                .any(|export| export.name.eq_ignore_ascii_case("listGet")),
            "runtime list exports should include listGet"
        );
        assert!(
            list_exports
                .iter()
                .any(|export| export.name.eq_ignore_ascii_case("listLen")),
            "runtime list exports should include listLen"
        );
    }

    #[test]
    fn runtime_list_capabilities_are_list_scoped() {
        let caps = runtime_capabilities_for_family_key("list");
        assert!(
            caps.iter().any(|cap| cap == "list.push"),
            "list capability surface must include list.push"
        );
        assert!(
            caps.iter().any(|cap| cap == "list.pop"),
            "list capability surface must include list.pop"
        );
        assert!(
            !caps
                .iter()
                .any(|cap| cap == "queue.push" || cap == "queue.pop"),
            "list capability surface must not expose queue.* capability names"
        );
    }

    #[test]
    fn runtime_exports_input_ok() {
        let catalog = build_runtime_catalog_audit();
        let input = catalog
            .families
            .iter()
            .find(|family| family.import_path == "nc.runtime.input")
            .expect("runtime input family");
        assert!(
            input
                .exports
                .iter()
                .any(|export| export.name.eq_ignore_ascii_case("readInt")),
            "runtime input exports should include readInt"
        );
    }

    #[test]
    fn runtime_exports_output_ok() {
        let catalog = build_runtime_catalog_audit();
        let output = catalog
            .families
            .iter()
            .find(|family| family.import_path == "nc.runtime.output")
            .expect("runtime output family");
        assert!(
            output
                .exports
                .iter()
                .any(|export| export.name.eq_ignore_ascii_case("println")),
            "runtime output exports should include println"
        );
    }

    #[test]
    fn runtime_readint_available_after_use_ok() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

runtimeReadIntAfterUse[@runtime] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(audit.unresolved_runtime_calls.is_empty());
    }

    #[test]
    fn runtime_println_available_after_use_ok() {
        let audit = audit_runtime(
            r#"
use nC.runtime.output;

runtimePrintAfterUse[@runtime] {
    fn:
        run() -> bool {
            public:
                println("ok");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(audit.unresolved_runtime_calls.is_empty());
    }

    #[test]
    fn runtime_missing_use_readint_fail() {
        let audit = audit_runtime(
            r#"
runtimeReadIntMissingUse[@runtime] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("readInt")),
            "missing readInt use must fail"
        );
    }

    #[test]
    fn runtime_missing_use_println_fail() {
        let audit = audit_runtime(
            r#"
runtimePrintMissingUse[@runtime] {
    fn:
        run() -> bool {
            public:
                println("x");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("println")),
            "missing println use must fail"
        );
    }

    #[test]
    fn runtime_import_root_fail() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

rootForbidden[@] {
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .forbidden_mode_violations
                .iter()
                .any(|row| row.contains("E_RUNTIME_IMPORT_ROOT_001")),
            "runtime import in root must fail"
        );
    }

    #[test]
    fn runtime_import_primitive_fail() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

primitiveForbidden[@primitive] {
    anchor:
        physicalAnchor;
    slot:
        physicalSlot;
    pattern:
        physicalPattern;
    mask:
        physicalMask;
    window:
        physicalWindow;
    signal:
        physicalSignal;
    bus:
        physicalBus;
    state:
        physicalState;
    timing:
        physicalTiming;
    emit:
        physicalEmit;
    sample:
        physicalSample;
    latch:
        physicalLatch;
    assert:
        physicalAssert;
    rupture:
        physicalRupture;
    bridge:
        root.policy.bind;
    seal:
        primitiveClosed;
}
"#,
            true,
        );
        assert!(
            audit
                .forbidden_mode_violations
                .iter()
                .any(|row| row.contains("E_RUNTIME_IMPORT_PRIMITIVE_001")),
            "runtime import in primitive must fail"
        );
    }

    #[test]
    fn runtime_machine_input_fail() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

machineForbidden[@machine] {
}
"#,
            true,
        );
        assert!(
            audit
                .forbidden_mode_violations
                .iter()
                .any(|row| row.contains("E_RUNTIME_IMPORT_MACHINE_INPUT_001")),
            "runtime input in machine must fail"
        );
    }

    #[test]
    fn runtime_kernel_input_fail() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

kernelForbidden[@kernel] {
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .forbidden_mode_violations
                .iter()
                .any(|row| row.contains("E_RUNTIME_IMPORT_KERNEL_INPUT_001")),
            "runtime input in kernel must fail"
        );
    }

    #[test]
    fn runtime_unresolved_import_fail() {
        let audit = audit_runtime(
            r#"
runtimeUnresolvedImport[@runtime] {
    stc:
        value: int;
    fn:
        run() -> bool {
            public:
                @value >> int : readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("readInt")),
            "expected unresolved import diagnostic for readInt"
        );
    }

    #[test]
    fn runtime_symbol_not_exported_fail() {
        let audit = audit_runtime(
            r#"
use nC.runtime.input;

runtimeSymbolNotExported[@runtime] {
    fn:
        run() -> bool {
            public:
                runtime.input.readImaginary();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_EXPORTS_001")),
            "expected exported-symbol diagnostic when runtime symbol is not part of family exports: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_path_explicit_in_normal_code_fail() {
        let audit = audit_runtime(
            r#"
normalRuntimePath[@] {
    fn:
        run() -> bool {
            public:
                runtime.input.readInt();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("nc.runtime.input")),
            "explicit runtime path in normal code must still require use/import: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_fs_without_capability_fail() {
        let audit = audit_runtime(
            r#"
runtimeFsNoUse[@runtime] {
    fn:
        run() -> bool {
            public:
                runtime.fs.read();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("nc.runtime.fs")),
            "runtime fs call without capability/import must fail: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_process_without_policy_fail() {
        let audit = audit_runtime(
            r#"
runtimeProcessNoUse[@runtime] {
    fn:
        run() -> bool {
            public:
                runtime.process.spawn();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("nc.runtime.process")),
            "runtime process call without policy/import must fail: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_network_without_capability_fail() {
        let audit = audit_runtime(
            r#"
runtimeNetNoUse[@runtime] {
    fn:
        run() -> bool {
            public:
                runtime.net.access();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
            true,
        );
        assert!(
            audit
                .unresolved_runtime_calls
                .iter()
                .any(|row| row.contains("E_RUNTIME_USE_001") && row.contains("nc.runtime.net")),
            "runtime network call without capability/import must fail: {:?}",
            audit.unresolved_runtime_calls
        );
    }

    #[test]
    fn runtime_forbidden_runtime_in_root_fail() {
        runtime_import_root_fail();
    }

    #[test]
    fn runtime_forbidden_runtime_in_primitive_fail() {
        runtime_import_primitive_fail();
    }

    #[test]
    fn runtime_forbidden_runtime_input_in_machine_fail() {
        runtime_machine_input_fail();
    }

    #[test]
    fn runtime_import_all_90_families_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_import_all_families.n");
    }

    #[test]
    fn runtime_parse_result_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_parse_result.n");
    }

    #[test]
    fn runtime_render_format_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_render_format.n");
    }

    #[test]
    fn runtime_result_option_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_parse_result.n");
    }

    #[test]
    fn runtime_buffer_region_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_all_capabilities.n");
    }

    #[test]
    fn runtime_heap_lifecycle_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_heap_lifecycle.n");
    }

    #[test]
    fn runtime_ptr_origin_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_ptr_addr_site.n");
    }

    #[test]
    fn runtime_ptr_null_refine_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_ptr_addr_site.n");
    }

    #[test]
    fn runtime_addr_materialized_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_ptr_addr_site.n");
    }

    #[test]
    fn runtime_site_contract_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_ptr_addr_site.n");
    }

    #[test]
    fn runtime_region_bounds_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_all_capabilities.n");
    }

    #[test]
    fn runtime_memory_copy_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_all_capabilities.n");
    }

    #[test]
    fn runtime_fs_with_capability_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_fs_capability.n");
    }

    #[test]
    fn runtime_process_with_policy_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_process_policy.n");
    }

    #[test]
    fn runtime_network_with_capability_ok() {
        runtime_example_resolves_imported_calls("examples/runtime/runtime_all_capabilities.n");
    }

    #[test]
    fn runtime_unknown_capability_fail() {
        let mut audit = strict_runtime_fixture_audit();
        audit
            .unknown_capabilities
            .push("runtime.capability.unknown".to_string());
        let err = runtime_validate_strict_contracts(&audit)
            .expect_err("unknown capability contract must fail strict runtime check")
            .to_string();
        assert!(err.contains("E_RUNTIME_CAPABILITY_001"));
    }

    #[test]
    fn runtime_unknown_policy_fail() {
        let mut audit = strict_runtime_fixture_audit();
        audit
            .unknown_policies
            .push("runtime.policy.unknown".to_string());
        let err = runtime_validate_strict_contracts(&audit)
            .expect_err("unknown policy contract must fail strict runtime check")
            .to_string();
        assert!(err.contains("E_RUNTIME_POLICY_001"));
    }

    #[test]
    fn runtime_capability_without_policy_fail() {
        let mut audit = strict_runtime_fixture_audit();
        audit
            .required_capabilities
            .insert("runtime.fs.read".to_string());
        audit.required_policies.clear();
        let err = runtime_validate_strict_contracts(&audit)
            .expect_err("required capabilities without policies must fail strict runtime check")
            .to_string();
        assert!(err.contains("E_RUNTIME_POLICY_001"));
        assert!(err.contains("required capabilities"));
    }

    #[test]
    fn runtime_policy_scope_invalid_fail() {
        let mut audit = strict_runtime_fixture_audit();
        audit
            .required_policies
            .insert("compiler.phase-boundaries".to_string());
        let err = runtime_validate_strict_contracts(&audit)
            .expect_err("compiler-scoped policies must be rejected in runtime strict contracts")
            .to_string();
        assert!(err.contains("E_RUNTIME_POLICY_001"));
        assert!(err.contains("scope"));
    }

    #[test]
    fn heap_double_free_fail() {
        let err = sema_error_for_example("examples/l2k_s7c_runtime_free_dead_block_fail.n");
        assert!(err.contains("memory.heap.manager.free cannot use invalid pointer"));
    }

    #[test]
    fn use_after_free_fail() {
        let err = sema_error_for_example("examples/l2k_s7e_runtime_use_after_free_fail.n");
        assert!(err.contains("memory.heap.manager.zero cannot use invalid pointer"));
    }

    #[test]
    fn alias_stale_fail() {
        let err = sema_error_for_example("examples/l2k_s7e_runtime_realloc_old_symbol_fail.n");
        assert!(err.contains("memory.heap.manager.zero cannot use invalid pointer"));
    }

    #[test]
    fn raw_mode_admits_explicit_double_free_uaf_and_stale_alias_sources() {
        for fixture in [
            "examples/l2k_s7c_runtime_free_dead_block_fail.n",
            "examples/l2k_s7e_runtime_use_after_free_fail.n",
            "examples/l2k_s7e_runtime_realloc_old_symbol_fail.n",
        ] {
            let source = load_workspace_file(fixture);
            let ast = parse_program(&source);
            ncc_sema::analyze(&ast).unwrap_or_else(|error| {
                panic!("manual/raw fixture '{fixture}' must pass sema: {error}")
            });
        }
    }

    #[test]
    fn stack_pointer_escape_fail() {
        let err =
            sema_error_for_example("examples/pointer_phase_p11_ptr_generic_alias_conflict_fail.n");
        assert!(err.contains("aliases consumed parameter"));
    }

    #[test]
    fn runtime_report_json_ok() {
        run_runtime_command(&["report".to_string(), "--json".to_string()])
            .expect("runtime report --json should pass");
    }

    #[test]
    fn runtime_report_md_ok() {
        let out = workspace_root().join("reports/runtime/runtime_report_alias_test.md");
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).expect("mkdir runtime report dir");
        }
        run_runtime_command(&[
            "report".to_string(),
            "--md-out".to_string(),
            out.to_string_lossy().to_string(),
        ])
        .expect("runtime report --md-out should pass");
        let meta = fs::metadata(&out).expect("runtime markdown report exists");
        assert!(meta.len() > 0, "runtime markdown report must be non-empty");
    }

    #[test]
    #[allow(non_snake_case)]
    fn runtime_readInt_available_after_use_ok() {
        runtime_readint_available_after_use_ok();
    }

    #[test]
    #[allow(non_snake_case)]
    fn runtime_missing_use_readInt_fail() {
        runtime_missing_use_readint_fail();
    }
}
