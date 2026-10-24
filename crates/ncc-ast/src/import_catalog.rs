// Baseical import/use catalog for nC v0.1 active line.
// Source basis: roadmap + PDF imports spec + active fixtures/examples.
// The list is intentionally broad to support the staged ~250+ import surface.
use crate::Context;
use serde::Serialize;
use std::sync::OnceLock;

mod runtime_atomic_api;
mod runtime_entropy_uuid_api;
mod runtime_filesystem_api;
mod runtime_json_api;
mod runtime_mobile_api;
mod runtime_sys_api;
mod runtime_time_clock_api;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeApiFunctionSpec {
    pub name: String,
    pub arg_types: Vec<String>,
    pub arity_min: usize,
    pub variadic: bool,
    pub return_ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeApiFamilySpec {
    pub import_path: String,
    pub module_path: String,
    pub functions: Vec<RuntimeApiFunctionSpec>,
}

/// The deliberately small implicit runtime surface available to hosted code.
///
/// This is a semantic/backend prelude, not a source-level `use` injection.
/// Keep this list narrow: adding a family changes unqualified name resolution.
pub const HOSTED_RUNTIME_PRELUDE_V1_FAMILIES: &[&str] = &[
    "nC.runtime.string",
    "nC.runtime.convert",
    "nC.runtime.escape",
    "nC.runtime.align",
    "nC.runtime.format",
    "nC.runtime.math",
    "nC.runtime.random",
    "nC.runtime.entropy",
    "nC.runtime.uuid",
    "nC.runtime.json",
    "nC.runtime.terminal",
    "nC.runtime.time",
    "nC.runtime.clock",
    "nC.runtime.sleep",
    "nC.runtime.fs",
    "nC.runtime.system",
    "nC.runtime.permission",
    "nC.runtime.telephony",
    "nC.runtime.tts",
    "nC.runtime.mobileNetwork",
    "nC.runtime.compilerManual",
    "nC.runtime.aiManual",
    "nC.runtime.highManual",
    "nC.runtime.inspectManual",
    "nC.runtime.toolingManual",
];

#[derive(Debug, Clone)]
pub struct HostedRuntimePreludeFunction {
    pub family: &'static RuntimeApiFamilySpec,
    pub function: &'static RuntimeApiFunctionSpec,
    pub canonical_symbol: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePreludeLowering {
    CompilerIntrinsic,
    Runtime,
}

impl NativePreludeLowering {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CompilerIntrinsic => "compiler_intrinsic",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NativePreludeFunctionSpec {
    pub source_name: &'static str,
    pub canonical_symbol: &'static str,
    pub accepted_arities: &'static [usize],
    pub lowering: NativePreludeLowering,
}

impl NativePreludeFunctionSpec {
    pub fn canonical_call_symbol(self, source_call: &str) -> String {
        let generic_suffix = source_call
            .trim()
            .find('<')
            .map(|index| &source_call.trim()[index..])
            .unwrap_or_default();
        format!("{}{}", self.canonical_symbol, generic_suffix)
    }

    pub fn available_in_context(self, context: Option<&Context>) -> bool {
        match context {
            None | Some(Context::Root | Context::Runtime) => true,
            Some(Context::Kernel | Context::System) => {
                self.lowering == NativePreludeLowering::CompilerIntrinsic
                    || matches!(self.source_name, "atomicLoad" | "atomicStore" | "fence")
            }
            Some(_) => false,
        }
    }
}

/// Exact import-free surface. This is deliberately function-scoped: adding a
/// runtime family here would silently expose unrelated hosted capabilities.
pub const NATIVE_PRELUDE_V1_FUNCTIONS: &[NativePreludeFunctionSpec] = &[
    NativePreludeFunctionSpec {
        source_name: "alloc",
        canonical_symbol: "nC.intrinsic.memory.alloc",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "calloc",
        canonical_symbol: "nC.intrinsic.memory.calloc",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "realloc",
        canonical_symbol: "nC.intrinsic.memory.realloc",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "free",
        canonical_symbol: "nC.intrinsic.memory.free",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "strConcatInto",
        canonical_symbol: "nC.intrinsic.string.strConcatInto",
        accepted_arities: &[4],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "memcpy",
        canonical_symbol: "nC.intrinsic.memory.memcpy",
        accepted_arities: &[3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "memmove",
        canonical_symbol: "nC.intrinsic.memory.memmove",
        accepted_arities: &[3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "memset",
        canonical_symbol: "nC.intrinsic.memory.memset",
        accepted_arities: &[3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "memcmp",
        canonical_symbol: "nC.intrinsic.memory.memcmp",
        accepted_arities: &[3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "closureAt",
        canonical_symbol: "nC.intrinsic.callable.closureAt",
        accepted_arities: &[2, 4],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "closureSize",
        canonical_symbol: "nC.intrinsic.callable.closureSize",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "closureAlign",
        canonical_symbol: "nC.intrinsic.callable.closureAlign",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "asyncAt",
        canonical_symbol: "nC.intrinsic.callable.asyncAt",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "callAt",
        canonical_symbol: "nC.intrinsic.callable.callAt",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "listAt",
        canonical_symbol: "nC.intrinsic.collection.listAt",
        accepted_arities: &[2, 3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "queueAt",
        canonical_symbol: "nC.intrinsic.collection.queueAt",
        accepted_arities: &[2, 3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "vectorAt",
        canonical_symbol: "nC.intrinsic.collection.vectorAt",
        accepted_arities: &[2, 3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "dinAt",
        canonical_symbol: "nC.intrinsic.collection.dinAt",
        accepted_arities: &[2, 3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "vmMap",
        canonical_symbol: "nC.intrinsic.memory.vmMap",
        accepted_arities: &[6],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "vmProtect",
        canonical_symbol: "nC.intrinsic.memory.vmProtect",
        accepted_arities: &[3],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "vmUnmap",
        canonical_symbol: "nC.intrinsic.memory.vmUnmap",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "vmExecutable",
        canonical_symbol: "nC.intrinsic.memory.vmExecutable",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrRead",
        canonical_symbol: "nC.intrinsic.ptr.ptrRead",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrWrite",
        canonical_symbol: "nC.intrinsic.ptr.ptrWrite",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrOffset",
        canonical_symbol: "nC.intrinsic.ptr.ptrOffset",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrDiff",
        canonical_symbol: "nC.intrinsic.ptr.ptrDiff",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrCast",
        canonical_symbol: "nC.intrinsic.ptr.ptrCast",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrAddr",
        canonical_symbol: "nC.intrinsic.ptr.ptrAddr",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrFromAddr",
        canonical_symbol: "nC.intrinsic.ptr.ptrFromAddr",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "ptrIsNull",
        canonical_symbol: "nC.intrinsic.ptr.ptrIsNull",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "checkedAdd",
        canonical_symbol: "nC.intrinsic.arithmetic.checkedAdd",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "sizeOf",
        canonical_symbol: "nC.intrinsic.layout.sizeOf",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "alignOf",
        canonical_symbol: "nC.intrinsic.layout.alignOf",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "offsetOf",
        canonical_symbol: "nC.intrinsic.layout.offsetOf",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::CompilerIntrinsic,
    },
    NativePreludeFunctionSpec {
        source_name: "atomicLoad",
        canonical_symbol: "nC.runtime.atomic.atomicLoad",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "atomicStore",
        canonical_symbol: "nC.runtime.atomic.atomicStore",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "fence",
        canonical_symbol: "nC.runtime.atomic.atomicFence",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "readInt",
        canonical_symbol: "nC.runtime.input.readInt",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "readStr",
        canonical_symbol: "nC.runtime.input.readLine",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "sleepMs",
        canonical_symbol: "nC.runtime.sleep.sleepMs",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "monotonicMs",
        canonical_symbol: "nC.runtime.time.monotonicMs",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "get_current_time",
        canonical_symbol: "nC.runtime.time.now",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "exists",
        canonical_symbol: "nC.runtime.fs.exists",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "fexists",
        canonical_symbol: "nC.runtime.fs.fexists",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "listDir",
        canonical_symbol: "nC.runtime.fs.listDir",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "fswrite",
        canonical_symbol: "nC.runtime.fs.fswrite",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "fread_lines",
        canonical_symbol: "nC.runtime.fs.fread_lines",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "delete_file",
        canonical_symbol: "nC.runtime.fs.delete_file",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "rename_file",
        canonical_symbol: "nC.runtime.fs.rename_file",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "cpu_load",
        canonical_symbol: "nC.runtime.system.cpu_load",
        accepted_arities: &[0],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "registry_write",
        canonical_symbol: "nC.runtime.system.registry_write",
        accepted_arities: &[2],
        lowering: NativePreludeLowering::Runtime,
    },
    NativePreludeFunctionSpec {
        source_name: "registry_read",
        canonical_symbol: "nC.runtime.system.registry_read",
        accepted_arities: &[1],
        lowering: NativePreludeLowering::Runtime,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UseDomain {
    Core,
    Alloc,
    Stoud,
    Auth,
    Dom,
    LegacyBaseical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseFamily {
    pub domain: UseDomain,
    pub name: &'static str,
    pub risk: UseRiskLevel,
    pub requires_policy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseItem {
    pub domain: UseDomain,
    pub family: &'static str,
    pub name: &'static str,
    pub expands_to: &'static str,
    pub risk: UseRiskLevel,
    pub requires_policy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseAliasExpansion {
    pub surface_import: String,
    pub expands_to: Vec<String>,
    pub domain: UseDomain,
    pub family: String,
    pub item: String,
    pub risk: UseRiskLevel,
    pub allowed_modes: Vec<String>,
    pub forbidden_modes: Vec<String>,
    pub requires_policy: bool,
    pub status: String,
    pub closure_status: &'static str,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UseRiskLevel {
    Low,
    Medium,
    High,
    Extreme,
    RootForbidden,
    ModeForbidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseModeGate {
    pub mode: &'static str,
    pub imports_allowed: bool,
    pub diagnostic: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsePolicyRequirement {
    pub domain: UseDomain,
    pub family: &'static str,
    pub requires_policy: bool,
    pub diagnostic: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseImportManifest {
    pub status: &'static str,
    pub entries: Vec<UseAliasExpansion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseExpansionReport {
    pub status: &'static str,
    pub expansions: Vec<UseAliasExpansion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseBackendTargetBytes {
    pub profile: &'static str,
    pub target_triple: &'static str,
    pub architecture: &'static str,
    pub object_format: &'static str,
    pub instruction: &'static str,
    pub bytes_hex: String,
    pub byte_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseBackendRoute {
    pub route: &'static str,
    pub status: &'static str,
    pub lowering: String,
    pub authority: &'static str,
    pub diagnostic_if_unavailable: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseBackendLoweringEntry {
    pub surface_import: String,
    pub expands_to: Vec<String>,
    pub domain: UseDomain,
    pub family: String,
    pub item: String,
    pub risk: UseRiskLevel,
    pub requires_policy: bool,
    pub family_group: String,
    pub backend_symbol: String,
    pub runtime_symbols: Vec<String>,
    pub mct_provider: &'static str,
    pub mct_profiles: Vec<&'static str>,
    pub lowering_kind: String,
    pub linkage: String,
    pub runtime_call_lowering: String,
    pub backend_routes: Vec<UseBackendRoute>,
    pub backend_parity_status: &'static str,
    pub runtime_semantics: String,
    pub emitted_bytes: Vec<UseBackendTargetBytes>,
    pub backing_status: &'static str,
    pub closed_real_backing_bytes: bool,
    pub closure_status: &'static str,
    pub can_run_without_runtime_import_failure: bool,
    pub status: &'static str,
    pub diagnostics: Vec<&'static str>,
}

fn use_runtime_stdlib_package(has_backing: bool) -> &'static str {
    if has_backing {
        "ncc-runtime-stdlib.external+ncc-native+MCT"
    } else {
        "ncc-native+MCT"
    }
}

fn use_runtime_provider_note(domain: UseDomain, family: &str, item: &str) -> Option<&'static str> {
    let family = family.to_ascii_lowercase();
    let item = item.to_ascii_lowercase();
    match (domain, family.as_str(), item.as_str()) {
        (UseDomain::Stoud, "io", "render") => Some(
            "runtime library package: ncc-runtime-stdlib; renderBinary/renderBin provider chain: external libc %b when OS runtime probe accepts %llb, otherwise portable exact byte renderer",
        ),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseBackendFamilyGroup {
    pub domain: UseDomain,
    pub family: String,
    pub risk: UseRiskLevel,
    pub requires_policy: bool,
    pub entries: Vec<UseBackendLoweringEntry>,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseBackendLoweringManifest {
    pub schema: &'static str,
    pub status: &'static str,
    pub mct_connection: &'static str,
    pub entries: Vec<UseBackendLoweringEntry>,
    pub groups: Vec<UseBackendFamilyGroup>,
    pub total_entries: usize,
    pub total_families: usize,
    pub truth_boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RuntimeImportClosureStatus {
    ClosedRealBackingBytes,
    ClosedNativeBackend,
    ClosedInterpreterRuntime,
    CatalogOnlyWithDiagnostic,
}

impl RuntimeImportClosureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClosedRealBackingBytes => "closed_real_backing_bytes",
            Self::ClosedNativeBackend => "closed_native_backend",
            Self::ClosedInterpreterRuntime => "closed_interpreter_runtime",
            Self::CatalogOnlyWithDiagnostic => "catalog_only_with_diagnostic",
        }
    }
}

pub const RUNTIME_IMPORT_CLOSURE_STATUSES: &[&str] = &[
    "closed_real_backing_bytes",
    "closed_native_backend",
    "closed_interpreter_runtime",
    "catalog_only_with_diagnostic",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeImportClosureEntry {
    pub surface_import: String,
    pub expands_to: Vec<String>,
    pub domain: UseDomain,
    pub family: String,
    pub item: String,
    pub closure_status: &'static str,
    pub can_run_without_runtime_import_failure: bool,
    pub backend_symbol: String,
    pub runtime_symbols: Vec<String>,
    pub mct_profiles: Vec<&'static str>,
    pub run_contract: &'static str,
    pub diagnostics: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeImportClosureTable {
    pub schema: &'static str,
    pub status: &'static str,
    pub allowed_statuses: Vec<&'static str>,
    pub entries: Vec<RuntimeImportClosureEntry>,
    pub total_entries: usize,
    pub closed_real_backing_bytes: usize,
    pub closed_native_backend: usize,
    pub closed_interpreter_runtime: usize,
    pub catalog_only_with_diagnostic: usize,
    pub truth_boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UseDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub help: Option<&'static str>,
}

static RUNTIME_API_FAMILY_SPECS_CACHE: OnceLock<Vec<RuntimeApiFamilySpec>> = OnceLock::new();

fn segment_eq_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

pub fn import_path_matches(path: &[String], expected: &[&str]) -> bool {
    path.len() == expected.len()
        && path
            .iter()
            .zip(expected.iter())
            .all(|(segment, expected)| segment_eq_ci(segment, expected))
}

pub fn import_path_starts_with(path: &[String], prefix: &[String]) -> bool {
    path.len() >= prefix.len()
        && path
            .iter()
            .zip(prefix.iter())
            .all(|(segment, expected)| segment_eq_ci(segment, expected))
}

fn runtime_api_function(name: &str, arg_types: &[&str], return_ty: &str) -> RuntimeApiFunctionSpec {
    RuntimeApiFunctionSpec {
        name: name.to_string(),
        arg_types: arg_types.iter().map(|ty| (*ty).to_string()).collect(),
        arity_min: arg_types.len(),
        variadic: false,
        return_ty: return_ty.to_string(),
    }
}

fn auth_resource_manual_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.authHandle".to_string(),
            module_path: "runtime.authHandle".to_string(),
            functions: vec![
                runtime_api_function("handleStride", &[], "int"),
                runtime_api_function(
                    "handleWrite",
                    &["addr", "int", "addr", "int", "int", "int", "int"],
                    "bool",
                ),
                runtime_api_function("handleValue", &["addr", "int"], "addr"),
                runtime_api_function("handleKind", &["addr", "int"], "int"),
                runtime_api_function("handleRights", &["addr", "int"], "int"),
                runtime_api_function("handleFlags", &["addr", "int"], "int"),
                runtime_api_function("handleGeneration", &["addr", "int"], "int"),
                runtime_api_function("handleAllows", &["addr", "int", "int", "int"], "bool"),
                runtime_api_function("handleCopy", &["addr", "int", "addr", "int"], "bool"),
                runtime_api_function("handleClear", &["addr", "int"], "bool"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.authResource".to_string(),
            module_path: "runtime.authResource".to_string(),
            functions: vec![
                runtime_api_function("resourceStride", &[], "int"),
                runtime_api_function(
                    "resourceWrite",
                    &[
                        "addr", "int", "int", "addr", "addr", "int", "int", "int", "int", "int",
                    ],
                    "bool",
                ),
                runtime_api_function("resourceId", &["addr", "int"], "int"),
                runtime_api_function("resourceHandle", &["addr", "int"], "addr"),
                runtime_api_function("resourceBase", &["addr", "int"], "addr"),
                runtime_api_function("resourceBytes", &["addr", "int"], "int"),
                runtime_api_function("resourceKind", &["addr", "int"], "int"),
                runtime_api_function("resourceRights", &["addr", "int"], "int"),
                runtime_api_function("resourceFlags", &["addr", "int"], "int"),
                runtime_api_function("resourceOwner", &["addr", "int"], "int"),
                runtime_api_function("resourceContains", &["addr", "int", "addr", "int"], "bool"),
                runtime_api_function("resourceAllows", &["addr", "int", "int", "int"], "bool"),
                runtime_api_function("resourceCopy", &["addr", "int", "addr", "int"], "bool"),
                runtime_api_function("resourceClear", &["addr", "int"], "bool"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.authSession".to_string(),
            module_path: "runtime.authSession".to_string(),
            functions: vec![
                runtime_api_function("sessionStride", &[], "int"),
                runtime_api_function(
                    "sessionWrite",
                    &[
                        "addr", "int", "int", "int", "addr", "addr", "int", "int", "int",
                    ],
                    "bool",
                ),
                runtime_api_function("sessionId", &["addr", "int"], "int"),
                runtime_api_function("sessionOwner", &["addr", "int"], "int"),
                runtime_api_function("sessionToken", &["addr", "int"], "addr"),
                runtime_api_function("sessionResources", &["addr", "int"], "addr"),
                runtime_api_function("sessionResourceCount", &["addr", "int"], "int"),
                runtime_api_function("sessionRights", &["addr", "int"], "int"),
                runtime_api_function("sessionState", &["addr", "int"], "int"),
                runtime_api_function("sessionHasRight", &["addr", "int", "int"], "bool"),
                runtime_api_function("sessionResourceAt", &["addr", "int", "int", "int"], "addr"),
                runtime_api_function("sessionCopy", &["addr", "int", "addr", "int"], "bool"),
                runtime_api_function("sessionClear", &["addr", "int"], "bool"),
            ],
        },
    ]
}

fn memory_buffer_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.buffer".to_string(),
        module_path: "runtime.buffer".to_string(),
        functions: vec![
            runtime_api_function("bufferStride", &[], "int"),
            runtime_api_function("bufferAt", &["addr", "addr", "int", "int", "int"], "bool"),
            runtime_api_function("bufferData", &["addr"], "addr"),
            runtime_api_function("bufferLen", &["addr"], "int"),
            runtime_api_function("bufferCapacity", &["addr"], "int"),
            runtime_api_function("bufferFlags", &["addr"], "int"),
            runtime_api_function("bufferSetLen", &["addr", "int"], "bool"),
            runtime_api_function("bufferRead8", &["addr", "int"], "int"),
            runtime_api_function("bufferRead16", &["addr", "int"], "int"),
            runtime_api_function("bufferRead32", &["addr", "int"], "int"),
            runtime_api_function("bufferRead64", &["addr", "int"], "u64"),
            runtime_api_function("bufferWrite8", &["addr", "int", "int"], "bool"),
            runtime_api_function("bufferWrite16", &["addr", "int", "int"], "bool"),
            runtime_api_function("bufferWrite32", &["addr", "int", "int"], "bool"),
            runtime_api_function("bufferWrite64", &["addr", "int", "u64"], "bool"),
            runtime_api_function("bufferFill", &["addr", "int", "int", "int"], "bool"),
            runtime_api_function("bufferCopy", &["addr", "int", "addr", "int", "int"], "bool"),
            runtime_api_function("bufferView", &["addr", "addr", "int", "int"], "bool"),
            runtime_api_function("bufferClear", &["addr"], "bool"),
        ],
    }
}

fn memory_bytes_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.bytes".to_string(),
        module_path: "runtime.bytes".to_string(),
        functions: vec![
            runtime_api_function("byteAt", &["addr", "int"], "int"),
            runtime_api_function("bytesToHex", &["addr", "int", "addr", "int"], "int"),
            runtime_api_function("hexToBytes", &["addr", "int", "addr", "int"], "int"),
            runtime_api_function("bytesEq", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("bytesLen", &["addr", "int"], "int"),
        ],
    }
}

fn memory_views_manual_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.slice".to_string(),
            module_path: "runtime.slice".to_string(),
            functions: vec![
                runtime_api_function("sliceStride", &[], "int"),
                runtime_api_function("sliceAt", &["addr", "addr", "int", "int"], "bool"),
                runtime_api_function("sliceData", &["addr"], "addr"),
                runtime_api_function("sliceLen", &["addr"], "int"),
                runtime_api_function("sliceFlags", &["addr"], "int"),
                runtime_api_function("sliceRead8", &["addr", "int"], "int"),
                runtime_api_function("sliceWrite8", &["addr", "int", "int"], "bool"),
                runtime_api_function("sliceView", &["addr", "addr", "int", "int"], "bool"),
                runtime_api_function("sliceIsEmpty", &["addr"], "bool"),
                runtime_api_function("sliceClear", &["addr"], "bool"),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.span".to_string(),
            module_path: "runtime.span".to_string(),
            functions: vec![
                runtime_api_function("spanStride", &[], "int"),
                runtime_api_function("spanSegmentStride", &[], "int"),
                runtime_api_function("spanSegmentAt", &["addr", "int", "addr", "int"], "bool"),
                runtime_api_function("spanAt", &["addr", "addr", "int", "int", "int"], "bool"),
                runtime_api_function("spanSegments", &["addr"], "addr"),
                runtime_api_function("spanCount", &["addr"], "int"),
                runtime_api_function("spanLen", &["addr"], "int"),
                runtime_api_function("spanFlags", &["addr"], "int"),
                runtime_api_function("spanRead8", &["addr", "int"], "int"),
                runtime_api_function("spanContains8", &["addr", "int"], "bool"),
                runtime_api_function("spanIsEmpty", &["addr"], "bool"),
                runtime_api_function("spanClear", &["addr"], "bool"),
            ],
        },
    ]
}

fn security_audit_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.securityAudit".to_string(),
        module_path: "runtime.securityAudit".to_string(),
        functions: vec![
            runtime_api_function("securityAuditStride", &[], "int"),
            runtime_api_function(
                "securityAuditWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("securityAuditEvent", &["addr", "int"], "int"),
            runtime_api_function("securityAuditSubject", &["addr", "int"], "int"),
            runtime_api_function("securityAuditResource", &["addr", "int"], "int"),
            runtime_api_function("securityAuditRights", &["addr", "int"], "int"),
            runtime_api_function("securityAuditDecision", &["addr", "int"], "int"),
            runtime_api_function("securityAuditTick", &["addr", "int"], "int"),
            runtime_api_function("securityAuditDetail", &["addr", "int"], "int"),
            runtime_api_function("securityAuditAllows", &["addr", "int", "int"], "bool"),
            runtime_api_function("securityAuditCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("securityAuditClear", &["addr", "int"], "bool"),
        ],
    }
}

fn compiler_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.compilerManual".to_string(),
        module_path: "runtime.compilerManual".to_string(),
        functions: vec![
            runtime_api_function("compilerUnitStride", &[], "int"),
            runtime_api_function("compilerTokenStride", &[], "int"),
            runtime_api_function("compilerNodeStride", &[], "int"),
            runtime_api_function("compilerIrStride", &[], "int"),
            runtime_api_function("compilerDiagnosticStride", &[], "int"),
            runtime_api_function("compilerManifestStride", &[], "int"),
            runtime_api_function(
                "compilerUnitInit",
                &[
                    "addr", "addr", "int", "addr", "int", "addr", "int", "addr", "int", "addr",
                    "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("compilerParse", &["addr"], "bool"),
            runtime_api_function("compilerAst", &["addr"], "bool"),
            runtime_api_function("compilerSema", &["addr"], "bool"),
            runtime_api_function("compilerLower", &["addr"], "bool"),
            runtime_api_function(
                "compilerDiagnosticWrite",
                &["addr", "int", "int", "int", "int", "int", "int"],
                "bool",
            ),
            runtime_api_function("compilerDiagnosticCount", &["addr"], "int"),
            runtime_api_function("compilerReport", &["addr", "addr"], "int"),
            runtime_api_function("compilerManifest", &["addr", "addr"], "int"),
            runtime_api_function("compilerTokenCount", &["addr"], "int"),
            runtime_api_function("compilerNodeCount", &["addr"], "int"),
            runtime_api_function("compilerIrCount", &["addr"], "int"),
            runtime_api_function("compilerState", &["addr"], "int"),
            runtime_api_function("compilerReset", &["addr"], "bool"),
            runtime_api_function("compilerCopy", &["addr", "addr"], "bool"),
        ],
    }
}

fn ai_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.aiManual".to_string(),
        module_path: "runtime.aiManual".to_string(),
        functions: vec![
            runtime_api_function("aiStride", &[], "int"),
            runtime_api_function(
                "aiWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("aiId", &["addr", "int"], "int"),
            runtime_api_function("aiKind", &["addr", "int"], "int"),
            runtime_api_function("aiInput", &["addr", "int"], "int"),
            runtime_api_function("aiOutput", &["addr", "int"], "int"),
            runtime_api_function("aiWeight", &["addr", "int"], "int"),
            runtime_api_function("aiBias", &["addr", "int"], "int"),
            runtime_api_function("aiFlags", &["addr", "int"], "int"),
            runtime_api_function("aiState", &["addr", "int"], "int"),
            runtime_api_function("aiScore", &["addr", "int", "int"], "int"),
            runtime_api_function("aiSelect", &["addr", "int", "int"], "int"),
            runtime_api_function("aiCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("aiClear", &["addr", "int"], "bool"),
        ],
    }
}

fn high_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.highManual".to_string(),
        module_path: "runtime.highManual".to_string(),
        functions: vec![
            runtime_api_function("highStride", &[], "int"),
            runtime_api_function(
                "highWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "addr", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("highKind", &["addr", "int"], "int"),
            runtime_api_function("highKey", &["addr", "int"], "int"),
            runtime_api_function("highValue", &["addr", "int"], "int"),
            runtime_api_function("highAux", &["addr", "int"], "int"),
            runtime_api_function("highFlags", &["addr", "int"], "int"),
            runtime_api_function("highSource", &["addr", "int"], "addr"),
            runtime_api_function("highSourceBytes", &["addr", "int"], "int"),
            runtime_api_function("highState", &["addr", "int"], "int"),
            runtime_api_function("highEval", &["int", "int", "int"], "int"),
            runtime_api_function("highCopyBytes", &["addr", "addr", "int"], "int"),
            runtime_api_function("highHashBytes", &["addr", "int"], "int"),
            runtime_api_function("highCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("highClear", &["addr", "int"], "bool"),
        ],
    }
}

fn inspect_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.inspectManual".to_string(),
        module_path: "runtime.inspectManual".to_string(),
        functions: vec![
            runtime_api_function("inspectStride", &[], "int"),
            runtime_api_function(
                "inspectWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("inspectEvent", &["addr", "int"], "int"),
            runtime_api_function("inspectSite", &["addr", "int"], "int"),
            runtime_api_function("inspectValue", &["addr", "int"], "int"),
            runtime_api_function("inspectFlags", &["addr", "int"], "int"),
            runtime_api_function("inspectTick", &["addr", "int"], "int"),
            runtime_api_function("inspectDetail", &["addr", "int"], "int"),
            runtime_api_function("inspectSequence", &["addr", "int"], "int"),
            runtime_api_function("inspectState", &["addr", "int"], "int"),
            runtime_api_function("inspectChecksum", &["addr", "int"], "int"),
            runtime_api_function("inspectFind", &["addr", "int", "int"], "int"),
            runtime_api_function("inspectCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("inspectClear", &["addr", "int"], "bool"),
        ],
    }
}

fn tooling_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.toolingManual".to_string(),
        module_path: "runtime.toolingManual".to_string(),
        functions: vec![
            runtime_api_function("toolingStride", &[], "int"),
            runtime_api_function(
                "toolingWrite",
                &[
                    "addr", "int", "int", "addr", "addr", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("toolingKind", &["addr", "int"], "int"),
            runtime_api_function("toolingPath", &["addr", "int"], "addr"),
            runtime_api_function("toolingData", &["addr", "int"], "addr"),
            runtime_api_function("toolingBytes", &["addr", "int"], "int"),
            runtime_api_function("toolingFlags", &["addr", "int"], "int"),
            runtime_api_function("toolingStatus", &["addr", "int"], "int"),
            runtime_api_function("toolingHash", &["addr", "int"], "int"),
            runtime_api_function("toolingState", &["addr", "int"], "int"),
            runtime_api_function("toolingHashBytes", &["addr", "int"], "int"),
            runtime_api_function("toolingCountLines", &["addr", "int"], "int"),
            runtime_api_function("toolingCopyBytes", &["addr", "addr", "int"], "int"),
            runtime_api_function("toolingFindByte", &["addr", "int", "int"], "int"),
            runtime_api_function("toolingCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("toolingClear", &["addr", "int"], "bool"),
        ],
    }
}

fn auth_file_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.authFile".to_string(),
        module_path: "runtime.authFile".to_string(),
        functions: vec![
            runtime_api_function("authFileStride", &[], "int"),
            runtime_api_function(
                "authFileWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("authFilePathKey", &["addr", "int"], "int"),
            runtime_api_function("authFileHandle", &["addr", "int"], "int"),
            runtime_api_function("authFileRights", &["addr", "int"], "int"),
            runtime_api_function("authFileFlags", &["addr", "int"], "int"),
            runtime_api_function("authFileOwner", &["addr", "int"], "int"),
            runtime_api_function("authFileGeneration", &["addr", "int"], "int"),
            runtime_api_function("authFileSizeHint", &["addr", "int"], "int"),
            runtime_api_function("authFileState", &["addr", "int"], "int"),
            runtime_api_function("authFileAllows", &["addr", "int", "int"], "bool"),
            runtime_api_function("authFileCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("authFileClear", &["addr", "int"], "bool"),
        ],
    }
}

fn auth_dir_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.authDir".to_string(),
        module_path: "runtime.authDir".to_string(),
        functions: vec![
            runtime_api_function("authDirStride", &[], "int"),
            runtime_api_function(
                "authDirWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("authDirPathKey", &["addr", "int"], "int"),
            runtime_api_function("authDirParentKey", &["addr", "int"], "int"),
            runtime_api_function("authDirHandle", &["addr", "int"], "int"),
            runtime_api_function("authDirRights", &["addr", "int"], "int"),
            runtime_api_function("authDirFlags", &["addr", "int"], "int"),
            runtime_api_function("authDirOwner", &["addr", "int"], "int"),
            runtime_api_function("authDirGeneration", &["addr", "int"], "int"),
            runtime_api_function("authDirState", &["addr", "int"], "int"),
            runtime_api_function("authDirAllows", &["addr", "int", "int"], "bool"),
            runtime_api_function("authDirCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("authDirClear", &["addr", "int"], "bool"),
        ],
    }
}

fn auth_path_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.authPath".to_string(),
        module_path: "runtime.authPath".to_string(),
        functions: vec![
            runtime_api_function("authPathStride", &[], "int"),
            runtime_api_function(
                "authPathWrite",
                &[
                    "addr", "int", "int", "int", "addr", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("authPathKey", &["addr", "int"], "int"),
            runtime_api_function("authPathBaseKey", &["addr", "int"], "int"),
            runtime_api_function("authPathData", &["addr", "int"], "addr"),
            runtime_api_function("authPathBytes", &["addr", "int"], "int"),
            runtime_api_function("authPathRights", &["addr", "int"], "int"),
            runtime_api_function("authPathFlags", &["addr", "int"], "int"),
            runtime_api_function("authPathOwner", &["addr", "int"], "int"),
            runtime_api_function("authPathState", &["addr", "int"], "int"),
            runtime_api_function("authPathAllows", &["addr", "int", "int"], "bool"),
            runtime_api_function("authPathCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("authPathClear", &["addr", "int"], "bool"),
        ],
    }
}

fn auth_env_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.authEnv".to_string(),
        module_path: "runtime.authEnv".to_string(),
        functions: vec![
            runtime_api_function("authEnvStride", &[], "int"),
            runtime_api_function(
                "authEnvWrite",
                &[
                    "addr", "int", "int", "addr", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("authEnvNameKey", &["addr", "int"], "int"),
            runtime_api_function("authEnvData", &["addr", "int"], "addr"),
            runtime_api_function("authEnvBytes", &["addr", "int"], "int"),
            runtime_api_function("authEnvRights", &["addr", "int"], "int"),
            runtime_api_function("authEnvFlags", &["addr", "int"], "int"),
            runtime_api_function("authEnvOwner", &["addr", "int"], "int"),
            runtime_api_function("authEnvGeneration", &["addr", "int"], "int"),
            runtime_api_function("authEnvState", &["addr", "int"], "int"),
            runtime_api_function("authEnvAllows", &["addr", "int", "int"], "bool"),
            runtime_api_function("authEnvCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("authEnvClear", &["addr", "int"], "bool"),
        ],
    }
}

fn auth_permission_manual_api_family_spec() -> RuntimeApiFamilySpec {
    RuntimeApiFamilySpec {
        import_path: "nC.runtime.authPermission".to_string(),
        module_path: "runtime.authPermission".to_string(),
        functions: vec![
            runtime_api_function("authPermissionStride", &[], "int"),
            runtime_api_function(
                "authPermissionWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "int", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("authPermissionSubject", &["addr", "int"], "int"),
            runtime_api_function("authPermissionResource", &["addr", "int"], "int"),
            runtime_api_function("authPermissionRights", &["addr", "int"], "int"),
            runtime_api_function("authPermissionDenied", &["addr", "int"], "int"),
            runtime_api_function("authPermissionFlags", &["addr", "int"], "int"),
            runtime_api_function("authPermissionOwner", &["addr", "int"], "int"),
            runtime_api_function("authPermissionGeneration", &["addr", "int"], "int"),
            runtime_api_function("authPermissionState", &["addr", "int"], "int"),
            runtime_api_function("authPermissionAllows", &["addr", "int", "int"], "bool"),
            runtime_api_function(
                "authPermissionCopy",
                &["addr", "int", "addr", "int"],
                "bool",
            ),
            runtime_api_function("authPermissionClear", &["addr", "int"], "bool"),
        ],
    }
}

fn auth_system_authority_api_family_spec(
    import_path: &str,
    operation_prefix: &str,
    fields: &[&str],
) -> RuntimeApiFamilySpec {
    let mut functions = vec![
        runtime_api_function(&format!("{operation_prefix}Stride"), &[], "int"),
        runtime_api_function(
            &format!("{operation_prefix}Write"),
            &[
                "addr", "int", "int", "int", "int", "int", "int", "int", "int", "int",
            ],
            "bool",
        ),
    ];
    functions.extend(fields.iter().map(|field| {
        runtime_api_function(
            &format!("{operation_prefix}{field}"),
            &["addr", "int"],
            "int",
        )
    }));
    functions.extend([
        runtime_api_function(
            &format!("{operation_prefix}Allows"),
            &["addr", "int", "int"],
            "bool",
        ),
        runtime_api_function(
            &format!("{operation_prefix}Copy"),
            &["addr", "int", "addr", "int"],
            "bool",
        ),
        runtime_api_function(
            &format!("{operation_prefix}Clear"),
            &["addr", "int"],
            "bool",
        ),
    ]);
    RuntimeApiFamilySpec {
        import_path: import_path.to_string(),
        module_path: import_path
            .strip_prefix("nC.")
            .unwrap_or(import_path)
            .to_string(),
        functions,
    }
}

fn generated_runtime_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    let mut families: Vec<RuntimeApiFamilySpec> =
        include!("import_catalog/runtime_api_catalog_generated.rs");
    families.retain(|family| {
        !family.import_path.eq_ignore_ascii_case("nC.runtime.atomic")
            && !family
                .import_path
                .eq_ignore_ascii_case("nC.runtime.entropy")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.uuid")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.time")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.clock")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.sleep")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.fs")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.dir")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.buffer")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.bytes")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.slice")
            && !family.import_path.eq_ignore_ascii_case("nC.runtime.span")
            && !family
                .import_path
                .eq_ignore_ascii_case("nC.runtime.permission")
    });
    families.push(runtime_atomic_api::runtime_atomic_api_family_spec());
    families.extend(runtime_entropy_uuid_api::runtime_entropy_uuid_api_family_specs());
    families.extend(runtime_time_clock_api::runtime_time_clock_api_family_specs());
    families.extend(runtime_filesystem_api::runtime_filesystem_api_family_specs());
    families.push(runtime_json_api::runtime_json_api_family_spec());
    families.extend(runtime_mobile_api::runtime_mobile_api_family_specs());
    families.extend(runtime_sys_api::runtime_sys_api_family_specs());
    families.extend(auth_resource_manual_api_family_specs());
    families.push(auth_dir_manual_api_family_spec());
    families.push(auth_env_manual_api_family_spec());
    families.push(auth_file_manual_api_family_spec());
    families.push(auth_path_manual_api_family_spec());
    families.push(auth_permission_manual_api_family_spec());
    families.push(auth_system_authority_api_family_spec(
        "nC.runtime.authService",
        "authService",
        &[
            "Key",
            "Handle",
            "Process",
            "Rights",
            "Flags",
            "Owner",
            "Generation",
            "State",
        ],
    ));
    families.push(auth_system_authority_api_family_spec(
        "nC.runtime.authSignal",
        "authSignal",
        &[
            "Key",
            "Target",
            "Code",
            "Rights",
            "Flags",
            "Owner",
            "Generation",
            "State",
        ],
    ));
    families.push(auth_system_authority_api_family_spec(
        "nC.runtime.authSystemSession",
        "authSystemSession",
        &[
            "Key",
            "Principal",
            "Handle",
            "Rights",
            "Flags",
            "Owner",
            "Generation",
            "State",
        ],
    ));
    families.push(auth_system_authority_api_family_spec(
        "nC.runtime.authPolicy",
        "authPolicy",
        &[
            "Id", "Subject", "Resource", "Rights", "Denied", "Flags", "Priority", "State",
        ],
    ));
    families.push(auth_system_authority_api_family_spec(
        "nC.runtime.authCapability",
        "authCapability",
        &[
            "Id",
            "Subject",
            "Resource",
            "Rights",
            "Scope",
            "Flags",
            "Generation",
            "State",
        ],
    ));
    families.push(auth_system_authority_api_family_spec(
        "nC.runtime.authSandbox",
        "authSandbox",
        &[
            "Id",
            "Principal",
            "Root",
            "Rights",
            "Restrictions",
            "Flags",
            "Generation",
            "State",
        ],
    ));
    families.push(memory_buffer_manual_api_family_spec());
    families.push(memory_bytes_manual_api_family_spec());
    families.extend(memory_views_manual_api_family_specs());
    families.push(security_audit_manual_api_family_spec());
    families.push(compiler_manual_api_family_spec());
    families.push(ai_manual_api_family_spec());
    families.push(high_manual_api_family_spec());
    families.push(inspect_manual_api_family_spec());
    families.push(tooling_manual_api_family_spec());
    if let Some(render) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.render"))
    {
        render.functions.push(RuntimeApiFunctionSpec {
            name: "renderStr".to_string(),
            arg_types: vec!["str".to_string()],
            arity_min: 1,
            variadic: false,
            return_ty: "str".to_string(),
        });
    }
    if let Some(string) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.string"))
    {
        string.functions.extend([
            RuntimeApiFunctionSpec {
                name: "strTrim".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "strSplit".to_string(),
                arg_types: vec!["str".to_string(), "str".to_string()],
                arity_min: 2,
                variadic: false,
                return_ty: "array<str>".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "strSplitRelease".to_string(),
                arg_types: vec!["ptr<byte>".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
        ]);
    }
    if let Some(text) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.text"))
    {
        if let Some(length) = text
            .functions
            .iter_mut()
            .find(|function| function.name.eq_ignore_ascii_case("textLength"))
        {
            length.return_ty = "int".to_string();
        }
        if let Some(split) = text
            .functions
            .iter_mut()
            .find(|function| function.name.eq_ignore_ascii_case("textSplit"))
        {
            split.return_ty = "array<str>".to_string();
        }
        text.functions.extend([
            RuntimeApiFunctionSpec {
                name: "textRelease".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "textSplitRelease".to_string(),
                arg_types: vec!["ptr<byte>".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
        ]);
    }
    if let Some(truncate) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.truncate")
    }) {
        for function in &mut truncate.functions {
            if matches!(
                function.name.as_str(),
                "truncate" | "truncateEnd" | "truncateMiddle"
            ) {
                function.arg_types = vec!["str".to_string(), "int".to_string()];
            }
        }
        truncate.functions.push(RuntimeApiFunctionSpec {
            name: "truncateRelease".to_string(),
            arg_types: vec!["str".to_string()],
            arity_min: 1,
            variadic: false,
            return_ty: "bool".to_string(),
        });
    }
    if let Some(unicode) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.unicode")
    }) {
        if let Some(mode) = unicode
            .functions
            .iter_mut()
            .find(|function| function.name.eq_ignore_ascii_case("unicodeMode"))
        {
            mode.return_ty = "str".to_string();
        }
        unicode.functions.extend([
            RuntimeApiFunctionSpec {
                name: "normalizeNfc".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "normalizeNfd".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "normalizeNfkc".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "normalizeNfkd".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "graphemeCount".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "int".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "graphemeSlice".to_string(),
                arg_types: vec!["str".to_string(), "int".to_string(), "int".to_string()],
                arity_min: 3,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "unicodeRelease".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
        ]);
    }
    if let Some(width) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.width"))
    {
        for function in &mut width.functions {
            function.arg_types = vec!["str".to_string()];
            function.arity_min = 1;
            function.variadic = false;
            function.return_ty = "int".to_string();
        }
    }
    if let Some(wrap) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.wrap"))
    {
        for function in &mut wrap.functions {
            function.arg_types = vec!["str".to_string(), "int".to_string()];
            function.arity_min = 2;
            function.variadic = false;
            function.return_ty = "str".to_string();
        }
        wrap.functions.push(RuntimeApiFunctionSpec {
            name: "wrapRelease".to_string(),
            arg_types: vec!["str".to_string()],
            arity_min: 1,
            variadic: false,
            return_ty: "bool".to_string(),
        });
    }
    if let Some(http) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.http"))
    {
        http.functions = vec![
            RuntimeApiFunctionSpec {
                name: "httpGet".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "addr".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpPost".to_string(),
                arg_types: vec!["str".to_string(), "str".to_string()],
                arity_min: 2,
                variadic: false,
                return_ty: "addr".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpRequestRaw".to_string(),
                arg_types: vec![
                    "str".to_string(),
                    "str".to_string(),
                    "str".to_string(),
                    "addr".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ],
                arity_min: 7,
                variadic: false,
                return_ty: "addr".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpStatus".to_string(),
                arg_types: vec!["addr".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "int".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpBody".to_string(),
                arg_types: vec!["addr".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "str".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpBodyData".to_string(),
                arg_types: vec!["addr".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "addr".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpBodyBytes".to_string(),
                arg_types: vec!["addr".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "int".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpError".to_string(),
                arg_types: vec!["addr".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "int".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "httpRelease".to_string(),
                arg_types: vec!["addr".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
        ];
    }
    if let Some(net) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.net"))
    {
        net.functions = vec![
            runtime_api_function("netStartup", &[], "int"),
            runtime_api_function("netShutdown", &[], "int"),
            runtime_api_function("netLastError", &[], "int"),
            runtime_api_function("netAvailable", &[], "bool"),
            runtime_api_function("netResolve", &["str", "int", "addr", "int"], "int"),
            runtime_api_function("netAddressOf", &["str", "int", "addr", "int"], "int"),
            runtime_api_function("netInterfaces", &["addr", "int"], "int"),
        ];
    }
    if let Some(socket) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.socket"))
    {
        socket.functions = vec![
            runtime_api_function("socketCreate", &["int", "int", "int"], "addr"),
            runtime_api_function("socketConnect", &["addr", "str", "int", "int"], "int"),
            runtime_api_function("socketBind", &["addr", "str", "int"], "int"),
            runtime_api_function("socketListen", &["addr", "int"], "int"),
            runtime_api_function("socketAccept", &["addr", "addr", "int"], "addr"),
            runtime_api_function("socketSend", &["addr", "addr", "int", "int"], "int"),
            runtime_api_function("socketRecv", &["addr", "addr", "int", "int"], "int"),
            runtime_api_function("socketShutdown", &["addr", "int"], "int"),
            runtime_api_function("socketSetTimeout", &["addr", "int", "int"], "int"),
            runtime_api_function("socketClose", &["addr"], "int"),
            runtime_api_function("socketLastError", &[], "int"),
        ];
    }
    if let Some(args) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.args"))
    {
        args.functions = vec![
            runtime_api_function("argsOpen", &[], "addr"),
            runtime_api_function("argsCount", &["addr"], "int"),
            runtime_api_function("argsAt", &["addr", "int"], "str"),
            runtime_api_function("argsData", &["addr"], "addr"),
            runtime_api_function("argsBytes", &["addr"], "int"),
            runtime_api_function("argsError", &["addr"], "int"),
            runtime_api_function("argsRelease", &["addr"], "bool"),
        ];
    }
    if let Some(command) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.command")
    }) {
        command.functions = vec![
            runtime_api_function(
                "commandOpen",
                &["str", "str", "addr", "addr", "addr", "int"],
                "addr",
            ),
            runtime_api_function("commandStarted", &["addr"], "bool"),
            runtime_api_function("commandWait", &["addr", "int"], "int"),
            runtime_api_function("commandExitCode", &["addr"], "int"),
            runtime_api_function("commandTerminate", &["addr", "int"], "bool"),
            runtime_api_function("commandProcessHandle", &["addr"], "addr"),
            runtime_api_function("commandThreadHandle", &["addr"], "addr"),
            runtime_api_function("commandId", &["addr"], "int"),
            runtime_api_function("commandThreadId", &["addr"], "int"),
            runtime_api_function("commandError", &["addr"], "int"),
            runtime_api_function("commandRelease", &["addr"], "bool"),
            runtime_api_function("commandPipeOpen", &["bool", "bool"], "addr"),
            runtime_api_function("commandPipeReadHandle", &["addr"], "addr"),
            runtime_api_function("commandPipeWriteHandle", &["addr"], "addr"),
            runtime_api_function("commandPipeRead", &["addr", "addr", "int"], "int"),
            runtime_api_function("commandPipeWrite", &["addr", "addr", "int"], "int"),
            runtime_api_function("commandPipeCloseRead", &["addr"], "bool"),
            runtime_api_function("commandPipeCloseWrite", &["addr"], "bool"),
            runtime_api_function("commandPipeError", &["addr"], "int"),
            runtime_api_function("commandPipeRelease", &["addr"], "bool"),
        ];
    }
    if let Some(env) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.env"))
    {
        env.functions = vec![
            runtime_api_function("envOpen", &[], "addr"),
            runtime_api_function("envCount", &["addr"], "int"),
            runtime_api_function("envAt", &["addr", "int"], "str"),
            runtime_api_function("envData", &["addr"], "addr"),
            runtime_api_function("envBytes", &["addr"], "int"),
            runtime_api_function("envError", &["addr"], "int"),
            runtime_api_function("envRelease", &["addr"], "bool"),
            runtime_api_function("envGet", &["str", "addr", "int"], "int"),
            runtime_api_function("envHas", &["str"], "bool"),
            runtime_api_function("envSet", &["str", "str"], "bool"),
            runtime_api_function("envRemove", &["str"], "bool"),
            runtime_api_function("envExpand", &["str", "addr", "int"], "int"),
            runtime_api_function("envLastError", &[], "int"),
        ];
    }
    if let Some(exit) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.exit"))
    {
        exit.functions = vec![
            runtime_api_function("processCurrent", &[], "addr"),
            runtime_api_function("processOpen", &["int", "bool", "int"], "addr"),
            runtime_api_function("processId", &["addr"], "int"),
            runtime_api_function("processExitCode", &["addr"], "int"),
            runtime_api_function("processTerminate", &["addr", "int"], "bool"),
            runtime_api_function("processClose", &["addr"], "bool"),
            runtime_api_function("exitNow", &["int"], "bool"),
            runtime_api_function("failFast", &["addr", "addr", "int"], "bool"),
            runtime_api_function("processLastError", &[], "int"),
        ];
    }
    if let Some(session) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.session")
    }) {
        session.functions = vec![
            runtime_api_function("sessionWideAlloc", &["str"], "addr"),
            runtime_api_function("sessionWideFree", &["addr"], "bool"),
            runtime_api_function("sessionCurrentId", &[], "int"),
            runtime_api_function("sessionActiveConsoleId", &[], "int"),
            runtime_api_function("sessionProcessId", &["int", "addr"], "bool"),
            runtime_api_function("sessionServerOpen", &["addr"], "addr"),
            runtime_api_function("sessionServerClose", &["addr"], "bool"),
            runtime_api_function("sessionEnumerate", &["addr", "addr", "addr"], "bool"),
            runtime_api_function(
                "sessionQuery",
                &["addr", "int", "int", "addr", "addr"],
                "bool",
            ),
            runtime_api_function("sessionFree", &["addr"], "bool"),
            runtime_api_function("sessionQueryUserToken", &["int", "addr"], "bool"),
            runtime_api_function(
                "sessionSendMessage",
                &[
                    "addr", "int", "addr", "int", "addr", "int", "int", "int", "addr", "bool",
                ],
                "bool",
            ),
            runtime_api_function("sessionDisconnect", &["addr", "int", "bool"], "bool"),
            runtime_api_function("sessionLogoff", &["addr", "int", "bool"], "bool"),
            runtime_api_function("sessionConnect", &["int", "int", "addr", "bool"], "bool"),
            runtime_api_function("sessionRegisterNotification", &["addr", "int"], "bool"),
            runtime_api_function("sessionUnregisterNotification", &["addr"], "bool"),
            runtime_api_function("sessionWaitEvent", &["addr", "int", "addr"], "bool"),
            runtime_api_function("sessionChannelOpen", &["int", "str", "int"], "addr"),
            runtime_api_function("sessionChannelClose", &["addr"], "bool"),
            runtime_api_function(
                "sessionChannelRead",
                &["addr", "int", "addr", "int", "addr"],
                "bool",
            ),
            runtime_api_function(
                "sessionChannelWrite",
                &["addr", "addr", "int", "addr"],
                "bool",
            ),
            runtime_api_function(
                "sessionChannelQuery",
                &["addr", "int", "addr", "addr"],
                "bool",
            ),
            runtime_api_function("sessionChannelPurgeInput", &["addr"], "bool"),
            runtime_api_function("sessionChannelPurgeOutput", &["addr"], "bool"),
            runtime_api_function("sessionLastError", &[], "int"),
        ];
    }
    if let Some(signal) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.signal"))
    {
        signal.functions = vec![
            runtime_api_function("signalWideAlloc", &["str"], "addr"),
            runtime_api_function("signalWideFree", &["addr"], "bool"),
            runtime_api_function("signalCtrlHandler", &["addr", "bool"], "bool"),
            runtime_api_function("signalCtrlEvent", &["int", "int"], "bool"),
            runtime_api_function(
                "signalCreateEvent",
                &["addr", "bool", "bool", "addr"],
                "addr",
            ),
            runtime_api_function("signalOpenEvent", &["int", "bool", "addr"], "addr"),
            runtime_api_function("signalSetEvent", &["addr"], "bool"),
            runtime_api_function("signalResetEvent", &["addr"], "bool"),
            runtime_api_function("signalPulseEvent", &["addr"], "bool"),
            runtime_api_function("signalWaitOne", &["addr", "int"], "int"),
            runtime_api_function("signalWaitOneEx", &["addr", "int", "bool"], "int"),
            runtime_api_function("signalWaitMany", &["int", "addr", "bool", "int"], "int"),
            runtime_api_function(
                "signalWaitManyEx",
                &["int", "addr", "bool", "int", "bool"],
                "int",
            ),
            runtime_api_function("signalQueueApc", &["addr", "addr", "int"], "int"),
            runtime_api_function("signalAddVectored", &["int", "addr"], "addr"),
            runtime_api_function("signalRemoveVectored", &["addr"], "bool"),
            runtime_api_function("signalSetUnhandledFilter", &["addr"], "addr"),
            runtime_api_function(
                "signalRaiseException",
                &["int", "int", "int", "addr"],
                "bool",
            ),
            runtime_api_function("signalClose", &["addr"], "bool"),
            runtime_api_function("signalLastError", &[], "int"),
        ];
    }
    if let Some(temp) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.temp"))
    {
        temp.functions = vec![
            runtime_api_function("tempWideAlloc", &["str"], "addr"),
            runtime_api_function("tempWideBuffer", &["int"], "addr"),
            runtime_api_function("tempWideFree", &["addr"], "bool"),
            runtime_api_function("tempDirectory", &["addr", "int"], "int"),
            runtime_api_function("tempFileName", &["addr", "addr", "int", "addr"], "int"),
            runtime_api_function(
                "tempOpen",
                &["addr", "int", "int", "addr", "int", "int", "addr"],
                "addr",
            ),
            runtime_api_function("tempDeleteFile", &["addr"], "bool"),
            runtime_api_function("tempCreateDirectory", &["addr", "addr"], "bool"),
            runtime_api_function("tempRemoveDirectory", &["addr"], "bool"),
            runtime_api_function("tempMove", &["addr", "addr", "int"], "bool"),
            runtime_api_function("tempAttributes", &["addr"], "int"),
            runtime_api_function("tempSetAttributes", &["addr", "int"], "bool"),
            runtime_api_function("tempFinalPath", &["addr", "addr", "int", "int"], "int"),
            runtime_api_function("tempClose", &["addr"], "bool"),
            runtime_api_function("tempLastError", &[], "int"),
        ];
    }
    if let Some(user) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.user"))
    {
        user.functions = vec![
            runtime_api_function("userBuffer", &["int"], "addr"),
            runtime_api_function("userFree", &["addr"], "bool"),
            runtime_api_function("userCurrentProcess", &[], "addr"),
            runtime_api_function("userName", &["addr", "addr"], "bool"),
            runtime_api_function("userNameEx", &["int", "addr", "addr"], "bool"),
            runtime_api_function("userProfileDirectory", &["addr", "addr", "addr"], "bool"),
            runtime_api_function("userOpenProcessToken", &["addr", "int", "addr"], "bool"),
            runtime_api_function(
                "userTokenInformation",
                &["addr", "int", "addr", "int", "addr"],
                "bool",
            ),
            runtime_api_function("userCheckMembership", &["addr", "addr", "addr"], "bool"),
            runtime_api_function(
                "userCreateWellKnownSid",
                &["int", "addr", "addr", "addr"],
                "bool",
            ),
            runtime_api_function("userDuplicateToken", &["addr", "int", "addr"], "bool"),
            runtime_api_function("userSetThreadToken", &["addr", "addr"], "bool"),
            runtime_api_function("userImpersonate", &["addr"], "bool"),
            runtime_api_function("userRevert", &[], "bool"),
            runtime_api_function("userLoadProfile", &["addr", "addr"], "bool"),
            runtime_api_function("userUnloadProfile", &["addr", "addr"], "bool"),
            runtime_api_function("userClose", &["addr"], "bool"),
            runtime_api_function("userLastError", &[], "int"),
        ];
    }
    if let Some(event) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.event"))
    {
        event.functions = vec![
            runtime_api_function("eventCreate", &["addr", "bool", "bool", "addr"], "addr"),
            runtime_api_function("eventCreateEx", &["addr", "addr", "int", "int"], "addr"),
            runtime_api_function("eventOpen", &["int", "bool", "addr"], "addr"),
            runtime_api_function("eventSet", &["addr"], "bool"),
            runtime_api_function("eventReset", &["addr"], "bool"),
            runtime_api_function("eventPulse", &["addr"], "bool"),
            runtime_api_function("eventWait", &["addr", "int"], "int"),
            runtime_api_function("eventWaitEx", &["addr", "int", "bool"], "int"),
            runtime_api_function("eventWaitMany", &["int", "addr", "bool", "int"], "int"),
            runtime_api_function(
                "eventWaitManyEx",
                &["int", "addr", "bool", "int", "bool"],
                "int",
            ),
            runtime_api_function(
                "eventSignalAndWait",
                &["addr", "addr", "int", "bool"],
                "int",
            ),
            runtime_api_function("eventClose", &["addr"], "bool"),
            runtime_api_function("eventLastError", &[], "int"),
        ];
    }
    if let Some(assertions) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.assert"))
    {
        assertions.functions = vec![
            runtime_api_function("assert", &["bool"], "bool"),
            runtime_api_function("assertMsg", &["bool", "addr", "int"], "bool"),
            runtime_api_function("assertEq", &["int", "int"], "bool"),
            runtime_api_function("assertEqAddr", &["addr", "addr"], "bool"),
            runtime_api_function("assertNotNull", &["addr"], "bool"),
            runtime_api_function("assertRange", &["int", "int", "int"], "bool"),
            runtime_api_function("assertFail", &[], "bool"),
            runtime_api_function("assertUnreachable", &[], "bool"),
        ];
    }
    if let Some(bridge) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.bridge"))
    {
        bridge.functions = vec![
            runtime_api_function("bridgeStride", &[], "int"),
            runtime_api_function(
                "bridgeWrite",
                &["addr", "int", "int", "addr", "int", "int"],
                "bool",
            ),
            runtime_api_function("bridgeFind", &["addr", "int", "int"], "int"),
            runtime_api_function("bridgeActive", &["addr", "int", "int"], "bool"),
            runtime_api_function("bridgeRequire", &["addr", "int", "int"], "int"),
            runtime_api_function("bridgeTarget", &["addr", "int"], "addr"),
            runtime_api_function("bridgeMode", &["addr", "int"], "int"),
            runtime_api_function("bridgeAuthority", &["addr", "int"], "int"),
            runtime_api_function("bridgeErase", &["addr", "int"], "bool"),
        ];
    }
    if let Some(debug) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.debug"))
    {
        debug.functions = vec![
            runtime_api_function("debugStride", &[], "int"),
            runtime_api_function(
                "debugRecord",
                &["addr", "int", "int", "addr", "int", "int"],
                "bool",
            ),
            runtime_api_function("debugTag", &["addr", "int"], "int"),
            runtime_api_function("debugLocation", &["addr", "int"], "addr"),
            runtime_api_function("debugValue", &["addr", "int"], "int"),
            runtime_api_function("debugFlags", &["addr", "int"], "int"),
            runtime_api_function("debugDump", &["addr", "addr", "int"], "int"),
            runtime_api_function("debugChecksum", &["addr", "int"], "int"),
            runtime_api_function("debugBreak", &[], "bool"),
            runtime_api_function("debugAssert", &["bool", "addr", "int"], "bool"),
        ];
    }
    if let Some(diagnostic) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.diagnostic")
    }) {
        diagnostic.functions = vec![
            runtime_api_function("diagStride", &[], "int"),
            runtime_api_function(
                "diagWrite",
                &[
                    "addr", "int", "int", "int", "addr", "int", "addr", "int", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("diagSeverity", &["addr", "int"], "int"),
            runtime_api_function("diagCode", &["addr", "int"], "int"),
            runtime_api_function("diagMessage", &["addr", "int"], "addr"),
            runtime_api_function("diagMessageBytes", &["addr", "int"], "int"),
            runtime_api_function("diagSource", &["addr", "int"], "addr"),
            runtime_api_function("diagLine", &["addr", "int"], "int"),
            runtime_api_function("diagColumn", &["addr", "int"], "int"),
            runtime_api_function("diagFlags", &["addr", "int"], "int"),
            runtime_api_function("diagCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("diagClear", &["addr", "int"], "bool"),
        ];
    }
    if let Some(log) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.log"))
    {
        log.functions = vec![
            runtime_api_function("logStride", &[], "int"),
            runtime_api_function(
                "logWrite",
                &["addr", "int", "int", "int", "addr", "int", "int", "int"],
                "bool",
            ),
            runtime_api_function("logLevel", &["addr", "int"], "int"),
            runtime_api_function("logCategory", &["addr", "int"], "int"),
            runtime_api_function("logMessage", &["addr", "int"], "addr"),
            runtime_api_function("logMessageBytes", &["addr", "int"], "int"),
            runtime_api_function("logValue", &["addr", "int"], "int"),
            runtime_api_function("logFlags", &["addr", "int"], "int"),
            runtime_api_function("logCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("logClear", &["addr", "int"], "bool"),
        ];
    }
    if let Some(profile) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.profile")
    }) {
        profile.functions = vec![
            runtime_api_function("profileStride", &[], "int"),
            runtime_api_function(
                "profileWrite",
                &["addr", "int", "int", "int", "int", "int", "addr", "int"],
                "bool",
            ),
            runtime_api_function("profileMode", &["addr", "int"], "int"),
            runtime_api_function("profileFlags", &["addr", "int"], "int"),
            runtime_api_function("profileCpuFeatures", &["addr", "int"], "int"),
            runtime_api_function("profileOptimization", &["addr", "int"], "int"),
            runtime_api_function("profileTarget", &["addr", "int"], "addr"),
            runtime_api_function("profileTargetBytes", &["addr", "int"], "int"),
            runtime_api_function("profileHas", &["addr", "int", "int"], "bool"),
            runtime_api_function("profileRequire", &["addr", "int", "int"], "bool"),
            runtime_api_function("profileCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("profileClear", &["addr", "int"], "bool"),
        ];
    }
    if let Some(status) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.status"))
    {
        status.functions = vec![
            runtime_api_function("statusStride", &[], "int"),
            runtime_api_function(
                "statusWrite",
                &["addr", "int", "int", "int", "addr", "int", "int"],
                "bool",
            ),
            runtime_api_function("statusKind", &["addr", "int"], "int"),
            runtime_api_function("statusCode", &["addr", "int"], "int"),
            runtime_api_function("statusMessage", &["addr", "int"], "addr"),
            runtime_api_function("statusMessageBytes", &["addr", "int"], "int"),
            runtime_api_function("statusDetail", &["addr", "int"], "int"),
            runtime_api_function("statusCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("statusClear", &["addr", "int"], "bool"),
        ];
    }
    if let Some(trace) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.trace"))
    {
        trace.functions = vec![
            runtime_api_function("traceStride", &[], "int"),
            runtime_api_function(
                "traceWrite",
                &[
                    "addr", "int", "int", "int", "int", "int", "addr", "int", "int",
                ],
                "bool",
            ),
            runtime_api_function("traceKind", &["addr", "int"], "int"),
            runtime_api_function("traceId", &["addr", "int"], "int"),
            runtime_api_function("traceParent", &["addr", "int"], "int"),
            runtime_api_function("traceTick", &["addr", "int"], "int"),
            runtime_api_function("tracePayload", &["addr", "int"], "addr"),
            runtime_api_function("tracePayloadBytes", &["addr", "int"], "int"),
            runtime_api_function("traceValue", &["addr", "int"], "int"),
            runtime_api_function("traceCopy", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("traceClear", &["addr", "int"], "bool"),
        ];
    }
    if let Some(validate) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.validate")
    }) {
        validate.functions = vec![
            runtime_api_function("validateNotNull", &["addr"], "bool"),
            runtime_api_function("validateRange", &["int", "int", "int"], "bool"),
            runtime_api_function("validateLength", &["int", "int", "int"], "bool"),
            runtime_api_function("validateEqual", &["addr", "int", "addr", "int"], "bool"),
            runtime_api_function("validateAscii", &["addr", "int"], "bool"),
            runtime_api_function("validateUtf8", &["addr", "int"], "bool"),
            runtime_api_function("validateAligned", &["addr", "int"], "bool"),
            runtime_api_function("validatePowerOfTwo", &["int"], "bool"),
            runtime_api_function("validateZero", &["addr", "int"], "bool"),
            runtime_api_function("validateMask", &["int", "int", "int"], "bool"),
        ];
    }
    if let Some(service) = families.iter_mut().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case("nC.runtime.service")
    }) {
        service.functions = vec![
            runtime_api_function("serviceWideAlloc", &["str"], "addr"),
            runtime_api_function("serviceWideFree", &["addr"], "bool"),
            runtime_api_function("serviceManagerOpen", &["addr", "addr", "int"], "addr"),
            runtime_api_function("serviceOpen", &["addr", "addr", "int"], "addr"),
            runtime_api_function(
                "serviceCreate",
                &[
                    "addr", "addr", "addr", "int", "int", "int", "int", "addr", "addr", "addr",
                    "addr", "addr", "addr",
                ],
                "addr",
            ),
            runtime_api_function("serviceDelete", &["addr"], "bool"),
            runtime_api_function("serviceStart", &["addr", "int", "addr"], "bool"),
            runtime_api_function("serviceControl", &["addr", "int", "addr"], "bool"),
            runtime_api_function("serviceControlEx", &["addr", "int", "int", "addr"], "bool"),
            runtime_api_function(
                "serviceQueryStatus",
                &["addr", "int", "addr", "int", "addr"],
                "bool",
            ),
            runtime_api_function(
                "serviceQueryConfig",
                &["addr", "addr", "int", "addr"],
                "bool",
            ),
            runtime_api_function(
                "serviceChangeConfig",
                &[
                    "addr", "int", "int", "int", "addr", "addr", "addr", "addr", "addr", "addr",
                    "addr",
                ],
                "bool",
            ),
            runtime_api_function(
                "serviceEnum",
                &[
                    "addr", "int", "int", "int", "addr", "int", "addr", "addr", "addr", "addr",
                ],
                "bool",
            ),
            runtime_api_function(
                "serviceEnumDependents",
                &["addr", "int", "addr", "int", "addr", "addr"],
                "bool",
            ),
            runtime_api_function("serviceKeyName", &["addr", "addr", "addr", "addr"], "bool"),
            runtime_api_function(
                "serviceDisplayName",
                &["addr", "addr", "addr", "addr"],
                "bool",
            ),
            runtime_api_function("serviceManagerLock", &["addr"], "addr"),
            runtime_api_function("serviceManagerUnlock", &["addr"], "bool"),
            runtime_api_function(
                "serviceManagerLockStatus",
                &["addr", "addr", "int", "addr"],
                "bool",
            ),
            runtime_api_function("serviceNotify", &["addr", "int", "addr"], "int"),
            runtime_api_function("serviceStartDispatcher", &["addr"], "bool"),
            runtime_api_function("serviceRegisterHandler", &["addr", "addr", "addr"], "addr"),
            runtime_api_function("serviceSetStatus", &["addr", "addr"], "bool"),
            runtime_api_function("serviceClose", &["addr"], "bool"),
            runtime_api_function("serviceLastError", &[], "int"),
        ];
    }
    if let Some(fs) = families
        .iter_mut()
        .find(|family| family.import_path.eq_ignore_ascii_case("nC.runtime.fs"))
    {
        if let Some(fswrite) = fs
            .functions
            .iter_mut()
            .find(|function| function.name.eq_ignore_ascii_case("fswrite"))
        {
            fswrite.return_ty = "bool".to_string();
        }
        fs.functions.push(RuntimeApiFunctionSpec {
            name: "fswriteResult".to_string(),
            arg_types: vec!["str".to_string(), "str".to_string()],
            arity_min: 2,
            variadic: false,
            return_ty: "Result<bool, FsError>".to_string(),
        });
        fs.functions.push(RuntimeApiFunctionSpec {
            name: "fread_lines".to_string(),
            arg_types: vec!["str".to_string()],
            arity_min: 1,
            variadic: false,
            return_ty: "array<str>".to_string(),
        });
        fs.functions.extend([
            RuntimeApiFunctionSpec {
                name: "fexists".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "delete_file".to_string(),
                arg_types: vec!["str".to_string()],
                arity_min: 1,
                variadic: false,
                return_ty: "bool".to_string(),
            },
            RuntimeApiFunctionSpec {
                name: "rename_file".to_string(),
                arg_types: vec!["str".to_string(), "str".to_string()],
                arity_min: 2,
                variadic: false,
                return_ty: "bool".to_string(),
            },
        ]);
    }
    families
}

pub fn runtime_api_find_family_by_source_module_path(
    path: &[String],
) -> Option<&'static RuntimeApiFamilySpec> {
    if let Some(family) = runtime_api_find_family_by_module_path(path) {
        return Some(family);
    }
    let alias = path.join(".");
    let import_path = match alias.to_ascii_lowercase().as_str() {
        "telephony" => "nC.runtime.telephony",
        "tts_engine" | "ttsengine" => "nC.runtime.tts",
        "network" => "nC.runtime.mobileNetwork",
        "permissions" => "nC.runtime.permission",
        "get_current_time" | "getcurrenttime" => "nC.runtime.time",
        "cpu_load" | "cpuload" | "registry_write" | "registrywrite" | "registry_read"
        | "registryread" => "nC.runtime.system",
        _ => return None,
    };
    runtime_api_family_specs()
        .iter()
        .find(|family| family.import_path.eq_ignore_ascii_case(import_path))
}

pub fn runtime_api_source_module_is_implicit(path: &[String]) -> bool {
    runtime_api_find_family_by_source_module_path(path).is_some_and(|family| {
        matches!(
            family.import_path.to_ascii_lowercase().as_str(),
            "nc.runtime.telephony"
                | "nc.runtime.tts"
                | "nc.runtime.mobilenetwork"
                | "nc.runtime.permission"
                | "nc.runtime.system"
                | "nc.runtime.time"
        )
    })
}

pub fn runtime_prelude_integer_constant(name: &str) -> Option<i64> {
    Some(match name.to_ascii_uppercase().as_str() {
        "PERM_PHONE" => 1 << 0,
        "PERM_NET" | "PERM_NETWORK" => 1 << 1,
        "PERM_MIC" | "PERM_MICROPHONE" => 1 << 2,
        "PERM_LOCATION" => 1 << 3,
        "PERM_CONTACTS" => 1 << 4,
        "PERM_SMS" => 1 << 5,
        "PERM_CAMERA" => 1 << 6,
        "PERM_STORAGE" => 1 << 7,
        "PERM_NOTIFICATIONS" => 1 << 8,
        _ => return None,
    })
}

pub fn runtime_api_family_specs() -> &'static [RuntimeApiFamilySpec] {
    RUNTIME_API_FAMILY_SPECS_CACHE
        .get_or_init(generated_runtime_api_family_specs)
        .as_slice()
}

pub fn runtime_api_find_family_by_import_path(
    path: &[String],
) -> Option<&'static RuntimeApiFamilySpec> {
    let joined = path.join(".");
    runtime_api_family_specs()
        .iter()
        .find(|fam| segment_eq_ci(&fam.import_path, &joined))
}

pub fn runtime_api_find_family_by_module_path(
    path: &[String],
) -> Option<&'static RuntimeApiFamilySpec> {
    let joined = path.join(".");
    runtime_api_family_specs()
        .iter()
        .find(|fam| segment_eq_ci(&fam.module_path, &joined))
}

pub fn runtime_api_find_function_by_module_path(
    path: &[String],
    fn_name: &str,
) -> Option<&'static RuntimeApiFunctionSpec> {
    let fam = runtime_api_find_family_by_module_path(path)?;
    fam.functions
        .iter()
        .find(|sig| segment_eq_ci(&sig.name, fn_name))
}

fn runtime_symbol_name_without_generics(name: &str) -> &str {
    name.split_once('<').map_or(name, |(base, _)| base)
}

/// Returns the one stable linker identity for every callable runtime symbol.
/// Source-level module aliases remain valid, but import/export manifests and
/// backend edges always use `nC.runtime.*` and never encode generic arguments
/// in the symbol spelling.
pub fn canonical_runtime_symbol(symbol: &str) -> String {
    let symbol = symbol.trim();
    let canonical = if symbol
        .get(..11)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("nC.runtime."))
    {
        format!("nC.runtime.{}", &symbol[11..])
    } else if symbol
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("nC.math."))
    {
        format!("nC.runtime.math.{}", &symbol[8..])
    } else if symbol
        .get(..10)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("nC.vector."))
    {
        format!("nC.runtime.vector.{}", &symbol[10..])
    } else {
        return symbol.to_string();
    };

    let Some((prefix, tail)) = canonical.rsplit_once('.') else {
        return canonical;
    };
    format!("{prefix}.{}", runtime_symbol_name_without_generics(tail))
}

pub fn runtime_api_find_function_by_runtime_symbol(
    symbol: &str,
) -> Option<(
    &'static RuntimeApiFamilySpec,
    &'static RuntimeApiFunctionSpec,
)> {
    let canonical = canonical_runtime_symbol(symbol);
    let tail = canonical.strip_prefix("nC.runtime.")?;
    let (family_name, function_name) = tail.split_once('.')?;
    let function_name = function_name.rsplit('.').next()?;
    let family = runtime_api_family_specs().iter().find(|family| {
        family
            .import_path
            .eq_ignore_ascii_case(&format!("nC.runtime.{family_name}"))
    })?;
    let function = family.functions.iter().find(|function| {
        runtime_symbol_name_without_generics(&function.name)
            .eq_ignore_ascii_case(runtime_symbol_name_without_generics(function_name))
    })?;
    Some((family, function))
}

fn runtime_api_function_accepts_arity(function: &RuntimeApiFunctionSpec, arity: usize) -> bool {
    if function.variadic {
        arity >= function.arity_min
    } else {
        arity == function.arity_min
    }
}

fn hosted_runtime_prelude_name(raw: &str) -> &str {
    runtime_symbol_name_without_generics(raw.trim())
}

pub fn native_prelude_resolve_function(
    fn_name: &str,
    arity: usize,
) -> Result<Option<&'static NativePreludeFunctionSpec>, UseDiagnostic> {
    let wanted = hosted_runtime_prelude_name(fn_name);
    let Some(function) = NATIVE_PRELUDE_V1_FUNCTIONS
        .iter()
        .find(|function| function.source_name.eq_ignore_ascii_case(wanted))
    else {
        return Ok(None);
    };

    if function.accepted_arities.contains(&arity) {
        return Ok(Some(function));
    }

    let expected = function
        .accepted_arities
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(" or ");
    let help = if wanted.eq_ignore_ascii_case("alloc") && arity == 2 {
        "alloc(bytes) uses the platform's default alignment; use alignedAlloc(bytes, alignment) and release it with alignedFree"
    } else {
        "use the canonical prelude signature or declare a local function"
    };
    Err(UseDiagnostic {
        code: "E_NATIVE_PRELUDE_ARITY_001",
        message: format!(
            "native prelude call '{}' expects {} argument(s), got {}",
            fn_name, expected, arity
        ),
        help: Some(help),
    })
}

/// Resolves an unqualified hosted-prelude call from the runtime API catalog.
///
/// `Ok(None)` means that the name does not belong to the prelude. A known name
/// with the wrong arity and an ambiguous name/arity pair are diagnostics, so
/// callers cannot silently select a different family as the catalog grows.
pub fn hosted_runtime_prelude_resolve_function(
    fn_name: &str,
    arity: usize,
) -> Result<Option<HostedRuntimePreludeFunction>, UseDiagnostic> {
    let wanted = hosted_runtime_prelude_name(fn_name);
    let mut named = Vec::new();
    let mut matching = Vec::new();

    for import_path in HOSTED_RUNTIME_PRELUDE_V1_FAMILIES {
        let path = import_path
            .split('.')
            .map(str::to_string)
            .collect::<Vec<_>>();
        let Some(family) = runtime_api_find_family_by_import_path(&path) else {
            continue;
        };
        for function in &family.functions {
            if hosted_runtime_prelude_name(&function.name).eq_ignore_ascii_case(wanted) {
                named.push((family, function));
                if runtime_api_function_accepts_arity(function, arity) {
                    matching.push((family, function));
                }
            }
        }
    }

    if named.is_empty() {
        return Ok(None);
    }

    if matching.is_empty() {
        let mut expected = named
            .iter()
            .map(|(family, function)| {
                let arity = if function.variadic {
                    format!("{}+", function.arity_min)
                } else {
                    function.arity_min.to_string()
                };
                format!("{}.{}({arity})", family.import_path, function.name)
            })
            .collect::<Vec<_>>();
        expected.sort();
        expected.dedup();
        return Err(UseDiagnostic {
            code: "E_RUNTIME_PRELUDE_ARITY_001",
            message: format!(
                "hosted runtime prelude call '{}' does not accept {} argument(s); candidates: {}",
                fn_name,
                arity,
                expected.join(", ")
            ),
            help: Some(
                "use the catalog signature or declare a local function with a distinct name",
            ),
        });
    }

    matching.sort_by(
        |(left_family, left_function), (right_family, right_function)| {
            left_family
                .import_path
                .cmp(&right_family.import_path)
                .then_with(|| left_function.name.cmp(&right_function.name))
        },
    );
    matching.dedup_by(
        |(left_family, left_function), (right_family, right_function)| {
            left_family
                .import_path
                .eq_ignore_ascii_case(&right_family.import_path)
                && hosted_runtime_prelude_name(&left_function.name)
                    .eq_ignore_ascii_case(hosted_runtime_prelude_name(&right_function.name))
                && left_function.arity_min == right_function.arity_min
                && left_function.variadic == right_function.variadic
        },
    );

    if matching.len() > 1 {
        let candidates = matching
            .iter()
            .map(|(family, function)| format!("{}.{}", family.import_path, function.name))
            .collect::<Vec<_>>();
        return Err(UseDiagnostic {
            code: "E_RUNTIME_PRELUDE_AMBIGUOUS_001",
            message: format!(
                "hosted runtime prelude call '{}' with {} argument(s) is ambiguous between {}",
                fn_name,
                arity,
                candidates.join(", ")
            ),
            help: Some("qualify the runtime family explicitly"),
        });
    }

    let (family, function) = matching[0];
    let canonical_symbol = canonical_runtime_symbol(&format!(
        "{}.{}",
        family.import_path,
        hosted_runtime_prelude_name(&function.name)
    ));
    Ok(Some(HostedRuntimePreludeFunction {
        family,
        function,
        canonical_symbol,
    }))
}

fn import_path_matches_dotted(path: &[String], dotted: &str) -> bool {
    let expected: Vec<&str> = dotted.split('.').collect();
    import_path_matches(path, &expected)
}

pub const NC_RUNTIME_FAMILIES: &[&str] = &[
    "nC.runtime.input",
    "nC.runtime.output",
    "nC.runtime.errorOutput",
    "nC.runtime.channel",
    "nC.runtime.flush",
    "nC.runtime.std",
    "nC.runtime.args",
    "nC.runtime.env",
    "nC.runtime.exit",
    "nC.runtime.session",
    "nC.runtime.text",
    "nC.runtime.string",
    "nC.runtime.render",
    "nC.runtime.math",
    "nC.runtime.format",
    "nC.runtime.concat",
    "nC.runtime.parse",
    "nC.runtime.convert",
    "nC.runtime.validate",
    "nC.runtime.json",
    "nC.runtime.width",
    "nC.runtime.align",
    "nC.runtime.pad",
    "nC.runtime.truncate",
    "nC.runtime.wrap",
    "nC.runtime.escape",
    "nC.runtime.unicode",
    "nC.runtime.result",
    "nC.runtime.option",
    "nC.runtime.error",
    "nC.runtime.diagnostic",
    "nC.runtime.log",
    "nC.runtime.trace",
    "nC.runtime.debug",
    "nC.runtime.assert",
    "nC.runtime.status",
    "nC.runtime.buffer",
    "nC.runtime.memory",
    "nC.runtime.heap",
    "nC.runtime.stack",
    "nC.runtime.region",
    "nC.runtime.slice",
    "nC.runtime.span",
    "nC.runtime.ptr",
    "nC.runtime.addr",
    "nC.runtime.bytes",
    "nC.runtime.endian",
    "nC.runtime.layout",
    "nC.runtime.alloc",
    "nC.runtime.fs",
    "nC.runtime.path",
    "nC.runtime.dir",
    "nC.runtime.file",
    "nC.runtime.stream",
    "nC.runtime.process",
    "nC.runtime.command",
    "nC.runtime.os",
    "nC.runtime.user",
    "nC.runtime.permission",
    "nC.runtime.telephony",
    "nC.runtime.tts",
    "nC.runtime.mobileNetwork",
    "nC.runtime.temp",
    "nC.runtime.service",
    "nC.runtime.signal",
    "nC.runtime.time",
    "nC.runtime.terminal",
    "nC.runtime.clock",
    "nC.runtime.timer",
    "nC.runtime.sleep",
    "nC.runtime.random",
    "nC.runtime.entropy",
    "nC.runtime.uuid",
    "nC.runtime.task",
    "nC.runtime.thread",
    "nC.runtime.async",
    "nC.runtime.sync",
    "nC.runtime.atomic",
    "nC.runtime.lock",
    "nC.runtime.queue",
    "nC.runtime.event",
    "nC.runtime.scheduler",
    "nC.runtime.cancel",
    "nC.runtime.net",
    "nC.runtime.socket",
    "nC.runtime.tcp",
    "nC.runtime.udp",
    "nC.runtime.dns",
    "nC.runtime.http",
    "nC.runtime.tls",
    "nC.runtime.capability",
    "nC.runtime.policy",
    "nC.runtime.profile",
    "nC.runtime.bridge",
    "nC.runtime.sandbox",
    "nC.runtime.resource",
];

pub fn official_runtime_families() -> &'static [&'static str] {
    NC_RUNTIME_FAMILIES
}

const KNOWN_USE_IMPORT_PATHS: &[&str] = &[
    "alt.net",
    "alt.net.core",
    "ai.agent.core",
    "ai.chat.core",
    "ai.memory.core",
    "ai.model.core",
    "ai.session.core",
    "Compiler.backend.coff",
    "Compiler.backend.elf",
    "Compiler.backend.machine",
    "Compiler.backend.macho",
    "Compiler.backend.text",
    "Compiler.lex.core",
    "Compiler.lex.scan",
    "Compiler.lex.stream",
    "Compiler.lex.token",
    "Compiler.lowering.core",
    "Compiler.lowering.machine",
    "Compiler.lowering.nir",
    "rodata.ncc",
    "Compiler.nir.core",
    "Compiler.nir.emit",
    "Compiler.nir.inspect",
    "Compiler.parse.ast",
    "Compiler.parse.grammar",
    "Compiler.parse.recover",
    "Compiler.parse.token",
    "Compiler.semantic.alias",
    "Compiler.semantic.core",
    "Compiler.semantic.flow",
    "Compiler.semantic.types",
    "kernel.control.core",
    "kernel.cpu.core",
    "kernel.boot.core",
    "kernel.driver.core",
    "kernel.interrupt.core",
    "kernel.ipi.core",
    "kernel.memory.core",
    "kernel.schedule.core",
    "kernel.syscall.core",
    "kernel.task.core",
    "kernel.user.core",
    "core.math",
    "core.mem.region",
    "Inspect.dump",
    "Inspect.event",
    "Inspect.graph",
    "Inspect.mem",
    "inspect.memoryInspector",
    "Inspect.probe",
    "Inspect.session",
    "Inspect.snapshot",
    "Inspect.timeline",
    "Inspect.trace",
    "inspect.trace",
    "Inspect.watch",
    "foreign.asm",
    "foreign.c",
    "foreign.cpp",
    "foreign.rust",
    "Machine.atomic",
    "Machine.boot",
    "Machine.call",
    "Machine.decode",
    "Machine.encode",
    "Machine.flag",
    "Machine.frame",
    "Machine.hazard",
    "Machine.interrupt",
    "machine.memoryScanner",
    "Machine.memview",
    "Machine.mmio",
    "Machine.pipeline",
    "Machine.port",
    "Machine.reg",
    "Machine.stack",
    "Machine.syscall",
    "memory.heap",
    "memory.heap.manager",
    "nC.heap.core",
    "nC.high.comptime",
    "nC.high.consteval",
    "nC.high.macros",
    "nC.high.metadata",
    "nC.high.typeinfo",
    "nC.compiler",
    "nC.compiler.parse",
    "nC.compiler.semantic",
    "nC.compiler.backend",
    "nC.ai",
    "nC.ai.agent",
    "nC.ai.chat",
    "nC.ai.memory",
    "nC.ai.model",
    "nC.ai.session",
    "nC.inspect.backend.bpf",
    "nC.inspect.backend.dtrace",
    "nC.inspect.backend.etw",
    "nC.inspect.backend.lttng",
    "nC.inspect.backend.otlp",
    "nC.inspect.backend.perf",
    "nC.inspect.redact.policy",
    "nC.inspect.redact.rules",
    "nC.inspect.session.core",
    "nC.inspect.session.timeline",
    "nC.inspect.signal.event",
    "nC.inspect.signal.log",
    "nC.inspect.signal.metric",
    "nC.inspect.signal.profile",
    "nC.inspect.signal.trace",
    "nC.kernel",
    "nC.kernel.boot",
    "nC.kernel.control",
    "nC.kernel.cpu",
    "nC.kernel.driver",
    "nC.kernel.interrupt",
    "nC.kernel.ipi",
    "nC.kernel.memory",
    "nC.kernel.schedule",
    "nC.kernel.syscall",
    "nC.kernel.task",
    "nC.kernel.user",
    "nC.list",
    "nC.list.arrayList",
    "nC.list.branchList",
    "nC.list.cacheList",
    "nC.list.chainList",
    "nC.list.core",
    "nC.list.cubeList",
    "nC.list.dbList",
    "nC.list.deque",
    "nC.list.flowList",
    "nC.list.index",
    "nC.list.indexList",
    "nC.list.jumpList",
    "nC.list.linkedList",
    "nC.list.list",
    "nC.list.memList",
    "nC.list.memory",
    "nC.list.memRegion",
    "nC.list.nodeList",
    "nC.list.packetList",
    "nC.list.priorityList",
    "nC.list.queue",
    "nC.list.rawList",
    "nC.list.ringList",
    "nC.list.routeList",
    "nC.list.safeList",
    "nC.list.sphereList",
    "nC.list.stack",
    "nC.list.traceList",
    "nC.list.watchList",
    "nC.math.core",
    "nC.math.algebra",
    "nC.math.trig",
    "nC.math.geometry",
    "nC.math.vector",
    "nC.math.matrix",
    "nC.math.tensor",
    "nC.math.stats",
    "nC.math.prob",
    "nC.math.calc",
    "nC.math.numeric",
    "nC.math.complex",
    "nC.math.discrete",
    "nC.math.physics",
    "nC.mem.addr",
    "nC.mem.alias",
    "nC.mem.bind",
    "nC.mem.guard",
    "nC.mem.map",
    "nC.mem.ptr",
    "nC.mem.region",
    "nC.mem.window",
    "nC.policy.core",
    "nC.policy.heap",
    "nC.runtime",
    "nC.runtime.alias",
    "nC.runtime.event",
    "nC.runtime.heap",
    "nC.runtime.array",
    "nC.runtime.net",
    "nC.runtime.queue",
    "nC.runtime.list",
    "nC.runtime.list.core",
    "nC.runtime.list.index",
    "nC.runtime.list.memory",
    "nC.runtime.region",
    "nC.runtime.sync",
    "nC.runtime.time",
    "nC.runtime.trace",
    "nC.std.io",
    "nC.system.dev",
    "nC.system.event",
    "nC.system.fs",
    "nC.system.ipc",
    "nC.system.mem",
    "nC.system.net",
    "nC.system.obs",
    "nC.system.proc",
    "nC.system.sec",
    "nC.system.thread",
    "nC.system.time",
    "nC.tooling.build",
    "nC.tooling.debug",
    "nC.tooling.doc",
    "nC.tooling.fmt",
    "nC.tooling.index",
    "nC.tooling.install",
    "nC.tooling.lint",
    "nC.tooling.pkg",
    "nC.tooling.plugin",
    "nC.tooling.policy",
    "nC.tooling.provenance",
    "nC.vector",
    "nC.vector.core",
    "nC.vector.math",
    "nC.vector.matrix",
    "nC.vector.region",
    "nC.vector.simd",
    "nC.vector.sphere",
    "nC.vector.tensor",
    "nC.vector.view",
    "project.mathCore",
    "project.runtimeCore",
    "Root.bind.anchor",
    "Root.bind.assert",
    "Root.bind.contract",
    "Root.bind.core",
    "Root.bind.emit",
    "Root.bind.guard",
    "Root.bind.latch",
    "Root.bind.map",
    "Root.bind.policy",
    "Root.bind.port",
    "Root.bind.region",
    "Root.bind.register",
    "Root.bind.sample",
    "Root.bind.signal",
    "Root.bind.state",
    "Root.bind.strict",
    "Root.bind.timing",
    "Root.bind.trace",
    "Root.bind.window",
    "Root.bind.wire",
    "Root.memory.addr",
    "Root.memory.atomic",
    "Root.memory.barrier",
    "Root.memory.bind",
    "Root.memory.bridge",
    "Root.memory.commit",
    "Root.memory.fallback",
    "Root.memory.fault",
    "Root.memory.frame",
    "Root.memory.guard",
    "Root.memory.hazard",
    "Root.memory.layout",
    "Root.memory.map",
    "Root.memory.mmio",
    "Root.memory.page",
    "Root.memory.policy",
    "Root.memory.ptr",
    "Root.memory.raw",
    "Root.memory.region",
    "Root.memory.register",
    "Root.memory.reset",
    "Root.memory.retry",
    "Root.memory.seal",
    "Root.memory.signal",
    "Root.memory.trace",
    "Root.memory.window",
    "Root.memory.zero",
    "Root.policy.base",
    "Root.policy.bind",
    "Root.policy.capability",
    "Root.policy.fallback",
    "Root.policy.guard",
    "Root.policy.inspect",
    "Root.policy.machine",
    "Root.policy.memory",
    "Root.policy.network",
    "Root.policy.primitive",
    "Root.policy.retry",
    "Root.policy.runtime",
    "Root.policy.security",
    "Root.policy.tooling",
    "Root.policy.trace",
    "Runtime.bootstrap.core",
    "Runtime.bootstrap.entry",
    "Runtime.bootstrap.handoff",
    "Runtime.bootstrap.loader",
    "Runtime.bootstrap.policy",
    "Runtime.buffer.channel",
    "Runtime.buffer.core",
    "Runtime.buffer.frame",
    "Runtime.buffer.packet",
    "Runtime.buffer.queue",
    "Runtime.buffer.ring",
    "Runtime.buffer.stream",
    "Runtime.buffer.window",
    "Runtime.error.core",
    "Runtime.error.fault",
    "Runtime.error.policy",
    "Runtime.error.recover",
    "Runtime.error.report",
    "Runtime.error.trace",
    "Runtime.heap.alloc",
    "Runtime.heap.arena",
    "Runtime.heap.audit",
    "Runtime.heap.free",
    "Runtime.heap.gc",
    "Runtime.heap.manager",
    "Runtime.heap.pool",
    "Runtime.heap.realloc",
    "Runtime.heap.region",
    "Runtime.heap.stats",
    "Runtime.heap.trace",
    "Runtime.heap.zero",
    "runtime.io",
    "runtime.io.completion",
    "runtime.io.driver",
    "runtime.io.poller",
    "runtime.io.reactor",
    "runtime.mem.arena",
    "runtime.mem.pool",
    "runtime.mem.region",
    "runtime.mem.shared",
    "runtime.policy",
    "runtime.policy.default",
    "runtime.policy.lowLatency",
    "runtime.policy.safe",
    "runtime.policy.strict",
    "runtime.proc.child",
    "Runtime.profile.core",
    "Runtime.profile.cpu",
    "Runtime.profile.io",
    "Runtime.profile.latency",
    "Runtime.profile.mem",
    "Runtime.profile.timeline",
    "runtime.sandbox.default",
    "runtime.supervisor.default",
    "runtime.sync",
    "Runtime.sync.barrier",
    "runtime.sync.barrier",
    "Runtime.sync.channel",
    "runtime.sync.channel",
    "Runtime.sync.core",
    "Runtime.sync.event",
    "runtime.sync.event",
    "Runtime.sync.latch",
    "Runtime.sync.mutex",
    "runtime.sync.mutex",
    "Runtime.sync.rwlock",
    "runtime.telemetry",
    "runtime.telemetry.core",
    "runtime.telemetry.event",
    "runtime.telemetry.metric",
    "runtime.telemetry.otel",
    "Runtime.text",
    "Runtime.text.ascii",
    "Runtime.text.core",
    "Runtime.text.decode",
    "Runtime.text.encode",
    "Runtime.text.format",
    "Runtime.text.scan",
    "Runtime.text.token",
    "Runtime.text.utf8",
    "runtime.time",
    "Runtime.time.clock",
    "runtime.time.clock",
    "Runtime.time.core",
    "Runtime.time.cycle",
    "Runtime.time.deadline",
    "runtime.time.deadline",
    "runtime.time.highres",
    "Runtime.time.sleep",
    "Runtime.time.timer",
    "Runtime.trace.audit",
    "Runtime.trace.core",
    "Runtime.trace.event",
    "Runtime.trace.mark",
    "Runtime.trace.pipe",
    "Runtime.trace.span",
    "Security.audit",
    "security.attack.core",
    "security.defense.core",
    "Security.crypto",
    "Security.crypto.block",
    "security.crypto.vault",
    "security.forensics.core",
    "Security.forensics",
    "Security.hash",
    "security.identity.core",
    "Security.identity",
    "security.intel.attack",
    "security.policy.core",
    "Security.policy",
    "security.policy.strict",
    "security.supply.core",
    "Security.sandbox",
    "security.sandbox",
    "security.sandbox.runtimeDefault",
    "Security.sign",
    "Security.tracekit",
    "security.vuln.core",
    "Security.vuln",
    "System.driver.block",
    "System.driver.bus",
    "System.driver.core",
    "System.driver.net",
    "System.driver.pci",
    "System.driver.usb",
    "System.fs.core",
    "System.fs.file",
    "System.fs.mount",
    "System.fs.path",
    "System.fs.watch",
    "System.io.core",
    "System.io.pipe",
    "System.io.serial",
    "System.io.stream",
    "system.net",
    "System.net.core",
    "system.net.core",
    "System.net.dns",
    "System.net.packet",
    "System.net.quic",
    "System.net.route",
    "System.net.socket",
    "System.net.stream",
    "System.net.tcp",
    "System.net.tls",
    "System.net.udp",
    "system.proc",
    "System.proc.child",
    "system.proc.child",
    "System.proc.core",
    "system.proc.core",
    "System.proc.exec",
    "System.proc.job",
    "System.proc.signal",
    "System.proc.spawn",
    "System.proc.wait",
    "System.service.core",
    "System.service.health",
    "System.service.manager",
    "System.service.registry",
    "System.socket.acceptor",
    "System.socket.client",
    "System.socket.core",
    "System.socket.packet",
    "System.socket.server",
    "System.socket.stream",
    "System.time.chrono",
    "System.time.clock",
    "System.time.deadline",
    "System.time.timer",
    "Tooling.build.core",
    "Tooling.build.plan",
    "Tooling.build.target",
    "Tooling.fmt.core",
    "Tooling.fmt.policy",
    "Tooling.fmt.style",
    "Tooling.lint.core",
    "Tooling.lint.policy",
    "Tooling.lint.rules",
    "Tooling.pkg.core",
    "Tooling.pkg.manifest",
    "Tooling.pkg.registry",
];

type UseFamilyItems = &'static [(&'static str, &'static str)];
type UseFamilyEntry = (&'static str, UseFamilyItems, UseRiskLevel);
type UseFamilyTable = &'static [UseFamilyEntry];

const CORE_FAMILIES: UseFamilyTable = &[
    (
        "types",
        &[
            ("types", "nC.core.types"),
            ("option", "nC.runtime.option"),
            ("result", "nC.runtime.result"),
            ("error", "nC.runtime.error"),
        ],
        UseRiskLevel::Low,
    ),
    (
        "memory",
        &[
            ("ptr", "nC.runtime.ptr"),
            ("region", "nC.runtime.region"),
            ("bytes", "nC.runtime.bytes"),
            ("layout", "nC.runtime.layout"),
            ("align", "nC.runtime.align"),
            ("endian", "nC.runtime.endian"),
            ("slice", "nC.runtime.slice"),
            ("span", "nC.runtime.span"),
            ("addr", "nC.runtime.addr"),
            ("volatile", "nC.runtime.ptr"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "sync",
        &[
            ("atomic", "nC.runtime.atomic"),
            ("fence", "nC.runtime.atomic"),
            ("barrier", "nC.runtime.atomic"),
            ("wait", "nC.runtime.atomic"),
            ("notify", "nC.runtime.atomic"),
        ],
        UseRiskLevel::Medium,
    ),
    ("math", &[("core", "nC.math.core")], UseRiskLevel::Low),
];

const ALLOC_FAMILIES: UseFamilyTable = &[
    (
        "heap",
        &[
            ("alloc", "nC.runtime.alloc"),
            ("heap", "nC.runtime.heap"),
            ("buffer", "nC.runtime.buffer"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "collections",
        &[
            ("array", "nC.runtime.array"),
            ("vector", "nC.vector.core"),
            ("list", "nC.runtime.list"),
            ("queue", "nC.runtime.queue"),
        ],
        UseRiskLevel::Low,
    ),
    (
        "string",
        &[("string", "nC.runtime.string")],
        UseRiskLevel::Low,
    ),
];

const STOUD_FAMILIES: UseFamilyTable = &[
    (
        "io",
        &[
            ("input", "nC.runtime.input"),
            ("output", "nC.runtime.output"),
            ("errorOutput", "nC.runtime.errorOutput"),
            ("std", "nC.runtime.std"),
            ("render", "nC.runtime.render"),
            ("concat", "nC.runtime.concat"),
            ("flush", "nC.runtime.flush"),
            ("random", "nC.runtime.random"),
            ("range", "nC.runtime.random"),
            ("int", "nC.runtime.random"),
            ("float", "nC.runtime.random"),
            ("bool", "nC.runtime.random"),
            ("seed", "nC.runtime.random"),
            ("entropy", "nC.runtime.entropy"),
            ("uuid", "nC.runtime.uuid"),
            // Compatibility aliases; the official family is stoud::fs.
            ("fs", "nC.runtime.fs"),
            ("fsread", "nC.runtime.fs"),
            ("fread_lines", "nC.runtime.fs"),
            ("fswrite", "nC.runtime.fs"),
            ("fsappend", "nC.runtime.fs"),
            ("fsync", "nC.runtime.fs"),
            ("exists", "nC.runtime.fs"),
            ("deleteFile", "nC.runtime.fs"),
            ("delete_file", "nC.runtime.fs"),
            ("copyFile", "nC.runtime.fs"),
            ("createDir", "nC.runtime.fs"),
            ("removeDir", "nC.runtime.fs"),
            ("listDir", "nC.runtime.fs"),
            ("metadata", "nC.runtime.fs"),
            ("permissions", "nC.runtime.fs"),
            ("setReadonly", "nC.runtime.fs"),
            ("renamePath", "nC.runtime.fs"),
            ("rename_file", "nC.runtime.fs"),
            ("movePath", "nC.runtime.fs"),
            ("absolutePath", "nC.runtime.fs"),
            ("officialPath", "nC.runtime.fs"),
            ("lockFile", "nC.runtime.fs"),
            ("unlockFile", "nC.runtime.fs"),
            ("readBytes", "nC.runtime.fs"),
            ("readBytesInto", "nC.runtime.fs"),
            ("writeBytes", "nC.runtime.fs"),
        ],
        UseRiskLevel::Low,
    ),
    (
        "memory",
        &[
            ("ptr", "nC.runtime.ptr"),
            ("region", "nC.runtime.region"),
            ("stack", "nC.runtime.stack"),
            ("heap", "nC.runtime.heap"),
            ("map", "nC.runtime.memory"),
            ("alloc", "nC.runtime.alloc"),
            ("buffer", "nC.runtime.buffer"),
            ("bytes", "nC.runtime.bytes"),
            ("layout", "nC.runtime.layout"),
            ("align", "nC.runtime.align"),
            ("endian", "nC.runtime.endian"),
            ("slice", "nC.runtime.slice"),
            ("span", "nC.runtime.span"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "collections",
        &[
            ("array", "nC.runtime.array"),
            ("list", "nC.runtime.list"),
            ("queue", "nC.runtime.queue"),
            ("vector", "nC.vector.core"),
            ("slice", "nC.runtime.slice"),
            ("span", "nC.runtime.span"),
            ("option", "nC.runtime.option"),
            ("result", "nC.runtime.result"),
            ("error", "nC.runtime.error"),
        ],
        UseRiskLevel::Low,
    ),
    ("data", &[("json", "nC.runtime.json")], UseRiskLevel::Low),
    (
        "math",
        &[
            ("core", "nC.math.core"),
            ("algebra", "nC.math.algebra"),
            ("trig", "nC.math.trig"),
            ("geometry", "nC.math.geometry"),
            ("vector", "nC.math.vector"),
            ("matrix", "nC.math.matrix"),
            ("tensor", "nC.math.tensor"),
            ("stats", "nC.math.stats"),
            ("prob", "nC.math.prob"),
            ("calc", "nC.math.calc"),
            ("numeric", "nC.math.numeric"),
            ("complex", "nC.math.complex"),
            ("discrete", "nC.math.discrete"),
            ("sign", "nC.runtime.math"),
        ],
        UseRiskLevel::Low,
    ),
    (
        "time",
        &[
            ("clock", "nC.runtime.clock"),
            ("time", "nC.runtime.time"),
            ("timer", "nC.runtime.timer"),
            ("sleep", "nC.runtime.time"),
        ],
        UseRiskLevel::Low,
    ),
    (
        "mobile",
        &[
            ("telephony", "nC.runtime.telephony"),
            ("tts_engine", "nC.runtime.tts"),
            ("network", "nC.runtime.mobileNetwork"),
            ("permissions", "nC.runtime.permission"),
        ],
        UseRiskLevel::High,
    ),
    (
        "network",
        &[
            ("network", "nC.runtime.mobileNetwork"),
            ("server", "nC.runtime.mobileNetwork"),
            ("client", "nC.runtime.mobileNetwork"),
            ("send_bytes", "nC.runtime.mobileNetwork"),
            ("receive_bytes", "nC.runtime.mobileNetwork"),
            ("server_open", "nC.runtime.mobileNetwork"),
            ("isWiFiConnected", "nC.runtime.mobileNetwork"),
        ],
        UseRiskLevel::High,
    ),
    (
        "sys",
        &[
            ("permissions", "nC.runtime.permission"),
            ("get_current_time", "nC.runtime.time"),
            ("cpu_load", "nC.runtime.system"),
            ("registry_write", "nC.runtime.system"),
            ("registry_read", "nC.runtime.system"),
            ("sleep", "nC.runtime.time"),
        ],
        UseRiskLevel::High,
    ),
    (
        "terminal",
        &[
            ("terminal", "nC.runtime.terminal"),
            ("clear", "nC.runtime.terminal"),
            ("moveCursor", "nC.runtime.terminal"),
            ("hideCursor", "nC.runtime.terminal"),
            ("showCursor", "nC.runtime.terminal"),
            ("enterRawMode", "nC.runtime.terminal"),
            ("exitRawMode", "nC.runtime.terminal"),
            ("readKeyNonBlocking", "nC.runtime.terminal"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "ui",
        &[
            ("window", "nC.runtime.terminal"),
            ("canvas", "nC.runtime.render"),
            ("input", "nC.runtime.input"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "os",
        &[
            ("os", "nC.runtime.os"),
            ("platform", "nC.runtime.os"),
            ("openUri", "nC.runtime.os"),
            ("media", "nC.runtime.os"),
            ("mediaNext", "nC.runtime.os"),
            ("mediaPlay", "nC.runtime.os"),
            ("mediaPause", "nC.runtime.os"),
            ("mediaPrevious", "nC.runtime.os"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "fs",
        &[
            ("fs", "nC.runtime.fs"),
            ("read", "nC.runtime.fs"),
            ("write", "nC.runtime.fs"),
            ("append", "nC.runtime.fs"),
            ("sync", "nC.runtime.fs"),
            ("exists", "nC.runtime.fs"),
            ("delete", "nC.runtime.fs"),
            ("copy", "nC.runtime.fs"),
            ("mkdir", "nC.runtime.dir"),
            ("rmdir", "nC.runtime.dir"),
            ("list", "nC.runtime.dir"),
            ("metadata", "nC.runtime.fs"),
            ("permissions", "nC.runtime.fs"),
            ("path", "nC.runtime.path"),
            ("dir", "nC.runtime.dir"),
            ("file", "nC.runtime.file"),
            ("stream", "nC.runtime.stream"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "net",
        &[
            ("net", "nC.runtime.net"),
            ("socket", "nC.runtime.socket"),
            ("tcp", "nC.runtime.tcp"),
            ("udp", "nC.runtime.udp"),
            ("http", "nC.runtime.http"),
            ("dns", "nC.runtime.dns"),
            ("tls", "nC.runtime.tls"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "process",
        &[
            ("process", "nC.runtime.process"),
            ("args", "nC.runtime.args"),
            ("env", "nC.runtime.env"),
            ("exit", "nC.runtime.exit"),
            ("session", "nC.runtime.session"),
            ("command", "nC.runtime.command"),
            ("user", "nC.runtime.user"),
            ("temp", "nC.runtime.temp"),
            ("service", "nC.runtime.service"),
            ("signal", "nC.runtime.signal"),
            ("spawn", "nC.runtime.process"),
            ("wait", "nC.runtime.process"),
            ("kill", "nC.runtime.process"),
            ("stdio", "nC.runtime.process"),
            ("status", "nC.runtime.process"),
        ],
        UseRiskLevel::High,
    ),
    (
        "thread",
        &[
            ("thread", "nC.runtime.thread"),
            ("spawn", "nC.runtime.thread"),
            ("join", "nC.runtime.thread"),
            ("sleep", "nC.runtime.thread"),
            ("local", "nC.runtime.thread"),
            ("sync", "nC.runtime.sync"),
            ("lock", "nC.runtime.lock"),
            ("atomic", "nC.runtime.atomic"),
            ("channel", "nC.runtime.channel"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "text",
        &[
            ("text", "nC.runtime.text"),
            ("string", "nC.runtime.string"),
            ("unicode", "nC.runtime.unicode"),
            ("format", "nC.runtime.format"),
            ("parse", "nC.runtime.parse"),
            ("convert", "nC.runtime.convert"),
            ("width", "nC.runtime.width"),
            ("escape", "nC.runtime.escape"),
            ("pad", "nC.runtime.pad"),
            ("wrap", "nC.runtime.wrap"),
            ("truncate", "nC.runtime.truncate"),
        ],
        UseRiskLevel::Low,
    ),
    (
        "task",
        &[
            ("task", "nC.runtime.task"),
            ("thread", "nC.runtime.thread"),
            ("async", "nC.runtime.async"),
            ("scheduler", "nC.runtime.scheduler"),
            ("cancel", "nC.runtime.cancel"),
            ("event", "nC.runtime.event"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "sync",
        &[
            ("sync", "nC.runtime.sync"),
            ("lock", "nC.runtime.lock"),
            ("atomic", "nC.runtime.atomic"),
            ("channel", "nC.runtime.channel"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "audit",
        &[
            ("debug", "nC.runtime.debug"),
            ("diagnostic", "nC.runtime.diagnostic"),
            ("log", "nC.runtime.log"),
            ("trace", "nC.runtime.trace"),
            ("status", "nC.runtime.status"),
            ("assert", "nC.runtime.assert"),
            ("validate", "nC.runtime.validate"),
            ("profile", "nC.runtime.profile"),
            ("bridge", "nC.runtime.bridge"),
        ],
        UseRiskLevel::Low,
    ),
];

const AUTH_FAMILIES: UseFamilyTable = &[
    (
        "system",
        &[
            ("file", "nC.runtime.authFile"),
            ("dir", "nC.runtime.authDir"),
            ("path", "nC.runtime.authPath"),
            ("env", "nC.runtime.authEnv"),
            ("process", "nC.runtime.process"),
            ("thread", "nC.runtime.thread"),
            ("permission", "nC.runtime.authPermission"),
            ("service", "nC.runtime.authService"),
            ("signal", "nC.runtime.authSignal"),
            ("session", "nC.runtime.authSystemSession"),
            ("media", "nC.runtime.os"),
            ("os", "nC.runtime.os"),
        ],
        UseRiskLevel::High,
    ),
    (
        "security",
        &[
            ("policy", "nC.runtime.authPolicy"),
            ("capability", "nC.runtime.authCapability"),
            ("sandbox", "nC.runtime.authSandbox"),
            ("audit", "nC.runtime.securityAudit"),
        ],
        UseRiskLevel::High,
    ),
    (
        "resource",
        &[
            ("resource", "nC.runtime.authResource"),
            ("handle", "nC.runtime.authHandle"),
            ("session", "nC.runtime.authSession"),
        ],
        UseRiskLevel::High,
    ),
];

const DOM_FAMILIES: UseFamilyTable = &[
    (
        "compiler",
        &[
            ("parse", "nC.runtime.compilerManual"),
            ("ast", "nC.runtime.compilerManual"),
            ("sema", "nC.runtime.compilerManual"),
            ("lowering", "nC.runtime.compilerManual"),
            ("diagnostics", "nC.runtime.compilerManual"),
            ("report", "nC.runtime.compilerManual"),
            ("manifest", "nC.runtime.compilerManual"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "high",
        &[
            ("typeinfo", "nC.runtime.highManual"),
            ("metadata", "nC.runtime.highManual"),
            ("consteval", "nC.runtime.highManual"),
            ("comptime", "nC.runtime.highManual"),
            ("macros", "nC.runtime.highManual"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "inspect",
        &[
            ("site", "nC.runtime.inspectManual"),
            ("metadata", "nC.runtime.inspectManual"),
            ("trace", "nC.runtime.inspectManual"),
            ("report", "nC.runtime.inspectManual"),
            ("evidence", "nC.runtime.inspectManual"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "ai",
        &[
            ("model", "nC.runtime.aiManual"),
            ("agent", "nC.runtime.aiManual"),
            ("memory", "nC.runtime.aiManual"),
            ("policy", "nC.runtime.aiManual"),
            ("eval", "nC.runtime.aiManual"),
        ],
        UseRiskLevel::Medium,
    ),
    (
        "tooling",
        &[
            ("fmt", "nC.runtime.toolingManual"),
            ("lint", "nC.runtime.toolingManual"),
            ("lsp", "nC.runtime.toolingManual"),
            ("package", "nC.runtime.toolingManual"),
            ("build", "nC.runtime.toolingManual"),
        ],
        UseRiskLevel::Medium,
    ),
];

pub fn parse_use_domain(raw: &str) -> Option<UseDomain> {
    match raw.to_ascii_lowercase().as_str() {
        "core" => Some(UseDomain::Core),
        "alloc" => Some(UseDomain::Alloc),
        "stoud" => Some(UseDomain::Stoud),
        "auth" => Some(UseDomain::Auth),
        "dom" => Some(UseDomain::Dom),
        _ => None,
    }
}

fn family_table(domain: UseDomain) -> Option<UseFamilyTable> {
    match domain {
        UseDomain::Core => Some(CORE_FAMILIES),
        UseDomain::Alloc => Some(ALLOC_FAMILIES),
        UseDomain::Stoud => Some(STOUD_FAMILIES),
        UseDomain::Auth => Some(AUTH_FAMILIES),
        UseDomain::Dom => Some(DOM_FAMILIES),
        UseDomain::LegacyBaseical => None,
    }
}

fn domain_label(domain: UseDomain) -> &'static str {
    match domain {
        UseDomain::Core => "core",
        UseDomain::Alloc => "alloc",
        UseDomain::Stoud => "stoud",
        UseDomain::Auth => "auth",
        UseDomain::Dom => "dom",
        UseDomain::LegacyBaseical => "legacy",
    }
}

fn find_family(domain: UseDomain, family: &str) -> Option<UseFamilyEntry> {
    family_table(domain)?
        .iter()
        .copied()
        .find(|(name, _, _)| name.eq_ignore_ascii_case(family))
}

pub fn use_mode_gates() -> Vec<UseModeGate> {
    vec![
        UseModeGate {
            mode: "normal/high-level/runtime/compiler",
            imports_allowed: true,
            diagnostic: None,
        },
        UseModeGate {
            mode: "machine",
            imports_allowed: false,
            diagnostic: Some("E_USE_FORBIDDEN_IN_MACHINE_001"),
        },
        UseModeGate {
            mode: "primitive",
            imports_allowed: false,
            diagnostic: Some("E_USE_FORBIDDEN_IN_PRIMITIVE_001"),
        },
        UseModeGate {
            mode: "basal",
            imports_allowed: false,
            diagnostic: Some("E_USE_FORBIDDEN_IN_BASAL_001"),
        },
        UseModeGate {
            mode: "kernel",
            imports_allowed: false,
            diagnostic: Some("E_USE_FORBIDDEN_IN_KERNEL_001"),
        },
    ]
}

pub fn use_policy_requirements() -> Vec<UsePolicyRequirement> {
    AUTH_FAMILIES
        .iter()
        .map(|(family, _, _)| UsePolicyRequirement {
            domain: UseDomain::Auth,
            family,
            requires_policy: false,
            diagnostic: None,
        })
        .collect()
}

const USE_BACKING_DEFERRED_DIAGNOSTIC: &str =
    "E_USE_BACKING_DEFERRED_001: existe en catálogo pero no está bajado a backend";

fn runtime_import_closure_status(
    domain: UseDomain,
    family: &str,
    item: &str,
    has_backing: bool,
) -> RuntimeImportClosureStatus {
    if !has_backing {
        return RuntimeImportClosureStatus::CatalogOnlyWithDiagnostic;
    }

    if matches!(domain, UseDomain::Core | UseDomain::Alloc) {
        return RuntimeImportClosureStatus::ClosedNativeBackend;
    }

    let family = family.to_ascii_lowercase();
    let item = item.to_ascii_lowercase();
    match (domain, family.as_str(), item.as_str()) {
        (UseDomain::Stoud, "io", "input")
        | (UseDomain::Stoud, "io", "output")
        | (UseDomain::Stoud, "io", "render")
        | (UseDomain::Stoud, "io", "bool")
        | (UseDomain::Stoud, "io", "concat")
        | (UseDomain::Stoud, "io", "entropy")
        | (UseDomain::Stoud, "io", "erroroutput")
        | (UseDomain::Stoud, "io", "float")
        | (UseDomain::Stoud, "io", "flush")
        | (UseDomain::Stoud, "io", "int")
        | (UseDomain::Stoud, "io", "range")
        | (UseDomain::Stoud, "io", "seed")
        | (UseDomain::Stoud, "io", "std")
        | (UseDomain::Stoud, "io", "uuid")
        | (UseDomain::Stoud, "io", "fs")
        | (UseDomain::Stoud, "io", "fsread")
        | (UseDomain::Stoud, "io", "fread_lines")
        | (UseDomain::Stoud, "io", "fswrite")
        | (UseDomain::Stoud, "io", "fsappend")
        | (UseDomain::Stoud, "io", "fsync")
        | (UseDomain::Stoud, "io", "exists")
        | (UseDomain::Stoud, "io", "deletefile")
        | (UseDomain::Stoud, "io", "delete_file")
        | (UseDomain::Stoud, "io", "copyfile")
        | (UseDomain::Stoud, "io", "createdir")
        | (UseDomain::Stoud, "io", "removedir")
        | (UseDomain::Stoud, "io", "listdir")
        | (UseDomain::Stoud, "io", "metadata")
        | (UseDomain::Stoud, "io", "permissions")
        | (UseDomain::Stoud, "io", "setreadonly")
        | (UseDomain::Stoud, "io", "renamepath")
        | (UseDomain::Stoud, "io", "rename_file")
        | (UseDomain::Stoud, "io", "movepath")
        | (UseDomain::Stoud, "io", "absolutepath")
        | (UseDomain::Stoud, "io", "officialpath")
        | (UseDomain::Stoud, "io", "lockfile")
        | (UseDomain::Stoud, "io", "unlockfile")
        | (UseDomain::Stoud, "io", "readbytes")
        | (UseDomain::Stoud, "io", "readbytesinto")
        | (UseDomain::Stoud, "io", "writebytes")
        | (UseDomain::Stoud, "fs", "fs")
        | (UseDomain::Stoud, "fs", "read")
        | (UseDomain::Stoud, "fs", "write")
        | (UseDomain::Stoud, "fs", "append")
        | (UseDomain::Stoud, "fs", "sync")
        | (UseDomain::Stoud, "fs", "exists")
        | (UseDomain::Stoud, "fs", "delete")
        | (UseDomain::Stoud, "fs", "copy")
        | (UseDomain::Stoud, "fs", "mkdir")
        | (UseDomain::Stoud, "fs", "rmdir")
        | (UseDomain::Stoud, "fs", "list")
        | (UseDomain::Stoud, "fs", "metadata")
        | (UseDomain::Stoud, "fs", "permissions")
        | (UseDomain::Stoud, "fs", "path")
        | (UseDomain::Stoud, "fs", "dir")
        | (UseDomain::Stoud, "fs", "file")
        | (UseDomain::Stoud, "fs", "stream")
        | (UseDomain::Stoud, "data", "json")
        | (UseDomain::Stoud, "time", "timer")
        | (UseDomain::Stoud, "time", "clock")
        | (UseDomain::Stoud, "time", "sleep")
        | (UseDomain::Stoud, "terminal", _)
        | (UseDomain::Stoud, "mobile", "telephony")
        | (UseDomain::Stoud, "mobile", "tts_engine")
        | (UseDomain::Stoud, "mobile", "network")
        | (UseDomain::Stoud, "mobile", "permissions")
        | (UseDomain::Stoud, "network", "server")
        | (UseDomain::Stoud, "network", "client")
        | (UseDomain::Stoud, "network", "send_bytes")
        | (UseDomain::Stoud, "network", "receive_bytes")
        | (UseDomain::Stoud, "network", "server_open")
        | (UseDomain::Stoud, "network", "iswificonnected")
        | (UseDomain::Stoud, "sys", "permissions")
        | (UseDomain::Stoud, "sys", "get_current_time")
        | (UseDomain::Stoud, "sys", "cpu_load")
        | (UseDomain::Stoud, "sys", "registry_write")
        | (UseDomain::Stoud, "sys", "registry_read")
        | (UseDomain::Stoud, "sys", "sleep")
        | (UseDomain::Stoud, "ui", "window")
        | (UseDomain::Stoud, "ui", "canvas")
        | (UseDomain::Stoud, "ui", "input")
        | (UseDomain::Stoud, "os", "os")
        | (UseDomain::Stoud, "os", "platform")
        | (UseDomain::Stoud, "os", "openuri")
        | (UseDomain::Stoud, "os", "media")
        | (UseDomain::Stoud, "os", "medianext")
        | (UseDomain::Stoud, "os", "mediaplay")
        | (UseDomain::Stoud, "os", "mediapause")
        | (UseDomain::Stoud, "os", "mediaprevious")
        | (UseDomain::Stoud, "net", "tcp")
        | (UseDomain::Stoud, "net", "udp")
        | (UseDomain::Stoud, "net", "dns")
        | (UseDomain::Stoud, "net", "http")
        | (UseDomain::Stoud, "net", "net")
        | (UseDomain::Stoud, "net", "socket")
        | (UseDomain::Stoud, "net", "tls")
        | (UseDomain::Stoud, "process", "process")
        | (UseDomain::Stoud, "process", "args")
        | (UseDomain::Stoud, "process", "command")
        | (UseDomain::Stoud, "process", "env")
        | (UseDomain::Stoud, "process", "exit")
        | (UseDomain::Stoud, "process", "session")
        | (UseDomain::Stoud, "process", "signal")
        | (UseDomain::Stoud, "process", "temp")
        | (UseDomain::Stoud, "process", "user")
        | (UseDomain::Stoud, "process", "service")
        | (UseDomain::Stoud, "process", "spawn")
        | (UseDomain::Stoud, "process", "wait")
        | (UseDomain::Stoud, "process", "kill")
        | (UseDomain::Stoud, "process", "stdio")
        | (UseDomain::Stoud, "process", "status")
        | (UseDomain::Stoud, "thread", "thread")
        | (UseDomain::Stoud, "thread", "spawn")
        | (UseDomain::Stoud, "thread", "join")
        | (UseDomain::Stoud, "thread", "sleep")
        | (UseDomain::Stoud, "thread", "local")
        | (UseDomain::Stoud, "thread", "sync")
        | (UseDomain::Stoud, "thread", "lock")
        | (UseDomain::Stoud, "thread", "atomic")
        | (UseDomain::Stoud, "thread", "channel")
        | (UseDomain::Stoud, "task", "task")
        | (UseDomain::Stoud, "task", "async")
        | (UseDomain::Stoud, "task", "scheduler")
        | (UseDomain::Stoud, "task", "cancel")
        | (UseDomain::Stoud, "task", "thread")
        | (UseDomain::Stoud, "task", "event")
        | (UseDomain::Stoud, "audit", "assert")
        | (UseDomain::Stoud, "audit", "bridge")
        | (UseDomain::Stoud, "audit", "debug")
        | (UseDomain::Stoud, "audit", "diagnostic")
        | (UseDomain::Stoud, "audit", "log")
        | (UseDomain::Stoud, "audit", "profile")
        | (UseDomain::Stoud, "audit", "status")
        | (UseDomain::Stoud, "audit", "trace")
        | (UseDomain::Stoud, "audit", "validate")
        | (UseDomain::Stoud, "sync", "sync")
        | (UseDomain::Stoud, "sync", "lock")
        | (UseDomain::Stoud, "sync", "atomic")
        | (UseDomain::Stoud, "sync", "channel")
        | (UseDomain::Stoud, "memory", "align")
        | (UseDomain::Stoud, "memory", "slice")
        | (UseDomain::Stoud, "memory", "span")
        | (UseDomain::Stoud, "memory", "stack")
        | (UseDomain::Stoud, "collections", "span")
        | (UseDomain::Stoud, "text", "convert")
        | (UseDomain::Stoud, "text", "escape")
        | (UseDomain::Stoud, "text", "format")
        | (UseDomain::Stoud, "text", "pad")
        | (UseDomain::Stoud, "text", "parse")
        | (UseDomain::Stoud, "text", "string")
        | (UseDomain::Stoud, "text", "text")
        | (UseDomain::Stoud, "text", "truncate")
        | (UseDomain::Stoud, "text", "unicode")
        | (UseDomain::Stoud, "text", "width")
        | (UseDomain::Stoud, "text", "wrap")
        | (UseDomain::Stoud, "math", "sign")
        | (UseDomain::Auth, "system", "process")
        | (UseDomain::Auth, "system", "thread")
        | (UseDomain::Auth, "system", "os")
        | (UseDomain::Auth, "system", "media")
        | (UseDomain::Auth, "resource", "handle")
        | (UseDomain::Auth, "resource", "resource")
        | (UseDomain::Auth, "resource", "session")
        | (UseDomain::Auth, "system", "dir")
        | (UseDomain::Auth, "system", "env")
        | (UseDomain::Auth, "system", "file")
        | (UseDomain::Auth, "system", "path")
        | (UseDomain::Auth, "system", "permission")
        | (UseDomain::Auth, "system", "service")
        | (UseDomain::Auth, "system", "signal")
        | (UseDomain::Auth, "system", "session")
        | (UseDomain::Auth, "security", "policy")
        | (UseDomain::Auth, "security", "capability")
        | (UseDomain::Auth, "security", "sandbox")
        | (UseDomain::Auth, "security", "audit")
        | (UseDomain::Stoud, "memory", "buffer")
        | (UseDomain::Stoud, "memory", "bytes")
        | (UseDomain::Dom, "compiler", _)
        | (UseDomain::Dom, "ai", _)
        | (UseDomain::Dom, "high", _)
        | (UseDomain::Dom, "inspect", _)
        | (UseDomain::Dom, "tooling", _) => RuntimeImportClosureStatus::ClosedRealBackingBytes,
        (UseDomain::Stoud, "memory", "ptr")
        | (UseDomain::Stoud, "memory", "heap")
        | (UseDomain::Stoud, "memory", "map")
        | (UseDomain::Stoud, "memory", "region")
        | (UseDomain::Stoud, "memory", "alloc")
        | (UseDomain::Stoud, "memory", "layout")
        | (UseDomain::Stoud, "memory", "endian")
        | (UseDomain::Stoud, "collections", "array")
        | (UseDomain::Stoud, "collections", "list")
        | (UseDomain::Stoud, "collections", "queue")
        | (UseDomain::Stoud, "collections", "vector")
        | (UseDomain::Stoud, "collections", "slice")
        | (UseDomain::Stoud, "collections", "option")
        | (UseDomain::Stoud, "collections", "result")
        | (UseDomain::Stoud, "collections", "error")
        | (UseDomain::Stoud, "math", _) => RuntimeImportClosureStatus::ClosedNativeBackend,
        _ => RuntimeImportClosureStatus::ClosedInterpreterRuntime,
    }
}

fn runtime_import_run_contract(status: RuntimeImportClosureStatus) -> &'static str {
    match status {
        RuntimeImportClosureStatus::ClosedRealBackingBytes => {
            "ncc_run_direct_runtime_symbol_and_mct_activation_bytes"
        }
        RuntimeImportClosureStatus::ClosedNativeBackend => {
            "ncc_run_native_backend_intrinsic_and_mct_activation_bytes"
        }
        RuntimeImportClosureStatus::ClosedInterpreterRuntime => {
            "ncc_run_interpreter_runtime_api_and_mct_activation_bytes"
        }
        RuntimeImportClosureStatus::CatalogOnlyWithDiagnostic => {
            "diagnostic_only_no_runtime_call_lowering"
        }
    }
}

fn use_import_has_runtime_or_language_backing(
    domain: UseDomain,
    family: &str,
    item: &str,
    expands_to: &str,
) -> bool {
    if matches!(domain, UseDomain::Core | UseDomain::Alloc) {
        return true;
    }
    let family = family.to_ascii_lowercase();
    let item = item.to_ascii_lowercase();
    matches!(
        (domain, family.as_str(), item.as_str(), expands_to),
        (UseDomain::Stoud, "io", "input", "nC.runtime.input")
            | (UseDomain::Stoud, "io", "output", "nC.runtime.output")
            | (UseDomain::Stoud, "io", "render", "nC.runtime.render")
            | (UseDomain::Stoud, "io", "bool", "nC.runtime.random")
            | (UseDomain::Stoud, "io", "concat", "nC.runtime.concat")
            | (UseDomain::Stoud, "io", "entropy", "nC.runtime.entropy")
            | (
                UseDomain::Stoud,
                "io",
                "erroroutput",
                "nC.runtime.errorOutput"
            )
            | (UseDomain::Stoud, "io", "float", "nC.runtime.random")
            | (UseDomain::Stoud, "io", "flush", "nC.runtime.flush")
            | (UseDomain::Stoud, "io", "int", "nC.runtime.random")
            | (UseDomain::Stoud, "io", "range", "nC.runtime.random")
            | (UseDomain::Stoud, "io", "seed", "nC.runtime.random")
            | (UseDomain::Stoud, "io", "std", "nC.runtime.std")
            | (UseDomain::Stoud, "io", "uuid", "nC.runtime.uuid")
            | (UseDomain::Stoud, "io", "random", "nC.runtime.random")
            | (UseDomain::Stoud, "io", "fs", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "fsread", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "fread_lines", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "fswrite", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "fsappend", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "fsync", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "exists", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "deletefile", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "delete_file", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "copyfile", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "createdir", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "removedir", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "listdir", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "metadata", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "permissions", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "setreadonly", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "renamepath", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "rename_file", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "movepath", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "absolutepath", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "officialpath", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "lockfile", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "unlockfile", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "readbytes", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "readbytesinto", "nC.runtime.fs")
            | (UseDomain::Stoud, "io", "writebytes", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "fs", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "read", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "write", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "append", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "sync", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "exists", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "delete", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "copy", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "mkdir", "nC.runtime.dir")
            | (UseDomain::Stoud, "fs", "rmdir", "nC.runtime.dir")
            | (UseDomain::Stoud, "fs", "list", "nC.runtime.dir")
            | (UseDomain::Stoud, "fs", "metadata", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "permissions", "nC.runtime.fs")
            | (UseDomain::Stoud, "fs", "path", "nC.runtime.path")
            | (UseDomain::Stoud, "fs", "dir", "nC.runtime.dir")
            | (UseDomain::Stoud, "fs", "file", "nC.runtime.file")
            | (UseDomain::Stoud, "fs", "stream", "nC.runtime.stream")
            | (UseDomain::Stoud, "data", "json", "nC.runtime.json")
            | (UseDomain::Stoud, "time", "time", "nC.runtime.time")
            | (UseDomain::Stoud, "time", "clock", "nC.runtime.clock")
            | (UseDomain::Stoud, "time", "sleep", "nC.runtime.time")
            | (UseDomain::Stoud, "time", "timer", "nC.runtime.timer")
            | (UseDomain::Stoud, "terminal", _, "nC.runtime.terminal")
            | (
                UseDomain::Stoud,
                "mobile",
                "telephony",
                "nC.runtime.telephony"
            )
            | (UseDomain::Stoud, "mobile", "tts_engine", "nC.runtime.tts")
            | (
                UseDomain::Stoud,
                "mobile",
                "network",
                "nC.runtime.mobileNetwork"
            )
            | (
                UseDomain::Stoud,
                "mobile",
                "permissions",
                "nC.runtime.permission"
            )
            | (UseDomain::Stoud, "network", _, "nC.runtime.mobileNetwork")
            | (
                UseDomain::Stoud,
                "sys",
                "permissions",
                "nC.runtime.permission"
            )
            | (
                UseDomain::Stoud,
                "sys",
                "get_current_time",
                "nC.runtime.time"
            )
            | (UseDomain::Stoud, "sys", "cpu_load", "nC.runtime.system")
            | (
                UseDomain::Stoud,
                "sys",
                "registry_write",
                "nC.runtime.system"
            )
            | (
                UseDomain::Stoud,
                "sys",
                "registry_read",
                "nC.runtime.system"
            )
            | (UseDomain::Stoud, "sys", "sleep", "nC.runtime.time")
            | (UseDomain::Stoud, "ui", "window", "nC.runtime.terminal")
            | (UseDomain::Stoud, "ui", "canvas", "nC.runtime.render")
            | (UseDomain::Stoud, "ui", "input", "nC.runtime.input")
            | (UseDomain::Stoud, "os", _, "nC.runtime.os")
            | (UseDomain::Stoud, "net", "tcp", "nC.runtime.tcp")
            | (UseDomain::Stoud, "net", "udp", "nC.runtime.udp")
            | (UseDomain::Stoud, "net", "dns", "nC.runtime.dns")
            | (UseDomain::Stoud, "net", "http", "nC.runtime.http")
            | (UseDomain::Stoud, "net", "net", "nC.runtime.net")
            | (UseDomain::Stoud, "net", "socket", "nC.runtime.socket")
            | (UseDomain::Stoud, "net", "tls", "nC.runtime.tls")
            | (UseDomain::Stoud, "process", "process", "nC.runtime.process")
            | (UseDomain::Stoud, "process", "args", "nC.runtime.args")
            | (UseDomain::Stoud, "process", "command", "nC.runtime.command")
            | (UseDomain::Stoud, "process", "env", "nC.runtime.env")
            | (UseDomain::Stoud, "process", "exit", "nC.runtime.exit")
            | (UseDomain::Stoud, "process", "session", "nC.runtime.session")
            | (UseDomain::Stoud, "process", "signal", "nC.runtime.signal")
            | (UseDomain::Stoud, "process", "temp", "nC.runtime.temp")
            | (UseDomain::Stoud, "process", "user", "nC.runtime.user")
            | (UseDomain::Stoud, "process", "service", "nC.runtime.service")
            | (UseDomain::Stoud, "process", "spawn", "nC.runtime.process")
            | (UseDomain::Stoud, "process", "wait", "nC.runtime.process")
            | (UseDomain::Stoud, "process", "kill", "nC.runtime.process")
            | (UseDomain::Stoud, "process", "stdio", "nC.runtime.process")
            | (UseDomain::Stoud, "process", "status", "nC.runtime.process")
            | (UseDomain::Stoud, "thread", "thread", "nC.runtime.thread")
            | (UseDomain::Stoud, "thread", "spawn", "nC.runtime.thread")
            | (UseDomain::Stoud, "thread", "join", "nC.runtime.thread")
            | (UseDomain::Stoud, "thread", "sleep", "nC.runtime.thread")
            | (UseDomain::Stoud, "thread", "local", "nC.runtime.thread")
            | (UseDomain::Stoud, "thread", "sync", "nC.runtime.sync")
            | (UseDomain::Stoud, "thread", "lock", "nC.runtime.lock")
            | (UseDomain::Stoud, "thread", "atomic", "nC.runtime.atomic")
            | (UseDomain::Stoud, "thread", "channel", "nC.runtime.channel")
            | (UseDomain::Stoud, "task", "task", "nC.runtime.task")
            | (UseDomain::Stoud, "task", "async", "nC.runtime.async")
            | (
                UseDomain::Stoud,
                "task",
                "scheduler",
                "nC.runtime.scheduler"
            )
            | (UseDomain::Stoud, "task", "cancel", "nC.runtime.cancel")
            | (UseDomain::Stoud, "task", "thread", "nC.runtime.thread")
            | (UseDomain::Stoud, "task", "event", "nC.runtime.event")
            | (UseDomain::Stoud, "audit", "assert", "nC.runtime.assert")
            | (UseDomain::Stoud, "audit", "bridge", "nC.runtime.bridge")
            | (UseDomain::Stoud, "audit", "debug", "nC.runtime.debug")
            | (
                UseDomain::Stoud,
                "audit",
                "diagnostic",
                "nC.runtime.diagnostic"
            )
            | (UseDomain::Stoud, "audit", "log", "nC.runtime.log")
            | (UseDomain::Stoud, "audit", "profile", "nC.runtime.profile")
            | (UseDomain::Stoud, "audit", "status", "nC.runtime.status")
            | (UseDomain::Stoud, "audit", "trace", "nC.runtime.trace")
            | (UseDomain::Stoud, "audit", "validate", "nC.runtime.validate")
            | (UseDomain::Stoud, "sync", "sync", "nC.runtime.sync")
            | (UseDomain::Stoud, "sync", "lock", "nC.runtime.lock")
            | (UseDomain::Stoud, "sync", "atomic", "nC.runtime.atomic")
            | (UseDomain::Stoud, "sync", "channel", "nC.runtime.channel")
            | (UseDomain::Stoud, "memory", "ptr", "nC.runtime.ptr")
            | (UseDomain::Stoud, "memory", "heap", "nC.runtime.heap")
            | (UseDomain::Stoud, "memory", "map", "nC.runtime.memory")
            | (UseDomain::Stoud, "memory", "region", "nC.runtime.region")
            | (UseDomain::Stoud, "memory", "alloc", "nC.runtime.alloc")
            | (UseDomain::Stoud, "memory", "buffer", "nC.runtime.buffer")
            | (UseDomain::Stoud, "memory", "bytes", "nC.runtime.bytes")
            | (UseDomain::Stoud, "memory", "layout", "nC.runtime.layout")
            | (UseDomain::Stoud, "memory", "align", "nC.runtime.align")
            | (UseDomain::Stoud, "memory", "slice", "nC.runtime.slice")
            | (UseDomain::Stoud, "memory", "span", "nC.runtime.span")
            | (UseDomain::Auth, "system", "dir", "nC.runtime.authDir")
            | (UseDomain::Auth, "system", "env", "nC.runtime.authEnv")
            | (UseDomain::Auth, "system", "file", "nC.runtime.authFile")
            | (UseDomain::Auth, "system", "path", "nC.runtime.authPath")
            | (
                UseDomain::Auth,
                "system",
                "permission",
                "nC.runtime.authPermission"
            )
            | (
                UseDomain::Auth,
                "system",
                "service",
                "nC.runtime.authService"
            )
            | (UseDomain::Auth, "system", "signal", "nC.runtime.authSignal")
            | (
                UseDomain::Auth,
                "system",
                "session",
                "nC.runtime.authSystemSession"
            )
            | (
                UseDomain::Auth,
                "security",
                "policy",
                "nC.runtime.authPolicy"
            )
            | (
                UseDomain::Auth,
                "security",
                "capability",
                "nC.runtime.authCapability"
            )
            | (
                UseDomain::Auth,
                "security",
                "sandbox",
                "nC.runtime.authSandbox"
            )
            | (UseDomain::Stoud, "memory", "stack", "nC.runtime.stack")
            | (UseDomain::Stoud, "memory", "endian", "nC.runtime.endian")
            | (UseDomain::Stoud, "collections", "array", "nC.runtime.array")
            | (UseDomain::Stoud, "collections", "list", "nC.runtime.list")
            | (UseDomain::Stoud, "collections", "queue", "nC.runtime.queue")
            | (UseDomain::Stoud, "collections", "vector", "nC.vector.core")
            | (UseDomain::Stoud, "collections", "slice", "nC.runtime.slice")
            | (UseDomain::Stoud, "collections", "span", "nC.runtime.span")
            | (
                UseDomain::Stoud,
                "collections",
                "option",
                "nC.runtime.option"
            )
            | (
                UseDomain::Stoud,
                "collections",
                "result",
                "nC.runtime.result"
            )
            | (UseDomain::Stoud, "collections", "error", "nC.runtime.error")
            | (UseDomain::Stoud, "text", "convert", "nC.runtime.convert")
            | (UseDomain::Stoud, "text", "escape", "nC.runtime.escape")
            | (UseDomain::Stoud, "text", "format", "nC.runtime.format")
            | (UseDomain::Stoud, "text", "pad", "nC.runtime.pad")
            | (UseDomain::Stoud, "text", "parse", "nC.runtime.parse")
            | (UseDomain::Stoud, "text", "string", "nC.runtime.string")
            | (UseDomain::Stoud, "text", "text", "nC.runtime.text")
            | (UseDomain::Stoud, "text", "truncate", "nC.runtime.truncate")
            | (UseDomain::Stoud, "text", "unicode", "nC.runtime.unicode")
            | (UseDomain::Stoud, "text", "width", "nC.runtime.width")
            | (UseDomain::Stoud, "text", "wrap", "nC.runtime.wrap")
            | (UseDomain::Stoud, "math", "sign", "nC.runtime.math")
            | (UseDomain::Stoud, "math", _, "nC.math.core")
            | (UseDomain::Stoud, "math", _, "nC.math.algebra")
            | (UseDomain::Stoud, "math", _, "nC.math.trig")
            | (UseDomain::Stoud, "math", _, "nC.math.geometry")
            | (UseDomain::Stoud, "math", _, "nC.math.vector")
            | (UseDomain::Stoud, "math", _, "nC.math.matrix")
            | (UseDomain::Stoud, "math", _, "nC.math.tensor")
            | (UseDomain::Stoud, "math", _, "nC.math.stats")
            | (UseDomain::Stoud, "math", _, "nC.math.prob")
            | (UseDomain::Stoud, "math", _, "nC.math.calc")
            | (UseDomain::Stoud, "math", _, "nC.math.numeric")
            | (UseDomain::Stoud, "math", _, "nC.math.complex")
            | (UseDomain::Stoud, "math", _, "nC.math.discrete")
            | (UseDomain::Auth, "system", "process", "nC.runtime.process")
            | (UseDomain::Auth, "system", "thread", "nC.runtime.thread")
            | (UseDomain::Auth, "system", "os", "nC.runtime.os")
            | (UseDomain::Auth, "system", "media", "nC.runtime.os")
            | (
                UseDomain::Auth,
                "resource",
                "handle",
                "nC.runtime.authHandle"
            )
            | (
                UseDomain::Auth,
                "resource",
                "resource",
                "nC.runtime.authResource"
            )
            | (
                UseDomain::Auth,
                "resource",
                "session",
                "nC.runtime.authSession"
            )
            | (
                UseDomain::Auth,
                "security",
                "audit",
                "nC.runtime.securityAudit"
            )
            | (UseDomain::Dom, "compiler", _, "nC.runtime.compilerManual")
            | (UseDomain::Dom, "ai", _, "nC.runtime.aiManual")
            | (UseDomain::Dom, "high", _, "nC.runtime.highManual")
            | (UseDomain::Dom, "inspect", _, "nC.runtime.inspectManual")
            | (UseDomain::Dom, "tooling", _, "nC.runtime.toolingManual")
    )
}

fn use_import_backing_diagnostic(
    domain: UseDomain,
    family: &str,
    item: &str,
    expands_to: &str,
) -> Option<String> {
    if use_import_has_runtime_or_language_backing(domain, family, item, expands_to) {
        None
    } else {
        Some(USE_BACKING_DEFERRED_DIAGNOSTIC.to_string())
    }
}

pub fn expand_use_domain_import(
    domain_raw: &str,
    family_raw: &str,
    items: &[String],
) -> Result<Vec<UseAliasExpansion>, UseDiagnostic> {
    let Some(domain) = parse_use_domain(domain_raw) else {
        return Err(UseDiagnostic {
            code: "E_USE_DOMAIN_UNKNOWN_001",
            message: format!("unknown use domain '{}'", domain_raw),
            help: Some("expected core, alloc, stoud, auth or dom"),
        });
    };

    if matches!(domain, UseDomain::Stoud) && family_raw.eq_ignore_ascii_case("system") {
        return Err(UseDiagnostic {
            code: "E_USE_STOUD_AUTH_REQUIRED_001",
            message: format!(
                "stoud::{} is forbidden because system authority must use auth::system",
                family_raw
            ),
            help: Some("use use::auth::system::<...>; with explicit policy/capability"),
        });
    }

    let Some((family, family_items, family_risk)) = find_family(domain, family_raw) else {
        return Err(UseDiagnostic {
            code: "E_USE_FAMILY_UNKNOWN_001",
            message: format!(
                "unknown use family '{}::{}'",
                domain_label(domain),
                family_raw
            ),
            help: Some("check the imports manifest for supported families"),
        });
    };

    if items.is_empty() {
        return Err(UseDiagnostic {
            code: "E_USE_ITEM_UNKNOWN_001",
            message: format!(
                "use::{}::{} requires at least one explicit item",
                domain_label(domain),
                family
            ),
            help: Some("wildcards and empty item lists are not part of this surface"),
        });
    }

    let requires_policy = false;
    let mut expansions = Vec::new();
    for item in items {
        if item == "*" {
            return Err(UseDiagnostic {
                code: "E_USE_WILDCARD_DENIED_001",
                message: "wildcard imports are denied for use::core/alloc/stoud/auth/dom"
                    .to_string(),
                help: Some("list every imported item explicitly"),
            });
        }
        let Some((official_item, expands_to)) = family_items
            .iter()
            .copied()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(item))
        else {
            return Err(UseDiagnostic {
                code: "E_USE_ITEM_UNKNOWN_001",
                message: format!(
                    "unknown item '{}::{}::{}'",
                    domain_label(domain),
                    family,
                    item
                ),
                help: Some("check the imports manifest for supported items"),
            });
        };

        let backing_diagnostic =
            use_import_backing_diagnostic(domain, family, official_item, expands_to);
        let has_backing = backing_diagnostic.is_none();
        let closure_status =
            runtime_import_closure_status(domain, family, official_item, has_backing).as_str();
        expansions.push(UseAliasExpansion {
            surface_import: format!(
                "use::{}::{}::<{}>;",
                domain_label(domain),
                family,
                official_item
            ),
            expands_to: vec![expands_to.to_string()],
            domain,
            family: family.to_string(),
            item: official_item.to_string(),
            risk: if requires_policy {
                UseRiskLevel::High
            } else {
                family_risk
            },
            allowed_modes: if matches!(domain, UseDomain::Core | UseDomain::Alloc) {
                vec![
                    "normal".to_string(),
                    "high-level".to_string(),
                    "runtime".to_string(),
                    "compiler".to_string(),
                    "tooling".to_string(),
                    "kernel".to_string(),
                ]
            } else {
                vec![
                    "normal".to_string(),
                    "high-level".to_string(),
                    "runtime".to_string(),
                    "compiler".to_string(),
                    "tooling".to_string(),
                ]
            },
            forbidden_modes: if matches!(domain, UseDomain::Core | UseDomain::Alloc) {
                vec![
                    "machine".to_string(),
                    "primitive".to_string(),
                    "basal".to_string(),
                ]
            } else {
                vec![
                    "machine".to_string(),
                    "primitive".to_string(),
                    "basal".to_string(),
                    "kernel".to_string(),
                ]
            },
            requires_policy,
            status: if backing_diagnostic.is_some() {
                "closed_catalog_only"
            } else {
                "closed_real_backing"
            }
            .to_string(),
            closure_status,
            diagnostic: backing_diagnostic,
        });
    }
    Ok(expansions)
}

pub fn use_import_manifest_for_domain(domain: UseDomain) -> UseImportManifest {
    let mut entries = Vec::new();
    if let Some(families) = family_table(domain) {
        for (family, items, _) in families {
            let names = items
                .iter()
                .map(|(name, _)| (*name).to_string())
                .collect::<Vec<_>>();
            if let Ok(mut expanded) = expand_use_domain_import(domain_label(domain), family, &names)
            {
                entries.append(&mut expanded);
            }
        }
    }
    UseImportManifest {
        status: "closed",
        entries,
    }
}

fn runtime_family_suffix(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn runtime_family_risk(key: &str) -> UseRiskLevel {
    match key.to_ascii_lowercase().as_str() {
        "addr" | "alloc" | "buffer" | "bytes" | "heap" | "layout" | "memory" | "ptr" | "region"
        | "span" | "stack" | "stream" => UseRiskLevel::Medium,
        "bridge" | "capability" | "command" | "dir" | "env" | "file" | "fs" | "net" | "os"
        | "permission" | "policy" | "process" | "sandbox" | "service" | "signal" | "socket"
        | "tcp" | "thread" | "tls" | "udp" | "user" => UseRiskLevel::High,
        _ => UseRiskLevel::Low,
    }
}

fn runtime_official_family_aliases() -> Vec<UseAliasExpansion> {
    NC_RUNTIME_FAMILIES
        .iter()
        .map(|path| {
            let key = runtime_family_suffix(path);
            UseAliasExpansion {
                surface_import: format!("use {path};"),
                expands_to: vec![(*path).to_string()],
                domain: UseDomain::Stoud,
                family: "runtime".to_string(),
                item: key.to_string(),
                risk: runtime_family_risk(key),
                allowed_modes: vec![
                    "normal".to_string(),
                    "high-level".to_string(),
                    "runtime".to_string(),
                    "compiler".to_string(),
                    "tooling".to_string(),
                ],
                forbidden_modes: vec![
                    "machine".to_string(),
                    "primitive".to_string(),
                    "basal".to_string(),
                    "kernel".to_string(),
                ],
                requires_policy: false,
                status: "closed_real_backing".to_string(),
                closure_status: RuntimeImportClosureStatus::ClosedInterpreterRuntime.as_str(),
                diagnostic: None,
            }
        })
        .collect()
}

pub fn use_import_manifest_all() -> UseImportManifest {
    let mut entries = Vec::new();
    for domain in [
        UseDomain::Core,
        UseDomain::Alloc,
        UseDomain::Stoud,
        UseDomain::Auth,
        UseDomain::Dom,
    ] {
        entries.extend(use_import_manifest_for_domain(domain).entries);
    }
    entries.extend(runtime_official_family_aliases());
    UseImportManifest {
        status: "closed",
        entries,
    }
}

fn use_backend_symbol(domain: UseDomain, family: &str, item: &str) -> String {
    format!(
        "__nc_import_activate_{}_{}_{}",
        domain_label(domain),
        family.to_ascii_lowercase(),
        item.to_ascii_lowercase()
    )
}

fn use_backend_runtime_symbols(
    domain: UseDomain,
    family: &str,
    item: &str,
    expands_to: &[String],
) -> Vec<String> {
    let family = family.to_ascii_lowercase();
    let item = item.to_ascii_lowercase();
    let official = expands_to.first().map(String::as_str).unwrap_or_default();
    let symbols: &[&str] = match (domain, family.as_str(), item.as_str(), official) {
        (UseDomain::Stoud, "io", "input", "nC.runtime.input") => &["nC.runtime.input.readInt"],
        (UseDomain::Stoud, "io", "output", "nC.runtime.output") => &[
            "nC.runtime.output.print",
            "nC.runtime.output.println",
            "nC.runtime.output.write",
            "nC.runtime.output.writeln",
            "nC.runtime.output.color",
            "nC.runtime.output.colorBold",
            "nC.runtime.output.bgColor",
            "nC.runtime.output.style",
            "nC.runtime.output.emoji",
            "nC.runtime.output.progressBar",
            "nC.runtime.output.spinnerFrame",
            "nC.runtime.output.table2",
            "nC.runtime.output.table3",
        ],
        (UseDomain::Stoud, "io", "render", "nC.runtime.render") => &[
            "nC.runtime.render.renderInt",
            "nC.runtime.render.renderUInt",
            "nC.runtime.render.renderBinary",
            "nC.runtime.render.renderBin",
            "nC.runtime.render.renderBool",
            "nC.runtime.render.renderFloat",
            "nC.runtime.render.renderChar",
            "nC.runtime.render.renderStr",
            "nC.runtime.render.renderType",
            "nC.runtime.render.renderDebug",
            "nC.runtime.render.renderStable",
            "nC.runtime.output.color",
            "nC.runtime.output.colorBold",
        ],
        (UseDomain::Stoud, "io", "random", "nC.runtime.random") => &[
            "nC.runtime.random.random",
            "nC.runtime.random.randomRange",
            "nC.runtime.random.seed",
        ],
        (UseDomain::Stoud, "io", "bool", "nC.runtime.random") => &["nC.runtime.random.randomBool"],
        (UseDomain::Stoud, "io", "concat", "nC.runtime.concat") => &["nC.runtime.concat.concat"],
        (UseDomain::Stoud, "io", "entropy", "nC.runtime.entropy") => &[
            "nC.runtime.entropy.entropyBytes",
            "nC.runtime.entropy.entropyAvailable",
        ],
        (UseDomain::Stoud, "io", "erroroutput", "nC.runtime.errorOutput") => &[
            "nC.runtime.errorOutput.eprint",
            "nC.runtime.errorOutput.eprintln",
            "nC.runtime.errorOutput.warn",
            "nC.runtime.errorOutput.error",
            "nC.runtime.errorOutput.flushErr",
        ],
        (UseDomain::Stoud, "io", "float", "nC.runtime.random") => {
            &["nC.runtime.random.randomFloat"]
        }
        (UseDomain::Stoud, "io", "int", "nC.runtime.random") => &["nC.runtime.random.randomInt"],
        (UseDomain::Stoud, "io", "range", "nC.runtime.random") => {
            &["nC.runtime.random.randomRange"]
        }
        (UseDomain::Stoud, "io", "seed", "nC.runtime.random") => &["nC.runtime.random.seed"],
        (UseDomain::Stoud, "io", "std", "nC.runtime.std") => &[
            "nC.runtime.std.stdIn",
            "nC.runtime.std.stdOut",
            "nC.runtime.std.stdErr",
            "nC.runtime.std.isInteractive",
        ],
        (UseDomain::Stoud, "io", "uuid", "nC.runtime.uuid") => &[
            "nC.runtime.uuid.uuidV4",
            "nC.runtime.uuid.uuidFromBytes",
            "nC.runtime.uuid.isUuid",
        ],
        (UseDomain::Stoud, "io", "flush", "nC.runtime.flush") => &[
            "nC.runtime.flush.flush",
            "nC.runtime.flush.flushStdout",
            "nC.runtime.flush.flushStderr",
            "nC.runtime.flush.autoFlushOnInput",
            "nC.runtime.flush.flushBeforeInput",
        ],
        (UseDomain::Stoud, "io", "fs", "nC.runtime.fs") => &[
            "nC.runtime.fs.fsread",
            "nC.runtime.fs.fswrite",
            "nC.runtime.fs.fsappend",
            "nC.runtime.fs.fsync",
            "nC.runtime.fs.exists",
            "nC.runtime.fs.deleteFile",
            "nC.runtime.fs.copyFile",
            "nC.runtime.fs.createDir",
            "nC.runtime.fs.removeDir",
            "nC.runtime.fs.listDir",
            "nC.runtime.fs.metadata",
            "nC.runtime.fs.permissions",
            "nC.runtime.fs.setReadonly",
            "nC.runtime.fs.renamePath",
            "nC.runtime.fs.movePath",
            "nC.runtime.fs.absolutePath",
            "nC.runtime.fs.officialPath",
            "nC.runtime.fs.lockFile",
            "nC.runtime.fs.unlockFile",
            "nC.runtime.fs.readBytes",
            "nC.runtime.fs.readBytesInto",
            "nC.runtime.fs.writeBytes",
        ],
        (UseDomain::Stoud, "io", "fsread", "nC.runtime.fs") => &["nC.runtime.fs.fsread"],
        (UseDomain::Stoud, "io", "fread_lines", "nC.runtime.fs") => &["nC.runtime.fs.fread_lines"],
        (UseDomain::Stoud, "io", "fswrite", "nC.runtime.fs") => &["nC.runtime.fs.fswrite"],
        (UseDomain::Stoud, "io", "fsappend", "nC.runtime.fs") => &["nC.runtime.fs.fsappend"],
        (UseDomain::Stoud, "io", "fsync", "nC.runtime.fs") => &["nC.runtime.fs.fsync"],
        (UseDomain::Stoud, "io", "exists", "nC.runtime.fs") => &["nC.runtime.fs.exists"],
        (UseDomain::Stoud, "io", "deletefile", "nC.runtime.fs") => &["nC.runtime.fs.deleteFile"],
        (UseDomain::Stoud, "io", "delete_file", "nC.runtime.fs") => &["nC.runtime.fs.delete_file"],
        (UseDomain::Stoud, "io", "copyfile", "nC.runtime.fs") => &["nC.runtime.fs.copyFile"],
        (UseDomain::Stoud, "io", "createdir", "nC.runtime.fs") => &["nC.runtime.fs.createDir"],
        (UseDomain::Stoud, "io", "removedir", "nC.runtime.fs") => &["nC.runtime.fs.removeDir"],
        (UseDomain::Stoud, "io", "listdir", "nC.runtime.fs") => &["nC.runtime.fs.listDir"],
        (UseDomain::Stoud, "io", "metadata", "nC.runtime.fs") => &["nC.runtime.fs.metadata"],
        (UseDomain::Stoud, "io", "permissions", "nC.runtime.fs") => &["nC.runtime.fs.permissions"],
        (UseDomain::Stoud, "io", "setreadonly", "nC.runtime.fs") => &["nC.runtime.fs.setReadonly"],
        (UseDomain::Stoud, "io", "renamepath", "nC.runtime.fs") => &["nC.runtime.fs.renamePath"],
        (UseDomain::Stoud, "io", "rename_file", "nC.runtime.fs") => &["nC.runtime.fs.rename_file"],
        (UseDomain::Stoud, "io", "movepath", "nC.runtime.fs") => &["nC.runtime.fs.movePath"],
        (UseDomain::Stoud, "io", "absolutepath", "nC.runtime.fs") => {
            &["nC.runtime.fs.absolutePath"]
        }
        (UseDomain::Stoud, "io", "officialpath", "nC.runtime.fs") => {
            &["nC.runtime.fs.officialPath"]
        }
        (UseDomain::Stoud, "io", "lockfile", "nC.runtime.fs") => &["nC.runtime.fs.lockFile"],
        (UseDomain::Stoud, "io", "unlockfile", "nC.runtime.fs") => &["nC.runtime.fs.unlockFile"],
        (UseDomain::Stoud, "io", "readbytes", "nC.runtime.fs") => &["nC.runtime.fs.readBytes"],
        (UseDomain::Stoud, "io", "readbytesinto", "nC.runtime.fs") => {
            &["nC.runtime.fs.readBytesInto"]
        }
        (UseDomain::Stoud, "io", "writebytes", "nC.runtime.fs") => &["nC.runtime.fs.writeBytes"],
        (UseDomain::Stoud, "fs", "dir", "nC.runtime.dir") => &[
            "nC.runtime.dir.listDir",
            "nC.runtime.dir.createDir",
            "nC.runtime.dir.removeDir",
            "nC.runtime.dir.currentDir",
            "nC.runtime.dir.changeDir",
        ],
        (UseDomain::Stoud, "text", "pad", "nC.runtime.pad") => &[
            "nC.runtime.pad.padLeft",
            "nC.runtime.pad.padRight",
            "nC.runtime.pad.padBoth",
        ],
        (UseDomain::Stoud, "text", "parse", "nC.runtime.parse") => &[
            "nC.runtime.parse.parseInt",
            "nC.runtime.parse.parseUInt",
            "nC.runtime.parse.parseFloat",
            "nC.runtime.parse.parseBool",
            "nC.runtime.parse.parseChar",
            "nC.runtime.parse.tryParseInt",
            "nC.runtime.parse.tryParseFloat",
            "nC.runtime.parse.tryParseBool",
        ],
        (UseDomain::Stoud, "text", "string", "nC.runtime.string") => &[
            "nC.runtime.string.strEmpty",
            "nC.runtime.string.strRelease",
            "nC.runtime.string.strLen",
            "nC.runtime.string.len",
            "nC.runtime.string.strByteLen",
            "nC.runtime.string.strConcat",
            "nC.runtime.string.strSlice",
            "nC.runtime.string.strSubstring",
            "nC.runtime.string.substring",
            "nC.runtime.string.repeatChar",
            "nC.runtime.string.charCode",
            "nC.runtime.string.charFromCode",
            "nC.runtime.string.charToStr",
            "nC.runtime.string.strCompare",
            "nC.runtime.string.strEq",
            "nC.runtime.string.strContains",
            "nC.runtime.string.strIndexOf",
            "nC.runtime.string.strFromInt",
            "nC.runtime.string.strFromBool",
            "nC.runtime.string.strFromFt64",
            "nC.runtime.string.strStartsWith",
            "nC.runtime.string.strEndsWith",
            "nC.runtime.string.strToUpper",
            "nC.runtime.string.strToLower",
            "nC.runtime.string.strTrim",
            "nC.runtime.string.strSplit",
            "nC.runtime.string.strSplitRelease",
        ],
        (UseDomain::Stoud, "text", "text", "nC.runtime.text") => &[
            "nC.runtime.text.textLength",
            "nC.runtime.text.textIsEmpty",
            "nC.runtime.text.textContains",
            "nC.runtime.text.textReplace",
            "nC.runtime.text.textTrim",
            "nC.runtime.text.textSplit",
            "nC.runtime.text.textRelease",
            "nC.runtime.text.textSplitRelease",
        ],
        (UseDomain::Stoud, "text", "truncate", "nC.runtime.truncate") => &[
            "nC.runtime.truncate.truncate",
            "nC.runtime.truncate.truncateEnd",
            "nC.runtime.truncate.truncateMiddle",
            "nC.runtime.truncate.truncateRelease",
        ],
        (UseDomain::Stoud, "text", "unicode", "nC.runtime.unicode") => &[
            "nC.runtime.unicode.isValidUtf8",
            "nC.runtime.unicode.normalizeUnicode",
            "nC.runtime.unicode.stripInvalidUnicode",
            "nC.runtime.unicode.unicodeMode",
            "nC.runtime.unicode.supportsGrapheme",
            "nC.runtime.unicode.normalizeNfc",
            "nC.runtime.unicode.normalizeNfd",
            "nC.runtime.unicode.normalizeNfkc",
            "nC.runtime.unicode.normalizeNfkd",
            "nC.runtime.unicode.graphemeCount",
            "nC.runtime.unicode.graphemeSlice",
            "nC.runtime.unicode.unicodeRelease",
        ],
        (UseDomain::Stoud, "text", "width", "nC.runtime.width") => &[
            "nC.runtime.width.displayWidth",
            "nC.runtime.width.byteLength",
            "nC.runtime.width.charCount",
            "nC.runtime.width.graphemeCount",
            "nC.runtime.width.ansiVisibleWidth",
        ],
        (UseDomain::Stoud, "text", "wrap", "nC.runtime.wrap") => &[
            "nC.runtime.wrap.wrap",
            "nC.runtime.wrap.wrapWords",
            "nC.runtime.wrap.indent",
            "nC.runtime.wrap.wrapRelease",
        ],
        (UseDomain::Stoud, "net", "http", "nC.runtime.http") => &[
            "nC.runtime.http.httpGet",
            "nC.runtime.http.httpPost",
            "nC.runtime.http.httpRequestRaw",
            "nC.runtime.http.httpStatus",
            "nC.runtime.http.httpBody",
            "nC.runtime.http.httpBodyData",
            "nC.runtime.http.httpBodyBytes",
            "nC.runtime.http.httpError",
            "nC.runtime.http.httpRelease",
        ],
        (UseDomain::Stoud, "net", "net", "nC.runtime.net") => &[
            "nC.runtime.net.netStartup",
            "nC.runtime.net.netShutdown",
            "nC.runtime.net.netLastError",
            "nC.runtime.net.netAvailable",
            "nC.runtime.net.netResolve",
            "nC.runtime.net.netAddressOf",
            "nC.runtime.net.netInterfaces",
        ],
        (UseDomain::Stoud, "net", "socket", "nC.runtime.socket") => &[
            "nC.runtime.socket.socketCreate",
            "nC.runtime.socket.socketConnect",
            "nC.runtime.socket.socketBind",
            "nC.runtime.socket.socketListen",
            "nC.runtime.socket.socketAccept",
            "nC.runtime.socket.socketSend",
            "nC.runtime.socket.socketRecv",
            "nC.runtime.socket.socketShutdown",
            "nC.runtime.socket.socketSetTimeout",
            "nC.runtime.socket.socketClose",
            "nC.runtime.socket.socketLastError",
        ],
        (UseDomain::Stoud, "process", "args", "nC.runtime.args") => &[
            "nC.runtime.args.argsOpen",
            "nC.runtime.args.argsCount",
            "nC.runtime.args.argsAt",
            "nC.runtime.args.argsData",
            "nC.runtime.args.argsBytes",
            "nC.runtime.args.argsError",
            "nC.runtime.args.argsRelease",
        ],
        (UseDomain::Stoud, "process", "command", "nC.runtime.command") => &[
            "nC.runtime.command.commandOpen",
            "nC.runtime.command.commandStarted",
            "nC.runtime.command.commandWait",
            "nC.runtime.command.commandExitCode",
            "nC.runtime.command.commandTerminate",
            "nC.runtime.command.commandProcessHandle",
            "nC.runtime.command.commandThreadHandle",
            "nC.runtime.command.commandId",
            "nC.runtime.command.commandThreadId",
            "nC.runtime.command.commandError",
            "nC.runtime.command.commandRelease",
            "nC.runtime.command.commandPipeOpen",
            "nC.runtime.command.commandPipeReadHandle",
            "nC.runtime.command.commandPipeWriteHandle",
            "nC.runtime.command.commandPipeRead",
            "nC.runtime.command.commandPipeWrite",
            "nC.runtime.command.commandPipeCloseRead",
            "nC.runtime.command.commandPipeCloseWrite",
            "nC.runtime.command.commandPipeError",
            "nC.runtime.command.commandPipeRelease",
        ],
        (UseDomain::Stoud, "process", "env", "nC.runtime.env") => &[
            "nC.runtime.env.envOpen",
            "nC.runtime.env.envCount",
            "nC.runtime.env.envAt",
            "nC.runtime.env.envData",
            "nC.runtime.env.envBytes",
            "nC.runtime.env.envError",
            "nC.runtime.env.envRelease",
            "nC.runtime.env.envGet",
            "nC.runtime.env.envHas",
            "nC.runtime.env.envSet",
            "nC.runtime.env.envRemove",
            "nC.runtime.env.envExpand",
            "nC.runtime.env.envLastError",
        ],
        (UseDomain::Stoud, "process", "exit", "nC.runtime.exit") => &[
            "nC.runtime.exit.processCurrent",
            "nC.runtime.exit.processOpen",
            "nC.runtime.exit.processId",
            "nC.runtime.exit.processExitCode",
            "nC.runtime.exit.processTerminate",
            "nC.runtime.exit.processClose",
            "nC.runtime.exit.exitNow",
            "nC.runtime.exit.failFast",
            "nC.runtime.exit.processLastError",
        ],
        (UseDomain::Stoud, "process", "session", "nC.runtime.session") => &[
            "nC.runtime.session.sessionWideAlloc",
            "nC.runtime.session.sessionWideFree",
            "nC.runtime.session.sessionCurrentId",
            "nC.runtime.session.sessionActiveConsoleId",
            "nC.runtime.session.sessionProcessId",
            "nC.runtime.session.sessionServerOpen",
            "nC.runtime.session.sessionServerClose",
            "nC.runtime.session.sessionEnumerate",
            "nC.runtime.session.sessionQuery",
            "nC.runtime.session.sessionFree",
            "nC.runtime.session.sessionQueryUserToken",
            "nC.runtime.session.sessionSendMessage",
            "nC.runtime.session.sessionDisconnect",
            "nC.runtime.session.sessionLogoff",
            "nC.runtime.session.sessionConnect",
            "nC.runtime.session.sessionRegisterNotification",
            "nC.runtime.session.sessionUnregisterNotification",
            "nC.runtime.session.sessionWaitEvent",
            "nC.runtime.session.sessionChannelOpen",
            "nC.runtime.session.sessionChannelClose",
            "nC.runtime.session.sessionChannelRead",
            "nC.runtime.session.sessionChannelWrite",
            "nC.runtime.session.sessionChannelQuery",
            "nC.runtime.session.sessionChannelPurgeInput",
            "nC.runtime.session.sessionChannelPurgeOutput",
            "nC.runtime.session.sessionLastError",
        ],
        (UseDomain::Stoud, "process", "signal", "nC.runtime.signal") => &[
            "nC.runtime.signal.signalWideAlloc",
            "nC.runtime.signal.signalWideFree",
            "nC.runtime.signal.signalCtrlHandler",
            "nC.runtime.signal.signalCtrlEvent",
            "nC.runtime.signal.signalCreateEvent",
            "nC.runtime.signal.signalOpenEvent",
            "nC.runtime.signal.signalSetEvent",
            "nC.runtime.signal.signalResetEvent",
            "nC.runtime.signal.signalPulseEvent",
            "nC.runtime.signal.signalWaitOne",
            "nC.runtime.signal.signalWaitOneEx",
            "nC.runtime.signal.signalWaitMany",
            "nC.runtime.signal.signalWaitManyEx",
            "nC.runtime.signal.signalQueueApc",
            "nC.runtime.signal.signalAddVectored",
            "nC.runtime.signal.signalRemoveVectored",
            "nC.runtime.signal.signalSetUnhandledFilter",
            "nC.runtime.signal.signalRaiseException",
            "nC.runtime.signal.signalClose",
            "nC.runtime.signal.signalLastError",
        ],
        (UseDomain::Stoud, "process", "temp", "nC.runtime.temp") => &[
            "nC.runtime.temp.tempWideAlloc",
            "nC.runtime.temp.tempWideBuffer",
            "nC.runtime.temp.tempWideFree",
            "nC.runtime.temp.tempDirectory",
            "nC.runtime.temp.tempFileName",
            "nC.runtime.temp.tempOpen",
            "nC.runtime.temp.tempDeleteFile",
            "nC.runtime.temp.tempCreateDirectory",
            "nC.runtime.temp.tempRemoveDirectory",
            "nC.runtime.temp.tempMove",
            "nC.runtime.temp.tempAttributes",
            "nC.runtime.temp.tempSetAttributes",
            "nC.runtime.temp.tempFinalPath",
            "nC.runtime.temp.tempClose",
            "nC.runtime.temp.tempLastError",
        ],
        (UseDomain::Stoud, "process", "user", "nC.runtime.user") => &[
            "nC.runtime.user.userBuffer",
            "nC.runtime.user.userFree",
            "nC.runtime.user.userCurrentProcess",
            "nC.runtime.user.userName",
            "nC.runtime.user.userNameEx",
            "nC.runtime.user.userProfileDirectory",
            "nC.runtime.user.userOpenProcessToken",
            "nC.runtime.user.userTokenInformation",
            "nC.runtime.user.userCheckMembership",
            "nC.runtime.user.userCreateWellKnownSid",
            "nC.runtime.user.userDuplicateToken",
            "nC.runtime.user.userSetThreadToken",
            "nC.runtime.user.userImpersonate",
            "nC.runtime.user.userRevert",
            "nC.runtime.user.userLoadProfile",
            "nC.runtime.user.userUnloadProfile",
            "nC.runtime.user.userClose",
            "nC.runtime.user.userLastError",
        ],
        (UseDomain::Stoud, "task", "event", "nC.runtime.event") => &[
            "nC.runtime.event.eventCreate",
            "nC.runtime.event.eventCreateEx",
            "nC.runtime.event.eventOpen",
            "nC.runtime.event.eventSet",
            "nC.runtime.event.eventReset",
            "nC.runtime.event.eventPulse",
            "nC.runtime.event.eventWait",
            "nC.runtime.event.eventWaitEx",
            "nC.runtime.event.eventWaitMany",
            "nC.runtime.event.eventWaitManyEx",
            "nC.runtime.event.eventSignalAndWait",
            "nC.runtime.event.eventClose",
            "nC.runtime.event.eventLastError",
        ],
        (UseDomain::Stoud, "audit", "assert", "nC.runtime.assert") => &[
            "nC.runtime.assert.assert",
            "nC.runtime.assert.assertMsg",
            "nC.runtime.assert.assertEq",
            "nC.runtime.assert.assertEqAddr",
            "nC.runtime.assert.assertNotNull",
            "nC.runtime.assert.assertRange",
            "nC.runtime.assert.assertFail",
            "nC.runtime.assert.assertUnreachable",
        ],
        (UseDomain::Stoud, "audit", "bridge", "nC.runtime.bridge") => &[
            "nC.runtime.bridge.bridgeStride",
            "nC.runtime.bridge.bridgeWrite",
            "nC.runtime.bridge.bridgeFind",
            "nC.runtime.bridge.bridgeActive",
            "nC.runtime.bridge.bridgeRequire",
            "nC.runtime.bridge.bridgeTarget",
            "nC.runtime.bridge.bridgeMode",
            "nC.runtime.bridge.bridgeAuthority",
            "nC.runtime.bridge.bridgeErase",
        ],
        (UseDomain::Stoud, "audit", "debug", "nC.runtime.debug") => &[
            "nC.runtime.debug.debugStride",
            "nC.runtime.debug.debugRecord",
            "nC.runtime.debug.debugTag",
            "nC.runtime.debug.debugLocation",
            "nC.runtime.debug.debugValue",
            "nC.runtime.debug.debugFlags",
            "nC.runtime.debug.debugDump",
            "nC.runtime.debug.debugChecksum",
            "nC.runtime.debug.debugBreak",
            "nC.runtime.debug.debugAssert",
        ],
        (UseDomain::Stoud, "audit", "diagnostic", "nC.runtime.diagnostic") => &[
            "nC.runtime.diagnostic.diagStride",
            "nC.runtime.diagnostic.diagWrite",
            "nC.runtime.diagnostic.diagSeverity",
            "nC.runtime.diagnostic.diagCode",
            "nC.runtime.diagnostic.diagMessage",
            "nC.runtime.diagnostic.diagMessageBytes",
            "nC.runtime.diagnostic.diagSource",
            "nC.runtime.diagnostic.diagLine",
            "nC.runtime.diagnostic.diagColumn",
            "nC.runtime.diagnostic.diagFlags",
            "nC.runtime.diagnostic.diagCopy",
            "nC.runtime.diagnostic.diagClear",
        ],
        (UseDomain::Stoud, "audit", "log", "nC.runtime.log") => &[
            "nC.runtime.log.logStride",
            "nC.runtime.log.logWrite",
            "nC.runtime.log.logLevel",
            "nC.runtime.log.logCategory",
            "nC.runtime.log.logMessage",
            "nC.runtime.log.logMessageBytes",
            "nC.runtime.log.logValue",
            "nC.runtime.log.logFlags",
            "nC.runtime.log.logCopy",
            "nC.runtime.log.logClear",
        ],
        (UseDomain::Stoud, "audit", "profile", "nC.runtime.profile") => &[
            "nC.runtime.profile.profileStride",
            "nC.runtime.profile.profileWrite",
            "nC.runtime.profile.profileMode",
            "nC.runtime.profile.profileFlags",
            "nC.runtime.profile.profileCpuFeatures",
            "nC.runtime.profile.profileOptimization",
            "nC.runtime.profile.profileTarget",
            "nC.runtime.profile.profileTargetBytes",
            "nC.runtime.profile.profileHas",
            "nC.runtime.profile.profileRequire",
            "nC.runtime.profile.profileCopy",
            "nC.runtime.profile.profileClear",
        ],
        (UseDomain::Stoud, "audit", "status", "nC.runtime.status") => &[
            "nC.runtime.status.statusStride",
            "nC.runtime.status.statusWrite",
            "nC.runtime.status.statusKind",
            "nC.runtime.status.statusCode",
            "nC.runtime.status.statusMessage",
            "nC.runtime.status.statusMessageBytes",
            "nC.runtime.status.statusDetail",
            "nC.runtime.status.statusCopy",
            "nC.runtime.status.statusClear",
        ],
        (UseDomain::Stoud, "audit", "trace", "nC.runtime.trace") => &[
            "nC.runtime.trace.traceStride",
            "nC.runtime.trace.traceWrite",
            "nC.runtime.trace.traceKind",
            "nC.runtime.trace.traceId",
            "nC.runtime.trace.traceParent",
            "nC.runtime.trace.traceTick",
            "nC.runtime.trace.tracePayload",
            "nC.runtime.trace.tracePayloadBytes",
            "nC.runtime.trace.traceValue",
            "nC.runtime.trace.traceCopy",
            "nC.runtime.trace.traceClear",
        ],
        (UseDomain::Stoud, "audit", "validate", "nC.runtime.validate") => &[
            "nC.runtime.validate.validateNotNull",
            "nC.runtime.validate.validateRange",
            "nC.runtime.validate.validateLength",
            "nC.runtime.validate.validateEqual",
            "nC.runtime.validate.validateAscii",
            "nC.runtime.validate.validateUtf8",
            "nC.runtime.validate.validateAligned",
            "nC.runtime.validate.validatePowerOfTwo",
            "nC.runtime.validate.validateZero",
            "nC.runtime.validate.validateMask",
        ],
        (UseDomain::Auth, "system", "file", "nC.runtime.authFile") => &[
            "nC.runtime.authFile.authFileStride",
            "nC.runtime.authFile.authFileWrite",
            "nC.runtime.authFile.authFilePathKey",
            "nC.runtime.authFile.authFileHandle",
            "nC.runtime.authFile.authFileRights",
            "nC.runtime.authFile.authFileFlags",
            "nC.runtime.authFile.authFileOwner",
            "nC.runtime.authFile.authFileGeneration",
            "nC.runtime.authFile.authFileSizeHint",
            "nC.runtime.authFile.authFileState",
            "nC.runtime.authFile.authFileAllows",
            "nC.runtime.authFile.authFileCopy",
            "nC.runtime.authFile.authFileClear",
        ],
        (UseDomain::Auth, "system", "dir", "nC.runtime.authDir") => &[
            "nC.runtime.authDir.authDirStride",
            "nC.runtime.authDir.authDirWrite",
            "nC.runtime.authDir.authDirPathKey",
            "nC.runtime.authDir.authDirParentKey",
            "nC.runtime.authDir.authDirHandle",
            "nC.runtime.authDir.authDirRights",
            "nC.runtime.authDir.authDirFlags",
            "nC.runtime.authDir.authDirOwner",
            "nC.runtime.authDir.authDirGeneration",
            "nC.runtime.authDir.authDirState",
            "nC.runtime.authDir.authDirAllows",
            "nC.runtime.authDir.authDirCopy",
            "nC.runtime.authDir.authDirClear",
        ],
        (UseDomain::Auth, "system", "path", "nC.runtime.authPath") => &[
            "nC.runtime.authPath.authPathStride",
            "nC.runtime.authPath.authPathWrite",
            "nC.runtime.authPath.authPathKey",
            "nC.runtime.authPath.authPathBaseKey",
            "nC.runtime.authPath.authPathData",
            "nC.runtime.authPath.authPathBytes",
            "nC.runtime.authPath.authPathRights",
            "nC.runtime.authPath.authPathFlags",
            "nC.runtime.authPath.authPathOwner",
            "nC.runtime.authPath.authPathState",
            "nC.runtime.authPath.authPathAllows",
            "nC.runtime.authPath.authPathCopy",
            "nC.runtime.authPath.authPathClear",
        ],
        (UseDomain::Auth, "system", "env", "nC.runtime.authEnv") => &[
            "nC.runtime.authEnv.authEnvStride",
            "nC.runtime.authEnv.authEnvWrite",
            "nC.runtime.authEnv.authEnvNameKey",
            "nC.runtime.authEnv.authEnvData",
            "nC.runtime.authEnv.authEnvBytes",
            "nC.runtime.authEnv.authEnvRights",
            "nC.runtime.authEnv.authEnvFlags",
            "nC.runtime.authEnv.authEnvOwner",
            "nC.runtime.authEnv.authEnvGeneration",
            "nC.runtime.authEnv.authEnvState",
            "nC.runtime.authEnv.authEnvAllows",
            "nC.runtime.authEnv.authEnvCopy",
            "nC.runtime.authEnv.authEnvClear",
        ],
        (UseDomain::Auth, "system", "permission", "nC.runtime.authPermission") => &[
            "nC.runtime.authPermission.authPermissionStride",
            "nC.runtime.authPermission.authPermissionWrite",
            "nC.runtime.authPermission.authPermissionSubject",
            "nC.runtime.authPermission.authPermissionResource",
            "nC.runtime.authPermission.authPermissionRights",
            "nC.runtime.authPermission.authPermissionDenied",
            "nC.runtime.authPermission.authPermissionFlags",
            "nC.runtime.authPermission.authPermissionOwner",
            "nC.runtime.authPermission.authPermissionGeneration",
            "nC.runtime.authPermission.authPermissionState",
            "nC.runtime.authPermission.authPermissionAllows",
            "nC.runtime.authPermission.authPermissionCopy",
            "nC.runtime.authPermission.authPermissionClear",
        ],
        (UseDomain::Auth, "system", "service", "nC.runtime.authService") => &[
            "nC.runtime.authService.authServiceStride",
            "nC.runtime.authService.authServiceWrite",
            "nC.runtime.authService.authServiceKey",
            "nC.runtime.authService.authServiceHandle",
            "nC.runtime.authService.authServiceProcess",
            "nC.runtime.authService.authServiceRights",
            "nC.runtime.authService.authServiceFlags",
            "nC.runtime.authService.authServiceOwner",
            "nC.runtime.authService.authServiceGeneration",
            "nC.runtime.authService.authServiceState",
            "nC.runtime.authService.authServiceAllows",
            "nC.runtime.authService.authServiceCopy",
            "nC.runtime.authService.authServiceClear",
        ],
        (UseDomain::Auth, "system", "signal", "nC.runtime.authSignal") => &[
            "nC.runtime.authSignal.authSignalStride",
            "nC.runtime.authSignal.authSignalWrite",
            "nC.runtime.authSignal.authSignalKey",
            "nC.runtime.authSignal.authSignalTarget",
            "nC.runtime.authSignal.authSignalCode",
            "nC.runtime.authSignal.authSignalRights",
            "nC.runtime.authSignal.authSignalFlags",
            "nC.runtime.authSignal.authSignalOwner",
            "nC.runtime.authSignal.authSignalGeneration",
            "nC.runtime.authSignal.authSignalState",
            "nC.runtime.authSignal.authSignalAllows",
            "nC.runtime.authSignal.authSignalCopy",
            "nC.runtime.authSignal.authSignalClear",
        ],
        (UseDomain::Auth, "system", "session", "nC.runtime.authSystemSession") => &[
            "nC.runtime.authSystemSession.authSystemSessionStride",
            "nC.runtime.authSystemSession.authSystemSessionWrite",
            "nC.runtime.authSystemSession.authSystemSessionKey",
            "nC.runtime.authSystemSession.authSystemSessionPrincipal",
            "nC.runtime.authSystemSession.authSystemSessionHandle",
            "nC.runtime.authSystemSession.authSystemSessionRights",
            "nC.runtime.authSystemSession.authSystemSessionFlags",
            "nC.runtime.authSystemSession.authSystemSessionOwner",
            "nC.runtime.authSystemSession.authSystemSessionGeneration",
            "nC.runtime.authSystemSession.authSystemSessionState",
            "nC.runtime.authSystemSession.authSystemSessionAllows",
            "nC.runtime.authSystemSession.authSystemSessionCopy",
            "nC.runtime.authSystemSession.authSystemSessionClear",
        ],
        (UseDomain::Auth, "security", "policy", "nC.runtime.authPolicy") => &[
            "nC.runtime.authPolicy.authPolicyStride",
            "nC.runtime.authPolicy.authPolicyWrite",
            "nC.runtime.authPolicy.authPolicyId",
            "nC.runtime.authPolicy.authPolicySubject",
            "nC.runtime.authPolicy.authPolicyResource",
            "nC.runtime.authPolicy.authPolicyRights",
            "nC.runtime.authPolicy.authPolicyDenied",
            "nC.runtime.authPolicy.authPolicyFlags",
            "nC.runtime.authPolicy.authPolicyPriority",
            "nC.runtime.authPolicy.authPolicyState",
            "nC.runtime.authPolicy.authPolicyAllows",
            "nC.runtime.authPolicy.authPolicyCopy",
            "nC.runtime.authPolicy.authPolicyClear",
        ],
        (UseDomain::Auth, "security", "capability", "nC.runtime.authCapability") => &[
            "nC.runtime.authCapability.authCapabilityStride",
            "nC.runtime.authCapability.authCapabilityWrite",
            "nC.runtime.authCapability.authCapabilityId",
            "nC.runtime.authCapability.authCapabilitySubject",
            "nC.runtime.authCapability.authCapabilityResource",
            "nC.runtime.authCapability.authCapabilityRights",
            "nC.runtime.authCapability.authCapabilityScope",
            "nC.runtime.authCapability.authCapabilityFlags",
            "nC.runtime.authCapability.authCapabilityGeneration",
            "nC.runtime.authCapability.authCapabilityState",
            "nC.runtime.authCapability.authCapabilityAllows",
            "nC.runtime.authCapability.authCapabilityCopy",
            "nC.runtime.authCapability.authCapabilityClear",
        ],
        (UseDomain::Auth, "security", "sandbox", "nC.runtime.authSandbox") => &[
            "nC.runtime.authSandbox.authSandboxStride",
            "nC.runtime.authSandbox.authSandboxWrite",
            "nC.runtime.authSandbox.authSandboxId",
            "nC.runtime.authSandbox.authSandboxPrincipal",
            "nC.runtime.authSandbox.authSandboxRoot",
            "nC.runtime.authSandbox.authSandboxRights",
            "nC.runtime.authSandbox.authSandboxRestrictions",
            "nC.runtime.authSandbox.authSandboxFlags",
            "nC.runtime.authSandbox.authSandboxGeneration",
            "nC.runtime.authSandbox.authSandboxState",
            "nC.runtime.authSandbox.authSandboxAllows",
            "nC.runtime.authSandbox.authSandboxCopy",
            "nC.runtime.authSandbox.authSandboxClear",
        ],
        (UseDomain::Auth, "resource", "handle", "nC.runtime.authHandle") => &[
            "nC.runtime.authHandle.handleStride",
            "nC.runtime.authHandle.handleWrite",
            "nC.runtime.authHandle.handleValue",
            "nC.runtime.authHandle.handleKind",
            "nC.runtime.authHandle.handleRights",
            "nC.runtime.authHandle.handleFlags",
            "nC.runtime.authHandle.handleGeneration",
            "nC.runtime.authHandle.handleAllows",
            "nC.runtime.authHandle.handleCopy",
            "nC.runtime.authHandle.handleClear",
        ],
        (UseDomain::Auth, "resource", "resource", "nC.runtime.authResource") => &[
            "nC.runtime.authResource.resourceStride",
            "nC.runtime.authResource.resourceWrite",
            "nC.runtime.authResource.resourceId",
            "nC.runtime.authResource.resourceHandle",
            "nC.runtime.authResource.resourceBase",
            "nC.runtime.authResource.resourceBytes",
            "nC.runtime.authResource.resourceKind",
            "nC.runtime.authResource.resourceRights",
            "nC.runtime.authResource.resourceFlags",
            "nC.runtime.authResource.resourceOwner",
            "nC.runtime.authResource.resourceContains",
            "nC.runtime.authResource.resourceAllows",
            "nC.runtime.authResource.resourceCopy",
            "nC.runtime.authResource.resourceClear",
        ],
        (UseDomain::Auth, "resource", "session", "nC.runtime.authSession") => &[
            "nC.runtime.authSession.sessionStride",
            "nC.runtime.authSession.sessionWrite",
            "nC.runtime.authSession.sessionId",
            "nC.runtime.authSession.sessionOwner",
            "nC.runtime.authSession.sessionToken",
            "nC.runtime.authSession.sessionResources",
            "nC.runtime.authSession.sessionResourceCount",
            "nC.runtime.authSession.sessionRights",
            "nC.runtime.authSession.sessionState",
            "nC.runtime.authSession.sessionHasRight",
            "nC.runtime.authSession.sessionResourceAt",
            "nC.runtime.authSession.sessionCopy",
            "nC.runtime.authSession.sessionClear",
        ],
        (UseDomain::Auth, "security", "audit", "nC.runtime.securityAudit") => &[
            "nC.runtime.securityAudit.securityAuditStride",
            "nC.runtime.securityAudit.securityAuditWrite",
            "nC.runtime.securityAudit.securityAuditEvent",
            "nC.runtime.securityAudit.securityAuditSubject",
            "nC.runtime.securityAudit.securityAuditResource",
            "nC.runtime.securityAudit.securityAuditRights",
            "nC.runtime.securityAudit.securityAuditDecision",
            "nC.runtime.securityAudit.securityAuditTick",
            "nC.runtime.securityAudit.securityAuditDetail",
            "nC.runtime.securityAudit.securityAuditAllows",
            "nC.runtime.securityAudit.securityAuditCopy",
            "nC.runtime.securityAudit.securityAuditClear",
        ],
        (UseDomain::Dom, "compiler", _, "nC.runtime.compilerManual") => &[
            "nC.runtime.compilerManual.compilerUnitStride",
            "nC.runtime.compilerManual.compilerTokenStride",
            "nC.runtime.compilerManual.compilerNodeStride",
            "nC.runtime.compilerManual.compilerIrStride",
            "nC.runtime.compilerManual.compilerDiagnosticStride",
            "nC.runtime.compilerManual.compilerManifestStride",
            "nC.runtime.compilerManual.compilerUnitInit",
            "nC.runtime.compilerManual.compilerParse",
            "nC.runtime.compilerManual.compilerAst",
            "nC.runtime.compilerManual.compilerSema",
            "nC.runtime.compilerManual.compilerLower",
            "nC.runtime.compilerManual.compilerDiagnosticWrite",
            "nC.runtime.compilerManual.compilerDiagnosticCount",
            "nC.runtime.compilerManual.compilerReport",
            "nC.runtime.compilerManual.compilerManifest",
            "nC.runtime.compilerManual.compilerTokenCount",
            "nC.runtime.compilerManual.compilerNodeCount",
            "nC.runtime.compilerManual.compilerIrCount",
            "nC.runtime.compilerManual.compilerState",
            "nC.runtime.compilerManual.compilerReset",
            "nC.runtime.compilerManual.compilerCopy",
        ],
        (UseDomain::Dom, "ai", _, "nC.runtime.aiManual") => &[
            "nC.runtime.aiManual.aiStride",
            "nC.runtime.aiManual.aiWrite",
            "nC.runtime.aiManual.aiId",
            "nC.runtime.aiManual.aiKind",
            "nC.runtime.aiManual.aiInput",
            "nC.runtime.aiManual.aiOutput",
            "nC.runtime.aiManual.aiWeight",
            "nC.runtime.aiManual.aiBias",
            "nC.runtime.aiManual.aiFlags",
            "nC.runtime.aiManual.aiState",
            "nC.runtime.aiManual.aiScore",
            "nC.runtime.aiManual.aiSelect",
            "nC.runtime.aiManual.aiCopy",
            "nC.runtime.aiManual.aiClear",
        ],
        (UseDomain::Dom, "high", _, "nC.runtime.highManual") => &[
            "nC.runtime.highManual.highStride",
            "nC.runtime.highManual.highWrite",
            "nC.runtime.highManual.highKind",
            "nC.runtime.highManual.highKey",
            "nC.runtime.highManual.highValue",
            "nC.runtime.highManual.highAux",
            "nC.runtime.highManual.highFlags",
            "nC.runtime.highManual.highSource",
            "nC.runtime.highManual.highSourceBytes",
            "nC.runtime.highManual.highState",
            "nC.runtime.highManual.highEval",
            "nC.runtime.highManual.highCopyBytes",
            "nC.runtime.highManual.highHashBytes",
            "nC.runtime.highManual.highCopy",
            "nC.runtime.highManual.highClear",
        ],
        (UseDomain::Dom, "inspect", _, "nC.runtime.inspectManual") => &[
            "nC.runtime.inspectManual.inspectStride",
            "nC.runtime.inspectManual.inspectWrite",
            "nC.runtime.inspectManual.inspectEvent",
            "nC.runtime.inspectManual.inspectSite",
            "nC.runtime.inspectManual.inspectValue",
            "nC.runtime.inspectManual.inspectFlags",
            "nC.runtime.inspectManual.inspectTick",
            "nC.runtime.inspectManual.inspectDetail",
            "nC.runtime.inspectManual.inspectSequence",
            "nC.runtime.inspectManual.inspectState",
            "nC.runtime.inspectManual.inspectChecksum",
            "nC.runtime.inspectManual.inspectFind",
            "nC.runtime.inspectManual.inspectCopy",
            "nC.runtime.inspectManual.inspectClear",
        ],
        (UseDomain::Dom, "tooling", _, "nC.runtime.toolingManual") => &[
            "nC.runtime.toolingManual.toolingStride",
            "nC.runtime.toolingManual.toolingWrite",
            "nC.runtime.toolingManual.toolingKind",
            "nC.runtime.toolingManual.toolingPath",
            "nC.runtime.toolingManual.toolingData",
            "nC.runtime.toolingManual.toolingBytes",
            "nC.runtime.toolingManual.toolingFlags",
            "nC.runtime.toolingManual.toolingStatus",
            "nC.runtime.toolingManual.toolingHash",
            "nC.runtime.toolingManual.toolingState",
            "nC.runtime.toolingManual.toolingHashBytes",
            "nC.runtime.toolingManual.toolingCountLines",
            "nC.runtime.toolingManual.toolingCopyBytes",
            "nC.runtime.toolingManual.toolingFindByte",
            "nC.runtime.toolingManual.toolingCopy",
            "nC.runtime.toolingManual.toolingClear",
        ],
        (UseDomain::Stoud, "process", "service", "nC.runtime.service") => &[
            "nC.runtime.service.serviceWideAlloc",
            "nC.runtime.service.serviceWideFree",
            "nC.runtime.service.serviceManagerOpen",
            "nC.runtime.service.serviceOpen",
            "nC.runtime.service.serviceCreate",
            "nC.runtime.service.serviceDelete",
            "nC.runtime.service.serviceStart",
            "nC.runtime.service.serviceControl",
            "nC.runtime.service.serviceControlEx",
            "nC.runtime.service.serviceQueryStatus",
            "nC.runtime.service.serviceQueryConfig",
            "nC.runtime.service.serviceChangeConfig",
            "nC.runtime.service.serviceEnum",
            "nC.runtime.service.serviceEnumDependents",
            "nC.runtime.service.serviceKeyName",
            "nC.runtime.service.serviceDisplayName",
            "nC.runtime.service.serviceManagerLock",
            "nC.runtime.service.serviceManagerUnlock",
            "nC.runtime.service.serviceManagerLockStatus",
            "nC.runtime.service.serviceNotify",
            "nC.runtime.service.serviceStartDispatcher",
            "nC.runtime.service.serviceRegisterHandler",
            "nC.runtime.service.serviceSetStatus",
            "nC.runtime.service.serviceClose",
            "nC.runtime.service.serviceLastError",
        ],
        (UseDomain::Stoud, "math", "core", "nC.math.core") => &[
            "nC.math.core.sqrtFloat",
            "nC.math.core.expFloat",
            "nC.math.core.logFloat",
            "nC.math.core.pi",
            "nC.math.core.euler",
            "nC.math.core.aureo",
            "nC.math.core.piDigits",
            "nC.math.core.eulerDigits",
            "nC.math.core.aureoDigits",
        ],
        (UseDomain::Stoud, "math", "trig", "nC.math.trig") => &[
            "nC.math.trig.sinFloat",
            "nC.math.trig.cosFloat",
            "nC.math.trig.atan2Float",
        ],
        (UseDomain::Stoud, "math", "numeric", "nC.math.numeric") => &[
            "nC.math.numeric.sqrtFloat",
            "nC.math.numeric.expFloat",
            "nC.math.numeric.logFloat",
            "nC.math.numeric.sinFloat",
            "nC.math.numeric.cosFloat",
            "nC.math.numeric.atan2Float",
        ],
        (UseDomain::Stoud, "time", "time", "nC.runtime.time") => &[
            "nC.runtime.time.now",
            "nC.runtime.time.nowMs",
            "nC.runtime.time.timestamp",
            "nC.runtime.time.monotonic",
            "nC.runtime.time.monotonicMs",
            "nC.runtime.time.monotonicNs",
            "nC.runtime.time.sleepUntilMs",
        ],
        (UseDomain::Stoud, "time", "sleep", "nC.runtime.time") => &["nC.runtime.time.sleep"],
        (UseDomain::Stoud, "mobile", "telephony", "nC.runtime.telephony") => &[
            "nC.runtime.telephony.getIncomingNumber",
            "nC.runtime.telephony.getState",
            "nC.runtime.telephony.isAvailable",
            "nC.runtime.telephony.dial",
            "nC.runtime.telephony.answer",
            "nC.runtime.telephony.hangUp",
            "nC.runtime.telephony.rejectCall",
            "nC.runtime.telephony.adb_connect",
        ],
        (UseDomain::Stoud, "mobile", "tts_engine", "nC.runtime.tts") => &[
            "nC.runtime.tts.connect",
            "nC.runtime.tts.speak",
            "nC.runtime.tts.stop",
            "nC.runtime.tts.isSpeaking",
            "nC.runtime.tts.disconnect",
        ],
        (UseDomain::Stoud, "mobile", "permissions", "nC.runtime.permission")
        | (UseDomain::Stoud, "sys", "permissions", "nC.runtime.permission") => &[
            "nC.runtime.permission.request",
            "nC.runtime.permission.cargarPermisos",
            "nC.runtime.permission.canRead",
            "nC.runtime.permission.canWrite",
            "nC.runtime.permission.canExecute",
            "nC.runtime.permission.requirePermission",
        ],
        (UseDomain::Stoud, "mobile", "network", "nC.runtime.mobileNetwork") => &[
            "nC.runtime.mobileNetwork.request",
            "nC.runtime.mobileNetwork.responseStatus",
            "nC.runtime.mobileNetwork.responseJson",
            "nC.runtime.mobileNetwork.responseBody",
            "nC.runtime.mobileNetwork.responseRelease",
            "nC.runtime.mobileNetwork.isWiFiConnected",
            "nC.runtime.mobileNetwork.conectarServidor",
            "nC.runtime.mobileNetwork.server_open",
            "nC.runtime.mobileNetwork.serverReceive",
            "nC.runtime.mobileNetwork.serverSend",
            "nC.runtime.mobileNetwork.serverClose",
            "nC.runtime.mobileNetwork.client_open",
            "nC.runtime.mobileNetwork.clientSend",
            "nC.runtime.mobileNetwork.clientReceive",
            "nC.runtime.mobileNetwork.clientClose",
            "nC.runtime.mobileNetwork.send_bytes",
            "nC.runtime.mobileNetwork.receive_bytes",
            "nC.runtime.mobileNetwork.sendBytesRaw",
            "nC.runtime.mobileNetwork.receiveBytesInto",
        ],
        (UseDomain::Stoud, "network", "server", "nC.runtime.mobileNetwork") => &[
            "nC.runtime.mobileNetwork.server_open",
            "nC.runtime.mobileNetwork.serverReceive",
            "nC.runtime.mobileNetwork.serverSend",
            "nC.runtime.mobileNetwork.serverClose",
        ],
        (UseDomain::Stoud, "network", "client", "nC.runtime.mobileNetwork") => &[
            "nC.runtime.mobileNetwork.client_open",
            "nC.runtime.mobileNetwork.clientReceive",
            "nC.runtime.mobileNetwork.clientSend",
            "nC.runtime.mobileNetwork.clientClose",
        ],
        (UseDomain::Stoud, "network", "send_bytes", "nC.runtime.mobileNetwork") => {
            &["nC.runtime.mobileNetwork.send_bytes"]
        }
        (UseDomain::Stoud, "network", "receive_bytes", "nC.runtime.mobileNetwork") => {
            &["nC.runtime.mobileNetwork.receive_bytes"]
        }
        (UseDomain::Stoud, "network", "server_open", "nC.runtime.mobileNetwork") => {
            &["nC.runtime.mobileNetwork.server_open"]
        }
        (UseDomain::Stoud, "network", "iswificonnected", "nC.runtime.mobileNetwork") => {
            &["nC.runtime.mobileNetwork.isWiFiConnected"]
        }
        (UseDomain::Stoud, "sys", "get_current_time", "nC.runtime.time") => {
            &["nC.runtime.time.now"]
        }
        (UseDomain::Stoud, "sys", "cpu_load", "nC.runtime.system") => {
            &["nC.runtime.system.cpu_load"]
        }
        (UseDomain::Stoud, "sys", "registry_write", "nC.runtime.system") => {
            &["nC.runtime.system.registry_write"]
        }
        (UseDomain::Stoud, "sys", "registry_read", "nC.runtime.system") => {
            &["nC.runtime.system.registry_read"]
        }
        (UseDomain::Stoud, "sys", "sleep", "nC.runtime.time") => &["nC.runtime.time.sleep"],
        (UseDomain::Stoud, "time", "timer", "nC.runtime.timer") => &[
            "nC.runtime.timer.timerStart",
            "nC.runtime.timer.timerStop",
            "nC.runtime.timer.timerReset",
            "nC.runtime.timer.timeout",
        ],
        (UseDomain::Stoud, "ui", "window", "nC.runtime.terminal") => &[
            "nC.runtime.terminal.windowOpen",
            "nC.runtime.terminal.windowShow",
            "nC.runtime.terminal.windowHide",
            "nC.runtime.terminal.windowPoll",
            "nC.runtime.terminal.windowWaitEvent",
            "nC.runtime.terminal.windowClose",
            "nC.runtime.terminal.windowIsOpen",
            "nC.runtime.terminal.windowIsFocused",
            "nC.runtime.terminal.windowSetTitle",
            "nC.runtime.terminal.windowSetSize",
            "nC.runtime.terminal.windowWidth",
            "nC.runtime.terminal.windowHeight",
            "nC.runtime.terminal.windowDpi",
            "nC.runtime.terminal.windowRefreshHz",
            "nC.runtime.terminal.windowState",
            "nC.runtime.terminal.windowSetState",
            "nC.runtime.terminal.windowFocus",
        ],
        (UseDomain::Stoud, "ui", "canvas", "nC.runtime.render") => &[
            "nC.runtime.render.canvasClear",
            "nC.runtime.render.canvasBeginFrame",
            "nC.runtime.render.canvasRect",
            "nC.runtime.render.canvasRoundRect",
            "nC.runtime.render.canvasText",
            "nC.runtime.render.canvasTextStyled",
            "nC.runtime.render.canvasSetQuality",
            "nC.runtime.render.canvasSetFrameMode",
            "nC.runtime.render.canvasFrameDeltaMs",
            "nC.runtime.render.canvasPollResize",
            "nC.runtime.render.canvasSetViewport",
            "nC.runtime.render.canvasPointerX",
            "nC.runtime.render.canvasPointerY",
            "nC.runtime.render.canvasTextCentered",
            "nC.runtime.render.canvasShadowRoundRect",
            "nC.runtime.render.canvasRoundRectAlpha",
            "nC.runtime.render.canvasRoundRectGradient",
            "nC.runtime.render.canvasLine",
            "nC.runtime.render.canvasEllipse",
            "nC.runtime.render.canvasBackdropBlur",
            "nC.runtime.render.canvasBackdropBlurV3",
            "nC.runtime.render.canvasPresent",
        ],
        (UseDomain::Stoud, "ui", "input", "nC.runtime.input") => &[
            "nC.runtime.input.mouseX",
            "nC.runtime.input.mouseY",
            "nC.runtime.input.mouseDown",
            "nC.runtime.input.keyDown",
            "nC.runtime.input.windowReadKey",
            "nC.runtime.input.windowReadChar",
        ],
        (UseDomain::Stoud, "net", "tcp", "nC.runtime.tcp") => &[
            "nC.runtime.tcp.tcpListen",
            "nC.runtime.tcp.tcpListenOn",
            "nC.runtime.tcp.tcpPort",
            "nC.runtime.tcp.tcpConnect",
            "nC.runtime.tcp.tcpConnectTimeout",
            "nC.runtime.tcp.tcpAccept",
            "nC.runtime.tcp.tcpSend",
            "nC.runtime.tcp.tcpRecv",
            "nC.runtime.tcp.tcpSetNonBlocking",
            "nC.runtime.tcp.tcpSetTimeout",
            "nC.runtime.tcp.tcpShutdownRead",
            "nC.runtime.tcp.tcpShutdownWrite",
            "nC.runtime.tcp.tcpClose",
            "nC.runtime.tcp.tcpConnectAsync",
            "nC.runtime.tcp.tcpAcceptAsync",
            "nC.runtime.tcp.tcpSendAsync",
            "nC.runtime.tcp.tcpRecvAsync",
        ],
        (UseDomain::Stoud, "net", "udp", "nC.runtime.udp") => &[
            "nC.runtime.udp.udpBind",
            "nC.runtime.udp.udpBindOn",
            "nC.runtime.udp.udpPort",
            "nC.runtime.udp.udpSetNonBlocking",
            "nC.runtime.udp.udpSetTimeout",
            "nC.runtime.udp.udpSendTo",
            "nC.runtime.udp.udpRecvFrom",
            "nC.runtime.udp.udpClose",
        ],
        (UseDomain::Stoud, "net", "dns", "nC.runtime.dns") => &[
            "nC.runtime.dns.resolve",
            "nC.runtime.dns.resolveAll",
            "nC.runtime.dns.reverseLookup",
        ],
        (UseDomain::Stoud, "net", "tls", "nC.runtime.tls") => &[
            "nC.runtime.tls.tlsConnect",
            "nC.runtime.tls.tlsSend",
            "nC.runtime.tls.tlsRecv",
            "nC.runtime.tls.tlsPeerCert",
            "nC.runtime.tls.tlsClose",
        ],
        (UseDomain::Stoud, "task", "task", "nC.runtime.task") => &[
            "nC.runtime.task.taskSpawn",
            "nC.runtime.task.taskAwait",
            "nC.runtime.task.taskCancel",
            "nC.runtime.task.taskStatus",
        ],
        (UseDomain::Stoud, "task", "async", "nC.runtime.async") => &[
            "nC.runtime.async.asyncRun",
            "nC.runtime.async.await",
            "nC.runtime.async.futureReady",
            "nC.runtime.async.futureCancel",
        ],
        (UseDomain::Stoud, "task", "scheduler", "nC.runtime.scheduler") => &[
            "nC.runtime.scheduler.schedule",
            "nC.runtime.scheduler.scheduleAfter",
            "nC.runtime.scheduler.scheduleEvery",
            "nC.runtime.scheduler.schedulerYield",
        ],
        (UseDomain::Stoud, "task", "cancel", "nC.runtime.cancel") => &[
            "nC.runtime.cancel.cancelToken",
            "nC.runtime.cancel.cancel",
            "nC.runtime.cancel.isCancelled",
            "nC.runtime.cancel.throwIfCancelled",
        ],
        (UseDomain::Stoud, "task", "thread", "nC.runtime.thread") => &[
            "nC.runtime.thread.threadSpawn",
            "nC.runtime.thread.threadJoin",
            "nC.runtime.thread.threadJoinTimeout",
            "nC.runtime.thread.threadId",
            "nC.runtime.thread.threadSleep",
            "nC.runtime.thread.threadLocalCreate",
            "nC.runtime.thread.threadLocalGet",
            "nC.runtime.thread.threadLocalSet",
            "nC.runtime.thread.threadLocalRemove",
        ],
        (UseDomain::Stoud, "sync", "sync", "nC.runtime.sync") => &[
            "nC.runtime.sync.mutexCreate",
            "nC.runtime.sync.lock",
            "nC.runtime.sync.tryLock",
            "nC.runtime.sync.unlock",
            "nC.runtime.sync.barrierCreate",
            "nC.runtime.sync.barrierWait",
            "nC.runtime.sync.rwLockCreate",
            "nC.runtime.sync.readLock",
            "nC.runtime.sync.tryReadLock",
            "nC.runtime.sync.writeLock",
            "nC.runtime.sync.tryWriteLock",
            "nC.runtime.sync.readUnlock",
            "nC.runtime.sync.writeUnlock",
        ],
        (UseDomain::Stoud, "sync", "lock", "nC.runtime.lock") => &[
            "nC.runtime.lock.rwLockCreate",
            "nC.runtime.lock.readLock",
            "nC.runtime.lock.tryReadLock",
            "nC.runtime.lock.writeLock",
            "nC.runtime.lock.tryWriteLock",
            "nC.runtime.lock.readUnlock",
            "nC.runtime.lock.writeUnlock",
            "nC.runtime.lock.unlock",
        ],
        (UseDomain::Stoud, "sync", "atomic", "nC.runtime.atomic") => &[
            "nC.runtime.atomic.atomicCreate",
            "nC.runtime.atomic.atomicLoad",
            "nC.runtime.atomic.atomicStore",
            "nC.runtime.atomic.atomicAdd",
            "nC.runtime.atomic.compareExchange",
            "nC.runtime.atomic.atomicFence",
        ],
        (UseDomain::Stoud, "sync", "channel", "nC.runtime.channel") => &[
            "nC.runtime.channel.stdin",
            "nC.runtime.channel.stdout",
            "nC.runtime.channel.stderr",
            "nC.runtime.channel.writeTo",
            "nC.runtime.channel.readFrom",
            "nC.runtime.channel.flushChannel",
            "nC.runtime.channel.channelCreate",
            "nC.runtime.channel.channelSend",
            "nC.runtime.channel.channelTrySend",
            "nC.runtime.channel.channelRecv",
            "nC.runtime.channel.channelTryRecv",
            "nC.runtime.channel.channelRecvTimeout",
            "nC.runtime.channel.channelClose",
        ],
        (UseDomain::Stoud, "memory", "ptr", "nC.runtime.ptr") => &[
            "nC.runtime.ptr.ptrRead",
            "nC.runtime.ptr.ptrWrite",
            "nC.runtime.ptr.ptrOffset",
            "nC.runtime.ptr.ptrDiff",
        ],
        (UseDomain::Stoud, "memory", "heap", "nC.runtime.heap") => &[
            "nC.runtime.heap.alloc",
            "nC.runtime.heap.free",
            "nC.runtime.heap.heapAlloc",
            "nC.runtime.heap.heapFree",
            "nC.runtime.heap.heapStats",
            "nC.runtime.heap.heapCheck",
        ],
        (UseDomain::Stoud, "memory", "region", "nC.runtime.region") => &[
            "nC.runtime.region.regionFromPtr",
            "nC.runtime.region.regionPtrAt",
            "nC.runtime.region.readAt",
            "nC.runtime.region.writeAt",
        ],
        (UseDomain::Stoud, "memory", "alloc", "nC.runtime.alloc") => &[
            "nC.runtime.alloc.defaultAllocator",
            "nC.runtime.alloc.arenaAllocator",
            "nC.runtime.alloc.allocWith",
            "nC.runtime.alloc.freeWith",
        ],
        (UseDomain::Stoud, "memory", "buffer", "nC.runtime.buffer") => &[
            "nC.runtime.buffer.bufferStride",
            "nC.runtime.buffer.bufferAt",
            "nC.runtime.buffer.bufferData",
            "nC.runtime.buffer.bufferLen",
            "nC.runtime.buffer.bufferCapacity",
            "nC.runtime.buffer.bufferFlags",
            "nC.runtime.buffer.bufferSetLen",
            "nC.runtime.buffer.bufferRead8",
            "nC.runtime.buffer.bufferRead16",
            "nC.runtime.buffer.bufferRead32",
            "nC.runtime.buffer.bufferRead64",
            "nC.runtime.buffer.bufferWrite8",
            "nC.runtime.buffer.bufferWrite16",
            "nC.runtime.buffer.bufferWrite32",
            "nC.runtime.buffer.bufferWrite64",
            "nC.runtime.buffer.bufferFill",
            "nC.runtime.buffer.bufferCopy",
            "nC.runtime.buffer.bufferView",
            "nC.runtime.buffer.bufferClear",
        ],
        (UseDomain::Stoud, "memory", "bytes", "nC.runtime.bytes") => &[
            "nC.runtime.bytes.byteAt",
            "nC.runtime.bytes.bytesToHex",
            "nC.runtime.bytes.hexToBytes",
            "nC.runtime.bytes.bytesEq",
            "nC.runtime.bytes.bytesLen",
        ],
        (UseDomain::Stoud, "memory", "layout", "nC.runtime.layout") => &[
            "nC.runtime.layout.layoutOf",
            "nC.runtime.layout.layoutSize",
            "nC.runtime.layout.layoutAlign",
            "nC.runtime.layout.fieldOffset",
        ],
        (UseDomain::Stoud, "memory", "align", "nC.runtime.align") => &[
            "nC.runtime.align.alignLeft",
            "nC.runtime.align.alignRight",
            "nC.runtime.align.alignCenter",
        ],
        (UseDomain::Stoud, "memory", "slice", "nC.runtime.slice")
        | (UseDomain::Stoud, "collections", "slice", "nC.runtime.slice") => &[
            "nC.runtime.slice.sliceStride",
            "nC.runtime.slice.sliceAt",
            "nC.runtime.slice.sliceData",
            "nC.runtime.slice.sliceLen",
            "nC.runtime.slice.sliceFlags",
            "nC.runtime.slice.sliceRead8",
            "nC.runtime.slice.sliceWrite8",
            "nC.runtime.slice.sliceView",
            "nC.runtime.slice.sliceIsEmpty",
            "nC.runtime.slice.sliceClear",
        ],
        (UseDomain::Stoud, "memory", "endian", "nC.runtime.endian") => &[
            "nC.runtime.endian.nativeEndian",
            "nC.runtime.endian.toLittleEndian",
            "nC.runtime.endian.toBigEndian",
            "nC.runtime.endian.fromLittleEndian",
            "nC.runtime.endian.fromBigEndian",
        ],
        (UseDomain::Stoud, "collections", "array", "nC.runtime.array") => {
            &["nC.runtime.array.index", "nC.runtime.array.len"]
        }
        (UseDomain::Stoud, "collections", "list", "nC.runtime.list") => {
            &["nC.runtime.list.get", "nC.runtime.list.len"]
        }
        (UseDomain::Stoud, "memory", "span", "nC.runtime.span")
        | (UseDomain::Stoud, "collections", "span", "nC.runtime.span") => &[
            "nC.runtime.span.spanStride",
            "nC.runtime.span.spanSegmentStride",
            "nC.runtime.span.spanSegmentAt",
            "nC.runtime.span.spanAt",
            "nC.runtime.span.spanSegments",
            "nC.runtime.span.spanCount",
            "nC.runtime.span.spanLen",
            "nC.runtime.span.spanFlags",
            "nC.runtime.span.spanRead8",
            "nC.runtime.span.spanContains8",
            "nC.runtime.span.spanIsEmpty",
            "nC.runtime.span.spanClear",
        ],
        (UseDomain::Stoud, "memory", "stack", "nC.runtime.stack") => &[
            "nC.runtime.stack.stackDepth",
            "nC.runtime.stack.stackLimit",
            "nC.runtime.stack.stackRemaining",
            "nC.runtime.stack.stackTrace",
            "nC.runtime.stack.stackTraceLen",
            "nC.runtime.stack.stackTraceFrame",
            "nC.runtime.stack.stackTraceRelease",
        ],
        (UseDomain::Stoud, "time", "clock", "nC.runtime.clock") => &[
            "nC.runtime.clock.monotonic",
            "nC.runtime.clock.elapsed",
            "nC.runtime.clock.measure",
            "nC.runtime.clock.clockResolution",
        ],
        (UseDomain::Stoud, "math", "sign", "nC.runtime.math") => {
            &["nC.runtime.math.sign", "nC.runtime.math.abs"]
        }
        (UseDomain::Stoud, "text", "convert", "nC.runtime.convert") => &[
            "nC.runtime.convert.toStr",
            "nC.runtime.convert.toText",
            "nC.runtime.convert.toInt",
            "nC.runtime.convert.toFloat",
            "nC.runtime.convert.toBool",
            "nC.runtime.convert.safeToInt",
            "nC.runtime.convert.forceToInt",
        ],
        (UseDomain::Stoud, "text", "escape", "nC.runtime.escape") => &[
            "nC.runtime.escape.escapeAnsi",
            "nC.runtime.escape.escapeJson",
            "nC.runtime.escape.escapeControl",
            "nC.runtime.escape.sanitizeTerminal",
            "nC.runtime.escape.sanitizeLog",
        ],
        (UseDomain::Stoud, "text", "format", "nC.runtime.format") => &[
            "nC.runtime.format.format",
            "nC.runtime.format.formatNamed",
            "nC.runtime.format.formatInt",
            "nC.runtime.format.formatFloat",
        ],
        (UseDomain::Stoud, "collections", "result", "nC.runtime.result") => &[
            "nC.runtime.result.Ok",
            "nC.runtime.result.Err",
            "nC.runtime.result.isOk",
            "nC.runtime.result.isErr",
            "nC.runtime.result.unwrap",
        ],
        (UseDomain::Stoud, "collections", "error", "nC.runtime.error") => &[
            "nC.runtime.error.errorCode",
            "nC.runtime.error.errorMessage",
            "nC.runtime.error.errorKind",
            "nC.runtime.error.errorOsCause",
            "nC.runtime.error.raiseRuntime",
            "nC.runtime.error.clearError",
            "nC.runtime.error.lastError",
        ],
        (UseDomain::Auth, "system", "process", "nC.runtime.process") => &[
            "nC.runtime.process.processId",
            "nC.runtime.process.parentProcessId",
            "nC.runtime.process.spawn",
            "nC.runtime.process.spawnPipe",
            "nC.runtime.process.spawnIn",
            "nC.runtime.process.spawnWithEnv",
            "nC.runtime.process.spawnWithOptions",
            "nC.runtime.process.spawnGroup",
            "nC.runtime.process.wait",
            "nC.runtime.process.waitStatus",
            "nC.runtime.process.waitTimeout",
            "nC.runtime.process.writeStdin",
            "nC.runtime.process.closeStdin",
            "nC.runtime.process.readStdout",
            "nC.runtime.process.readStderr",
            "nC.runtime.process.statusCode",
            "nC.runtime.process.statusSignal",
            "nC.runtime.process.statusSuccess",
            "nC.runtime.process.statusTimedOut",
            "nC.runtime.process.kill",
            "nC.runtime.process.signal",
            "nC.runtime.process.signalGroup",
            "nC.runtime.process.killGroup",
        ],
        (UseDomain::Auth, "system", "thread", "nC.runtime.thread") => &[
            "nC.runtime.thread.threadSpawn",
            "nC.runtime.thread.threadJoin",
            "nC.runtime.thread.threadJoinTimeout",
            "nC.runtime.thread.threadId",
            "nC.runtime.thread.threadSleep",
            "nC.runtime.thread.threadLocalCreate",
            "nC.runtime.thread.threadLocalGet",
            "nC.runtime.thread.threadLocalSet",
            "nC.runtime.thread.threadLocalRemove",
        ],
        (UseDomain::Stoud, "os", _, "nC.runtime.os")
        | (UseDomain::Auth, "system", "os", "nC.runtime.os")
        | (UseDomain::Auth, "system", "media", "nC.runtime.os") => &[
            "nC.runtime.os.osName",
            "nC.runtime.os.osVersion",
            "nC.runtime.os.osArch",
            "nC.runtime.os.osFamily",
            "nC.runtime.os.isWindows",
            "nC.runtime.os.isLinux",
            "nC.runtime.os.isMac",
            "nC.runtime.os.openUri",
            "nC.runtime.os.mediaNext",
            "nC.runtime.os.mediaPlay",
            "nC.runtime.os.mediaPause",
            "nC.runtime.os.mediaPrevious",
        ],
        _ => &[],
    };
    let mut out = symbols
        .iter()
        .map(|symbol| (*symbol).to_string())
        .collect::<Vec<_>>();
    if out.is_empty()
        && matches!(domain, UseDomain::Stoud)
        && matches!(
            family.as_str(),
            "runtime" | "fs" | "data" | "process" | "thread" | "time" | "terminal"
        )
        && official.starts_with("nC.runtime.")
    {
        if let Some(fam) = runtime_api_find_family_by_import_path(
            &official.split('.').map(String::from).collect::<Vec<_>>(),
        ) {
            out.extend(
                fam.functions
                    .iter()
                    .map(|function| format!("{}.{}", fam.import_path, function.name)),
            );
        } else {
            out.push(format!("{official}.__family_activation"));
        }
    }
    if out.is_empty() && matches!(domain, UseDomain::Stoud) && family == "math" {
        out.push(format!("{official}.__native_intrinsic_activation"));
    }
    if out.is_empty()
        && matches!(domain, UseDomain::Stoud)
        && family == "collections"
        && official.starts_with("nC.vector.")
    {
        out.push(format!("{official}.__native_vector_activation"));
    }
    out.into_iter()
        .map(|symbol| canonical_runtime_symbol(&symbol))
        .collect()
}

fn use_backend_lowering_kind(domain: UseDomain, risk: UseRiskLevel) -> &'static str {
    match (domain, risk) {
        (UseDomain::Auth, _) => "authority_gate_thunk",
        (UseDomain::Dom, _) => "domain_service_thunk",
        (_, UseRiskLevel::Medium) => "runtime_capability_thunk",
        (_, UseRiskLevel::High | UseRiskLevel::Extreme) => "authority_boundary_thunk",
        _ => "runtime_import_thunk",
    }
}

fn use_backend_family_group(domain: UseDomain, family: &str) -> String {
    format!("{}::{}", domain_label(domain), family)
}

fn stable_import_id(domain: UseDomain, family: &str, item: &str, expands_to: &[String]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in domain_label(domain)
        .bytes()
        .chain([b':', b':'])
        .chain(family.bytes())
        .chain([b':', b':'])
        .chain(item.bytes())
        .chain([b':'])
        .chain(expands_to.join("|").bytes())
    {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn wasm_i32_const_bytes(mut value: u32) -> Vec<u8> {
    let mut out = vec![0x41];
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out.push(0x0b);
    out
}

fn use_backend_target_bytes(stable_id: u32) -> Vec<UseBackendTargetBytes> {
    let x86 = {
        let mut bytes = vec![0xb8];
        bytes.extend_from_slice(&stable_id.to_le_bytes());
        bytes.push(0xc3);
        bytes
    };
    let aarch64 = {
        let imm16 = stable_id & 0xffff;
        let mov_w0_imm = 0x5280_0000u32 | (imm16 << 5);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&mov_w0_imm.to_le_bytes());
        bytes.extend_from_slice(&0xd65f_03c0u32.to_le_bytes());
        bytes
    };
    let wasm = wasm_i32_const_bytes(stable_id);
    vec![
        UseBackendTargetBytes {
            profile: "nC.sysv64",
            target_triple: "x86_64-unknown-linux-gnu",
            architecture: "x86_64",
            object_format: "elf64",
            instruction: "mov eax, import_id; ret",
            bytes_hex: bytes_hex(&x86),
            byte_count: x86.len(),
        },
        UseBackendTargetBytes {
            profile: "nC.win64",
            target_triple: "x86_64-pc-windows-msvc",
            architecture: "x86_64",
            object_format: "coff",
            instruction: "mov eax, import_id; ret",
            bytes_hex: bytes_hex(&x86),
            byte_count: x86.len(),
        },
        UseBackendTargetBytes {
            profile: "nC.aapcs64",
            target_triple: "aarch64-apple-darwin",
            architecture: "aarch64",
            object_format: "macho",
            instruction: "mov w0, import_id16; ret",
            bytes_hex: bytes_hex(&aarch64),
            byte_count: aarch64.len(),
        },
        UseBackendTargetBytes {
            profile: "nC.portable",
            target_triple: "wasm32-wasi",
            architecture: "wasm32",
            object_format: "wasm-module",
            instruction: "i32.const import_id; end",
            bytes_hex: bytes_hex(&wasm),
            byte_count: wasm.len(),
        },
    ]
}

fn use_backend_routes(
    backend_symbol: &str,
    runtime_symbols: &[String],
    has_backing: bool,
) -> Vec<UseBackendRoute> {
    let symbols = if runtime_symbols.is_empty() {
        "language intrinsic without external runtime symbol".to_string()
    } else {
        runtime_symbols.join(", ")
    };
    if !has_backing {
        return vec![
            UseBackendRoute {
                route: "ncc_run",
                status: "catalog_only_with_diagnostic",
                lowering: format!(
                    "{backend_symbol} activates the import namespace, but no real runtime call is allowed"
                ),
                authority: "runtime import diagnostic",
                diagnostic_if_unavailable: Some(USE_BACKING_DEFERRED_DIAGNOSTIC),
            },
            UseBackendRoute {
                route: "mct",
                status: "closed_activation_bytes_only",
                lowering: format!(
                    "{backend_symbol} emits deterministic activation bytes; runtime side effects are blocked until backing exists"
                ),
                authority: "MCT activation byte contract",
                diagnostic_if_unavailable: Some(USE_BACKING_DEFERRED_DIAGNOSTIC),
            },
            UseBackendRoute {
                route: "native_fast",
                status: "catalog_only_with_diagnostic",
                lowering: "native-fast refuses a runtime thunk for catalog-only imports".to_string(),
                authority: "native-fast gate",
                diagnostic_if_unavailable: Some(USE_BACKING_DEFERRED_DIAGNOSTIC),
            },
            UseBackendRoute {
                route: "llvm",
                status: "catalog_only_with_diagnostic",
                lowering: "LLVM declaration/call emission is blocked until a runtime ABI row exists".to_string(),
                authority: "LLVM/MCT runtime ABI gate",
                diagnostic_if_unavailable: Some(USE_BACKING_DEFERRED_DIAGNOSTIC),
            },
            UseBackendRoute {
                route: "machine",
                status: "import_forbidden_bridge_required",
                lowering: "direct high-level use imports are rejected in [@machine]; only explicit bridge/trap contracts may cross".to_string(),
                authority: "machine authority boundary",
                diagnostic_if_unavailable: Some("E_MACHINE_RUNTIME_IMPORT_FORBIDDEN_001"),
            },
            UseBackendRoute {
                route: "basal",
                status: "import_forbidden_bridge_required",
                lowering: "direct high-level use imports are rejected in basal/kernel layers; bridge materialization must be explicit".to_string(),
                authority: "basal authority boundary",
                diagnostic_if_unavailable: Some("E_BASAL_RUNTIME_IMPORT_FORBIDDEN_001"),
            },
        ];
    }

    vec![
        UseBackendRoute {
            route: "ncc_run",
            status: "closed_real_executor",
            lowering: format!(
                "ncc run calls {symbols} through the real runtime executor without requiring --real"
            ),
            authority: "runtime executor",
            diagnostic_if_unavailable: None,
        },
        UseBackendRoute {
            route: "mct",
            status: "closed_runtime_symbol_activation_bytes",
            lowering: format!(
                "{backend_symbol} emits deterministic activation bytes and binds runtime symbols: {symbols}"
            ),
            authority: "MCT byte contract",
            diagnostic_if_unavailable: Some("E_BACKEND_PARITY_MCT_BYTES_001"),
        },
        UseBackendRoute {
            route: "native_fast",
            status: "closed_native_runtime_thunk",
            lowering: format!(
                "native-fast lowers the call to an nC runtime-value thunk for: {symbols}"
            ),
            authority: "native-fast runtime thunk",
            diagnostic_if_unavailable: Some("E_BACKEND_PARITY_NATIVE_FAST_001"),
        },
        UseBackendRoute {
            route: "llvm",
            status: "closed_llvm_runtime_declaration_call",
            lowering: format!(
                "LLVM/MCT declares and calls {symbols} using nC.runtime-value-v1 layouts, not an inferred raw C ABI"
            ),
            authority: "LLVM hosted runtime ABI",
            diagnostic_if_unavailable: Some("E_BACKEND_PARITY_LLVM_EXEC_001"),
        },
        UseBackendRoute {
            route: "machine",
            status: "closed_bridge_required_when_applicable",
            lowering: "direct runtime imports stay forbidden in [@machine]; applicable calls cross by explicit typed bridge/trap rows".to_string(),
            authority: "machine low-authority boundary",
            diagnostic_if_unavailable: Some("E_MACHINE_RUNTIME_IMPORT_FORBIDDEN_001"),
        },
        UseBackendRoute {
            route: "basal",
            status: "closed_bridge_required_when_applicable",
            lowering: "basal does not import high-level runtime directly; applicable calls cross by explicit kernel/basal bridge rows".to_string(),
            authority: "basal low-authority boundary",
            diagnostic_if_unavailable: Some("E_BASAL_RUNTIME_IMPORT_FORBIDDEN_001"),
        },
    ]
}

fn use_backend_entry_from_alias(alias: UseAliasExpansion) -> UseBackendLoweringEntry {
    let stable_id = stable_import_id(alias.domain, &alias.family, &alias.item, &alias.expands_to);
    let lowering_kind = use_backend_lowering_kind(alias.domain, alias.risk).to_string();
    let has_backing = alias.diagnostic.is_none();
    let closure_status =
        runtime_import_closure_status(alias.domain, &alias.family, &alias.item, has_backing);
    let runtime_symbols =
        use_backend_runtime_symbols(alias.domain, &alias.family, &alias.item, &alias.expands_to);
    let backend_symbol = use_backend_symbol(alias.domain, &alias.family, &alias.item);
    let backend_routes = use_backend_routes(&backend_symbol, &runtime_symbols, has_backing);
    let runtime_call_lowering = if has_backing && !runtime_symbols.is_empty() {
        "direct_runtime_symbol_call_without_real_flag".to_string()
    } else if has_backing {
        "language_runtime_intrinsic_call_without_real_flag".to_string()
    } else {
        "catalog_only_activation_bytes_no_runtime_call_lowering".to_string()
    };
    let runtime_semantics = if !has_backing {
        format!(
            "{}; import {} still emits deterministic activation bytes, but the resolved runtime/domain target must be implemented before treating it as backed",
            USE_BACKING_DEFERRED_DIAGNOSTIC,
            alias.expands_to.join(",")
        )
    } else if matches!(alias.domain, UseDomain::Auth) {
        format!(
            "authority checked import boundary for {}; concrete side effects are performed by the resolved runtime/system call",
            alias.expands_to.join(",")
        )
    } else {
        format!(
            "resolved runtime/domain symbol {}; the import activation thunk materializes a concrete backend/MCT symbol",
            alias.expands_to.join(",")
        )
    };
    let runtime_semantics =
        if let Some(note) = use_runtime_provider_note(alias.domain, &alias.family, &alias.item) {
            format!("{runtime_semantics}; {note}")
        } else {
            runtime_semantics
        };
    UseBackendLoweringEntry {
        surface_import: alias.surface_import,
        expands_to: alias.expands_to,
        domain: alias.domain,
        family: alias.family.clone(),
        item: alias.item.clone(),
        risk: alias.risk,
        requires_policy: alias.requires_policy,
        family_group: use_backend_family_group(alias.domain, &alias.family),
        backend_symbol,
        runtime_symbols,
        mct_provider: use_runtime_stdlib_package(has_backing),
        mct_profiles: vec!["nC.sysv64", "nC.win64", "nC.aapcs64", "nC.portable"],
        lowering_kind,
        linkage: "strong local import activation symbol + resolved runtime/domain target"
            .to_string(),
        runtime_call_lowering,
        backend_routes,
        backend_parity_status: if has_backing {
            "closed_for_ncc_run_mct_native_fast_llvm_and_machine_basal_bridge"
        } else {
            "catalog_only_activation_bytes_with_backend_diagnostic"
        },
        runtime_semantics,
        emitted_bytes: use_backend_target_bytes(stable_id),
        backing_status: if has_backing {
            "closed_real_backing_bytes"
        } else {
            "closed_catalog_only_with_activation_bytes"
        },
        closed_real_backing_bytes: has_backing,
        closure_status: closure_status.as_str(),
        can_run_without_runtime_import_failure: !matches!(
            closure_status,
            RuntimeImportClosureStatus::CatalogOnlyWithDiagnostic
        ),
        status: if has_backing {
            "closed_real_backing_bytes"
        } else {
            "closed_catalog_only_with_activation_bytes"
        },
        diagnostics: if has_backing {
            Vec::new()
        } else {
            vec![USE_BACKING_DEFERRED_DIAGNOSTIC]
        },
    }
}

fn use_backend_manifest_from_aliases(
    aliases: Vec<UseAliasExpansion>,
) -> UseBackendLoweringManifest {
    let entries = aliases
        .into_iter()
        .map(use_backend_entry_from_alias)
        .collect::<Vec<_>>();
    let mut groups = Vec::new();
    for domain in [
        UseDomain::Core,
        UseDomain::Alloc,
        UseDomain::Stoud,
        UseDomain::Auth,
        UseDomain::Dom,
    ] {
        if let Some(families) = family_table(domain) {
            for (family, _, family_risk) in families {
                let group_entries = entries
                    .iter()
                    .filter(|entry| {
                        entry.domain == domain && entry.family.eq_ignore_ascii_case(family)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if group_entries.is_empty() {
                    continue;
                }
                groups.push(UseBackendFamilyGroup {
                    domain,
                    family: (*family).to_string(),
                    risk: *family_risk,
                    requires_policy: false,
                    status: if group_entries
                        .iter()
                        .all(|entry| entry.diagnostics.is_empty())
                    {
                        "closed_real_backing_bytes"
                    } else {
                        "closed_mixed_backing_with_activation_bytes"
                    },
                    entries: group_entries,
                });
            }
        }
    }
    let total_entries = entries.len();
    let total_families = groups.len();
    UseBackendLoweringManifest {
        schema: "nc.imports.backend_lowering.v1",
        status: "closed",
        mct_connection: "connected_to_mct_import_activation_and_runtime_symbols",
        entries,
        groups,
        total_entries,
        total_families,
        truth_boundary: vec![
            "each supported use::stoud/auth/dom item has a deterministic backend symbol",
            "each supported item has concrete activation bytes for sysv64, win64, aapcs64 and portable wasm profiles",
            "imports with runtime/language backing are marked closed_real_backing_bytes",
            "runtime-backed import families are distributed as the external ncc-runtime-stdlib package shipped with the toolchain, while MCT keeps deterministic activation bytes",
            "runtime closure status is the single truth layer: closed_real_backing_bytes, closed_native_backend, closed_interpreter_runtime or catalog_only_with_diagnostic",
            "real runtime-backed imports expose the runtime symbols called by ncc run without --real",
            "each import row declares backend_routes for ncc_run, mct, native_fast, llvm, machine and basal",
            "machine/basal direct imports stay forbidden; applicable runtime effects cross only through explicit bridge/trap rows",
            "catalog-only imports are kept visible with E_USE_BACKING_DEFERRED_001 instead of pretending backend support",
            "auth imports lower to authority gates and do not fake filesystem/process/security side effects",
            "runtime side effects remain owned by the resolved runtime/system symbol",
            "machine, primitive, basal and kernel remain import-forbidden modes",
        ],
    }
}

pub fn use_backend_lowering_manifest_for_domain(domain: UseDomain) -> UseBackendLoweringManifest {
    use_backend_manifest_from_aliases(use_import_manifest_for_domain(domain).entries)
}

pub fn use_backend_lowering_manifest_all() -> UseBackendLoweringManifest {
    use_backend_manifest_from_aliases(use_import_manifest_all().entries)
}

pub fn use_backend_lowering_manifest_for_paths(paths: &[String]) -> UseBackendLoweringManifest {
    let aliases = use_import_manifest_all()
        .entries
        .into_iter()
        .filter(|entry| {
            entry
                .expands_to
                .iter()
                .any(|expanded| paths.iter().any(|path| expanded.eq_ignore_ascii_case(path)))
        })
        .collect::<Vec<_>>();
    use_backend_manifest_from_aliases(aliases)
}

pub fn use_backend_lowering_manifest_for_aliases(
    aliases: Vec<UseAliasExpansion>,
) -> UseBackendLoweringManifest {
    use_backend_manifest_from_aliases(aliases)
}

pub fn use_backend_lowering_manifest_for_aliases_and_paths(
    mut aliases: Vec<UseAliasExpansion>,
    paths: &[String],
) -> UseBackendLoweringManifest {
    let path_aliases = use_import_manifest_all()
        .entries
        .into_iter()
        .filter(|entry| {
            entry
                .expands_to
                .iter()
                .any(|expanded| paths.iter().any(|path| expanded.eq_ignore_ascii_case(path)))
        })
        .collect::<Vec<_>>();
    for alias in path_aliases {
        if !aliases.iter().any(|existing| {
            existing
                .surface_import
                .eq_ignore_ascii_case(&alias.surface_import)
        }) {
            aliases.push(alias);
        }
    }
    use_backend_manifest_from_aliases(aliases)
}

fn runtime_import_closure_entry_from_backend(
    entry: UseBackendLoweringEntry,
) -> RuntimeImportClosureEntry {
    let closure_status = match entry.closure_status {
        "closed_real_backing_bytes" => RuntimeImportClosureStatus::ClosedRealBackingBytes,
        "closed_native_backend" => RuntimeImportClosureStatus::ClosedNativeBackend,
        "closed_interpreter_runtime" => RuntimeImportClosureStatus::ClosedInterpreterRuntime,
        _ => RuntimeImportClosureStatus::CatalogOnlyWithDiagnostic,
    };
    RuntimeImportClosureEntry {
        surface_import: entry.surface_import,
        expands_to: entry.expands_to,
        domain: entry.domain,
        family: entry.family,
        item: entry.item,
        closure_status: closure_status.as_str(),
        can_run_without_runtime_import_failure: !matches!(
            closure_status,
            RuntimeImportClosureStatus::CatalogOnlyWithDiagnostic
        ),
        backend_symbol: entry.backend_symbol,
        runtime_symbols: entry.runtime_symbols,
        mct_profiles: entry.mct_profiles,
        run_contract: runtime_import_run_contract(closure_status),
        diagnostics: entry.diagnostics,
    }
}

fn runtime_import_closure_table_from_backend(
    manifest: UseBackendLoweringManifest,
) -> RuntimeImportClosureTable {
    let entries = manifest
        .entries
        .into_iter()
        .map(runtime_import_closure_entry_from_backend)
        .collect::<Vec<_>>();
    let total_entries = entries.len();
    let closed_real_backing_bytes = entries
        .iter()
        .filter(|entry| entry.closure_status == "closed_real_backing_bytes")
        .count();
    let closed_native_backend = entries
        .iter()
        .filter(|entry| entry.closure_status == "closed_native_backend")
        .count();
    let closed_interpreter_runtime = entries
        .iter()
        .filter(|entry| entry.closure_status == "closed_interpreter_runtime")
        .count();
    let catalog_only_with_diagnostic = entries
        .iter()
        .filter(|entry| entry.closure_status == "catalog_only_with_diagnostic")
        .count();
    RuntimeImportClosureTable {
        schema: "nc.imports.runtime_closure_table.v1",
        status: "closed",
        allowed_statuses: RUNTIME_IMPORT_CLOSURE_STATUSES.to_vec(),
        entries,
        total_entries,
        closed_real_backing_bytes,
        closed_native_backend,
        closed_interpreter_runtime,
        catalog_only_with_diagnostic,
        truth_boundary: vec![
            "every use::stoud/auth/dom import item receives exactly one runtime closure status",
            "closed_real_backing_bytes means ncc run can call a real runtime/OS symbol and the import has MCT activation bytes",
            "closed_native_backend means ncc run can use the language/native backend intrinsic and the import has MCT activation bytes",
            "closed_interpreter_runtime means ncc run is supported by the current runtime executor and the import has MCT activation bytes",
            "catalog_only_with_diagnostic means the import is listed but must not be treated as runnable backend support",
            "catalog-only imports carry E_USE_BACKING_DEFERRED_001 instead of failing later as a surprise in ncc run",
        ],
    }
}

pub fn runtime_import_closure_table_for_domain(domain: UseDomain) -> RuntimeImportClosureTable {
    runtime_import_closure_table_from_backend(use_backend_lowering_manifest_for_domain(domain))
}

pub fn runtime_import_closure_table_all() -> RuntimeImportClosureTable {
    runtime_import_closure_table_from_backend(use_backend_lowering_manifest_all())
}

pub fn runtime_import_closure_table_for_paths(paths: &[String]) -> RuntimeImportClosureTable {
    runtime_import_closure_table_from_backend(use_backend_lowering_manifest_for_paths(paths))
}

pub fn runtime_import_closure_table_for_aliases(
    aliases: Vec<UseAliasExpansion>,
) -> RuntimeImportClosureTable {
    runtime_import_closure_table_from_backend(use_backend_lowering_manifest_for_aliases(aliases))
}

pub fn runtime_import_closure_table_for_aliases_and_paths(
    aliases: Vec<UseAliasExpansion>,
    paths: &[String],
) -> RuntimeImportClosureTable {
    runtime_import_closure_table_from_backend(use_backend_lowering_manifest_for_aliases_and_paths(
        aliases, paths,
    ))
}

pub fn known_use_import_path_count() -> usize {
    KNOWN_USE_IMPORT_PATHS.len()
}

fn runtime_family_suffix_lookup(segment: &str) -> Option<&'static str> {
    if segment.eq_ignore_ascii_case("list") {
        return Some("queue");
    }
    NC_RUNTIME_FAMILIES.iter().find_map(|entry| {
        let mut parts = entry.split('.');
        let head = parts.next()?;
        let mid = parts.next()?;
        let suffix = parts.next()?;
        if head.eq_ignore_ascii_case("nC")
            && mid.eq_ignore_ascii_case("runtime")
            && suffix.eq_ignore_ascii_case(segment)
        {
            Some(suffix)
        } else {
            None
        }
    })
}

fn is_legacy_runtime_family_import_path(path: &[String]) -> bool {
    path.len() == 2
        && path[0].eq_ignore_ascii_case("runtime")
        && runtime_family_suffix_lookup(&path[1]).is_some()
}

fn is_legacy_runtime_wildcard_root(path: &[String]) -> bool {
    path.len() == 1 && path[0].eq_ignore_ascii_case("runtime")
}

pub fn is_project_use_import_path(path: &[String]) -> bool {
    path.len() >= 2 && path[0].eq_ignore_ascii_case("project")
}

pub fn is_known_use_import_path(path: &[String]) -> bool {
    KNOWN_USE_IMPORT_PATHS
        .iter()
        .any(|candidate| import_path_matches_dotted(path, candidate))
        || NC_RUNTIME_FAMILIES
            .iter()
            .any(|candidate| import_path_matches_dotted(path, candidate))
        || runtime_api_find_family_by_import_path(path).is_some()
        || is_legacy_runtime_family_import_path(path)
        || is_legacy_runtime_wildcard_root(path)
        || is_project_use_import_path(path)
}

#[cfg(test)]
mod import_catalog_tests {
    use super::*;

    #[test]
    fn known_use_import_catalog_has_mature_surface() {
        assert!(
            known_use_import_path_count() >= 250,
            "expected >=250 official import paths, got {}",
            known_use_import_path_count()
        );
    }

    #[test]
    fn known_use_import_catalog_accepts_case_insensitive_paths() {
        let a = vec!["System".to_string(), "net".to_string(), "core".to_string()];
        assert!(is_known_use_import_path(&a));

        let b = vec![
            "runtime".to_string(),
            "sync".to_string(),
            "channel".to_string(),
        ];
        assert!(is_known_use_import_path(&b));
    }

    #[test]
    fn known_use_import_catalog_keeps_unknown_paths_out() {
        let ghost = vec!["system".to_string(), "net".to_string(), "ghost".to_string()];
        assert!(!is_known_use_import_path(&ghost));
    }

    #[test]
    fn known_use_import_catalog_covers_every_kernel_operational_family() {
        for family in [
            "task",
            "schedule",
            "control",
            "cpu",
            "ipi",
            "memory",
            "interrupt",
            "syscall",
            "driver",
            "user",
            "boot",
        ] {
            assert!(
                is_known_use_import_path(&[
                    "kernel".to_string(),
                    family.to_string(),
                    "core".to_string(),
                ]),
                "kernel.{family}.core must be canonical"
            );
            assert!(
                is_known_use_import_path(&[
                    "nC".to_string(),
                    "kernel".to_string(),
                    family.to_string(),
                ]),
                "nC.kernel.{family} must resolve to its canonical module"
            );
        }
    }

    #[test]
    fn official_runtime_families_are_frozen() {
        assert_eq!(
            official_runtime_families().len(),
            96,
            "runtime must expose 90 base families plus six native extensions"
        );
        assert!(
            is_known_use_import_path(&[
                "nC".to_string(),
                "runtime".to_string(),
                "input".to_string()
            ]),
            "nC.runtime.input must resolve as known import path"
        );
        assert!(
            is_known_use_import_path(&[
                "nC".to_string(),
                "runtime".to_string(),
                "resource".to_string()
            ]),
            "nC.runtime.resource must resolve as known import path"
        );
        assert!(
            is_known_use_import_path(&["runtime".to_string(), "input".to_string()]),
            "legacy runtime.input alias must remain accepted"
        );
        assert!(
            is_known_use_import_path(&["runtime".to_string()]),
            "legacy runtime.* root alias must remain accepted"
        );
    }

    #[test]
    fn runtime_api_families_are_parseable() {
        let families = runtime_api_family_specs();
        assert_eq!(
            families.len(),
            117,
            "runtime API parser must expose every material runtime family"
        );
        let total_functions: usize = families.iter().map(|f| f.functions.len()).sum();
        assert_eq!(
            total_functions, 1283,
            "runtime API parser must expose the complete typed material function catalog"
        );

        let input = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "input".to_string(),
        ])
        .expect("nC.runtime.input must exist");
        assert!(
            input.functions.iter().any(|f| f.name == "readInt"),
            "nC.runtime.input must export readInt()"
        );
        assert!(
            input.functions.iter().any(|f| f.name == "windowReadChar"),
            "nC.runtime.input must export windowReadChar(handle)"
        );
        let string = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "string".to_string(),
        ])
        .expect("nC.runtime.string must exist");
        assert!(
            string.functions.iter().any(|function| {
                function.name == "charFromCode"
                    && function.arg_types == ["int"]
                    && function.return_ty == "str"
            }),
            "nC.runtime.string must expose the typed int-to-character-string bridge"
        );
        for name in [
            "strRelease",
            "strLen",
            "strByteLen",
            "strSubstring",
            "strFromFt64",
        ] {
            assert!(
                string.functions.iter().any(|f| f.name == name),
                "nC.runtime.string must export {name}"
            );
        }

        let fmt = runtime_api_find_function_by_module_path(
            &["runtime".to_string(), "format".to_string()],
            "format",
        )
        .expect("runtime.format.format must exist");
        assert!(fmt.variadic, "runtime.format.format must be variadic");

        let fs = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "fs".to_string(),
        ])
        .expect("nC.runtime.fs must exist");
        assert!(
            fs.functions.iter().any(|f| f.name == "fsappend")
                && fs.functions.iter().any(|f| f.name == "fsync"),
            "nC.runtime.fs must export the real fs append/sync surface"
        );

        let net = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "tcp".to_string(),
        ])
        .expect("nC.runtime.tcp must exist");
        assert!(
            net.functions.iter().any(|f| f.name == "tcpListen")
                && net.functions.iter().any(|f| f.name == "tcpRecv"),
            "nC.runtime.tcp must export the real TCP surface"
        );
    }

    #[test]
    fn runtime_linker_symbols_are_canonical_and_generic_free() {
        assert_eq!(
            canonical_runtime_symbol("nC.math.trig.sinFloat"),
            "nC.runtime.math.trig.sinFloat"
        );
        assert_eq!(
            canonical_runtime_symbol("nC.vector.core.__native_vector_activation"),
            "nC.runtime.vector.core.__native_vector_activation"
        );
        assert_eq!(
            canonical_runtime_symbol("nC.runtime.result.ok<T>"),
            "nC.runtime.result.ok"
        );
        let (_, ok) = runtime_api_find_function_by_runtime_symbol("nC.runtime.result.ok")
            .expect("generic-free result symbol must retain its typed source signature");
        assert_eq!(ok.arg_types, ["T"]);
        assert_eq!(ok.return_ty, "Result<T,E>");

        let manifest = use_backend_lowering_manifest_all();
        for entry in manifest.entries {
            for symbol in entry.runtime_symbols {
                assert!(
                    symbol.starts_with("nC.runtime.") && !symbol.contains('<'),
                    "non-canonical runtime symbol: {symbol}"
                );
            }
        }
    }

    #[test]
    fn native_prelude_is_an_exact_function_allowlist() {
        let names = NATIVE_PRELUDE_V1_FUNCTIONS
            .iter()
            .map(|function| function.source_name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "alloc",
                "calloc",
                "realloc",
                "free",
                "strConcatInto",
                "memcpy",
                "memmove",
                "memset",
                "memcmp",
                "closureAt",
                "closureSize",
                "closureAlign",
                "asyncAt",
                "callAt",
                "listAt",
                "queueAt",
                "vectorAt",
                "dinAt",
                "vmMap",
                "vmProtect",
                "vmUnmap",
                "vmExecutable",
                "ptrRead",
                "ptrWrite",
                "ptrOffset",
                "ptrDiff",
                "ptrCast",
                "ptrAddr",
                "ptrFromAddr",
                "ptrIsNull",
                "checkedAdd",
                "sizeOf",
                "alignOf",
                "offsetOf",
                "atomicLoad",
                "atomicStore",
                "fence",
                "readInt",
                "readStr",
                "sleepMs",
                "monotonicMs",
                "get_current_time",
                "exists",
                "fexists",
                "listDir",
                "fswrite",
                "fread_lines",
                "delete_file",
                "rename_file",
                "cpu_load",
                "registry_write",
                "registry_read",
            ]
        );

        let read_str = native_prelude_resolve_function("readStr", 0)
            .expect("native prelude lookup")
            .expect("readStr must be import-free");
        assert_eq!(read_str.canonical_symbol, "nC.runtime.input.readLine");
        assert_eq!(read_str.lowering, NativePreludeLowering::Runtime);
        let typed_read = native_prelude_resolve_function("ptrRead<int>", 1)
            .expect("typed intrinsic lookup")
            .expect("ptrRead<T> must be import-free");
        assert_eq!(
            typed_read.canonical_call_symbol("ptrRead<int>"),
            "nC.intrinsic.ptr.ptrRead<int>"
        );
        let checked_add = native_prelude_resolve_function("checkedAdd<int64>", 2)
            .expect("checked arithmetic lookup")
            .expect("checkedAdd<int64> must be import-free");
        assert_eq!(
            checked_add.canonical_call_symbol("checkedAdd<int64>"),
            "nC.intrinsic.arithmetic.checkedAdd<int64>"
        );
        assert_eq!(
            checked_add.lowering,
            NativePreludeLowering::CompilerIntrinsic
        );
        assert!(native_prelude_resolve_function("atomicNotifyAll", 1)
            .expect("non-prelude lookup")
            .is_none());

        let wrong_arity = native_prelude_resolve_function("calloc", 1)
            .expect_err("known prelude names must reject the wrong arity");
        assert_eq!(wrong_arity.code, "E_NATIVE_PRELUDE_ARITY_001");

        let ambiguous_alloc = native_prelude_resolve_function("alloc", 2)
            .expect_err("aligned heap allocation must use its explicit ABI pair");
        assert!(ambiguous_alloc
            .help
            .is_some_and(|help| help.contains("alignedAlloc") && help.contains("alignedFree")));

        assert!(typed_read.available_in_context(None));
        assert!(typed_read.available_in_context(Some(&Context::Root)));
        assert!(typed_read.available_in_context(Some(&Context::Kernel)));
        let sleep = native_prelude_resolve_function("sleepMs", 1)
            .expect("sleep lookup")
            .expect("sleepMs must be import-free in hosted code");
        assert!(sleep.available_in_context(Some(&Context::Runtime)));
        assert!(!sleep.available_in_context(Some(&Context::Kernel)));
        assert!(!sleep.available_in_context(Some(&Context::Machine)));
    }

    #[test]
    fn hosted_runtime_prelude_v1_resolves_hosted_backend_families_by_catalog() {
        assert_eq!(
            HOSTED_RUNTIME_PRELUDE_V1_FAMILIES,
            [
                "nC.runtime.string",
                "nC.runtime.convert",
                "nC.runtime.escape",
                "nC.runtime.align",
                "nC.runtime.format",
                "nC.runtime.math",
                "nC.runtime.random",
                "nC.runtime.entropy",
                "nC.runtime.uuid",
                "nC.runtime.json",
                "nC.runtime.terminal",
                "nC.runtime.time",
                "nC.runtime.clock",
                "nC.runtime.sleep",
                "nC.runtime.fs",
                "nC.runtime.system",
                "nC.runtime.permission",
                "nC.runtime.telephony",
                "nC.runtime.tts",
                "nC.runtime.mobileNetwork",
                "nC.runtime.compilerManual",
                "nC.runtime.aiManual",
                "nC.runtime.highManual",
                "nC.runtime.inspectManual",
                "nC.runtime.toolingManual"
            ]
        );

        for (name, arity, symbol) in [
            ("randomInt", 0, "nC.runtime.random.randomInt"),
            ("randomBool", 0, "nC.runtime.random.randomBool"),
            ("randomFloat", 0, "nC.runtime.random.randomFloat"),
            ("randomRange", 2, "nC.runtime.random.randomRange"),
            ("seed", 1, "nC.runtime.random.seed"),
            ("sign", 1, "nC.runtime.math.sign"),
            ("abs", 1, "nC.runtime.math.abs"),
            ("entropyFill", 2, "nC.runtime.entropy.entropyFill"),
            ("uuidV4Into", 2, "nC.runtime.uuid.uuidV4Into"),
            ("uuidFromBytesInto", 3, "nC.runtime.uuid.uuidFromBytesInto"),
            ("cargarPermisos", 0, "nC.runtime.permission.cargarPermisos"),
            (
                "getIncomingNumber",
                0,
                "nC.runtime.telephony.getIncomingNumber",
            ),
            ("speak", 1, "nC.runtime.tts.speak"),
        ] {
            let function = hosted_runtime_prelude_resolve_function(name, arity)
                .expect("mobile hosted prelude lookup")
                .expect("mobile backend operation must be import-free");
            assert_eq!(function.canonical_symbol, symbol);
        }

        for (name, arity, symbol) in [
            ("fswrite", 2, "nC.runtime.fs.fswrite"),
            ("fread_lines", 1, "nC.runtime.fs.fread_lines"),
            ("delete_file", 1, "nC.runtime.fs.delete_file"),
            ("rename_file", 2, "nC.runtime.fs.rename_file"),
            ("cpu_load", 0, "nC.runtime.system.cpu_load"),
            ("registry_write", 2, "nC.runtime.system.registry_write"),
            ("registry_read", 1, "nC.runtime.system.registry_read"),
        ] {
            let function = native_prelude_resolve_function(name, arity)
                .expect("hosted native prelude lookup")
                .expect("requested hosted operation must be import-free");
            assert_eq!(function.canonical_symbol, symbol);
            assert_eq!(function.lowering, NativePreludeLowering::Runtime);
            assert!(function.available_in_context(Some(&Context::Runtime)));
            assert!(!function.available_in_context(Some(&Context::Kernel)));
        }

        for (name, arity, symbol) in [
            ("server_open", 1, "nC.runtime.mobileNetwork.server_open"),
            (
                "isWiFiConnected",
                0,
                "nC.runtime.mobileNetwork.isWiFiConnected",
            ),
            ("send_bytes", 1, "nC.runtime.mobileNetwork.send_bytes"),
            ("receive_bytes", 0, "nC.runtime.mobileNetwork.receive_bytes"),
        ] {
            let function = hosted_runtime_prelude_resolve_function(name, arity)
                .expect("network prelude lookup")
                .expect("network operation must be import-free");
            assert_eq!(function.canonical_symbol, symbol);
        }

        let string = hosted_runtime_prelude_resolve_function("strToUpper", 1)
            .expect("string prelude lookup")
            .expect("strToUpper must be implicit in hosted code");
        assert_eq!(string.family.module_path, "runtime.string");
        assert_eq!(string.function.return_ty, "str");
        assert_eq!(string.canonical_symbol, "nC.runtime.string.strToUpper");

        let release = hosted_runtime_prelude_resolve_function("strRelease", 1)
            .expect("owned string release prelude lookup")
            .expect("strRelease must be implicit in hosted code");
        assert_eq!(release.function.arg_types, ["str"]);
        assert_eq!(release.function.return_ty, "bool");
        assert_eq!(release.canonical_symbol, "nC.runtime.string.strRelease");

        let convert = hosted_runtime_prelude_resolve_function("safeToInt", 1)
            .expect("convert prelude lookup")
            .expect("safeToInt must be implicit in hosted code");
        assert_eq!(convert.family.module_path, "runtime.convert");
        assert_eq!(convert.function.return_ty, "Result<int, ConvertError>");
        assert_eq!(convert.canonical_symbol, "nC.runtime.convert.safeToInt");

        let escape = hosted_runtime_prelude_resolve_function("sanitizeTerminal", 1)
            .expect("escape prelude lookup")
            .expect("sanitizeTerminal must be implicit in hosted code");
        assert_eq!(escape.family.module_path, "runtime.escape");
        assert_eq!(escape.function.return_ty, "str");
        assert_eq!(
            escape.canonical_symbol,
            "nC.runtime.escape.sanitizeTerminal"
        );

        let align = hosted_runtime_prelude_resolve_function("alignCenter", 2)
            .expect("alignment prelude lookup")
            .expect("alignCenter must be implicit in hosted code");
        assert_eq!(align.family.module_path, "runtime.align");
        assert_eq!(align.function.arg_types, ["str", "int"]);
        assert_eq!(align.canonical_symbol, "nC.runtime.align.alignCenter");

        let format = hosted_runtime_prelude_resolve_function("format", 4)
            .expect("format prelude lookup")
            .expect("format must be implicit in hosted code");
        assert_eq!(format.family.module_path, "runtime.format");
        assert!(format.function.variadic);
        assert_eq!(format.function.return_ty, "str");
        assert_eq!(format.canonical_symbol, "nC.runtime.format.format");

        let json = hosted_runtime_prelude_resolve_function("jsonParse", 1)
            .expect("JSON prelude lookup")
            .expect("jsonParse must be implicit in hosted code");
        assert_eq!(json.family.module_path, "runtime.json");
        assert_eq!(json.function.return_ty, "int");
        assert_eq!(json.canonical_symbol, "nC.runtime.json.jsonParse");

        let terminal = hosted_runtime_prelude_resolve_function("hideCursor", 0)
            .expect("terminal prelude lookup")
            .expect("hideCursor must be implicit in hosted code");
        assert_eq!(terminal.family.module_path, "runtime.terminal");
        assert_eq!(terminal.function.return_ty, "bool");

        let time = hosted_runtime_prelude_resolve_function("monotonicMs", 0)
            .expect("time prelude lookup")
            .expect("monotonicMs must be implicit in hosted code");
        assert_eq!(time.family.module_path, "runtime.time");
        assert_eq!(time.function.return_ty, "int");

        let clock = hosted_runtime_prelude_resolve_function("clockResolution", 0)
            .expect("clock prelude lookup")
            .expect("clockResolution must be implicit in hosted code");
        assert_eq!(clock.family.module_path, "runtime.clock");
        assert_eq!(clock.function.return_ty, "u64");

        let sleep = hosted_runtime_prelude_resolve_function("sleepSec", 1)
            .expect("sleep prelude lookup")
            .expect("sleepSec must be implicit in hosted code");
        assert_eq!(sleep.family.module_path, "runtime.sleep");
        assert_eq!(sleep.function.return_ty, "void");

        assert!(hosted_runtime_prelude_resolve_function("listCreate", 0)
            .expect("non-prelude collection lookup")
            .is_none());
        let wrong_arity = hosted_runtime_prelude_resolve_function("jsonParse", 0)
            .expect_err("known prelude names must diagnose wrong arity");
        assert_eq!(wrong_arity.code, "E_RUNTIME_PRELUDE_ARITY_001");
    }

    #[test]
    fn stoud_ui_facade_expands_to_typed_runtime_families_with_real_backing() {
        let expansions = expand_use_domain_import(
            "stoud",
            "ui",
            &["window".into(), "canvas".into(), "input".into()],
        )
        .expect("stoud::ui must resolve");
        assert_eq!(expansions.len(), 3);
        assert!(expansions
            .iter()
            .all(|entry| entry.status == "closed_real_backing"));

        let lowering = use_backend_lowering_manifest_all();
        for symbol in [
            "nC.runtime.terminal.windowOpen",
            "nC.runtime.terminal.windowWaitEvent",
            "nC.runtime.terminal.windowIsFocused",
            "nC.runtime.terminal.windowDpi",
            "nC.runtime.terminal.windowRefreshHz",
            "nC.runtime.terminal.windowState",
            "nC.runtime.terminal.windowSetState",
            "nC.runtime.render.canvasRect",
            "nC.runtime.render.canvasBeginFrame",
            "nC.runtime.render.canvasRoundRect",
            "nC.runtime.render.canvasTextStyled",
            "nC.runtime.render.canvasSetQuality",
            "nC.runtime.render.canvasSetFrameMode",
            "nC.runtime.render.canvasFrameDeltaMs",
            "nC.runtime.render.canvasPollResize",
            "nC.runtime.render.canvasSetViewport",
            "nC.runtime.render.canvasPointerX",
            "nC.runtime.render.canvasPointerY",
            "nC.runtime.render.canvasTextCentered",
            "nC.runtime.render.canvasShadowRoundRect",
            "nC.runtime.render.canvasRoundRectAlpha",
            "nC.runtime.render.canvasRoundRectGradient",
            "nC.runtime.render.canvasBackdropBlur",
            "nC.runtime.render.canvasBackdropBlurV3",
            "nC.runtime.input.mouseX",
            "nC.runtime.input.windowReadChar",
        ] {
            assert!(lowering.entries.iter().any(|entry| {
                entry
                    .runtime_symbols
                    .iter()
                    .any(|candidate| candidate == symbol)
                    && entry.closure_status == "closed_real_backing_bytes"
            }));
        }

        for (symbol, args) in [
            ("nC.runtime.render.canvasBeginFrame", &["int", "int"][..]),
            (
                "nC.runtime.render.canvasRoundRect",
                &["int", "int", "int", "int", "int", "int", "int"][..],
            ),
            (
                "nC.runtime.render.canvasTextStyled",
                &["int", "int", "int", "str", "int", "int", "int"][..],
            ),
            ("nC.runtime.render.canvasBackdropBlur", &["int", "int"][..]),
            ("nC.runtime.terminal.windowDpi", &["int"][..]),
            ("nC.runtime.terminal.windowRefreshHz", &["int"][..]),
            ("nC.runtime.terminal.windowState", &["int"][..]),
            ("nC.runtime.terminal.windowSetState", &["int", "int"][..]),
            ("nC.runtime.terminal.windowWaitEvent", &["int", "int"][..]),
            ("nC.runtime.terminal.windowIsFocused", &["int"][..]),
            ("nC.runtime.input.windowReadChar", &["int"][..]),
            ("nC.runtime.render.canvasSetQuality", &["int", "int"][..]),
            (
                "nC.runtime.render.canvasSetFrameMode",
                &["int", "int", "int"][..],
            ),
            ("nC.runtime.render.canvasFrameDeltaMs", &["int"][..]),
            ("nC.runtime.render.canvasPollResize", &["int"][..]),
            (
                "nC.runtime.render.canvasSetViewport",
                &["int", "int", "int", "int"][..],
            ),
            ("nC.runtime.render.canvasPointerX", &["int"][..]),
            ("nC.runtime.render.canvasPointerY", &["int"][..]),
            (
                "nC.runtime.render.canvasTextCentered",
                &[
                    "int", "int", "int", "int", "int", "str", "int", "int", "int",
                ][..],
            ),
            (
                "nC.runtime.render.canvasShadowRoundRect",
                &["int", "int", "int", "int", "int", "int", "int", "int"][..],
            ),
            (
                "nC.runtime.render.canvasRoundRectAlpha",
                &["int", "int", "int", "int", "int", "int", "int", "int"][..],
            ),
            (
                "nC.runtime.render.canvasRoundRectGradient",
                &[
                    "int", "int", "int", "int", "int", "int", "int", "int", "int",
                ][..],
            ),
            (
                "nC.runtime.render.canvasBackdropBlurV3",
                &["int", "int", "int", "int"][..],
            ),
        ] {
            let (_, function) = runtime_api_find_function_by_runtime_symbol(symbol)
                .unwrap_or_else(|| panic!("missing typed window/canvas function {symbol}"));
            assert_eq!(function.arg_types, args);
            let expected_return = if symbol.ends_with("windowDpi")
                || symbol.ends_with("windowRefreshHz")
                || symbol.ends_with("windowState")
                || symbol.ends_with("canvasFrameDeltaMs")
                || symbol.ends_with("canvasPointerX")
                || symbol.ends_with("canvasPointerY")
                || symbol.ends_with("windowReadChar")
            {
                "int"
            } else {
                "bool"
            };
            assert_eq!(function.return_ty, expected_return);
        }
    }

    #[test]
    fn use_backend_lowering_manifest_materializes_every_import_group() {
        let manifest = use_backend_lowering_manifest_all();
        assert_eq!(manifest.status, "closed");
        assert!(manifest.total_entries >= 190);
        assert!(manifest.total_families >= 17);
        assert!(manifest
            .groups
            .iter()
            .any(|group| group.domain == UseDomain::Stoud && group.family == "io"));
        assert!(manifest
            .groups
            .iter()
            .any(|group| group.domain == UseDomain::Auth && group.family == "system"));
        assert!(manifest
            .groups
            .iter()
            .any(|group| group.domain == UseDomain::Dom && group.family == "compiler"));
    }

    #[test]
    fn use_backend_lowering_manifest_materializes_all_official_runtime_families() {
        let manifest = use_backend_lowering_manifest_all();
        for family in NC_RUNTIME_FAMILIES {
            let surface = format!("use {family};");
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing official runtime backend entry {surface}"));
            assert_eq!(entry.status, "closed_real_backing_bytes");
            assert_eq!(entry.backing_status, "closed_real_backing_bytes");
            assert_eq!(
                entry.runtime_call_lowering,
                "direct_runtime_symbol_call_without_real_flag"
            );
            assert_eq!(
                entry.mct_profiles,
                vec!["nC.sysv64", "nC.win64", "nC.aapcs64", "nC.portable"]
            );
            assert_eq!(entry.emitted_bytes.len(), 4);
            assert!(
                entry.backend_routes.iter().any(|route| route.route == "mct"
                    && route.status == "closed_runtime_symbol_activation_bytes"),
                "{surface} must expose MCT activation route"
            );
        }
    }

    #[test]
    fn use_backend_lowering_entries_have_real_target_bytes() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let input = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<input>;")
            .expect("input import backend entry");
        assert_eq!(input.backend_symbol, "__nc_import_activate_stoud_io_input");
        assert_eq!(input.backing_status, "closed_real_backing_bytes");
        assert!(input.closed_real_backing_bytes);
        assert_eq!(
            input.runtime_call_lowering,
            "direct_runtime_symbol_call_without_real_flag"
        );
        assert!(input
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.input.readInt"));
        assert_eq!(input.emitted_bytes.len(), 4);
        assert!(input
            .emitted_bytes
            .iter()
            .all(|row| !row.bytes_hex.is_empty() && row.byte_count > 0));
        assert!(input
            .emitted_bytes
            .iter()
            .any(|row| row.profile == "nC.sysv64" && row.bytes_hex.starts_with("b8")));
    }

    #[test]
    fn render_import_declares_external_stdlib_binary_provider() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let render = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<render>;")
            .expect("render import backend entry");
        assert_eq!(
            render.mct_provider,
            "ncc-runtime-stdlib.external+ncc-native+MCT"
        );
        assert!(render.runtime_semantics.contains("ncc-runtime-stdlib"));
        assert!(render.runtime_semantics.contains("libc %b"));
        assert!(render.runtime_semantics.contains("portable exact"));
    }

    #[test]
    fn stoud_runtime_standard_families_are_consolidated() {
        let manifest = use_import_manifest_for_domain(UseDomain::Stoud);
        let official_order = [
            "io",
            "memory",
            "collections",
            "math",
            "time",
            "fs",
            "net",
            "process",
            "thread",
        ];
        let mut seen = Vec::new();
        for entry in &manifest.entries {
            if official_order.contains(&entry.family.as_str())
                && seen
                    .last()
                    .map(|last| last != &entry.family)
                    .unwrap_or(true)
            {
                seen.push(entry.family.clone());
            }
        }
        assert_eq!(
            &seen[..official_order.len()],
            official_order,
            "stoud official families must keep canonical order"
        );
        for import in [
            "use::stoud::io::<input>;",
            "use::stoud::io::<output>;",
            "use::stoud::io::<render>;",
            "use::stoud::io::<random>;",
            "use::stoud::io::<fsread>;",
            "use::stoud::io::<fswrite>;",
            "use::stoud::io::<fsappend>;",
            "use::stoud::io::<fsync>;",
            "use::stoud::memory::<ptr>;",
            "use::stoud::memory::<heap>;",
            "use::stoud::memory::<map>;",
            "use::stoud::memory::<region>;",
            "use::stoud::collections::<array>;",
            "use::stoud::collections::<list>;",
            "use::stoud::collections::<queue>;",
            "use::stoud::collections::<vector>;",
            "use::stoud::collections::<result>;",
            "use::stoud::math::<core>;",
            "use::stoud::math::<trig>;",
            "use::stoud::time::<sleep>;",
            "use::stoud::fs::<read>;",
            "use::stoud::fs::<write>;",
            "use::stoud::fs::<path>;",
            "use::stoud::net::<tcp>;",
            "use::stoud::process::<spawn>;",
            "use::stoud::thread::<join>;",
        ] {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == import)
                .unwrap_or_else(|| panic!("missing standard runtime import {import}"));
            assert_eq!(entry.status, "closed_real_backing");
            assert!(entry.diagnostic.is_none(), "{import} must be backed");
        }
    }

    #[test]
    fn entropy_import_has_real_runtime_backing() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entropy = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<entropy>;")
            .expect("entropy catalog entry");
        assert_eq!(entropy.status, "closed_real_backing_bytes");
        assert_eq!(
            entropy.runtime_call_lowering,
            "direct_runtime_symbol_call_without_real_flag"
        );
        assert!(entropy.closed_real_backing_bytes);
        assert_eq!(
            entropy.runtime_symbols,
            [
                "nC.runtime.entropy.entropyBytes",
                "nC.runtime.entropy.entropyAvailable",
            ]
        );
        assert!(entropy.diagnostics.is_empty());
    }

    #[test]
    fn backend_manifest_records_runtime_symbols_for_real_runtime_families() {
        let manifest = use_backend_lowering_manifest_all();
        for (surface, symbol) in [
            ("use::stoud::io::<fsappend>;", "nC.runtime.fs.fsappend"),
            ("use::stoud::io::<fsync>;", "nC.runtime.fs.fsync"),
            ("use::stoud::fs::<read>;", "nC.runtime.fs.readFile"),
            ("use::stoud::fs::<write>;", "nC.runtime.fs.writeFile"),
            ("use::stoud::fs::<path>;", "nC.runtime.path.pathJoin"),
            ("use::stoud::net::<tcp>;", "nC.runtime.tcp.tcpRecv"),
            ("use::stoud::process::<spawn>;", "nC.runtime.process.spawn"),
            ("use::stoud::process::<wait>;", "nC.runtime.process.wait"),
            (
                "use::stoud::thread::<spawn>;",
                "nC.runtime.thread.threadSpawn",
            ),
            (
                "use::stoud::thread::<join>;",
                "nC.runtime.thread.threadJoin",
            ),
            ("use::auth::system::<process>;", "nC.runtime.process.spawn"),
            (
                "use::auth::system::<thread>;",
                "nC.runtime.thread.threadJoin",
            ),
            ("use::stoud::sync::<sync>;", "nC.runtime.sync.mutexCreate"),
            ("use::stoud::sync::<lock>;", "nC.runtime.lock.rwLockCreate"),
            (
                "use::stoud::sync::<atomic>;",
                "nC.runtime.atomic.atomicCreate",
            ),
            (
                "use::stoud::sync::<channel>;",
                "nC.runtime.channel.channelCreate",
            ),
            ("use::stoud::time::<sleep>;", "nC.runtime.time.sleep"),
        ] {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing backend entry {surface}"));
            assert_eq!(entry.status, "closed_real_backing_bytes");
            assert_eq!(entry.backing_status, "closed_real_backing_bytes");
            assert!(entry.closed_real_backing_bytes);
            assert_eq!(
                entry.runtime_call_lowering,
                "direct_runtime_symbol_call_without_real_flag"
            );
            assert!(
                entry
                    .runtime_symbols
                    .iter()
                    .any(|candidate| candidate == symbol),
                "{surface} must lower to runtime symbol {symbol}; got {:?}",
                entry.runtime_symbols
            );
        }
    }

    #[test]
    fn compact_terminal_family_is_backed_one_item_at_a_time() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        for (item, symbol) in [
            ("terminal", "nC.runtime.terminal.clear"),
            ("clear", "nC.runtime.terminal.clear"),
            ("moveCursor", "nC.runtime.terminal.moveCursor"),
            ("hideCursor", "nC.runtime.terminal.hideCursor"),
            ("showCursor", "nC.runtime.terminal.showCursor"),
            ("enterRawMode", "nC.runtime.terminal.enterRawMode"),
            ("exitRawMode", "nC.runtime.terminal.exitRawMode"),
            (
                "readKeyNonBlocking",
                "nC.runtime.terminal.readKeyNonBlocking",
            ),
        ] {
            let surface = format!("use::stoud::terminal::<{item}>;");
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing compact terminal import {surface}"));
            assert_eq!(entry.closure_status, "closed_real_backing_bytes");
            assert!(entry.closed_real_backing_bytes);
            assert!(entry.can_run_without_runtime_import_failure);
            assert!(entry
                .runtime_symbols
                .iter()
                .any(|candidate| candidate == symbol));
        }
    }

    #[test]
    fn stoud_fs_dir_closes_catalog_runtime_llvm_mct_and_target_bytes() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::fs::<dir>;")
            .expect("stoud fs dir backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.dir.listDir",
                "nC.runtime.dir.createDir",
                "nC.runtime.dir.removeDir",
                "nC.runtime.dir.currentDir",
                "nC.runtime.dir.changeDir",
            ]
        );
        assert_eq!(entry.emitted_bytes.len(), 4);
        assert!(entry
            .emitted_bytes
            .iter()
            .all(|row| row.byte_count > 0 && !row.bytes_hex.is_empty()));
        for route in ["ncc_run", "mct", "native_fast", "llvm", "machine", "basal"] {
            assert!(
                entry.backend_routes.iter().any(|candidate| {
                    candidate.route == route && candidate.status.starts_with("closed")
                }),
                "missing closed {route} route: {:?}",
                entry.backend_routes
            );
        }

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "dir".to_string(),
        ])
        .expect("nC.runtime.dir typed family");
        for (name, args, result) in [
            ("listDir", &["str"][..], "Result<list<str>, FsError>"),
            ("createDir", &["str"][..], "Result<bool, FsError>"),
            ("removeDir", &["str"][..], "Result<bool, FsError>"),
            ("currentDir", &[][..], "Result<str, FsError>"),
            ("changeDir", &["str"][..], "Result<bool, FsError>"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing nC.runtime.dir.{name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }
    }

    #[test]
    fn stoud_fs_dir_rejects_unknown_directory_alias() {
        let diagnostic = expand_use_domain_import("stoud", "fs", &["directory".to_string()])
            .expect_err("unknown stoud fs item must fail");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
        assert!(diagnostic.message.contains("directory"));
    }

    #[test]
    fn stoud_io_bool_exposes_only_material_random_bool() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<bool>;")
            .expect("stoud io bool backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.random.randomBool"]);
        assert_eq!(entry.emitted_bytes.len(), 4);
        assert!(entry
            .backend_routes
            .iter()
            .any(|route| route.route == "llvm"
                && route.status == "closed_llvm_runtime_declaration_call"));
    }

    #[test]
    fn stoud_io_concat_exposes_only_material_binary_concat() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<concat>;")
            .expect("stoud io concat backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.concat.concat"]);
        assert!(entry
            .backend_routes
            .iter()
            .any(|route| route.route == "llvm"
                && route.status == "closed_llvm_runtime_declaration_call"));
        assert!(!entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol.ends_with("concatMany") || symbol.ends_with("join")));
    }

    #[test]
    fn stoud_io_error_output_exposes_complete_stderr_family() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<errorOutput>;")
            .expect("stoud io errorOutput backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert_eq!(entry.runtime_symbols.len(), 5);
        for symbol in ["eprint", "eprintln", "warn", "error", "flushErr"] {
            assert!(entry
                .runtime_symbols
                .iter()
                .any(|candidate| candidate.ends_with(symbol)));
        }
    }

    #[test]
    fn stoud_io_float_exposes_only_material_random_float() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<float>;")
            .expect("stoud io float backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.random.randomFloat"]);
    }

    #[test]
    fn stoud_io_int_exposes_only_material_random_int() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<int>;")
            .expect("stoud io int backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.random.randomInt"]);

        let diagnostic = expand_use_domain_import("stoud", "io", &["integer".into()])
            .expect_err("unknown integer alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_text_pad_closes_catalog_runtime_and_target_bytes() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<pad>;")
            .expect("stoud text pad backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.pad.padLeft",
                "nC.runtime.pad.padRight",
                "nC.runtime.pad.padBoth",
            ]
        );
        assert_eq!(entry.emitted_bytes.len(), 4);
        assert!(entry
            .emitted_bytes
            .iter()
            .all(|row| row.byte_count > 0 && !row.bytes_hex.is_empty()));
    }

    #[test]
    fn stoud_text_parse_is_manual_nc_with_caller_owned_try_outputs() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<parse>;")
            .expect("stoud text parse backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 8);
        assert_eq!(entry.emitted_bytes.len(), 4);
        assert!(entry
            .emitted_bytes
            .iter()
            .all(|row| row.byte_count > 0 && !row.bytes_hex.is_empty()));

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "parse".to_string(),
        ])
        .expect("parse runtime family must exist");
        for name in ["tryParseInt", "tryParseFloat", "tryParseBool"] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .expect("manual try parse export");
            assert_eq!(function.arity_min, 2);
            assert_eq!(function.return_ty, "bool");
            assert!(function.arg_types[1].starts_with("ptr<"));
        }
    }

    #[test]
    fn stoud_text_string_is_manual_nc_with_explicit_release() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<string>;")
            .expect("stoud text string backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "string".to_string(),
        ])
        .expect("string runtime family must exist");
        let expected_symbols: std::collections::BTreeSet<_> = family
            .functions
            .iter()
            .map(|function| format!("nC.runtime.string.{}", function.name))
            .collect();
        let actual_symbols: std::collections::BTreeSet<_> =
            entry.runtime_symbols.iter().cloned().collect();

        // Check both independently maintained catalogs, including charFromCode.
        // A symbol count alone misses substitutions and becomes stale on API additions.
        assert_eq!(entry.runtime_symbols.len(), actual_symbols.len());
        assert_eq!(actual_symbols, expected_symbols);
        assert!(actual_symbols.contains("nC.runtime.string.charFromCode"));
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.string.strSplitRelease"));
        assert_eq!(entry.emitted_bytes.len(), 4);
        assert!(entry
            .emitted_bytes
            .iter()
            .all(|row| row.byte_count > 0 && !row.bytes_hex.is_empty()));
    }

    #[test]
    fn stoud_text_text_is_manual_nc_with_explicit_release() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<text>;")
            .expect("stoud text backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 8);
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.text.textRelease"));
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.text.textSplitRelease"));
        assert_eq!(entry.emitted_bytes.len(), 4);
        assert!(entry
            .emitted_bytes
            .iter()
            .all(|row| row.byte_count > 0 && !row.bytes_hex.is_empty()));

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "text".to_string(),
        ])
        .expect("text runtime family must exist");
        assert_eq!(
            family
                .functions
                .iter()
                .find(|function| function.name == "textLength")
                .expect("textLength")
                .return_ty,
            "int"
        );
        assert_eq!(
            family
                .functions
                .iter()
                .find(|function| function.name == "textSplit")
                .expect("textSplit")
                .return_ty,
            "array<str>"
        );
    }

    #[test]
    fn stoud_text_truncate_is_manual_nc_with_explicit_release() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<truncate>;")
            .expect("stoud text truncate backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 4);
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.truncate.truncateRelease"));
        assert_eq!(entry.emitted_bytes.len(), 4);

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "truncate".to_string(),
        ])
        .expect("truncate runtime family must exist");
        for name in ["truncate", "truncateEnd", "truncateMiddle"] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .expect("truncate export");
            assert_eq!(function.arg_types, ["str", "int"]);
        }
    }

    #[test]
    fn stoud_text_unicode_is_manual_nc_and_caller_owned() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<unicode>;")
            .expect("stoud text unicode backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 12);
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.unicode.unicodeRelease"));
        assert_eq!(entry.emitted_bytes.len(), 4);

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "unicode".to_string(),
        ])
        .expect("unicode runtime family must exist");
        assert_eq!(
            family
                .functions
                .iter()
                .find(|function| function.name == "unicodeMode")
                .expect("unicodeMode")
                .return_ty,
            "str"
        );
        assert_eq!(
            family
                .functions
                .iter()
                .find(|function| function.name == "graphemeSlice")
                .expect("graphemeSlice")
                .arg_types,
            ["str", "int", "int"]
        );
    }

    #[test]
    fn stoud_text_width_is_unicode17_backed_and_integer_typed() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<width>;")
            .expect("stoud text width backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 5);
        assert_eq!(entry.emitted_bytes.len(), 4);

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "width".to_string(),
        ])
        .expect("width runtime family must exist");
        assert_eq!(family.functions.len(), 5);
        assert!(family
            .functions
            .iter()
            .all(|function| function.return_ty == "int"));
    }

    #[test]
    fn stoud_text_wrap_is_unicode_backed_and_caller_owned() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<wrap>;")
            .expect("stoud text wrap backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 4);
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.wrap.wrapRelease"));

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "wrap".to_string(),
        ])
        .expect("wrap runtime family must exist");
        assert_eq!(family.functions.len(), 4);
        for name in ["wrap", "wrapWords", "indent"] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .expect(name);
            assert_eq!(function.arg_types, ["str", "int"]);
            assert_eq!(function.return_ty, "str");
        }
        let release = family
            .functions
            .iter()
            .find(|function| function.name == "wrapRelease")
            .expect("wrapRelease");
        assert_eq!(release.arg_types, ["str"]);
        assert_eq!(release.return_ty, "bool");
    }

    #[test]
    fn stoud_net_http_is_winhttp_backed_raw_and_caller_owned() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::net::<http>;")
            .expect("stoud net HTTP backend entry");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols.len(), 9);
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.http.httpRequestRaw"));
        assert!(entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.http.httpRelease"));

        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "http".to_string(),
        ])
        .expect("HTTP runtime family must exist");
        assert_eq!(family.functions.len(), 9);
        let request = family
            .functions
            .iter()
            .find(|function| function.name == "httpRequestRaw")
            .expect("httpRequestRaw");
        assert_eq!(
            request.arg_types,
            ["str", "str", "str", "addr", "int", "int", "int"]
        );
        assert_eq!(request.return_ty, "addr");
        let release = family
            .functions
            .iter()
            .find(|function| function.name == "httpRelease")
            .expect("httpRelease");
        assert_eq!(release.arg_types, ["addr"]);
        assert_eq!(release.return_ty, "bool");
    }

    #[test]
    fn stoud_net_and_socket_are_winsock_backed_raw_surfaces() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let net = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::net::<net>;")
            .expect("stoud net backend entry");
        let socket = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::net::<socket>;")
            .expect("stoud socket backend entry");

        for entry in [net, socket] {
            assert_eq!(entry.closure_status, "closed_real_backing_bytes");
            assert!(entry.closed_real_backing_bytes);
            assert!(entry.can_run_without_runtime_import_failure);
        }
        assert_eq!(net.runtime_symbols.len(), 7);
        assert_eq!(socket.runtime_symbols.len(), 11);
        assert!(net
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.net.netResolve"));
        assert!(socket
            .runtime_symbols
            .iter()
            .any(|symbol| symbol == "nC.runtime.socket.socketSetTimeout"));

        let (_, send) = runtime_api_find_function_by_runtime_symbol("nC.runtime.socket.socketSend")
            .expect("socketSend");
        assert_eq!(send.arg_types, ["addr", "addr", "int", "int"]);
        assert_eq!(send.return_ty, "int");
    }

    #[test]
    fn stoud_io_range_exposes_only_material_inclusive_random_range() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<range>;")
            .expect("stoud io range backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.random.randomRange"]);
    }

    #[test]
    fn stoud_io_seed_exposes_only_material_integer_prng_seed() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<seed>;")
            .expect("stoud io seed backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.random.seed"]);

        let diagnostic = expand_use_domain_import("stoud", "io", &["seeder".into()])
            .expect_err("unknown seed alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_io_std_exposes_raw_standard_handles_and_interactivity() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<std>;")
            .expect("stoud io std backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.std.stdIn",
                "nC.runtime.std.stdOut",
                "nC.runtime.std.stdErr",
                "nC.runtime.std.isInteractive",
            ]
        );

        for symbol in ["stdIn", "stdOut", "stdErr"] {
            let (_, function) =
                runtime_api_find_function_by_runtime_symbol(&format!("nC.runtime.std.{symbol}"))
                    .expect("standard handle function");
            assert_eq!(function.arg_types, Vec::<String>::new());
            assert_eq!(function.return_ty, "addr");
        }

        let diagnostic = expand_use_domain_import("stoud", "io", &["streams".into()])
            .expect_err("unknown std alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_io_uuid_exposes_v4_bytes_and_strict_validation() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<uuid>;")
            .expect("stoud io uuid backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.uuid.uuidV4",
                "nC.runtime.uuid.uuidFromBytes",
                "nC.runtime.uuid.isUuid",
            ]
        );

        let (_, from_bytes) =
            runtime_api_find_function_by_runtime_symbol("nC.runtime.uuid.uuidFromBytes")
                .expect("uuidFromBytes function");
        assert_eq!(from_bytes.arg_types, ["array<byte,16>"]);
        assert_eq!(from_bytes.return_ty, "str");

        let diagnostic = expand_use_domain_import("stoud", "io", &["guid".into()])
            .expect_err("unknown UUID alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_memory_align_exposes_three_byte_width_transformations() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::memory::<align>;")
            .expect("stoud memory align backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.align.alignLeft",
                "nC.runtime.align.alignRight",
                "nC.runtime.align.alignCenter",
            ]
        );

        for symbol in ["alignLeft", "alignRight", "alignCenter"] {
            let (_, function) =
                runtime_api_find_function_by_runtime_symbol(&format!("nC.runtime.align.{symbol}"))
                    .expect("alignment function");
            assert_eq!(function.arg_types, ["str", "int"]);
            assert_eq!(function.return_ty, "str");
        }

        let diagnostic = expand_use_domain_import("stoud", "memory", &["alignment".into()])
            .expect_err("unknown alignment alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_memory_slice_exposes_manual_raw_byte_views() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::memory::<slice>;")
            .expect("stoud memory slice backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.slice.sliceStride",
                "nC.runtime.slice.sliceAt",
                "nC.runtime.slice.sliceData",
                "nC.runtime.slice.sliceLen",
                "nC.runtime.slice.sliceFlags",
                "nC.runtime.slice.sliceRead8",
                "nC.runtime.slice.sliceWrite8",
                "nC.runtime.slice.sliceView",
                "nC.runtime.slice.sliceIsEmpty",
                "nC.runtime.slice.sliceClear",
            ]
        );

        let (_, create) = runtime_api_find_function_by_runtime_symbol("nC.runtime.slice.sliceAt")
            .expect("raw slice binder");
        assert_eq!(create.arg_types, ["addr", "addr", "int", "int"]);
        assert_eq!(create.return_ty, "bool");

        let (_, read) = runtime_api_find_function_by_runtime_symbol("nC.runtime.slice.sliceRead8")
            .expect("slice reader");
        assert_eq!(read.arg_types, ["addr", "int"]);
        assert_eq!(read.return_ty, "int");

        let diagnostic = expand_use_domain_import("stoud", "memory", &["slicer".into()])
            .expect_err("unknown slice alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_memory_span_exposes_manual_raw_segment_tables() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::memory::<span>;")
            .expect("stoud memory span backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.span.spanStride",
                "nC.runtime.span.spanSegmentStride",
                "nC.runtime.span.spanSegmentAt",
                "nC.runtime.span.spanAt",
                "nC.runtime.span.spanSegments",
                "nC.runtime.span.spanCount",
                "nC.runtime.span.spanLen",
                "nC.runtime.span.spanFlags",
                "nC.runtime.span.spanRead8",
                "nC.runtime.span.spanContains8",
                "nC.runtime.span.spanIsEmpty",
                "nC.runtime.span.spanClear",
            ]
        );

        let (_, contains) =
            runtime_api_find_function_by_runtime_symbol("nC.runtime.span.spanContains8")
                .expect("span membership operation");
        assert_eq!(contains.arg_types, ["addr", "int"]);
        assert_eq!(contains.return_ty, "bool");

        let (_, read) = runtime_api_find_function_by_runtime_symbol("nC.runtime.span.spanRead8")
            .expect("span reader");
        assert_eq!(read.arg_types, ["addr", "int"]);
        assert_eq!(read.return_ty, "int");

        let diagnostic = expand_use_domain_import("stoud", "memory", &["spans".into()])
            .expect_err("unknown span alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_memory_stack_exposes_native_thread_bounds_and_owned_traces() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::memory::<stack>;")
            .expect("stoud memory stack backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.stack.stackDepth",
                "nC.runtime.stack.stackLimit",
                "nC.runtime.stack.stackRemaining",
                "nC.runtime.stack.stackTrace",
                "nC.runtime.stack.stackTraceLen",
                "nC.runtime.stack.stackTraceFrame",
                "nC.runtime.stack.stackTraceRelease",
            ]
        );

        let (_, depth) = runtime_api_find_function_by_runtime_symbol("nC.runtime.stack.stackDepth")
            .expect("stack depth operation");
        assert_eq!(depth.return_ty, "u64");

        let (_, frame) =
            runtime_api_find_function_by_runtime_symbol("nC.runtime.stack.stackTraceFrame")
                .expect("stack frame operation");
        assert_eq!(frame.arg_types, ["StackTraceHandle", "int"]);
        assert_eq!(frame.return_ty, "Result<addr, MemoryError>");

        let diagnostic = expand_use_domain_import("stoud", "memory", &["stacks".into()])
            .expect_err("unknown stack alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_time_clock_exposes_monotonic_nanoseconds_and_direct_measurement() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::time::<clock>;")
            .expect("stoud time clock backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.clock.monotonic",
                "nC.runtime.clock.elapsed",
                "nC.runtime.clock.measure",
                "nC.runtime.clock.clockResolution",
            ]
        );

        let (_, elapsed) = runtime_api_find_function_by_runtime_symbol("nC.runtime.clock.elapsed")
            .expect("clock elapsed operation");
        assert_eq!(elapsed.arg_types, ["u64"]);
        assert_eq!(elapsed.return_ty, "u64");

        let (_, measure) = runtime_api_find_function_by_runtime_symbol("nC.runtime.clock.measure")
            .expect("clock measurement operation");
        assert_eq!(measure.arg_types, ["fn() -> bool"]);
        assert_eq!(measure.return_ty, "u64");

        let diagnostic = expand_use_domain_import("stoud", "time", &["clocks".into()])
            .expect_err("unknown clock alias must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_data_json_exposes_the_complete_native_dom_abi() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::data::<json>;")
            .expect("stoud data json backend entry");
        let family = runtime_api_find_family_by_import_path(&[
            "nC".to_string(),
            "runtime".to_string(),
            "json".to_string(),
        ])
        .expect("JSON runtime family");

        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(family.functions.len(), 49);
        assert_eq!(entry.runtime_symbols.len(), family.functions.len());
        for function in &family.functions {
            assert!(entry
                .runtime_symbols
                .contains(&format!("{}.{}", family.import_path, function.name)));
        }

        let diagnostic = expand_use_domain_import("stoud", "data", &["yaml".into()])
            .expect_err("unknown data format must be rejected");
        assert_eq!(diagnostic.code, "E_USE_ITEM_UNKNOWN_001");
    }

    #[test]
    fn stoud_math_sign_exposes_signed_classification_and_saturating_abs() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::math::<sign>;")
            .expect("stoud math sign backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            ["nC.runtime.math.sign", "nC.runtime.math.abs"]
        );

        let (_, sign) = runtime_api_find_function_by_runtime_symbol("nC.runtime.math.sign")
            .expect("math sign operation");
        assert_eq!(sign.arg_types, ["int"]);
        assert_eq!(sign.return_ty, "int");
    }

    #[test]
    fn stoud_text_convert_exposes_typed_native_conversions_and_hosted_prelude() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<convert>;")
            .expect("stoud text convert backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.convert.toStr",
                "nC.runtime.convert.toText",
                "nC.runtime.convert.toInt",
                "nC.runtime.convert.toFloat",
                "nC.runtime.convert.toBool",
                "nC.runtime.convert.safeToInt",
                "nC.runtime.convert.forceToInt",
            ]
        );

        let (_, to_int) = runtime_api_find_function_by_runtime_symbol("nC.runtime.convert.toInt")
            .expect("convert toInt operation");
        assert_eq!(to_int.arg_types, ["any"]);
        assert_eq!(to_int.return_ty, "Result<int, ConvertError>");

        let (_, to_float) =
            runtime_api_find_function_by_runtime_symbol("nC.runtime.convert.toFloat")
                .expect("convert toFloat operation");
        assert_eq!(to_float.return_ty, "Result<ft64, ConvertError>");

        let implicit = hosted_runtime_prelude_resolve_function("forceToInt", 1)
            .expect("convert prelude lookup")
            .expect("forceToInt must be implicit");
        assert_eq!(implicit.canonical_symbol, "nC.runtime.convert.forceToInt");
    }

    #[test]
    fn stoud_text_escape_exposes_owned_native_sanitizers_and_hosted_prelude() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<escape>;")
            .expect("stoud text escape backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.escape.escapeAnsi",
                "nC.runtime.escape.escapeJson",
                "nC.runtime.escape.escapeControl",
                "nC.runtime.escape.sanitizeTerminal",
                "nC.runtime.escape.sanitizeLog",
            ]
        );

        for symbol in &entry.runtime_symbols {
            let (_, function) = runtime_api_find_function_by_runtime_symbol(symbol)
                .unwrap_or_else(|| panic!("missing escape operation {symbol}"));
            assert_eq!(function.arg_types, ["str"]);
            assert_eq!(function.return_ty, "str");
        }

        let implicit = hosted_runtime_prelude_resolve_function("escapeJson", 1)
            .expect("escape prelude lookup")
            .expect("escapeJson must be implicit");
        assert_eq!(implicit.canonical_symbol, "nC.runtime.escape.escapeJson");
    }

    #[test]
    fn stoud_text_format_exposes_typed_descriptor_abi_and_hosted_prelude() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::text::<format>;")
            .expect("stoud text format backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(
            entry.runtime_symbols,
            [
                "nC.runtime.format.format",
                "nC.runtime.format.formatNamed",
                "nC.runtime.format.formatInt",
                "nC.runtime.format.formatFloat",
            ]
        );

        let (_, integer) =
            runtime_api_find_function_by_runtime_symbol("nC.runtime.format.formatInt")
                .expect("formatInt operation");
        assert_eq!(integer.arg_types, ["int", "str"]);
        assert_eq!(integer.return_ty, "str");

        let (_, floating) =
            runtime_api_find_function_by_runtime_symbol("nC.runtime.format.formatFloat")
                .expect("formatFloat operation");
        assert_eq!(floating.arg_types, ["ft64", "str"]);

        let implicit = hosted_runtime_prelude_resolve_function("formatNamed", 5)
            .expect("format prelude lookup")
            .expect("formatNamed must be implicit");
        assert_eq!(implicit.canonical_symbol, "nC.runtime.format.formatNamed");
    }

    #[test]
    fn stoud_io_read_bytes_into_exposes_caller_owned_byte_storage_operation() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<readBytesInto>;")
            .expect("stoud io readBytesInto backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert!(entry.can_run_without_runtime_import_failure);
        assert_eq!(entry.runtime_symbols, ["nC.runtime.fs.readBytesInto"]);
    }

    #[test]
    fn stoud_io_flush_exposes_complete_material_flush_policy() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::io::<flush>;")
            .expect("stoud io flush backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert!(entry.closed_real_backing_bytes);
        assert_eq!(entry.runtime_symbols.len(), 5);
    }

    #[test]
    fn mobile_network_io_and_sys_requested_surface_is_material() {
        let requested = [
            (
                "mobile",
                ["telephony", "tts_engine", "permissions"].as_slice(),
            ),
            (
                "network",
                [
                    "server",
                    "client",
                    "send_bytes",
                    "receive_bytes",
                    "server_open",
                    "isWiFiConnected",
                ]
                .as_slice(),
            ),
            (
                "io",
                [
                    "input",
                    "output",
                    "render",
                    "fswrite",
                    "fread_lines",
                    "delete_file",
                    "rename_file",
                ]
                .as_slice(),
            ),
            (
                "sys",
                [
                    "permissions",
                    "get_current_time",
                    "cpu_load",
                    "registry_write",
                    "registry_read",
                    "sleep",
                ]
                .as_slice(),
            ),
        ];

        for (family, items) in requested {
            let aliases = expand_use_domain_import(
                "stoud",
                family,
                &items
                    .iter()
                    .map(|item| (*item).to_string())
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|diagnostic| panic!("{}: {}", diagnostic.code, diagnostic.message));
            assert_eq!(aliases.len(), items.len());
            for alias in aliases {
                assert_eq!(
                    alias.closure_status, "closed_real_backing_bytes",
                    "{}",
                    alias.surface_import
                );
                assert!(alias.diagnostic.is_none());
            }
        }
    }

    #[test]
    fn stoud_process_args_is_a_manual_owned_raw_descriptor_surface() {
        let path = ["nC".to_string(), "runtime".to_string(), "args".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process args runtime family");
        let actual = family
            .functions
            .iter()
            .map(|function| {
                (
                    function.name.as_str(),
                    function.arg_types.clone(),
                    function.return_ty.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                ("argsOpen", vec![], "addr"),
                ("argsCount", vec!["addr".to_string()], "int"),
                ("argsAt", vec!["addr".to_string(), "int".to_string()], "str"),
                ("argsData", vec!["addr".to_string()], "addr"),
                ("argsBytes", vec!["addr".to_string()], "int"),
                ("argsError", vec!["addr".to_string()], "int"),
                ("argsRelease", vec!["addr".to_string()], "bool"),
            ]
        );

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<args>;")
            .expect("process args backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 7);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.args.args")));
    }

    #[test]
    fn stoud_process_command_exposes_manual_process_and_pipe_descriptors() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "command".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process command runtime family");
        assert_eq!(family.functions.len(), 20);
        for (name, args, result) in [
            (
                "commandOpen",
                vec!["str", "str", "addr", "addr", "addr", "int"],
                "addr",
            ),
            ("commandWait", vec!["addr", "int"], "int"),
            ("commandTerminate", vec!["addr", "int"], "bool"),
            ("commandPipeOpen", vec!["bool", "bool"], "addr"),
            ("commandPipeRead", vec!["addr", "addr", "int"], "int"),
            ("commandPipeWrite", vec!["addr", "addr", "int"], "int"),
            ("commandPipeRelease", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process command operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<command>;")
            .expect("process command backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 20);
    }

    #[test]
    fn stoud_process_env_exposes_manual_buffers_and_owned_enumeration() {
        let path = ["nC".to_string(), "runtime".to_string(), "env".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process env runtime family");
        assert_eq!(family.functions.len(), 13);
        for (name, args, result) in [
            ("envOpen", vec![], "addr"),
            ("envAt", vec!["addr", "int"], "str"),
            ("envGet", vec!["str", "addr", "int"], "int"),
            ("envSet", vec!["str", "str"], "bool"),
            ("envRemove", vec!["str"], "bool"),
            ("envExpand", vec!["str", "addr", "int"], "int"),
            ("envRelease", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process env operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<env>;")
            .expect("process env backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 13);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.env.env")));
    }

    #[test]
    fn stoud_process_exit_exposes_native_handles_without_managed_status() {
        let path = ["nC".to_string(), "runtime".to_string(), "exit".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process exit runtime family");
        assert_eq!(family.functions.len(), 9);
        for (name, args, result) in [
            ("processCurrent", vec![], "addr"),
            ("processOpen", vec!["int", "bool", "int"], "addr"),
            ("processId", vec!["addr"], "int"),
            ("processExitCode", vec!["addr"], "int"),
            ("processTerminate", vec!["addr", "int"], "bool"),
            ("processClose", vec!["addr"], "bool"),
            ("exitNow", vec!["int"], "bool"),
            ("failFast", vec!["addr", "addr", "int"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process exit operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<exit>;")
            .expect("process exit backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 9);
        assert!(!entry
            .runtime_symbols
            .iter()
            .any(|symbol| symbol.ends_with(".setExitCode")));
    }

    #[test]
    fn stoud_process_session_exposes_manual_wts_buffers_tokens_and_channels() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "session".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process session runtime family");
        assert_eq!(family.functions.len(), 26);
        for (name, args, result) in [
            ("sessionCurrentId", vec![], "int"),
            ("sessionEnumerate", vec!["addr", "addr", "addr"], "bool"),
            (
                "sessionQuery",
                vec!["addr", "int", "int", "addr", "addr"],
                "bool",
            ),
            ("sessionQueryUserToken", vec!["int", "addr"], "bool"),
            (
                "sessionChannelRead",
                vec!["addr", "int", "addr", "int", "addr"],
                "bool",
            ),
            ("sessionChannelClose", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process session operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<session>;")
            .expect("process session backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 26);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.session.session")));
    }

    #[test]
    fn stoud_process_signal_exposes_manual_events_waits_apc_and_exception_handlers() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "signal".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process signal runtime family");
        assert_eq!(family.functions.len(), 20);
        for (name, args, result) in [
            (
                "signalCreateEvent",
                vec!["addr", "bool", "bool", "addr"],
                "addr",
            ),
            ("signalWaitOne", vec!["addr", "int"], "int"),
            (
                "signalWaitManyEx",
                vec!["int", "addr", "bool", "int", "bool"],
                "int",
            ),
            ("signalQueueApc", vec!["addr", "addr", "int"], "int"),
            ("signalAddVectored", vec!["int", "addr"], "addr"),
            (
                "signalRaiseException",
                vec!["int", "int", "int", "addr"],
                "bool",
            ),
            ("signalClose", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process signal operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<signal>;")
            .expect("process signal backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 20);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.signal.signal")));
    }

    #[test]
    fn stoud_process_temp_exposes_manual_buffers_handles_paths_and_cleanup() {
        let path = ["nC".to_string(), "runtime".to_string(), "temp".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process temp runtime family");
        assert_eq!(family.functions.len(), 15);
        for (name, args, result) in [
            ("tempWideBuffer", vec!["int"], "addr"),
            ("tempDirectory", vec!["addr", "int"], "int"),
            ("tempFileName", vec!["addr", "addr", "int", "addr"], "int"),
            (
                "tempOpen",
                vec!["addr", "int", "int", "addr", "int", "int", "addr"],
                "addr",
            ),
            ("tempDeleteFile", vec!["addr"], "bool"),
            ("tempFinalPath", vec!["addr", "addr", "int", "int"], "int"),
            ("tempClose", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process temp operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<temp>;")
            .expect("process temp backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 15);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.temp.temp")));
    }

    #[test]
    fn stoud_process_user_exposes_manual_identity_tokens_sids_and_profiles() {
        let path = ["nC".to_string(), "runtime".to_string(), "user".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process user runtime family");
        assert_eq!(family.functions.len(), 18);
        for (name, args, result) in [
            ("userName", vec!["addr", "addr"], "bool"),
            ("userNameEx", vec!["int", "addr", "addr"], "bool"),
            ("userOpenProcessToken", vec!["addr", "int", "addr"], "bool"),
            (
                "userTokenInformation",
                vec!["addr", "int", "addr", "int", "addr"],
                "bool",
            ),
            (
                "userCreateWellKnownSid",
                vec!["int", "addr", "addr", "addr"],
                "bool",
            ),
            ("userImpersonate", vec!["addr"], "bool"),
            ("userLoadProfile", vec!["addr", "addr"], "bool"),
            ("userClose", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process user operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<user>;")
            .expect("process user backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 18);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.user.user")));
    }

    #[test]
    fn stoud_task_event_exposes_manual_handles_waits_and_cleanup() {
        let path = ["nC".to_string(), "runtime".to_string(), "event".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("task event runtime family");
        assert_eq!(family.functions.len(), 13);
        for (name, args, result) in [
            ("eventCreate", vec!["addr", "bool", "bool", "addr"], "addr"),
            ("eventCreateEx", vec!["addr", "addr", "int", "int"], "addr"),
            ("eventWait", vec!["addr", "int"], "int"),
            (
                "eventWaitManyEx",
                vec!["int", "addr", "bool", "int", "bool"],
                "int",
            ),
            (
                "eventSignalAndWait",
                vec!["addr", "addr", "int", "bool"],
                "int",
            ),
            ("eventClose", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing task event operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::task::<event>;")
            .expect("task event backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 13);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.event.event")));
    }

    #[test]
    fn stoud_audit_assert_exposes_raw_traps_without_managed_messages() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "assert".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("audit assert runtime family");
        assert_eq!(family.functions.len(), 8);
        for (name, args) in [
            ("assert", vec!["bool"]),
            ("assertMsg", vec!["bool", "addr", "int"]),
            ("assertEq", vec!["int", "int"]),
            ("assertEqAddr", vec!["addr", "addr"]),
            ("assertNotNull", vec!["addr"]),
            ("assertRange", vec!["int", "int", "int"]),
            ("assertFail", vec![]),
            ("assertUnreachable", vec![]),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing audit assert operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, "bool");
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::audit::<assert>;")
            .expect("audit assert backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 8);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.assert.assert")));
    }

    #[test]
    fn stoud_audit_bridge_exposes_caller_owned_raw_table() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "bridge".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("audit bridge runtime family");
        assert_eq!(family.functions.len(), 9);
        for (name, args, result) in [
            ("bridgeStride", vec![], "int"),
            (
                "bridgeWrite",
                vec!["addr", "int", "int", "addr", "int", "int"],
                "bool",
            ),
            ("bridgeFind", vec!["addr", "int", "int"], "int"),
            ("bridgeActive", vec!["addr", "int", "int"], "bool"),
            ("bridgeRequire", vec!["addr", "int", "int"], "int"),
            ("bridgeTarget", vec!["addr", "int"], "addr"),
            ("bridgeMode", vec!["addr", "int"], "int"),
            ("bridgeAuthority", vec!["addr", "int"], "int"),
            ("bridgeErase", vec!["addr", "int"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing audit bridge operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::audit::<bridge>;")
            .expect("audit bridge backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 9);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.bridge.bridge")));
    }

    #[test]
    fn stoud_audit_debug_exposes_caller_owned_raw_records() {
        let path = ["nC".to_string(), "runtime".to_string(), "debug".to_string()];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("audit debug runtime family");
        assert_eq!(family.functions.len(), 10);
        for (name, args, result) in [
            ("debugStride", vec![], "int"),
            (
                "debugRecord",
                vec!["addr", "int", "int", "addr", "int", "int"],
                "bool",
            ),
            ("debugTag", vec!["addr", "int"], "int"),
            ("debugLocation", vec!["addr", "int"], "addr"),
            ("debugValue", vec!["addr", "int"], "int"),
            ("debugFlags", vec!["addr", "int"], "int"),
            ("debugDump", vec!["addr", "addr", "int"], "int"),
            ("debugChecksum", vec!["addr", "int"], "int"),
            ("debugBreak", vec![], "bool"),
            ("debugAssert", vec!["bool", "addr", "int"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing audit debug operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::audit::<debug>;")
            .expect("audit debug backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 10);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.debug.debug")));
    }

    #[test]
    fn stoud_audit_diagnostic_exposes_caller_owned_raw_records() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "diagnostic".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("audit diagnostic runtime family");
        assert_eq!(family.functions.len(), 12);
        for (name, args, result) in [
            ("diagStride", vec![], "int"),
            (
                "diagWrite",
                vec![
                    "addr", "int", "int", "int", "addr", "int", "addr", "int", "int", "int",
                ],
                "bool",
            ),
            ("diagSeverity", vec!["addr", "int"], "int"),
            ("diagCode", vec!["addr", "int"], "int"),
            ("diagMessage", vec!["addr", "int"], "addr"),
            ("diagMessageBytes", vec!["addr", "int"], "int"),
            ("diagSource", vec!["addr", "int"], "addr"),
            ("diagLine", vec!["addr", "int"], "int"),
            ("diagColumn", vec!["addr", "int"], "int"),
            ("diagFlags", vec!["addr", "int"], "int"),
            ("diagCopy", vec!["addr", "int", "addr", "int"], "bool"),
            ("diagClear", vec!["addr", "int"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing diagnostic operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::audit::<diagnostic>;")
            .expect("audit diagnostic backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 12);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.diagnostic.diag")));
    }

    #[test]
    fn stoud_remaining_audit_families_are_manual_raw_and_material() {
        for (item, import_path, function_count) in [
            ("log", "nC.runtime.log", 10),
            ("profile", "nC.runtime.profile", 12),
            ("status", "nC.runtime.status", 9),
            ("trace", "nC.runtime.trace", 11),
            ("validate", "nC.runtime.validate", 10),
        ] {
            let path = import_path
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let family = runtime_api_find_family_by_import_path(&path)
                .unwrap_or_else(|| panic!("missing audit family {item}"));
            assert_eq!(family.functions.len(), function_count, "{item}");
            assert!(family.functions.iter().all(|function| {
                function.return_ty != "any"
                    && function.return_ty != "result"
                    && function
                        .arg_types
                        .iter()
                        .all(|arg| arg != "any" && arg != "str")
            }));

            let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
            let surface = format!("use::stoud::audit::<{item}>;");
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing backend entry {surface}"));
            assert_eq!(entry.closure_status, "closed_real_backing_bytes", "{item}");
            assert_eq!(entry.runtime_symbols.len(), function_count, "{item}");
        }
    }

    #[test]
    fn auth_resource_families_are_distinct_manual_raw_and_material() {
        for (item, import_path, function_count) in [
            ("handle", "nC.runtime.authHandle", 10),
            ("resource", "nC.runtime.authResource", 14),
            ("session", "nC.runtime.authSession", 13),
        ] {
            let path = import_path
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let family = runtime_api_find_family_by_import_path(&path)
                .unwrap_or_else(|| panic!("missing auth resource family {item}"));
            assert_eq!(family.functions.len(), function_count, "{item}");
            assert!(family.functions.iter().all(|function| {
                function.return_ty != "any"
                    && function.return_ty != "result"
                    && function
                        .arg_types
                        .iter()
                        .all(|arg| arg != "any" && arg != "str")
            }));

            let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Auth);
            let surface = format!("use::auth::resource::<{item}>;");
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing backend entry {surface}"));
            assert_eq!(entry.closure_status, "closed_real_backing_bytes", "{item}");
            assert_eq!(entry.runtime_symbols.len(), function_count, "{item}");
        }
    }

    #[test]
    fn auth_audit_and_system_authority_are_manual_raw_and_material() {
        for (family_name, item, import_path, function_count) in [
            ("security", "audit", "nC.runtime.securityAudit", 12),
            ("system", "file", "nC.runtime.authFile", 13),
            ("system", "dir", "nC.runtime.authDir", 13),
            ("system", "path", "nC.runtime.authPath", 13),
            ("system", "env", "nC.runtime.authEnv", 13),
            ("system", "permission", "nC.runtime.authPermission", 13),
            ("system", "service", "nC.runtime.authService", 13),
            ("system", "signal", "nC.runtime.authSignal", 13),
            ("system", "session", "nC.runtime.authSystemSession", 13),
            ("security", "policy", "nC.runtime.authPolicy", 13),
            ("security", "capability", "nC.runtime.authCapability", 13),
            ("security", "sandbox", "nC.runtime.authSandbox", 13),
        ] {
            let path = import_path
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let family = runtime_api_find_family_by_import_path(&path)
                .unwrap_or_else(|| panic!("missing auth family {family_name}.{item}"));
            assert_eq!(
                family.functions.len(),
                function_count,
                "{family_name}.{item}"
            );
            assert!(family.functions.iter().all(|function| {
                function.return_ty != "any"
                    && function.return_ty != "result"
                    && function
                        .arg_types
                        .iter()
                        .all(|arg| arg != "any" && arg != "str")
            }));

            let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Auth);
            let surface = format!("use::auth::{family_name}::<{item}>;");
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing backend entry {surface}"));
            assert_eq!(entry.closure_status, "closed_real_backing_bytes");
            assert_eq!(entry.runtime_symbols.len(), function_count);
        }
    }

    #[test]
    fn dom_compiler_is_import_free_manual_raw_and_material() {
        let path = ["nC", "runtime", "compilerManual"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let family =
            runtime_api_find_family_by_import_path(&path).expect("manual compiler runtime family");
        assert_eq!(family.functions.len(), 21);
        assert!(family.functions.iter().all(|function| {
            function.return_ty != "any"
                && function.return_ty != "str"
                && function
                    .arg_types
                    .iter()
                    .all(|arg| arg != "any" && arg != "str")
        }));

        let parse = hosted_runtime_prelude_resolve_function("compilerParse", 1)
            .expect("compiler prelude lookup")
            .expect("compilerParse must be import-free");
        assert_eq!(
            parse.canonical_symbol,
            "nC.runtime.compilerManual.compilerParse"
        );

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Dom);
        for item in [
            "parse",
            "ast",
            "sema",
            "lowering",
            "diagnostics",
            "report",
            "manifest",
        ] {
            let surface = format!("use::dom::compiler::<{item}>;");
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.surface_import == surface)
                .unwrap_or_else(|| panic!("missing compiler entry {surface}"));
            assert_eq!(entry.closure_status, "closed_real_backing_bytes");
            assert_eq!(entry.runtime_symbols.len(), family.functions.len());
        }
    }

    #[test]
    fn remaining_dom_families_are_import_free_manual_raw_and_material() {
        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Dom);
        for (family_name, import_path, function_count, probe, arity, items) in [
            (
                "ai",
                "nC.runtime.aiManual",
                14,
                "aiScore",
                3,
                &["model", "agent", "memory", "policy", "eval"][..],
            ),
            (
                "high",
                "nC.runtime.highManual",
                15,
                "highEval",
                3,
                &["typeinfo", "metadata", "consteval", "comptime", "macros"][..],
            ),
            (
                "inspect",
                "nC.runtime.inspectManual",
                14,
                "inspectChecksum",
                2,
                &["site", "metadata", "trace", "report", "evidence"][..],
            ),
            (
                "tooling",
                "nC.runtime.toolingManual",
                16,
                "toolingCountLines",
                2,
                &["fmt", "lint", "lsp", "package", "build"][..],
            ),
        ] {
            let path = import_path
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let family = runtime_api_find_family_by_import_path(&path)
                .unwrap_or_else(|| panic!("missing DOM family {family_name}"));
            assert_eq!(family.functions.len(), function_count, "{family_name}");
            assert!(family.functions.iter().all(|function| {
                function.return_ty != "any"
                    && function.return_ty != "str"
                    && function
                        .arg_types
                        .iter()
                        .all(|arg| arg != "any" && arg != "str")
            }));
            let resolved = hosted_runtime_prelude_resolve_function(probe, arity)
                .expect("DOM prelude lookup")
                .unwrap_or_else(|| panic!("missing import-free operation {probe}"));
            assert!(resolved.canonical_symbol.starts_with(import_path));

            for item in items {
                let surface = format!("use::dom::{family_name}::<{item}>;");
                let entry = manifest
                    .entries
                    .iter()
                    .find(|entry| entry.surface_import == surface)
                    .unwrap_or_else(|| panic!("missing DOM entry {surface}"));
                assert_eq!(entry.closure_status, "closed_real_backing_bytes");
                assert_eq!(entry.runtime_symbols.len(), function_count);
            }
        }
    }

    #[test]
    fn memory_bytes_is_manual_raw_and_material() {
        let path = ["nC", "runtime", "bytes"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let family =
            runtime_api_find_family_by_import_path(&path).expect("manual memory bytes family");
        assert_eq!(family.functions.len(), 5);
        assert!(family.functions.iter().all(|function| {
            function.return_ty != "any"
                && function.return_ty != "str"
                && !function.return_ty.contains("Result")
                && function
                    .arg_types
                    .iter()
                    .all(|arg| arg != "any" && arg != "str")
        }));

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::memory::<bytes>;")
            .expect("memory bytes backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 5);
    }

    #[test]
    fn stoud_process_service_exposes_raw_scm_handles_buffers_and_callbacks() {
        let path = [
            "nC".to_string(),
            "runtime".to_string(),
            "service".to_string(),
        ];
        let family =
            runtime_api_find_family_by_import_path(&path).expect("process service runtime family");
        assert_eq!(family.functions.len(), 25);
        for (name, args, result) in [
            ("serviceWideAlloc", vec!["str"], "addr"),
            ("serviceManagerOpen", vec!["addr", "addr", "int"], "addr"),
            ("serviceOpen", vec!["addr", "addr", "int"], "addr"),
            (
                "serviceQueryStatus",
                vec!["addr", "int", "addr", "int", "addr"],
                "bool",
            ),
            (
                "serviceRegisterHandler",
                vec!["addr", "addr", "addr"],
                "addr",
            ),
            ("serviceSetStatus", vec!["addr", "addr"], "bool"),
            ("serviceClose", vec!["addr"], "bool"),
        ] {
            let function = family
                .functions
                .iter()
                .find(|function| function.name == name)
                .unwrap_or_else(|| panic!("missing process service operation {name}"));
            assert_eq!(function.arg_types, args);
            assert_eq!(function.return_ty, result);
        }

        let manifest = use_backend_lowering_manifest_for_domain(UseDomain::Stoud);
        let entry = manifest
            .entries
            .iter()
            .find(|entry| entry.surface_import == "use::stoud::process::<service>;")
            .expect("process service backend entry");
        assert_eq!(entry.closure_status, "closed_real_backing_bytes");
        assert_eq!(entry.runtime_symbols.len(), 25);
        assert!(entry
            .runtime_symbols
            .iter()
            .all(|symbol| symbol.starts_with("nC.runtime.service.service")));
    }
}
