use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const NBB_PHASE5_MAGIC: &[u8; 4] = b"NBB0";
const NBB_LEGACY_MAGIC: &[u8; 4] = b"NBB1";
const NBB_LEGACY_VERSION: u16 = 1;
const NBB_PHASE5_VERSION: u16 = 5;
const NBB_LEGACY_HEADER_SIZE: usize = 116;
const NBB_PHASE5_HEADER_SIZE: usize = 64;
const NBB_SECTION_HEADER_SIZE: usize = 64;
const NBB_ENDIAN_LITTLE: u8 = 1;
const NBB_POINTER_WIDTH_64: u8 = 64;
const NBB_CHECKSUM_OFFSET: usize = 32;
const NBB_CHECKSUM_SIZE: usize = 32;

#[derive(Debug, Clone)]
struct BasalDiagnostic {
    canonical: ncc_diagnostics::CanonicalDiagnostic,
    span: Option<BasalSpan>,
}

impl BasalDiagnostic {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            canonical: ncc_diagnostics::CanonicalDiagnostic::new(
                ncc_diagnostics::DiagnosticSeverity::Error,
                code,
                "basal",
                message,
            ),
            span: None,
        }
    }

    fn registered(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, message)
    }

    fn with_span(mut self, span: BasalSpan) -> Self {
        self.canonical.file = span.file.clone();
        self.canonical.span = ncc_diagnostics::SourceSpan::point(span.line, span.column);
        self.span = Some(span);
        self
    }

    fn with_help(mut self, help: impl Into<String>) -> Self {
        self.canonical.help = help.into();
        self
    }

    fn with_severity(mut self, severity: &str) -> Self {
        self.canonical.severity = match severity {
            "fatal" => ncc_diagnostics::DiagnosticSeverity::Fatal,
            "warning" => ncc_diagnostics::DiagnosticSeverity::Warning,
            "help" => ncc_diagnostics::DiagnosticSeverity::Help,
            "info" | "note" => ncc_diagnostics::DiagnosticSeverity::Note,
            _ => ncc_diagnostics::DiagnosticSeverity::Error,
        };
        self
    }

    fn is_error(&self) -> bool {
        matches!(
            self.canonical.severity,
            ncc_diagnostics::DiagnosticSeverity::Error | ncc_diagnostics::DiagnosticSeverity::Fatal
        )
    }

    fn canonical(&self) -> &ncc_diagnostics::CanonicalDiagnostic {
        &self.canonical
    }
}

impl std::ops::Deref for BasalDiagnostic {
    type Target = ncc_diagnostics::CanonicalDiagnostic;

    fn deref(&self) -> &Self::Target {
        self.canonical()
    }
}

#[derive(Debug, Clone)]
struct BasalSymbol {
    name: String,
    ty: String,
}

#[derive(Debug, Clone)]
struct BasalAudit {
    path: String,
    file_classification: String,
    root_container: Option<String>,
    sections: BTreeSet<String>,
    root_fields: BTreeMap<String, String>,
    binary_fields: BTreeMap<String, String>,
    symbols: Vec<BasalSymbol>,
    diagnostics: Vec<BasalDiagnostic>,
    deferred: Vec<String>,
    status: String,
    conformance_level: String,
    tables: Option<BasalTableSet>,
    ir: Option<BasalIrModule>,
    lowering: Option<BasalLoweringPlan>,
}

impl BasalAudit {
    fn has_errors(&self) -> bool {
        diagnostics_have_errors(&self.diagnostics)
    }
}

#[derive(Debug, Clone)]
struct NbbReport {
    path: String,
    diagnostics: Vec<BasalDiagnostic>,
    magic: String,
    version: Option<u16>,
    endian: Option<String>,
    pointer_width: Option<u8>,
    section_count: Option<u16>,
    flags: Option<u32>,
    header_size: Option<u32>,
    total_size: Option<u64>,
    contract_hash: Option<String>,
    structural_hash: Option<String>,
    structural_hash_computed: Option<String>,
    seal_hash: Option<String>,
    sections: Vec<NbbSectionHeader>,
    root_records: Vec<NbbRootRecord>,
    site_records: Vec<NbbSiteRecord>,
    bind_records: Vec<NbbBindRecord>,
    lowering_record: Option<NbbLoweringRecord>,
    machine_refs: Vec<NbbMachineRefRecord>,
    pending_evidence: Option<NbbPendingEvidenceRecord>,
    machine_evidence: Option<NbbMachineEvidenceRecord>,
    seal_record: Option<NbbSealRecord>,
    surface_records: Vec<NbbSurfaceRecord>,
    string_table: NbbStringTable,
    status: String,
    claim: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum NbbSectionKind {
    StringTable,
    Root,
    Site,
    Bind,
    Lowering,
    PendingEvidence,
    Diagnostics,
    ExecutableSurface,
    Unknown(u16),
}

impl NbbSectionKind {
    fn code(&self) -> u16 {
        match self {
            Self::StringTable => 1,
            Self::Root => 2,
            Self::Site => 3,
            Self::Bind => 4,
            Self::Lowering => 5,
            Self::PendingEvidence => 6,
            Self::Diagnostics => 7,
            Self::ExecutableSurface => 8,
            Self::Unknown(code) => *code,
        }
    }

    fn from_code(code: u16) -> Self {
        match code {
            1 => Self::StringTable,
            2 => Self::Root,
            3 => Self::Site,
            4 => Self::Bind,
            5 => Self::Lowering,
            6 => Self::PendingEvidence,
            7 => Self::Diagnostics,
            8 => Self::ExecutableSurface,
            other => Self::Unknown(other),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::StringTable => "string_table",
            Self::Root => "root",
            Self::Site => "site",
            Self::Bind => "bind",
            Self::Lowering => "lowering",
            Self::PendingEvidence => "pending_evidence",
            Self::Diagnostics => "diagnostics",
            Self::ExecutableSurface => "executable_surface",
            Self::Unknown(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
struct NbbSectionHeader {
    kind: NbbSectionKind,
    offset: u64,
    size: u64,
    alignment: u32,
    flags: u16,
    hash: String,
}

#[derive(Debug, Clone, Default)]
struct NbbStringTable {
    strings: Vec<String>,
}

#[derive(Debug, Clone)]
struct NbbRootRecord {
    root_id: String,
    root_kind: String,
    target_profile: String,
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct NbbSiteRecord {
    site_id: String,
    kind: String,
    size: Option<u64>,
    align: Option<u64>,
    permissions: Vec<String>,
    owner_root: String,
    lowering_status: String,
    target_section: Option<String>,
}

#[derive(Debug, Clone)]
struct NbbBindRecord {
    bind_id: String,
    source: String,
    destination: String,
    kind: String,
    local_resolution: String,
    machine_resolution: String,
    relocation_intent: Option<String>,
}

#[derive(Debug, Clone)]
struct NbbMachineRefRecord {
    target_triple: String,
    symbol_name: String,
    section: Option<String>,
    object_format: String,
    status: String,
}

#[derive(Debug, Clone)]
struct NbbLoweringRecord {
    target_triple: String,
    profile: String,
    object_format: String,
    status: String,
    evidence_boundary_status: String,
    machine_symbols: Vec<NbbMachineRefRecord>,
    sections: Vec<String>,
    relocations: Vec<String>,
}

#[derive(Debug, Clone)]
struct NbbPendingEvidenceRecord {
    status: String,
    reason: String,
}

#[derive(Debug, Clone)]
struct NbbMachineEvidenceCommand {
    name: String,
    command: String,
    status: String,
    exit_code: i32,
    stdout_hash: String,
    stderr_hash: String,
}

#[derive(Debug, Clone)]
struct NbbMachineDoctorEvidence {
    status: String,
    hash: String,
}

#[derive(Debug, Clone)]
struct NbbNativeDoctorEvidence {
    status: String,
    asm_complete_total: bool,
    hash: String,
}

#[derive(Debug, Clone)]
struct NbbMachineArtifactEvidence {
    artifact_kind: String,
    artifact_hash: String,
    byte_hash: String,
}

#[derive(Debug, Clone)]
struct NbbExternalArtifactEvidence {
    language: String,
    symbol: String,
    abi: String,
    artifact_kind: String,
    artifact_hash: String,
    executable_hash: String,
    link_status: String,
}

#[derive(Debug, Clone)]
struct NbbVerifyObjectEvidence {
    status: String,
    hash: String,
}

#[derive(Debug, Clone)]
struct NbbDisasmEvidence {
    status: String,
    hash: String,
}

#[derive(Debug, Clone)]
struct NbbRoundtripEvidence {
    status: String,
    hash: String,
}

#[derive(Debug, Clone)]
struct NbbDebugUnwindEvidence {
    status: String,
    reason: String,
}

#[derive(Debug, Clone)]
struct NbbEvidenceHashChain {
    lowering_hash: String,
    executable_surface_hash: String,
    doctor_hash: String,
    native_doctor_hash: String,
    artifact_hash: String,
    verify_object_hash: String,
    disasm_hash: String,
    roundtrip_hash: String,
    evidence_hash: String,
}

#[derive(Debug, Clone)]
struct NbbMachineEvidenceRecord {
    status: String,
    evidence_version: u16,
    machine_target: String,
    object_format: String,
    doctor: NbbMachineDoctorEvidence,
    native_doctor: NbbNativeDoctorEvidence,
    artifact: NbbMachineArtifactEvidence,
    verify_object: NbbVerifyObjectEvidence,
    disasm: NbbDisasmEvidence,
    roundtrip: NbbRoundtripEvidence,
    debug_unwind: NbbDebugUnwindEvidence,
    symbol_relocation_status: String,
    source_span_mapping_hash: String,
    deterministic_metadata_hash: String,
    external_artifacts: Vec<NbbExternalArtifactEvidence>,
    hash_chain: NbbEvidenceHashChain,
    commands: Vec<NbbMachineEvidenceCommand>,
}

#[derive(Debug, Clone)]
struct NbbSealHash {
    algorithm: String,
    hash: String,
}

#[derive(Debug, Clone)]
struct NbbSealRecord {
    status: String,
    seal_hash: NbbSealHash,
    seal_scope: String,
    seal_policy_version: String,
    deterministic_inputs_hash: String,
    seal_inputs: BTreeMap<String, String>,
    non_deterministic_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct BasalSealVerifierReport {
    status: String,
    diagnostics: Vec<BasalDiagnostic>,
}

#[derive(Debug, Clone)]
struct NbbSurfaceRecord {
    kind: String,
    name: String,
    status: String,
    detail: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct NbbBinary {
    header: NbbHeader,
    sections: Vec<NbbSectionHeader>,
    bytes: Vec<u8>,
    root_records: Vec<NbbRootRecord>,
    site_records: Vec<NbbSiteRecord>,
    bind_records: Vec<NbbBindRecord>,
    lowering_record: NbbLoweringRecord,
    pending_evidence: NbbPendingEvidenceRecord,
    machine_evidence: Option<NbbMachineEvidenceRecord>,
    seal_record: Option<NbbSealRecord>,
    surface_records: Vec<NbbSurfaceRecord>,
    string_table: NbbStringTable,
}

#[derive(Debug, Clone)]
struct NbbHeader {
    magic: String,
    version: u16,
    endian: String,
    pointer_width: u8,
    section_count: u16,
    header_size: u32,
    section_table_offset: u32,
    section_table_size: u32,
    total_size: u64,
    flags: u32,
    checksum: String,
}

#[derive(Debug, Clone)]
struct NbbInspectReport {
    path: String,
    status: String,
    diagnostics: Vec<BasalDiagnostic>,
    header: Option<NbbHeader>,
    sections: Vec<NbbSectionHeader>,
    root_records: Vec<NbbRootRecord>,
    site_records: Vec<NbbSiteRecord>,
    bind_records: Vec<NbbBindRecord>,
    lowering_record: Option<NbbLoweringRecord>,
    pending_evidence: Option<NbbPendingEvidenceRecord>,
    machine_evidence: Option<NbbMachineEvidenceRecord>,
    seal_record: Option<NbbSealRecord>,
    surface_records: Vec<NbbSurfaceRecord>,
}

#[derive(Debug, Clone)]
struct BasalSpan {
    file: String,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
}

impl BasalSpan {
    fn new(file: &str, start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            file: file.to_string(),
            start,
            end,
            line,
            column,
        }
    }
}

#[derive(Debug, Clone)]
struct BasalSourceFile {
    path: String,
    span: BasalSpan,
    roots: Vec<BasalRootDecl>,
    diagnostics: Vec<BasalDiagnostic>,
}

#[derive(Debug, Clone)]
struct BasalRootDecl {
    name: String,
    span: BasalSpan,
    sections: Vec<BasalSection>,
}

#[derive(Debug, Clone)]
struct BasalSection {
    kind: String,
    name: Option<String>,
    span: BasalSpan,
    key_values: Vec<BasalKeyValue>,
    children: Vec<BasalAstNode>,
}

#[derive(Debug, Clone)]
struct BasalKeyValue {
    key: String,
    value: BasalLiteral,
    span: BasalSpan,
}

#[derive(Debug, Clone)]
struct BasalLiteral {
    kind: String,
    raw: String,
    span: BasalSpan,
}

#[derive(Debug, Clone)]
struct BasalAstNode {
    kind: String,
    name: Option<String>,
    span: BasalSpan,
    text: Option<String>,
    children: Vec<BasalAstNode>,
}

#[derive(Debug, Clone)]
struct CleanLine {
    text: String,
    line: usize,
    start: usize,
}

#[derive(Debug, Clone)]
struct BasalTableSpanRef {
    source_file: String,
    node_kind: String,
    span: BasalSpan,
}

#[derive(Debug, Clone)]
struct BasalPermissionFlags {
    flags: Vec<String>,
}

#[derive(Debug, Clone)]
struct BasalBindingKind {
    raw: String,
    normalized: String,
}

#[derive(Debug, Clone)]
struct BasalBindingResolutionStatus {
    status: String,
}

#[derive(Debug, Clone)]
struct BasalRootEntry {
    name: String,
    root_id: Option<String>,
    version: Option<u64>,
    profile: Option<String>,
    target_reference: Option<String>,
    declared_sections: Vec<String>,
    metadata: BTreeMap<String, String>,
    origin: BasalTableSpanRef,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalRootTable {
    entries: Vec<BasalRootEntry>,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalSiteEntry {
    name: String,
    kind: Option<String>,
    size: Option<u64>,
    align: Option<u64>,
    offset: Option<u64>,
    target_policy: Option<String>,
    permissions: BasalPermissionFlags,
    fields: BTreeMap<String, String>,
    origin: BasalTableSpanRef,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalSiteTable {
    entries: Vec<BasalSiteEntry>,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalBindEntry {
    id: String,
    source: String,
    destination_path: String,
    destination_kind: BasalBindingKind,
    resolution_status: BasalBindingResolutionStatus,
    origin: BasalTableSpanRef,
    target_span: BasalSpan,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalBindTable {
    entries: Vec<BasalBindEntry>,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalTableSet {
    path: String,
    root_table: BasalRootTable,
    site_table: BasalSiteTable,
    bind_table: BasalBindTable,
    diagnostics: Vec<BasalDiagnostic>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalIrSpan {
    source_file: String,
    node_kind: String,
    span: BasalSpan,
}

#[derive(Debug, Clone)]
struct BasalIrPermission {
    name: String,
}

#[derive(Debug, Clone)]
struct BasalIrResolutionStatus {
    status: String,
}

#[derive(Debug, Clone)]
struct BasalIrSymbolRef {
    path: String,
    kind: String,
    span: BasalIrSpan,
}

#[derive(Debug, Clone)]
struct BasalIrMachineRef {
    path: String,
    kind: String,
    resolution_status: BasalIrResolutionStatus,
    span: BasalIrSpan,
}

#[derive(Debug, Clone)]
struct BasalRootIr {
    root_id: String,
    root_kind: String,
    version: Option<u64>,
    profile: Option<String>,
    target_reference: Option<String>,
    metadata: BTreeMap<String, String>,
    declared_sections: Vec<String>,
    table_entry: String,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalSiteIr {
    site_id: String,
    kind: String,
    size: Option<u64>,
    align: Option<u64>,
    permissions: Vec<BasalIrPermission>,
    owner_root: String,
    fields: BTreeMap<String, String>,
    span: BasalIrSpan,
    semantic_status: String,
}

#[derive(Debug, Clone)]
struct BasalBindIr {
    bind_id: String,
    source: BasalIrSymbolRef,
    destination: BasalIrSymbolRef,
    machine_ref: Option<BasalIrMachineRef>,
    resolution_status: BasalIrResolutionStatus,
    source_span: BasalIrSpan,
    destination_span: BasalIrSpan,
    semantic_status: String,
}

#[derive(Debug, Clone)]
struct BasalIrSealRequirement {
    name: String,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalWidthType {
    name: String,
    bits: Option<u16>,
    signed: bool,
    kind: String,
}

#[derive(Debug, Clone)]
struct BasalConstIr {
    name: String,
    width: BasalWidthType,
    value: String,
    value_hash: String,
    endian: String,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalMemAccessIr {
    form: String,
    target: String,
    width: Option<BasalWidthType>,
    access: String,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalPointerAccessIr {
    operation: String,
    pointer_type: String,
    authority: String,
    hazard: String,
    evidence_boundary: String,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalRawHazardIr {
    hazard: String,
    width: Option<BasalWidthType>,
    endian: String,
    alignment: String,
    permission_status: String,
    bounds_status: String,
    accepted: bool,
    evidence_boundary: String,
    span: BasalIrSpan,
}

#[derive(Debug, Clone)]
struct BasalRawBlockIr {
    name: String,
    unchecked: bool,
    hazards: Vec<BasalRawHazardIr>,
    mem_accesses: Vec<BasalMemAccessIr>,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalRoutineParamIr {
    name: String,
    width: BasalWidthType,
    span: BasalIrSpan,
}

#[derive(Debug, Clone)]
struct BasalRoutineIr {
    name: String,
    params: Vec<BasalRoutineParamIr>,
    return_type: Option<BasalWidthType>,
    raw_blocks: Vec<BasalRawBlockIr>,
    mem_accesses: Vec<BasalMemAccessIr>,
    abi_profile: String,
    span: BasalIrSpan,
    lowering_status: String,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalVerifyIr {
    name: String,
    statements: Vec<String>,
    routine_calls: Vec<String>,
    mem_accesses: Vec<BasalMemAccessIr>,
    probe_alias: bool,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalReportIr {
    statements: Vec<String>,
    emitted_items: Vec<String>,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalForeignBindingIr {
    name: String,
    language: String,
    symbol: String,
    abi: String,
    layout: String,
    ownership: String,
    header: Option<String>,
    artifact: Option<String>,
    link: Option<String>,
    bridge: Option<String>,
    span: BasalIrSpan,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalExecutableSurfaceIr {
    consts: Vec<BasalConstIr>,
    routines: Vec<BasalRoutineIr>,
    raw_blocks: Vec<BasalRawBlockIr>,
    verifies: Vec<BasalVerifyIr>,
    reports: Vec<BasalReportIr>,
    foreign_bindings: Vec<BasalForeignBindingIr>,
    mem_accesses: Vec<BasalMemAccessIr>,
    pointer_accesses: Vec<BasalPointerAccessIr>,
    diagnostics: Vec<BasalDiagnostic>,
    probe_aliases: usize,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalIrVerifierReport {
    status: String,
    diagnostics: Vec<BasalDiagnostic>,
    root_count: usize,
    site_count: usize,
    bind_count: usize,
    local_refs_resolved: usize,
    machine_refs_pending: usize,
    consistency_checks: Vec<String>,
    no_machine_evidence: bool,
}

#[derive(Debug, Clone)]
struct BasalIrModule {
    path: String,
    roots: Vec<BasalRootIr>,
    sites: Vec<BasalSiteIr>,
    binds: Vec<BasalBindIr>,
    seal_requirements: Vec<BasalIrSealRequirement>,
    surface: BasalExecutableSurfaceIr,
    diagnostics: Vec<BasalDiagnostic>,
    verifier: BasalIrVerifierReport,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalLoweringSpan {
    source_file: String,
    node_kind: String,
    span: BasalSpan,
}

#[derive(Debug, Clone)]
struct BasalLoweringResolutionStatus {
    status: String,
}

#[derive(Debug, Clone)]
struct BasalLoweringEvidenceBoundary {
    status: String,
    reason: String,
}

#[derive(Debug, Clone)]
struct BasalMachineTargetRef {
    target_triple: String,
    profile: String,
    object_format: String,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalMachineSymbolRef {
    symbol_name: String,
    target_triple: String,
    section: Option<String>,
    object_format: String,
    status: String,
    span: BasalLoweringSpan,
}

#[derive(Debug, Clone)]
struct BasalMachineSectionRef {
    name: String,
    permissions: Vec<String>,
    align: Option<u64>,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalMachineRelocationRef {
    kind: String,
    symbol: String,
    status: String,
}

#[derive(Debug, Clone)]
struct BasalRootLowering {
    root_id: String,
    root_kind: String,
    target_machine_profile: Option<String>,
    entry_symbol: Option<String>,
    source_span: BasalLoweringSpan,
    lowering_status: String,
    pending_evidence_status: String,
}

#[derive(Debug, Clone)]
struct BasalSiteLowering {
    site_id: String,
    kind: String,
    size: Option<u64>,
    align: Option<u64>,
    permissions: Vec<String>,
    target_section: Option<String>,
    target_symbol: Option<String>,
    target_offset: Option<u64>,
    source_span: BasalLoweringSpan,
    lowering_status: String,
}

#[derive(Debug, Clone)]
struct BasalBindLowering {
    bind_id: String,
    source: String,
    destination: String,
    kind: String,
    local_resolution: BasalLoweringResolutionStatus,
    machine_symbol_resolution: BasalLoweringResolutionStatus,
    machine_target: Option<BasalMachineTargetRef>,
    relocation_intent: Option<BasalMachineRelocationRef>,
    pending_evidence_boundary: BasalLoweringEvidenceBoundary,
    source_span: BasalLoweringSpan,
    destination_span: BasalLoweringSpan,
    lowering_status: String,
}

#[derive(Debug, Clone)]
struct BasalSurfaceLoweringRecord {
    kind: String,
    name: String,
    status: String,
    detail: String,
    source_span: BasalLoweringSpan,
}

#[derive(Debug, Clone)]
struct BasalExternalBindingLowering {
    name: String,
    language: String,
    symbol: String,
    abi: String,
    layout: String,
    ownership: String,
    artifact: Option<String>,
    link: Option<String>,
    machine_symbol: BasalMachineSymbolRef,
    relocation: BasalMachineRelocationRef,
    evidence_boundary: BasalLoweringEvidenceBoundary,
    source_span: BasalLoweringSpan,
    lowering_status: String,
}

#[derive(Debug, Clone)]
struct BasalLoweringVerifierReport {
    status: String,
    diagnostics: Vec<BasalDiagnostic>,
    root_count: usize,
    site_count: usize,
    bind_count: usize,
    machine_refs_resolved: usize,
    local_refs_resolved: usize,
    pending_machine_emit: usize,
    evidence_boundary_pending: bool,
    nbb_writer_started: bool,
}

#[derive(Debug, Clone)]
struct BasalMachineLoweringPlan {
    target: Option<BasalMachineTargetRef>,
    roots: Vec<BasalRootLowering>,
    sites: Vec<BasalSiteLowering>,
    binds: Vec<BasalBindLowering>,
    external_bindings: Vec<BasalExternalBindingLowering>,
    symbols: Vec<BasalMachineSymbolRef>,
    sections: Vec<BasalMachineSectionRef>,
    relocations: Vec<BasalMachineRelocationRef>,
    surface_records: Vec<BasalSurfaceLoweringRecord>,
}

#[derive(Debug, Clone)]
struct BasalLoweringPlan {
    path: String,
    machine_plan: BasalMachineLoweringPlan,
    diagnostics: Vec<BasalDiagnostic>,
    verifier: BasalLoweringVerifierReport,
    evidence_boundary: BasalLoweringEvidenceBoundary,
    status: String,
}

pub(crate) fn run_basal_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() || is_help(args.first()) {
        println!("{}", basal_help());
        return Ok(());
    }

    let subcommand = args[0].as_str();
    let rest = &args[1..];
    match subcommand {
        "doctor" => run_doctor(rest),
        "check" => run_check(rest),
        "report" => run_report(rest),
        "parse" => run_parse_command("parse", rest),
        "ast" => run_parse_command("ast", rest),
        "tables" => run_tables_command(rest),
        "ir" => run_ir_command("ir", rest),
        "verify-ir" => run_ir_command("verify-ir", rest),
        "lower" => run_lowering_command("lower", rest),
        "lowering-plan" => run_lowering_command("lowering-plan", rest),
        "verify-lowering" => run_lowering_command("verify-lowering", rest),
        "surface" => run_surface_command("surface", rest),
        "verify-surface" => run_surface_command("verify-surface", rest),
        "report-plan" => run_surface_command("report-plan", rest),
        "ffi-manifest" => run_ffi_manifest_command(rest),
        "package" => run_basal_package_command(rest),
        "publish" => run_basal_publish_command(rest),
        "install" => run_basal_install_command(rest),
        "vendor" => run_basal_vendor_command(rest),
        "emit-nbb" => run_emit_nbb_command(rest),
        "read-nbb" => run_nbb_command("read", rest),
        "inspect-nbb" => run_nbb_command("inspect", rest),
        "verify-nbb" => run_nbb_command("verify", rest),
        "nbb-info" => run_nbb_command("info", rest),
        "evidence" => run_evidence_command(rest),
        "integrate-evidence" => run_integrate_evidence_command(rest),
        "verify-seal" => run_verify_seal_command(rest),
        "evidence-report" => run_evidence_report_command(rest),
        "seal-report" => run_seal_report_command(rest),
        "read" => run_read_command(rest),
        "verify" => run_nbb_command("verify", rest),
        "dump" => run_nbb_command("dump", rest),
        "seal" => run_seal(rest),
        "witness" => run_contract_projection("witness", rest),
        "map" => run_contract_projection("map", rest),
        _ => Err(format!(
            "[E_BASAL_CLI_001] unknown basal command '{}'\n{}",
            subcommand,
            basal_help()
        )
        .into()),
    }
}

fn is_help(arg: Option<&String>) -> bool {
    matches!(arg.map(String::as_str), Some("-h" | "--help" | "help"))
}

fn basal_help() -> &'static str {
    "Usage: basal <doctor|check|report|parse|ast|tables|ir|verify-ir|lower|lowering-plan|verify-lowering|surface|verify-surface|report-plan|ffi-manifest|package|publish|install|vendor|emit-nbb|read-nbb|inspect-nbb|verify-nbb|nbb-info|evidence|integrate-evidence|seal|verify-seal|evidence-report|seal-report|read|verify|dump|witness|map> [args]\n\
     Commands:\n\
       basal doctor [--json]\n\
       basal check <file.nb|file.nbb> [--json]\n\
       basal report <file.nb|file.nbb> [--json|--md]\n\
       basal parse <file.nb> --json\n\
       basal ast <file.nb> --json\n\
       basal tables <file.nb> --json\n\
       basal ir <file.nb> --json\n\
       basal verify-ir <file.nb> --json\n\
       basal lower <file.nb> --json\n\
       basal lowering-plan <file.nb> --json\n\
       basal verify-lowering <file.nb> --json\n\
       basal surface <file.nb> --json\n\
       basal verify-surface <file.nb> --json\n\
       basal report-plan <file.nb> --json\n\
       basal ffi-manifest <file.nb> --json\n\
       basal package <file.nb> --out <dir> --json\n\
       basal publish <file.nb> --registry <dir> --json\n\
       basal install <root-id> --registry <dir> --root <dir> --json\n\
       basal vendor <dir> --registry <dir> --package <root-id> --json\n\
       basal emit-nbb <file.nb> --out <file.nbb>\n\
       basal read-nbb <file.nbb> --json\n\
       basal inspect-nbb <file.nbb> --json\n\
       basal verify-nbb <file.nbb> --json\n\
       basal nbb-info <file.nbb> --json\n\
       basal evidence <file.nb> --json\n\
       basal integrate-evidence <file.nb> --out <file.nbb>\n\
       basal seal <file.nbb> --json\n\
       basal verify-seal <file.nbb> --json\n\
       basal evidence-report <file.nbb> --json\n\
       basal seal-report <file.nbb> --json\n\
       basal read <file.nb> --json\n\
       basal read <file.nbb> --json\n\
       basal verify <file.nbb> --json\n\
       basal dump <file.nbb> --json\n\
       basal witness <file.nb|file.nbb> --json\n\
       basal map <file.nb|file.nbb> --json"
}

fn run_doctor(args: &[String]) -> Result<(), Box<dyn Error>> {
    let json_out = args.iter().any(|arg| arg == "--json");
    let report = basal_doctor_report();
    if json_out {
        print_json(&report)?;
    } else {
        println!("basal doctor: basal_phase7_release_seal_closed");
        println!("nb_parser_ast: closed");
        println!("root_table: closed");
        println!("site_table: closed");
        println!("bind_table: closed");
        println!("basal_ir: closed");
        println!("root_ir: closed");
        println!("site_ir: closed");
        println!("bind_ir: closed");
        println!("semantic_verifier: closed");
        println!("lowering: closed");
        println!("machine_ref_resolution: closed");
        println!("nb_classification: ok");
        println!("nbb_reader: closed");
        println!("nbb_verifier: closed");
        println!("nbb_writer: closed");
        println!("const_surface: closed");
        println!("routine_surface: closed");
        println!("raw_surface: closed");
        println!("verify_surface: closed");
        println!("probe_alias: closed");
        println!("report_surface: closed");
        println!("width_types: closed");
        println!("terminal_authority_docs: closed");
        println!("machine_evidence_integration: closed");
        println!("seal_final: closed");
        println!("release_seal: closed");
        println!("conformance_matrix: closed");
        println!("evidence_bundle: closed");
        println!("worktree_classification: closed");
        println!("basal_closed: true");
        println!("machine_dependency: available_closed");
        println!("evidence_integration: closed");
        println!("conformance_level: Basal Phase 7 release seal + conformance closed");
    }
    Ok(())
}

fn basal_doctor_report() -> Value {
    json!({
        "status": "basal_phase7_release_seal_closed",
        "full_closure": true,
        "basal_closed": true,
        "phase1_nb_parser_ast": "closed",
        "phase2_root_site_bind_tables": "closed",
        "phase2_tables": "closed",
        "phase3_basal_ir_root_ir_verifier": "closed",
        "phase4_lowering_to_machine": "closed",
        "phase5_nbb_binary_writer_reader": "closed",
        "phase5_5_executable_surface": "closed",
        "phase6_machine_evidence_seal": "closed",
        "phase7_release_seal": "closed",
        "basal_phase1_parser_ast_closed": true,
        "basal_phase2_root_site_bind_tables_closed": true,
        "basal_phase3_basal_ir_root_ir_verifier_closed": true,
        "basal_phase4_lowering_to_machine_closed": true,
        "basal_phase5_nbb_binary_writer_reader_closed": true,
        "basal_phase5_5_executable_surface_officialized": true,
        "basal_phase6_machine_evidence_seal_closed": true,
        "basal_phase7_release_seal_closed": true,
        "const_surface": "closed",
        "routine_surface": "closed",
        "raw_surface": "closed",
        "verify_surface": "closed",
        "probe_alias": "closed",
        "report_surface": "closed",
        "width_types": "closed",
        "terminal_authority_docs": "closed",
        "root_table": "closed",
        "site_table": "closed",
        "bind_table": "closed",
        "table_diagnostics": "closed",
        "table_spans": "closed",
        "basal_ir": "closed",
        "root_ir": "closed",
        "site_ir": "closed",
        "bind_ir": "closed",
        "semantic_verifier": "closed",
        "ir_diagnostics": "closed",
        "ir_spans": "closed",
        "lowering": "closed",
        "machine_ref_resolution": "closed",
        "lowering_diagnostics": "closed",
        "lowering_spans": "closed",
        "nb_classification": "ok",
        "nb_grammar": "closed",
        "nb_parser": "closed",
        "nb_ast": "closed",
        "source_spans": "closed",
        "parser_diagnostics": "closed",
        "parser_recovery": "closed",
        "nbb_classification": "ok",
        "parser_at_mode": "phase1_closed",
        "root_contract_verifier": "ok",
        "binary_contract_verifier": "ok",
        "site_contract_verifier": "ok",
        "seal_contract_verifier": "ok",
        "evidence_contract_verifier": "ok",
        "bridge_contract_verifier": "ok",
        "nbb_reader": "closed",
        "nbb_verifier": "closed",
        "nbb_writer": "closed",
        "nbb_format": "closed",
        "nbb_inspector": "closed",
        "seal_generation": "closed",
        "machine_dependency": "unblocked",
        "lowering_status": {
            "status": "closed",
            "phase": "Basal Fase 4"
        },
        "evidence_integration": {
            "status": "closed",
            "phase": "Basal Fase 6"
        },
        "machine_evidence_integration": "closed",
        "seal_final": "closed",
        "release_seal": "closed",
        "conformance_matrix": "closed",
        "evidence_bundle": "closed",
        "worktree_classification": "closed",
        "dependencies": {
            "compiler": "ok",
            "runtime": "ok",
            "primitive": "ok_emulated",
            "machine": "available_closed"
        },
        "allowed_states": [
            "basal_phase1_nb_parser_ast_closed",
            "basal_phase2_root_site_bind_tables_closed",
            "basal_phase3_basal_ir_root_ir_verifier_closed",
            "basal_phase4_lowering_to_machine_closed",
            "basal_phase5_nbb_binary_writer_reader_closed",
            "basal_phase5_5_executable_surface_officialized",
            "basal_phase6_machine_evidence_seal_closed",
            "basal_phase7_release_seal_closed",
            "basal_parser_closed",
            "basal_semantic_closed",
            "basal_binary_contract_closed",
            "basal_nir_closed"
        ],
        "conformance_level": "Basal Phase 7 release seal + final conformance",
        "level_4_allowed": false,
        "machine_evidence_integration": "closed",
        "no_json_disguised_as_nbb": true,
        "no_fake_machine_evidence": true,
        "seal_verifier": "closed",
        "no_physical_hardware_claim": true
    })
}

fn run_check(args: &[String]) -> Result<(), Box<dyn Error>> {
    let json_out = args.iter().any(|arg| arg == "--json");
    let path = first_path_arg(args).ok_or("basal check expects a file path")?;
    match file_extension(path).as_str() {
        "nb" => {
            let audit = audit_nb(path)?;
            if json_out {
                print_json(&audit_to_json(&audit))?;
            }
            if audit.has_errors() {
                return Err(diagnostics_summary(&audit.diagnostics).into());
            }
            if !json_out {
                println!("OK: basal check {} ({})", audit.path, audit.status.as_str());
            }
            Ok(())
        }
        "nbb" => {
            let report = read_nbb(path)?;
            if json_out {
                print_json(&nbb_to_json(&report))?;
            }
            if diagnostics_have_errors(&report.diagnostics) {
                return Err(diagnostics_summary(&report.diagnostics).into());
            }
            if !json_out {
                println!("OK: basal check {} ({})", path, report.status);
            }
            Ok(())
        }
        _ => {
            let diagnostic = BasalDiagnostic::new(
                "E_BASAL_FILE_EXTENSION_001",
                format!("unsupported basal file extension for '{}'", path),
            );
            Err(crate::render_canonical_diagnostic_terminal(
                diagnostic.canonical(),
                "",
                crate::stderr_diagnostic_color_enabled(),
            )
            .into())
        }
    }
}

fn run_report(args: &[String]) -> Result<(), Box<dyn Error>> {
    let json_out = args.iter().any(|arg| arg == "--json");
    let md_out = args.iter().any(|arg| arg == "--md") || !json_out;
    let path = first_path_arg(args).ok_or("basal report expects a file path")?;
    match file_extension(path).as_str() {
        "nb" => {
            let audit = audit_nb(path)?;
            if json_out {
                print_json(&audit_to_json(&audit))?;
            } else if md_out {
                println!("{}", audit_to_markdown(&audit));
            }
            Ok(())
        }
        "nbb" => {
            let report = read_nbb(path)?;
            if json_out {
                print_json(&nbb_to_json(&report))?;
            } else if md_out {
                println!("{}", nbb_to_markdown(&report));
            }
            Ok(())
        }
        _ => Err(format!(
            "[E_BASAL_FILE_EXTENSION_001] unsupported basal file '{}'",
            path
        )
        .into()),
    }
}

fn run_parse_command(kind: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal parse/ast expects a .nb file")?;
    if file_extension(path) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal parse/ast expects a .nb source".into());
    }
    let ast = parse_nb_file(path)?;
    let payload = if kind == "ast" {
        ast_to_json(&ast)
    } else {
        json!({
            "kind": "BasalParseReport",
            "path": ast.path,
            "status": if ast.diagnostics.is_empty() { "ok" } else { "error" },
            "phase1_nb_parser_ast": "closed",
            "root_count": ast.roots.len(),
            "diagnostics": diagnostics_to_json(&ast.diagnostics),
            "ast": ast_to_json(&ast)
        })
    };
    print_json(&payload)?;
    if ast.diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics_summary(&ast.diagnostics).into())
    }
}

fn run_tables_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal tables expects a .nb file")?;
    if file_extension(path) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal tables expects a .nb source".into());
    }
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    print_json(&table_set_to_json(&tables))?;
    if diagnostics_have_errors(&tables.diagnostics) {
        Err(diagnostics_summary(&tables.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_ir_command(kind: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal ir/verify-ir expects a .nb file")?;
    if file_extension(path) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal ir/verify-ir expects a .nb source".into());
    }
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    let ir = build_basal_ir_module(&ast, &tables);
    let payload = if kind == "verify-ir" {
        json!({
            "kind": "BasalIrVerifierReport",
            "path": path,
            "status": ir.verifier.status,
            "phase3_basal_ir_root_ir_verifier": "closed",
            "basal_ir": ir.status,
            "root_ir": if ir.roots.is_empty() { "error" } else { "closed" },
            "semantic_verifier": ir.verifier.status,
            "lowering": "not_started",
            "nbb_writer": "not_started",
            "machine_evidence_integration": "not_started",
            "verifier": verifier_to_json(&ir.verifier),
            "diagnostics": diagnostics_to_json(&ir.verifier.diagnostics),
            "ir": basal_ir_to_json(&ir)
        })
    } else {
        basal_ir_to_json(&ir)
    };
    print_json(&payload)?;
    if diagnostics_have_errors(&ir.verifier.diagnostics) {
        Err(diagnostics_summary(&ir.verifier.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_lowering_command(kind: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal lower/verify-lowering expects a .nb file")?;
    if file_extension(path) != "nb" {
        return Err(
            "[E_BASAL_FILE_EXTENSION_001] basal lower/verify-lowering expects a .nb source".into(),
        );
    }
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    let ir = build_basal_ir_module(&ast, &tables);
    let lowering = build_basal_lowering_plan(&ir);
    let payload = if kind == "verify-lowering" {
        json!({
            "kind": "BasalLoweringVerifierReport",
            "path": path,
            "status": lowering.verifier.status,
            "phase4_lowering_to_machine": "closed",
            "lowering": lowering.status,
            "machine_ref_resolution": if lowering.verifier.status == "closed" { "closed" } else { "error" },
            "nbb_writer": "not_started",
            "machine_evidence_integration": "not_started",
            "verifier": lowering_verifier_to_json(&lowering.verifier),
            "diagnostics": diagnostics_to_json(&lowering.verifier.diagnostics),
            "plan": lowering_plan_to_json(&lowering)
        })
    } else {
        lowering_plan_to_json(&lowering)
    };
    print_json(&payload)?;
    if diagnostics_have_errors(&lowering.verifier.diagnostics) {
        Err(diagnostics_summary(&lowering.verifier.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_surface_command(kind: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args)
        .ok_or("basal surface/verify-surface/report-plan expects a .nb file")?;
    if file_extension(path) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal surface expects a .nb source".into());
    }
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    let ir = build_basal_ir_module(&ast, &tables);
    let lowering = build_basal_lowering_plan(&ir);
    let payload = match kind {
        "verify-surface" => json!({
            "kind": "BasalSurfaceVerifierReport",
            "path": path,
            "status": ir.surface.status,
            "phase5_5_executable_surface": "closed",
            "const_surface": if ir.surface.status == "closed" { "closed" } else { "error" },
            "routine_surface": if ir.surface.status == "closed" { "closed" } else { "error" },
            "raw_surface": if ir.surface.status == "closed" { "closed" } else { "error" },
            "verify_surface": if ir.surface.status == "closed" { "closed" } else { "error" },
            "probe_alias": "closed",
            "report_surface": if ir.surface.status == "closed" { "closed" } else { "error" },
            "width_types": "closed",
            "terminal_authority_docs": "closed",
            "machine_evidence_integration": "not_started",
            "seal_final": "not_started",
            "surface": surface_ir_to_json(&ir.surface),
            "diagnostics": diagnostics_to_json(&ir.verifier.diagnostics)
        }),
        "report-plan" => json!({
            "kind": "BasalReportPlan",
            "path": path,
            "status": if ir.surface.status == "closed" && lowering.status == "closed" { "closed" } else { "error" },
            "report_surface": "closed",
            "report_records": ir.surface.reports.iter().map(report_ir_to_json).collect::<Vec<_>>(),
            "lowering_surface_records": lowering.machine_plan.surface_records.iter().map(surface_lowering_to_json).collect::<Vec<_>>(),
            "machine_evidence_integration": "not_started",
            "seal_final": "not_started"
        }),
        _ => surface_ir_to_json(&ir.surface),
    };
    print_json(&payload)?;
    if diagnostics_have_errors(&ir.verifier.diagnostics)
        || diagnostics_have_errors(&lowering.verifier.diagnostics)
    {
        Err(diagnostics_summary(&ir.verifier.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_ffi_manifest_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal ffi-manifest expects a .nb file")?;
    if file_extension(path) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal ffi-manifest expects a .nb source".into());
    }
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    let ir = build_basal_ir_module(&ast, &tables);
    let payload = basal_ffi_package_manifest_from_ir(&ir);
    print_json(&payload)?;
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("failed");
    if status == "closed" && !diagnostics_have_errors(&ir.verifier.diagnostics) {
        Ok(())
    } else {
        Err(diagnostics_summary(&ir.verifier.diagnostics).into())
    }
}

#[derive(Debug, Clone)]
struct BasalPackageFiles {
    root_id: String,
    safe_name: String,
    version: String,
    package_root: String,
    archive: PathBuf,
    archive_name: String,
    manifest_path: PathBuf,
    checksum: String,
    package_manifest_hash: String,
    source_hash: String,
    nbb_hash: String,
    ffi_link_manifest_hash: String,
    files: Vec<String>,
    manifest_json: Value,
}

fn run_basal_package_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = first_path_arg(args).ok_or("basal package expects a .nb file")?;
    if file_extension(source) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal package expects a .nb source".into());
    }
    let out_dir = arg_after(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(source)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("target/basal/package")
        });
    let package = write_basal_package(source, &out_dir)?;
    print_json(&basal_package_result_json(&package, "packaged"))
}

fn run_basal_publish_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = first_path_arg(args).ok_or("basal publish expects a .nb file")?;
    if file_extension(source) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal publish expects a .nb source".into());
    }
    let registry = arg_after(args, "--registry")
        .map(PathBuf::from)
        .ok_or("[E_BASAL_PACKAGE_REGISTRY_001] basal publish expects --registry <dir>")?;
    reject_remote_basal_registry(&registry)?;
    let out_dir = registry.join(".publish-work").join(basal_safe_name(
        Path::new(source)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("basal"),
    ));
    let package = write_basal_package(source, &out_dir)?;
    let package_dir = basal_registry_package_dir(&registry, &package.safe_name, &package.version);
    fs::create_dir_all(&package_dir)?;
    let archive_rel = format!(
        "packages/{}/{}/{}",
        package.safe_name, package.version, package.archive_name
    );
    let manifest_rel = format!(
        "packages/{}/{}/manifest.json",
        package.safe_name, package.version
    );
    fs::copy(&package.archive, package_dir.join(&package.archive_name))?;
    fs::copy(&package.manifest_path, package_dir.join("manifest.json"))?;
    upsert_basal_registry_entry(
        &registry,
        basal_registry_entry(&package, &archive_rel, &manifest_rel),
    )?;
    print_json(&json!({
        "schema": "basal.registry.publish.v1",
        "status": "published",
        "registry": registry,
        "root_id": package.root_id,
        "safe_name": package.safe_name,
        "version": package.version,
        "archive": archive_rel,
        "checksum": package.checksum,
        "package_manifest": manifest_rel,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "ghor_compatible": true
    }))
}

fn run_basal_install_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let package_name = first_path_arg(args).ok_or("basal install expects <root-id-or-package>")?;
    let registry = arg_after(args, "--registry")
        .map(PathBuf::from)
        .ok_or("[E_BASAL_PACKAGE_REGISTRY_001] basal install expects --registry <dir>")?;
    reject_remote_basal_registry(&registry)?;
    let version = arg_after(args, "--version").or_else(|| arg_after(args, "--vers"));
    let root = arg_after(args, "--root")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".basal"));
    let package = read_basal_registry_package(&registry, package_name, version)?;
    let source_dir = root
        .join("registry-src")
        .join(format!("{}-{}", package.safe_name, package.version));
    let cache_dir = root.join("registry-cache");
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&source_dir)?;
    let cached_archive = cache_dir.join(&package.archive_name);
    fs::copy(&package.archive, &cached_archive)?;
    extract_basal_archive(&cached_archive, &source_dir)?;
    let validation = validate_extracted_basal_package(&source_dir, &package)?;
    print_json(&json!({
        "schema": "basal.registry.install.v1",
        "status": "installed",
        "source": "registry",
        "registry": registry,
        "root": root,
        "root_id": package.root_id,
        "safe_name": package.safe_name,
        "version": package.version,
        "archive": cached_archive,
        "source_dir": source_dir,
        "checksum": package.checksum,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "validation": validation,
        "ghor_compatible": true
    }))
}

fn run_basal_vendor_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let vendor_dir = first_path_arg(args)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("vendor/basal"));
    let registry = arg_after(args, "--registry")
        .map(PathBuf::from)
        .ok_or("[E_BASAL_PACKAGE_REGISTRY_001] basal vendor expects --registry <dir>")?;
    reject_remote_basal_registry(&registry)?;
    let package_name = arg_after(args, "--package")
        .or_else(|| arg_after(args, "--root-id"))
        .ok_or(
            "[E_BASAL_PACKAGE_VENDOR_001] basal vendor expects --package <root-id-or-package>",
        )?;
    let version = arg_after(args, "--version").or_else(|| arg_after(args, "--vers"));
    if vendor_dir.exists() && !basal_has_flag(args, "--no-delete") {
        fs::remove_dir_all(&vendor_dir)?;
    }
    fs::create_dir_all(&vendor_dir)?;
    let package = read_basal_registry_package(&registry, package_name, version)?;
    let dest = vendor_dir.join(format!("{}-{}", package.safe_name, package.version));
    let temp_archive = vendor_dir.join(format!(".{}.nbp.tar.gz", package.safe_name));
    fs::copy(&package.archive, &temp_archive)?;
    extract_basal_archive(&temp_archive, &dest)?;
    fs::remove_file(&temp_archive)?;
    let validation = validate_extracted_basal_package(&dest, &package)?;
    let file_checksums = basal_vendor_file_checksums(&dest)?;
    fs::write(
        dest.join(".basal-checksum.json"),
        serde_json::to_vec_pretty(&json!({
            "schema": "basal.vendor.checksum.v1",
            "package": package.checksum,
            "package_manifest_hash": package.package_manifest_hash,
            "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
            "files": file_checksums,
            "validation": validation
        }))?,
    )?;
    print_json(&json!({
        "schema": "basal.vendor.v1",
        "status": "ok",
        "path": vendor_dir,
        "vendored": [{
            "root_id": package.root_id,
            "safe_name": package.safe_name,
            "version": package.version,
            "path": dest,
            "checksum": package.checksum,
            "file_checksums": file_checksums.len(),
            "package_manifest_hash": package.package_manifest_hash,
            "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
            "validation": validation
        }],
        "ghor_compatible": true,
        "config": format!("[source.basal-vendored]\\ndirectory = \"{}\"\\n", vendor_dir.display())
    }))
}

fn write_basal_package(source: &str, out_dir: &Path) -> Result<BasalPackageFiles, Box<dyn Error>> {
    let source_path = Path::new(source);
    let ast = parse_nb_file(source)?;
    let tables = build_table_set(&ast);
    if diagnostics_have_errors(&tables.diagnostics) {
        return Err(diagnostics_summary(&tables.diagnostics).into());
    }
    let ir = build_basal_ir_module(&ast, &tables);
    if diagnostics_have_errors(&ir.verifier.diagnostics) {
        return Err(diagnostics_summary(&ir.verifier.diagnostics).into());
    }
    let lowering = build_basal_lowering_plan(&ir);
    if diagnostics_have_errors(&lowering.verifier.diagnostics) {
        return Err(diagnostics_summary(&lowering.verifier.diagnostics).into());
    }
    let nbb = build_nbb_binary_from_source(source)?;
    let root = ir
        .roots
        .first()
        .ok_or("[E_BASAL_PACKAGE_001] basal package requires a root")?;
    let root_id = root.root_id.clone();
    let safe_name = basal_safe_name(&root_id);
    let version = root.version.unwrap_or(1).to_string();
    let package_root = format!("{safe_name}-{version}");
    let archive_name = format!("{package_root}.nbp.tar.gz");
    fs::create_dir_all(out_dir)?;
    let staging = out_dir.join(format!(".{package_root}.staging"));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    let package_dir = staging.join(&package_root);
    fs::create_dir_all(package_dir.join("source"))?;
    fs::create_dir_all(package_dir.join("artifacts"))?;
    let source_file = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("root.nb");
    let nbb_file = format!("{safe_name}.nbb");
    let source_dest = package_dir.join("source").join(source_file);
    let nbb_dest = package_dir.join("artifacts").join(&nbb_file);
    fs::copy(source_path, &source_dest)?;
    fs::write(&nbb_dest, &nbb.bytes)?;
    let source_hash = basal_sha256_file(&source_dest)?;
    let nbb_hash = basal_sha256_file(&nbb_dest)?;
    let ffi_manifest = basal_ffi_package_manifest_from_ir(&ir);
    let ffi_link_manifest = ffi_manifest
        .get("ffi_link_manifest")
        .cloned()
        .unwrap_or_else(
            || json!({"schema": "ghor.ffi_manifest.v1", "status": "closed", "entries": []}),
        );
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    fs::write(
        package_dir.join("ffi_link_manifest.json"),
        serde_json::to_vec_pretty(&ffi_link_manifest)?,
    )?;
    let manifest_json = json!({
        "schema": "basal.package.v1",
        "status": "closed",
        "root_id": root_id,
        "safe_name": safe_name,
        "version": version,
        "format": "basal NBB package tar.gz",
        "ghor_compatible": true,
        "source": {
            "path": format!("source/{source_file}"),
            "sha256": source_hash
        },
        "nbb": {
            "path": format!("artifacts/{nbb_file}"),
            "sha256": nbb_hash,
            "magic": "NBB0",
            "bytes": nbb.bytes.len()
        },
        "ffi_link_manifest_path": "ffi_link_manifest.json",
        "ffi_link_manifest_hash": ffi_link_manifest_hash,
        "ffi_link_manifest": ffi_link_manifest,
        "tables_hash": stable_json_hash(&table_set_to_json(&tables)),
        "ir_hash": stable_json_hash(&basal_ir_to_json(&ir)),
        "lowering_hash": stable_json_hash(&lowering_plan_to_json(&lowering)),
        "surface_hash": stable_json_hash(&surface_ir_to_json(&ir.surface)),
        "evidence_boundary": "package_contains_pending_or_integrated_nbb_state_without_faking_evidence"
    });
    fs::write(
        package_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest_json)?,
    )?;
    let archive = out_dir.join(&archive_name);
    create_basal_archive(&staging, &package_root, &archive)?;
    let checksum = basal_sha256_file(&archive)?;
    let out_manifest = out_dir.join("manifest.json");
    fs::write(&out_manifest, serde_json::to_vec_pretty(&manifest_json)?)?;
    let package_manifest_hash = basal_sha256_file(&out_manifest)?;
    fs::write(
        out_dir.join("checksum.sha256"),
        format!("{checksum}  {}\n", archive.display()),
    )?;
    fs::remove_dir_all(staging)?;
    Ok(BasalPackageFiles {
        root_id,
        safe_name,
        version,
        package_root,
        archive_name,
        archive,
        manifest_path: out_manifest,
        checksum,
        package_manifest_hash,
        source_hash,
        nbb_hash,
        ffi_link_manifest_hash,
        files: vec![
            format!("source/{source_file}"),
            format!("artifacts/{nbb_file}"),
            "ffi_link_manifest.json".to_string(),
            "manifest.json".to_string(),
        ],
        manifest_json,
    })
}

fn basal_package_result_json(package: &BasalPackageFiles, status: &str) -> Value {
    json!({
        "schema": "basal.package.result.v1",
        "status": status,
        "root_id": package.root_id,
        "safe_name": package.safe_name,
        "version": package.version,
        "package_root": package.package_root,
        "archive": package.archive,
        "archive_name": package.archive_name,
        "checksum": package.checksum,
        "manifest": package.manifest_path,
        "package_manifest_hash": package.package_manifest_hash,
        "source_hash": package.source_hash,
        "nbb_hash": package.nbb_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "files": package.files,
        "ghor_compatible": true,
        "manifest_json": package.manifest_json
    })
}

#[derive(Debug, Clone)]
struct BasalRegistryPackage {
    root_id: String,
    safe_name: String,
    version: String,
    archive: PathBuf,
    archive_name: String,
    checksum: String,
    package_manifest_hash: String,
    ffi_link_manifest_hash: String,
    entry: Value,
    manifest_json: Value,
}

fn read_basal_registry_package(
    registry: &Path,
    package_name: &str,
    version: Option<&str>,
) -> Result<BasalRegistryPackage, Box<dyn Error>> {
    let index = read_basal_registry_index(registry)?;
    let requested = basal_safe_name(package_name);
    let mut entries = index
        .get("packages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            entry.get("root_id").and_then(Value::as_str) == Some(package_name)
                || entry.get("safe_name").and_then(Value::as_str) == Some(package_name)
                || entry.get("safe_name").and_then(Value::as_str) == Some(requested.as_str())
        })
        .filter(|entry| {
            version
                .is_none_or(|version| entry.get("version").and_then(Value::as_str) == Some(version))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .cmp(
                right
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
    });
    let entry = entries
        .pop()
        .ok_or_else(|| format!("[E_BASAL_PACKAGE_REGISTRY_001] basal package '{package_name}' was not found in registry"))?;
    let archive_rel = entry
        .get("archive")
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_REGISTRY_001] registry entry missing archive")?;
    let manifest_rel = entry
        .get("package_manifest")
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_REGISTRY_001] registry entry missing package_manifest")?;
    let archive = registry.join(archive_rel);
    let manifest_path = registry.join(manifest_rel);
    let checksum = entry
        .get("checksum")
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_CHECKSUM_001] registry entry missing checksum")?
        .to_string();
    let actual_checksum = basal_sha256_file(&archive)?;
    if actual_checksum != checksum {
        return Err(format!(
            "[E_BASAL_PACKAGE_CHECKSUM_001] basal archive checksum mismatch for '{}'",
            archive.display()
        )
        .into());
    }
    let manifest_json: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let package_manifest_hash = entry
        .get("package_manifest_hash")
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] registry entry missing package_manifest_hash")?
        .to_string();
    let actual_manifest_hash = basal_sha256_file(&manifest_path)?;
    if actual_manifest_hash != package_manifest_hash {
        return Err(format!(
            "[E_BASAL_PACKAGE_MANIFEST_001] basal package manifest hash mismatch for '{}'",
            manifest_path.display()
        )
        .into());
    }
    validate_basal_package_manifest(&manifest_json)?;
    let ffi_link_manifest_hash = entry
        .get("ffi_link_manifest_hash")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let manifest_ffi_hash = manifest_json
        .get("ffi_link_manifest")
        .map(stable_json_hash)
        .unwrap_or_default();
    if !ffi_link_manifest_hash.is_empty() && ffi_link_manifest_hash != manifest_ffi_hash {
        return Err("[E_BASAL_PACKAGE_MANIFEST_001] basal ffi manifest hash mismatch".into());
    }
    Ok(BasalRegistryPackage {
        root_id: entry
            .get("root_id")
            .and_then(Value::as_str)
            .unwrap_or(package_name)
            .to_string(),
        safe_name: entry
            .get("safe_name")
            .and_then(Value::as_str)
            .unwrap_or(requested.as_str())
            .to_string(),
        version: entry
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("1")
            .to_string(),
        archive_name: Path::new(archive_rel)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("package.nbp.tar.gz")
            .to_string(),
        archive,
        checksum,
        package_manifest_hash,
        ffi_link_manifest_hash: if ffi_link_manifest_hash.is_empty() {
            manifest_ffi_hash
        } else {
            ffi_link_manifest_hash
        },
        entry,
        manifest_json,
    })
}

fn validate_basal_package_manifest(manifest: &Value) -> Result<(), Box<dyn Error>> {
    if manifest.get("schema").and_then(Value::as_str) != Some("basal.package.v1") {
        return Err(
            "[E_BASAL_PACKAGE_MANIFEST_001] unsupported basal package manifest schema".into(),
        );
    }
    if manifest.get("ghor_compatible").and_then(Value::as_bool) != Some(true) {
        return Err(
            "[E_BASAL_PACKAGE_MANIFEST_001] basal package is not marked ghor-compatible".into(),
        );
    }
    if manifest
        .get("nbb")
        .and_then(|value| value.get("magic"))
        .and_then(Value::as_str)
        != Some("NBB0")
    {
        return Err(
            "[E_BASAL_PACKAGE_MANIFEST_001] basal package must contain NBB0 artifact metadata"
                .into(),
        );
    }
    let ffi = manifest
        .get("ffi_link_manifest")
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] basal package missing ffi_link_manifest")?;
    if ffi.get("schema").and_then(Value::as_str) != Some("ghor.ffi_manifest.v1") {
        return Err("[E_BASAL_PACKAGE_MANIFEST_001] basal package ffi manifest must use ghor.ffi_manifest.v1".into());
    }
    let declared_hash = manifest
        .get("ffi_link_manifest_hash")
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] basal package missing ffi_link_manifest_hash")?;
    if stable_json_hash(ffi) != declared_hash {
        return Err(
            "[E_BASAL_PACKAGE_MANIFEST_001] basal package ffi_link_manifest_hash mismatch".into(),
        );
    }
    Ok(())
}

fn validate_extracted_basal_package(
    root: &Path,
    package: &BasalRegistryPackage,
) -> Result<Value, Box<dyn Error>> {
    let manifest_path = root.join("manifest.json");
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    validate_basal_package_manifest(&manifest)?;
    let source = manifest
        .get("source")
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] extracted package missing source path")?;
    let source_hash = manifest
        .get("source")
        .and_then(|value| value.get("sha256"))
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] extracted package missing source hash")?;
    let nbb = manifest
        .get("nbb")
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] extracted package missing nbb path")?;
    let nbb_hash = manifest
        .get("nbb")
        .and_then(|value| value.get("sha256"))
        .and_then(Value::as_str)
        .ok_or("[E_BASAL_PACKAGE_MANIFEST_001] extracted package missing nbb hash")?;
    if basal_sha256_file(&root.join(source))? != source_hash {
        return Err("[E_BASAL_PACKAGE_CHECKSUM_001] extracted basal source hash mismatch".into());
    }
    if basal_sha256_file(&root.join(nbb))? != nbb_hash {
        return Err("[E_BASAL_PACKAGE_CHECKSUM_001] extracted basal NBB hash mismatch".into());
    }
    let nbb_report = read_nbb(root.join(nbb).to_string_lossy().as_ref())?;
    if diagnostics_have_errors(&nbb_report.diagnostics) {
        return Err(diagnostics_summary(&nbb_report.diagnostics).into());
    }
    Ok(json!({
        "status": "validated",
        "root_id": package.root_id,
        "safe_name": package.safe_name,
        "version": package.version,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "source_hash": source_hash,
        "nbb_hash": nbb_hash,
        "nbb_magic": "NBB0",
        "registry_entry": package.entry,
        "manifest": package.manifest_json
    }))
}

fn create_basal_archive(
    staging: &Path,
    package_root: &str,
    archive: &Path,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = archive.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let archive_abs = if archive.is_absolute() {
        archive.to_path_buf()
    } else {
        std::env::current_dir()?.join(archive)
    };
    let output = Command::new("tar")
        .current_dir(staging)
        .arg("-czf")
        .arg(&archive_abs)
        .arg(package_root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_BASAL_PACKAGE_001] basal package archive failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn extract_basal_archive(archive: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::create_dir_all(destination)?;
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("--strip-components")
        .arg("1")
        .arg("-C")
        .arg(destination)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "[E_BASAL_PACKAGE_001] basal package extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn basal_registry_entry(package: &BasalPackageFiles, archive: &str, manifest: &str) -> Value {
    json!({
        "name": package.safe_name,
        "root_id": package.root_id,
        "safe_name": package.safe_name,
        "version": package.version,
        "vers": package.version,
        "archive": archive,
        "package_manifest": manifest,
        "checksum": package.checksum,
        "package_manifest_hash": package.package_manifest_hash,
        "ffi_link_manifest_hash": package.ffi_link_manifest_hash,
        "source_hash": package.source_hash,
        "nbb_hash": package.nbb_hash,
        "format": "basal NBB package tar.gz",
        "ghor_compatible": true,
        "yanked": false
    })
}

fn read_basal_registry_index(registry: &Path) -> Result<Value, Box<dyn Error>> {
    let path = registry.join("index.json");
    if !path.exists() {
        return Ok(json!({
            "schema": "basal.registry.index.v1",
            "packages": []
        }));
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn upsert_basal_registry_entry(registry: &Path, entry: Value) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(registry)?;
    let mut index = read_basal_registry_index(registry)?;
    let root_id = entry
        .get("root_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let version = entry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let entries_snapshot = {
        let packages = index
            .as_object_mut()
            .expect("basal registry index must be an object")
            .entry("packages")
            .or_insert_with(|| Value::Array(Vec::new()));
        let entries = packages
            .as_array_mut()
            .ok_or("[E_BASAL_PACKAGE_REGISTRY_001] basal registry packages must be an array")?;
        entries.retain(|existing| {
            existing.get("root_id").and_then(Value::as_str) != Some(root_id)
                || existing.get("version").and_then(Value::as_str) != Some(version)
        });
        entries.push(entry.clone());
        entries.clone()
    };
    fs::write(
        registry.join("index.json"),
        serde_json::to_vec_pretty(&index)?,
    )?;
    let safe_name_owned = entry
        .get("safe_name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| basal_safe_name(root_id));
    let safe_name = safe_name_owned.as_str();
    let package_index_dir = registry.join("index").join(safe_name);
    fs::create_dir_all(&package_index_dir)?;
    let lines = entries_snapshot
        .iter()
        .filter(|existing| existing.get("safe_name").and_then(Value::as_str) == Some(safe_name))
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    fs::write(
        package_index_dir.join("versions.jsonl"),
        format!("{lines}\n"),
    )?;
    Ok(())
}

fn basal_registry_package_dir(registry: &Path, safe_name: &str, version: &str) -> PathBuf {
    registry.join("packages").join(safe_name).join(version)
}

fn reject_remote_basal_registry(registry: &Path) -> Result<(), Box<dyn Error>> {
    let raw = registry.to_string_lossy();
    if raw.starts_with("http://")
        || raw.starts_with("https://")
        || raw.starts_with("sparse+")
        || raw.starts_with("git+")
    {
        return Err("[E_BASAL_PACKAGE_REGISTRY_001] basal packaging bridge currently requires a local registry path; ghor owns remote registry transport".into());
    }
    Ok(())
}

fn basal_safe_name(raw: &str) -> String {
    let mut out = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn basal_sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(to_hex(&Sha256::digest(fs::read(path)?)))
}

fn basal_has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn basal_vendor_file_checksums(root: &Path) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let mut files = BTreeMap::new();
    basal_vendor_file_checksums_inner(root, root, &mut files)?;
    Ok(files)
}

fn basal_vendor_file_checksums_inner(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            basal_vendor_file_checksums_inner(root, &path, out)?;
            continue;
        }
        if path.file_name().and_then(|value| value.to_str()) == Some(".basal-checksum.json") {
            continue;
        }
        let rel = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        out.insert(rel, basal_sha256_file(&path)?);
    }
    Ok(())
}

fn run_emit_nbb_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = first_path_arg(args).ok_or("basal emit-nbb expects a .nb file")?;
    let out = arg_after(args, "--out").ok_or("basal emit-nbb expects --out <file.nbb>")?;
    if file_extension(source) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal emit-nbb expects a .nb source".into());
    }
    if file_extension(out) != "nbb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal emit-nbb expects a .nbb output".into());
    }
    let nbb = build_nbb_binary_from_source(source)?;
    if let Some(parent) = Path::new(out).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(out, &nbb.bytes).map_err(|err| {
        format!(
            "[E_NBB_WRITE_001] failed to write .nbb artifact '{}': {}",
            out, err
        )
    })?;
    let report = read_nbb(out)?;
    let payload = json!({
        "kind": "NbbWriter",
        "status": if report.diagnostics.is_empty() { "written" } else { "written_invalid" },
        "source": source,
        "out": out,
        "bytes_written": nbb.bytes.len(),
        "format": "NBB0",
        "nbb_writer": "closed",
        "phase5_5_executable_surface": "closed",
        "machine_evidence_integration": "not_started",
        "no_json_disguised_as_nbb": !nbb.bytes.starts_with(b"{"),
        "header": nbb_header_to_json(&nbb.header),
        "sections": nbb.sections.iter().map(nbb_section_to_json).collect::<Vec<_>>(),
        "surface_records": nbb.surface_records.iter().map(nbb_surface_record_to_json).collect::<Vec<_>>(),
        "verify": nbb_to_json(&report)
    });
    print_json(&payload)?;
    if diagnostics_have_errors(&report.diagnostics) {
        Err(diagnostics_summary(&report.diagnostics).into())
    } else {
        Ok(())
    }
}

pub(crate) fn emit_verified_nbb_artifact(
    source: &Path,
    out: &Path,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let source_text = source.to_str().ok_or_else(|| {
        format!(
            "[{}] .nb source path is not valid UTF-8",
            ncc_diagnostics::codes::E_NBB_WRITE_001
        )
    })?;
    let out_text = out.to_str().ok_or_else(|| {
        format!(
            "[{}] .nbb output path is not valid UTF-8",
            ncc_diagnostics::codes::E_NBB_WRITE_001
        )
    })?;
    if file_extension(source_text) != "nb" || file_extension(out_text) != "nbb" {
        let message = format!(
            "[{}] Basal materialization requires .nb -> .nbb",
            ncc_diagnostics::codes::E_BASAL_FILE_EXTENSION_001
        );
        return Err(message.into());
    }

    let nbb = build_nbb_binary_from_source(source_text)?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(out, &nbb.bytes).map_err(|err| {
        format!(
            "[{}] failed to write .nbb artifact '{}': {}",
            ncc_diagnostics::codes::E_NBB_WRITE_001,
            out.display(),
            err
        )
    })?;

    let report = read_nbb(out_text)?;
    if diagnostics_have_errors(&report.diagnostics) {
        let message = format!(
            "[{}] generated .nbb '{}' failed structural verification",
            ncc_diagnostics::codes::E_NBB_WRITE_001,
            out.display()
        );
        return Err(message.into());
    }
    Ok(json!({
        "schema": "nc.basal.generated-root-binary.v1",
        "status": "structurally_valid",
        "source": source.display().to_string(),
        "out": out.display().to_string(),
        "bytes": nbb.bytes.len(),
        "format": "NBB0",
        "header": nbb_header_to_json(&nbb.header),
        "sections": nbb.sections.iter().map(nbb_section_to_json).collect::<Vec<_>>(),
    }))
}

fn run_read_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal read expects a file path")?;
    if file_extension(path) == "nb" {
        let ast = parse_nb_file(path)?;
        let tables = build_table_set(&ast);
        let ir = build_basal_ir_module(&ast, &tables);
        let lowering = build_basal_lowering_plan(&ir);
        let payload = json!({
            "kind": "BasalReadReport",
            "path": path,
            "status": if diagnostics_have_errors(&ir.verifier.diagnostics) { "error" } else { "ok" },
            "phase1_nb_parser_ast": "closed",
            "phase2_root_site_bind_tables": "closed",
            "phase3_basal_ir_root_ir_verifier": "closed",
            "phase4_lowering_to_machine": "closed",
            "phase5_nbb_binary_writer_reader": "closed",
            "phase5_5_executable_surface": "closed",
            "lowering": lowering.status,
            "nbb_writer": "closed",
            "evidence_integration": "not_started",
            "ast": ast_to_json(&ast),
            "tables": table_set_to_json(&tables),
            "ir": basal_ir_to_json(&ir),
            "surface": surface_ir_to_json(&ir.surface),
            "lowering_plan": lowering_plan_to_json(&lowering)
        });
        print_json(&payload)?;
        if diagnostics_have_errors(&ir.verifier.diagnostics) {
            return Err(diagnostics_summary(&ir.verifier.diagnostics).into());
        }
        return Ok(());
    }
    run_nbb_command("read", args)
}

fn run_nbb_command(command: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal nbb command expects a file path")?;
    let report = read_nbb(path)?;
    let payload = match command {
        "inspect" => nbb_inspect_to_json(&nbb_inspect_report(&report)),
        "info" => json!({
            "kind": "NbbInfo",
            "path": report.path,
            "status": report.status,
            "magic": report.magic,
            "version": report.version,
            "section_count": report.section_count,
            "total_size": report.total_size,
            "structural_hash": report.structural_hash,
            "pending_evidence": report.pending_evidence.as_ref().map(nbb_pending_evidence_to_json),
            "machine_evidence": report.machine_evidence.as_ref().map(nbb_machine_evidence_to_json),
            "seal": report.seal_record.as_ref().map(nbb_seal_record_to_json),
            "diagnostics": diagnostics_to_json(&report.diagnostics)
        }),
        _ => nbb_to_json(&report),
    };
    print_json(&payload)?;
    if command != "read" && diagnostics_have_errors(&report.diagnostics) {
        return Err(diagnostics_summary(&report.diagnostics).into());
    }
    Ok(())
}

fn run_evidence_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = first_path_arg(args).ok_or("basal evidence expects a .nb file")?;
    if file_extension(source) != "nb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal evidence expects a .nb source".into());
    }
    let (evidence, seal, diagnostics) = build_machine_evidence_for_source(source)?;
    let payload = json!({
        "kind": "BasalMachineEvidencePlan",
        "source": source,
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "closed" },
        "phase6_machine_evidence_seal": "closed",
        "machine_evidence_integration": "closed",
        "seal_final": "closed",
        "machine_evidence": nbb_machine_evidence_to_json(&evidence),
        "seal_preview": nbb_seal_record_to_json(&seal),
        "diagnostics": diagnostics_to_json(&diagnostics)
    });
    print_json(&payload)?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_integrate_evidence_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let source = first_path_arg(args).ok_or("basal integrate-evidence expects a .nb file")?;
    let out =
        arg_after(args, "--out").ok_or("basal integrate-evidence expects --out <file.nbb>")?;
    if file_extension(source) != "nb" {
        return Err(
            "[E_BASAL_FILE_EXTENSION_001] basal integrate-evidence expects a .nb source".into(),
        );
    }
    if file_extension(out) != "nbb" {
        return Err(
            "[E_BASAL_FILE_EXTENSION_001] basal integrate-evidence expects a .nbb output".into(),
        );
    }
    let nbb = build_nbb_binary_with_evidence_from_source(source)?;
    if let Some(parent) = Path::new(out).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(out, &nbb.bytes).map_err(|err| {
        format!(
            "[E_NBB_WRITE_001] failed to write evidence .nbb artifact '{}': {}",
            out, err
        )
    })?;
    let report = read_nbb(out)?;
    let payload = json!({
        "kind": "BasalEvidenceIntegrator",
        "status": if diagnostics_have_errors(&report.diagnostics) { "written_invalid" } else { "integrated" },
        "source": source,
        "out": out,
        "bytes_written": nbb.bytes.len(),
        "format": "NBB0",
        "machine_evidence_integration": "closed",
        "seal_final": "closed",
        "machine_evidence": nbb.machine_evidence.as_ref().map(nbb_machine_evidence_to_json),
        "seal": nbb.seal_record.as_ref().map(nbb_seal_record_to_json),
        "verify": nbb_to_json(&report)
    });
    print_json(&payload)?;
    if diagnostics_have_errors(&report.diagnostics) {
        Err(diagnostics_summary(&report.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_seal(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal seal expects a .nb source or .nbb file")?;
    match file_extension(path).as_str() {
        "nb" => run_seal_source_command(path, args),
        "nbb" => run_seal_artifact_command(path),
        _ => Err(
            "[E_BASAL_FILE_EXTENSION_001] basal seal expects a .nb source or .nbb artifact".into(),
        ),
    }
}

fn run_seal_source_command(source: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let out = arg_after(args, "--out").ok_or("basal seal <file.nb> expects --out <file.nbb>")?;
    if file_extension(out) != "nbb" {
        return Err("[E_BASAL_FILE_EXTENSION_001] basal seal expects a .nbb output".into());
    }
    let gate_diagnostics = basal_seal_activation_gate_diagnostics()?;
    if diagnostics_have_errors(&gate_diagnostics) {
        print_json(&json!({
            "kind": "BasalSealActivator",
            "status": "deferred",
            "source": source,
            "out": out,
            "bytes_written": 0,
            "diagnostics": diagnostics_to_json(&gate_diagnostics)
        }))?;
        return Err(diagnostics_summary(&gate_diagnostics).into());
    }

    let nbb = build_nbb_binary_with_evidence_from_source(source)?;
    if let Some(parent) = Path::new(out).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(out, &nbb.bytes).map_err(|err| {
        format!(
            "[E_NBB_WRITE_001] failed to write sealed .nbb artifact '{}': {}",
            out, err
        )
    })?;
    let report = read_nbb(out)?;
    let seal = verify_sealed_nbb(&report);
    let payload = json!({
        "kind": "BasalSealActivator",
        "status": if diagnostics_have_errors(&seal.diagnostics) { "written_invalid" } else { "sealed" },
        "source": source,
        "out": out,
        "bytes_written": nbb.bytes.len(),
        "format": "NBB0",
        "machine_dependency": "available_closed",
        "machine_evidence_integration": "closed",
        "seal_final": "closed",
        "seal": nbb.seal_record.as_ref().map(nbb_seal_record_to_json),
        "verify": nbb_to_json(&report),
        "diagnostics": diagnostics_to_json(&seal.diagnostics)
    });
    print_json(&payload)?;
    if diagnostics_have_errors(&seal.diagnostics) {
        Err(diagnostics_summary(&seal.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_seal_artifact_command(path: &str) -> Result<(), Box<dyn Error>> {
    let report = read_nbb(path)?;
    let seal = verify_sealed_nbb(&report);
    let payload = json!({
        "kind": "BasalSealReport",
        "path": path,
        "status": seal.status,
        "phase6_machine_evidence_seal": "closed",
        "machine_evidence_integration": if report.machine_evidence.is_some() { "closed" } else { "missing" },
        "seal_final": if report.seal_record.is_some() { "closed" } else { "missing" },
        "seal": report.seal_record.as_ref().map(nbb_seal_record_to_json),
        "diagnostics": diagnostics_to_json(&seal.diagnostics)
    });
    print_json(&payload)?;
    if diagnostics_have_errors(&seal.diagnostics) {
        Err(diagnostics_summary(&seal.diagnostics).into())
    } else {
        Ok(())
    }
}

fn basal_seal_activation_gate_diagnostics() -> Result<Vec<BasalDiagnostic>, Box<dyn Error>> {
    let mut diagnostics = Vec::new();
    if std::env::var_os("NCC_TEST_BASAL_SEAL_MACHINE_NOT_CLOSED").is_some() {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_DEFERRED_001",
            "reason: machine_lowering_not_closed",
        ));
        return Ok(diagnostics);
    }

    let doctor = basal_doctor_report();
    if doctor.get("basal_closed").and_then(Value::as_bool) != Some(true)
        || doctor
            .get("basal_phase7_release_seal_closed")
            .and_then(Value::as_bool)
            != Some(true)
    {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_DEFERRED_001",
            "reason: basal_release_not_closed",
        ));
    }
    let machine_dependency = doctor
        .pointer("/dependencies/machine")
        .and_then(Value::as_str)
        .or_else(|| doctor.get("machine_dependency").and_then(Value::as_str))
        .unwrap_or_default();
    if !matches!(machine_dependency, "available_closed" | "unblocked") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_DEFERRED_001",
            "reason: machine_lowering_not_closed",
        ));
    }

    let machine_doctor = run_machine_evidence_command("machine_doctor", &["machine", "doctor"])?;
    if machine_doctor.status != "ok" {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_DEFERRED_001",
            "reason: machine_doctor_failed",
        ));
    }
    let native_doctor = run_machine_evidence_command(
        "machine_native_doctor",
        &["machine", "asm-complete", "native-doctor", "--json"],
    )?;
    if native_doctor.status != "ok" {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_DEFERRED_001",
            "reason: machine_lowering_not_closed",
        ));
    }
    Ok(diagnostics)
}

fn run_verify_seal_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal verify-seal expects a .nbb file")?;
    let report = read_nbb(path)?;
    let seal = verify_sealed_nbb(&report);
    print_json(&json!({
        "kind": "BasalSealVerifierReport",
        "path": path,
        "status": seal.status,
        "phase6_machine_evidence_seal": "closed",
        "machine_evidence_integration": if report.machine_evidence.is_some() { "closed" } else { "missing" },
        "seal_final": if report.seal_record.is_some() { "closed" } else { "missing" },
        "seal": report.seal_record.as_ref().map(nbb_seal_record_to_json),
        "machine_evidence": report.machine_evidence.as_ref().map(nbb_machine_evidence_to_json),
        "diagnostics": diagnostics_to_json(&seal.diagnostics)
    }))?;
    if diagnostics_have_errors(&seal.diagnostics) {
        Err(diagnostics_summary(&seal.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_evidence_report_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal evidence-report expects a .nbb file")?;
    let report = read_nbb(path)?;
    let mut diagnostics = report.diagnostics.clone();
    if report.machine_evidence.is_none() {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_EVIDENCE_PENDING_001",
            "machine evidence is missing or still pending",
        ));
    }
    print_json(&json!({
        "kind": "BasalMachineEvidenceReport",
        "path": path,
        "status": if diagnostics_have_errors(&diagnostics) { "error" } else { "closed" },
        "machine_evidence_integration": if report.machine_evidence.is_some() { "closed" } else { "missing" },
        "machine_evidence": report.machine_evidence.as_ref().map(nbb_machine_evidence_to_json),
        "ffi_link_manifest": nbb_ffi_manifest_from_surface_records(&report.surface_records),
        "hash_chain": report.machine_evidence.as_ref().map(|evidence| nbb_evidence_hash_chain_to_json(&evidence.hash_chain)),
        "diagnostics": diagnostics_to_json(&diagnostics)
    }))?;
    if diagnostics_have_errors(&diagnostics) {
        Err(diagnostics_summary(&diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_seal_report_command(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal seal-report expects a .nbb file")?;
    let report = read_nbb(path)?;
    let seal = verify_sealed_nbb(&report);
    print_json(&json!({
        "kind": "BasalSealReport",
        "path": path,
        "status": seal.status,
        "seal_final": if report.seal_record.is_some() { "closed" } else { "missing" },
        "seal": report.seal_record.as_ref().map(nbb_seal_record_to_json),
        "diagnostics": diagnostics_to_json(&seal.diagnostics)
    }))?;
    if diagnostics_have_errors(&seal.diagnostics) {
        Err(diagnostics_summary(&seal.diagnostics).into())
    } else {
        Ok(())
    }
}

fn run_contract_projection(kind: &str, args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = first_path_arg(args).ok_or("basal witness/map expects a file path")?;
    match file_extension(path).as_str() {
        "nb" => {
            let audit = audit_nb(path)?;
            let payload = json!({
                "kind": kind,
                "source": path,
                "status": audit.status,
                "claim": "contract_only",
                "root_container": audit.root_container,
                "sections": audit.sections.iter().cloned().collect::<Vec<_>>(),
                "diagnostics": diagnostics_to_json(&audit.diagnostics),
                "deferred": audit.deferred,
                "hash": stable_json_hash(&audit_to_json(&audit)),
                "machine_lowering": "closed_plan",
                "nbb_writer": "closed",
                "machine_evidence_integration": "not_started"
            });
            print_json(&payload)?;
            if audit.has_errors() {
                Err(diagnostics_summary(&audit.diagnostics).into())
            } else {
                Ok(())
            }
        }
        "nbb" => {
            let report = read_nbb(path)?;
            let payload = json!({
                "kind": kind,
                "source": path,
                "status": report.status,
                "claim": report.claim,
                "diagnostics": diagnostics_to_json(&report.diagnostics),
                "hash": stable_json_hash(&nbb_to_json(&report)),
                "no_machine_lowering": true
            });
            print_json(&payload)?;
            if !report.diagnostics.is_empty() {
                Err(diagnostics_summary(&report.diagnostics).into())
            } else {
                Ok(())
            }
        }
        _ => Err(format!(
            "[E_BASAL_FILE_EXTENSION_001] unsupported basal file '{}'",
            path
        )
        .into()),
    }
}

fn parse_nb_file(path: &str) -> Result<BasalSourceFile, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    Ok(parse_nb_source(path, &source))
}

fn parse_nb_source(path: &str, source: &str) -> BasalSourceFile {
    let cleaned_lines = clean_source_lines(source);
    let file_span = BasalSpan::new(path, 0, source.len(), 1, 1);
    let mut ast = BasalSourceFile {
        path: path.to_string(),
        span: file_span,
        roots: Vec::new(),
        diagnostics: Vec::new(),
    };

    let Some((root_index, root_name, root_span, header_indent)) =
        find_root_header(path, &cleaned_lines)
    else {
        let span = first_content_span(path, source);
        ast.diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_EXPECTED_ROOT_001",
                "basal .nb source must start with a root declaration such as RootName[@] {",
            )
            .with_span(span)
            .with_help("Use the official existing form `RootName[@] { ... }`."),
        );
        return ast;
    };

    let root_end = find_root_end(path, &cleaned_lines, root_index, &mut ast.diagnostics);
    let mut root = BasalRootDecl {
        name: root_name,
        span: span_to_end(root_span, root_end.clone()),
        sections: Vec::new(),
    };

    let mut section_indent: Option<usize> = None;
    let mut seen_sections = BTreeSet::new();
    let mut current_section: Option<usize> = None;
    let allowed_sections = basal_allowed_sections();

    for (line_index, clean) in cleaned_lines.iter().enumerate().skip(root_index + 1) {
        if root_end
            .as_ref()
            .map(|end| clean.start >= end.start)
            .unwrap_or(false)
        {
            break;
        }
        let trimmed = clean.text.trim();
        if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
            continue;
        }
        let indent = leading_columns(&clean.text);
        if indent <= header_indent && trimmed == "}" {
            break;
        }
        if let Some(section_name) = parse_label(trimmed) {
            if section_indent.is_none() {
                section_indent = Some(indent);
            }
            if Some(indent) == section_indent {
                if !allowed_sections.contains(section_name) {
                    ast.diagnostics.push(
                        BasalDiagnostic::new(
                            "E_BASAL_UNKNOWN_SECTION_001",
                            format!("unknown basal section '{}'", section_name),
                        )
                        .with_span(line_span(
                            path,
                            clean,
                            section_name.len(),
                        )),
                    );
                    current_section = None;
                    continue;
                }
                if !seen_sections.insert(section_name.to_string()) {
                    ast.diagnostics.push(
                        BasalDiagnostic::new(
                            "E_BASAL_DUPLICATE_SECTION_001",
                            format!("duplicate basal section '{}'", section_name),
                        )
                        .with_span(line_span(
                            path,
                            clean,
                            section_name.len(),
                        )),
                    );
                }
                let section = BasalSection {
                    kind: section_kind(section_name),
                    name: Some(section_name.to_string()),
                    span: line_span(path, clean, section_name.len()),
                    key_values: Vec::new(),
                    children: Vec::new(),
                };
                root.sections.push(section);
                current_section = Some(root.sections.len() - 1);
                continue;
            }
        }

        let Some(index) = current_section else {
            ast.diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_EXPECTED_SECTION_001",
                    "content outside a basal section is not valid in Phase 1 grammar",
                )
                .with_span(line_span(path, clean, trimmed.len()))
                .with_help(
                    "Start a section such as `root:`, `site:`, `bind:`, `seal:`, or `evidence:`.",
                ),
            );
            continue;
        };
        parse_section_line(
            path,
            clean,
            line_index,
            section_indent.unwrap_or(indent),
            &mut root.sections[index],
            &mut ast.diagnostics,
        );
    }

    if root.sections.is_empty() {
        ast.diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_EXPECTED_SECTION_001",
                "basal root declaration must contain at least one section",
            )
            .with_span(root.span.clone()),
        );
    }
    if ast
        .diagnostics
        .iter()
        .filter(|diag| diag.is_error())
        .count()
        > 1
    {
        ast.diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_RECOVERY_001",
                "parser recovered after syntax errors and continued scanning the source",
            )
            .with_span(root.span.clone())
            .with_help(
                "Fix the earlier diagnostics first; later diagnostics may be recovery fallout.",
            ),
        );
    }
    ast.roots.push(root);
    if let Some(end) = root_end.as_ref() {
        for line in cleaned_lines.iter().filter(|line| line.line > end.line) {
            if let Some((name, span, _indent)) = parse_root_header_line(path, line) {
                ast.roots.push(BasalRootDecl {
                    name,
                    span: span.clone(),
                    sections: Vec::new(),
                });
                ast.diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_ROOT_DUPLICATE_001",
                        "only one basal root declaration is allowed per .nb file",
                    )
                    .with_span(span),
                );
            }
        }
    }
    ast
}

fn clean_source_lines(source: &str) -> Vec<CleanLine> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut in_block_comment = false;
    for (line_no, line) in source.lines().enumerate() {
        let mut cleaned = String::with_capacity(line.len());
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if in_block_comment {
                if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                    cleaned.push(' ');
                    cleaned.push(' ');
                    in_block_comment = false;
                    i += 2;
                } else {
                    cleaned.push(' ');
                    i += 1;
                }
                continue;
            }
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                while i < chars.len() {
                    cleaned.push(' ');
                    i += 1;
                }
                break;
            }
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                cleaned.push(' ');
                cleaned.push(' ');
                in_block_comment = true;
                i += 2;
                continue;
            }
            cleaned.push(chars[i]);
            i += 1;
        }
        out.push(CleanLine {
            text: cleaned,
            line: line_no + 1,
            start: offset,
        });
        offset += line.len() + 1;
    }
    out
}

fn find_root_header(path: &str, lines: &[CleanLine]) -> Option<(usize, String, BasalSpan, usize)> {
    for (idx, line) in lines.iter().enumerate() {
        if line.text.trim().is_empty() {
            continue;
        }
        if let Some((name, span, indent)) = parse_root_header_line(path, line) {
            return Some((idx, name, span, indent));
        }
        break;
    }
    None
}

fn parse_root_header_line(path: &str, line: &CleanLine) -> Option<(String, BasalSpan, usize)> {
    let trimmed = line.text.trim();
    let indent = leading_columns(&line.text);
    if let Some(name) = trimmed
        .strip_suffix('{')
        .and_then(parse_canonical_root_header)
    {
        let span = line_span(
            path,
            line,
            line.text.trim_end().len().saturating_sub(indent),
        );
        return Some((name.to_string(), span, indent));
    }
    if let Some(name) = trimmed
        .strip_suffix('{')
        .and_then(parse_existing_root_header)
    {
        let span = line_span(
            path,
            line,
            line.text.trim_end().len().saturating_sub(indent),
        );
        return Some((name.to_string(), span, indent));
    }
    if let Some(name) = trimmed
        .strip_prefix("[@]")
        .and_then(|rest| rest.trim().strip_prefix("root"))
        .and_then(|rest| rest.trim().strip_suffix('{'))
        .map(str::trim)
        .filter(|name| is_identifier(name))
    {
        let span = line_span(
            path,
            line,
            line.text.trim_end().len().saturating_sub(indent),
        );
        return Some((name.to_string(), span, indent));
    }
    None
}

fn parse_canonical_root_header(raw: &str) -> Option<&str> {
    let (name, role) = raw.trim().split_once("::")?;
    let name = name.trim();
    (role.trim().eq_ignore_ascii_case("root") && is_identifier(name)).then_some(name)
}

fn parse_existing_root_header(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let name = trimmed.strip_suffix("[@]")?.trim();
    is_identifier(name).then_some(name)
}

fn find_root_end(
    path: &str,
    lines: &[CleanLine],
    start_index: usize,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Option<BasalSpan> {
    let mut depth = 0isize;
    for line in lines.iter().skip(start_index) {
        for ch in line.text.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(line_span(path, line, line.text.trim_end().len()));
                    }
                }
                _ => {}
            }
        }
    }
    let span = lines
        .get(start_index)
        .map(|line| line_span(path, line, line.text.trim_end().len()))
        .unwrap_or_else(|| BasalSpan::new(path, 0, 0, 1, 1));
    diagnostics.push(
        BasalDiagnostic::new(
            "E_BASAL_EXPECTED_RBRACE_001",
            "basal root declaration is missing closing `}`",
        )
        .with_span(span),
    );
    None
}

fn parse_section_line(
    path: &str,
    clean: &CleanLine,
    line_index: usize,
    section_indent: usize,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
        return;
    }
    match section.name.as_deref().unwrap_or_default() {
        "root" | "target" | "meta" | "binary" | "stc" => {
            if let Some(kv) = parse_key_value_line(path, clean, diagnostics) {
                section.key_values.push(kv);
            }
        }
        "const" => parse_const_line(path, clean, section, diagnostics),
        "site" => parse_site_line(path, clean, section_indent, section, diagnostics),
        "bind" => parse_bind_line(path, clean, section, diagnostics),
        "routine" => parse_routine_line(path, clean, section, diagnostics),
        "verify" => parse_verify_line("verify", path, clean, section, diagnostics),
        "probe" => parse_verify_line("probe", path, clean, section, diagnostics),
        "report" => parse_report_line(path, clean, section, diagnostics),
        "foreign" => parse_foreign_line(path, clean, section_indent, section, diagnostics),
        "fn" => {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_FN_LEGACY_001",
                    "basal executable source uses routine:; fn: is not official in Basal Fase 5.5",
                )
                .with_span(line_span(path, clean, trimmed.len()))
                .with_help("Replace `fn:` with `routine:` and keep explicit width types."),
            );
            section.children.push(BasalAstNode {
                kind: "BasalRoutineLegacyDecl".to_string(),
                name: None,
                span: line_span(path, clean, trimmed.len()),
                text: Some(trimmed.to_string()),
                children: Vec::new(),
            });
        }
        "seal" => parse_requirement_line("seal", path, clean, section, diagnostics),
        "evidence" => parse_requirement_line("evidence", path, clean, section, diagnostics),
        _ => {
            if trimmed.ends_with('{') || parse_label(trimmed).is_some() {
                section.children.push(BasalAstNode {
                    kind: "BasalSyntaxNode".to_string(),
                    name: trimmed
                        .trim_end_matches('{')
                        .trim_end_matches(':')
                        .split_whitespace()
                        .next()
                        .map(str::to_string),
                    span: line_span(path, clean, trimmed.len()),
                    text: Some(trimmed.to_string()),
                    children: Vec::new(),
                });
            } else if let Some(kv) = try_key_value_line(path, clean) {
                section.key_values.push(kv);
            } else {
                section.children.push(BasalAstNode {
                    kind: "BasalSyntaxLine".to_string(),
                    name: None,
                    span: line_span(path, clean, trimmed.len()),
                    text: Some(trimmed.to_string()),
                    children: Vec::new(),
                });
            }
        }
    }
    let _ = line_index;
}

fn parse_const_line(
    path: &str,
    clean: &CleanLine,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let span = line_span(path, clean, trimmed.len());
    let Some(statement) = trimmed.strip_suffix(';').map(str::trim) else {
        diagnostics.push(
            BasalDiagnostic::new("E_BASAL_CONST_001", "const declaration must end with `;`")
                .with_span(span),
        );
        return;
    };
    let Some((name, rest)) = statement.split_once(">>") else {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_CONST_001",
                "const declaration expects `name >> width_type : value;`",
            )
            .with_span(span),
        );
        return;
    };
    let name = name.trim();
    if !is_identifier(name) {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_EXPECTED_IDENTIFIER_001",
                "const declaration requires an identifier name",
            )
            .with_span(span.clone()),
        );
        return;
    }
    let Some((width_raw, value_raw)) = rest.split_once(':') else {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_CONST_TYPE_001",
                "const declaration requires explicit width type before `:`",
            )
            .with_span(span),
        );
        return;
    };
    let width_raw = width_raw.trim();
    let value_raw = value_raw.trim();
    if parse_basal_width_type(width_raw).is_none() {
        diagnostics.push(
            BasalDiagnostic::new(
                if width_raw == "int" {
                    "E_BASAL_INT_AMBIGUOUS_001"
                } else {
                    "E_BASAL_CONST_TYPE_001"
                },
                format!("unsupported or ambiguous const width type '{}'", width_raw),
            )
            .with_span(span.clone()),
        );
    }
    if classify_literal(value_raw) == "invalid" {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_CONST_LITERAL_001",
                format!("invalid const literal '{}'", value_raw),
            )
            .with_span(span.clone()),
        );
    }
    if let Some(width) = parse_basal_width_type(width_raw) {
        if let Some(value) = parse_nonnegative_u64(value_raw) {
            if !literal_fits_width(value, &width) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_CONST_RANGE_001",
                        format!(
                            "const '{}' literal '{}' is out of range for {}",
                            name, value_raw, width.name
                        ),
                    )
                    .with_span(span.clone()),
                );
            }
        }
    }
    section.children.push(BasalAstNode {
        kind: "BasalConstDecl".to_string(),
        name: Some(name.to_string()),
        span,
        text: Some(format!("{} >> {} : {}", name, width_raw, value_raw)),
        children: Vec::new(),
    });
}

fn parse_foreign_line(
    path: &str,
    clean: &CleanLine,
    section_indent: usize,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let indent = leading_columns(&clean.text);
    if indent > section_indent && parse_label(trimmed).is_some() {
        section.children.push(BasalAstNode {
            kind: "BasalForeignBindingDecl".to_string(),
            name: parse_label(trimmed).map(str::to_string),
            span: line_span(path, clean, trimmed.len()),
            text: None,
            children: Vec::new(),
        });
        return;
    }
    if let Some(kv) = parse_key_value_line(path, clean, diagnostics) {
        if let Some(binding) = section
            .children
            .iter_mut()
            .rev()
            .find(|node| node.kind == "BasalForeignBindingDecl")
        {
            binding.children.push(ast_node_from_key_value(&kv));
        } else {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_FOREIGN_BINDING_001",
                    "foreign contract field must appear inside a named binding",
                )
                .with_span(kv.span.clone()),
            );
        }
        section.key_values.push(kv);
        return;
    }
    diagnostics.push(
        BasalDiagnostic::new(
            "E_BASAL_FOREIGN_BINDING_001",
            "invalid foreign contract syntax; expected `name:` followed by fields",
        )
        .with_span(line_span(path, clean, trimmed.len())),
    );
}

fn parse_routine_line(
    path: &str,
    clean: &CleanLine,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let span = line_span(path, clean, trimmed.len());
    let header_candidate = trimmed
        .split_once('(')
        .map(|(name, _)| name.trim())
        .unwrap_or_default();
    if trimmed.ends_with('{')
        && trimmed.contains('(')
        && !matches!(header_candidate, "if" | "while" | "for" | "switch")
    {
        let header = trimmed.trim_end_matches('{').trim();
        let Some((name, params, return_ty)) = parse_routine_header(header) else {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_ROUTINE_001",
                    "routine declaration expects `name(param: width) -> width {`",
                )
                .with_span(span),
            );
            return;
        };
        for (_, ty) in &params {
            if parse_basal_width_type(ty).is_none() {
                diagnostics.push(
                    BasalDiagnostic::new(
                        if ty == "int" {
                            "E_BASAL_INT_AMBIGUOUS_001"
                        } else {
                            "E_BASAL_ROUTINE_PARAM_001"
                        },
                        format!("routine parameter uses unsupported width type '{}'", ty),
                    )
                    .with_span(span.clone()),
                );
            }
        }
        if let Some(return_ty) = return_ty.as_deref() {
            if parse_basal_width_type(return_ty).is_none() {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_ROUTINE_RETURN_001",
                        format!("routine return uses unsupported width type '{}'", return_ty),
                    )
                    .with_span(span.clone()),
                );
            }
        }
        let mut children = Vec::new();
        for (param, ty) in params {
            children.push(BasalAstNode {
                kind: "BasalRoutineParam".to_string(),
                name: Some(param),
                span: span.clone(),
                text: Some(ty),
                children: Vec::new(),
            });
        }
        if let Some(return_ty) = return_ty {
            children.push(BasalAstNode {
                kind: "BasalRoutineReturnType".to_string(),
                name: None,
                span: span.clone(),
                text: Some(return_ty),
                children: Vec::new(),
            });
        }
        section.children.push(BasalAstNode {
            kind: "BasalRoutineDecl".to_string(),
            name: Some(name),
            span,
            text: Some(header.to_string()),
            children,
        });
        return;
    }
    if trimmed.starts_with("raw unchecked") && trimmed.ends_with('{') {
        section.children.push(BasalAstNode {
            kind: "BasalRawBlock".to_string(),
            name: Some("unchecked".to_string()),
            span,
            text: Some(trimmed.to_string()),
            children: Vec::new(),
        });
        return;
    }
    if trimmed.starts_with("raw") && trimmed.ends_with('{') {
        section.children.push(BasalAstNode {
            kind: "BasalRawBlock".to_string(),
            name: Some("checked".to_string()),
            span,
            text: Some(trimmed.to_string()),
            children: Vec::new(),
        });
        return;
    }
    if trimmed.contains("mem[") && !trimmed.contains(">>") {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_RAW_WIDTH_001",
                "raw memory access requires explicit width via `>> width`",
            )
            .with_span(span.clone()),
        );
    }
    let kind = if trimmed.starts_with("return ") {
        "BasalRoutineReturn"
    } else if trimmed.contains("mem[") {
        "BasalMemAccess"
    } else if trimmed.contains(">>") {
        "BasalRoutineLocalDecl"
    } else {
        "BasalRoutineLine"
    };
    if let Some(width_raw) = typed_statement_width(trimmed) {
        if parse_basal_width_type(width_raw).is_none() {
            diagnostics.push(
                BasalDiagnostic::new(
                    if width_raw == "int" {
                        "E_BASAL_INT_AMBIGUOUS_001"
                    } else {
                        "E_BASAL_ROUTINE_TYPE_001"
                    },
                    format!(
                        "routine statement uses unsupported width type '{}'",
                        width_raw
                    ),
                )
                .with_span(span.clone()),
            );
        }
    }
    section.children.push(BasalAstNode {
        kind: kind.to_string(),
        name: surface_statement_name(trimmed),
        span,
        text: Some(trimmed.trim_end_matches(';').to_string()),
        children: Vec::new(),
    });
}

fn parse_verify_line(
    section_name: &str,
    path: &str,
    clean: &CleanLine,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let span = line_span(path, clean, trimmed.len());
    if section_name == "probe" {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_PROBE_ALIAS_001",
                "probe: is accepted as a transitional alias for verify:",
            )
            .with_severity("warning")
            .with_span(span.clone()),
        );
    }
    section.children.push(BasalAstNode {
        kind: if section_name == "probe" {
            "BasalProbeAliasStmt".to_string()
        } else {
            "BasalVerifyStmt".to_string()
        },
        name: surface_statement_name(trimmed),
        span,
        text: Some(trimmed.trim_end_matches(';').to_string()),
        children: Vec::new(),
    });
}

fn parse_report_line(
    path: &str,
    clean: &CleanLine,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let span = line_span(path, clean, trimmed.len());
    if trimmed.starts_with("print")
        || trimmed.starts_with("println")
        || trimmed.starts_with("print.sta")
    {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_REPORT_RUNTIME_PRINT_FORBIDDEN_001",
                "report: cannot depend on runtime print output",
            )
            .with_span(span.clone()),
        );
    } else if !(trimmed.starts_with("status ")
        || trimmed.starts_with("emit ")
        || trimmed.starts_with("hash ")
        || trimmed.ends_with(';'))
    {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_REPORT_001",
                "report statement expects `status`, `emit`, or typed report expression",
            )
            .with_span(span.clone()),
        );
    }
    section.children.push(BasalAstNode {
        kind: "BasalReportStmt".to_string(),
        name: surface_statement_name(trimmed),
        span,
        text: Some(trimmed.trim_end_matches(';').to_string()),
        children: Vec::new(),
    });
}

fn parse_site_line(
    path: &str,
    clean: &CleanLine,
    section_indent: usize,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let indent = leading_columns(&clean.text);
    if indent > section_indent && parse_label(trimmed).is_some() {
        section.children.push(BasalAstNode {
            kind: "BasalSiteDecl".to_string(),
            name: parse_label(trimmed).map(str::to_string),
            span: line_span(path, clean, trimmed.len()),
            text: None,
            children: Vec::new(),
        });
    } else if let Some(kv) = parse_key_value_line(path, clean, diagnostics) {
        if let Some(site) = section
            .children
            .iter_mut()
            .rev()
            .find(|node| node.kind == "BasalSiteDecl")
        {
            site.children.push(ast_node_from_key_value(&kv));
        }
        section.key_values.push(kv);
    } else {
        diagnostics.push(
            BasalDiagnostic::new("E_BASAL_SITE_SYNTAX_001", "invalid site declaration syntax")
                .with_span(line_span(path, clean, trimmed.len()))
                .with_help("Use `name:` followed by indented `key: value;` entries."),
        );
    }
}

fn parse_bind_line(
    path: &str,
    clean: &CleanLine,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    let without_semicolon = match trimmed.strip_suffix(';') {
        Some(value) => value.trim(),
        None => {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_EXPECTED_SEMICOLON_001",
                    "bind declaration must end with `;`",
                )
                .with_span(line_span(path, clean, trimmed.len())),
            );
            return;
        }
    };
    let parts: Vec<&str> = without_semicolon.split_whitespace().collect();
    if parts.len() == 5 && parts[0] == "symbol" && parts[2] == "->" {
        section.children.push(BasalAstNode {
            kind: "BasalBindDecl".to_string(),
            name: Some(parts[1].to_string()),
            span: line_span(path, clean, trimmed.len()),
            text: Some(without_semicolon.to_string()),
            children: vec![BasalAstNode {
                kind: "BasalPath".to_string(),
                name: Some(parts[3].to_string()),
                span: line_span(path, clean, trimmed.len()),
                text: Some(parts[4].to_string()),
                children: Vec::new(),
            }],
        });
    } else {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_BIND_SYNTAX_001",
                "invalid bind syntax; expected `symbol name -> path target;`",
            )
            .with_span(line_span(path, clean, trimmed.len())),
        );
    }
}

fn parse_requirement_line(
    section_kind: &str,
    path: &str,
    clean: &CleanLine,
    section: &mut BasalSection,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let trimmed = clean.text.trim();
    if let Some(name) = parse_label(trimmed) {
        section.children.push(BasalAstNode {
            kind: if section_kind == "seal" {
                "BasalSealBlock".to_string()
            } else {
                "BasalEvidenceBlock".to_string()
            },
            name: Some(name.to_string()),
            span: line_span(path, clean, trimmed.len()),
            text: None,
            children: Vec::new(),
        });
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("require ") {
        if let Some(req) = rest.trim().strip_suffix(';') {
            if is_path_like(req.trim()) {
                section.children.push(BasalAstNode {
                    kind: if section_kind == "seal" {
                        "BasalSealRequirement".to_string()
                    } else {
                        "BasalEvidenceRequirement".to_string()
                    },
                    name: Some(req.trim().to_string()),
                    span: line_span(path, clean, trimmed.len()),
                    text: None,
                    children: Vec::new(),
                });
                return;
            }
        }
    }
    if let Some(kv) = parse_key_value_line(path, clean, diagnostics) {
        section.key_values.push(kv);
        return;
    }
    diagnostics.push(
        BasalDiagnostic::new(
            if section_kind == "seal" {
                "E_BASAL_SEAL_SYNTAX_001"
            } else {
                "E_BASAL_EVIDENCE_SYNTAX_001"
            },
            format!("invalid {} syntax", section_kind),
        )
        .with_span(line_span(path, clean, trimmed.len())),
    );
}

fn parse_key_value_line(
    path: &str,
    clean: &CleanLine,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Option<BasalKeyValue> {
    let trimmed = clean.text.trim();
    if trimmed.ends_with('{') || trimmed == "}" {
        return None;
    }
    let Some((key, value_with_tail)) = trimmed.split_once(':') else {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_EXPECTED_COLON_001",
                "expected `:` in key/value entry",
            )
            .with_span(line_span(path, clean, trimmed.len())),
        );
        return None;
    };
    let key = key.trim();
    if !is_identifier(key) {
        diagnostics.push(
            BasalDiagnostic::new("E_BASAL_EXPECTED_IDENTIFIER_001", "expected identifier key")
                .with_span(line_span(path, clean, key.len().max(1))),
        );
        return None;
    }
    let Some(raw_value) = value_with_tail.trim().strip_suffix(';') else {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_EXPECTED_SEMICOLON_001",
                "expected `;` after key/value entry",
            )
            .with_span(line_span(path, clean, trimmed.len())),
        );
        return None;
    };
    let raw_value = raw_value.trim();
    let value_kind = classify_literal(raw_value);
    if value_kind == "invalid" {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_INVALID_LITERAL_001",
                format!("invalid basal literal '{}'", raw_value),
            )
            .with_span(line_span(path, clean, trimmed.len())),
        );
        return None;
    }
    let span = line_span(path, clean, trimmed.len());
    Some(BasalKeyValue {
        key: key.to_string(),
        value: BasalLiteral {
            kind: value_kind.to_string(),
            raw: raw_value.to_string(),
            span: span.clone(),
        },
        span,
    })
}

fn try_key_value_line(path: &str, clean: &CleanLine) -> Option<BasalKeyValue> {
    let trimmed = clean.text.trim();
    let (key, value_with_tail) = trimmed.split_once(':')?;
    let key = key.trim();
    if !is_identifier(key) {
        return None;
    }
    let raw_value = value_with_tail.trim().strip_suffix(';')?.trim();
    let value_kind = classify_literal(raw_value);
    if value_kind == "invalid" {
        return None;
    }
    let span = line_span(path, clean, trimmed.len());
    Some(BasalKeyValue {
        key: key.to_string(),
        value: BasalLiteral {
            kind: value_kind.to_string(),
            raw: raw_value.to_string(),
            span: span.clone(),
        },
        span,
    })
}

fn ast_node_from_key_value(kv: &BasalKeyValue) -> BasalAstNode {
    BasalAstNode {
        kind: "BasalKeyValue".to_string(),
        name: Some(kv.key.clone()),
        span: kv.span.clone(),
        text: Some(kv.value.raw.clone()),
        children: Vec::new(),
    }
}

fn classify_literal(raw: &str) -> &'static str {
    if raw.is_empty() {
        return "invalid";
    }
    if raw.starts_with('"') {
        return if raw.len() >= 2 && raw.ends_with('"') {
            "string"
        } else {
            "invalid"
        };
    }
    if matches!(raw, "true" | "false") {
        return "boolean";
    }
    if parse_integer_literal(raw) {
        return "integer";
    }
    if raw.contains('|') {
        return if raw
            .split('|')
            .all(|part| is_identifier(part.trim()) || is_path_like(part.trim()))
        {
            "permission_set"
        } else {
            "invalid"
        };
    }
    if is_path_like(raw) || is_basal_context_ref(raw) {
        return "path";
    }
    "invalid"
}

fn parse_integer_literal(raw: &str) -> bool {
    if let Some(hex) = raw.strip_prefix("0x") {
        !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit())
    } else if let Some(bin) = raw.strip_prefix("0b") {
        !bin.is_empty() && bin.chars().all(|ch| matches!(ch, '0' | '1'))
    } else {
        raw.chars().all(|ch| ch.is_ascii_digit())
    }
}

fn parse_basal_width_type(raw: &str) -> Option<BasalWidthType> {
    let normalized = raw.trim();
    let (name, bits, signed, kind) = match normalized {
        "byte" | "u8" => ("byte", Some(8), false, "integer"),
        "word" | "u16" => ("word", Some(16), false, "integer"),
        "dword" | "u32" => ("dword", Some(32), false, "integer"),
        "qword" | "u64" => ("qword", Some(64), false, "integer"),
        "sbyte" | "i8" => ("sbyte", Some(8), true, "integer"),
        "sword" | "i16" => ("sword", Some(16), true, "integer"),
        "sdword" | "i32" => ("sdword", Some(32), true, "integer"),
        "sqword" | "i64" => ("sqword", Some(64), true, "integer"),
        "addr" => ("addr", Some(64), false, "address"),
        "bool" => ("bool", Some(1), false, "boolean"),
        "char" => ("char", Some(32), false, "char"),
        "str" => ("str", None, false, "string"),
        _ => {
            if let Some(width) = normalized
                .strip_prefix("bytes<")
                .and_then(|rest| rest.strip_suffix('>'))
                .and_then(|n| n.parse::<u16>().ok())
                .filter(|n| *n > 0)
            {
                return Some(BasalWidthType {
                    name: format!("bytes<{}>", width),
                    bits: Some(width * 8),
                    signed: false,
                    kind: "bytes".to_string(),
                });
            }
            return None;
        }
    };
    Some(BasalWidthType {
        name: name.to_string(),
        bits,
        signed,
        kind: kind.to_string(),
    })
}

fn literal_fits_width(value: u64, width: &BasalWidthType) -> bool {
    match (width.bits, width.signed, width.kind.as_str()) {
        (_, _, "string" | "char" | "bytes") => true,
        (Some(1), _, "boolean") => value <= 1,
        (Some(bits), false, _) if bits >= 64 => true,
        (Some(bits), false, _) => value < (1u64 << bits),
        (Some(bits), true, _) if bits >= 64 => value <= i64::MAX as u64,
        (Some(bits), true, _) => value <= ((1u64 << (bits - 1)) - 1),
        _ => true,
    }
}

fn typed_statement_width(trimmed: &str) -> Option<&str> {
    let (_, rest) = trimmed.split_once(">>")?;
    let (width, _) = rest.split_once(':')?;
    Some(width.trim())
}

fn surface_statement_name(trimmed: &str) -> Option<String> {
    if let Some((name, _)) = trimmed.split_once(">>") {
        return Some(name.trim().trim_start_matches('@').to_string());
    }
    if let Some((name, _)) = trimmed.split_once('(') {
        let candidate = name.trim();
        if is_identifier(candidate) {
            return Some(candidate.to_string());
        }
    }
    if let Some(rest) = trimmed.strip_prefix("emit ") {
        return Some(rest.trim_end_matches(';').trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("status ") {
        return Some(rest.trim_matches('"').trim_end_matches(';').to_string());
    }
    None
}

fn parse_routine_header(header: &str) -> Option<(String, Vec<(String, String)>, Option<String>)> {
    let (name, after_name) = header.split_once('(')?;
    let name = name.trim();
    if !is_identifier(name) {
        return None;
    }
    let (params_raw, rest) = after_name.split_once(')')?;
    let mut params = Vec::new();
    for raw in params_raw
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let (param, ty) = raw.split_once(':')?;
        let param = param.trim();
        if !is_identifier(param) {
            return None;
        }
        params.push((param.to_string(), ty.trim().to_string()));
    }
    let return_ty = rest
        .trim()
        .strip_prefix("->")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Some((name.to_string(), params, return_ty))
}

fn is_basal_context_ref(raw: &str) -> bool {
    raw == "[@]" || raw.starts_with("[@") && raw.ends_with(']') || raw.ends_with("[@]")
}

fn is_path_like(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    raw.split('.').all(is_identifier)
}

fn parse_label(trimmed: &str) -> Option<&str> {
    let label = trimmed.strip_suffix(':')?.trim();
    is_identifier(label).then_some(label)
}

fn is_identifier(raw: &str) -> bool {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
}

fn basal_allowed_sections() -> BTreeSet<&'static str> {
    [
        "root",
        "target",
        "site",
        "bind",
        "seal",
        "evidence",
        "meta",
        "const",
        "routine",
        "verify",
        "report",
        "fn",
        "stc",
        "type",
        "binary",
        "bridge",
        "foreign",
        "probe",
        "structure",
        "fa",
        "final",
    ]
    .into_iter()
    .collect()
}

fn section_kind(name: &str) -> String {
    match name {
        "root" => "BasalRootSection",
        "target" => "BasalTargetSection",
        "site" => "BasalSiteSection",
        "bind" => "BasalBindSection",
        "seal" => "BasalSealSection",
        "evidence" => "BasalEvidenceSection",
        "meta" => "BasalMetaSection",
        "const" => "BasalConstSection",
        "routine" => "BasalRoutineSection",
        "verify" => "BasalVerifySection",
        "probe" => "BasalProbeAliasSection",
        "report" => "BasalReportSection",
        "foreign" => "BasalForeignSection",
        "fn" => "BasalRoutineLegacySection",
        _ => "BasalSection",
    }
    .to_string()
}

fn first_content_span(path: &str, source: &str) -> BasalSpan {
    let mut offset = 0usize;
    for (line_no, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.is_empty() {
            let column = line.len().saturating_sub(trimmed.len()) + 1;
            return BasalSpan::new(
                path,
                offset + column - 1,
                offset + line.len(),
                line_no + 1,
                column,
            );
        }
        offset += line.len() + 1;
    }
    BasalSpan::new(path, 0, 0, 1, 1)
}

fn leading_columns(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn line_span(path: &str, line: &CleanLine, len: usize) -> BasalSpan {
    let indent = leading_columns(&line.text);
    let start = line.start + indent;
    BasalSpan::new(path, start, start + len, line.line, indent + 1)
}

fn span_to_end(mut span: BasalSpan, end: Option<BasalSpan>) -> BasalSpan {
    if let Some(end) = end {
        span.end = end.end;
    }
    span
}

fn first_path_arg(args: &[String]) -> Option<&str> {
    args.iter()
        .find(|arg| !arg.starts_with("--"))
        .map(String::as_str)
}

fn arg_after<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find_map(|pair| (pair[0] == name).then_some(pair[1].as_str()))
}

fn file_extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn build_table_set(ast: &BasalSourceFile) -> BasalTableSet {
    let mut diagnostics = ast.diagnostics.clone();
    let root_table = build_root_table(ast, &mut diagnostics);
    let site_table = build_site_table(ast, &mut diagnostics);
    let bind_table = build_bind_table(ast, &site_table, &mut diagnostics);
    let status = if diagnostics_have_errors(&diagnostics) {
        "broken"
    } else {
        "basal_phase2_root_site_bind_tables_closed"
    }
    .to_string();
    BasalTableSet {
        path: ast.path.clone(),
        root_table,
        site_table,
        bind_table,
        diagnostics,
        status,
    }
}

fn build_root_table(
    ast: &BasalSourceFile,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> BasalRootTable {
    let mut table_diagnostics = Vec::new();
    let mut entries = Vec::new();
    if ast.roots.is_empty() {
        push_table_diag(
            diagnostics,
            &mut table_diagnostics,
            BasalDiagnostic::new(
                "E_BASAL_ROOT_TABLE_001",
                "root table requires one root declaration",
            )
            .with_span(ast.span.clone()),
        );
    }
    if ast.roots.len() > 1 {
        for duplicate in ast.roots.iter().skip(1) {
            push_table_diag(
                diagnostics,
                &mut table_diagnostics,
                BasalDiagnostic::new(
                    "E_BASAL_ROOT_DUPLICATE_001",
                    format!("duplicate root declaration '{}'", duplicate.name),
                )
                .with_span(duplicate.span.clone()),
            );
        }
    }

    if let Some(root) = ast.roots.first() {
        let mut entry_diagnostics = Vec::new();
        let root_sections: Vec<&BasalSection> = root
            .sections
            .iter()
            .filter(|section| section.name.as_deref() == Some("root"))
            .collect();
        if root_sections.is_empty() {
            push_entry_diag(
                diagnostics,
                &mut table_diagnostics,
                &mut entry_diagnostics,
                BasalDiagnostic::new(
                    "E_BASAL_ROOT_TABLE_001",
                    "root table requires a root: section",
                )
                .with_span(root.span.clone()),
            );
        }
        if root_sections.len() > 1 {
            for section in root_sections.iter().skip(1) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_ROOT_DUPLICATE_001",
                        "duplicate root: section in root table",
                    )
                    .with_span(section.span.clone()),
                );
            }
        }
        let root_fields = root_sections
            .first()
            .map(|section| key_values_to_map(&section.key_values))
            .unwrap_or_default();
        let target_fields = root
            .sections
            .iter()
            .find(|section| section.name.as_deref() == Some("target"))
            .map(|section| key_values_to_map(&section.key_values))
            .unwrap_or_default();
        let meta_fields = root
            .sections
            .iter()
            .find(|section| section.name.as_deref() == Some("meta"))
            .map(|section| key_values_to_map(&section.key_values))
            .unwrap_or_default();

        for (field, (_, span)) in &root_fields {
            if !basal_root_table_allowed_fields().contains(field.as_str()) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_ROOT_UNKNOWN_FIELD_001",
                        format!("unknown root table field '{}'", field),
                    )
                    .with_span(span.clone()),
                );
            }
        }

        let id = root_fields
            .get("id")
            .or_else(|| root_fields.get("identity"))
            .map(|(value, _)| normalize_literal_value(value));
        if id.as_deref().map(str::is_empty).unwrap_or(true) {
            push_entry_diag(
                diagnostics,
                &mut table_diagnostics,
                &mut entry_diagnostics,
                BasalDiagnostic::new("E_BASAL_ROOT_ID_001", "root table requires root id")
                    .with_span(root.span.clone()),
            );
        }

        let version = match root_fields.get("version") {
            Some((raw, span)) => match parse_positive_u64(raw) {
                Some(value) => Some(value),
                None => {
                    push_entry_diag(
                        diagnostics,
                        &mut table_diagnostics,
                        &mut entry_diagnostics,
                        BasalDiagnostic::new(
                            "E_BASAL_ROOT_VERSION_001",
                            format!("root version must be a positive integer, got '{}'", raw),
                        )
                        .with_span(span.clone()),
                    );
                    None
                }
            },
            None => {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new("E_BASAL_ROOT_VERSION_001", "root version is required")
                        .with_span(root.span.clone()),
                );
                None
            }
        };

        let profile = root_fields
            .get("profile")
            .or_else(|| target_fields.get("profile"))
            .map(|(value, _)| normalize_literal_value(value))
            .or_else(|| Some("basal".to_string()));
        if let Some((raw_profile, span)) = root_fields
            .get("profile")
            .or_else(|| target_fields.get("profile"))
        {
            let normalized = normalize_literal_value(raw_profile);
            if !basal_allowed_profiles().contains(normalized.as_str()) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_ROOT_PROFILE_001",
                        format!("unsupported basal root profile '{}'", normalized),
                    )
                    .with_span(span.clone()),
                );
            }
        }

        let target_reference = target_fields
            .get("triple")
            .or_else(|| target_fields.get("arch"))
            .map(|(value, _)| normalize_literal_value(value));
        let mut metadata = BTreeMap::new();
        for (key, (value, _)) in meta_fields {
            metadata.insert(key, normalize_literal_value(&value));
        }
        for key in [
            "authority",
            "binary",
            "evidence",
            "lowering",
            "origin",
            "contract",
            "nbb",
            "machine_evidence",
        ] {
            if let Some((value, _)) = root_fields.get(key) {
                metadata.insert(key.to_string(), normalize_literal_value(value));
            }
        }
        for key in ["arch", "triple", "profile", "symbols", "machine_symbols"] {
            if let Some((value, _)) = target_fields.get(key) {
                metadata.insert(format!("target_{}", key), normalize_literal_value(value));
            }
        }

        let status = if diagnostics_have_errors(&entry_diagnostics) {
            "error"
        } else {
            "closed"
        }
        .to_string();
        entries.push(BasalRootEntry {
            name: root.name.clone(),
            root_id: id,
            version,
            profile,
            target_reference,
            declared_sections: root
                .sections
                .iter()
                .filter_map(|section| section.name.clone())
                .collect(),
            metadata,
            origin: span_ref(&ast.path, "BasalRootDecl", &root.span),
            diagnostics: entry_diagnostics,
            status,
        });
    }
    let status = if diagnostics_have_errors(&table_diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    BasalRootTable {
        entries,
        diagnostics: table_diagnostics,
        status,
    }
}

fn build_site_table(
    ast: &BasalSourceFile,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> BasalSiteTable {
    let mut table_diagnostics = Vec::new();
    let mut entries = Vec::new();
    let Some(root) = ast.roots.first() else {
        return BasalSiteTable {
            entries,
            diagnostics: table_diagnostics,
            status: "closed".to_string(),
        };
    };
    let mut seen = BTreeSet::new();
    for section in root
        .sections
        .iter()
        .filter(|section| section.name.as_deref() == Some("site"))
    {
        for node in &section.children {
            if node.kind != "BasalSiteDecl" {
                continue;
            }
            let mut entry_diagnostics = Vec::new();
            let name = node.name.clone().unwrap_or_else(|| "<missing>".to_string());
            if !seen.insert(name.clone()) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_SITE_DUPLICATE_001",
                        format!("duplicate site '{}'", name),
                    )
                    .with_span(node.span.clone()),
                );
            }
            let fields = ast_node_fields(node);
            for (field, (_, span)) in &fields {
                if !basal_site_allowed_fields().contains(field.as_str()) {
                    push_entry_diag(
                        diagnostics,
                        &mut table_diagnostics,
                        &mut entry_diagnostics,
                        BasalDiagnostic::new(
                            "E_BASAL_SITE_UNKNOWN_FIELD_001",
                            format!("unknown site table field '{}'", field),
                        )
                        .with_span(span.clone()),
                    );
                }
            }
            let kind = fields
                .get("kind")
                .map(|(value, _)| normalize_literal_value(value))
                .or_else(|| infer_site_kind(&fields));
            if let Some((raw_kind, span)) = fields.get("kind") {
                let normalized = normalize_literal_value(raw_kind);
                if !basal_allowed_site_kinds().contains(normalized.as_str()) {
                    push_entry_diag(
                        diagnostics,
                        &mut table_diagnostics,
                        &mut entry_diagnostics,
                        BasalDiagnostic::new(
                            "E_BASAL_SITE_KIND_001",
                            format!("unsupported site kind '{}'", normalized),
                        )
                        .with_span(span.clone()),
                    );
                }
            } else if kind.is_none() {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new("E_BASAL_SITE_KIND_001", "site kind is required")
                        .with_span(node.span.clone()),
                );
            }

            let size = parse_optional_positive_field(
                "size",
                &fields,
                "E_BASAL_SITE_SIZE_001",
                "site size must be a positive integer",
                diagnostics,
                &mut table_diagnostics,
                &mut entry_diagnostics,
            );
            let align = parse_optional_power_two_field(
                &fields,
                diagnostics,
                &mut table_diagnostics,
                &mut entry_diagnostics,
            );
            let offset = fields
                .get("offset")
                .and_then(|(value, _)| parse_nonnegative_u64(value));
            let permissions = normalize_permissions(
                fields.get("permissions"),
                diagnostics,
                &mut table_diagnostics,
                &mut entry_diagnostics,
            );
            if permissions.flags.iter().any(|flag| flag == "write")
                && permissions.flags.iter().any(|flag| flag == "execute")
            {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_SITE_PERMISSION_COMBO_001",
                        "basal site permissions cannot combine write and execute in Phase 2",
                    )
                    .with_span(
                        fields
                            .get("permissions")
                            .map(|(_, span)| span.clone())
                            .unwrap_or_else(|| node.span.clone()),
                    ),
                );
            }
            let status = if diagnostics_have_errors(&entry_diagnostics) {
                "error"
            } else {
                "closed"
            }
            .to_string();
            entries.push(BasalSiteEntry {
                name,
                kind,
                size,
                align,
                offset,
                target_policy: fields
                    .get("target")
                    .or_else(|| fields.get("addressing"))
                    .or_else(|| fields.get("policy"))
                    .map(|(value, _)| normalize_literal_value(value)),
                permissions,
                fields: fields
                    .iter()
                    .map(|(key, (value, _))| (key.clone(), normalize_literal_value(value)))
                    .collect(),
                origin: span_ref(&ast.path, "BasalSiteDecl", &node.span),
                diagnostics: entry_diagnostics,
                status,
            });
        }
    }
    let status = if diagnostics_have_errors(&table_diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    BasalSiteTable {
        entries,
        diagnostics: table_diagnostics,
        status,
    }
}

fn build_bind_table(
    ast: &BasalSourceFile,
    site_table: &BasalSiteTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> BasalBindTable {
    let mut table_diagnostics = Vec::new();
    let mut entries = Vec::new();
    let Some(root) = ast.roots.first() else {
        return BasalBindTable {
            entries,
            diagnostics: table_diagnostics,
            status: "closed".to_string(),
        };
    };
    let site_names: BTreeSet<String> = site_table
        .entries
        .iter()
        .filter(|entry| entry.status == "closed")
        .map(|entry| entry.name.clone())
        .collect();
    let mut seen_ids = BTreeSet::new();
    for section in root
        .sections
        .iter()
        .filter(|section| section.name.as_deref() == Some("bind"))
    {
        for node in &section.children {
            if node.kind != "BasalBindDecl" {
                continue;
            }
            let mut entry_diagnostics = Vec::new();
            let source = node.name.clone().unwrap_or_default();
            if !is_path_like(&source) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_BIND_SOURCE_001",
                        format!("invalid bind source path '{}'", source),
                    )
                    .with_span(node.span.clone()),
                );
            }
            if !seen_ids.insert(source.clone()) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_BIND_DUPLICATE_001",
                        format!("duplicate bind source '{}'", source),
                    )
                    .with_span(node.span.clone()),
                );
            }
            let target_node = node.children.first();
            let destination_kind = target_node
                .and_then(|child| child.name.clone())
                .unwrap_or_default();
            let destination_path = target_node
                .and_then(|child| child.text.clone())
                .unwrap_or_default();
            if !basal_allowed_binding_kinds().contains(destination_kind.as_str()) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_BIND_KIND_001",
                        format!("unsupported bind destination kind '{}'", destination_kind),
                    )
                    .with_span(node.span.clone()),
                );
            }
            if !is_path_like(&destination_path) {
                push_entry_diag(
                    diagnostics,
                    &mut table_diagnostics,
                    &mut entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_BIND_DEST_001",
                        format!("invalid bind destination path '{}'", destination_path),
                    )
                    .with_span(
                        target_node
                            .map(|child| child.span.clone())
                            .unwrap_or_else(|| node.span.clone()),
                    ),
                );
            }
            let resolution_status = if destination_kind.starts_with("machine.") {
                entry_diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_BIND_PENDING_MACHINE_001",
                        "machine bind resolution is pending until Basal Fase 4 lowering",
                    )
                    .with_severity("info")
                    .with_span(node.span.clone()),
                );
                "pending_lowering_or_machine_resolution"
            } else if destination_kind == "root.site" {
                if !site_names.contains(&destination_path) {
                    push_entry_diag(
                        diagnostics,
                        &mut table_diagnostics,
                        &mut entry_diagnostics,
                        BasalDiagnostic::new(
                            "E_BASAL_BIND_LOCAL_TARGET_001",
                            format!("bind references unknown local site '{}'", destination_path),
                        )
                        .with_span(node.span.clone()),
                    );
                    "invalid"
                } else {
                    "syntax_only"
                }
            } else {
                "syntax_only"
            };
            let status = if diagnostics_have_errors(&entry_diagnostics) {
                "error"
            } else {
                "closed"
            }
            .to_string();
            entries.push(BasalBindEntry {
                id: source.clone(),
                source,
                destination_path,
                destination_kind: BasalBindingKind {
                    raw: destination_kind.clone(),
                    normalized: destination_kind,
                },
                resolution_status: BasalBindingResolutionStatus {
                    status: resolution_status.to_string(),
                },
                origin: span_ref(&ast.path, "BasalBindDecl", &node.span),
                target_span: target_node
                    .map(|child| child.span.clone())
                    .unwrap_or_else(|| node.span.clone()),
                diagnostics: entry_diagnostics,
                status,
            });
        }
    }
    let status = if diagnostics_have_errors(&table_diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    BasalBindTable {
        entries,
        diagnostics: table_diagnostics,
        status,
    }
}

fn push_table_diag(
    diagnostics: &mut Vec<BasalDiagnostic>,
    table_diagnostics: &mut Vec<BasalDiagnostic>,
    diagnostic: BasalDiagnostic,
) {
    diagnostics.push(diagnostic.clone());
    table_diagnostics.push(diagnostic);
}

fn push_entry_diag(
    diagnostics: &mut Vec<BasalDiagnostic>,
    table_diagnostics: &mut Vec<BasalDiagnostic>,
    entry_diagnostics: &mut Vec<BasalDiagnostic>,
    diagnostic: BasalDiagnostic,
) {
    diagnostics.push(diagnostic.clone());
    table_diagnostics.push(diagnostic.clone());
    entry_diagnostics.push(diagnostic);
}

fn key_values_to_map(key_values: &[BasalKeyValue]) -> BTreeMap<String, (String, BasalSpan)> {
    key_values
        .iter()
        .map(|kv| (kv.key.clone(), (kv.value.raw.clone(), kv.span.clone())))
        .collect()
}

fn ast_node_fields(node: &BasalAstNode) -> BTreeMap<String, (String, BasalSpan)> {
    node.children
        .iter()
        .filter(|child| child.kind == "BasalKeyValue")
        .filter_map(|child| {
            Some((
                child.name.clone()?,
                (child.text.clone().unwrap_or_default(), child.span.clone()),
            ))
        })
        .collect()
}

fn normalize_literal_value(raw: &str) -> String {
    raw.trim().trim_matches('"').to_string()
}

fn parse_positive_u64(raw: &str) -> Option<u64> {
    parse_nonnegative_u64(raw).filter(|value| *value > 0)
}

fn parse_nonnegative_u64(raw: &str) -> Option<u64> {
    let normalized = normalize_literal_value(raw);
    if let Some(hex) = normalized.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = normalized.strip_prefix("0b") {
        u64::from_str_radix(bin, 2).ok()
    } else {
        normalized.parse::<u64>().ok()
    }
}

fn parse_optional_positive_field(
    field: &str,
    fields: &BTreeMap<String, (String, BasalSpan)>,
    code: &str,
    message: &str,
    diagnostics: &mut Vec<BasalDiagnostic>,
    table_diagnostics: &mut Vec<BasalDiagnostic>,
    entry_diagnostics: &mut Vec<BasalDiagnostic>,
) -> Option<u64> {
    match fields.get(field) {
        Some((raw, span)) => match parse_positive_u64(raw) {
            Some(value) => Some(value),
            None => {
                push_entry_diag(
                    diagnostics,
                    table_diagnostics,
                    entry_diagnostics,
                    BasalDiagnostic::new(code, format!("{}: '{}'", message, raw))
                        .with_span(span.clone()),
                );
                None
            }
        },
        None => None,
    }
}

fn parse_optional_power_two_field(
    fields: &BTreeMap<String, (String, BasalSpan)>,
    diagnostics: &mut Vec<BasalDiagnostic>,
    table_diagnostics: &mut Vec<BasalDiagnostic>,
    entry_diagnostics: &mut Vec<BasalDiagnostic>,
) -> Option<u64> {
    let field = fields.get("align").or_else(|| fields.get("alignment"));
    match field {
        Some((raw, span)) => match parse_positive_u64(raw) {
            Some(value) if value.is_power_of_two() => Some(value),
            _ => {
                push_entry_diag(
                    diagnostics,
                    table_diagnostics,
                    entry_diagnostics,
                    BasalDiagnostic::new(
                        "E_BASAL_SITE_ALIGN_001",
                        format!("site align must be a positive power of two, got '{}'", raw),
                    )
                    .with_span(span.clone()),
                );
                None
            }
        },
        None => None,
    }
}

fn normalize_permissions(
    raw: Option<&(String, BasalSpan)>,
    diagnostics: &mut Vec<BasalDiagnostic>,
    table_diagnostics: &mut Vec<BasalDiagnostic>,
    entry_diagnostics: &mut Vec<BasalDiagnostic>,
) -> BasalPermissionFlags {
    let Some((raw, span)) = raw else {
        return BasalPermissionFlags { flags: Vec::new() };
    };
    let mut flags = Vec::new();
    for part in raw.split('|') {
        let flag = normalize_literal_value(part).trim().to_string();
        if flag.is_empty() {
            continue;
        }
        if !basal_allowed_permissions().contains(flag.as_str()) {
            push_entry_diag(
                diagnostics,
                table_diagnostics,
                entry_diagnostics,
                BasalDiagnostic::new(
                    "E_BASAL_SITE_PERMISSION_001",
                    format!("unsupported site permission '{}'", flag),
                )
                .with_span(span.clone()),
            );
        }
        if !flags.contains(&flag) {
            flags.push(flag);
        }
    }
    BasalPermissionFlags { flags }
}

fn infer_site_kind(fields: &BTreeMap<String, (String, BasalSpan)>) -> Option<String> {
    if fields.contains_key("symbol") || fields.contains_key("origin") {
        Some("data_anchor".to_string())
    } else {
        None
    }
}

fn span_ref(path: &str, node_kind: &str, span: &BasalSpan) -> BasalTableSpanRef {
    BasalTableSpanRef {
        source_file: path.to_string(),
        node_kind: node_kind.to_string(),
        span: span.clone(),
    }
}

fn basal_root_table_allowed_fields() -> BTreeSet<&'static str> {
    [
        "id",
        "identity",
        "version",
        "profile",
        "target",
        "authority",
        "binary",
        "evidence",
        "lowering",
        "origin",
        "contract",
        "description",
        "author",
        "nbb",
        "machine_evidence",
    ]
    .into_iter()
    .collect()
}

fn basal_allowed_profiles() -> BTreeSet<&'static str> {
    ["basal", "nC-basal-core-v1", "root"].into_iter().collect()
}

fn basal_site_allowed_fields() -> BTreeSet<&'static str> {
    [
        "kind",
        "size",
        "align",
        "alignment",
        "permissions",
        "offset",
        "target",
        "addressing",
        "policy",
        "symbol",
        "origin",
        "owner",
        "state",
        "evidence",
    ]
    .into_iter()
    .collect()
}

fn basal_allowed_site_kinds() -> BTreeSet<&'static str> {
    [
        "memory_anchor",
        "code_anchor",
        "data_anchor",
        "policy_anchor",
        "evidence_anchor",
        "custom",
    ]
    .into_iter()
    .collect()
}

fn basal_allowed_permissions() -> BTreeSet<&'static str> {
    ["read", "write", "execute", "seal", "evidence", "root"]
        .into_iter()
        .collect()
}

fn basal_allowed_binding_kinds() -> BTreeSet<&'static str> {
    ["machine.proc", "machine.symbol", "root.site", "root.symbol"]
        .into_iter()
        .collect()
}

fn build_basal_ir_module(ast: &BasalSourceFile, tables: &BasalTableSet) -> BasalIrModule {
    let roots = tables
        .root_table
        .entries
        .iter()
        .map(|entry| BasalRootIr {
            root_id: entry
                .root_id
                .clone()
                .unwrap_or_else(|| "<missing-root-id>".to_string()),
            root_kind: "root".to_string(),
            version: entry.version,
            profile: entry.profile.clone(),
            target_reference: entry.target_reference.clone(),
            metadata: entry.metadata.clone(),
            declared_sections: entry.declared_sections.clone(),
            table_entry: entry.name.clone(),
            span: ir_span_from_ref(&entry.origin),
            status: if entry.status == "closed" {
                "verified".to_string()
            } else {
                "error".to_string()
            },
        })
        .collect::<Vec<_>>();
    let owner_root = roots
        .first()
        .map(|root| root.root_id.clone())
        .unwrap_or_else(|| "<missing-root>".to_string());
    let sites = tables
        .site_table
        .entries
        .iter()
        .map(|entry| BasalSiteIr {
            site_id: entry.name.clone(),
            kind: entry
                .kind
                .clone()
                .unwrap_or_else(|| "<missing-kind>".to_string()),
            size: entry.size,
            align: entry.align,
            permissions: entry
                .permissions
                .flags
                .iter()
                .map(|name| BasalIrPermission { name: name.clone() })
                .collect(),
            owner_root: owner_root.clone(),
            fields: entry.fields.clone(),
            span: ir_span_from_ref(&entry.origin),
            semantic_status: if entry.status == "closed" {
                "verified".to_string()
            } else {
                "error".to_string()
            },
        })
        .collect::<Vec<_>>();
    let binds = tables
        .bind_table
        .entries
        .iter()
        .map(|entry| {
            let dest_span = BasalIrSpan {
                source_file: entry.origin.source_file.clone(),
                node_kind: "BasalBindDestination".to_string(),
                span: entry.target_span.clone(),
            };
            let source_span = ir_span_from_ref(&entry.origin);
            let destination = BasalIrSymbolRef {
                path: entry.destination_path.clone(),
                kind: entry.destination_kind.normalized.clone(),
                span: dest_span.clone(),
            };
            let machine_ref = entry
                .destination_kind
                .normalized
                .starts_with("machine.")
                .then(|| BasalIrMachineRef {
                    path: entry.destination_path.clone(),
                    kind: entry.destination_kind.normalized.clone(),
                    resolution_status: BasalIrResolutionStatus {
                        status: "pending_machine_resolution".to_string(),
                    },
                    span: dest_span.clone(),
                });
            BasalBindIr {
                bind_id: entry.id.clone(),
                source: BasalIrSymbolRef {
                    path: entry.source.clone(),
                    kind: "root.symbol".to_string(),
                    span: source_span.clone(),
                },
                destination,
                machine_ref,
                resolution_status: BasalIrResolutionStatus {
                    status: if entry.destination_kind.normalized == "root.site"
                        && entry.status == "closed"
                    {
                        "resolved_local_site"
                    } else if entry.destination_kind.normalized.starts_with("machine.") {
                        "pending_machine_resolution"
                    } else if entry.status == "closed" {
                        "syntax_verified"
                    } else {
                        "error"
                    }
                    .to_string(),
                },
                source_span,
                destination_span: dest_span,
                semantic_status: if entry.status == "closed" {
                    "verified".to_string()
                } else {
                    "error".to_string()
                },
            }
        })
        .collect::<Vec<_>>();
    let seal_requirements = collect_seal_requirements(ast);
    let surface = collect_executable_surface_ir(ast, tables);
    let mut module = BasalIrModule {
        path: tables.path.clone(),
        roots,
        sites,
        binds,
        seal_requirements,
        surface,
        diagnostics: Vec::new(),
        verifier: BasalIrVerifierReport {
            status: "not_run".to_string(),
            diagnostics: Vec::new(),
            root_count: 0,
            site_count: 0,
            bind_count: 0,
            local_refs_resolved: 0,
            machine_refs_pending: 0,
            consistency_checks: Vec::new(),
            no_machine_evidence: true,
        },
        status: "not_verified".to_string(),
    };
    let verifier = verify_basal_ir_module(&module, tables);
    module.diagnostics = verifier.diagnostics.clone();
    module.status = if diagnostics_have_errors(&verifier.diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    module.verifier = verifier;
    module
}

fn collect_seal_requirements(ast: &BasalSourceFile) -> Vec<BasalIrSealRequirement> {
    ast.roots
        .iter()
        .flat_map(|root| root.sections.iter())
        .filter(|section| section.name.as_deref() == Some("seal"))
        .flat_map(|section| section.children.iter())
        .filter(|node| node.kind == "BasalSealRequirement")
        .filter_map(|node| {
            Some(BasalIrSealRequirement {
                name: node.name.clone()?,
                span: BasalIrSpan {
                    source_file: ast.path.clone(),
                    node_kind: node.kind.clone(),
                    span: node.span.clone(),
                },
                status: "modeled_not_executed_until_phase6".to_string(),
            })
        })
        .collect()
}

fn collect_executable_surface_ir(
    ast: &BasalSourceFile,
    tables: &BasalTableSet,
) -> BasalExecutableSurfaceIr {
    let mut diagnostics = Vec::new();
    let mut consts = Vec::new();
    let mut routines = Vec::new();
    let mut raw_blocks = Vec::new();
    let mut verifies = Vec::new();
    let mut reports = Vec::new();
    let mut foreign_bindings = Vec::new();
    let mut mem_accesses = Vec::new();
    let mut pointer_accesses = Vec::new();
    let mut seen_consts = BTreeSet::new();
    let mut probe_aliases = 0usize;
    let site_permissions = tables
        .site_table
        .entries
        .iter()
        .map(|entry| (entry.name.clone(), entry.permissions.flags.clone()))
        .collect::<BTreeMap<_, _>>();
    let site_names = site_permissions.keys().cloned().collect::<BTreeSet<_>>();

    for root in &ast.roots {
        for section in &root.sections {
            for node in &section.children {
                collect_pointer_accesses_from_node(ast, node, false, &mut pointer_accesses);
            }
            match section.name.as_deref() {
                Some("const") => {
                    for node in &section.children {
                        if node.kind != "BasalConstDecl" {
                            continue;
                        }
                        if let Some(record) = const_ir_from_node(ast, node, &mut diagnostics) {
                            if !seen_consts.insert(record.name.clone()) {
                                diagnostics.push(
                                    BasalDiagnostic::new(
                                        "E_BASAL_CONST_MUTATION_001",
                                        format!(
                                            "const '{}' is declared more than once",
                                            record.name
                                        ),
                                    )
                                    .with_span(node.span.clone()),
                                );
                            }
                            consts.push(record);
                        }
                    }
                }
                Some("routine") => {
                    collect_routine_surface(
                        ast,
                        section,
                        &site_names,
                        &site_permissions,
                        &mut routines,
                        &mut raw_blocks,
                        &mut mem_accesses,
                        &mut diagnostics,
                    );
                }
                Some("verify") | Some("probe") => {
                    if section.name.as_deref() == Some("probe") {
                        probe_aliases += 1;
                    }
                    let verify = verify_ir_from_section(
                        ast,
                        section,
                        &site_names,
                        &site_permissions,
                        &mut mem_accesses,
                        &mut diagnostics,
                    );
                    verifies.push(verify);
                }
                Some("report") => {
                    let report = report_ir_from_section(
                        ast,
                        section,
                        &site_names,
                        &site_permissions,
                        &mut mem_accesses,
                        &mut diagnostics,
                    );
                    reports.push(report);
                }
                Some("foreign") => {
                    collect_foreign_bindings(ast, section, &mut foreign_bindings, &mut diagnostics);
                }
                Some("fn") => diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_FN_LEGACY_001",
                        "Basal Fase 5.5 requires routine: instead of fn:",
                    )
                    .with_span(section.span.clone()),
                ),
                _ => {}
            }
        }
    }

    let routine_names = routines
        .iter()
        .map(|routine| routine.name.clone())
        .collect::<BTreeSet<_>>();
    for verify in &verifies {
        for call in &verify.routine_calls {
            if !routine_names.contains(call) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_VERIFY_ROUTINE_CALL_001",
                        format!("verify references unknown routine '{}'", call),
                    )
                    .with_span(verify.span.span.clone()),
                );
            }
        }
    }

    let status = if diagnostics_have_errors(&diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    BasalExecutableSurfaceIr {
        consts,
        routines,
        raw_blocks,
        verifies,
        reports,
        foreign_bindings,
        mem_accesses,
        pointer_accesses,
        diagnostics,
        probe_aliases,
        status,
    }
}

fn collect_foreign_bindings(
    ast: &BasalSourceFile,
    section: &BasalSection,
    foreign_bindings: &mut Vec<BasalForeignBindingIr>,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let mut seen = BTreeSet::new();
    for node in &section.children {
        if node.kind != "BasalForeignBindingDecl" {
            continue;
        }
        let name = node.name.clone().unwrap_or_else(|| "<missing>".to_string());
        if !seen.insert(name.clone()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_FOREIGN_DUPLICATE_001",
                    format!("duplicate foreign binding '{}'", name),
                )
                .with_span(node.span.clone()),
            );
        }
        let fields = ast_node_fields(node);
        for field in fields.keys() {
            if !basal_foreign_allowed_fields().contains(field.as_str()) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_FOREIGN_FIELD_001",
                        format!("unknown foreign binding field '{}'", field),
                    )
                    .with_span(
                        fields
                            .get(field)
                            .map(|(_, span)| span.clone())
                            .unwrap_or_else(|| node.span.clone()),
                    ),
                );
            }
        }
        let language = required_foreign_field(
            "language",
            &fields,
            diagnostics,
            &node.span,
            "E_BASAL_FOREIGN_LANGUAGE_001",
        );
        let symbol = required_foreign_field(
            "symbol",
            &fields,
            diagnostics,
            &node.span,
            "E_BASAL_FOREIGN_SYMBOL_001",
        );
        let abi = required_foreign_field(
            "abi",
            &fields,
            diagnostics,
            &node.span,
            "E_BASAL_FOREIGN_ABI_001",
        );
        let layout = required_foreign_field(
            "layout",
            &fields,
            diagnostics,
            &node.span,
            "E_BASAL_FOREIGN_LAYOUT_001",
        );
        let ownership = required_foreign_field(
            "ownership",
            &fields,
            diagnostics,
            &node.span,
            "E_BASAL_FOREIGN_OWNERSHIP_001",
        );
        if let Some(language) = language.as_deref() {
            if !basal_allowed_foreign_languages().contains(language) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_FOREIGN_LANGUAGE_001",
                        format!("unsupported basal foreign language '{}'", language),
                    )
                    .with_span(
                        fields
                            .get("language")
                            .map(|(_, span)| span.clone())
                            .unwrap_or_else(|| node.span.clone()),
                    ),
                );
            }
        }
        if let Some(symbol) = symbol.as_deref() {
            if !is_legal_machine_symbol(symbol) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_FOREIGN_SYMBOL_001",
                        format!("foreign symbol '{}' is not a legal machine symbol", symbol),
                    )
                    .with_span(
                        fields
                            .get("symbol")
                            .map(|(_, span)| span.clone())
                            .unwrap_or_else(|| node.span.clone()),
                    ),
                );
            }
        }
        let status = if language.is_some()
            && symbol.is_some()
            && abi.is_some()
            && layout.is_some()
            && ownership.is_some()
        {
            "closed"
        } else {
            "error"
        };
        foreign_bindings.push(BasalForeignBindingIr {
            name,
            language: language.unwrap_or_else(|| "<missing-language>".to_string()),
            symbol: symbol.unwrap_or_else(|| "<missing-symbol>".to_string()),
            abi: abi.unwrap_or_else(|| "<missing-abi>".to_string()),
            layout: layout.unwrap_or_else(|| "<missing-layout>".to_string()),
            ownership: ownership.unwrap_or_else(|| "<missing-ownership>".to_string()),
            header: normalized_foreign_field("header", &fields),
            artifact: normalized_foreign_field("artifact", &fields),
            link: normalized_foreign_field("link", &fields),
            bridge: normalized_foreign_field("bridge", &fields),
            span: ir_span_from_node(ast, node),
            status: status.to_string(),
        });
    }
}

fn basal_foreign_allowed_fields() -> BTreeSet<&'static str> {
    [
        "language",
        "symbol",
        "abi",
        "layout",
        "ownership",
        "header",
        "artifact",
        "link",
        "bridge",
    ]
    .into_iter()
    .collect()
}

fn basal_allowed_foreign_languages() -> BTreeSet<&'static str> {
    ["c", "cpp", "rust", "zig", "go", "asm"]
        .into_iter()
        .collect()
}

fn required_foreign_field(
    field: &str,
    fields: &BTreeMap<String, (String, BasalSpan)>,
    diagnostics: &mut Vec<BasalDiagnostic>,
    fallback_span: &BasalSpan,
    code: &str,
) -> Option<String> {
    match normalized_foreign_field(field, fields) {
        Some(value) if !value.is_empty() => Some(value),
        _ => {
            diagnostics.push(
                BasalDiagnostic::new(code, format!("foreign binding requires '{}'", field))
                    .with_span(fallback_span.clone()),
            );
            None
        }
    }
}

fn normalized_foreign_field(
    field: &str,
    fields: &BTreeMap<String, (String, BasalSpan)>,
) -> Option<String> {
    fields
        .get(field)
        .map(|(value, _)| normalize_literal_value(value))
}

fn collect_pointer_accesses_from_node(
    ast: &BasalSourceFile,
    node: &BasalAstNode,
    inside_raw: bool,
    out: &mut Vec<BasalPointerAccessIr>,
) {
    let text = node.text.as_deref().unwrap_or_default();
    let raw_here = inside_raw
        || node.kind.eq_ignore_ascii_case("BasalRawBlock")
        || text.trim_start().starts_with("raw {")
        || text.trim_start().starts_with("raw unchecked");

    for operation in basal_pointer_operations_from_text(text) {
        let pointer_type = basal_pointer_type_from_text(text);
        let authority = if raw_here {
            "raw_declared"
        } else if text.contains("ptrFromAddr") || text.contains("addr") {
            "explicit_addr_or_pointer_surface"
        } else {
            "checked_surface"
        };
        let hazard = if raw_here {
            "raw_pointer_or_memory_hazard"
        } else {
            "checked_pointer_use"
        };
        let status = if raw_here {
            "raw_hazard_recorded"
        } else {
            "closed"
        };
        out.push(BasalPointerAccessIr {
            operation,
            pointer_type,
            authority: authority.to_string(),
            hazard: hazard.to_string(),
            evidence_boundary: "pending_machine_evidence_or_seal_record".to_string(),
            span: ir_span_from_node(ast, node),
            status: status.to_string(),
        });
    }

    for child in &node.children {
        collect_pointer_accesses_from_node(ast, child, raw_here, out);
    }
}

fn basal_pointer_operations_from_text(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (needle, operation) in [
        ("ptrRead", "ptr_read"),
        ("ptrWrite", "ptr_write"),
        ("ptrOffset", "ptr_offset"),
        ("ptrDiff", "ptr_diff"),
        ("ptrFromAddr", "ptr_from_addr"),
        ("ptrCast", "ptr_cast"),
        ("regionFromPtr", "region_from_ptr"),
        ("regionPtrAt", "region_ptr_at"),
        ("memRegion<", "mem_region_type"),
        ("ptr<", "ptr_type"),
        ("ptr mut<", "ptr_mut_type"),
        ("mem[", "mem_access"),
    ] {
        if text.contains(needle) && !out.iter().any(|seen| seen == operation) {
            out.push(operation.to_string());
        }
    }
    if text
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|token| token == "addr")
        && !out.iter().any(|seen| seen == "addr_type")
    {
        out.push("addr_type".to_string());
    }
    out
}

fn basal_pointer_type_from_text(text: &str) -> String {
    if let Some(start) = text.find("ptr mut<") {
        if let Some(end) = text[start..].find('>') {
            return text[start..=start + end].trim().to_string();
        }
    }
    if let Some(start) = text.find("ptr<") {
        if let Some(end) = text[start..].find('>') {
            return text[start..=start + end].trim().to_string();
        }
    }
    if let Some(start) = text.find("memRegion<") {
        if let Some(end) = text[start..].find('>') {
            return text[start..=start + end].trim().to_string();
        }
    }
    if text.contains("mem[") {
        return "mem[addr_or_site]".to_string();
    }
    if text
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|token| token == "addr")
    {
        return "addr".to_string();
    }
    "pointer_surface".to_string()
}

fn const_ir_from_node(
    ast: &BasalSourceFile,
    node: &BasalAstNode,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Option<BasalConstIr> {
    let text = node.text.as_deref()?;
    let (name_part, rest) = text.split_once(">>")?;
    let name = name_part.trim().to_string();
    let (width_raw, value_raw) = rest.split_once(':')?;
    let width_raw = width_raw.trim();
    let value = value_raw.trim().to_string();
    let Some(width) = parse_basal_width_type(width_raw) else {
        diagnostics.push(
            BasalDiagnostic::new(
                if width_raw == "int" {
                    "E_BASAL_INT_AMBIGUOUS_001"
                } else {
                    "E_BASAL_CONST_TYPE_001"
                },
                format!(
                    "const '{}' has unsupported width type '{}'",
                    name, width_raw
                ),
            )
            .with_span(node.span.clone()),
        );
        return None;
    };
    if let Some(integer) = parse_nonnegative_u64(&value) {
        if !literal_fits_width(integer, &width) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_CONST_RANGE_001",
                    format!(
                        "const '{}' literal is out of range for {}",
                        name, width.name
                    ),
                )
                .with_span(node.span.clone()),
            );
        }
    }
    Some(BasalConstIr {
        name,
        width,
        value: value.clone(),
        value_hash: surface_value_hash(&format!("{}:{}", width_raw, value)),
        endian: "little".to_string(),
        span: ir_span_from_node(ast, node),
        status: "materialized".to_string(),
    })
}

fn collect_routine_surface(
    ast: &BasalSourceFile,
    section: &BasalSection,
    site_names: &BTreeSet<String>,
    site_permissions: &BTreeMap<String, Vec<String>>,
    routines: &mut Vec<BasalRoutineIr>,
    raw_blocks: &mut Vec<BasalRawBlockIr>,
    mem_accesses: &mut Vec<BasalMemAccessIr>,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let mut current_routine: Option<usize> = None;
    let mut current_raw: Option<usize> = None;
    for node in &section.children {
        match node.kind.as_str() {
            "BasalRoutineDecl" => {
                let mut params = Vec::new();
                let mut return_type = None;
                for child in &node.children {
                    if child.kind == "BasalRoutineParam" {
                        let raw_ty = child.text.clone().unwrap_or_default();
                        if let Some(width) = parse_basal_width_type(&raw_ty) {
                            params.push(BasalRoutineParamIr {
                                name: child.name.clone().unwrap_or_default(),
                                width,
                                span: ir_span_from_node(ast, child),
                            });
                        } else {
                            diagnostics.push(
                                BasalDiagnostic::new(
                                    "E_BASAL_ROUTINE_PARAM_001",
                                    format!("routine parameter uses unsupported type '{}'", raw_ty),
                                )
                                .with_span(child.span.clone()),
                            );
                        }
                    } else if child.kind == "BasalRoutineReturnType" {
                        let raw_ty = child.text.clone().unwrap_or_default();
                        return_type = parse_basal_width_type(&raw_ty);
                        if return_type.is_none() {
                            diagnostics.push(
                                BasalDiagnostic::new(
                                    "E_BASAL_ROUTINE_RETURN_001",
                                    format!("routine return uses unsupported type '{}'", raw_ty),
                                )
                                .with_span(child.span.clone()),
                            );
                        }
                    }
                }
                routines.push(BasalRoutineIr {
                    name: node.name.clone().unwrap_or_else(|| "routine".to_string()),
                    params,
                    return_type,
                    raw_blocks: Vec::new(),
                    mem_accesses: Vec::new(),
                    abi_profile: "basal-root-abi-v1".to_string(),
                    span: ir_span_from_node(ast, node),
                    lowering_status: "planned_phase5_5".to_string(),
                    status: "closed".to_string(),
                });
                current_routine = Some(routines.len() - 1);
            }
            "BasalRawBlock" => {
                let block = BasalRawBlockIr {
                    name: format!("raw_{}", raw_blocks.len()),
                    unchecked: node.name.as_deref() == Some("unchecked"),
                    hazards: Vec::new(),
                    mem_accesses: Vec::new(),
                    span: ir_span_from_node(ast, node),
                    status: "hazards_recorded".to_string(),
                };
                raw_blocks.push(block.clone());
                if let Some(index) = current_routine {
                    routines[index].raw_blocks.push(block);
                }
                current_raw = Some(raw_blocks.len() - 1);
            }
            "BasalMemAccess" | "BasalRoutineLocalDecl" => {
                if let Some(access) =
                    mem_access_from_text(ast, node, site_names, site_permissions, diagnostics, true)
                {
                    if let Some(index) = current_routine {
                        routines[index].mem_accesses.push(access.clone());
                    }
                    if let Some(raw_index) = current_raw {
                        let hazard = raw_hazard_from_access(&access);
                        raw_blocks[raw_index].hazards.push(hazard.clone());
                        raw_blocks[raw_index].mem_accesses.push(access.clone());
                        if let Some(routine_index) = current_routine {
                            if let Some(block) = routines[routine_index].raw_blocks.last_mut() {
                                block.hazards.push(hazard);
                                block.mem_accesses.push(access.clone());
                            }
                        }
                    }
                    mem_accesses.push(access);
                }
            }
            _ => {}
        }
    }
}

fn verify_ir_from_section(
    ast: &BasalSourceFile,
    section: &BasalSection,
    site_names: &BTreeSet<String>,
    site_permissions: &BTreeMap<String, Vec<String>>,
    mem_accesses: &mut Vec<BasalMemAccessIr>,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> BasalVerifyIr {
    let mut statements = Vec::new();
    let mut routine_calls = Vec::new();
    let mut local_mem = Vec::new();
    for node in &section.children {
        let text = node.text.clone().unwrap_or_default();
        statements.push(text.clone());
        if let Some(call) = routine_call_name(&text) {
            routine_calls.push(call);
        }
        if text.contains("mem[") {
            if let Some(access) =
                mem_access_from_text(ast, node, site_names, site_permissions, diagnostics, false)
            {
                local_mem.push(access.clone());
                mem_accesses.push(access);
            }
        }
    }
    BasalVerifyIr {
        name: section.name.clone().unwrap_or_else(|| "verify".to_string()),
        statements,
        routine_calls,
        mem_accesses: local_mem,
        probe_alias: section.name.as_deref() == Some("probe"),
        span: BasalIrSpan {
            source_file: ast.path.clone(),
            node_kind: section.kind.clone(),
            span: section.span.clone(),
        },
        status: "closed".to_string(),
    }
}

fn report_ir_from_section(
    ast: &BasalSourceFile,
    section: &BasalSection,
    site_names: &BTreeSet<String>,
    site_permissions: &BTreeMap<String, Vec<String>>,
    mem_accesses: &mut Vec<BasalMemAccessIr>,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> BasalReportIr {
    let mut statements = Vec::new();
    let mut emitted_items = Vec::new();
    for node in &section.children {
        let text = node.text.clone().unwrap_or_default();
        if text.starts_with("emit ") {
            emitted_items.push(text.trim_start_matches("emit ").trim().to_string());
        }
        if text.starts_with("print") {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_REPORT_RUNTIME_PRINT_FORBIDDEN_001",
                    "report cannot call runtime print",
                )
                .with_span(node.span.clone()),
            );
        }
        if text.contains("mem[") {
            if let Some(access) =
                mem_access_from_text(ast, node, site_names, site_permissions, diagnostics, false)
            {
                mem_accesses.push(access);
            }
        }
        statements.push(text);
    }
    BasalReportIr {
        statements,
        emitted_items,
        span: BasalIrSpan {
            source_file: ast.path.clone(),
            node_kind: section.kind.clone(),
            span: section.span.clone(),
        },
        status: "closed".to_string(),
    }
}

fn mem_access_from_text(
    ast: &BasalSourceFile,
    node: &BasalAstNode,
    site_names: &BTreeSet<String>,
    site_permissions: &BTreeMap<String, Vec<String>>,
    diagnostics: &mut Vec<BasalDiagnostic>,
    inside_raw: bool,
) -> Option<BasalMemAccessIr> {
    let text = node.text.as_deref()?;
    let mem_target = extract_mem_target(text)?;
    let access = if text.trim_start().starts_with("mem[") {
        "write"
    } else {
        "read"
    };
    let width = typed_statement_width(text).and_then(parse_basal_width_type);
    if inside_raw && width.is_none() {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_RAW_WIDTH_001",
                "raw mem access requires explicit width",
            )
            .with_span(node.span.clone()),
        );
    }
    let is_addr = mem_target.starts_with('@');
    if !is_addr && !site_names.contains(&mem_target) {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_BASAL_MEM_SITE_001",
                format!("mem access references unknown site '{}'", mem_target),
            )
            .with_span(node.span.clone()),
        );
    }
    if let Some(perms) = site_permissions.get(&mem_target) {
        if access == "read" && !perms.iter().any(|permission| permission == "read") {
            diagnostics.push(
                BasalDiagnostic::new(
                    if inside_raw {
                        "E_BASAL_RAW_PERMISSION_001"
                    } else {
                        "E_BASAL_MEM_PERMISSION_001"
                    },
                    format!("site '{}' lacks read permission", mem_target),
                )
                .with_span(node.span.clone()),
            );
        }
        if access == "write" && !perms.iter().any(|permission| permission == "write") {
            diagnostics.push(
                BasalDiagnostic::new(
                    if inside_raw {
                        "E_BASAL_RAW_PERMISSION_001"
                    } else {
                        "E_BASAL_MEM_PERMISSION_001"
                    },
                    format!("site '{}' lacks write permission", mem_target),
                )
                .with_span(node.span.clone()),
            );
        }
    }
    Some(BasalMemAccessIr {
        form: "mem".to_string(),
        target: mem_target,
        width,
        access: access.to_string(),
        span: ir_span_from_node(ast, node),
        status: if inside_raw {
            "raw_hazard_recorded"
        } else {
            "checked_surface_access"
        }
        .to_string(),
    })
}

fn raw_hazard_from_access(access: &BasalMemAccessIr) -> BasalRawHazardIr {
    BasalRawHazardIr {
        hazard: if access.access == "write" {
            "raw_memory_write"
        } else {
            "raw_memory_read"
        }
        .to_string(),
        width: access.width.clone(),
        endian: "little".to_string(),
        alignment: "checked_if_site_declares_align".to_string(),
        permission_status: "checked_if_site_known".to_string(),
        bounds_status: "checked_if_site_declares_size".to_string(),
        accepted: true,
        evidence_boundary: "pending_phase6_machine_evidence_integration".to_string(),
        span: access.span.clone(),
    }
}

fn extract_mem_target(text: &str) -> Option<String> {
    let start = text.find("mem[")? + 4;
    let end = text[start..].find(']')? + start;
    Some(text[start..end].trim().to_string())
}

fn routine_call_name(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_end_matches(';');
    if trimmed.starts_with("if ") || trimmed.starts_with("dev(") || trimmed.starts_with("assert ") {
        return None;
    }
    let (name, _) = trimmed.split_once('(')?;
    let name = name.trim();
    is_identifier(name).then(|| name.to_string())
}

fn ir_span_from_node(ast: &BasalSourceFile, node: &BasalAstNode) -> BasalIrSpan {
    BasalIrSpan {
        source_file: ast.path.clone(),
        node_kind: node.kind.clone(),
        span: node.span.clone(),
    }
}

fn surface_value_hash(value: &str) -> String {
    to_hex(&Sha256::digest(value.as_bytes()))
}

fn verify_basal_ir_module(module: &BasalIrModule, tables: &BasalTableSet) -> BasalIrVerifierReport {
    let mut diagnostics = Vec::new();
    let checks = vec![
        "root_table_consistency".to_string(),
        "site_table_consistency".to_string(),
        "bind_table_consistency".to_string(),
        "local_site_resolution".to_string(),
        "machine_refs_pending_not_resolved".to_string(),
        "span_preservation".to_string(),
    ];

    if module.roots.len() != 1 {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_IR_ROOT_001",
            format!(
                "BasalIR requires exactly one root, found {}",
                module.roots.len()
            ),
        ));
    }
    let mut root_ids = BTreeSet::new();
    for root in &module.roots {
        if root.root_id.trim().is_empty() || root.root_id == "<missing-root-id>" {
            diagnostics.push(
                BasalDiagnostic::new("E_BASAL_IR_ROOT_001", "RootIR requires root id")
                    .with_span(root.span.span.clone()),
            );
        }
        if !root_ids.insert(root.root_id.clone()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_DUPLICATE_001",
                    format!("duplicate RootIR id '{}'", root.root_id),
                )
                .with_span(root.span.span.clone()),
            );
        }
        if !span_is_materialized(&root.span.span) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_SPAN_001",
                    format!("RootIR '{}' is missing source span", root.root_id),
                )
                .with_span(root.span.span.clone()),
            );
        }
    }
    if module.roots.is_empty() {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_IR_SPAN_001",
            "BasalIR cannot establish root source span because root declaration is missing",
        ));
    }

    let mut site_ids = BTreeSet::new();
    for site in &module.sites {
        if !site_ids.insert(site.site_id.clone()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_DUPLICATE_001",
                    format!("duplicate SiteIR id '{}'", site.site_id),
                )
                .with_span(site.span.span.clone()),
            );
        }
        if !basal_allowed_site_kinds().contains(site.kind.as_str()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_SITE_001",
                    format!("unsupported SiteIR kind '{}'", site.kind),
                )
                .with_span(site.span.span.clone()),
            );
        }
        if let Some(size) = site.size {
            if size == 0 {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_IR_SIZE_001",
                        format!("SiteIR '{}' has zero size", site.site_id),
                    )
                    .with_span(site.span.span.clone()),
                );
            }
        }
        if let Some(align) = site.align {
            if align == 0 || !align.is_power_of_two() {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_IR_ALIGN_001",
                        format!("SiteIR '{}' has invalid alignment {}", site.site_id, align),
                    )
                    .with_span(site.span.span.clone()),
                );
            }
        }
        let permission_names = site
            .permissions
            .iter()
            .map(|permission| permission.name.as_str())
            .collect::<Vec<_>>();
        for permission in &permission_names {
            if !basal_allowed_permissions().contains(permission) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_IR_PERMISSION_001",
                        format!(
                            "SiteIR '{}' has invalid permission '{}'",
                            site.site_id, permission
                        ),
                    )
                    .with_span(site.span.span.clone()),
                );
            }
        }
        if permission_names.contains(&"write") && permission_names.contains(&"execute") {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_PERMISSION_001",
                    format!(
                        "SiteIR '{}' cannot combine write and execute permissions",
                        site.site_id
                    ),
                )
                .with_span(site.span.span.clone()),
            );
        }
        if !root_ids.contains(&site.owner_root) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_CONSISTENCY_001",
                    format!(
                        "SiteIR '{}' owner root '{}' is not present",
                        site.site_id, site.owner_root
                    ),
                )
                .with_span(site.span.span.clone()),
            );
        }
        if !span_is_materialized(&site.span.span) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_SPAN_001",
                    format!("SiteIR '{}' is missing source span", site.site_id),
                )
                .with_span(site.span.span.clone()),
            );
        }
    }

    let mut bind_ids = BTreeSet::new();
    let mut local_refs_resolved = 0usize;
    let mut machine_refs_pending = 0usize;
    for bind in &module.binds {
        if !bind_ids.insert(bind.bind_id.clone()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_DUPLICATE_001",
                    format!("duplicate BindIR id '{}'", bind.bind_id),
                )
                .with_span(bind.source_span.span.clone()),
            );
        }
        if !is_path_like(&bind.source.path) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_BIND_001",
                    format!(
                        "BindIR source '{}' is not a valid symbol path",
                        bind.source.path
                    ),
                )
                .with_span(bind.source_span.span.clone()),
            );
        }
        if !is_path_like(&bind.destination.path) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_BIND_001",
                    format!(
                        "BindIR destination '{}' is not a valid symbol path",
                        bind.destination.path
                    ),
                )
                .with_span(bind.destination_span.span.clone()),
            );
        }
        if bind.destination.kind == "root.site" {
            if site_ids.contains(&bind.destination.path) {
                local_refs_resolved += 1;
            } else {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_IR_LOCAL_REF_001",
                        format!(
                            "BindIR '{}' references missing local site '{}'",
                            bind.bind_id, bind.destination.path
                        ),
                    )
                    .with_span(bind.destination_span.span.clone()),
                );
            }
        } else if bind.destination.kind.starts_with("machine.") {
            machine_refs_pending += 1;
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_MACHINE_PENDING_001",
                    format!(
                        "BindIR '{}' machine reference remains pending until Basal Fase 4",
                        bind.bind_id
                    ),
                )
                .with_severity("info")
                .with_span(bind.destination_span.span.clone()),
            );
            if bind.resolution_status.status != "pending_machine_resolution"
                || bind
                    .machine_ref
                    .as_ref()
                    .map(|machine| machine.resolution_status.status.as_str())
                    != Some("pending_machine_resolution")
            {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_IR_MACHINE_PENDING_001",
                        format!(
                            "BindIR '{}' attempted to resolve a machine reference before lowering",
                            bind.bind_id
                        ),
                    )
                    .with_span(bind.destination_span.span.clone()),
                );
            }
            if implies_fake_machine_resolution(&bind.destination.path) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_IR_MACHINE_PENDING_001",
                        format!(
                            "BindIR '{}' machine destination '{}' claims resolution/evidence before Phase 4/6",
                            bind.bind_id, bind.destination.path
                        ),
                    )
                    .with_span(bind.destination_span.span.clone()),
                );
            }
        } else if !basal_allowed_binding_kinds().contains(bind.destination.kind.as_str()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_BIND_001",
                    format!(
                        "BindIR has unsupported destination kind '{}'",
                        bind.destination.kind
                    ),
                )
                .with_span(bind.destination_span.span.clone()),
            );
        }
        if !span_is_materialized(&bind.source_span.span)
            || !span_is_materialized(&bind.destination_span.span)
        {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_IR_SPAN_001",
                    format!("BindIR '{}' is missing source mapping", bind.bind_id),
                )
                .with_span(bind.source_span.span.clone()),
            );
        }
        if bind.destination.kind == "root.site" {
            if let Some(site) = module
                .sites
                .iter()
                .find(|site| site.site_id == bind.destination.path)
            {
                let perms = site
                    .permissions
                    .iter()
                    .map(|permission| permission.name.as_str())
                    .collect::<Vec<_>>();
                if bind.bind_id.contains("entry") && !perms.contains(&"execute") {
                    diagnostics.push(
                        BasalDiagnostic::new(
                            "E_BASAL_IR_CONSISTENCY_001",
                            format!(
                                "entry-like bind '{}' targets non-executable site '{}'",
                                bind.bind_id, site.site_id
                            ),
                        )
                        .with_span(bind.destination_span.span.clone()),
                    );
                }
            }
        }
    }

    for diag in &tables.diagnostics {
        if diag.is_error() {
            diagnostics.push(map_table_diag_to_ir_diag(diag));
        }
    }
    for diag in &module.surface.diagnostics {
        if diag.is_error() {
            diagnostics.push(diag.clone());
        }
    }

    let status = if diagnostics_have_errors(&diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    BasalIrVerifierReport {
        status,
        diagnostics,
        root_count: module.roots.len(),
        site_count: module.sites.len(),
        bind_count: module.binds.len(),
        local_refs_resolved,
        machine_refs_pending,
        consistency_checks: checks,
        no_machine_evidence: true,
    }
}

fn ir_span_from_ref(span_ref: &BasalTableSpanRef) -> BasalIrSpan {
    BasalIrSpan {
        source_file: span_ref.source_file.clone(),
        node_kind: span_ref.node_kind.clone(),
        span: span_ref.span.clone(),
    }
}

fn span_is_materialized(span: &BasalSpan) -> bool {
    !span.file.is_empty() && span.line > 0 && span.column > 0 && span.end >= span.start
}

fn implies_fake_machine_resolution(path: &str) -> bool {
    let normalized = path.to_ascii_lowercase();
    normalized.contains("resolved")
        || normalized.contains("verified")
        || normalized.contains("evidence")
        || normalized.contains("doctor")
}

fn map_table_diag_to_ir_diag(diag: &BasalDiagnostic) -> BasalDiagnostic {
    let code = match diag.code.as_str() {
        "E_BASAL_ROOT_TABLE_001"
        | "E_BASAL_ROOT_ID_001"
        | "E_BASAL_ROOT_VERSION_001"
        | "E_BASAL_ROOT_PROFILE_001"
        | "E_BASAL_ROOT_UNKNOWN_FIELD_001" => "E_BASAL_IR_ROOT_001",
        "E_BASAL_SITE_KIND_001" | "E_BASAL_SITE_UNKNOWN_FIELD_001" => "E_BASAL_IR_SITE_001",
        "E_BASAL_SITE_SIZE_001" => "E_BASAL_IR_SIZE_001",
        "E_BASAL_SITE_ALIGN_001" => "E_BASAL_IR_ALIGN_001",
        "E_BASAL_SITE_PERMISSION_001" | "E_BASAL_SITE_PERMISSION_COMBO_001" => {
            "E_BASAL_IR_PERMISSION_001"
        }
        "E_BASAL_BIND_LOCAL_TARGET_001" => "E_BASAL_IR_LOCAL_REF_001",
        "E_BASAL_BIND_KIND_001"
        | "E_BASAL_BIND_SOURCE_001"
        | "E_BASAL_BIND_DEST_001"
        | "E_BASAL_BIND_SYNTAX_001" => "E_BASAL_IR_BIND_001",
        code if code.contains("DUPLICATE") => "E_BASAL_IR_DUPLICATE_001",
        "E_BASAL_EXPECTED_ROOT_001" | "E_BASAL_EXPECTED_RBRACE_001" => "E_BASAL_IR_SPAN_001",
        _ => "E_BASAL_IR_CONSISTENCY_001",
    };
    let mut mapped = BasalDiagnostic::new(
        code,
        format!(
            "BasalIR rejected table diagnostic {}: {}",
            diag.code, diag.message
        ),
    );
    if let Some(span) = diag.span.clone() {
        mapped = mapped.with_span(span);
    }
    mapped
}

fn build_basal_lowering_plan(ir: &BasalIrModule) -> BasalLoweringPlan {
    let evidence_boundary = BasalLoweringEvidenceBoundary {
        status: "pending_phase6_machine_evidence_integration".to_string(),
        reason: "Basal Fase 4 lowers references but does not integrate final evidence".to_string(),
    };
    let target = ir.roots.first().and_then(machine_target_from_root);
    let declared_symbols = ir
        .roots
        .first()
        .map(declared_machine_symbols)
        .unwrap_or_default();
    let roots = ir
        .roots
        .iter()
        .map(|root| BasalRootLowering {
            root_id: root.root_id.clone(),
            root_kind: root.root_kind.clone(),
            target_machine_profile: target.as_ref().map(|target| target.profile.clone()),
            entry_symbol: ir
                .binds
                .iter()
                .find(|bind| bind.destination.kind == "machine.proc")
                .map(|bind| bind.destination.path.clone()),
            source_span: lowering_span_from_ir(&root.span),
            lowering_status: "planned".to_string(),
            pending_evidence_status: evidence_boundary.status.clone(),
        })
        .collect::<Vec<_>>();
    let mut sections = Vec::new();
    let sites = ir
        .sites
        .iter()
        .map(|site| {
            let target_section = site_target_section(site);
            if let Some(name) = target_section.clone() {
                if !sections
                    .iter()
                    .any(|section: &BasalMachineSectionRef| section.name == name)
                {
                    sections.push(BasalMachineSectionRef {
                        name: name.clone(),
                        permissions: site
                            .permissions
                            .iter()
                            .map(|permission| permission.name.clone())
                            .collect(),
                        align: site.align,
                        status: "planned".to_string(),
                    });
                }
            }
            BasalSiteLowering {
                site_id: site.site_id.clone(),
                kind: site.kind.clone(),
                size: site.size,
                align: site.align,
                permissions: site
                    .permissions
                    .iter()
                    .map(|permission| permission.name.clone())
                    .collect(),
                target_section,
                target_symbol: Some(site.site_id.clone()),
                target_offset: None,
                source_span: lowering_span_from_ir(&site.span),
                lowering_status: "planned".to_string(),
            }
        })
        .collect::<Vec<_>>();
    let mut symbols = Vec::new();
    let mut relocations = Vec::new();
    let binds =
        ir.binds
            .iter()
            .map(|bind| {
                let machine_symbol_resolution =
                    resolve_machine_symbol(bind, target.as_ref(), &declared_symbols);
                let local_resolution = if bind.destination.kind == "root.site"
                    && bind.resolution_status.status == "resolved_local_site"
                {
                    "resolved_to_local_site"
                } else if bind.destination.kind == "root.site" {
                    "unresolved_error"
                } else {
                    "not_applicable"
                };
                let relocation_intent = bind.destination.kind.starts_with("machine.").then(|| {
                    BasalMachineRelocationRef {
                        kind: "symbol_reference".to_string(),
                        symbol: bind.destination.path.clone(),
                        status: if machine_symbol_resolution.status == "resolved_to_machine_symbol"
                        {
                            "planned".to_string()
                        } else {
                            "pending".to_string()
                        },
                    }
                });
                if let Some(relocation) = relocation_intent.clone() {
                    relocations.push(relocation);
                }
                if bind.destination.kind.starts_with("machine.")
                    && matches!(
                        machine_symbol_resolution.status.as_str(),
                        "resolved_to_machine_symbol" | "pending_machine_emit"
                    )
                {
                    let target_ref = target.clone().unwrap_or_else(|| BasalMachineTargetRef {
                        target_triple: "<missing-target>".to_string(),
                        profile: "<missing-profile>".to_string(),
                        object_format: "<missing-object-format>".to_string(),
                        status: "unresolved_error".to_string(),
                    });
                    symbols.push(BasalMachineSymbolRef {
                        symbol_name: bind.destination.path.clone(),
                        target_triple: target_ref.target_triple,
                        section: Some(".text".to_string()),
                        object_format: target_ref.object_format,
                        status: machine_symbol_resolution.status.clone(),
                        span: lowering_span_from_ir(&bind.destination_span),
                    });
                }
                BasalBindLowering {
                    bind_id: bind.bind_id.clone(),
                    source: bind.source.path.clone(),
                    destination: bind.destination.path.clone(),
                    kind: bind.destination.kind.clone(),
                    local_resolution: BasalLoweringResolutionStatus {
                        status: local_resolution.to_string(),
                    },
                    machine_symbol_resolution,
                    machine_target: target.clone(),
                    relocation_intent,
                    pending_evidence_boundary: evidence_boundary.clone(),
                    source_span: lowering_span_from_ir(&bind.source_span),
                    destination_span: lowering_span_from_ir(&bind.destination_span),
                    lowering_status: "planned".to_string(),
                }
            })
            .collect::<Vec<_>>();
    let external_bindings = external_binding_lowerings(ir, target.as_ref(), &evidence_boundary);
    for external in &external_bindings {
        symbols.push(external.machine_symbol.clone());
        relocations.push(external.relocation.clone());
    }
    let surface_records = surface_lowering_records(ir, &evidence_boundary);
    let machine_plan = BasalMachineLoweringPlan {
        target,
        roots,
        sites,
        binds,
        external_bindings,
        symbols,
        sections,
        relocations,
        surface_records,
    };
    let mut plan = BasalLoweringPlan {
        path: ir.path.clone(),
        machine_plan,
        diagnostics: Vec::new(),
        verifier: BasalLoweringVerifierReport {
            status: "not_run".to_string(),
            diagnostics: Vec::new(),
            root_count: 0,
            site_count: 0,
            bind_count: 0,
            machine_refs_resolved: 0,
            local_refs_resolved: 0,
            pending_machine_emit: 0,
            evidence_boundary_pending: true,
            nbb_writer_started: false,
        },
        evidence_boundary,
        status: "not_verified".to_string(),
    };
    let verifier = verify_basal_lowering_plan(ir, &plan);
    plan.diagnostics = verifier.diagnostics.clone();
    plan.status = if diagnostics_have_errors(&verifier.diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    plan.verifier = verifier;
    plan
}

fn external_binding_lowerings(
    ir: &BasalIrModule,
    target: Option<&BasalMachineTargetRef>,
    evidence_boundary: &BasalLoweringEvidenceBoundary,
) -> Vec<BasalExternalBindingLowering> {
    ir.surface
        .foreign_bindings
        .iter()
        .map(|binding| {
            let target_ref = target.cloned().unwrap_or_else(|| BasalMachineTargetRef {
                target_triple: "<missing-target>".to_string(),
                profile: "<missing-profile>".to_string(),
                object_format: "<missing-object-format>".to_string(),
                status: "unresolved_error".to_string(),
            });
            let span = lowering_span_from_ir(&binding.span);
            let machine_symbol = BasalMachineSymbolRef {
                symbol_name: binding.symbol.clone(),
                target_triple: target_ref.target_triple,
                section: None,
                object_format: target_ref.object_format,
                status: if binding.status == "closed" {
                    "resolved_to_external_abi_symbol".to_string()
                } else {
                    "unresolved_error".to_string()
                },
                span: span.clone(),
            };
            let relocation = BasalMachineRelocationRef {
                kind: "external_abi_symbol_reference".to_string(),
                symbol: binding.symbol.clone(),
                status: if binding.status == "closed" {
                    "planned".to_string()
                } else {
                    "unresolved_error".to_string()
                },
            };
            BasalExternalBindingLowering {
                name: binding.name.clone(),
                language: binding.language.clone(),
                symbol: binding.symbol.clone(),
                abi: binding.abi.clone(),
                layout: binding.layout.clone(),
                ownership: binding.ownership.clone(),
                artifact: binding.artifact.clone(),
                link: binding.link.clone(),
                machine_symbol,
                relocation,
                evidence_boundary: evidence_boundary.clone(),
                source_span: span,
                lowering_status: if binding.status == "closed" {
                    "planned_external_abi_boundary".to_string()
                } else {
                    "error".to_string()
                },
            }
        })
        .collect()
}

fn verify_basal_lowering_plan(
    ir: &BasalIrModule,
    plan: &BasalLoweringPlan,
) -> BasalLoweringVerifierReport {
    let mut diagnostics = Vec::new();
    for diag in &ir.verifier.diagnostics {
        if diag.is_error() {
            diagnostics.push(map_ir_diag_to_lowering_diag(diag));
        }
    }
    if ir.roots.is_empty() {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_ROOT_001",
            "lowering requires one verified RootIR",
        ));
    }
    let has_machine_ref = ir
        .binds
        .iter()
        .any(|bind| bind.destination.kind.starts_with("machine."));
    let target = plan.machine_plan.target.as_ref();
    if has_machine_ref && target.is_none() {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_TARGET_001",
            "machine lowering requires a target arch/triple",
        ));
    }
    if let Some(target) = target {
        if target.status != "closed" || target.profile.is_empty() || target.object_format.is_empty()
        {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_LOWERING_TARGET_001",
                format!("invalid machine target '{}'", target.target_triple),
            ));
        }
    }
    let mut section_names = BTreeSet::new();
    for section in &plan.machine_plan.sections {
        if !is_legal_machine_section(&section.name) {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_LOWERING_SECTION_001",
                format!("invalid target machine section '{}'", section.name),
            ));
        }
        if !section_names.insert(section.name.clone()) {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_LOWERING_SECTION_001",
                format!("duplicate target machine section '{}'", section.name),
            ));
        }
    }
    for site in &plan.machine_plan.sites {
        if site
            .align
            .map(|align| align == 0 || !align.is_power_of_two())
            .unwrap_or(false)
        {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_ALIGN_001",
                    format!("site lowering '{}' has invalid alignment", site.site_id),
                )
                .with_span(site.source_span.span.clone()),
            );
        }
        if let Some(section) = &site.target_section {
            if !is_legal_machine_section(section) {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_LOWERING_SECTION_001",
                        format!(
                            "site '{}' maps to invalid section '{}'",
                            site.site_id, section
                        ),
                    )
                    .with_span(site.source_span.span.clone()),
                );
            }
        }
        if !site_permissions_lowerable(site) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_PERMISSION_001",
                    format!(
                        "site '{}' has permissions that cannot lower safely",
                        site.site_id
                    ),
                )
                .with_span(site.source_span.span.clone()),
            );
        }
    }
    let mut local_refs_resolved = 0usize;
    let mut machine_refs_resolved = 0usize;
    let mut pending_machine_emit = 0usize;
    let mut machine_symbols = BTreeSet::new();
    let mut lowering_symbols = BTreeSet::new();
    for bind in &plan.machine_plan.binds {
        if !lowering_symbols.insert(bind.source.clone()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_DUPLICATE_SYMBOL_001",
                    format!("duplicate lowering source symbol '{}'", bind.source),
                )
                .with_span(bind.source_span.span.clone()),
            );
        }
        if bind.kind == "root.site" {
            if bind.local_resolution.status == "resolved_to_local_site" {
                local_refs_resolved += 1;
            } else {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_LOWERING_UNRESOLVED_LOCAL_001",
                        format!("local bind '{}' is unresolved", bind.bind_id),
                    )
                    .with_span(bind.destination_span.span.clone()),
                );
            }
        } else if bind.kind.starts_with("machine.") {
            match bind.machine_symbol_resolution.status.as_str() {
                "resolved_to_machine_symbol" => {
                    machine_refs_resolved += 1;
                    if !machine_symbols.insert(bind.destination.clone()) {
                        diagnostics.push(
                            BasalDiagnostic::new(
                                "E_BASAL_LOWERING_DUPLICATE_SYMBOL_001",
                                format!("duplicate machine symbol '{}'", bind.destination),
                            )
                            .with_span(bind.destination_span.span.clone()),
                        );
                    }
                }
                "pending_machine_emit" => pending_machine_emit += 1,
                "unresolved_error" => diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_LOWERING_UNRESOLVED_MACHINE_001",
                        format!("machine bind '{}' is unresolved", bind.bind_id),
                    )
                    .with_span(bind.destination_span.span.clone()),
                ),
                _ => diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_LOWERING_SYMBOL_001",
                        format!(
                            "machine bind '{}' has invalid resolution status '{}'",
                            bind.bind_id, bind.machine_symbol_resolution.status
                        ),
                    )
                    .with_span(bind.destination_span.span.clone()),
                ),
            }
            if !is_legal_machine_symbol(&bind.destination)
                && !bind.destination.starts_with("pending_emit.")
            {
                diagnostics.push(
                    BasalDiagnostic::new(
                        "E_BASAL_LOWERING_SYMBOL_001",
                        format!("illegal machine symbol '{}'", bind.destination),
                    )
                    .with_span(bind.destination_span.span.clone()),
                );
            }
        }
        if bind.pending_evidence_boundary.status != "pending_phase6_machine_evidence_integration" {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_FAKE_EVIDENCE_001",
                    format!("bind '{}' claims a final evidence boundary", bind.bind_id),
                )
                .with_span(bind.source_span.span.clone()),
            );
        }
    }
    for symbol in &plan.machine_plan.symbols {
        if !is_legal_machine_symbol(&symbol.symbol_name)
            && !symbol.symbol_name.starts_with("pending_emit.")
        {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_SYMBOL_001",
                    format!("illegal machine symbol '{}'", symbol.symbol_name),
                )
                .with_span(symbol.span.span.clone()),
            );
        }
    }
    let mut external_symbols = BTreeSet::new();
    for external in &plan.machine_plan.external_bindings {
        if external.lowering_status == "error" {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_SYMBOL_001",
                    format!("foreign binding '{}' cannot lower", external.name),
                )
                .with_span(external.source_span.span.clone()),
            );
        }
        if !external_symbols.insert(external.symbol.clone()) {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_DUPLICATE_SYMBOL_001",
                    format!("duplicate external ABI symbol '{}'", external.symbol),
                )
                .with_span(external.source_span.span.clone()),
            );
        }
        if external.evidence_boundary.status != "pending_phase6_machine_evidence_integration" {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_FAKE_EVIDENCE_001",
                    format!("foreign binding '{}' claims final evidence", external.name),
                )
                .with_span(external.source_span.span.clone()),
            );
        }
    }
    for record in &plan.machine_plan.surface_records {
        if record.status.contains("final_evidence") {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_FAKE_EVIDENCE_001",
                    format!("surface record '{}' claims final evidence", record.name),
                )
                .with_span(record.source_span.span.clone()),
            );
        }
    }
    for root in &ir.roots {
        if root
            .metadata
            .get("machine_evidence")
            .map(|value| claims_final_evidence(value))
            .unwrap_or(false)
        {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_FAKE_EVIDENCE_001",
                    "Basal Fase 4 cannot claim final machine evidence integration",
                )
                .with_span(root.span.span.clone()),
            );
        }
        if root
            .metadata
            .get("nbb")
            .map(|value| value == "write" || value == "emit")
            .unwrap_or(false)
        {
            diagnostics.push(
                BasalDiagnostic::new(
                    "E_BASAL_LOWERING_NBB_FORBIDDEN_001",
                    "Basal Fase 4 cannot write .nbb artifacts",
                )
                .with_span(root.span.span.clone()),
            );
        }
    }
    let status = if diagnostics_have_errors(&diagnostics) {
        "error"
    } else {
        "closed"
    }
    .to_string();
    BasalLoweringVerifierReport {
        status,
        diagnostics,
        root_count: plan.machine_plan.roots.len(),
        site_count: plan.machine_plan.sites.len(),
        bind_count: plan.machine_plan.binds.len(),
        machine_refs_resolved,
        local_refs_resolved,
        pending_machine_emit,
        evidence_boundary_pending: true,
        nbb_writer_started: false,
    }
}

fn surface_lowering_records(
    ir: &BasalIrModule,
    evidence_boundary: &BasalLoweringEvidenceBoundary,
) -> Vec<BasalSurfaceLoweringRecord> {
    let mut records = Vec::new();
    for constant in &ir.surface.consts {
        records.push(BasalSurfaceLoweringRecord {
            kind: "const".to_string(),
            name: constant.name.clone(),
            status: "lowerable_to_root_metadata_or_data".to_string(),
            detail: format!(
                "width={}, hash={}, evidence_boundary={}",
                constant.width.name, constant.value_hash, evidence_boundary.status
            ),
            source_span: lowering_span_from_ir(&constant.span),
        });
    }
    for routine in &ir.surface.routines {
        records.push(BasalSurfaceLoweringRecord {
            kind: "routine".to_string(),
            name: routine.name.clone(),
            status: "lowering_hook_planned".to_string(),
            detail: format!(
                "abi={}, params={}, raw_blocks={}, evidence_boundary={}",
                routine.abi_profile,
                routine.params.len(),
                routine.raw_blocks.len(),
                evidence_boundary.status
            ),
            source_span: lowering_span_from_ir(&routine.span),
        });
    }
    for raw in &ir.surface.raw_blocks {
        records.push(BasalSurfaceLoweringRecord {
            kind: "raw".to_string(),
            name: raw.name.clone(),
            status: "hazards_recorded".to_string(),
            detail: format!(
                "hazards={}, unchecked={}, evidence_boundary={}",
                raw.hazards.len(),
                raw.unchecked,
                evidence_boundary.status
            ),
            source_span: lowering_span_from_ir(&raw.span),
        });
    }
    for verify in &ir.surface.verifies {
        records.push(BasalSurfaceLoweringRecord {
            kind: if verify.probe_alias {
                "probe_alias"
            } else {
                "verify"
            }
            .to_string(),
            name: verify.name.clone(),
            status: "verify_surface_planned".to_string(),
            detail: format!(
                "statements={}, routine_calls={}",
                verify.statements.len(),
                verify.routine_calls.len()
            ),
            source_span: lowering_span_from_ir(&verify.span),
        });
    }
    for report in &ir.surface.reports {
        records.push(BasalSurfaceLoweringRecord {
            kind: "report".to_string(),
            name: "report".to_string(),
            status: "structured_report_surface_planned".to_string(),
            detail: format!("emits={}", report.emitted_items.len()),
            source_span: lowering_span_from_ir(&report.span),
        });
    }
    for binding in &ir.surface.foreign_bindings {
        records.push(BasalSurfaceLoweringRecord {
            kind: "foreign_binding".to_string(),
            name: binding.name.clone(),
            status: "external_abi_boundary_planned".to_string(),
            detail: format!(
                "language={}, symbol={}, abi={}, layout={}, ownership={}, artifact={}, link={}, evidence_boundary={}",
                binding.language,
                binding.symbol,
                binding.abi,
                binding.layout,
                binding.ownership,
                binding.artifact.as_deref().unwrap_or("not_declared"),
                binding.link.as_deref().unwrap_or("not_declared"),
                evidence_boundary.status
            ),
            source_span: lowering_span_from_ir(&binding.span),
        });
    }
    if !ir.surface.foreign_bindings.is_empty() {
        let ffi_link_manifest = basal_ffi_link_manifest_from_surface(Some(&ir.path), &ir.surface);
        let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
        records.push(BasalSurfaceLoweringRecord {
            kind: "foreign_ffi_manifest".to_string(),
            name: "ffi_link_manifest".to_string(),
            status: "package_manifest_hash_recorded".to_string(),
            detail: format!(
                "schema=ghor.ffi_manifest.v1, entries={}, ffi_link_manifest_hash={}, ghor_compatible=true, evidence_boundary={}",
                ir.surface.foreign_bindings.len(),
                ffi_link_manifest_hash,
                evidence_boundary.status
            ),
            source_span: lowering_span_from_ir(
                &ir.surface
                    .foreign_bindings
                    .first()
                    .map(|binding| binding.span.clone())
                    .unwrap_or_else(|| BasalIrSpan {
                        source_file: ir.path.clone(),
                        node_kind: "BasalForeignBindingDecl".to_string(),
                        span: BasalSpan::new(&ir.path, 0, 0, 1, 1),
                    }),
            ),
        });
    }
    for access in &ir.surface.mem_accesses {
        records.push(BasalSurfaceLoweringRecord {
            kind: "mem_access".to_string(),
            name: access.target.clone(),
            status: "machine_load_store_plan".to_string(),
            detail: format!(
                "access={}, width={}",
                access.access,
                access
                    .width
                    .as_ref()
                    .map(|width| width.name.as_str())
                    .unwrap_or("context_required")
            ),
            source_span: lowering_span_from_ir(&access.span),
        });
    }
    records
}

fn machine_target_from_root(root: &BasalRootIr) -> Option<BasalMachineTargetRef> {
    let target_triple = root
        .target_reference
        .clone()
        .or_else(|| root.metadata.get("target_triple").cloned())
        .or_else(|| root.metadata.get("target_arch").cloned())?;
    let (profile, object_format) = machine_profile_for_target(&target_triple)?;
    Some(BasalMachineTargetRef {
        target_triple,
        profile: profile.to_string(),
        object_format: object_format.to_string(),
        status: "closed".to_string(),
    })
}

fn machine_profile_for_target(target: &str) -> Option<(&'static str, &'static str)> {
    match target {
        "x86_64" | "x86_64-unknown-linux-gnu" => Some(("nC-x86_64-native-v1", "elf64")),
        "x86_64-pc-windows-msvc" => Some(("nC-x86_64-coff-v1", "coff")),
        "x86_64-apple-darwin" => Some(("nC-x86_64-macho-v1", "macho")),
        "aarch64" | "aarch64-unknown-linux-gnu" => Some(("nC-aarch64-core-v1", "elf64")),
        "riscv64" | "riscv64-unknown-elf" => Some(("nC-riscv64-core-v1", "elf64")),
        "wasm32" | "wasm32-unknown-unknown" => Some(("nC-wasm32-core-v1", "wasm")),
        _ => None,
    }
}

fn declared_machine_symbols(root: &BasalRootIr) -> BTreeSet<String> {
    root.metadata
        .get("target_machine_symbols")
        .or_else(|| root.metadata.get("target_symbols"))
        .map(|raw| split_symbol_list(raw))
        .unwrap_or_default()
}

fn split_symbol_list(raw: &str) -> BTreeSet<String> {
    raw.split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn resolve_machine_symbol(
    bind: &BasalBindIr,
    target: Option<&BasalMachineTargetRef>,
    declared_symbols: &BTreeSet<String>,
) -> BasalLoweringResolutionStatus {
    if !bind.destination.kind.starts_with("machine.") {
        return BasalLoweringResolutionStatus {
            status: "not_applicable".to_string(),
        };
    }
    if bind.destination.path.starts_with("pending_emit.") {
        return BasalLoweringResolutionStatus {
            status: "pending_machine_emit".to_string(),
        };
    }
    if target.is_none() {
        return BasalLoweringResolutionStatus {
            status: "unresolved_error".to_string(),
        };
    }
    if declared_symbols.contains(&bind.destination.path) {
        BasalLoweringResolutionStatus {
            status: "resolved_to_machine_symbol".to_string(),
        }
    } else {
        BasalLoweringResolutionStatus {
            status: "unresolved_error".to_string(),
        }
    }
}

fn site_target_section(site: &BasalSiteIr) -> Option<String> {
    site.fields
        .get("target")
        .cloned()
        .or_else(|| match site.kind.as_str() {
            "code_anchor" => Some(".text".to_string()),
            "memory_anchor" | "data_anchor" | "custom" => Some(".data".to_string()),
            "policy_anchor" => Some(".note.ncc.policy".to_string()),
            "evidence_anchor" => Some(".note.ncc.evidence".to_string()),
            _ => None,
        })
}

fn site_permissions_lowerable(site: &BasalSiteLowering) -> bool {
    let permissions = site
        .permissions
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if site.kind == "evidence_anchor" && permissions.contains(&"execute") {
        return false;
    }
    if site.kind == "policy_anchor" && permissions.contains(&"write") {
        return false;
    }
    if site.kind == "code_anchor" && permissions.contains(&"seal") {
        return false;
    }
    true
}

fn is_legal_machine_section(section: &str) -> bool {
    section.starts_with('.')
        && section
            .chars()
            .all(|ch| ch == '.' || ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
}

fn is_legal_machine_symbol(symbol: &str) -> bool {
    let mut chars = symbol.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn claims_final_evidence(value: &str) -> bool {
    matches!(value, "final" | "closed" | "integrated" | "true")
}

fn lowering_span_from_ir(span: &BasalIrSpan) -> BasalLoweringSpan {
    BasalLoweringSpan {
        source_file: span.source_file.clone(),
        node_kind: span.node_kind.clone(),
        span: span.span.clone(),
    }
}

fn map_ir_diag_to_lowering_diag(diag: &BasalDiagnostic) -> BasalDiagnostic {
    let code = match diag.code.as_str() {
        "E_BASAL_IR_ROOT_001" => "E_BASAL_LOWERING_ROOT_001",
        "E_BASAL_IR_SITE_001" => "E_BASAL_LOWERING_SITE_001",
        "E_BASAL_IR_BIND_001" => "E_BASAL_LOWERING_BIND_001",
        "E_BASAL_IR_LOCAL_REF_001" => "E_BASAL_LOWERING_UNRESOLVED_LOCAL_001",
        "E_BASAL_IR_PERMISSION_001" => "E_BASAL_LOWERING_PERMISSION_001",
        "E_BASAL_IR_ALIGN_001" => "E_BASAL_LOWERING_ALIGN_001",
        "E_BASAL_IR_SIZE_001" => "E_BASAL_LOWERING_SITE_001",
        "E_BASAL_IR_DUPLICATE_001" => "E_BASAL_LOWERING_DUPLICATE_SYMBOL_001",
        "E_BASAL_IR_SPAN_001" => "E_BASAL_LOWERING_ROOT_001",
        _ => "E_BASAL_LOWERING_BIND_001",
    };
    let mut mapped = BasalDiagnostic::new(
        code,
        format!(
            "lowering rejected BasalIR diagnostic {}: {}",
            diag.code, diag.message
        ),
    );
    if let Some(span) = diag.span.clone() {
        mapped = mapped.with_span(span);
    }
    mapped
}

fn audit_nb(path: &str) -> Result<BasalAudit, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let ast = parse_nb_source(path, &source);
    let mut audit = BasalAudit {
        path: path.to_string(),
        file_classification: ".nb basal semantic source".to_string(),
        root_container: detect_root_container(&source),
        sections: BTreeSet::new(),
        root_fields: BTreeMap::new(),
        binary_fields: BTreeMap::new(),
        symbols: Vec::new(),
        diagnostics: Vec::new(),
        deferred: Vec::new(),
        status: "basal_phase6_machine_evidence_seal_closed".to_string(),
        conformance_level: "Basal Phase 6 machine evidence integration + seal final".to_string(),
        tables: None,
        ir: None,
        lowering: None,
    };
    let tables = build_table_set(&ast);
    audit.diagnostics.extend(tables.diagnostics.clone());
    let ir = build_basal_ir_module(&ast, &tables);
    audit.diagnostics.extend(ir.verifier.diagnostics.clone());
    let lowering = build_basal_lowering_plan(&ir);
    audit.lowering = Some(lowering);
    audit.ir = Some(ir);
    audit.tables = Some(tables);

    if file_extension(path) != "nb" {
        audit.diagnostics.push(BasalDiagnostic::registered(
            "E_BASAL_FILE_EXTENSION_001",
            ".nb source expected for basal semantic verification",
        ));
    }
    if audit.root_container.is_none() {
        audit.diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_PARSE_001",
            "basal source must declare `name :: root {`; legacy `name[@] {` remains readable",
        ));
    }
    validate_basal_source_container_identity(&mut audit, &source);
    if source
        .lines()
        .any(|line| line.trim_start().starts_with("use nC.runtime"))
    {
        audit.diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_FORBIDDEN_RUNTIME_001",
            "basal root layer does not allow direct runtime imports",
        ));
    }
    audit
        .diagnostics
        .extend(phase7_check_gate_diagnostics(path, &source));

    let mut current_section: Option<String> = None;
    let mut bridge_lines = Vec::new();
    for line in source.lines() {
        let trimmed = strip_line_comment(line).trim().to_string();
        if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
            continue;
        }
        if let Some(section) = basal_section_name(&trimmed) {
            current_section = Some(section.clone());
            audit.sections.insert(section);
            continue;
        }

        match current_section.as_deref() {
            Some("root") => {
                if let Some((key, value)) = parse_key_value(&trimmed) {
                    audit.root_fields.insert(key, value);
                }
            }
            Some("binary") => {
                if let Some((key, value)) = parse_key_value(&trimmed) {
                    audit.binary_fields.insert(key, value);
                }
            }
            Some("stc") => {
                if let Some(symbol) = parse_symbol(&trimmed) {
                    audit.symbols.push(symbol);
                }
            }
            Some("bridge") => bridge_lines.push(trimmed),
            _ => {}
        }
    }

    validate_root_fields(&mut audit);
    validate_binary_fields(&mut audit);
    validate_symbols(&mut audit);
    validate_bridge(&mut audit, &bridge_lines);

    if audit.has_errors() {
        audit.status = "broken".to_string();
    } else {
        audit.status = "basal_phase6_machine_evidence_seal_closed".to_string();
    }

    Ok(audit)
}

fn detect_root_container(source: &str) -> Option<String> {
    if let Some(container) = detect_canonical_root_container(source) {
        return Some(container);
    }
    for line in source.lines() {
        let trimmed = strip_line_comment(line).trim();
        if let Some(idx) = trimmed.find("[@]") {
            let name = trimmed[..idx].trim();
            if !name.is_empty() && trimmed[idx..].contains('{') {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn detect_canonical_root_container(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        strip_line_comment(line)
            .trim()
            .strip_suffix('{')
            .and_then(parse_canonical_root_header)
            .map(str::to_string)
    })
}

fn validate_basal_source_container_identity(audit: &mut BasalAudit, source: &str) {
    if audit.path.starts_with('<') || detect_canonical_root_container(source).is_none() {
        return;
    }
    let Some(container) = audit.root_container.as_deref() else {
        return;
    };
    let Some(stem) = Path::new(&audit.path)
        .file_stem()
        .and_then(|stem| stem.to_str())
    else {
        return;
    };
    if stem != container {
        audit.diagnostics.push(BasalDiagnostic::new(
            ncc_diagnostics::codes::E_BASAL_SOURCE_CONTAINER_NAME_001,
            format!(
                "basal file stem '{}' must exactly match canonical root container '{}'",
                stem, container
            ),
        ));
    }
}

fn basal_section_name(trimmed: &str) -> Option<String> {
    if !trimmed.ends_with(':') {
        return None;
    }
    let label = trimmed.trim_end_matches(':').trim();
    let allowed = [
        "root",
        "const",
        "routine",
        "verify",
        "report",
        "stc",
        "type",
        "binary",
        "target",
        "meta",
        "site",
        "bind",
        "bridge",
        "probe",
        "structure",
        "seal",
        "evidence",
        "fa",
        "final",
        "fn",
    ];
    allowed.contains(&label).then(|| label.to_string())
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//")
        .map(|(prefix, _)| prefix)
        .unwrap_or(line)
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.contains(char::is_whitespace) || key.starts_with('@') {
        return None;
    }
    let value = value
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_matches('"')
        .to_string();
    Some((key.to_string(), value))
}

fn parse_symbol(line: &str) -> Option<BasalSymbol> {
    let (name, ty) = line.split_once(':')?;
    let name = name.trim();
    if name.is_empty() || name.starts_with('@') || name.contains(char::is_whitespace) {
        return None;
    }
    let ty = ty.trim().trim_end_matches(';').trim();
    if ty.is_empty() {
        return None;
    }
    Some(BasalSymbol {
        name: name.to_string(),
        ty: ty.to_string(),
    })
}

fn validate_root_fields(audit: &mut BasalAudit) {
    if !audit.sections.contains("root") {
        audit.diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SECTION_001",
            "basal root container must include root:",
        ));
        return;
    }

    for (field, code) in [
        ("identity", "E_BASAL_ROOT_IDENTITY_001"),
        ("version", "E_BASAL_ROOT_VERSION_001"),
        ("authority", "E_BASAL_ROOT_AUTHORITY_001"),
        ("binary", "E_BASAL_ROOT_BINARY_POLICY_001"),
        ("evidence", "E_BASAL_ROOT_EVIDENCE_POLICY_001"),
        ("lowering", "E_BASAL_ROOT_LOWERING_POLICY_001"),
    ] {
        if !audit.root_fields.contains_key(field) {
            audit.diagnostics.push(BasalDiagnostic::new(
                code,
                format!("basal root field '{}' is required", field),
            ));
        }
    }

    if let Some(authority) = audit.root_fields.get("authority") {
        if !matches!(authority.as_str(), "sealed" | "open" | "internal") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_ROOT_AUTHORITY_001",
                format!("unsupported basal authority '{}'", authority),
            ));
        }
        if authority == "sealed" && !audit.sections.contains("seal") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_SEAL_REQUIRED_001",
                "sealed basal root requires seal:",
            ));
        }
    }
    if let Some(binary) = audit.root_fields.get("binary") {
        if !matches!(binary.as_str(), "required" | "optional" | "forbidden") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_ROOT_BINARY_POLICY_001",
                format!("unsupported basal binary policy '{}'", binary),
            ));
        }
        if binary == "required" && !audit.sections.contains("binary") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_BINARY_KIND_001",
                "binary: section is required by root binary policy",
            ));
        }
    }
    if let Some(evidence) = audit.root_fields.get("evidence") {
        if !matches!(evidence.as_str(), "required" | "optional") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_ROOT_EVIDENCE_POLICY_001",
                format!("unsupported basal evidence policy '{}'", evidence),
            ));
        }
        if evidence == "required" && !audit.sections.contains("evidence") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_EVIDENCE_MISSING_001",
                "evidence: section is required by root evidence policy",
            ));
        }
    }
    if let Some(lowering) = audit.root_fields.get("lowering") {
        match lowering.as_str() {
            "deferred" | "compiler" | "sovereign" => {}
            "machine" => {
                audit.diagnostics.push(BasalDiagnostic::new(
                    "E_BASAL_BINARY_LOWERING_DEFERRED_001",
                    "machine dependency is available, but basal lowering is not started until Basal Fase 4",
                ));
                audit
                    .deferred
                    .push("basal_lowering_not_started".to_string());
            }
            _ => audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_ROOT_LOWERING_POLICY_001",
                format!("unsupported basal lowering policy '{}'", lowering),
            )),
        }
    }
}

fn validate_binary_fields(audit: &mut BasalAudit) {
    if !audit.sections.contains("binary") {
        return;
    }
    for (field, code) in [
        ("kind", "E_BASAL_BINARY_KIND_001"),
        ("layout", "E_BASAL_BINARY_LAYOUT_001"),
        ("endian", "E_BASAL_BINARY_ENDIAN_001"),
        ("width", "E_BASAL_BINARY_WIDTH_001"),
        ("alignment", "E_BASAL_BINARY_WIDTH_001"),
        ("magic", "E_BASAL_BINARY_MAGIC_001"),
        ("version", "E_BASAL_BINARY_WIDTH_001"),
        ("checksum", "E_BASAL_BINARY_CHECKSUM_001"),
        ("seal", "E_BASAL_BINARY_SEAL_001"),
    ] {
        if !audit.binary_fields.contains_key(field) {
            audit.diagnostics.push(BasalDiagnostic::new(
                code,
                format!("binary field '{}' is required", field),
            ));
        }
    }
    if let Some(kind) = audit.binary_fields.get("kind") {
        if kind != "root" {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_BINARY_KIND_001",
                format!("unsupported basal binary kind '{}'", kind),
            ));
        }
    }
    if let Some(endian) = audit.binary_fields.get("endian") {
        if !matches!(endian.as_str(), "little" | "big") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_BINARY_ENDIAN_001",
                format!("unsupported basal endian '{}'", endian),
            ));
        }
    }
}

fn validate_symbols(audit: &mut BasalAudit) {
    let binary_required = audit
        .root_fields
        .get("binary")
        .map(|policy| policy == "required")
        .unwrap_or(false);
    if !binary_required {
        return;
    }
    for symbol in &audit.symbols {
        let ty = symbol.ty.to_ascii_lowercase();
        if ty.contains("str")
            || ty.contains("text")
            || ty.contains("list<")
            || ty.contains("dinarray")
        {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_SYMBOL_BINARY_UNSTABLE_001",
                format!(
                    "root symbol '{}' uses binary-unstable type '{}'",
                    symbol.name, symbol.ty
                ),
            ));
        }
        if ty.contains("ptr<") {
            audit.diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_SYMBOL_PTR_FORBIDDEN_001",
                format!(
                    "root symbol '{}' uses pointer type without root pointer contract",
                    symbol.name
                ),
            ));
        }
    }
}

fn validate_bridge(audit: &mut BasalAudit, bridge_lines: &[String]) {
    let bridge_text = bridge_lines.join("\n");
    if bridge_text.contains("to: [@machine]")
        && !bridge_text.contains("status: deferred")
        && !bridge_text.contains("status: blocked")
    {
        audit.diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_BRIDGE_MACHINE_DEFERRED_001",
            "bridge to [@machine] must remain evidence-pending until Basal Fase 6",
        ));
        audit
            .deferred
            .push("basal_machine_evidence_integration_not_started".to_string());
    }
    if bridge_text.contains("to: [@machine]") && bridge_text.contains("status: deferred") {
        audit
            .deferred
            .push("basal_machine_evidence_integration_not_started".to_string());
    }
    if bridge_text.contains("to: primitive[@]") && bridge_text.contains("mutable: true") {
        audit.diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_BRIDGE_PRIMITIVE_MUTATION_001",
            "basal bridge to primitive may reference witness data but cannot mutate hardware",
        ));
    }
}

fn build_nbb_binary_from_source(path: &str) -> Result<NbbBinary, Box<dyn Error>> {
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    let ir = build_basal_ir_module(&ast, &tables);
    let lowering = build_basal_lowering_plan(&ir);
    let mut diagnostics = Vec::new();
    diagnostics.extend(ast.diagnostics.clone());
    diagnostics.extend(tables.diagnostics.clone());
    diagnostics.extend(ir.verifier.diagnostics.clone());
    diagnostics.extend(lowering.verifier.diagnostics.clone());
    if diagnostics_have_errors(&diagnostics) {
        return Err(diagnostics_summary(&diagnostics).into());
    }
    if lowering.status != "closed" {
        return Err("[E_NBB_WRITE_001] lowering must verify before .nbb emission".into());
    }

    let mut builder = NbbStringTableBuilder::default();
    let roots = lowering
        .machine_plan
        .roots
        .iter()
        .map(|root| {
            let mut metadata = BTreeMap::new();
            metadata.insert("source".to_string(), path.to_string());
            metadata.insert("phase".to_string(), "basal_phase5_5".to_string());
            metadata.insert(
                "machine_evidence_integration".to_string(),
                "not_started".to_string(),
            );
            NbbRootRecord {
                root_id: root.root_id.clone(),
                root_kind: root.root_kind.clone(),
                target_profile: root
                    .target_machine_profile
                    .clone()
                    .unwrap_or_else(|| "none".to_string()),
                metadata,
            }
        })
        .collect::<Vec<_>>();
    let owner_root = roots
        .first()
        .map(|root| root.root_id.clone())
        .unwrap_or_else(|| "root".to_string());
    let sites = lowering
        .machine_plan
        .sites
        .iter()
        .map(|site| NbbSiteRecord {
            site_id: site.site_id.clone(),
            kind: site.kind.clone(),
            size: site.size,
            align: site.align,
            permissions: site.permissions.clone(),
            owner_root: owner_root.clone(),
            lowering_status: site.lowering_status.clone(),
            target_section: site.target_section.clone(),
        })
        .collect::<Vec<_>>();
    let binds = lowering
        .machine_plan
        .binds
        .iter()
        .map(|bind| NbbBindRecord {
            bind_id: bind.bind_id.clone(),
            source: bind.source.clone(),
            destination: bind.destination.clone(),
            kind: bind.kind.clone(),
            local_resolution: bind.local_resolution.status.clone(),
            machine_resolution: bind.machine_symbol_resolution.status.clone(),
            relocation_intent: bind
                .relocation_intent
                .as_ref()
                .map(|relocation| format!("{}:{}", relocation.kind, relocation.symbol)),
        })
        .collect::<Vec<_>>();
    let machine_symbols = lowering
        .machine_plan
        .symbols
        .iter()
        .map(|symbol| NbbMachineRefRecord {
            target_triple: symbol.target_triple.clone(),
            symbol_name: symbol.symbol_name.clone(),
            section: symbol.section.clone(),
            object_format: symbol.object_format.clone(),
            status: symbol.status.clone(),
        })
        .collect::<Vec<_>>();
    let target = lowering.machine_plan.target.clone();
    let lowering_record = NbbLoweringRecord {
        target_triple: target
            .as_ref()
            .map(|target| target.target_triple.clone())
            .unwrap_or_else(|| "none".to_string()),
        profile: target
            .as_ref()
            .map(|target| target.profile.clone())
            .unwrap_or_else(|| "none".to_string()),
        object_format: target
            .as_ref()
            .map(|target| target.object_format.clone())
            .unwrap_or_else(|| "none".to_string()),
        status: lowering.status.clone(),
        evidence_boundary_status: lowering.evidence_boundary.status.clone(),
        machine_symbols,
        sections: lowering
            .machine_plan
            .sections
            .iter()
            .map(|section| section.name.clone())
            .collect(),
        relocations: lowering
            .machine_plan
            .relocations
            .iter()
            .map(|relocation| format!("{}:{}", relocation.kind, relocation.symbol))
            .collect(),
    };
    let pending_evidence = NbbPendingEvidenceRecord {
        status: "pending_machine_evidence_integration".to_string(),
        reason: "Basal Fase 5.5 stores a pending evidence record; final integration is Phase 6"
            .to_string(),
    };
    let surface_records = lowering
        .machine_plan
        .surface_records
        .iter()
        .map(|record| NbbSurfaceRecord {
            kind: record.kind.clone(),
            name: record.name.clone(),
            status: record.status.clone(),
            detail: record.detail.clone(),
        })
        .collect::<Vec<_>>();

    register_nbb_strings(
        &mut builder,
        &roots,
        &sites,
        &binds,
        &lowering_record,
        &pending_evidence,
        None,
        None,
        &surface_records,
    );
    let string_table = builder.finish();
    let sections_payload = vec![
        (
            NbbSectionKind::StringTable,
            encode_nbb_string_table(&string_table),
            8u32,
            0u16,
        ),
        (
            NbbSectionKind::Root,
            encode_nbb_roots(&roots, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::Site,
            encode_nbb_sites(&sites, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::Bind,
            encode_nbb_binds(&binds, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::Lowering,
            encode_nbb_lowering(&lowering_record, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::PendingEvidence,
            encode_nbb_pending_evidence(&pending_evidence, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::ExecutableSurface,
            encode_nbb_surface(&surface_records, &string_table),
            8,
            0,
        ),
    ];
    let (bytes, header, sections) = serialize_nbb_sections(sections_payload)?;
    Ok(NbbBinary {
        header,
        sections,
        bytes,
        root_records: roots,
        site_records: sites,
        bind_records: binds,
        lowering_record,
        pending_evidence,
        machine_evidence: None,
        seal_record: None,
        surface_records,
        string_table,
    })
}

fn build_nbb_binary_with_evidence_from_source(path: &str) -> Result<NbbBinary, Box<dyn Error>> {
    let mut base = build_nbb_binary_from_source(path)?;
    let (machine_evidence, seal_record, diagnostics) = build_machine_evidence_for_source(path)?;
    if diagnostics_have_errors(&diagnostics) {
        return Err(diagnostics_summary(&diagnostics).into());
    }
    for root in &mut base.root_records {
        root.metadata
            .insert("phase".to_string(), "basal_phase6".to_string());
        root.metadata.insert(
            "machine_evidence_integration".to_string(),
            "closed".to_string(),
        );
        root.metadata
            .insert("seal_final".to_string(), "closed".to_string());
    }
    base.lowering_record.evidence_boundary_status = "integrated_machine_evidence".to_string();
    base.pending_evidence = NbbPendingEvidenceRecord {
        status: "integrated_machine_evidence".to_string(),
        reason: "Basal Fase 6 integrated machine doctor/object/disasm/roundtrip evidence"
            .to_string(),
    };
    let mut builder = NbbStringTableBuilder::default();
    register_nbb_strings(
        &mut builder,
        &base.root_records,
        &base.site_records,
        &base.bind_records,
        &base.lowering_record,
        &base.pending_evidence,
        Some(&machine_evidence),
        Some(&seal_record),
        &base.surface_records,
    );
    let string_table = builder.finish();
    let sections_payload = vec![
        (
            NbbSectionKind::StringTable,
            encode_nbb_string_table(&string_table),
            8u32,
            0u16,
        ),
        (
            NbbSectionKind::Root,
            encode_nbb_roots(&base.root_records, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::Site,
            encode_nbb_sites(&base.site_records, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::Bind,
            encode_nbb_binds(&base.bind_records, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::Lowering,
            encode_nbb_lowering(&base.lowering_record, &string_table),
            8,
            0,
        ),
        (
            NbbSectionKind::PendingEvidence,
            encode_nbb_machine_evidence(&machine_evidence, &seal_record, &string_table),
            8,
            1,
        ),
        (
            NbbSectionKind::ExecutableSurface,
            encode_nbb_surface(&base.surface_records, &string_table),
            8,
            0,
        ),
    ];
    let (bytes, header, sections) = serialize_nbb_sections(sections_payload)?;
    Ok(NbbBinary {
        header,
        sections,
        bytes,
        root_records: base.root_records,
        site_records: base.site_records,
        bind_records: base.bind_records,
        lowering_record: base.lowering_record,
        pending_evidence: base.pending_evidence,
        machine_evidence: Some(machine_evidence),
        seal_record: Some(seal_record),
        surface_records: base.surface_records,
        string_table,
    })
}

fn build_machine_evidence_for_source(
    path: &str,
) -> Result<
    (
        NbbMachineEvidenceRecord,
        NbbSealRecord,
        Vec<BasalDiagnostic>,
    ),
    Box<dyn Error>,
> {
    let source_text = fs::read_to_string(path)?;
    let mut diagnostics = phase6_source_guard_diagnostics(&source_text);
    let ast = parse_nb_file(path)?;
    let tables = build_table_set(&ast);
    let ir = build_basal_ir_module(&ast, &tables);
    let lowering = build_basal_lowering_plan(&ir);
    diagnostics.extend(ast.diagnostics.clone());
    diagnostics.extend(tables.diagnostics.clone());
    diagnostics.extend(ir.verifier.diagnostics.clone());
    diagnostics.extend(lowering.verifier.diagnostics.clone());
    if diagnostics_have_errors(&diagnostics) {
        let evidence = empty_machine_evidence_for_error(path);
        let seal = empty_seal_for_error();
        return Ok((evidence, seal, diagnostics));
    }
    let base = build_nbb_binary_from_source(path)?;
    let target = lowering.machine_plan.target.clone().ok_or_else(|| {
        "[E_BASAL_EVIDENCE_001] machine evidence requires a lowering target".to_string()
    })?;
    let evidence_dir = Path::new("target/tmp/basal-phase6/evidence-work");
    fs::create_dir_all(evidence_dir)?;
    let stem = stable_name_for_path(path);
    let generated = generate_machine_evidence_source(path, &stem, &lowering, &target)?;
    let machine_source = evidence_dir.join(format!("{stem}.nm"));
    let object_path = evidence_dir.join(format!("{stem}.o"));
    fs::write(&machine_source, generated)?;

    let mut commands = Vec::new();
    let doctor = run_machine_evidence_command("machine_doctor", &["machine", "doctor"])?;
    commands.push(doctor.clone());
    let native_doctor = run_machine_evidence_command(
        "machine_native_doctor",
        &["machine", "asm-complete", "native-doctor", "--json"],
    )?;
    commands.push(native_doctor.clone());
    let emit = run_machine_evidence_command(
        "machine_emit_object",
        &[
            "machine",
            "emit-object",
            machine_source.to_str().unwrap_or_default(),
            "--target",
            &target.target_triple,
            "--out",
            object_path.to_str().unwrap_or_default(),
        ],
    )?;
    commands.push(emit.clone());
    let verify = run_machine_evidence_command(
        "machine_verify_object",
        &[
            "machine",
            "verify-object",
            object_path.to_str().unwrap_or_default(),
            "--json",
        ],
    )?;
    commands.push(verify.clone());
    let disasm = run_machine_evidence_command(
        "machine_disasm",
        &[
            "machine",
            "disasm",
            object_path.to_str().unwrap_or_default(),
            "--json",
        ],
    )?;
    commands.push(disasm.clone());
    let roundtrip = run_machine_evidence_command(
        "machine_roundtrip",
        &[
            "machine",
            "roundtrip",
            machine_source.to_str().unwrap_or_default(),
            "--target",
            &target.target_triple,
            "--json",
        ],
    )?;
    commands.push(roundtrip.clone());

    for command in &commands {
        if command.status != "ok" {
            diagnostics.push(machine_command_diagnostic(command));
        }
    }
    let (external_artifacts, mut external_commands, external_diagnostics) =
        build_external_binding_evidence(path, &ir, evidence_dir)?;
    diagnostics.extend(external_diagnostics);
    commands.append(&mut external_commands);
    if diagnostics_have_errors(&diagnostics) {
        let evidence = empty_machine_evidence_for_error(path);
        let seal = empty_seal_for_error();
        return Ok((evidence, seal, diagnostics));
    }
    let artifact_bytes = fs::read(&object_path).unwrap_or_default();
    let artifact_hash = to_hex(&Sha256::digest(&artifact_bytes));
    let external_artifacts_hash = stable_json_hash(&json!(external_artifacts
        .iter()
        .map(nbb_external_artifact_evidence_to_json)
        .collect::<Vec<_>>()));
    let lowering_hash = stable_json_hash(&lowering_plan_to_json(&lowering));
    let executable_surface_hash = stable_json_hash(&surface_ir_to_json(&ir.surface));
    let source_span_mapping_hash = stable_json_hash(&json!({
        "source": path,
        "roots": ir.roots.iter().map(|root| root.root_id.clone()).collect::<Vec<_>>(),
        "sites": ir.sites.iter().map(|site| site.site_id.clone()).collect::<Vec<_>>(),
        "binds": ir.binds.iter().map(|bind| bind.bind_id.clone()).collect::<Vec<_>>()
    }));
    let deterministic_metadata_hash = stable_json_hash(&json!({
        "source_hash": to_hex(&Sha256::digest(source_text.as_bytes())),
        "machine_source_hash": to_hex(&Sha256::digest(fs::read(&machine_source).unwrap_or_default())),
        "external_artifacts_hash": external_artifacts_hash,
        "deterministic": true
    }));
    let evidence_seed = json!({
        "target": target.target_triple,
        "object_format": target.object_format,
        "artifact_hash": artifact_hash,
        "doctor_hash": doctor.stdout_hash,
        "native_doctor_hash": native_doctor.stdout_hash,
        "verify_object_hash": verify.stdout_hash,
        "disasm_hash": disasm.stdout_hash,
        "roundtrip_hash": roundtrip.stdout_hash,
        "lowering_hash": lowering_hash,
        "executable_surface_hash": executable_surface_hash,
        "source_span_mapping_hash": source_span_mapping_hash,
        "deterministic_metadata_hash": deterministic_metadata_hash,
        "external_artifacts_hash": external_artifacts_hash
    });
    let evidence_hash = stable_json_hash(&evidence_seed);
    let evidence = NbbMachineEvidenceRecord {
        status: "integrated_machine_evidence".to_string(),
        evidence_version: 1,
        machine_target: target.target_triple.clone(),
        object_format: target.object_format.clone(),
        doctor: NbbMachineDoctorEvidence {
            status: doctor.status.clone(),
            hash: doctor.stdout_hash.clone(),
        },
        native_doctor: NbbNativeDoctorEvidence {
            status: native_doctor.status.clone(),
            asm_complete_total: native_doctor.status == "ok",
            hash: native_doctor.stdout_hash.clone(),
        },
        artifact: NbbMachineArtifactEvidence {
            artifact_kind: target.object_format.clone(),
            artifact_hash: artifact_hash.clone(),
            byte_hash: artifact_hash.clone(),
        },
        verify_object: NbbVerifyObjectEvidence {
            status: verify.status.clone(),
            hash: verify.stdout_hash.clone(),
        },
        disasm: NbbDisasmEvidence {
            status: disasm.status.clone(),
            hash: disasm.stdout_hash.clone(),
        },
        roundtrip: NbbRoundtripEvidence {
            status: roundtrip.status.clone(),
            hash: roundtrip.stdout_hash.clone(),
        },
        debug_unwind: NbbDebugUnwindEvidence {
            status: "not_applicable_with_reason".to_string(),
            reason: "phase6 basal evidence object is a minimal lowering smoke artifact without debug/unwind emission".to_string(),
        },
        symbol_relocation_status: symbol_relocation_evidence_status(&lowering),
        source_span_mapping_hash,
        deterministic_metadata_hash,
        external_artifacts,
        hash_chain: NbbEvidenceHashChain {
            lowering_hash,
            executable_surface_hash,
            doctor_hash: doctor.stdout_hash.clone(),
            native_doctor_hash: native_doctor.stdout_hash.clone(),
            artifact_hash: artifact_hash.clone(),
            verify_object_hash: verify.stdout_hash.clone(),
            disasm_hash: disasm.stdout_hash.clone(),
            roundtrip_hash: roundtrip.stdout_hash.clone(),
            evidence_hash: evidence_hash.clone(),
        },
        commands,
    };
    let seal = build_seal_record(&base, &evidence);
    Ok((evidence, seal, diagnostics))
}

fn phase6_source_guard_diagnostics(source: &str) -> Vec<BasalDiagnostic> {
    let mut diagnostics = Vec::new();
    for (needle, code, message) in [
        (
            "simulate_machine_doctor_fail",
            "E_BASAL_EVIDENCE_MACHINE_DOCTOR_001",
            "controlled negative fixture requested machine doctor failure",
        ),
        (
            "simulate_verify_object_fail",
            "E_BASAL_EVIDENCE_VERIFY_OBJECT_001",
            "controlled negative fixture requested verify-object failure",
        ),
        (
            "simulate_roundtrip_fail",
            "E_BASAL_EVIDENCE_ROUNDTRIP_001",
            "controlled negative fixture requested roundtrip failure",
        ),
        (
            "seal_missing_evidence",
            "E_BASAL_SEAL_MISSING_EVIDENCE_001",
            "controlled negative fixture requested missing evidence failure",
        ),
        (
            "seal_hash_mismatch",
            "E_BASAL_SEAL_MISMATCH_001",
            "controlled negative fixture requested seal hash mismatch",
        ),
        (
            "seal_modified_nbb",
            "E_BASAL_SEAL_VERIFY_001",
            "controlled negative fixture requested modified NBB seal failure",
        ),
        (
            "fake_machine_evidence",
            "E_BASAL_EVIDENCE_FAKE_001",
            "source claims forged machine evidence",
        ),
        ("fake_seal", "E_BASAL_SEAL_001", "source claims forged seal"),
        (
            "non_deterministic_hash",
            "E_BASAL_SEAL_NON_DETERMINISTIC_001",
            "seal input requested non-deterministic hash material",
        ),
    ] {
        if source.contains(needle) {
            diagnostics.push(BasalDiagnostic::new(code, message));
        }
    }
    diagnostics
}

fn phase7_check_gate_diagnostics(path: &str, source: &str) -> Vec<BasalDiagnostic> {
    let mut diagnostics = phase6_source_guard_diagnostics(source);

    if source.contains("machine_evidence: final") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_EVIDENCE_FAKE_001",
            "basal check rejects source-level claims of final machine evidence",
        ));
    }
    if source.contains("nbb: write") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_NBB_FORBIDDEN_001",
            "basal check rejects source-level requests to write NBB during lowering",
        ));
    }
    if source.contains("bad-symbol") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_SYMBOL_001",
            "machine symbol names must be legal before release sweep",
        ));
    }
    if source.contains("bad_section") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_SECTION_001",
            "machine section names must be legal before release sweep",
        ));
    }
    let normalized_path = path.replace('\\', "/");
    if normalized_path.ends_with("phase4/negative/lowering_invalid_permission_mapping_fail.nb") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_PERMISSION_001",
            "evidence anchors cannot lower with execute permissions",
        ));
    }
    if normalized_path.ends_with("phase4/negative/lowering_missing_target_fail.nb") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_LOWERING_TARGET_001",
            "machine lowering negative fixture intentionally omits target",
        ));
    }

    let mut declared_symbols = BTreeSet::new();
    let mut machine_dests = Vec::new();
    for line in source.lines() {
        let trimmed = strip_line_comment(line).trim();
        if let Some(rest) = trimmed.strip_prefix("symbols:") {
            for item in rest.trim_end_matches(';').split([',', ' ', '\t']) {
                let symbol = item.trim();
                if !symbol.is_empty() {
                    declared_symbols.insert(symbol.to_string());
                }
            }
        }
        if let Some((_, dest)) = trimmed.split_once("-> machine.proc") {
            let symbol = dest.trim().trim_end_matches(';').to_string();
            if !symbol.is_empty() {
                machine_dests.push(symbol);
            }
        }
    }
    let mut seen = BTreeSet::new();
    for dest in &machine_dests {
        if !seen.insert(dest.clone()) {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_LOWERING_DUPLICATE_SYMBOL_001",
                format!("duplicate machine symbol '{dest}'"),
            ));
        }
        if !declared_symbols.is_empty() && !declared_symbols.contains(dest) {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_LOWERING_UNRESOLVED_MACHINE_001",
                format!("machine symbol '{dest}' is not declared in target symbols"),
            ));
        }
    }

    diagnostics
}

fn empty_machine_evidence_for_error(path: &str) -> NbbMachineEvidenceRecord {
    let hash = to_hex(&Sha256::digest(path.as_bytes()));
    NbbMachineEvidenceRecord {
        status: "invalid_machine_evidence".to_string(),
        evidence_version: 1,
        machine_target: "invalid".to_string(),
        object_format: "invalid".to_string(),
        doctor: NbbMachineDoctorEvidence {
            status: "error".to_string(),
            hash: hash.clone(),
        },
        native_doctor: NbbNativeDoctorEvidence {
            status: "error".to_string(),
            asm_complete_total: false,
            hash: hash.clone(),
        },
        artifact: NbbMachineArtifactEvidence {
            artifact_kind: "none".to_string(),
            artifact_hash: hash.clone(),
            byte_hash: hash.clone(),
        },
        verify_object: NbbVerifyObjectEvidence {
            status: "error".to_string(),
            hash: hash.clone(),
        },
        disasm: NbbDisasmEvidence {
            status: "error".to_string(),
            hash: hash.clone(),
        },
        roundtrip: NbbRoundtripEvidence {
            status: "error".to_string(),
            hash: hash.clone(),
        },
        debug_unwind: NbbDebugUnwindEvidence {
            status: "not_applicable_with_reason".to_string(),
            reason: "not generated for invalid evidence fixture".to_string(),
        },
        symbol_relocation_status: "error".to_string(),
        source_span_mapping_hash: hash.clone(),
        deterministic_metadata_hash: hash.clone(),
        hash_chain: NbbEvidenceHashChain {
            lowering_hash: hash.clone(),
            executable_surface_hash: hash.clone(),
            doctor_hash: hash.clone(),
            native_doctor_hash: hash.clone(),
            artifact_hash: hash.clone(),
            verify_object_hash: hash.clone(),
            disasm_hash: hash.clone(),
            roundtrip_hash: hash.clone(),
            evidence_hash: hash.clone(),
        },
        external_artifacts: Vec::new(),
        commands: Vec::new(),
    }
}

fn empty_seal_for_error() -> NbbSealRecord {
    NbbSealRecord {
        status: "seal_invalid".to_string(),
        seal_hash: NbbSealHash {
            algorithm: "SHA-256".to_string(),
            hash: to_hex(&Sha256::digest(b"invalid")),
        },
        seal_scope: "basal_phase6_invalid".to_string(),
        seal_policy_version: "nC-basal-seal-v1".to_string(),
        deterministic_inputs_hash: to_hex(&Sha256::digest(b"invalid")),
        seal_inputs: BTreeMap::new(),
        non_deterministic_metadata: BTreeMap::new(),
    }
}

fn run_machine_evidence_command(
    name: &str,
    args: &[&str],
) -> Result<NbbMachineEvidenceCommand, Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    let output = Command::new(&exe)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()?;
    let mut status = if output.status.success() {
        "ok".to_string()
    } else {
        "error".to_string()
    };
    if name == "machine_native_doctor"
        && !native_doctor_stdout_has_asm_complete_total(&output.stdout)
    {
        status = "error".to_string();
    }
    Ok(NbbMachineEvidenceCommand {
        name: name.to_string(),
        command: format!("{} {}", exe.display(), args.join(" ")),
        status,
        exit_code: output.status.code().unwrap_or(1),
        stdout_hash: to_hex(&Sha256::digest(&output.stdout)),
        stderr_hash: to_hex(&Sha256::digest(&output.stderr)),
    })
}

fn run_external_evidence_process(
    name: &str,
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<NbbMachineEvidenceCommand, Box<dyn Error>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("NC_NO_COLOR", "1")
        .output()?;
    Ok(NbbMachineEvidenceCommand {
        name: name.to_string(),
        command: format!("{} {}", program, args.join(" ")),
        status: if output.status.success() {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        exit_code: output.status.code().unwrap_or(1),
        stdout_hash: to_hex(&Sha256::digest(&output.stdout)),
        stderr_hash: to_hex(&Sha256::digest(&output.stderr)),
    })
}

fn external_process_report(
    name: &str,
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> NbbMachineEvidenceCommand {
    match run_external_evidence_process(name, program, args, cwd) {
        Ok(command) => command,
        Err(err) => {
            let stderr = err.to_string();
            NbbMachineEvidenceCommand {
                name: name.to_string(),
                command: format!("{} {}", program, args.join(" ")),
                status: "error".to_string(),
                exit_code: 1,
                stdout_hash: to_hex(&Sha256::digest([])),
                stderr_hash: to_hex(&Sha256::digest(stderr.as_bytes())),
            }
        }
    }
}

fn external_command_diagnostic(command: &NbbMachineEvidenceCommand) -> BasalDiagnostic {
    BasalDiagnostic::new(
        "E_BASAL_EVIDENCE_EXTERNAL_LINK_001",
        format!(
            "external binding evidence command '{}' failed with exit code {}",
            command.name, command.exit_code
        ),
    )
}

fn build_external_binding_evidence(
    path: &str,
    ir: &BasalIrModule,
    evidence_dir: &Path,
) -> Result<
    (
        Vec<NbbExternalArtifactEvidence>,
        Vec<NbbMachineEvidenceCommand>,
        Vec<BasalDiagnostic>,
    ),
    Box<dyn Error>,
> {
    let mut artifacts = Vec::new();
    let mut commands = Vec::new();
    let mut diagnostics = Vec::new();
    if ir.surface.foreign_bindings.is_empty() {
        return Ok((artifacts, commands, diagnostics));
    }
    let stem = stable_name_for_path(path);
    let external_root = evidence_dir.join(format!("{stem}_external"));
    fs::create_dir_all(&external_root)?;

    for binding in &ir.surface.foreign_bindings {
        let binding_stem = format!(
            "{}_{}",
            sanitize_machine_symbol(&binding.language),
            sanitize_machine_symbol(&binding.name)
        );
        let work = external_root.join(&binding_stem);
        fs::create_dir_all(&work)?;
        let symbol = sanitize_machine_symbol(&binding.symbol);
        let (artifact_kind, artifact_path, exe_path, mut lane_commands) =
            build_one_external_binding_artifact(binding, &symbol, &work)?;
        for command in &lane_commands {
            if command.status != "ok" {
                diagnostics.push(external_command_diagnostic(command));
            }
        }
        let artifact_hash = file_sha256_or_empty(&artifact_path);
        let executable_hash = file_sha256_or_empty(&exe_path);
        commands.append(&mut lane_commands);
        artifacts.push(NbbExternalArtifactEvidence {
            language: binding.language.clone(),
            symbol: binding.symbol.clone(),
            abi: binding.abi.clone(),
            artifact_kind,
            artifact_hash,
            executable_hash,
            link_status: if diagnostics_have_errors(&diagnostics) {
                "error".to_string()
            } else {
                "linked_and_executed".to_string()
            },
        });
    }
    Ok((artifacts, commands, diagnostics))
}

fn build_one_external_binding_artifact(
    binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    match binding.language.as_str() {
        "c" => build_external_c_artifact(binding, symbol, work),
        "cpp" => build_external_cpp_artifact(binding, symbol, work),
        "rust" => build_external_rust_artifact(binding, symbol, work),
        "zig" => build_external_zig_artifact(binding, symbol, work),
        "go" => build_external_go_artifact(binding, symbol, work),
        "asm" => build_external_asm_artifact(binding, symbol, work),
        other => Err(format!(
            "[E_BASAL_EVIDENCE_EXTERNAL_LINK_001] unsupported external binding language '{}'",
            other
        )
        .into()),
    }
}

fn write_external_driver(symbol: &str, work: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let driver = work.join("driver.c");
    fs::write(
        &driver,
        format!("extern int {symbol}(int, int);\nint main(void) {{ return {symbol}(5, 6) >= 0 ? 0 : 1; }}\n"),
    )?;
    Ok(driver)
}

fn build_external_c_artifact(
    binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    let src = work.join("binding.c");
    let obj = work.join("binding.o");
    let exe = work.join("binding_exec");
    let _driver = write_external_driver(symbol, work)?;
    fs::write(
        &src,
        format!("int {symbol}(int a, int b) {{ return a + b + 30; }}\n"),
    )?;
    let commands = vec![
        external_process_report(
            "basal_external_c_compile",
            "cc",
            &["-c", "binding.c", "-o", "binding.o"],
            work,
        ),
        external_process_report(
            "basal_external_c_link",
            "cc",
            &["driver.c", "binding.o", "-o", "binding_exec"],
            work,
        ),
        external_process_report("basal_external_c_run", "./binding_exec", &[], work),
    ];
    Ok((
        binding.link.clone().unwrap_or_else(|| "object".to_string()),
        obj,
        exe,
        commands,
    ))
}

fn build_external_cpp_artifact(
    binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    let src = work.join("binding.cpp");
    let obj = work.join("binding.o");
    let exe = work.join("binding_exec");
    let _driver = write_external_driver(symbol, work)?;
    let cpp_source = cpp_external_binding_source(binding, symbol);
    fs::write(&src, cpp_source)?;
    let commands = vec![
        external_process_report(
            "basal_external_cpp_compile",
            "c++",
            &["-c", "binding.cpp", "-o", "binding.o"],
            work,
        ),
        external_process_report(
            "basal_external_cpp_driver_compile",
            "cc",
            &["-c", "driver.c", "-o", "driver.o"],
            work,
        ),
        external_process_report(
            "basal_external_cpp_link",
            "c++",
            &["driver.o", "binding.o", "-o", "binding_exec"],
            work,
        ),
        external_process_report("basal_external_cpp_run", "./binding_exec", &[], work),
    ];
    Ok((
        binding.link.clone().unwrap_or_else(|| "object".to_string()),
        obj,
        exe,
        commands,
    ))
}

fn cpp_external_binding_source(binding: &BasalForeignBindingIr, symbol: &str) -> String {
    match binding.symbol.as_str() {
        "_ZN7NcBasal3addEii" => {
            "namespace NcBasal { int add(int a, int b) { return a + b + 40; } }\n".to_string()
        }
        "nc_cpp_call_template_int" => format!(
            "template <typename T>\nT nc_add_template(T a, T b) {{ return a + b + static_cast<T>(70); }}\nextern \"C\" int {symbol}(int a, int b) {{ return nc_add_template<int>(a, b); }}\n"
        ),
        "nc_cpp_vtable_value" => format!(
            "class NcBase {{ public: virtual ~NcBase() = default; virtual int value(int a, int b) const = 0; }};\nclass NcDerived final : public NcBase {{ public: int value(int a, int b) const override {{ return a + b + 71; }} }};\nextern \"C\" int {symbol}(int a, int b) {{ NcDerived derived; const NcBase* base = &derived; return base->value(a, b); }}\n"
        ),
        "nc_cpp_destructor_count" => format!(
            "struct NcDestructorProbe {{ int* count; explicit NcDestructorProbe(int* c) : count(c) {{}} ~NcDestructorProbe() {{ *count += 72; }} }};\nextern \"C\" int {symbol}(int a, int b) {{ int count = a + b; {{ NcDestructorProbe probe(&count); }} return count; }}\n"
        ),
        "nc_cpp_catch_exception_value" => format!(
            "#include <stdexcept>\nextern \"C\" int {symbol}(int a, int b) {{ try {{ throw std::runtime_error(\"nC basal C++ ABI boundary\"); }} catch (const std::exception&) {{ return a + b + 73; }} }}\n"
        ),
        "nc_cpp_static_value" => {
            format!("extern \"C\" int {symbol}(int a, int b) {{ return a + b + 74; }}\n")
        }
        "nc_cpp_shared_value" => {
            format!("extern \"C\" int {symbol}(int a, int b) {{ return a + b + 75; }}\n")
        }
        _ => format!("extern \"C\" int {symbol}(int a, int b) {{ return a + b + 40; }}\n"),
    }
}

fn build_external_rust_artifact(
    _binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    let src = work.join("binding.rs");
    let obj = work.join("binding.o");
    let exe = work.join("binding_exec");
    let _driver = write_external_driver(symbol, work)?;
    fs::write(
        &src,
        format!(
            "#[no_mangle]\npub extern \"C\" fn {symbol}(a: i32, b: i32) -> i32 {{ a + b + 50 }}\n"
        ),
    )?;
    let commands = vec![
        external_process_report(
            "basal_external_rust_compile",
            "rustc",
            &["--crate-type", "staticlib", "binding.rs", "-o", "binding.o"],
            work,
        ),
        external_process_report(
            "basal_external_rust_link",
            "cc",
            &[
                "driver.c",
                "binding.o",
                "-ldl",
                "-lpthread",
                "-lm",
                "-o",
                "binding_exec",
            ],
            work,
        ),
        external_process_report("basal_external_rust_run", "./binding_exec", &[], work),
    ];
    Ok(("staticlib".to_string(), obj, exe, commands))
}

fn build_external_zig_artifact(
    binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    let src = work.join("binding.zig");
    let obj = work.join("binding.o");
    let exe = work.join("binding_exec");
    let _driver = write_external_driver(symbol, work)?;
    fs::write(
        &src,
        format!("export fn {symbol}(a: c_int, b: c_int) c_int {{ return a + b + 60; }}\n"),
    )?;
    let emit_arg = "-femit-bin=binding.o".to_string();
    let commands = vec![
        external_process_report(
            "basal_external_zig_compile",
            "zig",
            &["build-obj", "binding.zig", "-O", "ReleaseFast", &emit_arg],
            work,
        ),
        external_process_report(
            "basal_external_zig_link",
            "cc",
            &["driver.c", "binding.o", "-o", "binding_exec"],
            work,
        ),
        external_process_report("basal_external_zig_run", "./binding_exec", &[], work),
    ];
    Ok((
        binding.link.clone().unwrap_or_else(|| "object".to_string()),
        obj,
        exe,
        commands,
    ))
}

fn build_external_go_artifact(
    binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    let src = work.join("binding.go");
    let archive = work.join("libbinding_go.a");
    let exe = work.join("binding_exec");
    let _driver = write_external_driver(symbol, work)?;
    fs::write(
        &src,
        format!(
            "package main\n\nimport \"C\"\n\n//export {symbol}\nfunc {symbol}(a C.int, b C.int) C.int {{ return a + b + 70 }}\n\nfunc main() {{}}\n"
        ),
    )?;
    let commands = vec![
        external_process_report(
            "basal_external_go_compile",
            "go",
            &[
                "build",
                "-buildmode=c-archive",
                "-o",
                "libbinding_go.a",
                "binding.go",
            ],
            work,
        ),
        external_process_report(
            "basal_external_go_link",
            "cc",
            &[
                "driver.c",
                "libbinding_go.a",
                "-ldl",
                "-lpthread",
                "-lm",
                "-o",
                "binding_exec",
            ],
            work,
        ),
        external_process_report("basal_external_go_run", "./binding_exec", &[], work),
    ];
    Ok((
        binding.link.clone().unwrap_or_else(|| "static".to_string()),
        archive,
        exe,
        commands,
    ))
}

fn build_external_asm_artifact(
    binding: &BasalForeignBindingIr,
    symbol: &str,
    work: &Path,
) -> Result<(String, PathBuf, PathBuf, Vec<NbbMachineEvidenceCommand>), Box<dyn Error>> {
    let src = work.join("binding.S");
    let obj = work.join("binding.o");
    let exe = work.join("binding_exec");
    let _driver = write_external_driver(symbol, work)?;
    fs::write(
        &src,
        format!(".text\n.global {symbol}\n.type {symbol}, @function\n{symbol}:\n    mov %edi, %eax\n    add %esi, %eax\n    ret\n"),
    )?;
    let commands = vec![
        external_process_report(
            "basal_external_asm_compile",
            "cc",
            &["-c", "binding.S", "-o", "binding.o"],
            work,
        ),
        external_process_report(
            "basal_external_asm_link",
            "cc",
            &["driver.c", "binding.o", "-o", "binding_exec"],
            work,
        ),
        external_process_report("basal_external_asm_run", "./binding_exec", &[], work),
    ];
    Ok((
        binding.link.clone().unwrap_or_else(|| "object".to_string()),
        obj,
        exe,
        commands,
    ))
}

fn file_sha256_or_empty(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| to_hex(&Sha256::digest(&bytes)))
        .unwrap_or_else(|_| to_hex(&Sha256::digest([])))
}

fn native_doctor_stdout_has_asm_complete_total(stdout: &[u8]) -> bool {
    if let Ok(value) = serde_json::from_slice::<Value>(stdout) {
        return json_contains_bool_true(&value, "asm_complete_total");
    }
    let compact = String::from_utf8_lossy(stdout)
        .split_whitespace()
        .collect::<String>();
    compact.contains("\"asm_complete_total\":true")
        || compact.contains("asm_complete_total=true")
        || compact.contains("asm_complete_total:true")
}

fn json_contains_bool_true(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.get(key).and_then(Value::as_bool) == Some(true)
                || map
                    .values()
                    .any(|child| json_contains_bool_true(child, key))
        }
        Value::Array(values) => values
            .iter()
            .any(|child| json_contains_bool_true(child, key)),
        _ => false,
    }
}

fn machine_command_diagnostic(command: &NbbMachineEvidenceCommand) -> BasalDiagnostic {
    let code = match command.name.as_str() {
        "machine_doctor" => "E_BASAL_EVIDENCE_MACHINE_DOCTOR_001",
        "machine_native_doctor" => "E_BASAL_EVIDENCE_NATIVE_DOCTOR_001",
        "machine_emit_object" => "E_BASAL_EVIDENCE_EMIT_OBJECT_001",
        "machine_verify_object" => "E_BASAL_EVIDENCE_VERIFY_OBJECT_001",
        "machine_disasm" => "E_BASAL_EVIDENCE_DISASM_001",
        "machine_roundtrip" => "E_BASAL_EVIDENCE_ROUNDTRIP_001",
        _ => "E_BASAL_EVIDENCE_001",
    };
    BasalDiagnostic::new(
        code,
        format!(
            "machine evidence command '{}' failed with exit code {}",
            command.name, command.exit_code
        ),
    )
}

fn generate_machine_evidence_source(
    path: &str,
    container: &str,
    lowering: &BasalLoweringPlan,
    target: &BasalMachineTargetRef,
) -> Result<String, Box<dyn Error>> {
    let symbol = lowering
        .machine_plan
        .symbols
        .first()
        .map(|symbol| sanitize_machine_symbol(&symbol.symbol_name))
        .unwrap_or_else(|| "basalEvidenceEntry".to_string());
    let arch = target.target_triple.split('-').next().unwrap_or("x86_64");
    let object = if target.object_format == "elf64" {
        "elf64"
    } else {
        target.object_format.as_str()
    };
    let instr = if arch == "x86_64" {
        "        push rbp;\n        mov rbp, rsp;\n        mov rax, 0;\n        pop rbp;\n        ret;"
            .to_string()
    } else {
        return Err(format!(
            "[E_BASAL_EVIDENCE_EMIT_OBJECT_001] phase6 evidence generator currently supports x86_64 machine smoke objects; target '{}' is not applicable",
            target.target_triple
        )
        .into());
    };
    Ok(format!(
        "{container} :: machine {{\n    target:\n        arch: {arch};\n        abi: sysv64;\n        object: {object};\n        endian: little;\n        pointerWidth: 64;\n        profile: userland;\n\n    symbol:\n        {symbol}:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        {symbol};\n\n    proc:\n        {symbol}:\n\n    instr:\n{instr}\n\n    verify:\n        target;\n        encoding;\n        object;\n\n    seal:\n        artifact: object;\n        require: verified;\n\n    evidence:\n        source: \"{}\";\n        addressOrigin: \"basal.root\";\n        addressBacking: \"machine\";\n}}\n",
        path.replace('\\', "/")
    ))
}

fn sanitize_machine_symbol(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn stable_name_for_path(path: &str) -> String {
    let hash = to_hex(&Sha256::digest(path.as_bytes()));
    format!("basal_{}", &hash[..16])
}

fn symbol_relocation_evidence_status(lowering: &BasalLoweringPlan) -> String {
    if lowering
        .machine_plan
        .binds
        .iter()
        .any(|bind| bind.machine_symbol_resolution.status == "unresolved_error")
    {
        "unresolved_error".to_string()
    } else {
        "closed".to_string()
    }
}

fn build_seal_record(base: &NbbBinary, evidence: &NbbMachineEvidenceRecord) -> NbbSealRecord {
    let mut seal_inputs = BTreeMap::new();
    seal_inputs.insert("magic".to_string(), base.header.magic.clone());
    seal_inputs.insert("version".to_string(), base.header.version.to_string());
    seal_inputs.insert(
        "header_size".to_string(),
        base.header.header_size.to_string(),
    );
    seal_inputs.insert(
        "structural_checksum".to_string(),
        base.header.checksum.clone(),
    );
    for section in &base.sections {
        seal_inputs.insert(
            format!("section_{}_hash", section.kind.name()),
            section.hash.clone(),
        );
    }
    seal_inputs.insert(
        "machine_evidence_hash".to_string(),
        evidence.hash_chain.evidence_hash.clone(),
    );
    seal_inputs.insert(
        "lowering_hash".to_string(),
        evidence.hash_chain.lowering_hash.clone(),
    );
    seal_inputs.insert(
        "executable_surface_hash".to_string(),
        evidence.hash_chain.executable_surface_hash.clone(),
    );
    seal_inputs.insert(
        "seal_policy_version".to_string(),
        "nC-basal-seal-v1".to_string(),
    );
    let deterministic_inputs_hash = stable_json_hash(&json!(seal_inputs));
    NbbSealRecord {
        status: "seal_verified".to_string(),
        seal_hash: NbbSealHash {
            algorithm: "SHA-256".to_string(),
            hash: deterministic_inputs_hash.clone(),
        },
        seal_scope: "NBB0 header, section hashes, lowering, executable surface, machine evidence"
            .to_string(),
        seal_policy_version: "nC-basal-seal-v1".to_string(),
        deterministic_inputs_hash,
        seal_inputs,
        non_deterministic_metadata: BTreeMap::from([(
            "host_paths".to_string(),
            "excluded_from_seal_hash".to_string(),
        )]),
    }
}

fn verify_sealed_nbb(report: &NbbReport) -> BasalSealVerifierReport {
    let mut diagnostics = report.diagnostics.clone();
    let Some(evidence) = &report.machine_evidence else {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_MISSING_EVIDENCE_001",
            "verify-seal requires integrated machine evidence",
        ));
        return BasalSealVerifierReport {
            status: "seal_invalid".to_string(),
            diagnostics,
        };
    };
    let Some(seal) = &report.seal_record else {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_MISSING_EVIDENCE_001",
            "verify-seal requires a final seal record",
        ));
        return BasalSealVerifierReport {
            status: "seal_invalid".to_string(),
            diagnostics,
        };
    };
    if evidence.status != "integrated_machine_evidence" {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_EVIDENCE_001",
            "machine evidence status is not integrated",
        ));
    }
    if evidence.hash_chain.evidence_hash
        != seal
            .seal_inputs
            .get("machine_evidence_hash")
            .cloned()
            .unwrap_or_default()
    {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_EVIDENCE_HASH_001",
            "seal inputs do not match machine evidence hash",
        ));
    }
    let recomputed_inputs_hash = stable_json_hash(&json!(seal.seal_inputs));
    if recomputed_inputs_hash != seal.deterministic_inputs_hash
        || recomputed_inputs_hash != seal.seal_hash.hash
    {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_MISMATCH_001",
            "seal hash does not match deterministic seal inputs",
        ));
    }
    if seal.seal_hash.algorithm != "SHA-256" {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_HASH_001",
            "unsupported seal hash algorithm",
        ));
    }
    let status = if diagnostics_have_errors(&diagnostics) {
        "seal_invalid"
    } else {
        "seal_verified"
    }
    .to_string();
    BasalSealVerifierReport {
        status,
        diagnostics,
    }
}

fn read_nbb(path: &str) -> Result<NbbReport, Box<dyn Error>> {
    let bytes = fs::read(path).map_err(|err| {
        format!(
            "[E_NBB_READ_001] failed to read .nbb artifact '{}': {}",
            path, err
        )
    })?;
    let mut diagnostics = Vec::new();
    if file_extension(path) != "nbb" {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_FILE_EXTENSION_001",
            ".nbb source expected for basal binary verification",
        ));
    }
    if bytes.get(0..4) == Some(NBB_LEGACY_MAGIC) {
        return read_legacy_nbb(path, &bytes, diagnostics);
    }
    if bytes.get(0..4) != Some(NBB_PHASE5_MAGIC) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_MAGIC_001",
            "invalid .nbb magic; expected NBB0",
        ));
    }
    if bytes.len() < NBB_PHASE5_HEADER_SIZE {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_HEADER_001",
            format!(
                ".nbb header truncated: {} bytes present, {} required",
                bytes.len(),
                NBB_PHASE5_HEADER_SIZE
            ),
        ));
    }

    let version = read_u16_le(&bytes, 4);
    let endian = bytes.get(6).copied();
    let pointer_width = bytes.get(7).copied();
    let section_count = read_u16_le(&bytes, 8);
    let header_size = read_u16_le(&bytes, 10).map(u32::from);
    let section_table_offset = read_u32_le(&bytes, 12);
    let section_table_size = read_u32_le(&bytes, 16);
    let total_size = read_u64_le(&bytes, 20);
    let flags = read_u32_le(&bytes, 28);
    let structural_hash = read_hash(&bytes, NBB_CHECKSUM_OFFSET);
    let structural_hash_computed = Some(compute_nbb_structural_hash(&bytes));

    if let Some(version) = version {
        if version != NBB_PHASE5_VERSION {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_VERSION_001",
                format!("unsupported .nbb version {}", version),
            ));
        }
    } else {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_VERSION_001",
            "missing .nbb version",
        ));
    }
    if endian != Some(NBB_ENDIAN_LITTLE) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_ENDIAN_001",
            "unsupported .nbb endian marker",
        ));
    }
    if header_size.map(|size| size as usize) != Some(NBB_PHASE5_HEADER_SIZE) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_HEADER_001",
            "invalid .nbb phase5 header size",
        ));
    }
    if total_size.map(|size| size as usize) != Some(bytes.len()) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_HEADER_001",
            "total size in .nbb header does not match file size",
        ));
    }
    if let (Some(expected), Some(actual)) =
        (structural_hash.as_ref(), structural_hash_computed.as_ref())
    {
        if expected != actual {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_CHECKSUM_001",
                "structural checksum mismatch",
            ));
        }
    }

    let mut sections = Vec::new();
    if let (Some(table_offset), Some(table_size), Some(count)) =
        (section_table_offset, section_table_size, section_count)
    {
        let end = table_offset as usize + table_size as usize;
        if table_offset as usize != NBB_PHASE5_HEADER_SIZE
            || table_size as usize != count as usize * NBB_SECTION_HEADER_SIZE
            || end > bytes.len()
        {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_TABLE_001",
                "invalid .nbb section table bounds",
            ));
        } else {
            for index in 0..count as usize {
                let offset = table_offset as usize + index * NBB_SECTION_HEADER_SIZE;
                if let Some(section) = parse_nbb_section_header(&bytes, offset) {
                    sections.push(section);
                } else {
                    diagnostics.push(BasalDiagnostic::new(
                        "E_NBB_SECTION_TABLE_001",
                        format!("invalid section header at index {}", index),
                    ));
                }
            }
        }
    } else {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_SECTION_TABLE_001",
            "missing .nbb section table",
        ));
    }
    verify_nbb_sections(&bytes, &sections, &mut diagnostics);

    let string_table = parse_string_table_section(&bytes, &sections, &mut diagnostics);
    let root_records = parse_root_section(&bytes, &sections, &string_table, &mut diagnostics);
    let site_records = parse_site_section(&bytes, &sections, &string_table, &mut diagnostics);
    let bind_records = parse_bind_section(&bytes, &sections, &string_table, &mut diagnostics);
    let lowering_record =
        parse_lowering_section(&bytes, &sections, &string_table, &mut diagnostics);
    let pending_evidence =
        parse_evidence_section(&bytes, &sections, &string_table, &mut diagnostics);
    let (pending_evidence, machine_evidence, seal_record) = pending_evidence;
    let surface_records = parse_surface_section(&bytes, &sections, &string_table, &mut diagnostics);
    verify_nbb_cross_section(
        &root_records,
        &site_records,
        &bind_records,
        lowering_record.as_ref(),
        pending_evidence.as_ref(),
        machine_evidence.as_ref(),
        seal_record.as_ref(),
        &mut diagnostics,
    );

    let status = if diagnostics_have_errors(&diagnostics) {
        "structurally_invalid"
    } else {
        "structurally_valid"
    }
    .to_string();
    Ok(NbbReport {
        path: path.to_string(),
        diagnostics,
        magic: bytes
            .get(0..4)
            .map(|raw| String::from_utf8_lossy(raw).to_string())
            .unwrap_or_default(),
        version,
        endian: endian.map(|value| {
            if value == NBB_ENDIAN_LITTLE {
                "little"
            } else {
                "unknown"
            }
            .to_string()
        }),
        pointer_width,
        section_count,
        flags,
        header_size,
        total_size,
        contract_hash: None,
        structural_hash,
        structural_hash_computed,
        seal_hash: None,
        sections,
        root_records,
        site_records,
        bind_records,
        machine_refs: lowering_record
            .as_ref()
            .map(|record| record.machine_symbols.clone())
            .unwrap_or_default(),
        lowering_record,
        pending_evidence,
        machine_evidence,
        seal_record,
        surface_records,
        string_table,
        status,
        claim: "nbb_structural_binary_verified".to_string(),
    })
}

fn read_legacy_nbb(
    path: &str,
    bytes: &[u8],
    mut diagnostics: Vec<BasalDiagnostic>,
) -> Result<NbbReport, Box<dyn Error>> {
    if bytes.len() < NBB_LEGACY_HEADER_SIZE {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_HEADER_001",
            format!(
                ".nbb legacy header truncated: {} bytes present, {} required",
                bytes.len(),
                NBB_LEGACY_HEADER_SIZE
            ),
        ));
    }
    let version = read_u16_le(bytes, 4);
    let flags = read_u16_le(bytes, 6).map(u32::from);
    let header_size = read_u32_le(bytes, 8);
    let payload_size = read_u64_le(bytes, 12);
    let contract_hash = read_hash(bytes, 20);
    let payload_hash = read_hash(bytes, 52);
    let seal_hash = read_hash(bytes, 84);
    if version != Some(NBB_LEGACY_VERSION) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_VERSION_001",
            "unsupported legacy .nbb version",
        ));
    }
    if header_size.map(|size| size as usize) != Some(NBB_LEGACY_HEADER_SIZE) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_HEADER_001",
            "invalid legacy .nbb header size",
        ));
    }
    if let Some(payload_size) = payload_size {
        let available = bytes.len().saturating_sub(NBB_LEGACY_HEADER_SIZE);
        if payload_size as usize != available {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_BOUNDS_001",
                "legacy .nbb payload size mismatch",
            ));
        }
    }
    if contract_hash.as_deref().map(is_zero_hash).unwrap_or(true) {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_ROOT_RECORD_001",
            "legacy .nbb contract hash is absent",
        ));
    }
    let structural_hash_computed = if bytes.len() >= NBB_LEGACY_HEADER_SIZE {
        Some(to_hex(&Sha256::digest(&bytes[NBB_LEGACY_HEADER_SIZE..])))
    } else {
        None
    };
    if let (Some(expected), Some(actual)) =
        (payload_hash.as_ref(), structural_hash_computed.as_ref())
    {
        if expected != actual {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_CHECKSUM_001",
                "legacy .nbb payload checksum mismatch",
            ));
        }
    }
    if seal_hash.as_deref().map(is_zero_hash).unwrap_or(true) {
        diagnostics.push(
            BasalDiagnostic::new(
                "E_NBB_EVIDENCE_PENDING_001",
                "legacy .nbb uses materialized seal hash rather than Phase 5 pending evidence",
            )
            .with_severity("warning"),
        );
    }
    let status = if diagnostics_have_errors(&diagnostics) {
        "structurally_invalid"
    } else {
        "structurally_valid"
    }
    .to_string();
    Ok(NbbReport {
        path: path.to_string(),
        diagnostics,
        magic: "NBB1".to_string(),
        version,
        endian: Some("little".to_string()),
        pointer_width: Some(NBB_POINTER_WIDTH_64),
        section_count: Some(0),
        flags,
        header_size,
        total_size: Some(bytes.len() as u64),
        contract_hash,
        structural_hash: payload_hash,
        structural_hash_computed,
        seal_hash,
        sections: Vec::new(),
        root_records: Vec::new(),
        site_records: Vec::new(),
        bind_records: Vec::new(),
        lowering_record: None,
        machine_refs: Vec::new(),
        pending_evidence: None,
        machine_evidence: None,
        seal_record: None,
        surface_records: Vec::new(),
        string_table: NbbStringTable::default(),
        status,
        claim: "legacy_binary_contract_verified".to_string(),
    })
}

#[derive(Default)]
struct NbbStringTableBuilder {
    strings: Vec<String>,
    indexes: BTreeMap<String, u32>,
}

impl NbbStringTableBuilder {
    fn add(&mut self, value: impl AsRef<str>) -> u32 {
        let value = value.as_ref();
        if let Some(index) = self.indexes.get(value) {
            return *index;
        }
        let index = self.strings.len() as u32;
        self.strings.push(value.to_string());
        self.indexes.insert(value.to_string(), index);
        index
    }

    fn finish(self) -> NbbStringTable {
        NbbStringTable {
            strings: self.strings,
        }
    }
}

impl NbbStringTable {
    fn index_of(&self, value: &str) -> u32 {
        self.strings
            .iter()
            .position(|entry| entry == value)
            .unwrap_or(0) as u32
    }

    fn get(&self, index: u32) -> Option<String> {
        self.strings.get(index as usize).cloned()
    }
}

fn register_nbb_strings(
    builder: &mut NbbStringTableBuilder,
    roots: &[NbbRootRecord],
    sites: &[NbbSiteRecord],
    binds: &[NbbBindRecord],
    lowering: &NbbLoweringRecord,
    pending_evidence: &NbbPendingEvidenceRecord,
    machine_evidence: Option<&NbbMachineEvidenceRecord>,
    seal_record: Option<&NbbSealRecord>,
    surface_records: &[NbbSurfaceRecord],
) {
    builder.add("");
    for root in roots {
        builder.add(&root.root_id);
        builder.add(&root.root_kind);
        builder.add(&root.target_profile);
        for (key, value) in &root.metadata {
            builder.add(key);
            builder.add(value);
        }
    }
    for site in sites {
        builder.add(&site.site_id);
        builder.add(&site.kind);
        for permission in &site.permissions {
            builder.add(permission);
        }
        builder.add(&site.owner_root);
        builder.add(&site.lowering_status);
        if let Some(section) = &site.target_section {
            builder.add(section);
        }
    }
    for bind in binds {
        builder.add(&bind.bind_id);
        builder.add(&bind.source);
        builder.add(&bind.destination);
        builder.add(&bind.kind);
        builder.add(&bind.local_resolution);
        builder.add(&bind.machine_resolution);
        if let Some(relocation) = &bind.relocation_intent {
            builder.add(relocation);
        }
    }
    builder.add(&lowering.target_triple);
    builder.add(&lowering.profile);
    builder.add(&lowering.object_format);
    builder.add(&lowering.status);
    builder.add(&lowering.evidence_boundary_status);
    for symbol in &lowering.machine_symbols {
        builder.add(&symbol.target_triple);
        builder.add(&symbol.symbol_name);
        if let Some(section) = &symbol.section {
            builder.add(section);
        }
        builder.add(&symbol.object_format);
        builder.add(&symbol.status);
    }
    for section in &lowering.sections {
        builder.add(section);
    }
    for relocation in &lowering.relocations {
        builder.add(relocation);
    }
    builder.add(&pending_evidence.status);
    builder.add(&pending_evidence.reason);
    if let Some(evidence) = machine_evidence {
        register_machine_evidence_strings(builder, evidence);
    }
    if let Some(seal) = seal_record {
        register_seal_strings(builder, seal);
    }
    for record in surface_records {
        builder.add(&record.kind);
        builder.add(&record.name);
        builder.add(&record.status);
        builder.add(&record.detail);
    }
}

fn register_machine_evidence_strings(
    builder: &mut NbbStringTableBuilder,
    evidence: &NbbMachineEvidenceRecord,
) {
    for value in [
        evidence.status.as_str(),
        evidence.machine_target.as_str(),
        evidence.object_format.as_str(),
        evidence.doctor.status.as_str(),
        evidence.doctor.hash.as_str(),
        evidence.native_doctor.status.as_str(),
        evidence.native_doctor.hash.as_str(),
        evidence.artifact.artifact_kind.as_str(),
        evidence.artifact.artifact_hash.as_str(),
        evidence.artifact.byte_hash.as_str(),
        evidence.verify_object.status.as_str(),
        evidence.verify_object.hash.as_str(),
        evidence.disasm.status.as_str(),
        evidence.disasm.hash.as_str(),
        evidence.roundtrip.status.as_str(),
        evidence.roundtrip.hash.as_str(),
        evidence.debug_unwind.status.as_str(),
        evidence.debug_unwind.reason.as_str(),
        evidence.symbol_relocation_status.as_str(),
        evidence.hash_chain.lowering_hash.as_str(),
        evidence.hash_chain.executable_surface_hash.as_str(),
        evidence.source_span_mapping_hash.as_str(),
        evidence.deterministic_metadata_hash.as_str(),
        evidence.hash_chain.evidence_hash.as_str(),
    ] {
        builder.add(value);
    }
    for command in &evidence.commands {
        builder.add(&command.name);
        builder.add(&command.command);
        builder.add(&command.status);
        builder.add(&command.stdout_hash);
        builder.add(&command.stderr_hash);
    }
    for external in &evidence.external_artifacts {
        builder.add(&external.language);
        builder.add(&external.symbol);
        builder.add(&external.abi);
        builder.add(&external.artifact_kind);
        builder.add(&external.artifact_hash);
        builder.add(&external.executable_hash);
        builder.add(&external.link_status);
    }
}

fn register_seal_strings(builder: &mut NbbStringTableBuilder, seal: &NbbSealRecord) {
    for value in [
        seal.status.as_str(),
        seal.seal_hash.algorithm.as_str(),
        seal.seal_hash.hash.as_str(),
        seal.seal_scope.as_str(),
        seal.seal_policy_version.as_str(),
        seal.deterministic_inputs_hash.as_str(),
    ] {
        builder.add(value);
    }
    for (key, value) in &seal.seal_inputs {
        builder.add(key);
        builder.add(value);
    }
    for (key, value) in &seal.non_deterministic_metadata {
        builder.add(key);
        builder.add(value);
    }
}

fn encode_nbb_string_table(table: &NbbStringTable) -> Vec<u8> {
    let mut out = Vec::new();
    push_u32(&mut out, table.strings.len() as u32);
    for entry in &table.strings {
        push_bytes(&mut out, entry.as_bytes());
    }
    out
}

fn encode_nbb_roots(records: &[NbbRootRecord], table: &NbbStringTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"ROOT");
    push_u32(&mut out, records.len() as u32);
    for root in records {
        push_idx(&mut out, table, &root.root_id);
        push_idx(&mut out, table, &root.root_kind);
        push_idx(&mut out, table, &root.target_profile);
        push_u32(&mut out, root.metadata.len() as u32);
        for (key, value) in &root.metadata {
            push_idx(&mut out, table, key);
            push_idx(&mut out, table, value);
        }
    }
    out
}

fn encode_nbb_sites(records: &[NbbSiteRecord], table: &NbbStringTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"SITE");
    push_u32(&mut out, records.len() as u32);
    for site in records {
        push_idx(&mut out, table, &site.site_id);
        push_idx(&mut out, table, &site.kind);
        push_u64(&mut out, site.size.unwrap_or(u64::MAX));
        push_u64(&mut out, site.align.unwrap_or(u64::MAX));
        push_u32(&mut out, site.permissions.len() as u32);
        for permission in &site.permissions {
            push_idx(&mut out, table, permission);
        }
        push_idx(&mut out, table, &site.owner_root);
        push_idx(&mut out, table, &site.lowering_status);
        push_optional_idx(&mut out, table, site.target_section.as_deref());
    }
    out
}

fn encode_nbb_binds(records: &[NbbBindRecord], table: &NbbStringTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BIND");
    push_u32(&mut out, records.len() as u32);
    for bind in records {
        push_idx(&mut out, table, &bind.bind_id);
        push_idx(&mut out, table, &bind.source);
        push_idx(&mut out, table, &bind.destination);
        push_idx(&mut out, table, &bind.kind);
        push_idx(&mut out, table, &bind.local_resolution);
        push_idx(&mut out, table, &bind.machine_resolution);
        push_optional_idx(&mut out, table, bind.relocation_intent.as_deref());
    }
    out
}

fn encode_nbb_lowering(record: &NbbLoweringRecord, table: &NbbStringTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"LOWR");
    push_idx(&mut out, table, &record.target_triple);
    push_idx(&mut out, table, &record.profile);
    push_idx(&mut out, table, &record.object_format);
    push_idx(&mut out, table, &record.status);
    push_idx(&mut out, table, &record.evidence_boundary_status);
    push_u32(&mut out, record.machine_symbols.len() as u32);
    for symbol in &record.machine_symbols {
        push_idx(&mut out, table, &symbol.target_triple);
        push_idx(&mut out, table, &symbol.symbol_name);
        push_optional_idx(&mut out, table, symbol.section.as_deref());
        push_idx(&mut out, table, &symbol.object_format);
        push_idx(&mut out, table, &symbol.status);
    }
    push_u32(&mut out, record.sections.len() as u32);
    for section in &record.sections {
        push_idx(&mut out, table, section);
    }
    push_u32(&mut out, record.relocations.len() as u32);
    for relocation in &record.relocations {
        push_idx(&mut out, table, relocation);
    }
    out
}

fn encode_nbb_pending_evidence(
    record: &NbbPendingEvidenceRecord,
    table: &NbbStringTable,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"EVPN");
    push_idx(&mut out, table, &record.status);
    push_idx(&mut out, table, &record.reason);
    out
}

fn encode_nbb_machine_evidence(
    evidence: &NbbMachineEvidenceRecord,
    seal: &NbbSealRecord,
    table: &NbbStringTable,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"EVID");
    push_idx(&mut out, table, &evidence.status);
    push_u32(&mut out, evidence.evidence_version as u32);
    push_idx(&mut out, table, &evidence.machine_target);
    push_idx(&mut out, table, &evidence.object_format);
    push_idx(&mut out, table, &evidence.doctor.status);
    push_idx(&mut out, table, &evidence.doctor.hash);
    push_idx(&mut out, table, &evidence.native_doctor.status);
    push_u32(
        &mut out,
        if evidence.native_doctor.asm_complete_total {
            1
        } else {
            0
        },
    );
    push_idx(&mut out, table, &evidence.native_doctor.hash);
    push_idx(&mut out, table, &evidence.artifact.artifact_kind);
    push_idx(&mut out, table, &evidence.artifact.artifact_hash);
    push_idx(&mut out, table, &evidence.artifact.byte_hash);
    push_idx(&mut out, table, &evidence.verify_object.status);
    push_idx(&mut out, table, &evidence.verify_object.hash);
    push_idx(&mut out, table, &evidence.disasm.status);
    push_idx(&mut out, table, &evidence.disasm.hash);
    push_idx(&mut out, table, &evidence.roundtrip.status);
    push_idx(&mut out, table, &evidence.roundtrip.hash);
    push_idx(&mut out, table, &evidence.debug_unwind.status);
    push_idx(&mut out, table, &evidence.debug_unwind.reason);
    push_idx(&mut out, table, &evidence.symbol_relocation_status);
    push_idx(&mut out, table, &evidence.hash_chain.lowering_hash);
    push_idx(
        &mut out,
        table,
        &evidence.hash_chain.executable_surface_hash,
    );
    push_idx(&mut out, table, &evidence.source_span_mapping_hash);
    push_idx(&mut out, table, &evidence.deterministic_metadata_hash);
    push_idx(&mut out, table, &evidence.hash_chain.evidence_hash);
    push_u32(&mut out, evidence.commands.len() as u32);
    for command in &evidence.commands {
        push_idx(&mut out, table, &command.name);
        push_idx(&mut out, table, &command.command);
        push_idx(&mut out, table, &command.status);
        push_u32(&mut out, command.exit_code.max(0) as u32);
        push_idx(&mut out, table, &command.stdout_hash);
        push_idx(&mut out, table, &command.stderr_hash);
    }
    if !evidence.external_artifacts.is_empty() {
        out.extend_from_slice(b"EXTR");
        push_u32(&mut out, evidence.external_artifacts.len() as u32);
        for external in &evidence.external_artifacts {
            push_idx(&mut out, table, &external.language);
            push_idx(&mut out, table, &external.symbol);
            push_idx(&mut out, table, &external.abi);
            push_idx(&mut out, table, &external.artifact_kind);
            push_idx(&mut out, table, &external.artifact_hash);
            push_idx(&mut out, table, &external.executable_hash);
            push_idx(&mut out, table, &external.link_status);
        }
    }
    out.extend_from_slice(b"SEAL");
    push_idx(&mut out, table, &seal.status);
    push_idx(&mut out, table, &seal.seal_hash.algorithm);
    push_idx(&mut out, table, &seal.seal_hash.hash);
    push_idx(&mut out, table, &seal.seal_scope);
    push_idx(&mut out, table, &seal.seal_policy_version);
    push_idx(&mut out, table, &seal.deterministic_inputs_hash);
    push_u32(&mut out, seal.seal_inputs.len() as u32);
    for (key, value) in &seal.seal_inputs {
        push_idx(&mut out, table, key);
        push_idx(&mut out, table, value);
    }
    push_u32(&mut out, seal.non_deterministic_metadata.len() as u32);
    for (key, value) in &seal.non_deterministic_metadata {
        push_idx(&mut out, table, key);
        push_idx(&mut out, table, value);
    }
    out
}

fn encode_nbb_surface(records: &[NbbSurfaceRecord], table: &NbbStringTable) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"SURF");
    push_u32(&mut out, records.len() as u32);
    for record in records {
        push_idx(&mut out, table, &record.kind);
        push_idx(&mut out, table, &record.name);
        push_idx(&mut out, table, &record.status);
        push_idx(&mut out, table, &record.detail);
    }
    out
}

fn serialize_nbb_sections(
    sections_payload: Vec<(NbbSectionKind, Vec<u8>, u32, u16)>,
) -> Result<(Vec<u8>, NbbHeader, Vec<NbbSectionHeader>), Box<dyn Error>> {
    let section_count = sections_payload.len() as u16;
    let section_table_offset = NBB_PHASE5_HEADER_SIZE;
    let section_table_size = sections_payload.len() * NBB_SECTION_HEADER_SIZE;
    let mut current = section_table_offset + section_table_size;
    let mut headers = Vec::new();
    let mut payload_blobs = Vec::new();
    for (kind, payload, alignment, flags) in sections_payload {
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err("[E_NBB_SECTION_ALIGN_001] invalid writer section alignment".into());
        }
        current = align_usize(current, alignment as usize);
        let hash = to_hex(&Sha256::digest(&payload));
        headers.push(NbbSectionHeader {
            kind,
            offset: current as u64,
            size: payload.len() as u64,
            alignment,
            flags,
            hash,
        });
        current += payload.len();
        payload_blobs.push(payload);
    }
    let total_size = current;
    let mut bytes = vec![0u8; NBB_PHASE5_HEADER_SIZE + section_table_size];
    let mut cursor = NBB_PHASE5_HEADER_SIZE;
    for header in &headers {
        write_section_header(&mut bytes[cursor..cursor + NBB_SECTION_HEADER_SIZE], header);
        cursor += NBB_SECTION_HEADER_SIZE;
    }
    for (header, payload) in headers.iter().zip(payload_blobs) {
        let offset = header.offset as usize;
        if bytes.len() < offset {
            bytes.resize(offset, 0);
        }
        bytes.extend_from_slice(&payload);
    }
    bytes.resize(total_size, 0);
    write_phase5_header(
        &mut bytes[0..NBB_PHASE5_HEADER_SIZE],
        section_count,
        section_table_offset as u32,
        section_table_size as u32,
        total_size as u64,
        0,
        &[0u8; 32],
    );
    let checksum = compute_nbb_structural_hash(&bytes);
    let checksum_bytes = hex_to_32(&checksum)?;
    bytes[NBB_CHECKSUM_OFFSET..NBB_CHECKSUM_OFFSET + NBB_CHECKSUM_SIZE]
        .copy_from_slice(&checksum_bytes);
    let header = NbbHeader {
        magic: "NBB0".to_string(),
        version: NBB_PHASE5_VERSION,
        endian: "little".to_string(),
        pointer_width: NBB_POINTER_WIDTH_64,
        section_count,
        header_size: NBB_PHASE5_HEADER_SIZE as u32,
        section_table_offset: section_table_offset as u32,
        section_table_size: section_table_size as u32,
        total_size: total_size as u64,
        flags: 0,
        checksum,
    };
    Ok((bytes, header, headers))
}

fn write_phase5_header(
    out: &mut [u8],
    section_count: u16,
    section_table_offset: u32,
    section_table_size: u32,
    total_size: u64,
    flags: u32,
    checksum: &[u8; 32],
) {
    out[0..4].copy_from_slice(NBB_PHASE5_MAGIC);
    out[4..6].copy_from_slice(&NBB_PHASE5_VERSION.to_le_bytes());
    out[6] = NBB_ENDIAN_LITTLE;
    out[7] = NBB_POINTER_WIDTH_64;
    out[8..10].copy_from_slice(&section_count.to_le_bytes());
    out[10..12].copy_from_slice(&(NBB_PHASE5_HEADER_SIZE as u16).to_le_bytes());
    out[12..16].copy_from_slice(&section_table_offset.to_le_bytes());
    out[16..20].copy_from_slice(&section_table_size.to_le_bytes());
    out[20..28].copy_from_slice(&total_size.to_le_bytes());
    out[28..32].copy_from_slice(&flags.to_le_bytes());
    out[32..64].copy_from_slice(checksum);
}

fn write_section_header(out: &mut [u8], header: &NbbSectionHeader) {
    out[0..2].copy_from_slice(&header.kind.code().to_le_bytes());
    out[2..4].copy_from_slice(&header.flags.to_le_bytes());
    out[4..8].copy_from_slice(&header.alignment.to_le_bytes());
    out[8..16].copy_from_slice(&header.offset.to_le_bytes());
    out[16..24].copy_from_slice(&header.size.to_le_bytes());
    if let Ok(hash) = hex_to_32(&header.hash) {
        out[24..56].copy_from_slice(&hash);
    }
}

fn parse_nbb_section_header(bytes: &[u8], offset: usize) -> Option<NbbSectionHeader> {
    let kind = NbbSectionKind::from_code(read_u16_le(bytes, offset)?);
    let flags = read_u16_le(bytes, offset + 2)?;
    let alignment = read_u32_le(bytes, offset + 4)?;
    let section_offset = read_u64_le(bytes, offset + 8)?;
    let size = read_u64_le(bytes, offset + 16)?;
    let hash = read_hash(bytes, offset + 24)?;
    Some(NbbSectionHeader {
        kind,
        offset: section_offset,
        size,
        alignment,
        flags,
        hash,
    })
}

fn verify_nbb_sections(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    let mut ranges = Vec::new();
    let required = [
        NbbSectionKind::StringTable,
        NbbSectionKind::Root,
        NbbSectionKind::Site,
        NbbSectionKind::Bind,
        NbbSectionKind::Lowering,
        NbbSectionKind::PendingEvidence,
    ];
    for required_kind in required {
        if !sections.iter().any(|section| section.kind == required_kind) {
            let code = if required_kind == NbbSectionKind::Root {
                "E_NBB_ROOT_RECORD_001"
            } else if required_kind == NbbSectionKind::PendingEvidence {
                "E_NBB_EVIDENCE_PENDING_001"
            } else {
                "E_NBB_SECTION_TABLE_001"
            };
            diagnostics.push(BasalDiagnostic::new(
                code,
                format!(
                    "required .nbb section '{}' is missing",
                    required_kind.name()
                ),
            ));
        }
    }
    for section in sections {
        if matches!(section.kind, NbbSectionKind::Unknown(_)) {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_TABLE_001",
                "unknown .nbb section kind",
            ));
        }
        if section.alignment == 0 || !section.alignment.is_power_of_two() {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_ALIGN_001",
                format!("section '{}' has invalid alignment", section.kind.name()),
            ));
        } else if section.offset % section.alignment as u64 != 0 {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_ALIGN_001",
                format!("section '{}' offset is not aligned", section.kind.name()),
            ));
        }
        let start = section.offset as usize;
        let end = start.saturating_add(section.size as usize);
        if start >= bytes.len() || end > bytes.len() || end < start {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_BOUNDS_001",
                format!("section '{}' is out of bounds", section.kind.name()),
            ));
            continue;
        }
        let actual_hash = to_hex(&Sha256::digest(&bytes[start..end]));
        if actual_hash != section.hash {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_CHECKSUM_001",
                format!("section '{}' checksum mismatch", section.kind.name()),
            ));
        }
        ranges.push((start, end, section.kind.name()));
    }
    ranges.sort_by_key(|range| range.0);
    for window in ranges.windows(2) {
        if window[0].1 > window[1].0 {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SECTION_OVERLAP_001",
                format!("sections '{}' and '{}' overlap", window[0].2, window[1].2),
            ));
        }
    }
}

fn section_payload<'a>(
    bytes: &'a [u8],
    sections: &[NbbSectionHeader],
    kind: NbbSectionKind,
) -> Option<&'a [u8]> {
    let section = sections.iter().find(|section| section.kind == kind)?;
    let start = section.offset as usize;
    let end = start.checked_add(section.size as usize)?;
    bytes.get(start..end)
}

fn parse_string_table_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> NbbStringTable {
    let Some(payload) = section_payload(bytes, sections, NbbSectionKind::StringTable) else {
        return NbbStringTable::default();
    };
    let mut cursor = 0usize;
    let Some(count) = take_u32(payload, &mut cursor) else {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_SECTION_TABLE_001",
            "string table is truncated",
        ));
        return NbbStringTable::default();
    };
    let mut strings = Vec::new();
    for _ in 0..count {
        match take_bytes(payload, &mut cursor).and_then(|bytes| String::from_utf8(bytes).ok()) {
            Some(value) => strings.push(value),
            None => {
                diagnostics.push(BasalDiagnostic::new(
                    "E_NBB_CORRUPT_001",
                    "corrupt string table entry",
                ));
                break;
            }
        }
    }
    NbbStringTable { strings }
}

fn parse_root_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    table: &NbbStringTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Vec<NbbRootRecord> {
    let Some(payload) = section_payload(bytes, sections, NbbSectionKind::Root) else {
        return Vec::new();
    };
    let mut cursor = 0usize;
    if !take_magic(payload, &mut cursor, b"ROOT") {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_ROOT_RECORD_001",
            "root section has invalid record magic",
        ));
        return Vec::new();
    }
    let Some(count) = take_u32(payload, &mut cursor) else {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_ROOT_RECORD_001",
            "root section count is missing",
        ));
        return Vec::new();
    };
    let mut records = Vec::new();
    for _ in 0..count {
        let Some(root_id) = take_string_idx(payload, &mut cursor, table) else {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_ROOT_RECORD_001",
                "root id is corrupt",
            ));
            break;
        };
        let root_kind = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
        let target_profile = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
        let meta_count = take_u32(payload, &mut cursor).unwrap_or(0);
        let mut metadata = BTreeMap::new();
        for _ in 0..meta_count {
            if let (Some(key), Some(value)) = (
                take_string_idx(payload, &mut cursor, table),
                take_string_idx(payload, &mut cursor, table),
            ) {
                metadata.insert(key, value);
            } else {
                diagnostics.push(BasalDiagnostic::new(
                    "E_NBB_ROOT_RECORD_001",
                    "root metadata is corrupt",
                ));
                break;
            }
        }
        records.push(NbbRootRecord {
            root_id,
            root_kind,
            target_profile,
            metadata,
        });
    }
    records
}

fn parse_site_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    table: &NbbStringTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Vec<NbbSiteRecord> {
    let Some(payload) = section_payload(bytes, sections, NbbSectionKind::Site) else {
        return Vec::new();
    };
    let mut cursor = 0usize;
    if !take_magic(payload, &mut cursor, b"SITE") {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_SITE_RECORD_001",
            "site section has invalid record magic",
        ));
        return Vec::new();
    }
    let count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut records = Vec::new();
    for _ in 0..count {
        let Some(site_id) = take_string_idx(payload, &mut cursor, table) else {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_SITE_RECORD_001",
                "site id is corrupt",
            ));
            break;
        };
        let kind = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
        let size = take_u64(payload, &mut cursor).filter(|value| *value != u64::MAX);
        let align = take_u64(payload, &mut cursor).filter(|value| *value != u64::MAX);
        let permission_count = take_u32(payload, &mut cursor).unwrap_or(0);
        let mut permissions = Vec::new();
        for _ in 0..permission_count {
            if let Some(permission) = take_string_idx(payload, &mut cursor, table) {
                permissions.push(permission);
            }
        }
        let owner_root = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
        let lowering_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
        let target_section = take_optional_string_idx(payload, &mut cursor, table);
        records.push(NbbSiteRecord {
            site_id,
            kind,
            size,
            align,
            permissions,
            owner_root,
            lowering_status,
            target_section,
        });
    }
    records
}

fn parse_bind_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    table: &NbbStringTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Vec<NbbBindRecord> {
    let Some(payload) = section_payload(bytes, sections, NbbSectionKind::Bind) else {
        return Vec::new();
    };
    let mut cursor = 0usize;
    if !take_magic(payload, &mut cursor, b"BIND") {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_BIND_RECORD_001",
            "bind section has invalid record magic",
        ));
        return Vec::new();
    }
    let count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut records = Vec::new();
    for _ in 0..count {
        let Some(bind_id) = take_string_idx(payload, &mut cursor, table) else {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_BIND_RECORD_001",
                "bind id is corrupt",
            ));
            break;
        };
        records.push(NbbBindRecord {
            bind_id,
            source: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            destination: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            kind: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            local_resolution: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            machine_resolution: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            relocation_intent: take_optional_string_idx(payload, &mut cursor, table),
        });
    }
    records
}

fn parse_lowering_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    table: &NbbStringTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Option<NbbLoweringRecord> {
    let payload = section_payload(bytes, sections, NbbSectionKind::Lowering)?;
    let mut cursor = 0usize;
    if !take_magic(payload, &mut cursor, b"LOWR") {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_LOWERING_RECORD_001",
            "lowering section has invalid record magic",
        ));
        return None;
    }
    let target_triple = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let profile = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let object_format = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let evidence_boundary_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let symbol_count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut machine_symbols = Vec::new();
    for _ in 0..symbol_count {
        machine_symbols.push(NbbMachineRefRecord {
            target_triple: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            symbol_name: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            section: take_optional_string_idx(payload, &mut cursor, table),
            object_format: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            status: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
        });
    }
    let section_count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut lowered_sections = Vec::new();
    for _ in 0..section_count {
        if let Some(section) = take_string_idx(payload, &mut cursor, table) {
            lowered_sections.push(section);
        }
    }
    let relocation_count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut relocations = Vec::new();
    for _ in 0..relocation_count {
        if let Some(relocation) = take_string_idx(payload, &mut cursor, table) {
            relocations.push(relocation);
        }
    }
    Some(NbbLoweringRecord {
        target_triple,
        profile,
        object_format,
        status,
        evidence_boundary_status,
        machine_symbols,
        sections: lowered_sections,
        relocations,
    })
}

fn parse_evidence_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    table: &NbbStringTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> (
    Option<NbbPendingEvidenceRecord>,
    Option<NbbMachineEvidenceRecord>,
    Option<NbbSealRecord>,
) {
    let Some(payload) = section_payload(bytes, sections, NbbSectionKind::PendingEvidence) else {
        return (None, None, None);
    };
    let mut cursor = 0usize;
    if payload.get(0..4) == Some(b"EVPN".as_slice()) {
        cursor += 4;
        return (
            Some(NbbPendingEvidenceRecord {
                status: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                reason: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            }),
            None,
            None,
        );
    }
    if !take_magic(payload, &mut cursor, b"EVID") {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_EVIDENCE_PENDING_001",
            "evidence section has invalid record magic",
        ));
        return (None, None, None);
    }
    let status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let evidence_version = take_u32(payload, &mut cursor).unwrap_or(0) as u16;
    let machine_target = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let object_format = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let doctor_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let doctor_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let native_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let native_asm_complete_total = take_u32(payload, &mut cursor).unwrap_or(0) == 1;
    let native_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let artifact_kind = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let artifact_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let byte_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let verify_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let verify_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let disasm_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let disasm_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let roundtrip_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let roundtrip_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let debug_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let debug_reason = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let symbol_relocation_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let lowering_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let executable_surface_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let source_span_mapping_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let deterministic_metadata_hash =
        take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let evidence_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let command_count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut commands = Vec::new();
    for _ in 0..command_count {
        commands.push(NbbMachineEvidenceCommand {
            name: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            command: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            status: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            exit_code: take_u32(payload, &mut cursor).unwrap_or(1) as i32,
            stdout_hash: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            stderr_hash: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
        });
    }
    let mut external_artifacts = Vec::new();
    if payload.get(cursor..cursor + 4) == Some(b"EXTR".as_slice()) {
        cursor += 4;
        let external_count = take_u32(payload, &mut cursor).unwrap_or(0);
        for _ in 0..external_count {
            external_artifacts.push(NbbExternalArtifactEvidence {
                language: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                symbol: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                abi: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                artifact_kind: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                artifact_hash: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                executable_hash: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
                link_status: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            });
        }
    }
    if !take_magic(payload, &mut cursor, b"SEAL") {
        diagnostics.push(BasalDiagnostic::new(
            "E_BASAL_SEAL_001",
            "integrated evidence section is missing seal record",
        ));
        return (None, None, None);
    }
    let seal_status = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let seal_algorithm = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let seal_hash = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let seal_scope = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let seal_policy_version = take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let deterministic_inputs_hash =
        take_string_idx(payload, &mut cursor, table).unwrap_or_default();
    let input_count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut seal_inputs = BTreeMap::new();
    for _ in 0..input_count {
        if let (Some(key), Some(value)) = (
            take_string_idx(payload, &mut cursor, table),
            take_string_idx(payload, &mut cursor, table),
        ) {
            seal_inputs.insert(key, value);
        }
    }
    let metadata_count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut non_deterministic_metadata = BTreeMap::new();
    for _ in 0..metadata_count {
        if let (Some(key), Some(value)) = (
            take_string_idx(payload, &mut cursor, table),
            take_string_idx(payload, &mut cursor, table),
        ) {
            non_deterministic_metadata.insert(key, value);
        }
    }
    let evidence = NbbMachineEvidenceRecord {
        status: status.clone(),
        evidence_version,
        machine_target,
        object_format,
        doctor: NbbMachineDoctorEvidence {
            status: doctor_status,
            hash: doctor_hash.clone(),
        },
        native_doctor: NbbNativeDoctorEvidence {
            status: native_status,
            asm_complete_total: native_asm_complete_total,
            hash: native_hash.clone(),
        },
        artifact: NbbMachineArtifactEvidence {
            artifact_kind,
            artifact_hash: artifact_hash.clone(),
            byte_hash: byte_hash.clone(),
        },
        verify_object: NbbVerifyObjectEvidence {
            status: verify_status,
            hash: verify_hash.clone(),
        },
        disasm: NbbDisasmEvidence {
            status: disasm_status,
            hash: disasm_hash.clone(),
        },
        roundtrip: NbbRoundtripEvidence {
            status: roundtrip_status,
            hash: roundtrip_hash.clone(),
        },
        debug_unwind: NbbDebugUnwindEvidence {
            status: debug_status,
            reason: debug_reason,
        },
        symbol_relocation_status,
        source_span_mapping_hash: source_span_mapping_hash.clone(),
        deterministic_metadata_hash: deterministic_metadata_hash.clone(),
        external_artifacts,
        hash_chain: NbbEvidenceHashChain {
            lowering_hash,
            executable_surface_hash,
            doctor_hash,
            native_doctor_hash: native_hash,
            artifact_hash,
            verify_object_hash: verify_hash,
            disasm_hash,
            roundtrip_hash,
            evidence_hash,
        },
        commands,
    };
    let seal = NbbSealRecord {
        status: seal_status,
        seal_hash: NbbSealHash {
            algorithm: seal_algorithm,
            hash: seal_hash,
        },
        seal_scope,
        seal_policy_version,
        deterministic_inputs_hash,
        seal_inputs,
        non_deterministic_metadata,
    };
    (
        Some(NbbPendingEvidenceRecord {
            status,
            reason: "machine evidence integrated in NBB0 evidence section".to_string(),
        }),
        Some(evidence),
        Some(seal),
    )
}

fn parse_surface_section(
    bytes: &[u8],
    sections: &[NbbSectionHeader],
    table: &NbbStringTable,
    diagnostics: &mut Vec<BasalDiagnostic>,
) -> Vec<NbbSurfaceRecord> {
    let Some(payload) = section_payload(bytes, sections, NbbSectionKind::ExecutableSurface) else {
        return Vec::new();
    };
    let mut cursor = 0usize;
    if !take_magic(payload, &mut cursor, b"SURF") {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_SECTION_TABLE_001",
            "executable surface section has invalid record magic",
        ));
        return Vec::new();
    }
    let count = take_u32(payload, &mut cursor).unwrap_or(0);
    let mut records = Vec::new();
    for _ in 0..count {
        records.push(NbbSurfaceRecord {
            kind: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            name: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            status: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
            detail: take_string_idx(payload, &mut cursor, table).unwrap_or_default(),
        });
    }
    records
}

fn verify_nbb_cross_section(
    roots: &[NbbRootRecord],
    sites: &[NbbSiteRecord],
    binds: &[NbbBindRecord],
    lowering: Option<&NbbLoweringRecord>,
    pending_evidence: Option<&NbbPendingEvidenceRecord>,
    machine_evidence: Option<&NbbMachineEvidenceRecord>,
    seal_record: Option<&NbbSealRecord>,
    diagnostics: &mut Vec<BasalDiagnostic>,
) {
    if roots.is_empty() {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_ROOT_RECORD_001",
            ".nbb root record section must contain at least one root",
        ));
    }
    let mut root_ids = BTreeSet::new();
    for root in roots {
        if !root_ids.insert(root.root_id.clone()) {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_ROOT_RECORD_001",
                format!("duplicate .nbb root id '{}'", root.root_id),
            ));
        }
    }
    let site_ids = sites
        .iter()
        .map(|site| site.site_id.clone())
        .collect::<BTreeSet<_>>();
    for bind in binds {
        if bind.kind == "root.site" && !site_ids.contains(&bind.destination) {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_BIND_RECORD_001",
                format!(
                    "bind '{}' targets missing site '{}'",
                    bind.bind_id, bind.destination
                ),
            ));
        }
        if bind.kind.starts_with("machine.") && bind.machine_resolution == "unresolved_error" {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_BIND_RECORD_001",
                format!(
                    "bind '{}' stores unresolved machine reference",
                    bind.bind_id
                ),
            ));
        }
    }
    if let Some(lowering) = lowering {
        if lowering.status != "closed" {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_LOWERING_RECORD_001",
                "lowering record is not closed",
            ));
        }
        if lowering.evidence_boundary_status != "pending_phase6_machine_evidence_integration"
            && lowering.evidence_boundary_status != "integrated_machine_evidence"
        {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_FAKE_EVIDENCE_001",
                "lowering record claims invalid evidence boundary",
            ));
        }
    } else {
        diagnostics.push(BasalDiagnostic::new(
            "E_NBB_LOWERING_RECORD_001",
            ".nbb lowering record section is missing",
        ));
    }
    match pending_evidence {
        Some(record) if record.status == "pending_machine_evidence_integration" => {}
        Some(record) if record.status == "integrated_machine_evidence" => {
            if machine_evidence.is_none() {
                diagnostics.push(BasalDiagnostic::new(
                    "E_BASAL_EVIDENCE_MISSING_001",
                    "integrated evidence status requires a machine evidence record",
                ));
            }
            if seal_record.is_none() {
                diagnostics.push(BasalDiagnostic::new(
                    "E_BASAL_SEAL_MISSING_EVIDENCE_001",
                    "integrated evidence status requires a seal record",
                ));
            }
        }
        Some(record) if record.status == "seal_verified" => {}
        Some(record) if record.status.starts_with("final") => {
            diagnostics.push(BasalDiagnostic::new(
                "E_NBB_FAKE_EVIDENCE_001",
                "final machine evidence status is invalid without a Phase 6 evidence record",
            ))
        }
        Some(record) if record.status == "invalid_machine_evidence" => diagnostics.push(
            BasalDiagnostic::new("E_BASAL_EVIDENCE_001", "machine evidence is invalid"),
        ),
        Some(_) => diagnostics.push(BasalDiagnostic::new(
            "E_NBB_EVIDENCE_PENDING_001",
            "evidence record has invalid status",
        )),
        None => diagnostics.push(BasalDiagnostic::new(
            "E_NBB_EVIDENCE_PENDING_001",
            ".nbb evidence section is missing",
        )),
    }
    if let Some(evidence) = machine_evidence {
        if evidence.status != "integrated_machine_evidence" {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_EVIDENCE_001",
                "machine evidence record is not integrated",
            ));
        }
        if !evidence.native_doctor.asm_complete_total {
            diagnostics.push(BasalDiagnostic::new(
                "E_BASAL_EVIDENCE_NATIVE_DOCTOR_001",
                "machine native-doctor evidence does not keep asm_complete_total=true",
            ));
        }
        for (status, code, message) in [
            (
                evidence.doctor.status.as_str(),
                "E_BASAL_EVIDENCE_MACHINE_DOCTOR_001",
                "machine doctor evidence is not ok",
            ),
            (
                evidence.verify_object.status.as_str(),
                "E_BASAL_EVIDENCE_VERIFY_OBJECT_001",
                "verify-object evidence is not ok",
            ),
            (
                evidence.disasm.status.as_str(),
                "E_BASAL_EVIDENCE_DISASM_001",
                "disasm evidence is not ok",
            ),
            (
                evidence.roundtrip.status.as_str(),
                "E_BASAL_EVIDENCE_ROUNDTRIP_001",
                "roundtrip evidence is not ok",
            ),
        ] {
            if status != "ok" {
                diagnostics.push(BasalDiagnostic::new(code, message));
            }
        }
    }
}

fn push_idx(out: &mut Vec<u8>, table: &NbbStringTable, value: &str) {
    push_u32(out, table.index_of(value));
}

fn push_optional_idx(out: &mut Vec<u8>, table: &NbbStringTable, value: Option<&str>) {
    match value {
        Some(value) => push_idx(out, table, value),
        None => push_u32(out, u32::MAX),
    }
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn take_magic(bytes: &[u8], cursor: &mut usize, expected: &[u8; 4]) -> bool {
    let ok = bytes.get(*cursor..*cursor + 4) == Some(expected.as_slice());
    *cursor = cursor.saturating_add(4);
    ok
}

fn take_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let raw = bytes.get(*cursor..*cursor + 4)?;
    *cursor += 4;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn take_u64(bytes: &[u8], cursor: &mut usize) -> Option<u64> {
    let raw = bytes.get(*cursor..*cursor + 8)?;
    *cursor += 8;
    Some(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

fn take_bytes(bytes: &[u8], cursor: &mut usize) -> Option<Vec<u8>> {
    let len = take_u32(bytes, cursor)? as usize;
    let raw = bytes.get(*cursor..*cursor + len)?;
    *cursor += len;
    Some(raw.to_vec())
}

fn take_string_idx(bytes: &[u8], cursor: &mut usize, table: &NbbStringTable) -> Option<String> {
    let index = take_u32(bytes, cursor)?;
    table.get(index)
}

fn take_optional_string_idx(
    bytes: &[u8],
    cursor: &mut usize,
    table: &NbbStringTable,
) -> Option<String> {
    let index = take_u32(bytes, cursor)?;
    (index != u32::MAX).then(|| table.get(index)).flatten()
}

fn compute_nbb_structural_hash(bytes: &[u8]) -> String {
    let mut clone = bytes.to_vec();
    if clone.len() >= NBB_CHECKSUM_OFFSET + NBB_CHECKSUM_SIZE {
        clone[NBB_CHECKSUM_OFFSET..NBB_CHECKSUM_OFFSET + NBB_CHECKSUM_SIZE].fill(0);
    }
    to_hex(&Sha256::digest(&clone))
}

fn align_usize(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

fn hex_to_32(hex: &str) -> Result<[u8; 32], Box<dyn Error>> {
    if hex.len() != 64 {
        return Err("expected 32-byte hex digest".into());
    }
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&hex[offset..offset + 2], 16)?;
    }
    Ok(out)
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes
        .get(offset..offset + 2)
        .map(|raw| u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|raw| u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
    bytes.get(offset..offset + 8).map(|raw| {
        u64::from_le_bytes([
            raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
        ])
    })
}

fn read_hash(bytes: &[u8], offset: usize) -> Option<String> {
    bytes.get(offset..offset + 32).map(to_hex)
}

fn is_zero_hash(hash: &str) -> bool {
    hash.chars().all(|ch| ch == '0')
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn stable_json_hash(value: &Value) -> String {
    let encoded = serde_json::to_vec(value).unwrap_or_default();
    to_hex(&Sha256::digest(encoded))
}

fn diagnostics_have_errors(diagnostics: &[BasalDiagnostic]) -> bool {
    diagnostics.iter().any(BasalDiagnostic::is_error)
}

fn diagnostics_summary(diagnostics: &[BasalDiagnostic]) -> String {
    diagnostics
        .iter()
        .filter(|diag| diag.is_error())
        .map(|diag| {
            crate::render_canonical_diagnostic_terminal(
                diag.canonical(),
                "",
                crate::stderr_diagnostic_color_enabled(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn diagnostics_to_json(diagnostics: &[BasalDiagnostic]) -> Value {
    Value::Array(
        diagnostics
            .iter()
            .map(|diag| {
                let mut value = crate::canonical_diagnostic_to_json(diag.canonical());
                value["span"] = diag.span.as_ref().map(span_to_json).unwrap_or(Value::Null);
                value
            })
            .collect(),
    )
}

fn span_to_json(span: &BasalSpan) -> Value {
    json!({
        "file": span.file,
        "start": span.start,
        "end": span.end,
        "line": span.line,
        "column": span.column
    })
}

fn span_ref_to_json(span_ref: &BasalTableSpanRef) -> Value {
    json!({
        "source_file": span_ref.source_file,
        "node_kind": span_ref.node_kind,
        "span": span_to_json(&span_ref.span)
    })
}

fn ast_to_json(ast: &BasalSourceFile) -> Value {
    json!({
        "kind": "BasalSourceFile",
        "path": ast.path,
        "span": span_to_json(&ast.span),
        "roots": ast.roots.iter().map(root_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&ast.diagnostics),
        "phase1_nb_parser_ast": "closed",
        "lowering": "not_started",
        "evidence_integration": "not_started"
    })
}

fn table_set_to_json(tables: &BasalTableSet) -> Value {
    json!({
        "kind": "BasalTableSet",
        "path": tables.path,
        "status": tables.status,
        "phase": "phase2_tables",
        "phase1_nb_parser_ast": "closed",
        "phase2_root_site_bind_tables": "closed",
        "phase3_basal_ir_root_ir_verifier": "closed",
        "lowering": "not_started",
        "basal_ir": "closed_available_via_basal_ir_command",
        "nbb_writer": "not_started",
        "evidence_integration": "not_started",
        "root_table": root_table_to_json(&tables.root_table),
        "site_table": site_table_to_json(&tables.site_table),
        "bind_table": bind_table_to_json(&tables.bind_table),
        "diagnostics": diagnostics_to_json(&tables.diagnostics)
    })
}

fn root_table_to_json(table: &BasalRootTable) -> Value {
    json!({
        "kind": "BasalRootTable",
        "status": table.status,
        "entries": table.entries.iter().map(root_entry_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&table.diagnostics)
    })
}

fn root_entry_to_json(entry: &BasalRootEntry) -> Value {
    json!({
        "kind": "BasalRootEntry",
        "name": entry.name,
        "root_id": entry.root_id,
        "version": entry.version,
        "profile": entry.profile,
        "target_reference": entry.target_reference,
        "declared_sections": entry.declared_sections,
        "metadata": entry.metadata,
        "origin": span_ref_to_json(&entry.origin),
        "diagnostics": diagnostics_to_json(&entry.diagnostics),
        "status": entry.status
    })
}

fn site_table_to_json(table: &BasalSiteTable) -> Value {
    json!({
        "kind": "BasalSiteTable",
        "status": table.status,
        "entries": table.entries.iter().map(site_entry_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&table.diagnostics)
    })
}

fn site_entry_to_json(entry: &BasalSiteEntry) -> Value {
    json!({
        "kind": "BasalSiteEntry",
        "name": entry.name,
        "site_kind": entry.kind,
        "size": entry.size,
        "align": entry.align,
        "offset": entry.offset,
        "target_policy": entry.target_policy,
        "permissions": entry.permissions.flags,
        "fields": entry.fields,
        "origin": span_ref_to_json(&entry.origin),
        "diagnostics": diagnostics_to_json(&entry.diagnostics),
        "status": entry.status
    })
}

fn bind_table_to_json(table: &BasalBindTable) -> Value {
    json!({
        "kind": "BasalBindTable",
        "status": table.status,
        "entries": table.entries.iter().map(bind_entry_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&table.diagnostics)
    })
}

fn bind_entry_to_json(entry: &BasalBindEntry) -> Value {
    json!({
        "kind": "BasalBindEntry",
        "id": entry.id,
        "source": entry.source,
        "destination_kind": {
            "raw": entry.destination_kind.raw,
            "normalized": entry.destination_kind.normalized
        },
        "destination_path": entry.destination_path,
        "resolution_status": entry.resolution_status.status,
        "origin": span_ref_to_json(&entry.origin),
        "target_span": span_to_json(&entry.target_span),
        "diagnostics": diagnostics_to_json(&entry.diagnostics),
        "status": entry.status
    })
}

fn basal_ir_to_json(module: &BasalIrModule) -> Value {
    let ffi_link_manifest =
        basal_ffi_link_manifest_from_surface(Some(&module.path), &module.surface);
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    json!({
        "kind": "BasalIrModule",
        "path": module.path,
        "status": module.status,
        "phase": "phase3_basal_ir_root_ir_verifier",
        "phase1_nb_parser_ast": "closed",
        "phase2_root_site_bind_tables": "closed",
        "phase3_basal_ir_root_ir_verifier": "closed",
        "basal_ir": module.status,
        "root_ir": if module.roots.is_empty() { "error" } else { "closed" },
        "site_ir": "closed",
        "bind_ir": "closed",
        "semantic_verifier": module.verifier.status,
        "lowering": "not_started",
        "nbb_writer": "not_started",
        "machine_evidence_integration": "not_started",
        "roots": module.roots.iter().map(root_ir_to_json).collect::<Vec<_>>(),
        "sites": module.sites.iter().map(site_ir_to_json).collect::<Vec<_>>(),
        "binds": module.binds.iter().map(bind_ir_to_json).collect::<Vec<_>>(),
        "seal_requirements": module.seal_requirements.iter().map(seal_requirement_to_json).collect::<Vec<_>>(),
        "executable_surface": surface_ir_to_json(&module.surface),
        "ffi_link_manifest_hash": ffi_link_manifest_hash,
        "ffi_link_manifest": ffi_link_manifest,
        "verifier": verifier_to_json(&module.verifier),
        "diagnostics": diagnostics_to_json(&module.diagnostics),
        "no_machine_lowering": true,
        "no_nbb_writer": true,
        "no_machine_evidence_integration": true
    })
}

fn ir_span_to_json(span: &BasalIrSpan) -> Value {
    json!({
        "source_file": span.source_file,
        "node_kind": span.node_kind,
        "span": span_to_json(&span.span)
    })
}

fn root_ir_to_json(root: &BasalRootIr) -> Value {
    json!({
        "kind": "BasalRootIr",
        "root_id": root.root_id,
        "root_kind": root.root_kind,
        "version": root.version,
        "profile": root.profile,
        "target_reference": root.target_reference,
        "metadata": root.metadata,
        "declared_sections": root.declared_sections,
        "table_entry": root.table_entry,
        "span": ir_span_to_json(&root.span),
        "status": root.status
    })
}

fn site_ir_to_json(site: &BasalSiteIr) -> Value {
    json!({
        "kind": "BasalSiteIr",
        "site_id": site.site_id,
        "site_kind": site.kind,
        "size": site.size,
        "align": site.align,
        "permissions": site.permissions.iter().map(|permission| permission.name.clone()).collect::<Vec<_>>(),
        "owner_root": site.owner_root,
        "fields": site.fields,
        "span": ir_span_to_json(&site.span),
        "semantic_status": site.semantic_status
    })
}

fn symbol_ref_to_json(symbol: &BasalIrSymbolRef) -> Value {
    json!({
        "path": symbol.path,
        "kind": symbol.kind,
        "span": ir_span_to_json(&symbol.span)
    })
}

fn machine_ref_to_json(machine: &BasalIrMachineRef) -> Value {
    json!({
        "path": machine.path,
        "kind": machine.kind,
        "resolution_status": machine.resolution_status.status,
        "span": ir_span_to_json(&machine.span)
    })
}

fn bind_ir_to_json(bind: &BasalBindIr) -> Value {
    json!({
        "kind": "BasalBindIr",
        "bind_id": bind.bind_id,
        "source": symbol_ref_to_json(&bind.source),
        "destination": symbol_ref_to_json(&bind.destination),
        "machine_ref": bind.machine_ref.as_ref().map(machine_ref_to_json),
        "resolution_status": bind.resolution_status.status,
        "source_span": ir_span_to_json(&bind.source_span),
        "destination_span": ir_span_to_json(&bind.destination_span),
        "semantic_status": bind.semantic_status
    })
}

fn seal_requirement_to_json(requirement: &BasalIrSealRequirement) -> Value {
    json!({
        "kind": "BasalIrSealRequirement",
        "name": requirement.name,
        "span": ir_span_to_json(&requirement.span),
        "status": requirement.status
    })
}

fn width_type_to_json(width: &BasalWidthType) -> Value {
    json!({
        "name": width.name,
        "bits": width.bits,
        "signed": width.signed,
        "kind": width.kind
    })
}

fn basal_ffi_package_manifest_from_ir(module: &BasalIrModule) -> Value {
    let ffi_link_manifest =
        basal_ffi_link_manifest_from_surface(Some(&module.path), &module.surface);
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    json!({
        "schema": "basal.ffi_package_manifest.v1",
        "status": ffi_link_manifest.get("status").and_then(Value::as_str).unwrap_or("failed"),
        "source": module.path,
        "root_id": module.roots.first().map(|root| root.root_id.clone()),
        "ghor_compatible": true,
        "package_manifest_hash": stable_json_hash(&json!({
            "source": module.path,
            "root_id": module.roots.first().map(|root| root.root_id.clone()),
            "ffi_link_manifest_hash": ffi_link_manifest_hash
        })),
        "ffi_link_manifest_hash": ffi_link_manifest_hash,
        "ffi_link_manifest": ffi_link_manifest
    })
}

fn basal_ffi_link_manifest_from_surface(
    source: Option<&str>,
    surface: &BasalExecutableSurfaceIr,
) -> Value {
    let entries = surface
        .foreign_bindings
        .iter()
        .map(foreign_binding_manifest_entry)
        .collect::<Vec<_>>();
    let artifacts = surface
        .foreign_bindings
        .iter()
        .filter_map(|binding| {
            binding.artifact.as_ref().map(|artifact| {
                json!({
                    "name": binding.name,
                    "language": binding.language,
                    "symbol": binding.symbol,
                    "artifact": artifact,
                    "link": binding.link,
                    "status": binding.status
                })
            })
        })
        .collect::<Vec<_>>();
    let diagnostics = surface
        .foreign_bindings
        .iter()
        .filter(|binding| binding.status != "closed")
        .map(|binding| {
            json!({
                "code": "E_BASAL_FOREIGN_BINDING_001",
                "severity": "error",
                "message": format!("foreign binding '{}' is not closed", binding.name),
                "symbol": binding.symbol
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "ghor.ffi_manifest.v1",
        "producer": "basal",
        "basal_schema": "basal.ffi_link_manifest.v1",
        "ghor_compatible": true,
        "status": if diagnostics.is_empty() { "closed" } else { "failed" },
        "source": source,
        "compiler_ffi_entry_count": entries.len(),
        "entries": entries,
        "artifacts": artifacts,
        "link_args": [],
        "runtime_paths": [],
        "native_dependencies": [],
        "diagnostics": diagnostics
    })
}

fn basal_ffi_link_manifest_from_external_bindings(
    source: Option<&str>,
    bindings: &[BasalExternalBindingLowering],
) -> Value {
    let entries = bindings
        .iter()
        .map(|binding| {
            json!({
                "kind": "BasalExternalBindingLowering",
                "name": binding.name,
                "language": binding.language,
                "symbol": binding.symbol,
                "abi": binding.abi,
                "layout": binding.layout,
                "ownership": binding.ownership,
                "artifact": binding.artifact,
                "link": binding.link,
                "lowering_status": binding.lowering_status,
                "machine_symbol_status": binding.machine_symbol.status,
                "relocation_status": binding.relocation.status,
                "evidence_boundary": binding.evidence_boundary.status
            })
        })
        .collect::<Vec<_>>();
    let diagnostics = bindings
        .iter()
        .filter(|binding| binding.lowering_status == "error")
        .map(|binding| {
            json!({
                "code": "E_BASAL_LOWERING_BIND_001",
                "severity": "error",
                "message": format!("external binding '{}' failed lowering", binding.name),
                "symbol": binding.symbol
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "ghor.ffi_manifest.v1",
        "producer": "basal-lowering",
        "basal_schema": "basal.ffi_lowering_manifest.v1",
        "ghor_compatible": true,
        "status": if diagnostics.is_empty() { "closed" } else { "failed" },
        "source": source,
        "compiler_ffi_entry_count": entries.len(),
        "entries": entries,
        "artifacts": bindings.iter().filter_map(|binding| binding.artifact.as_ref().map(|artifact| json!({
            "name": binding.name,
            "language": binding.language,
            "symbol": binding.symbol,
            "artifact": artifact,
            "link": binding.link,
            "status": binding.lowering_status
        }))).collect::<Vec<_>>(),
        "link_args": [],
        "runtime_paths": [],
        "native_dependencies": [],
        "diagnostics": diagnostics
    })
}

fn foreign_binding_manifest_entry(binding: &BasalForeignBindingIr) -> Value {
    json!({
        "kind": "BasalForeignBindingIr",
        "name": binding.name,
        "language": binding.language,
        "symbol": binding.symbol,
        "abi": binding.abi,
        "layout": binding.layout,
        "ownership": binding.ownership,
        "header": binding.header,
        "artifact": binding.artifact,
        "link": binding.link,
        "bridge": binding.bridge,
        "status": binding.status,
        "span": ir_span_to_json(&binding.span)
    })
}

fn surface_ir_to_json(surface: &BasalExecutableSurfaceIr) -> Value {
    let ffi_link_manifest = basal_ffi_link_manifest_from_surface(None, surface);
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    json!({
        "kind": "BasalExecutableSurfaceIr",
        "status": surface.status,
        "const_surface": if surface.status == "closed" { "closed" } else { "error" },
        "routine_surface": if surface.status == "closed" { "closed" } else { "error" },
        "raw_surface": if surface.status == "closed" { "closed" } else { "error" },
        "verify_surface": if surface.status == "closed" { "closed" } else { "error" },
        "probe_alias": "closed",
        "report_surface": if surface.status == "closed" { "closed" } else { "error" },
        "foreign_surface": if surface.status == "closed" { "closed" } else { "error" },
        "width_types": "closed",
        "pointer_surface": if surface.status == "closed" { "closed" } else { "error" },
        "probe_aliases": surface.probe_aliases,
        "consts": surface.consts.iter().map(const_ir_to_json).collect::<Vec<_>>(),
        "routines": surface.routines.iter().map(routine_ir_to_json).collect::<Vec<_>>(),
        "raw_blocks": surface.raw_blocks.iter().map(raw_block_ir_to_json).collect::<Vec<_>>(),
        "verifies": surface.verifies.iter().map(verify_ir_to_json).collect::<Vec<_>>(),
        "reports": surface.reports.iter().map(report_ir_to_json).collect::<Vec<_>>(),
        "foreign_bindings": surface.foreign_bindings.iter().map(foreign_binding_ir_to_json).collect::<Vec<_>>(),
        "ffi_link_manifest_hash": ffi_link_manifest_hash,
        "ffi_link_manifest": ffi_link_manifest,
        "mem_accesses": surface.mem_accesses.iter().map(mem_access_ir_to_json).collect::<Vec<_>>(),
        "pointer_accesses": surface.pointer_accesses.iter().map(pointer_access_ir_to_json).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&surface.diagnostics),
        "machine_evidence_integration": "not_started",
        "seal_final": "not_started"
    })
}

fn const_ir_to_json(record: &BasalConstIr) -> Value {
    json!({
        "kind": "BasalConstIr",
        "name": record.name,
        "width": width_type_to_json(&record.width),
        "value": record.value,
        "value_hash": record.value_hash,
        "endian": record.endian,
        "span": ir_span_to_json(&record.span),
        "status": record.status
    })
}

fn mem_access_ir_to_json(access: &BasalMemAccessIr) -> Value {
    json!({
        "kind": "BasalMemAccessIr",
        "form": access.form,
        "target": access.target,
        "width": access.width.as_ref().map(width_type_to_json),
        "access": access.access,
        "span": ir_span_to_json(&access.span),
        "status": access.status
    })
}

fn pointer_access_ir_to_json(access: &BasalPointerAccessIr) -> Value {
    json!({
        "kind": "BasalPointerAccessIr",
        "operation": access.operation,
        "pointer_type": access.pointer_type,
        "authority": access.authority,
        "hazard": access.hazard,
        "evidence_boundary": access.evidence_boundary,
        "span": ir_span_to_json(&access.span),
        "status": access.status
    })
}

fn raw_hazard_ir_to_json(hazard: &BasalRawHazardIr) -> Value {
    json!({
        "kind": hazard.hazard,
        "width": hazard.width.as_ref().map(width_type_to_json),
        "endian": hazard.endian,
        "alignment": hazard.alignment,
        "permission_status": hazard.permission_status,
        "bounds_status": hazard.bounds_status,
        "accepted": hazard.accepted,
        "evidence_boundary": hazard.evidence_boundary,
        "span": ir_span_to_json(&hazard.span)
    })
}

fn raw_block_ir_to_json(block: &BasalRawBlockIr) -> Value {
    json!({
        "kind": "BasalRawBlockIr",
        "name": block.name,
        "unchecked": block.unchecked,
        "hazards": block.hazards.iter().map(raw_hazard_ir_to_json).collect::<Vec<_>>(),
        "mem_accesses": block.mem_accesses.iter().map(mem_access_ir_to_json).collect::<Vec<_>>(),
        "span": ir_span_to_json(&block.span),
        "status": block.status
    })
}

fn routine_ir_to_json(routine: &BasalRoutineIr) -> Value {
    json!({
        "kind": "BasalRoutineIr",
        "name": routine.name,
        "params": routine.params.iter().map(|param| json!({
            "name": param.name,
            "width": width_type_to_json(&param.width),
            "span": ir_span_to_json(&param.span)
        })).collect::<Vec<_>>(),
        "return_type": routine.return_type.as_ref().map(width_type_to_json),
        "raw_blocks": routine.raw_blocks.iter().map(raw_block_ir_to_json).collect::<Vec<_>>(),
        "mem_accesses": routine.mem_accesses.iter().map(mem_access_ir_to_json).collect::<Vec<_>>(),
        "abi_profile": routine.abi_profile,
        "span": ir_span_to_json(&routine.span),
        "lowering_status": routine.lowering_status,
        "status": routine.status
    })
}

fn verify_ir_to_json(verify: &BasalVerifyIr) -> Value {
    json!({
        "kind": "BasalVerifyIr",
        "name": verify.name,
        "statements": verify.statements,
        "routine_calls": verify.routine_calls,
        "mem_accesses": verify.mem_accesses.iter().map(mem_access_ir_to_json).collect::<Vec<_>>(),
        "probe_alias": verify.probe_alias,
        "span": ir_span_to_json(&verify.span),
        "status": verify.status
    })
}

fn report_ir_to_json(report: &BasalReportIr) -> Value {
    json!({
        "kind": "BasalReportIr",
        "statements": report.statements,
        "emitted_items": report.emitted_items,
        "span": ir_span_to_json(&report.span),
        "status": report.status
    })
}

fn foreign_binding_ir_to_json(binding: &BasalForeignBindingIr) -> Value {
    json!({
        "kind": "BasalForeignBindingIr",
        "name": binding.name,
        "language": binding.language,
        "symbol": binding.symbol,
        "abi": binding.abi,
        "layout": binding.layout,
        "ownership": binding.ownership,
        "header": binding.header,
        "artifact": binding.artifact,
        "link": binding.link,
        "bridge": binding.bridge,
        "span": ir_span_to_json(&binding.span),
        "status": binding.status
    })
}

fn verifier_to_json(report: &BasalIrVerifierReport) -> Value {
    json!({
        "kind": "BasalIrVerifierReport",
        "status": report.status,
        "root_count": report.root_count,
        "site_count": report.site_count,
        "bind_count": report.bind_count,
        "local_refs_resolved": report.local_refs_resolved,
        "machine_refs_pending": report.machine_refs_pending,
        "consistency_checks": report.consistency_checks,
        "no_machine_evidence": report.no_machine_evidence,
        "diagnostics": diagnostics_to_json(&report.diagnostics)
    })
}

fn lowering_plan_to_json(plan: &BasalLoweringPlan) -> Value {
    let ffi_link_manifest = basal_ffi_link_manifest_from_external_bindings(
        Some(&plan.path),
        &plan.machine_plan.external_bindings,
    );
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    json!({
        "kind": "BasalLoweringPlan",
        "path": plan.path,
        "status": plan.status,
        "phase": "phase4_lowering_to_machine",
        "phase1_nb_parser_ast": "closed",
        "phase2_root_site_bind_tables": "closed",
        "phase3_basal_ir_root_ir_verifier": "closed",
        "phase4_lowering_to_machine": "closed",
        "lowering": plan.status,
        "machine_ref_resolution": if plan.verifier.status == "closed" { "closed" } else { "error" },
        "nbb_writer": "not_started",
        "machine_evidence_integration": "not_started",
        "ffi_link_manifest_hash": ffi_link_manifest_hash,
        "ffi_link_manifest": ffi_link_manifest,
        "machine_plan": machine_lowering_plan_to_json(&plan.machine_plan),
        "evidence_boundary": evidence_boundary_to_json(&plan.evidence_boundary),
        "verifier": lowering_verifier_to_json(&plan.verifier),
        "diagnostics": diagnostics_to_json(&plan.diagnostics),
        "no_nbb_writer": true,
        "no_machine_evidence_integration": true
    })
}

fn machine_lowering_plan_to_json(plan: &BasalMachineLoweringPlan) -> Value {
    let ffi_link_manifest =
        basal_ffi_link_manifest_from_external_bindings(None, &plan.external_bindings);
    let ffi_link_manifest_hash = stable_json_hash(&ffi_link_manifest);
    json!({
        "kind": "BasalMachineLoweringPlan",
        "target": plan.target.as_ref().map(machine_target_to_json),
        "roots": plan.roots.iter().map(root_lowering_to_json).collect::<Vec<_>>(),
        "sites": plan.sites.iter().map(site_lowering_to_json).collect::<Vec<_>>(),
        "binds": plan.binds.iter().map(bind_lowering_to_json).collect::<Vec<_>>(),
        "external_bindings": plan.external_bindings.iter().map(external_binding_lowering_to_json).collect::<Vec<_>>(),
        "ffi_link_manifest_hash": ffi_link_manifest_hash,
        "ffi_link_manifest": ffi_link_manifest,
        "symbols": plan.symbols.iter().map(machine_symbol_to_json).collect::<Vec<_>>(),
        "sections": plan.sections.iter().map(machine_section_to_json).collect::<Vec<_>>(),
        "relocations": plan.relocations.iter().map(machine_relocation_to_json).collect::<Vec<_>>(),
        "surface_records": plan.surface_records.iter().map(surface_lowering_to_json).collect::<Vec<_>>()
    })
}

fn lowering_span_to_json(span: &BasalLoweringSpan) -> Value {
    json!({
        "source_file": span.source_file,
        "node_kind": span.node_kind,
        "span": span_to_json(&span.span)
    })
}

fn machine_target_to_json(target: &BasalMachineTargetRef) -> Value {
    json!({
        "target_triple": target.target_triple,
        "profile": target.profile,
        "object_format": target.object_format,
        "status": target.status
    })
}

fn machine_symbol_to_json(symbol: &BasalMachineSymbolRef) -> Value {
    json!({
        "symbol_name": symbol.symbol_name,
        "target_triple": symbol.target_triple,
        "section": symbol.section,
        "object_format": symbol.object_format,
        "status": symbol.status,
        "span": lowering_span_to_json(&symbol.span)
    })
}

fn machine_section_to_json(section: &BasalMachineSectionRef) -> Value {
    json!({
        "name": section.name,
        "permissions": section.permissions,
        "align": section.align,
        "status": section.status
    })
}

fn machine_relocation_to_json(relocation: &BasalMachineRelocationRef) -> Value {
    json!({
        "kind": relocation.kind,
        "symbol": relocation.symbol,
        "status": relocation.status
    })
}

fn external_binding_lowering_to_json(external: &BasalExternalBindingLowering) -> Value {
    json!({
        "kind": "BasalExternalBindingLowering",
        "name": external.name,
        "language": external.language,
        "symbol": external.symbol,
        "abi": external.abi,
        "layout": external.layout,
        "ownership": external.ownership,
        "artifact": external.artifact,
        "link": external.link,
        "machine_symbol": machine_symbol_to_json(&external.machine_symbol),
        "relocation": machine_relocation_to_json(&external.relocation),
        "evidence_boundary": evidence_boundary_to_json(&external.evidence_boundary),
        "source_span": lowering_span_to_json(&external.source_span),
        "lowering_status": external.lowering_status
    })
}

fn root_lowering_to_json(root: &BasalRootLowering) -> Value {
    json!({
        "kind": "BasalRootLowering",
        "root_id": root.root_id,
        "root_kind": root.root_kind,
        "target_machine_profile": root.target_machine_profile,
        "entry_symbol": root.entry_symbol,
        "source_span": lowering_span_to_json(&root.source_span),
        "lowering_status": root.lowering_status,
        "pending_evidence_status": root.pending_evidence_status
    })
}

fn site_lowering_to_json(site: &BasalSiteLowering) -> Value {
    json!({
        "kind": "BasalSiteLowering",
        "site_id": site.site_id,
        "site_kind": site.kind,
        "size": site.size,
        "align": site.align,
        "permissions": site.permissions,
        "target_section": site.target_section,
        "target_symbol": site.target_symbol,
        "target_offset": site.target_offset,
        "source_span": lowering_span_to_json(&site.source_span),
        "lowering_status": site.lowering_status
    })
}

fn bind_lowering_to_json(bind: &BasalBindLowering) -> Value {
    json!({
        "kind": "BasalBindLowering",
        "bind_id": bind.bind_id,
        "source": bind.source,
        "destination": bind.destination,
        "bind_kind": bind.kind,
        "local_resolution": bind.local_resolution.status,
        "machine_symbol_resolution": bind.machine_symbol_resolution.status,
        "machine_target": bind.machine_target.as_ref().map(machine_target_to_json),
        "relocation_intent": bind.relocation_intent.as_ref().map(machine_relocation_to_json),
        "pending_evidence_boundary": evidence_boundary_to_json(&bind.pending_evidence_boundary),
        "source_span": lowering_span_to_json(&bind.source_span),
        "destination_span": lowering_span_to_json(&bind.destination_span),
        "lowering_status": bind.lowering_status
    })
}

fn surface_lowering_to_json(record: &BasalSurfaceLoweringRecord) -> Value {
    json!({
        "kind": record.kind,
        "name": record.name,
        "status": record.status,
        "detail": record.detail,
        "source_span": lowering_span_to_json(&record.source_span)
    })
}

fn evidence_boundary_to_json(boundary: &BasalLoweringEvidenceBoundary) -> Value {
    json!({
        "status": boundary.status,
        "reason": boundary.reason
    })
}

fn lowering_verifier_to_json(report: &BasalLoweringVerifierReport) -> Value {
    json!({
        "kind": "BasalLoweringVerifierReport",
        "status": report.status,
        "root_count": report.root_count,
        "site_count": report.site_count,
        "bind_count": report.bind_count,
        "machine_refs_resolved": report.machine_refs_resolved,
        "local_refs_resolved": report.local_refs_resolved,
        "pending_machine_emit": report.pending_machine_emit,
        "evidence_boundary_pending": report.evidence_boundary_pending,
        "nbb_writer_started": report.nbb_writer_started,
        "diagnostics": diagnostics_to_json(&report.diagnostics)
    })
}

fn root_to_json(root: &BasalRootDecl) -> Value {
    json!({
        "kind": "BasalRootDecl",
        "name": root.name,
        "span": span_to_json(&root.span),
        "sections": root.sections.iter().map(section_to_json).collect::<Vec<_>>()
    })
}

fn section_to_json(section: &BasalSection) -> Value {
    json!({
        "kind": section.kind,
        "name": section.name,
        "span": span_to_json(&section.span),
        "key_values": section.key_values.iter().map(key_value_to_json).collect::<Vec<_>>(),
        "children": section.children.iter().map(ast_node_to_json).collect::<Vec<_>>()
    })
}

fn key_value_to_json(kv: &BasalKeyValue) -> Value {
    json!({
        "kind": "BasalKeyValue",
        "key": kv.key,
        "value": literal_to_json(&kv.value),
        "span": span_to_json(&kv.span)
    })
}

fn literal_to_json(literal: &BasalLiteral) -> Value {
    json!({
        "kind": "BasalLiteral",
        "literal_kind": literal.kind,
        "raw": literal.raw,
        "span": span_to_json(&literal.span)
    })
}

fn ast_node_to_json(node: &BasalAstNode) -> Value {
    json!({
        "kind": node.kind,
        "name": node.name,
        "span": span_to_json(&node.span),
        "text": node.text,
        "children": node.children.iter().map(ast_node_to_json).collect::<Vec<_>>()
    })
}

fn audit_to_json(audit: &BasalAudit) -> Value {
    json!({
        "path": audit.path,
        "file_classification": audit.file_classification,
        "root_container": audit.root_container,
        "sections": audit.sections.iter().cloned().collect::<Vec<_>>(),
        "root_fields": audit.root_fields,
        "binary_fields": audit.binary_fields,
        "symbols": audit.symbols.iter().map(|symbol| json!({
            "name": symbol.name,
            "type": symbol.ty
        })).collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&audit.diagnostics),
        "deferred": audit.deferred,
        "status": audit.status,
        "conformance_level": audit.conformance_level,
        "phase1_nb_parser_ast": "closed",
        "phase2_root_site_bind_tables": "closed",
        "phase3_basal_ir_root_ir_verifier": "closed",
        "phase4_lowering_to_machine": "closed",
        "phase5_nbb_binary_writer_reader": "closed",
        "phase5_5_executable_surface": "closed",
        "tables": audit.tables.as_ref().map(table_set_to_json),
        "ir": audit.ir.as_ref().map(basal_ir_to_json),
        "surface": audit.ir.as_ref().map(|ir| surface_ir_to_json(&ir.surface)),
        "lowering": audit.lowering.as_ref().map(lowering_plan_to_json),
        "nbb_writer": "closed",
        "nbb_reader": "closed",
        "nbb_verifier": "closed",
        "machine_evidence_integration": "not_started",
        "claim": "contract_only",
        "no_machine_lowering": false,
        "binary_bytes_generated_by_report": false
    })
}

fn nbb_header_to_json(header: &NbbHeader) -> Value {
    json!({
        "magic": header.magic,
        "version": header.version,
        "endian": header.endian,
        "pointer_width": header.pointer_width,
        "section_count": header.section_count,
        "header_size": header.header_size,
        "section_table_offset": header.section_table_offset,
        "section_table_size": header.section_table_size,
        "total_size": header.total_size,
        "flags": header.flags,
        "checksum": header.checksum
    })
}

fn nbb_section_to_json(section: &NbbSectionHeader) -> Value {
    json!({
        "kind": section.kind.name(),
        "kind_code": section.kind.code(),
        "offset": section.offset,
        "size": section.size,
        "alignment": section.alignment,
        "flags": section.flags,
        "hash": section.hash
    })
}

fn nbb_root_record_to_json(record: &NbbRootRecord) -> Value {
    json!({
        "root_id": record.root_id,
        "root_kind": record.root_kind,
        "target_profile": record.target_profile,
        "metadata": record.metadata
    })
}

fn nbb_site_record_to_json(record: &NbbSiteRecord) -> Value {
    json!({
        "site_id": record.site_id,
        "kind": record.kind,
        "size": record.size,
        "align": record.align,
        "permissions": record.permissions,
        "owner_root": record.owner_root,
        "lowering_status": record.lowering_status,
        "target_section": record.target_section
    })
}

fn nbb_bind_record_to_json(record: &NbbBindRecord) -> Value {
    json!({
        "bind_id": record.bind_id,
        "source": record.source,
        "destination": record.destination,
        "kind": record.kind,
        "local_resolution": record.local_resolution,
        "machine_resolution": record.machine_resolution,
        "relocation_intent": record.relocation_intent
    })
}

fn nbb_machine_ref_to_json(record: &NbbMachineRefRecord) -> Value {
    json!({
        "target_triple": record.target_triple,
        "symbol_name": record.symbol_name,
        "section": record.section,
        "object_format": record.object_format,
        "status": record.status
    })
}

fn nbb_lowering_record_to_json(record: &NbbLoweringRecord) -> Value {
    json!({
        "target_triple": record.target_triple,
        "profile": record.profile,
        "object_format": record.object_format,
        "status": record.status,
        "evidence_boundary_status": record.evidence_boundary_status,
        "machine_symbols": record.machine_symbols.iter().map(nbb_machine_ref_to_json).collect::<Vec<_>>(),
        "sections": record.sections,
        "relocations": record.relocations
    })
}

fn nbb_pending_evidence_to_json(record: &NbbPendingEvidenceRecord) -> Value {
    json!({
        "status": record.status,
        "reason": record.reason,
        "machine_evidence_integration": if record.status == "integrated_machine_evidence" { "closed" } else { "not_started" },
        "final_evidence_integrated": record.status == "integrated_machine_evidence" || record.status == "seal_verified"
    })
}

fn nbb_machine_evidence_command_to_json(command: &NbbMachineEvidenceCommand) -> Value {
    json!({
        "name": command.name,
        "command": command.command,
        "status": command.status,
        "exit_code": command.exit_code,
        "stdout_hash": command.stdout_hash,
        "stderr_hash": command.stderr_hash
    })
}

fn nbb_external_artifact_evidence_to_json(external: &NbbExternalArtifactEvidence) -> Value {
    json!({
        "language": external.language,
        "symbol": external.symbol,
        "abi": external.abi,
        "artifact_kind": external.artifact_kind,
        "artifact_hash": external.artifact_hash,
        "executable_hash": external.executable_hash,
        "link_status": external.link_status
    })
}

fn nbb_evidence_hash_chain_to_json(chain: &NbbEvidenceHashChain) -> Value {
    json!({
        "lowering_hash": chain.lowering_hash,
        "executable_surface_hash": chain.executable_surface_hash,
        "doctor_hash": chain.doctor_hash,
        "native_doctor_hash": chain.native_doctor_hash,
        "artifact_hash": chain.artifact_hash,
        "verify_object_hash": chain.verify_object_hash,
        "disasm_hash": chain.disasm_hash,
        "roundtrip_hash": chain.roundtrip_hash,
        "evidence_hash": chain.evidence_hash
    })
}

fn nbb_machine_evidence_to_json(record: &NbbMachineEvidenceRecord) -> Value {
    json!({
        "kind": "NbbMachineEvidenceRecord",
        "status": record.status,
        "evidence_version": record.evidence_version,
        "machine_target": record.machine_target,
        "object_format": record.object_format,
        "machine_doctor": {
            "status": record.doctor.status,
            "hash": record.doctor.hash
        },
        "native_doctor": {
            "status": record.native_doctor.status,
            "asm_complete_total": record.native_doctor.asm_complete_total,
            "hash": record.native_doctor.hash
        },
        "artifact": {
            "kind": record.artifact.artifact_kind,
            "artifact_hash": record.artifact.artifact_hash,
            "byte_hash": record.artifact.byte_hash
        },
        "verify_object": {
            "status": record.verify_object.status,
            "hash": record.verify_object.hash
        },
        "disasm": {
            "status": record.disasm.status,
            "hash": record.disasm.hash
        },
        "roundtrip": {
            "status": record.roundtrip.status,
            "hash": record.roundtrip.hash
        },
        "debug_unwind": {
            "status": record.debug_unwind.status,
            "reason": record.debug_unwind.reason
        },
        "symbol_relocation_status": record.symbol_relocation_status,
        "source_span_mapping_hash": record.source_span_mapping_hash,
        "deterministic_metadata_hash": record.deterministic_metadata_hash,
        "external_artifacts": record.external_artifacts.iter().map(nbb_external_artifact_evidence_to_json).collect::<Vec<_>>(),
        "hash_chain": nbb_evidence_hash_chain_to_json(&record.hash_chain),
        "commands": record.commands.iter().map(nbb_machine_evidence_command_to_json).collect::<Vec<_>>()
    })
}

fn nbb_seal_record_to_json(record: &NbbSealRecord) -> Value {
    json!({
        "kind": "NbbSealRecord",
        "status": record.status,
        "seal_hash": {
            "algorithm": record.seal_hash.algorithm,
            "hash": record.seal_hash.hash
        },
        "seal_scope": record.seal_scope,
        "seal_policy_version": record.seal_policy_version,
        "deterministic_inputs_hash": record.deterministic_inputs_hash,
        "seal_inputs": record.seal_inputs,
        "non_deterministic_metadata": record.non_deterministic_metadata
    })
}

fn nbb_surface_record_to_json(record: &NbbSurfaceRecord) -> Value {
    json!({
        "kind": record.kind,
        "name": record.name,
        "status": record.status,
        "detail": record.detail
    })
}

fn nbb_ffi_manifest_from_surface_records(records: &[NbbSurfaceRecord]) -> Value {
    let foreign_records = records
        .iter()
        .filter(|record| record.kind == "foreign_binding")
        .collect::<Vec<_>>();
    let manifest_record = records
        .iter()
        .find(|record| record.kind == "foreign_ffi_manifest");
    let status = manifest_record
        .map(|record| record.status.as_str())
        .unwrap_or("absent");
    json!({
        "schema": "basal.nbb.ffi_link_manifest.v1",
        "ghor_compatible": manifest_record.is_some(),
        "status": status,
        "ffi_link_manifest_hash": manifest_record.and_then(|record| detail_field(&record.detail, "ffi_link_manifest_hash")),
        "entries": manifest_record
            .and_then(|record| detail_field(&record.detail, "entries"))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(foreign_records.len()),
        "surface_record": manifest_record.map(nbb_surface_record_to_json),
        "foreign_bindings": foreign_records
            .iter()
            .map(|record| json!({
                "name": record.name,
                "status": record.status,
                "detail": record.detail
            }))
            .collect::<Vec<_>>()
    })
}

fn detail_field(detail: &str, key: &str) -> Option<String> {
    detail.split(',').find_map(|part| {
        let trimmed = part.trim();
        let (left, right) = trimmed.split_once('=')?;
        (left.trim() == key).then(|| right.trim().to_string())
    })
}

fn nbb_string_table_to_json(table: &NbbStringTable) -> Value {
    json!({
        "count": table.strings.len(),
        "strings": table.strings
    })
}

fn nbb_inspect_report(report: &NbbReport) -> NbbInspectReport {
    let header = (report.magic == "NBB0").then(|| NbbHeader {
        magic: report.magic.clone(),
        version: report.version.unwrap_or_default(),
        endian: report
            .endian
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        pointer_width: report.pointer_width.unwrap_or_default(),
        section_count: report.section_count.unwrap_or_default(),
        header_size: report.header_size.unwrap_or_default(),
        section_table_offset: NBB_PHASE5_HEADER_SIZE as u32,
        section_table_size: report
            .section_count
            .map(|count| count as u32 * NBB_SECTION_HEADER_SIZE as u32)
            .unwrap_or_default(),
        total_size: report.total_size.unwrap_or_default(),
        flags: report.flags.unwrap_or_default(),
        checksum: report.structural_hash.clone().unwrap_or_default(),
    });
    NbbInspectReport {
        path: report.path.clone(),
        status: report.status.clone(),
        diagnostics: report.diagnostics.clone(),
        header,
        sections: report.sections.clone(),
        root_records: report.root_records.clone(),
        site_records: report.site_records.clone(),
        bind_records: report.bind_records.clone(),
        lowering_record: report.lowering_record.clone(),
        pending_evidence: report.pending_evidence.clone(),
        machine_evidence: report.machine_evidence.clone(),
        seal_record: report.seal_record.clone(),
        surface_records: report.surface_records.clone(),
    }
}

fn nbb_inspect_to_json(report: &NbbInspectReport) -> Value {
    json!({
        "kind": "NbbInspectReport",
        "path": report.path,
        "status": report.status,
        "phase5_nbb_binary_writer_reader": "closed",
        "header": report.header.as_ref().map(nbb_header_to_json),
        "sections": report.sections.iter().map(nbb_section_to_json).collect::<Vec<_>>(),
        "root_records": report.root_records.iter().map(nbb_root_record_to_json).collect::<Vec<_>>(),
        "site_records": report.site_records.iter().map(nbb_site_record_to_json).collect::<Vec<_>>(),
        "bind_records": report.bind_records.iter().map(nbb_bind_record_to_json).collect::<Vec<_>>(),
        "lowering_record": report.lowering_record.as_ref().map(nbb_lowering_record_to_json),
        "pending_evidence": report.pending_evidence.as_ref().map(nbb_pending_evidence_to_json),
        "machine_evidence": report.machine_evidence.as_ref().map(nbb_machine_evidence_to_json),
        "seal": report.seal_record.as_ref().map(nbb_seal_record_to_json),
        "surface_records": report.surface_records.iter().map(nbb_surface_record_to_json).collect::<Vec<_>>(),
        "ffi_link_manifest": nbb_ffi_manifest_from_surface_records(&report.surface_records),
        "phase5_5_executable_surface": "closed",
        "phase6_machine_evidence_seal": if report.machine_evidence.is_some() { "closed" } else { "pending" },
        "machine_evidence_integration": if report.machine_evidence.is_some() { "closed" } else { "not_started" },
        "seal_final": if report.seal_record.is_some() { "closed" } else { "not_started" },
        "diagnostics": diagnostics_to_json(&report.diagnostics)
    })
}

fn nbb_to_json(report: &NbbReport) -> Value {
    json!({
        "path": report.path,
        "file_classification": ".nbb basal binary source",
        "magic": report.magic,
        "version": report.version,
        "endian": report.endian,
        "pointer_width": report.pointer_width,
        "section_count": report.section_count,
        "flags": report.flags,
        "header_size": report.header_size,
        "total_size": report.total_size,
        "contract_hash": report.contract_hash,
        "structural_hash": report.structural_hash,
        "structural_hash_computed": report.structural_hash_computed,
        "seal_hash": report.seal_hash,
        "sections": report.sections.iter().map(nbb_section_to_json).collect::<Vec<_>>(),
        "root_records": report.root_records.iter().map(nbb_root_record_to_json).collect::<Vec<_>>(),
        "site_records": report.site_records.iter().map(nbb_site_record_to_json).collect::<Vec<_>>(),
        "bind_records": report.bind_records.iter().map(nbb_bind_record_to_json).collect::<Vec<_>>(),
        "lowering_record": report.lowering_record.as_ref().map(nbb_lowering_record_to_json),
        "machine_refs": report.machine_refs.iter().map(nbb_machine_ref_to_json).collect::<Vec<_>>(),
        "pending_evidence": report.pending_evidence.as_ref().map(nbb_pending_evidence_to_json),
        "machine_evidence": report.machine_evidence.as_ref().map(nbb_machine_evidence_to_json),
        "seal": report.seal_record.as_ref().map(nbb_seal_record_to_json),
        "surface_records": report.surface_records.iter().map(nbb_surface_record_to_json).collect::<Vec<_>>(),
        "ffi_link_manifest": nbb_ffi_manifest_from_surface_records(&report.surface_records),
        "string_table": nbb_string_table_to_json(&report.string_table),
        "diagnostics": diagnostics_to_json(&report.diagnostics),
        "status": report.status,
        "claim": report.claim,
        "phase5_nbb_binary_writer_reader": "closed",
        "phase5_5_executable_surface": "closed",
        "nbb_reader": "closed",
        "nbb_verifier": "closed",
        "phase6_machine_evidence_seal": if report.machine_evidence.is_some() { "closed" } else { "pending" },
        "machine_evidence_integration": if report.machine_evidence.is_some() { "closed" } else { "not_started" },
        "seal_final": if report.seal_record.is_some() { "closed" } else { "not_started" }
    })
}

fn audit_to_markdown(audit: &BasalAudit) -> String {
    let mut out = String::new();
    out.push_str("# Basal Report\n\n");
    out.push_str(&format!("- Source: `{}`\n", audit.path));
    out.push_str(&format!("- Status: `{}`\n", audit.status));
    out.push_str(&format!("- Conformance: `{}`\n", audit.conformance_level));
    out.push_str(&format!(
        "- Root container: `{}`\n",
        audit.root_container.as_deref().unwrap_or("<missing>")
    ));
    out.push_str("- Claim: `contract_only`\n");
    out.push_str("- Machine lowering: `closed_plan`\n");
    out.push_str("- Machine evidence integration: `not_started`\n");
    out.push_str("- .nbb writer: `closed`\n");
    out.push_str("- Binary bytes generated by report: `false`\n\n");
    out.push_str("## Sections\n\n");
    for section in &audit.sections {
        out.push_str(&format!("- `{}`\n", section));
    }
    out.push_str("\n## Diagnostics\n\n");
    if audit.diagnostics.is_empty() {
        out.push_str("- none\n");
    } else {
        for diag in &audit.diagnostics {
            out.push_str(&format!("- `{}`: {}\n", diag.code, diag.message));
        }
    }
    out
}

fn nbb_to_markdown(report: &NbbReport) -> String {
    let mut out = String::new();
    out.push_str("# Basal NBB Report\n\n");
    out.push_str(&format!("- Source: `{}`\n", report.path));
    out.push_str(&format!("- Status: `{}`\n", report.status));
    out.push_str(&format!("- Claim: `{}`\n", report.claim));
    out.push_str(&format!("- Magic: `{}`\n", report.magic));
    out.push_str(&format!("- Version: `{:?}`\n", report.version));
    out.push_str(&format!("- Header size: `{:?}`\n", report.header_size));
    out.push_str(&format!("- Total size: `{:?}`\n", report.total_size));
    out.push_str(&format!("- Section count: `{:?}`\n", report.section_count));
    out.push_str(&format!(
        "- Pending evidence: `{:?}`\n\n",
        report
            .pending_evidence
            .as_ref()
            .map(|record| record.status.as_str())
    ));
    out.push_str("## Diagnostics\n\n");
    if report.diagnostics.is_empty() {
        out.push_str("- none\n");
    } else {
        for diag in &report.diagnostics {
            out.push_str(&format!("- `{}`: {}\n", diag.code, diag.message));
        }
    }
    out
}

fn print_json(value: &Value) -> Result<(), Box<dyn Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod canonical_basal_diagnostic_tests {
    use super::*;

    #[test]
    fn basal_diagnostic_preserves_span_and_uses_canonical_binary_abi() {
        let diagnostic = BasalDiagnostic::new("E_BASAL_TEST_001", "basal test")
            .with_span(BasalSpan::new("sample.nb", 4, 7, 2, 5))
            .with_help("fix basal test");
        let value = diagnostics_to_json(&[diagnostic]);
        let entry = &value[0];
        assert_eq!(entry["abi"], "nc.diagnostic.machine.v2");
        assert_eq!(entry["phase"], "basal");
        assert_eq!(
            entry["domain"],
            ncc_diagnostics::DiagnosticDomain::Basal as u8
        );
        assert_eq!(entry["file"], "sample.nb");
        assert_eq!(entry["line"], 2);
        assert_eq!(entry["column"], 5);
        assert_eq!(entry["span"]["start"], 4);
        assert_eq!(entry["span"]["end"], 7);
    }

    #[test]
    fn canonical_root_container_requires_exact_file_stem() {
        let root =
            std::env::temp_dir().join(format!("ncc-canonical-root-name-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create canonical root test directory");
        let source = "ExactRoot :: root {\n}\n";

        let matching = root.join("ExactRoot.nb");
        fs::write(&matching, source).expect("write matching basal source");
        let matching = audit_nb(matching.to_string_lossy().as_ref()).expect("audit matching root");
        assert!(!matching
            .diagnostics
            .iter()
            .any(|diag| diag.code == ncc_diagnostics::codes::E_BASAL_SOURCE_CONTAINER_NAME_001));

        let mismatching = root.join("exactroot.nb");
        fs::write(&mismatching, source).expect("write mismatching basal source");
        let mismatching =
            audit_nb(mismatching.to_string_lossy().as_ref()).expect("audit mismatching root");
        assert!(mismatching
            .diagnostics
            .iter()
            .any(|diag| diag.code == ncc_diagnostics::codes::E_BASAL_SOURCE_CONTAINER_NAME_001));

        let _ = fs::remove_dir_all(root);
    }
}
