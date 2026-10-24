use std::collections::{BTreeSet, HashSet};

use ncc_ast::{PrimitiveAtSection, PrimitiveDirectiveAtom, PrimitiveDirectiveItem};

fn primitive_clause_word(atoms: &[PrimitiveDirectiveAtom], idx: usize) -> Option<&str> {
    match atoms.get(idx) {
        Some(PrimitiveDirectiveAtom::Word(word)) => Some(word.as_str()),
        _ => None,
    }
}

fn primitive_collect_clause_words(item: &PrimitiveDirectiveItem, out: &mut Vec<String>) {
    for atom in &item.atoms {
        if let PrimitiveDirectiveAtom::Word(word) = atom {
            out.push(word.to_ascii_lowercase());
        }
    }
}

pub(crate) fn primitive_collect_protocol_requirements(
    program: &ncc_ast::Program,
) -> BTreeSet<String> {
    let mut protocols = BTreeSet::new();
    for container in &program.containers {
        let Some(primitive) = container.as_primitive_at() else {
            continue;
        };
        for section in primitive.sections {
            if let PrimitiveAtSection::Operation(operation) = section {
                for clause in operation.clauses {
                    let atoms = clause.atoms.as_slice();
                    if primitive_clause_word(atoms, 0) != Some("protocol") {
                        continue;
                    }
                    let Some(protocol_word) = primitive_clause_word(atoms, 1) else {
                        continue;
                    };
                    if let Some(normalized) =
                        super::primitive_normalize_protocol_word(protocol_word)
                    {
                        protocols.insert(normalized.to_string());
                    }
                }
            }
        }
    }
    protocols
}

pub(crate) fn primitive_protocol_requires_controller(protocol: &str) -> bool {
    matches!(
        protocol,
        "PCIE" | "USB" | "SATA" | "NVME" | "I2C" | "SPI" | "ETHERNET" | "HDMI" | "DISPLAYPORT"
    )
}

pub(crate) fn primitive_protocol_requires_peripheral(protocol: &str) -> bool {
    matches!(protocol, "USB" | "SATA" | "NVME" | "ETHERNET")
}

pub(crate) fn primitive_protocol_requires_phy(protocol: &str) -> bool {
    matches!(protocol, "ETHERNET" | "HDMI" | "DISPLAYPORT")
}

pub(crate) fn primitive_protocol_requires_lane(protocol: &str) -> bool {
    matches!(protocol, "PCIE" | "DISPLAYPORT")
}

pub(crate) fn primitive_protocol_requires_channel(protocol: &str) -> bool {
    matches!(protocol, "SATA" | "NVME" | "I2C" | "SPI")
}

pub(crate) fn primitive_collect_all_words(program: &ncc_ast::Program) -> Vec<String> {
    let mut words = Vec::new();
    for container in &program.containers {
        let Some(primitive) = container.as_primitive_at() else {
            continue;
        };
        for section in primitive.sections {
            match section {
                PrimitiveAtSection::Operation(section) => {
                    for clause in section.clauses {
                        primitive_collect_clause_words(&clause, &mut words);
                    }
                }
                PrimitiveAtSection::Port(section) => {
                    for clause in section.clauses {
                        primitive_collect_clause_words(&clause, &mut words);
                    }
                }
                PrimitiveAtSection::WindowBlock(section) => {
                    for clause in section.clauses {
                        primitive_collect_clause_words(&clause, &mut words);
                    }
                }
                PrimitiveAtSection::SignalBlock(section) => {
                    for clause in section.clauses {
                        primitive_collect_clause_words(&clause, &mut words);
                    }
                }
                PrimitiveAtSection::Directive(section) => {
                    for clause in section.items {
                        primitive_collect_clause_words(&clause, &mut words);
                    }
                }
                _ => {}
            }
        }
    }
    words
}

pub(crate) fn primitive_has_rupture_section(program: &ncc_ast::Program) -> bool {
    for container in &program.containers {
        let Some(primitive) = container.as_primitive_at() else {
            continue;
        };
        for section in primitive.sections {
            if matches!(section, PrimitiveAtSection::Rupture(_)) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn primitive_collect_directive_kinds_impl(
    program: &ncc_ast::Program,
) -> HashSet<ncc_ast::PrimitiveDirectiveKind> {
    use ncc_ast::PrimitiveClauseKind;

    let mut out = HashSet::new();
    for container in &program.containers {
        let Some(primitive) = container.as_primitive_at() else {
            continue;
        };
        for section in primitive.sections {
            match section {
                ncc_ast::PrimitiveAtSection::Directive(section) => {
                    out.insert(section.kind.clone());
                    for item in section.items {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Operation(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Port(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::SignalBlock(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::WindowBlock(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::AnchorBlock(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::BusBlock(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Signature(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Invariant(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Constraint(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Entry(section) => {
                    for item in section.clauses {
                        if let Some(PrimitiveClauseKind::Directive(kind)) = item.clause_kind() {
                            out.insert(kind);
                        }
                    }
                }
                ncc_ast::PrimitiveAtSection::Target(_)
                | ncc_ast::PrimitiveAtSection::Anchor(_)
                | ncc_ast::PrimitiveAtSection::Slot(_)
                | ncc_ast::PrimitiveAtSection::Pattern(_)
                | ncc_ast::PrimitiveAtSection::Mask(_)
                | ncc_ast::PrimitiveAtSection::Window(_)
                | ncc_ast::PrimitiveAtSection::Signal(_)
                | ncc_ast::PrimitiveAtSection::Bus(_)
                | ncc_ast::PrimitiveAtSection::State(_)
                | ncc_ast::PrimitiveAtSection::Timing(_)
                | ncc_ast::PrimitiveAtSection::Emit(_)
                | ncc_ast::PrimitiveAtSection::Sample(_)
                | ncc_ast::PrimitiveAtSection::Latch(_)
                | ncc_ast::PrimitiveAtSection::Assert(_)
                | ncc_ast::PrimitiveAtSection::Rupture(_)
                | ncc_ast::PrimitiveAtSection::Bridge(_)
                | ncc_ast::PrimitiveAtSection::Seal(_)
                | ncc_ast::PrimitiveAtSection::Capability(_)
                | ncc_ast::PrimitiveAtSection::Bitfield(_)
                | ncc_ast::PrimitiveAtSection::TypeAlias(_) => {}
            }
        }
    }
    out
}

pub(crate) fn primitive_directive_kind_labels_impl(program: &ncc_ast::Program) -> Vec<String> {
    let mut labels: Vec<String> = primitive_collect_directive_kinds_impl(program)
        .into_iter()
        .map(|kind| format!("{kind:?}"))
        .collect();
    labels.sort();
    labels
}

pub(crate) fn primitive_material_semantic_name_impl(
    kind: &ncc_ast::PrimitiveDirectiveKind,
) -> Option<&'static str> {
    use ncc_ast::PrimitiveDirectiveKind as K;
    match kind {
        K::Voltage => Some("voltage"),
        K::Current => Some("current"),
        K::Noise => Some("noise"),
        K::Clock => Some("clock"),
        K::Edge => Some("edge"),
        K::Cycle => Some("cycle"),
        K::Latency => Some("latency"),
        K::Timeout => Some("timeout"),
        K::Metastability => Some("metastability"),
        K::Power => Some("power"),
        K::PowerGate => Some("powerGate"),
        K::Temperature => Some("temperature"),
        K::ThermalState => Some("thermalState"),
        K::ThrottleState => Some("throttleState"),
        K::Watchdog => Some("watchdog"),
        K::DmaChannel => Some("dmaChannel"),
        K::InterruptLine => Some("interruptLine"),
        K::CacheState => Some("cacheState"),
        K::TlbState => Some("tlbState"),
        K::FirmwareRegion => Some("firmwareRegion"),
        K::BootRomRegion => Some("bootRomRegion"),
        K::BiosRegion => Some("biosRegion"),
        K::UefiHandoff => Some("uefiHandoff"),
        K::ResetVector => Some("resetVector"),
        K::BootWindow => Some("bootWindow"),
        K::BootAnchor => Some("bootAnchor"),
        K::FirmwareTable => Some("firmwareTable"),
        K::DeviceTree => Some("deviceTree"),
        K::AcpiTable => Some("acpiTable"),
        K::MicrocodeState => Some("microcodeState"),
        K::Controller => Some("controller"),
        K::Peripheral => Some("peripheral"),
        K::Phy => Some("phy"),
        K::Lane => Some("lane"),
        K::Channel => Some("channel"),
        K::PipelineState => Some("pipelineState"),
        K::BranchPredictorState => Some("branchPredictorState"),
        K::ReorderState => Some("reorderState"),
        K::OutOfOrder => Some("outOfOrder"),
        K::PhysicalFault => Some("physicalFault"),
        K::BusFault => Some("busFault"),
        K::SignalLoss => Some("signalLoss"),
        K::TimingViolation => Some("timingViolation"),
        K::ThermalViolation => Some("thermalViolation"),
        K::MetastabilityRisk => Some("metastabilityRisk"),
        K::DmaViolation => Some("dmaViolation"),
        K::CacheCoherenceViolation => Some("cacheCoherenceViolation"),
        K::MmioSideEffectViolation => Some("mmioSideEffectViolation"),
        _ => None,
    }
}

pub(crate) fn primitive_material_semantic_labels_impl(program: &ncc_ast::Program) -> Vec<String> {
    let mut labels: Vec<String> = primitive_collect_directive_kinds_impl(program)
        .into_iter()
        .filter_map(|kind| primitive_material_semantic_name_impl(&kind))
        .map(|name| name.to_string())
        .collect();
    labels.sort();
    labels.dedup();
    labels
}
