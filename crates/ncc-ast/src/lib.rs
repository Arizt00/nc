use serde::{Deserialize, Serialize};

mod import_catalog;
pub use import_catalog::*;
mod memory_model;
pub use memory_model::*;
mod language_spec;
pub use language_spec::*;
mod binary_layout;
pub use binary_layout::*;
mod execution_mode;
pub use execution_mode::*;
mod pointer_contract;
pub use pointer_contract::*;
mod storage_contract;
pub use storage_contract::*;
mod raw_api;
pub use raw_api::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub modules: Vec<ModuleDecl>,
    pub packages: Vec<PackageDecl>,
    pub uses: Vec<UseDecl>,
    pub containers: Vec<Container>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseDecl {
    pub path: Vec<String>,
    pub wildcard: bool,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDecl {
    pub path: Vec<String>,
    pub package: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDecl {
    pub name: String,
    pub version: Option<String>,
    pub profile: Option<String>,
    pub modules: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExportKind {
    Function,
    Type,
    Enum,
    Trait,
    Interface,
    Contract,
    Module,
    Use,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDecl {
    pub kind: ExportKind,
    pub path: Vec<String>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub context: Option<Context>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RootAtSection {
    Stc(Vec<StcDecl>),
    Local(LocalSection),
    Binary(Vec<Stmt>),
    Bind(Vec<Stmt>),
    Raw(Vec<Stmt>),
    Probe(Vec<Stmt>),
    Foreign(Vec<ForeignSymbolDecl>),
    Fn(Vec<FunctionDecl>),
    MainEntry {
        kind: MainEntryKind,
        body: Vec<Stmt>,
    },
    Final(Vec<Stmt>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootAtContainer {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub sections: Vec<RootAtSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAtSection {
    Stc(Vec<StcDecl>),
    Local(LocalSection),
    Asset(Vec<String>),
    Identity(Vec<String>),
    Secret(Vec<String>),
    Finding(Vec<String>),
    Incident(Vec<String>),
    Policy(Vec<String>),
    Evidence(Vec<String>),
    Probe(Vec<Stmt>),
    Fn(Vec<FunctionDecl>),
    Final(Vec<Stmt>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAtContainer {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub sections: Vec<SecurityAtSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveAtSection {
    Target(String),
    Anchor(Vec<String>),
    AnchorBlock(PrimitiveAnchorBlockSection),
    Slot(Vec<String>),
    Pattern(Vec<String>),
    Mask(Vec<String>),
    Window(Vec<String>),
    WindowBlock(PrimitiveWindowBlockSection),
    Signal(Vec<String>),
    SignalBlock(PrimitiveSignalBlockSection),
    Bus(Vec<String>),
    BusBlock(PrimitiveBusBlockSection),
    State(Vec<String>),
    Timing(Vec<String>),
    Emit(Vec<String>),
    Sample(Vec<String>),
    Latch(Vec<String>),
    Assert(Vec<String>),
    Rupture(Vec<String>),
    Bridge(Vec<PrimitiveBridgeTarget>),
    Seal(Vec<String>),
    Directive(PrimitiveDirectiveSection),
    Capability(PrimitiveCapabilitySection),
    Bitfield(PrimitiveBitfieldSection),
    TypeAlias(PrimitiveTypeAliasSection),
    Operation(PrimitiveOperationSection),
    Port(PrimitivePortSection),
    Signature(PrimitiveSignatureSection),
    Invariant(PrimitiveInvariantSection),
    Constraint(PrimitiveConstraintSection),
    Entry(PrimitiveEntrySection),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveAtContainer {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub sections: Vec<PrimitiveAtSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimitiveBridgeTarget {
    RootPath(Vec<String>),
    MachinePath(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimitiveDirectiveKind {
    Id,
    Module,
    Type,
    Version,
    Isa,
    Opcode,
    Encoding,
    Relocation,
    Abi,
    Clock,
    Edge,
    Cycle,
    Domain,
    Latency,
    Timeout,
    Voltage,
    Current,
    Noise,
    Metastability,
    Power,
    PowerGate,
    Temperature,
    ThermalState,
    ThrottleState,
    Watchdog,
    Pin,
    Port,
    Wire,
    Width,
    InterruptLine,
    Addr,
    Register,
    Map,
    Read,
    Write,
    Volatile,
    DmaChannel,
    CacheState,
    TlbState,
    FirmwareRegion,
    BootRomRegion,
    BiosRegion,
    UefiHandoff,
    ResetVector,
    BootWindow,
    BootAnchor,
    FirmwareTable,
    DeviceTree,
    AcpiTable,
    MicrocodeState,
    Route,
    Valid,
    Ready,
    Ack,
    Trigger,
    Guard,
    Irq,
    Fault,
    Error,
    Retry,
    Fallback,
    Atomic,
    Barrier,
    Commit,
    Reset,
    Trace,
    Ptr,
    Load,
    Store,
    Mmio,
    Trap,
    Fence,
    Comb,
    Seq,
    Assign,
    Always,
    Transition,
    Drive,
    Release,
    Mux,
    Decode,
    Encode,
    Pipeline,
    PipelineState,
    BranchPredictorState,
    ReorderState,
    OutOfOrder,
    Hazard,
    Controller,
    Peripheral,
    Phy,
    Lane,
    Channel,
    Protocol,
    PhysicalFault,
    BusFault,
    SignalLoss,
    TimingViolation,
    ThermalViolation,
    MetastabilityRisk,
    DmaViolation,
    CacheCoherenceViolation,
    MmioSideEffectViolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveDirectiveSection {
    pub kind: PrimitiveDirectiveKind,
    pub items: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimitiveCapabilityAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveCapabilityRule {
    pub action: PrimitiveCapabilityAction,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveCapabilitySection {
    pub name: String,
    pub rules: Vec<PrimitiveCapabilityRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimitiveBitfieldEntry {
    Bit {
        index: String,
        name: String,
    },
    Bits {
        start: String,
        end: String,
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveBitfieldSection {
    pub name: String,
    pub base: String,
    pub entries: Vec<PrimitiveBitfieldEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveTypeAliasSection {
    pub name: String,
    pub value: PrimitiveDirectiveItem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveOperationParam {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveOperationSection {
    pub name: Vec<String>,
    pub params: Vec<PrimitiveOperationParam>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveAnchorBlockSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveWindowBlockSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveSignalBlockSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveBusBlockSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitivePortSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveSignatureSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveInvariantSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveConstraintSection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimitiveEntrySection {
    pub name: Vec<String>,
    pub clauses: Vec<PrimitiveDirectiveItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PrimitiveDirectiveItem {
    pub atoms: Vec<PrimitiveDirectiveAtom>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimitiveClauseKind {
    Directive(PrimitiveDirectiveKind),
    PhysicalEventContract,
    AsyncHardwareEventContract,
    SideEffectContract,
    AwaitTimeoutCycles,
    SettleCycles,
    AccessSymbol,
    RiskSymbol,
    StableAfterCycles,
    HighMeans,
    ReadSideEffect,
    CapabilityPath,
    RequiresExpr,
    SourcePath,
    FieldPath,
    WindowPath,
    CacheSymbol,
    OwnerPath,
    ImmutableAfter,
    EventInterrupt,
    EventDmaCompletion,
    EventWatchdogTrip,
    EventMmioReadClearsFlag,
    EventPeripheralStateChange,
    EventPhysicalFault,
}

impl PrimitiveDirectiveItem {
    pub fn clause_kind(&self) -> Option<PrimitiveClauseKind> {
        let head = self.word_at(0)?;

        if head == "read" && self.word_at(1) == Some("side_effect") {
            return Some(PrimitiveClauseKind::ReadSideEffect);
        }
        if matches!(head, "sideEffect" | "side_effect") {
            return Some(PrimitiveClauseKind::SideEffectContract);
        }
        if matches!(head, "physicalEvent" | "asyncHardwareEvent") {
            let mapped = matches!(
                self.word_at(1),
                Some("interrupt")
                    | Some("dmaCompletion")
                    | Some("watchdogTrip")
                    | Some("mmioReadClearsFlag")
                    | Some("peripheralStateChange")
                    | Some("physicalFault")
            );
            if !mapped {
                return None;
            }
            return Some(if head == "physicalEvent" {
                PrimitiveClauseKind::PhysicalEventContract
            } else {
                PrimitiveClauseKind::AsyncHardwareEventContract
            });
        }

        if let Some(kind) = PrimitiveDirectiveKind::from_clause_head_word(head) {
            return Some(PrimitiveClauseKind::Directive(kind));
        }

        if matches!(head, "timing" | "body") {
            let nested = self.word_at(1)?;
            return match nested {
                "await" => Some(PrimitiveClauseKind::AwaitTimeoutCycles),
                "settle" => Some(PrimitiveClauseKind::SettleCycles),
                _ => None,
            };
        }

        match head {
            "await" => Some(PrimitiveClauseKind::AwaitTimeoutCycles),
            "settle" => Some(PrimitiveClauseKind::SettleCycles),
            "access" => Some(PrimitiveClauseKind::AccessSymbol),
            "risk" => Some(PrimitiveClauseKind::RiskSymbol),
            "stable" => Some(PrimitiveClauseKind::StableAfterCycles),
            "high" => Some(PrimitiveClauseKind::HighMeans),
            "capability" => Some(PrimitiveClauseKind::CapabilityPath),
            "requires" => Some(PrimitiveClauseKind::RequiresExpr),
            "source" => Some(PrimitiveClauseKind::SourcePath),
            "field" => Some(PrimitiveClauseKind::FieldPath),
            "window" => Some(PrimitiveClauseKind::WindowPath),
            "cache" => Some(PrimitiveClauseKind::CacheSymbol),
            "owner" => Some(PrimitiveClauseKind::OwnerPath),
            "immutable" => Some(PrimitiveClauseKind::ImmutableAfter),
            "event" => match self.word_at(1) {
                Some("interrupt") => Some(PrimitiveClauseKind::EventInterrupt),
                Some("dmaCompletion") => Some(PrimitiveClauseKind::EventDmaCompletion),
                Some("watchdogTrip") => Some(PrimitiveClauseKind::EventWatchdogTrip),
                Some("mmioReadClearsFlag") => Some(PrimitiveClauseKind::EventMmioReadClearsFlag),
                Some("peripheralStateChange") => {
                    Some(PrimitiveClauseKind::EventPeripheralStateChange)
                }
                Some("physicalFault") => Some(PrimitiveClauseKind::EventPhysicalFault),
                _ => None,
            },
            _ => None,
        }
    }

    fn word_at(&self, idx: usize) -> Option<&str> {
        match self.atoms.get(idx) {
            Some(PrimitiveDirectiveAtom::Word(word)) => Some(word.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimitiveDirectiveAtom {
    Word(String),
    Number(String),
    StringLiteral(String),
    Punct(PrimitiveDirectivePunct),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimitiveDirectivePunct {
    Comma,
    Dot,
    At,
    Star,
    Colon,
    Arrow,
    Arrow2,
    Shift2,
    ShiftLeft,
    BangShift,
    Bang,
    Pipe,
    Amp,
    Caret,
    Tilde,
    Plus,
    Minus,
    Slash,
    Percent,
    AndAnd,
    OrOr,
    Less,
    Greater,
    GreaterEq,
    LessEq,
    EqEq,
    NotEq,
    Eq,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
}

impl PrimitiveDirectiveKind {
    pub fn from_clause_head_word(word: &str) -> Option<Self> {
        match word {
            "id" => Some(Self::Id),
            "module" => Some(Self::Module),
            "type" => Some(Self::Type),
            "version" => Some(Self::Version),
            "isa" => Some(Self::Isa),
            "opcode" => Some(Self::Opcode),
            "encoding" => Some(Self::Encoding),
            "relocation" | "reloc" => Some(Self::Relocation),
            "abi" => Some(Self::Abi),
            "clock" => Some(Self::Clock),
            "edge" => Some(Self::Edge),
            "cycle" => Some(Self::Cycle),
            "domain" => Some(Self::Domain),
            "latency" => Some(Self::Latency),
            "timeout" => Some(Self::Timeout),
            "voltage" => Some(Self::Voltage),
            "current" => Some(Self::Current),
            "noise" => Some(Self::Noise),
            "metastability" => Some(Self::Metastability),
            "power" => Some(Self::Power),
            "powergate" | "powerGate" => Some(Self::PowerGate),
            "temperature" => Some(Self::Temperature),
            "thermalstate" | "thermalState" => Some(Self::ThermalState),
            "throttlestate" | "throttleState" => Some(Self::ThrottleState),
            "watchdog" => Some(Self::Watchdog),
            "pin" => Some(Self::Pin),
            "port" => Some(Self::Port),
            "wire" => Some(Self::Wire),
            "width" => Some(Self::Width),
            "interruptline" | "interruptLine" => Some(Self::InterruptLine),
            "addr" | "address" => Some(Self::Addr),
            "register" | "reg" => Some(Self::Register),
            "map" => Some(Self::Map),
            "read" | "reads" => Some(Self::Read),
            "write" | "writes" => Some(Self::Write),
            "volatile" => Some(Self::Volatile),
            "dmachannel" | "dmaChannel" => Some(Self::DmaChannel),
            "cachestate" | "cacheState" => Some(Self::CacheState),
            "tlbstate" | "tlbState" => Some(Self::TlbState),
            "firmwareregion" | "firmwareRegion" => Some(Self::FirmwareRegion),
            "bootromregion" | "bootRomRegion" => Some(Self::BootRomRegion),
            "biosregion" | "biosRegion" => Some(Self::BiosRegion),
            "uefihandoff" | "uefiHandoff" => Some(Self::UefiHandoff),
            "resetvector" | "resetVector" => Some(Self::ResetVector),
            "bootwindow" | "bootWindow" => Some(Self::BootWindow),
            "bootanchor" | "bootAnchor" => Some(Self::BootAnchor),
            "firmwaretable" | "firmwareTable" => Some(Self::FirmwareTable),
            "devicetree" | "deviceTree" => Some(Self::DeviceTree),
            "acpitable" | "acpiTable" => Some(Self::AcpiTable),
            "microcodestate" | "microcodeState" => Some(Self::MicrocodeState),
            "route" => Some(Self::Route),
            "valid" => Some(Self::Valid),
            "ready" => Some(Self::Ready),
            "ack" => Some(Self::Ack),
            "trigger" => Some(Self::Trigger),
            "guard" => Some(Self::Guard),
            "irq" => Some(Self::Irq),
            "fault" | "faults" => Some(Self::Fault),
            "error" => Some(Self::Error),
            "retry" => Some(Self::Retry),
            "fallback" => Some(Self::Fallback),
            "atomic" => Some(Self::Atomic),
            "barrier" => Some(Self::Barrier),
            "commit" => Some(Self::Commit),
            "reset" => Some(Self::Reset),
            "trace" => Some(Self::Trace),
            "ptr" => Some(Self::Ptr),
            "load" => Some(Self::Load),
            "store" => Some(Self::Store),
            "mmio" => Some(Self::Mmio),
            "trap" => Some(Self::Trap),
            "fence" => Some(Self::Fence),
            "comb" => Some(Self::Comb),
            "seq" => Some(Self::Seq),
            "assign" => Some(Self::Assign),
            "always" => Some(Self::Always),
            "transition" => Some(Self::Transition),
            "drive" => Some(Self::Drive),
            "release" => Some(Self::Release),
            "mux" => Some(Self::Mux),
            "decode" => Some(Self::Decode),
            "encode" => Some(Self::Encode),
            "pipeline" => Some(Self::Pipeline),
            "pipelinestate" | "pipelineState" => Some(Self::PipelineState),
            "branchpredictorstate" | "branchPredictorState" => Some(Self::BranchPredictorState),
            "reorderstate" | "reorderState" => Some(Self::ReorderState),
            "outoforder" | "outOfOrder" => Some(Self::OutOfOrder),
            "hazard" => Some(Self::Hazard),
            "controller" => Some(Self::Controller),
            "peripheral" => Some(Self::Peripheral),
            "phy" => Some(Self::Phy),
            "lane" => Some(Self::Lane),
            "channel" => Some(Self::Channel),
            "protocol" => Some(Self::Protocol),
            "physicalfault" | "physicalFault" | "physicalfailure" | "physicalFailure" => {
                Some(Self::PhysicalFault)
            }
            "busfault" | "busFault" => Some(Self::BusFault),
            "signalloss" | "signalLoss" => Some(Self::SignalLoss),
            "timingviolation" | "timingViolation" => Some(Self::TimingViolation),
            "thermalviolation" | "thermalViolation" => Some(Self::ThermalViolation),
            "metastabilityrisk" | "metastabilityRisk" => Some(Self::MetastabilityRisk),
            "dmaviolation" | "dmaViolation" => Some(Self::DmaViolation),
            "cachecoherenceviolation" | "cacheCoherenceViolation" => {
                Some(Self::CacheCoherenceViolation)
            }
            "mmiosideeffectviolation" | "mmioSideEffectViolation" => {
                Some(Self::MmioSideEffectViolation)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MachineInst {
    Label {
        name: String,
    },

    Load {
        dst: String,
        src: MachineMemRef,
    },
    Store {
        dst: MachineMemRef,
        src: String,
    },
    Lea {
        dst: String,
        src: MachineMemRef,
    },

    Mov {
        dst: String,
        src: String,
    },
    Cmp {
        left: String,
        right: String,
    },
    Test {
        left: String,
        right: String,
    },
    Bsf {
        dst: String,
        src: String,
    },
    Bsr {
        dst: String,
        src: String,
    },

    Add {
        dst: String,
        src: String,
    },
    Sub {
        dst: String,
        src: String,
    },
    Mul {
        dst: String,
        src: String,
    },
    Adc {
        dst: String,
        src: String,
    },
    Sbb {
        dst: String,
        src: String,
    },
    And {
        dst: String,
        src: String,
    },
    Or {
        dst: String,
        src: String,
    },
    Xor {
        dst: String,
        src: String,
    },
    Shl {
        dst: String,
        src: String,
    },
    Shr {
        dst: String,
        src: String,
    },
    Sar {
        dst: String,
        src: String,
    },
    Rol {
        dst: String,
        src: String,
    },
    Ror {
        dst: String,
        src: String,
    },
    Xchg {
        dst: String,
        src: String,
    },

    NotBin {
        reg: String,
    },
    Bswap {
        reg: String,
    },
    NegBin {
        reg: String,
    },
    Inc {
        reg: String,
    },
    Dec {
        reg: String,
    },
    Push {
        reg: String,
    },
    Pop {
        reg: String,
    },

    VmxOn {
        region: MachineMemRef,
    },
    VmxOff,
    VmClear {
        region: MachineMemRef,
    },
    VmPtrLoad {
        region: MachineMemRef,
    },
    VmPtrStore {
        destination: MachineMemRef,
    },
    VmRead {
        destination: MachineRegOrMem,
        field: String,
    },
    VmWrite {
        value: String,
        field: MachineRegOrMem,
    },
    InvEpt {
        kind: String,
        descriptor: MachineMemRef,
    },
    InvVpid {
        kind: String,
        descriptor: MachineMemRef,
    },
    VmCall,
    VmLaunch,
    VmResume,
    VmRun,
    VmmCall,
    VmLoad,
    VmSave,
    StGi,
    ClGi,
    InvLpga,

    Call {
        target: MachineCallTarget,
    },
    Jmp {
        target: String,
    },
    JmpEq {
        target: String,
    },
    JmpNe {
        target: String,
    },
    JmpGt {
        target: String,
    },
    JmpLt {
        target: String,
    },
    JmpGe {
        target: String,
    },
    JmpLe {
        target: String,
    },
    JmpOverflow {
        target: String,
    },

    Ret,
    Nop,
    Wait,
    Halt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MachineMemRef {
    MemReg {
        base: String,
    },
    MemAddr {
        base: String,
        index: Option<String>,
        scale: u8,
        disp: isize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MachineRegOrMem {
    Reg(String),
    Mem(MachineMemRef),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MachineCallTarget {
    Label(String),
    RootPath(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MachineFinalOp {
    Ret,
    Halt,
    Jmp { target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MachineAtSection {
    Reg(Vec<String>),
    Flag(Vec<String>),
    Label(Vec<String>),
    Inst(Vec<MachineInst>),
    Final(Vec<MachineFinalOp>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineAtContainer {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub sections: Vec<MachineAtSection>,
}

impl Container {
    pub fn as_root_at(&self) -> Option<RootAtContainer> {
        if !matches!(self.context, Some(Context::Root)) {
            return None;
        }

        let mut sections = Vec::new();
        for section in &self.sections {
            let mapped = match section {
                Section::Stc(items) => RootAtSection::Stc(items.clone()),
                Section::Local(section) => RootAtSection::Local(section.clone()),
                Section::Binary(stmts) => RootAtSection::Binary(stmts.clone()),
                Section::Bind(stmts) => RootAtSection::Bind(stmts.clone()),
                Section::Raw(stmts) => RootAtSection::Raw(stmts.clone()),
                Section::Probe(stmts) => RootAtSection::Probe(stmts.clone()),
                Section::Foreign(items) => RootAtSection::Foreign(items.clone()),
                Section::Fn(funcs) => RootAtSection::Fn(funcs.clone()),
                Section::MainEntry { kind, body } => RootAtSection::MainEntry {
                    kind: kind.clone(),
                    body: body.clone(),
                },
                Section::Final(stmts) => RootAtSection::Final(stmts.clone()),
                _ => return None,
            };
            sections.push(mapped);
        }

        Some(RootAtContainer {
            visibility: self.visibility.clone(),
            name: self.name.clone(),
            sections,
        })
    }

    pub fn as_primitive_at(&self) -> Option<PrimitiveAtContainer> {
        if !matches!(self.context, Some(Context::Primitive)) {
            return None;
        }

        let mut sections = Vec::new();
        for section in &self.sections {
            let mapped = match section {
                Section::PrimitiveTarget(arch) => PrimitiveAtSection::Target(arch.clone()),
                Section::Anchor(items) => PrimitiveAtSection::Anchor(items.clone()),
                Section::AnchorBlock(section) => PrimitiveAtSection::AnchorBlock(section.clone()),
                Section::Slot(items) => PrimitiveAtSection::Slot(items.clone()),
                Section::Pattern(items) => PrimitiveAtSection::Pattern(items.clone()),
                Section::Mask(items) => PrimitiveAtSection::Mask(items.clone()),
                Section::Window(items) => PrimitiveAtSection::Window(items.clone()),
                Section::WindowBlock(section) => PrimitiveAtSection::WindowBlock(section.clone()),
                Section::Signal(items) => PrimitiveAtSection::Signal(items.clone()),
                Section::SignalBlock(section) => PrimitiveAtSection::SignalBlock(section.clone()),
                Section::Bus(items) => PrimitiveAtSection::Bus(items.clone()),
                Section::BusBlock(section) => PrimitiveAtSection::BusBlock(section.clone()),
                Section::State(items) => PrimitiveAtSection::State(items.clone()),
                Section::Timing(items) => PrimitiveAtSection::Timing(items.clone()),
                Section::Emit(items) => PrimitiveAtSection::Emit(items.clone()),
                Section::Sample(items) => PrimitiveAtSection::Sample(items.clone()),
                Section::Latch(items) => PrimitiveAtSection::Latch(items.clone()),
                Section::Assert(items) => PrimitiveAtSection::Assert(items.clone()),
                Section::Rupture(items) => PrimitiveAtSection::Rupture(items.clone()),
                Section::Bridge(items) => PrimitiveAtSection::Bridge(items.clone()),
                Section::Seal(items) => PrimitiveAtSection::Seal(items.clone()),
                Section::PrimitiveDirective(section) => {
                    PrimitiveAtSection::Directive(section.clone())
                }
                Section::Capability(section) => PrimitiveAtSection::Capability(section.clone()),
                Section::Bitfield(section) => PrimitiveAtSection::Bitfield(section.clone()),
                Section::PrimitiveTypeAlias(section) => {
                    PrimitiveAtSection::TypeAlias(section.clone())
                }
                Section::Operation(section) => PrimitiveAtSection::Operation(section.clone()),
                Section::Port(section) => PrimitiveAtSection::Port(section.clone()),
                Section::Signature(section) => PrimitiveAtSection::Signature(section.clone()),
                Section::Invariant(section) => PrimitiveAtSection::Invariant(section.clone()),
                Section::Constraint(section) => PrimitiveAtSection::Constraint(section.clone()),
                Section::Entry(section) => PrimitiveAtSection::Entry(section.clone()),
                _ => return None,
            };
            sections.push(mapped);
        }

        Some(PrimitiveAtContainer {
            visibility: self.visibility.clone(),
            name: self.name.clone(),
            sections,
        })
    }

    pub fn as_security_at(&self) -> Option<SecurityAtContainer> {
        if !matches!(self.context, Some(Context::Security)) {
            return None;
        }

        let mut sections = Vec::new();
        for section in &self.sections {
            let mapped = match section {
                Section::Stc(items) => SecurityAtSection::Stc(items.clone()),
                Section::Local(section) => SecurityAtSection::Local(section.clone()),
                Section::SecurityAsset(items) => SecurityAtSection::Asset(items.clone()),
                Section::SecurityIdentity(items) => SecurityAtSection::Identity(items.clone()),
                Section::SecuritySecret(items) => SecurityAtSection::Secret(items.clone()),
                Section::SecurityFinding(items) => SecurityAtSection::Finding(items.clone()),
                Section::SecurityIncident(items) => SecurityAtSection::Incident(items.clone()),
                Section::SecurityPolicy(items) => SecurityAtSection::Policy(items.clone()),
                Section::SecurityEvidence(items) => SecurityAtSection::Evidence(items.clone()),
                Section::Probe(stmts) => SecurityAtSection::Probe(stmts.clone()),
                Section::Fn(funcs) => SecurityAtSection::Fn(funcs.clone()),
                Section::Final(stmts) => SecurityAtSection::Final(stmts.clone()),
                _ => return None,
            };
            sections.push(mapped);
        }

        Some(SecurityAtContainer {
            visibility: self.visibility.clone(),
            name: self.name.clone(),
            sections,
        })
    }

    pub fn as_machine_at(&self) -> Option<MachineAtContainer> {
        if !matches!(self.context, Some(Context::Machine)) {
            return None;
        }

        let mut sections = Vec::new();
        for section in &self.sections {
            let mapped = match section {
                Section::Reg(items) => MachineAtSection::Reg(items.clone()),
                Section::Flag(items) => MachineAtSection::Flag(items.clone()),
                Section::Label(items) => MachineAtSection::Label(items.clone()),
                Section::Inst(items) => MachineAtSection::Inst(items.clone()),
                Section::MachineFinal(items) => MachineAtSection::Final(items.clone()),
                _ => return None,
            };
            sections.push(mapped);
        }

        Some(MachineAtContainer {
            visibility: self.visibility.clone(),
            name: self.name.clone(),
            sections,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Internal,
    Strictly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Context {
    System,
    Kernel,
    Machine,
    Network,
    Tooling,
    Runtime,
    Compiler,
    Security,
    Inspect,
    Ai,
    Primitive,
    Unknown(String),
    Root,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Section {
    Stc(Vec<StcDecl>),
    Local(LocalSection),
    ConstEval(Vec<ConstEvalDecl>),
    ConstFn(Vec<FunctionDecl>),
    Comptime(Vec<ConstEvalDecl>),
    ComptimeUnsafe(Vec<ComptimeUnsafeBlock>),
    Macros(Vec<MacroItem>),
    Risk(Vec<HighRiskItem>),
    Binary(Vec<Stmt>),
    Interfaces(Vec<InterfaceDecl>),
    Traits(Vec<TraitDecl>),
    ImplBlocks(Vec<ImplDecl>),
    Contracts(Vec<ContractDecl>),
    Enums(Vec<EnumDecl>),
    ModuleDecl(ModuleDecl),
    Use(Vec<UseDecl>),
    Exports(Vec<ExportDecl>),
    Fn(Vec<FunctionDecl>),
    MoveFn(Vec<FunctionDecl>),
    Probe(Vec<Stmt>),
    Structure(Vec<Stmt>),
    Foreign(Vec<ForeignSymbolDecl>),
    TypeContracts(Vec<TypeContractDecl>),
    Final(Vec<Stmt>),
    Bind(Vec<Stmt>),
    Raw(Vec<Stmt>),
    SecurityAsset(Vec<String>),
    SecurityIdentity(Vec<String>),
    SecuritySecret(Vec<String>),
    SecurityFinding(Vec<String>),
    SecurityIncident(Vec<String>),
    SecurityPolicy(Vec<String>),
    SecurityEvidence(Vec<String>),
    PrimitiveTarget(String),
    Anchor(Vec<String>),
    AnchorBlock(PrimitiveAnchorBlockSection),
    Slot(Vec<String>),
    Pattern(Vec<String>),
    Mask(Vec<String>),
    Window(Vec<String>),
    WindowBlock(PrimitiveWindowBlockSection),
    Signal(Vec<String>),
    SignalBlock(PrimitiveSignalBlockSection),
    Bus(Vec<String>),
    BusBlock(PrimitiveBusBlockSection),
    State(Vec<String>),
    Timing(Vec<String>),
    Emit(Vec<String>),
    Sample(Vec<String>),
    Latch(Vec<String>),
    Assert(Vec<String>),
    Rupture(Vec<String>),
    Bridge(Vec<PrimitiveBridgeTarget>),
    Seal(Vec<String>),
    PrimitiveDirective(PrimitiveDirectiveSection),
    Capability(PrimitiveCapabilitySection),
    Bitfield(PrimitiveBitfieldSection),
    PrimitiveTypeAlias(PrimitiveTypeAliasSection),
    Operation(PrimitiveOperationSection),
    Port(PrimitivePortSection),
    Signature(PrimitiveSignatureSection),
    Invariant(PrimitiveInvariantSection),
    Constraint(PrimitiveConstraintSection),
    Entry(PrimitiveEntrySection),
    Reg(Vec<String>),
    Flag(Vec<String>),
    Label(Vec<String>),
    Inst(Vec<MachineInst>),
    MachineFinal(Vec<MachineFinalOp>),
    ManualAbi(Vec<ManualAbiItem>),
    MainEntry {
        kind: MainEntryKind,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManualAbiItem {
    Contract(ManualAbiDecl),
    Binding(ManualAbiBinding),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualAbiDecl {
    pub name: String,
    pub target: String,
    pub pointer_bits: u16,
    pub stack_align_bytes: usize,
    pub shadow_space_bytes: usize,
    pub red_zone_bytes: usize,
    pub integer_argument_registers: Vec<String>,
    pub float_argument_registers: Vec<String>,
    pub integer_return_registers: Vec<String>,
    pub float_return_registers: Vec<String>,
    pub callee_saved_registers: Vec<String>,
    pub clobbered_registers: Vec<String>,
    pub sret_register: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualAbiBinding {
    pub symbol: Vec<String>,
    pub function: Vec<String>,
    pub abi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSection {
    pub decls: Vec<LocalDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalDecl {
    Var {
        visibility: Option<Visibility>,
        name: String,
        ty: TypeName,
        init: Option<Expr>,
    },
    Const {
        visibility: Option<Visibility>,
        name: String,
        ty: TypeName,
        value: Expr,
    },
    TypeAlias {
        visibility: Option<Visibility>,
        name: String,
        ty: TypeName,
    },
    TypeRecord {
        visibility: Option<Visibility>,
        name: String,
        fields: Vec<LocalTypeField>,
    },
    Metadata {
        visibility: Option<Visibility>,
        name: String,
        value: Expr,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTypeField {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstEvalDecl {
    pub visibility: Option<Visibility>,
    pub name: String,
    pub ty: TypeName,
    pub value: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComptimeUnsafeBlock {
    pub name: String,
    pub no_limit: bool,
    pub allows: Vec<ComptimeAllow>,
    pub body: Vec<Stmt>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComptimeAllow {
    pub capability: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HighRiskItem {
    Allow(HighRiskAllowance),
    Accept(HighRiskAccept),
    Bridge(HighRiskBridge),
    Report(HighRiskReportDecl),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighRiskAllowance {
    pub capability: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighRiskAccept {
    pub kind: HighRiskKind,
    pub detail: Option<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighRiskBridge {
    pub target: String,
    pub reason: Option<String>,
    pub authority: Option<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighRiskReportDecl {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HighRiskKind {
    DataRace,
    AliasMutation,
    NoBackpressure,
    StreamNoBackpressure,
    ChannelNoBackpressure,
    NoRollback,
    TransactionPartialCommit,
    PartialMatch,
    UncheckedUnwrap,
    UncheckedCast,
    EffectErasure,
    PrivateLeak,
    AnyLeak,
    AnyConstruction,
    ConstructionLeak,
    RawPtr,
    RawAddr,
    RawSite,
    LowLevelBridge,
    ComptimeUnsafe,
    UnsafeMacro,
    NoLimit,
    UnboundedLoop,
    UnboundedRecursion,
    LowLevelPatch,
    FragileDesign,
    SilentPointer,
    HeapRegionView,
    ImpossibleLowering,
    UnresolvedSymbol,
    InvalidType,
    FakeSafety,
    SilentFallback,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacroItem {
    Decl(MacroDecl),
    Invocation(MacroInvocation),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MacroKind {
    Declarative,
    Template,
    Attribute,
    Derive,
    Unsafe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDecl {
    pub kind: MacroKind,
    pub name: String,
    pub params: Vec<MacroParam>,
    pub derive_target: Option<String>,
    pub no_limit: bool,
    pub allows: Vec<MacroAllow>,
    pub generated_items: Vec<MacroGeneratedItem>,
    pub const_eval_bridges: Vec<Expr>,
    pub effects: Vec<MacroEffect>,
    pub hygiene: MacroHygienePolicy,
    pub recursion_depth: usize,
    pub expansion_steps: usize,
    pub generated_nodes: usize,
    pub nested_expansions: usize,
    pub source_map_required: bool,
    pub invalid_generated_ast: bool,
    pub generated_sema_fail: bool,
    pub private_leak: bool,
    pub any_leak: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroParam {
    pub name: String,
    pub kind: MacroParamKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MacroParamKind {
    Ident,
    Type,
    Expr,
    Str,
    Item,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroInvocation {
    pub name: String,
    pub args: Vec<MacroArg>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacroArg {
    Ident(String),
    Type(TypeName),
    Expr(Expr),
    Str(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAllow {
    pub capability: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEffect {
    pub capability: String,
    pub detail: Option<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroGeneratedItem {
    pub kind: MacroGeneratedKind,
    pub name_template: String,
    pub ty: Option<TypeName>,
    pub value: Option<Expr>,
    pub export: bool,
    pub explicit_capture: bool,
    pub hygiene_escape: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MacroGeneratedKind {
    Function,
    Type,
    Const,
    Impl,
    Annotation,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroHygienePolicy {
    pub explicit_captures: Vec<String>,
    pub exports: Vec<String>,
    pub hygiene_escapes: Vec<String>,
    pub implicit_capture: Option<String>,
    pub private_metadata_access: bool,
    pub any_leak: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StcDecl {
    Typed {
        visibility: Option<Visibility>,
        strictly: bool,
        name: String,
        ty: TypeName,
        init: Option<Expr>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub visibility: Option<Visibility>,
    pub is_async: bool,
    pub effects: Vec<String>,
    pub name: String,
    pub generic_params: Vec<String>,
    pub generic_bounds: Vec<GenericBound>,
    pub params: Vec<Param>,
    pub return_type: TypeName,
    pub public_body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodSignatureDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDecl {
    pub name: String,
    pub generic_params: Vec<String>,
    pub methods: Vec<MethodSignatureDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDecl {
    pub name: String,
    pub generic_params: Vec<String>,
    pub associated_types: Vec<AssociatedTypeDecl>,
    pub required_methods: Vec<MethodSignatureDecl>,
    pub default_methods: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplDecl {
    pub trait_ref: TypeName,
    pub for_type: TypeName,
    pub associated_type_bindings: Vec<AssociatedTypeBinding>,
    pub methods: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociatedTypeDecl {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociatedTypeBinding {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericBound {
    pub param: String,
    pub bound: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDecl {
    pub name: String,
    pub generic_params: Vec<String>,
    pub bounds: Vec<GenericBound>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub invariants: Vec<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub generic_params: Vec<String>,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Vec<EnumPayloadField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumPayloadField {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeContractDecl {
    pub name: String,
    pub kind: TypeContractKind,
    pub generic_params: Vec<String>,
    pub variants: Vec<TypeContractVariant>,
    pub clauses: Vec<TypeContractClause>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ForeignLanguage {
    C,
    Cpp,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForeignContractField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForeignSymbolDecl {
    pub language: ForeignLanguage,
    pub symbol: String,
    pub fields: Vec<ForeignContractField>,
}

pub type TypeContractSection = TypeContractClause;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TypeContractKind {
    Struct,
    Union,
    Enum,
    TaggedUnion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeContractVariant {
    pub name: String,
    pub payload: Vec<TypeContractField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeContractField {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeContractClause {
    Fields {
        visibility: Visibility,
        fields: Vec<TypeContractField>,
    },
    Layout(Vec<String>),
    Invariant(Vec<String>),
    Bounds(Vec<String>),
    Constructure(Vec<String>),
    Destructure(Vec<String>),
    Copy(Vec<String>),
    Clone(Vec<String>),
    Move(Vec<String>),
    Ffi(Vec<String>),
    Ownership(Vec<String>),
    Nullability(Vec<String>),
    Methods(Vec<String>),
    Associated(Vec<String>),
    Validate(Vec<String>),
    Drop(Vec<String>),
    Default(Vec<String>),
    Compare(Vec<String>),
    Hash(Vec<String>),
    Debug(Vec<String>),
    Stable(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDecl {
    pub name: String,
    pub public_fields: Vec<TypeField>,
    pub private_fields: Vec<TypeField>,
    pub internal_fields: Vec<TypeField>,
    pub strictly_fields: Vec<TypeField>,
    pub layout_contract: Option<TypeLayoutContract>,
    pub bounds_contracts: Vec<TypeBoundsContract>,
    pub invariants: Vec<TypeInvariant>,
    pub constructure_contracts: Vec<TypeConstructureContract>,
    pub destructure_contracts: Vec<TypeDestructureContract>,
    pub copy_contract: Option<TypeCopyContract>,
    pub clone_contract: Option<TypeCloneContract>,
    pub move_contract: Option<TypeMoveContract>,
    pub drop_contract: Option<TypeDropContract>,
    pub ffi_contract: Option<TypeFfiContract>,
    pub method_contracts: Vec<TypeMethodContract>,
    pub associated_contracts: Vec<TypeAssociatedFunction>,
    pub validate_contracts: Vec<String>,
    pub default_contract: Option<String>,
    pub compare_contract: Option<String>,
    pub hash_contract: Option<String>,
    pub debug_contract: Option<String>,
    pub stable_contract: Option<String>,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeField {
    pub name: String,
    pub ty: TypeName,
    pub visibility: Visibility,
    pub bounds: Vec<String>,
    pub nullability: Option<String>,
    pub ownership: Option<String>,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeLayoutContract {
    pub kind: Option<String>,
    pub repr: Option<String>,
    pub endian: Option<String>,
    pub align: Option<u64>,
    pub size: Option<u64>,
    pub stable: Option<bool>,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeBoundsContract {
    pub target: String,
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInvariant {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConstructureContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDestructureContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCopyContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCloneContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMoveContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDropContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeFfiContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMethodContract {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAssociatedFunction {
    pub expression: String,
    pub span: Option<TypeContractSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeContractSpan {
    pub start_line: Option<usize>,
    pub start_col: Option<usize>,
    pub end_line: Option<usize>,
    pub end_col: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureParam {
    pub name: String,
    pub ty: TypeName,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CaptureMode {
    Value,
    Ref,
    Mut,
    Move,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureItem {
    pub mode: CaptureMode,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureExpr {
    #[serde(default)]
    pub is_async: bool,
    pub is_move: bool,
    pub params: Vec<ClosureParam>,
    pub return_type: TypeName,
    pub captures: Vec<CaptureItem>,
    pub effects: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TypeName {
    Vector(Box<TypeName>),
    Simple(String),
    Union(Vec<TypeName>),
    Array(Box<TypeName>),
    Generic { base: String, args: Vec<TypeName> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScalarTypeClass {
    Void,
    Bool,
    Char,
    SignedInteger,
    UnsignedInteger,
    Float,
    Pointer,
    Text,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScalarTypeContract {
    pub official_name: &'static str,
    pub class: ScalarTypeClass,
    pub bit_width: Option<u16>,
    pub size_bytes: usize,
    pub align_bytes: usize,
    pub llvm_type: &'static str,
}

impl ScalarTypeContract {
    pub fn is_integer(self) -> bool {
        matches!(
            self.class,
            ScalarTypeClass::SignedInteger | ScalarTypeClass::UnsignedInteger
        )
    }

    pub fn is_numeric(self) -> bool {
        self.is_integer() || self.class == ScalarTypeClass::Float
    }

    pub fn is_float(self) -> bool {
        self.class == ScalarTypeClass::Float
    }

    pub fn is_pointer_like(self) -> bool {
        matches!(self.class, ScalarTypeClass::Pointer | ScalarTypeClass::Text)
    }

    pub fn is_orderable(self) -> bool {
        self.is_numeric() || self.class == ScalarTypeClass::Char
    }
}

pub fn compact_type_surface_name(name: &str) -> String {
    name.trim().replace(' ', "").to_ascii_lowercase()
}

pub fn scalar_type_contract(name: &str, pointer_width_bytes: usize) -> Option<ScalarTypeContract> {
    let compact = compact_type_surface_name(name);
    let pointer_width_bytes = pointer_width_bytes.max(1);
    let pointer_bits = pointer_width_bytes.saturating_mul(8).min(u16::MAX as usize) as u16;
    let text = |official_name| ScalarTypeContract {
        official_name,
        class: ScalarTypeClass::Text,
        bit_width: Some(pointer_bits),
        size_bytes: pointer_width_bytes,
        align_bytes: pointer_width_bytes,
        llvm_type: "ptr",
    };

    if compact == "addr" {
        return Some(ScalarTypeContract {
            official_name: "addr",
            class: ScalarTypeClass::Pointer,
            bit_width: Some(pointer_bits),
            size_bytes: pointer_width_bytes,
            align_bytes: pointer_width_bytes,
            llvm_type: "ptr",
        });
    }

    if matches!(compact.as_str(), "ptr" | "skk*" | "skk**") || scalar_pointer_family_name(&compact)
    {
        let resolved_bytes = pointer_type_contract_from_surface(&compact)
            .ok()
            .flatten()
            .map(|contract| usize::from(contract.resolved_width_bits(pointer_bits)).div_ceil(8))
            .unwrap_or(pointer_width_bytes);
        return Some(ScalarTypeContract {
            official_name: "ptr",
            class: ScalarTypeClass::Pointer,
            bit_width: Some(resolved_bytes.saturating_mul(8).min(u16::MAX as usize) as u16),
            size_bytes: resolved_bytes,
            align_bytes: resolved_bytes,
            llvm_type: "ptr",
        });
    }

    if matches!(compact.as_str(), "isize" | "iptr" | "usize" | "uptr") {
        let llvm_type = match pointer_width_bytes {
            1 => "i8",
            2 => "i16",
            4 => "i32",
            8 => "i64",
            16 => "i128",
            _ => return None,
        };
        return Some(if matches!(compact.as_str(), "isize" | "iptr") {
            signed_scalar("isize", pointer_bits, pointer_width_bytes, llvm_type)
        } else {
            unsigned_scalar("usize", pointer_bits, pointer_width_bytes, llvm_type)
        });
    }

    match compact.as_str() {
        "void" | "unit" => Some(ScalarTypeContract {
            official_name: "void",
            class: ScalarTypeClass::Void,
            bit_width: Some(0),
            size_bytes: 0,
            align_bytes: 1,
            llvm_type: "void",
        }),
        "bool" => Some(ScalarTypeContract {
            official_name: "bool",
            class: ScalarTypeClass::Bool,
            bit_width: Some(1),
            size_bytes: 1,
            align_bytes: 1,
            llvm_type: "i1",
        }),
        "char" => Some(ScalarTypeContract {
            official_name: "char",
            class: ScalarTypeClass::Char,
            bit_width: Some(8),
            size_bytes: 1,
            align_bytes: 1,
            llvm_type: "i8",
        }),
        "byte" | "u8" | "uint8" => Some(unsigned_scalar("byte", 8, 1, "i8")),
        "sbyte" | "i8" | "int8" => Some(signed_scalar("sbyte", 8, 1, "i8")),
        "word" | "u16" | "uint16" | "ushort" => Some(unsigned_scalar("word", 16, 2, "i16")),
        "sword" | "i16" | "int16" | "short" => Some(signed_scalar("sword", 16, 2, "i16")),
        "dword" | "u32" | "uint32" | "uint" => Some(unsigned_scalar("dword", 32, 4, "i32")),
        "sdword" | "i32" | "int32" | "int" => Some(signed_scalar("sdword", 32, 4, "i32")),
        "qword" | "u64" | "uint64" | "ulong" => Some(unsigned_scalar("qword", 64, 8, "i64")),
        "sqword" | "i64" | "int64" | "long" => Some(signed_scalar("sqword", 64, 8, "i64")),
        "ft32" | "f32" | "float" => Some(float_scalar("ft32", 32, 4, "float")),
        "ft64" | "f64" | "double" | "ft" => Some(float_scalar("ft64", 64, 8, "double")),
        "str" | "string" | "text" => Some(text("str")),
        "bytes" => Some(text("bytes")),
        _ => None,
    }
}

pub fn scalar_type_official_name(name: &str) -> Option<&'static str> {
    scalar_type_contract(name, 8).map(|contract| contract.official_name)
}

fn unsigned_scalar(
    official_name: &'static str,
    bit_width: u16,
    size_bytes: usize,
    llvm_type: &'static str,
) -> ScalarTypeContract {
    ScalarTypeContract {
        official_name,
        class: ScalarTypeClass::UnsignedInteger,
        bit_width: Some(bit_width),
        size_bytes,
        align_bytes: size_bytes,
        llvm_type,
    }
}

fn signed_scalar(
    official_name: &'static str,
    bit_width: u16,
    size_bytes: usize,
    llvm_type: &'static str,
) -> ScalarTypeContract {
    ScalarTypeContract {
        official_name,
        class: ScalarTypeClass::SignedInteger,
        bit_width: Some(bit_width),
        size_bytes,
        align_bytes: size_bytes,
        llvm_type,
    }
}

fn float_scalar(
    official_name: &'static str,
    bit_width: u16,
    size_bytes: usize,
    llvm_type: &'static str,
) -> ScalarTypeContract {
    ScalarTypeContract {
        official_name,
        class: ScalarTypeClass::Float,
        bit_width: Some(bit_width),
        size_bytes,
        align_bytes: size_bytes,
        llvm_type,
    }
}

fn scalar_pointer_family_name(compact: &str) -> bool {
    [
        "ptr<",
        "ptrmut<",
        "p<",
        "cap<",
        "member<",
        "pm<",
        "ownerptr<",
        "ref<",
        "refmut<",
        "site<",
        "fn<",
        "dyn<",
    ]
    .iter()
    .any(|prefix| compact.starts_with(prefix) && compact.ends_with('>'))
}

#[cfg(test)]
mod scalar_type_contract_ast_tests {
    use super::*;

    #[test]
    fn scalar_contract_closes_default_widths() {
        let int = scalar_type_contract("int", 8).expect("int contract");
        assert_eq!(int.official_name, "sdword");
        assert_eq!(int.class, ScalarTypeClass::SignedInteger);
        assert_eq!(int.bit_width, Some(32));
        assert_eq!(int.size_bytes, 4);
        assert_eq!(int.align_bytes, 4);
        assert_eq!(int.llvm_type, "i32");

        let uint = scalar_type_contract("uint", 8).expect("uint contract");
        assert_eq!(uint.official_name, "dword");
        assert_eq!(uint.class, ScalarTypeClass::UnsignedInteger);
        assert_eq!(uint.size_bytes, 4);
        assert_eq!(uint.llvm_type, "i32");

        let ft = scalar_type_contract("ft", 8).expect("ft contract");
        assert_eq!(ft.official_name, "ft64");
        assert_eq!(ft.class, ScalarTypeClass::Float);
        assert_eq!(ft.size_bytes, 8);
        assert_eq!(ft.llvm_type, "double");

        let ch = scalar_type_contract("char", 8).expect("char contract");
        assert_eq!(ch.class, ScalarTypeClass::Char);
        assert_eq!(ch.size_bytes, 1);
        assert_eq!(ch.llvm_type, "i8");
    }

    #[test]
    fn scalar_contract_aliases_and_pointer_width_are_official() {
        assert_eq!(
            scalar_type_contract("uint8", 8).map(|contract| contract.official_name),
            Some("byte")
        );
        assert_eq!(
            scalar_type_contract("int64", 8).map(|contract| contract.official_name),
            Some("sqword")
        );
        assert_eq!(
            scalar_type_contract("float", 8).map(|contract| contract.official_name),
            Some("ft32")
        );
        assert_eq!(
            scalar_type_contract("double", 8).map(|contract| contract.official_name),
            Some("ft64")
        );

        let addr32 = scalar_type_contract("addr", 4).expect("addr32 contract");
        assert_eq!(addr32.class, ScalarTypeClass::Pointer);
        assert_eq!(addr32.bit_width, Some(32));
        assert_eq!(addr32.size_bytes, 4);
        assert_eq!(addr32.align_bytes, 4);

        let ptr64 = scalar_type_contract("ptr mut<int>", 8).expect("ptr64 contract");
        assert_eq!(ptr64.official_name, "ptr");
        assert_eq!(ptr64.size_bytes, 8);
        assert_eq!(ptr64.llvm_type, "ptr");
    }

    #[test]
    fn scalar_contract_declares_real_backend_sizes_for_all_core_scalars() {
        let cases = [
            ("bool", "bool", ScalarTypeClass::Bool, 1usize, 1usize, "i1"),
            ("char", "char", ScalarTypeClass::Char, 1, 1, "i8"),
            ("byte", "byte", ScalarTypeClass::UnsignedInteger, 1, 1, "i8"),
            ("sbyte", "sbyte", ScalarTypeClass::SignedInteger, 1, 1, "i8"),
            (
                "word",
                "word",
                ScalarTypeClass::UnsignedInteger,
                2,
                2,
                "i16",
            ),
            (
                "sword",
                "sword",
                ScalarTypeClass::SignedInteger,
                2,
                2,
                "i16",
            ),
            (
                "dword",
                "dword",
                ScalarTypeClass::UnsignedInteger,
                4,
                4,
                "i32",
            ),
            (
                "sdword",
                "sdword",
                ScalarTypeClass::SignedInteger,
                4,
                4,
                "i32",
            ),
            (
                "qword",
                "qword",
                ScalarTypeClass::UnsignedInteger,
                8,
                8,
                "i64",
            ),
            (
                "sqword",
                "sqword",
                ScalarTypeClass::SignedInteger,
                8,
                8,
                "i64",
            ),
            ("ft32", "ft32", ScalarTypeClass::Float, 4, 4, "float"),
            ("ft64", "ft64", ScalarTypeClass::Float, 8, 8, "double"),
            ("str", "str", ScalarTypeClass::Text, 8, 8, "ptr"),
            ("bytes", "bytes", ScalarTypeClass::Text, 8, 8, "ptr"),
            ("addr", "addr", ScalarTypeClass::Pointer, 8, 8, "ptr"),
            ("ptr<int>", "ptr", ScalarTypeClass::Pointer, 8, 8, "ptr"),
        ];

        for (name, official, class, size, align, llvm_type) in cases {
            let contract = scalar_type_contract(name, 8).expect("scalar contract");
            assert_eq!(contract.official_name, official, "{name}");
            assert_eq!(contract.class, class, "{name}");
            assert_eq!(contract.size_bytes, size, "{name}");
            assert_eq!(contract.align_bytes, align, "{name}");
            assert_eq!(contract.llvm_type, llvm_type, "{name}");
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmt {
    Return(Expr),
    Dev(Vec<String>),
    Projection {
        target: AssignTarget,
        ty: TypeName,
        value: Expr,
    },
    StrictCheck {
        target: AssignTarget,
        ty: TypeName,
        value: Expr,
    },
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
    },
    Unless {
        condition: Expr,
        body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    DoWhile {
        body: Vec<Stmt>,
        condition: Expr,
    },
    ForClassic {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        step: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    ForIn {
        item: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    SameIn {
        item: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    ForRange {
        item: String,
        start: Expr,
        end: Expr,
        inclusive: bool,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },
    Loop {
        body: Vec<Stmt>,
    },
    RepeatUntil {
        body: Vec<Stmt>,
        condition: Expr,
    },
    RepeatCount {
        count: Expr,
        body: Vec<Stmt>,
    },
    EachDoWhile {
        item: String,
        iterable: Expr,
        body: Vec<Stmt>,
        condition: Expr,
    },
    Break {
        label: Option<String>,
    },
    Continue {
        label: Option<String>,
    },
    Labeled {
        label: String,
        stmt: Box<Stmt>,
    },
    Switch {
        value: Expr,
        cases: Vec<(Expr, Vec<Stmt>)>,
        default: Vec<Stmt>,
    },
    Match {
        value: Expr,
        cases: Vec<(Expr, Vec<Stmt>)>,
        default: Vec<Stmt>,
    },
    PatternMatch {
        value: Expr,
        arms: Vec<MatchArm>,
    },
    Defer {
        body: Vec<Stmt>,
    },
    GuardElse {
        condition: Expr,
        else_block: Vec<Stmt>,
    },
    Select {
        branches: Vec<(Expr, Vec<Stmt>)>,
        default: Vec<Stmt>,
    },
    MainRefFn {
        kind: MainKind,
        fn_ref: FnRef,
    },
    FunctionRelation {
        fn_ref: FnRef,
        kind: FunctionRelationKind,
    },
    Print {
        kind: PrintKind,
        value: Expr,
    },
    Destructure {
        kind: DestructureKind,
        args: Vec<Expr>,
    },
    Constructure {
        kind: ConstructureKind,
        args: Vec<Expr>,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pattern {
    Wildcard,
    Variant {
        enum_name: String,
        variant_name: String,
        bindings: Vec<String>,
    },
    VariantPattern {
        enum_name: String,
        variant_name: String,
        items: Vec<Pattern>,
    },
    Struct {
        type_name: String,
        fields: Vec<PatternField>,
        has_rest: bool,
    },
    Array {
        items: Vec<Pattern>,
        rest: Option<String>,
    },
    Range {
        start: Expr,
        end: Expr,
        inclusive: bool,
    },
    Literal(Expr),
    Binding(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternField {
    pub name: String,
    pub pattern: Pattern,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DestructureKind {
    Default,
    Static,
    Move,
    Ref,
    Region,
    Binary,
    Result,
    Ownership,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstructureKind {
    Default,
    Static,
    Move,
    Copy,
    Region,
    Binary,
    Unbinary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MainKind {
    Ref,
    Skk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionRelationKind {
    Legacy,
    Connect,
    Concurrent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrintKind {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MainEntryKind {
    ToMain,
    ToMainProfile(String),
    RefFn(FnRef),
    SkkFn(FnRef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnRef {
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    StructLiteral {
        ty: String,
        fields: Vec<(String, Expr)>,
    },
    VectorLiteral(Vec<Expr>),
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Closure(ClosureExpr),
    Bool(bool),
    Number(String),
    Char(char),
    String(String),
    Identifier(String),
    ContextRef(ContextRef),
    ArrayLiteral(Vec<Expr>),
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    QualifiedCall {
        path: Vec<String>,
        args: Vec<Expr>,
    },
    Dev(Vec<String>),
    StrictCheckExpr {
        target: ContextRef,
        ty: TypeName,
        value: Box<Expr>,
    },
    PostIncrement {
        target: ContextRef,
    },
    FieldPostIncrement {
        target: ContextRef,
    },
    PointerPostIncrementRead {
        target: ContextRef,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    IfElse {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssignTarget {
    Context(ContextRef),
    Mem { addr: Box<Expr> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRef {
    pub path: Vec<String>,
    pub origin: ContextRefOrigin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextRefOrigin {
    Plain,
    FnSyntax,
    ProbeSyntax,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextRefKind {
    Symbol,
    Function,
    Probe,
    ContextPath,
    Resource,
}

impl ContextRefKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Symbol => "symbol",
            Self::Function => "function",
            Self::Probe => "probe",
            Self::ContextPath => "path",
            Self::Resource => "resource",
        }
    }

    pub fn from_path(path: &[String]) -> Self {
        if path.is_empty() {
            return Self::ContextPath;
        }

        let head = path[0].as_str();
        if head.eq_ignore_ascii_case("fn") {
            return Self::Function;
        }
        if head.eq_ignore_ascii_case("probe") {
            return Self::Probe;
        }
        if is_resource_ref_head(head) {
            return Self::Resource;
        }
        if path.len() == 1 {
            return Self::Symbol;
        }
        Self::ContextPath
    }
}

impl ContextRef {
    pub fn plain(path: Vec<String>) -> Self {
        Self {
            path,
            origin: ContextRefOrigin::Plain,
        }
    }

    pub fn from_fn(path: Vec<String>) -> Self {
        Self {
            path,
            origin: ContextRefOrigin::FnSyntax,
        }
    }

    pub fn from_probe(path: Vec<String>) -> Self {
        Self {
            path,
            origin: ContextRefOrigin::ProbeSyntax,
        }
    }

    pub fn inferred_kind(&self) -> ContextRefKind {
        match self.origin {
            ContextRefOrigin::FnSyntax => ContextRefKind::Function,
            ContextRefOrigin::ProbeSyntax => ContextRefKind::Probe,
            ContextRefOrigin::Plain => ContextRefKind::from_path(&self.path),
        }
    }
}

fn is_resource_ref_head(segment: &str) -> bool {
    let head = segment.to_ascii_lowercase();
    matches!(
        head.as_str(),
        "root"
            | "runtime"
            | "memory"
            | "system"
            | "tooling"
            | "compiler"
            | "kernel"
            | "security"
            | "inspect"
            | "network"
            | "net"
            | "ai"
            | "machine"
            | "primitive"
            | "heap"
            | "policy"
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOp {
    Greater,
    Less,
    GreaterEq,
    LessEq,
    EqEq,
    NotEq,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    ShiftRightLogical,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AndAnd,
    OrOr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    NotBin,
    TryPropagate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapManagerOp {
    Alloc,
    Free,
    Realloc,
    Zero,
}

impl HeapManagerOp {
    pub fn from_qualified_path(path: &[String]) -> Option<Self> {
        match path {
            [a, b, c, d] if a == "memory" && b == "heap" && c == "manager" => match d.as_str() {
                "alloc" => Some(Self::Alloc),
                "free" => Some(Self::Free),
                "realloc" => Some(Self::Realloc),
                "zero" => Some(Self::Zero),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn official_path(self) -> &'static str {
        match self {
            Self::Alloc => "memory.heap.manager.alloc",
            Self::Free => "memory.heap.manager.free",
            Self::Realloc => "memory.heap.manager.realloc",
            Self::Zero => "memory.heap.manager.zero",
        }
    }

    pub fn requires_valid_pointer(self) -> bool {
        matches!(self, Self::Free | Self::Realloc | Self::Zero)
    }

    pub fn invalidates_alias_group(self) -> bool {
        matches!(self, Self::Free | Self::Realloc)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InspectRuntimePath {
    HeapEventsTail,
    HeapEventsLast,
    HeapEventsCount,
    HeapEventsValid,
    HeapEventsError,
    HeapLiveAllocs,
    HeapAllocCount,
    HeapBlocksCount,
    NetConnected,
    NetListening,
    NetAccepted,
    NetLastSend,
    NetHost,
    NetPort,
    NetLastPayload,
    SecurityLastAction,
    SecurityAllowCount,
    SecurityDenyCount,
    SecurityQuarantineCount,
    SecurityEscalateCount,
    SecurityAuditCount,
    SecurityAttackCount,
    SecurityLastAttack,
    SystemOpen,
    SystemHealthy,
    SystemDegraded,
    SystemRecovered,
    SystemEventsCount,
    SystemServicesCount,
    SystemLastEvent,
    SystemLastService,
    ToolingBuildReady,
    ToolingLintReady,
    ToolingFormatReady,
    ToolingPackageReady,
    ToolingPublishReady,
    ToolingEventsCount,
    ToolingLastTarget,
    CompilerParseReady,
    CompilerSemaReady,
    CompilerBackendReady,
    CompilerEventsCount,
    CompilerLastUnit,
    KernelSchedulerOpen,
    KernelPanicState,
    KernelQuarantineCount,
    KernelEventsCount,
    KernelLastTask,
    KernelLastEvent,
    AiSessionActive,
    AiModelReady,
    AiPlannerReady,
    AiCriticReady,
    AiExecutorReady,
    AiMemoryCount,
    AiEventsCount,
    AiLastProvider,
    AiLastModel,
    AiLastGoal,
    AiLastPrompt,
    AiLastResponse,
}

pub const INSPECT_RUNTIME_OFFICIAL_PATHS: &[&str] = &[
    "heap.events.tail",
    "heap.events.last",
    "heap.events.count",
    "heap.events.valid",
    "heap.events.error",
    "heap.live.allocs",
    "heap.alloc.count",
    "heap.blocks.count",
    "net.connected",
    "net.listening",
    "net.accepted",
    "net.last_send",
    "net.host",
    "net.port",
    "net.last_payload",
    "security.last_action",
    "security.allow.count",
    "security.deny.count",
    "security.quarantine.count",
    "security.escalate.count",
    "security.audit.count",
    "security.attack.count",
    "security.last_attack",
    "system.open",
    "system.healthy",
    "system.degraded",
    "system.recovered",
    "system.events.count",
    "system.services.count",
    "system.last_event",
    "system.last_service",
    "tooling.build.ready",
    "tooling.lint.ready",
    "tooling.format.ready",
    "tooling.package.ready",
    "tooling.publish.ready",
    "tooling.events.count",
    "tooling.last_target",
    "compiler.parse.ready",
    "compiler.sema.ready",
    "compiler.backend.ready",
    "compiler.events.count",
    "compiler.last_unit",
    "kernel.scheduler.open",
    "kernel.panic.state",
    "kernel.quarantine.count",
    "kernel.events.count",
    "kernel.last_task",
    "kernel.last_event",
    "ai.session.active",
    "ai.model.ready",
    "ai.planner.ready",
    "ai.critic.ready",
    "ai.executor.ready",
    "ai.memory.count",
    "ai.events.count",
    "ai.last_provider",
    "ai.last_model",
    "ai.last_goal",
    "ai.last_prompt",
    "ai.last_response",
];

impl InspectRuntimePath {
    pub fn official_path(self) -> &'static str {
        match self {
            Self::HeapEventsTail => "heap.events.tail",
            Self::HeapEventsLast => "heap.events.last",
            Self::HeapEventsCount => "heap.events.count",
            Self::HeapEventsValid => "heap.events.valid",
            Self::HeapEventsError => "heap.events.error",
            Self::HeapLiveAllocs => "heap.live.allocs",
            Self::HeapAllocCount => "heap.alloc.count",
            Self::HeapBlocksCount => "heap.blocks.count",
            Self::NetConnected => "net.connected",
            Self::NetListening => "net.listening",
            Self::NetAccepted => "net.accepted",
            Self::NetLastSend => "net.last_send",
            Self::NetHost => "net.host",
            Self::NetPort => "net.port",
            Self::NetLastPayload => "net.last_payload",
            Self::SecurityLastAction => "security.last_action",
            Self::SecurityAllowCount => "security.allow.count",
            Self::SecurityDenyCount => "security.deny.count",
            Self::SecurityQuarantineCount => "security.quarantine.count",
            Self::SecurityEscalateCount => "security.escalate.count",
            Self::SecurityAuditCount => "security.audit.count",
            Self::SecurityAttackCount => "security.attack.count",
            Self::SecurityLastAttack => "security.last_attack",
            Self::SystemOpen => "system.open",
            Self::SystemHealthy => "system.healthy",
            Self::SystemDegraded => "system.degraded",
            Self::SystemRecovered => "system.recovered",
            Self::SystemEventsCount => "system.events.count",
            Self::SystemServicesCount => "system.services.count",
            Self::SystemLastEvent => "system.last_event",
            Self::SystemLastService => "system.last_service",
            Self::ToolingBuildReady => "tooling.build.ready",
            Self::ToolingLintReady => "tooling.lint.ready",
            Self::ToolingFormatReady => "tooling.format.ready",
            Self::ToolingPackageReady => "tooling.package.ready",
            Self::ToolingPublishReady => "tooling.publish.ready",
            Self::ToolingEventsCount => "tooling.events.count",
            Self::ToolingLastTarget => "tooling.last_target",
            Self::CompilerParseReady => "compiler.parse.ready",
            Self::CompilerSemaReady => "compiler.sema.ready",
            Self::CompilerBackendReady => "compiler.backend.ready",
            Self::CompilerEventsCount => "compiler.events.count",
            Self::CompilerLastUnit => "compiler.last_unit",
            Self::KernelSchedulerOpen => "kernel.scheduler.open",
            Self::KernelPanicState => "kernel.panic.state",
            Self::KernelQuarantineCount => "kernel.quarantine.count",
            Self::KernelEventsCount => "kernel.events.count",
            Self::KernelLastTask => "kernel.last_task",
            Self::KernelLastEvent => "kernel.last_event",
            Self::AiSessionActive => "ai.session.active",
            Self::AiModelReady => "ai.model.ready",
            Self::AiPlannerReady => "ai.planner.ready",
            Self::AiCriticReady => "ai.critic.ready",
            Self::AiExecutorReady => "ai.executor.ready",
            Self::AiMemoryCount => "ai.memory.count",
            Self::AiEventsCount => "ai.events.count",
            Self::AiLastProvider => "ai.last_provider",
            Self::AiLastModel => "ai.last_model",
            Self::AiLastGoal => "ai.last_goal",
            Self::AiLastPrompt => "ai.last_prompt",
            Self::AiLastResponse => "ai.last_response",
        }
    }
}

pub fn inspect_runtime_path_from_segments(path: &[String]) -> Option<InspectRuntimePath> {
    match path {
        [a, b, c] if a == "heap" && b == "events" && c == "tail" => {
            Some(InspectRuntimePath::HeapEventsTail)
        }
        [a, b, c] if a == "heap" && b == "events" && c == "last" => {
            Some(InspectRuntimePath::HeapEventsLast)
        }
        [a, b, c] if a == "heap" && b == "events" && c == "count" => {
            Some(InspectRuntimePath::HeapEventsCount)
        }
        [a, b, c] if a == "heap" && b == "events" && c == "valid" => {
            Some(InspectRuntimePath::HeapEventsValid)
        }
        [a, b, c] if a == "heap" && b == "events" && c == "error" => {
            Some(InspectRuntimePath::HeapEventsError)
        }
        [a, b, c] if a == "heap" && b == "live" && c == "allocs" => {
            Some(InspectRuntimePath::HeapLiveAllocs)
        }
        [a, b, c] if a == "heap" && b == "alloc" && c == "count" => {
            Some(InspectRuntimePath::HeapAllocCount)
        }
        [a, b, c] if a == "heap" && b == "blocks" && c == "count" => {
            Some(InspectRuntimePath::HeapBlocksCount)
        }
        [a, b] if a == "net" && b == "connected" => Some(InspectRuntimePath::NetConnected),
        [a, b] if a == "net" && b == "listening" => Some(InspectRuntimePath::NetListening),
        [a, b] if a == "net" && b == "accepted" => Some(InspectRuntimePath::NetAccepted),
        [a, b] if a == "net" && b == "last_send" => Some(InspectRuntimePath::NetLastSend),
        [a, b] if a == "net" && b == "host" => Some(InspectRuntimePath::NetHost),
        [a, b] if a == "net" && b == "port" => Some(InspectRuntimePath::NetPort),
        [a, b] if a == "net" && b == "last_payload" => Some(InspectRuntimePath::NetLastPayload),
        [a, b] if a == "security" && b == "last_action" => {
            Some(InspectRuntimePath::SecurityLastAction)
        }
        [a, b] if a == "security" && b == "last_attack" => {
            Some(InspectRuntimePath::SecurityLastAttack)
        }
        [a, b, c] if a == "security" && b == "allow" && c == "count" => {
            Some(InspectRuntimePath::SecurityAllowCount)
        }
        [a, b, c] if a == "security" && b == "deny" && c == "count" => {
            Some(InspectRuntimePath::SecurityDenyCount)
        }
        [a, b, c] if a == "security" && b == "quarantine" && c == "count" => {
            Some(InspectRuntimePath::SecurityQuarantineCount)
        }
        [a, b, c] if a == "security" && b == "escalate" && c == "count" => {
            Some(InspectRuntimePath::SecurityEscalateCount)
        }
        [a, b, c] if a == "security" && b == "audit" && c == "count" => {
            Some(InspectRuntimePath::SecurityAuditCount)
        }
        [a, b, c] if a == "security" && b == "attack" && c == "count" => {
            Some(InspectRuntimePath::SecurityAttackCount)
        }
        [a, b] if a == "system" && b == "open" => Some(InspectRuntimePath::SystemOpen),
        [a, b] if a == "system" && b == "healthy" => Some(InspectRuntimePath::SystemHealthy),
        [a, b] if a == "system" && b == "degraded" => Some(InspectRuntimePath::SystemDegraded),
        [a, b] if a == "system" && b == "recovered" => Some(InspectRuntimePath::SystemRecovered),
        [a, b] if a == "system" && b == "last_event" => Some(InspectRuntimePath::SystemLastEvent),
        [a, b] if a == "system" && b == "last_service" => {
            Some(InspectRuntimePath::SystemLastService)
        }
        [a, b, c] if a == "system" && b == "events" && c == "count" => {
            Some(InspectRuntimePath::SystemEventsCount)
        }
        [a, b, c] if a == "system" && b == "services" && c == "count" => {
            Some(InspectRuntimePath::SystemServicesCount)
        }
        [a, b, c] if a == "tooling" && b == "build" && c == "ready" => {
            Some(InspectRuntimePath::ToolingBuildReady)
        }
        [a, b, c] if a == "tooling" && b == "lint" && c == "ready" => {
            Some(InspectRuntimePath::ToolingLintReady)
        }
        [a, b, c] if a == "tooling" && b == "format" && c == "ready" => {
            Some(InspectRuntimePath::ToolingFormatReady)
        }
        [a, b, c] if a == "tooling" && b == "package" && c == "ready" => {
            Some(InspectRuntimePath::ToolingPackageReady)
        }
        [a, b, c] if a == "tooling" && b == "publish" && c == "ready" => {
            Some(InspectRuntimePath::ToolingPublishReady)
        }
        [a, b, c] if a == "tooling" && b == "events" && c == "count" => {
            Some(InspectRuntimePath::ToolingEventsCount)
        }
        [a, b] if a == "tooling" && b == "last_target" => {
            Some(InspectRuntimePath::ToolingLastTarget)
        }
        [a, b, c] if a == "compiler" && b == "parse" && c == "ready" => {
            Some(InspectRuntimePath::CompilerParseReady)
        }
        [a, b, c] if a == "compiler" && b == "sema" && c == "ready" => {
            Some(InspectRuntimePath::CompilerSemaReady)
        }
        [a, b, c] if a == "compiler" && b == "backend" && c == "ready" => {
            Some(InspectRuntimePath::CompilerBackendReady)
        }
        [a, b, c] if a == "compiler" && b == "events" && c == "count" => {
            Some(InspectRuntimePath::CompilerEventsCount)
        }
        [a, b] if a == "compiler" && b == "last_unit" => Some(InspectRuntimePath::CompilerLastUnit),
        [a, b, c] if a == "kernel" && b == "scheduler" && c == "open" => {
            Some(InspectRuntimePath::KernelSchedulerOpen)
        }
        [a, b, c] if a == "kernel" && b == "panic" && c == "state" => {
            Some(InspectRuntimePath::KernelPanicState)
        }
        [a, b, c] if a == "kernel" && b == "quarantine" && c == "count" => {
            Some(InspectRuntimePath::KernelQuarantineCount)
        }
        [a, b, c] if a == "kernel" && b == "events" && c == "count" => {
            Some(InspectRuntimePath::KernelEventsCount)
        }
        [a, b] if a == "kernel" && b == "last_task" => Some(InspectRuntimePath::KernelLastTask),
        [a, b] if a == "kernel" && b == "last_event" => Some(InspectRuntimePath::KernelLastEvent),
        [a, b, c] if a == "ai" && b == "session" && c == "active" => {
            Some(InspectRuntimePath::AiSessionActive)
        }
        [a, b, c] if a == "ai" && b == "model" && c == "ready" => {
            Some(InspectRuntimePath::AiModelReady)
        }
        [a, b, c] if a == "ai" && b == "planner" && c == "ready" => {
            Some(InspectRuntimePath::AiPlannerReady)
        }
        [a, b, c] if a == "ai" && b == "critic" && c == "ready" => {
            Some(InspectRuntimePath::AiCriticReady)
        }
        [a, b, c] if a == "ai" && b == "executor" && c == "ready" => {
            Some(InspectRuntimePath::AiExecutorReady)
        }
        [a, b, c] if a == "ai" && b == "memory" && c == "count" => {
            Some(InspectRuntimePath::AiMemoryCount)
        }
        [a, b, c] if a == "ai" && b == "events" && c == "count" => {
            Some(InspectRuntimePath::AiEventsCount)
        }
        [a, b] if a == "ai" && b == "last_provider" => Some(InspectRuntimePath::AiLastProvider),
        [a, b] if a == "ai" && b == "last_model" => Some(InspectRuntimePath::AiLastModel),
        [a, b] if a == "ai" && b == "last_goal" => Some(InspectRuntimePath::AiLastGoal),
        [a, b] if a == "ai" && b == "last_prompt" => Some(InspectRuntimePath::AiLastPrompt),
        [a, b] if a == "ai" && b == "last_response" => Some(InspectRuntimePath::AiLastResponse),
        _ => None,
    }
}

pub fn inspect_runtime_expected_paths_diagnostic() -> String {
    INSPECT_RUNTIME_OFFICIAL_PATHS
        .iter()
        .map(|path| format!("'{}'", path))
        .collect::<Vec<_>>()
        .join(" or ")
}

#[cfg(test)]
mod machine_at_ast_tests {
    use super::*;

    #[test]
    fn container_as_machine_at_maps_supported_sections() {
        let container = Container {
            visibility: None,
            name: "machineCore".into(),
            context: Some(Context::Machine),
            sections: vec![
                Section::Reg(vec!["ax".into(), "bx".into()]),
                Section::Flag(vec!["zeroFlag".into()]),
                Section::Label(vec!["entryPoint".into()]),
                Section::Inst(vec![
                    MachineInst::Mov {
                        dst: "ax".into(),
                        src: "bx".into(),
                    },
                    MachineInst::Cmp {
                        left: "ax".into(),
                        right: "bx".into(),
                    },
                ]),
                Section::MachineFinal(vec![MachineFinalOp::Ret]),
            ],
        };

        let machine = container.as_machine_at();
        assert!(
            machine.is_some(),
            "expected machine container to map to MachineAtContainer"
        );
        let machine = machine.unwrap();
        assert_eq!(machine.name, "machineCore");
        assert_eq!(machine.sections.len(), 5);
    }

    #[test]
    fn container_as_machine_at_rejects_non_machine_or_unsupported_sections() {
        let non_machine = Container {
            visibility: None,
            name: "rootLike".into(),
            context: Some(Context::Root),
            sections: vec![Section::Reg(vec!["ax".into()])],
        };
        assert!(non_machine.as_machine_at().is_none());

        let bad_machine = Container {
            visibility: None,
            name: "badMachine".into(),
            context: Some(Context::Machine),
            sections: vec![Section::Fn(vec![])],
        };
        assert!(bad_machine.as_machine_at().is_none());
    }
}

#[cfg(test)]
mod root_at_ast_tests {
    use super::*;

    #[test]
    fn container_as_root_at_maps_supported_sections() {
        let container = Container {
            visibility: None,
            name: "rootCore".into(),
            context: Some(Context::Root),
            sections: vec![
                Section::Stc(vec![]),
                Section::Binary(vec![]),
                Section::Bind(vec![]),
                Section::Raw(vec![]),
                Section::Probe(vec![]),
                Section::Fn(vec![]),
                Section::MainEntry {
                    kind: MainEntryKind::ToMain,
                    body: vec![],
                },
                Section::Final(vec![]),
            ],
        };

        let root = container.as_root_at();
        assert!(
            root.is_some(),
            "expected root container to map to RootAtContainer"
        );
        let root = root.unwrap();
        assert_eq!(root.name, "rootCore");
        assert_eq!(root.sections.len(), 8);
    }

    #[test]
    fn container_as_root_at_rejects_non_root_or_unsupported_sections() {
        let non_root = Container {
            visibility: None,
            name: "sys".into(),
            context: Some(Context::System),
            sections: vec![Section::Fn(vec![])],
        };
        assert!(non_root.as_root_at().is_none());

        let bad_root = Container {
            visibility: None,
            name: "badRoot".into(),
            context: Some(Context::Root),
            sections: vec![Section::Structure(vec![])],
        };
        assert!(bad_root.as_root_at().is_none());
    }
}

#[cfg(test)]
mod primitive_at_ast_tests {
    use super::*;

    #[test]
    fn container_as_primitive_at_maps_supported_sections() {
        let container = Container {
            visibility: None,
            name: "primitiveCore".into(),
            context: Some(Context::Primitive),
            sections: vec![
                Section::PrimitiveTarget("x86_64".into()),
                Section::Anchor(vec!["baseEdge".into()]),
                Section::AnchorBlock(PrimitiveAnchorBlockSection {
                    name: vec!["boot0".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("align".into())],
                    }],
                }),
                Section::Slot(vec!["inputSlot".into()]),
                Section::Pattern(vec!["edgePattern".into()]),
                Section::Mask(vec!["edgeMask".into()]),
                Section::Window(vec!["edgeWindow".into()]),
                Section::WindowBlock(PrimitiveWindowBlockSection {
                    name: vec!["boot_sector".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("cache".into())],
                    }],
                }),
                Section::Signal(vec!["pulseSignal".into()]),
                Section::SignalBlock(PrimitiveSignalBlockSection {
                    name: vec!["uart0".into(), "tx_ready".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("sample".into())],
                    }],
                }),
                Section::Bus(vec!["ioBus".into()]),
                Section::BusBlock(PrimitiveBusBlockSection {
                    name: vec!["legacy_io8".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("width".into())],
                    }],
                }),
                Section::State(vec!["bootState".into()]),
                Section::Timing(vec!["bootTick".into()]),
                Section::Emit(vec!["bootEmit".into()]),
                Section::Sample(vec!["bootSample".into()]),
                Section::Latch(vec!["bootLatch".into()]),
                Section::Assert(vec!["bootAssert".into()]),
                Section::Rupture(vec!["bootRupture".into()]),
                Section::Capability(PrimitiveCapabilitySection {
                    name: "boot_serial_io".into(),
                    rules: vec![PrimitiveCapabilityRule {
                        action: PrimitiveCapabilityAction::Allow,
                        path: vec!["port".into(), "read".into()],
                    }],
                }),
                Section::Bitfield(PrimitiveBitfieldSection {
                    name: "uart_status".into(),
                    base: "byte".into(),
                    entries: vec![PrimitiveBitfieldEntry::Bit {
                        index: "0".into(),
                        name: "data_ready".into(),
                    }],
                }),
                Section::PrimitiveTypeAlias(PrimitiveTypeAliasSection {
                    name: "page_size".into(),
                    value: PrimitiveDirectiveItem {
                        atoms: vec![
                            PrimitiveDirectiveAtom::Word("bytes".into()),
                            PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::LParen),
                            PrimitiveDirectiveAtom::Number("4096".into()),
                            PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::RParen),
                        ],
                    },
                }),
                Section::Operation(PrimitiveOperationSection {
                    name: vec!["uart0".into(), "emit_byte".into()],
                    params: vec![PrimitiveOperationParam {
                        name: "value".into(),
                        ty: TypeName::Simple("byte".into()),
                    }],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("barrier".into())],
                    }],
                }),
                Section::Port(PrimitivePortSection {
                    name: vec!["uart0".into(), "data".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("volatile".into())],
                    }],
                }),
                Section::Signature(PrimitiveSignatureSection {
                    name: vec!["boot_magic".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("required".into())],
                    }],
                }),
                Section::Invariant(PrimitiveInvariantSection {
                    name: vec!["boot_sector_exact".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("check".into())],
                    }],
                }),
                Section::Constraint(PrimitiveConstraintSection {
                    name: vec!["page_entry_reserved".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("reserved".into())],
                    }],
                }),
                Section::Entry(PrimitiveEntrySection {
                    name: vec!["boot_entry".into()],
                    clauses: vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("mode".into())],
                    }],
                }),
                Section::Bridge(vec![PrimitiveBridgeTarget::RootPath(vec![
                    "root".into(),
                    "policy".into(),
                    "bind".into(),
                ])]),
                Section::Seal(vec!["primitiveClosed".into()]),
            ],
        };

        let primitive = container.as_primitive_at();
        assert!(
            primitive.is_some(),
            "expected primitive container to map to PrimitiveAtContainer"
        );
        let primitive = primitive.unwrap();
        assert_eq!(primitive.name, "primitiveCore");
        assert_eq!(primitive.sections.len(), 30);
    }

    #[test]
    fn container_as_primitive_at_rejects_non_primitive_or_unsupported_sections() {
        let non_primitive = Container {
            visibility: None,
            name: "rootCore".into(),
            context: Some(Context::Root),
            sections: vec![Section::Anchor(vec!["x".into()])],
        };
        assert!(non_primitive.as_primitive_at().is_none());

        let bad_primitive = Container {
            visibility: None,
            name: "badPrimitive".into(),
            context: Some(Context::Primitive),
            sections: vec![Section::Fn(vec![])],
        };
        assert!(bad_primitive.as_primitive_at().is_none());
    }

    #[test]
    fn container_as_primitive_at_maps_directive_sections() {
        let container = Container {
            visibility: None,
            name: "primitiveDirectiveCore".into(),
            context: Some(Context::Primitive),
            sections: vec![Section::PrimitiveDirective(PrimitiveDirectiveSection {
                kind: PrimitiveDirectiveKind::Id,
                items: vec![PrimitiveDirectiveItem {
                    atoms: vec![PrimitiveDirectiveAtom::Word("coreId".into())],
                }],
            })],
        };

        let primitive = container
            .as_primitive_at()
            .expect("expected primitive directive container to map");
        assert_eq!(primitive.sections.len(), 1);

        match &primitive.sections[0] {
            PrimitiveAtSection::Directive(section) => {
                assert_eq!(section.kind, PrimitiveDirectiveKind::Id);
                assert_eq!(
                    section.items,
                    vec![PrimitiveDirectiveItem {
                        atoms: vec![PrimitiveDirectiveAtom::Word("coreId".into())]
                    }]
                );
            }
            other => panic!("expected directive section, got {other:?}"),
        }
    }

    #[test]
    fn primitive_directive_item_clause_kind_maps_alias_directives() {
        let reads = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("reads".into()),
                PrimitiveDirectiveAtom::Word("uart0".into()),
                PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Dot),
                PrimitiveDirectiveAtom::Word("status".into()),
            ],
        };
        assert_eq!(
            reads.clause_kind(),
            Some(PrimitiveClauseKind::Directive(PrimitiveDirectiveKind::Read))
        );

        let address = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("address".into()),
                PrimitiveDirectiveAtom::Word("ioaddr".into()),
            ],
        };
        assert_eq!(
            address.clause_kind(),
            Some(PrimitiveClauseKind::Directive(PrimitiveDirectiveKind::Addr))
        );

        let faults = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("faults".into()),
                PrimitiveDirectiveAtom::Word("UartTimeout".into()),
            ],
        };
        assert_eq!(
            faults.clause_kind(),
            Some(PrimitiveClauseKind::Directive(
                PrimitiveDirectiveKind::Fault
            ))
        );

        let physical_failure = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("physicalFailure".into()),
                PrimitiveDirectiveAtom::Word("thermalTrip".into()),
            ],
        };
        assert_eq!(
            physical_failure.clause_kind(),
            Some(PrimitiveClauseKind::Directive(
                PrimitiveDirectiveKind::PhysicalFault
            ))
        );
    }

    #[test]
    fn primitive_directive_item_clause_kind_maps_material_shapes() {
        let await_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("await".into()),
                PrimitiveDirectiveAtom::Word("uart0".into()),
                PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Dot),
                PrimitiveDirectiveAtom::Word("tx_ready".into()),
                PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::EqEq),
                PrimitiveDirectiveAtom::Word("high".into()),
                PrimitiveDirectiveAtom::Word("timeout".into()),
                PrimitiveDirectiveAtom::Number("100000".into()),
                PrimitiveDirectiveAtom::Word("cycles".into()),
            ],
        };
        assert_eq!(
            await_clause.clause_kind(),
            Some(PrimitiveClauseKind::AwaitTimeoutCycles)
        );

        let read_side_effect_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("read".into()),
                PrimitiveDirectiveAtom::Word("side_effect".into()),
                PrimitiveDirectiveAtom::Word("none".into()),
            ],
        };
        assert_eq!(
            read_side_effect_clause.clause_kind(),
            Some(PrimitiveClauseKind::ReadSideEffect)
        );

        let immutable_after_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("immutable".into()),
                PrimitiveDirectiveAtom::Word("after".into()),
                PrimitiveDirectiveAtom::Word("boot".into()),
            ],
        };
        assert_eq!(
            immutable_after_clause.clause_kind(),
            Some(PrimitiveClauseKind::ImmutableAfter)
        );

        let event_interrupt_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("event".into()),
                PrimitiveDirectiveAtom::Word("interrupt".into()),
                PrimitiveDirectiveAtom::Word("irq0".into()),
            ],
        };
        assert_eq!(
            event_interrupt_clause.clause_kind(),
            Some(PrimitiveClauseKind::EventInterrupt)
        );

        let event_mmio_flag_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("event".into()),
                PrimitiveDirectiveAtom::Word("mmioReadClearsFlag".into()),
                PrimitiveDirectiveAtom::Word("status".into()),
                PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Dot),
                PrimitiveDirectiveAtom::Word("ready".into()),
            ],
        };
        assert_eq!(
            event_mmio_flag_clause.clause_kind(),
            Some(PrimitiveClauseKind::EventMmioReadClearsFlag)
        );

        let physical_event_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("physicalEvent".into()),
                PrimitiveDirectiveAtom::Word("interrupt".into()),
                PrimitiveDirectiveAtom::Word("irq0".into()),
            ],
        };
        assert_eq!(
            physical_event_clause.clause_kind(),
            Some(PrimitiveClauseKind::PhysicalEventContract)
        );

        let async_event_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("asyncHardwareEvent".into()),
                PrimitiveDirectiveAtom::Word("interrupt".into()),
                PrimitiveDirectiveAtom::Word("irq0".into()),
            ],
        };
        assert_eq!(
            async_event_clause.clause_kind(),
            Some(PrimitiveClauseKind::AsyncHardwareEventContract)
        );

        let side_effect_contract_clause = PrimitiveDirectiveItem {
            atoms: vec![
                PrimitiveDirectiveAtom::Word("sideEffect".into()),
                PrimitiveDirectiveAtom::Word("read_may_affect_device_state".into()),
            ],
        };
        assert_eq!(
            side_effect_contract_clause.clause_kind(),
            Some(PrimitiveClauseKind::SideEffectContract)
        );
    }
}

#[cfg(test)]
mod security_at_ast_tests {
    use super::*;

    #[test]
    fn container_as_security_at_maps_supported_sections() {
        let container = Container {
            visibility: None,
            name: "securityCore".into(),
            context: Some(Context::Security),
            sections: vec![
                Section::Stc(vec![]),
                Section::SecurityAsset(vec!["paymentApi".into()]),
                Section::SecurityIdentity(vec!["serviceOwner".into()]),
                Section::SecuritySecret(vec!["kmsPrimary".into()]),
                Section::SecurityFinding(vec!["suspiciousMailboxRule".into()]),
                Section::SecurityIncident(vec!["credentialTheft".into()]),
                Section::SecurityPolicy(vec!["strictAuth".into()]),
                Section::SecurityEvidence(vec!["evd_boot_chain".into()]),
                Section::Probe(vec![]),
                Section::Fn(vec![]),
                Section::Final(vec![]),
            ],
        };

        let security = container.as_security_at();
        assert!(
            security.is_some(),
            "expected security container to map to SecurityAtContainer"
        );
        let security = security.unwrap();
        assert_eq!(security.name, "securityCore");
        assert_eq!(security.sections.len(), 11);
    }

    #[test]
    fn container_as_security_at_rejects_non_security_or_unsupported_sections() {
        let non_security = Container {
            visibility: None,
            name: "rootCore".into(),
            context: Some(Context::Root),
            sections: vec![Section::SecurityPolicy(vec!["strictAuth".into()])],
        };
        assert!(non_security.as_security_at().is_none());

        let bad_security = Container {
            visibility: None,
            name: "badSecurity".into(),
            context: Some(Context::Security),
            sections: vec![Section::Binary(vec![])],
        };
        assert!(bad_security.as_security_at().is_none());
    }
}

#[cfg(test)]
mod heap_manager_op_ast_tests {
    use super::*;

    #[test]
    fn heap_manager_op_from_qualified_path_maps_official_targets() {
        let alloc = vec![
            "memory".to_string(),
            "heap".to_string(),
            "manager".to_string(),
            "alloc".to_string(),
        ];
        let free = vec![
            "memory".to_string(),
            "heap".to_string(),
            "manager".to_string(),
            "free".to_string(),
        ];
        let realloc = vec![
            "memory".to_string(),
            "heap".to_string(),
            "manager".to_string(),
            "realloc".to_string(),
        ];
        let zero = vec![
            "memory".to_string(),
            "heap".to_string(),
            "manager".to_string(),
            "zero".to_string(),
        ];

        assert_eq!(
            HeapManagerOp::from_qualified_path(&alloc),
            Some(HeapManagerOp::Alloc)
        );
        assert_eq!(
            HeapManagerOp::from_qualified_path(&free),
            Some(HeapManagerOp::Free)
        );
        assert_eq!(
            HeapManagerOp::from_qualified_path(&realloc),
            Some(HeapManagerOp::Realloc)
        );
        assert_eq!(
            HeapManagerOp::from_qualified_path(&zero),
            Some(HeapManagerOp::Zero)
        );
    }

    #[test]
    fn heap_manager_op_pointer_guard_flags_match_base() {
        assert!(!HeapManagerOp::Alloc.requires_valid_pointer());
        assert!(!HeapManagerOp::Alloc.invalidates_alias_group());

        assert!(HeapManagerOp::Free.requires_valid_pointer());
        assert!(HeapManagerOp::Free.invalidates_alias_group());

        assert!(HeapManagerOp::Realloc.requires_valid_pointer());
        assert!(HeapManagerOp::Realloc.invalidates_alias_group());

        assert!(HeapManagerOp::Zero.requires_valid_pointer());
        assert!(!HeapManagerOp::Zero.invalidates_alias_group());
    }
}

#[cfg(test)]
mod inspect_runtime_path_ast_tests {
    use super::*;

    #[test]
    fn inspect_runtime_path_from_segments_maps_official_paths() {
        let heap_tail = vec!["heap".to_string(), "events".to_string(), "tail".to_string()];
        let heap_count = vec![
            "heap".to_string(),
            "events".to_string(),
            "count".to_string(),
        ];
        let net_connected = vec!["net".to_string(), "connected".to_string()];
        let security_last_action = vec!["security".to_string(), "last_action".to_string()];
        let security_deny_count = vec![
            "security".to_string(),
            "deny".to_string(),
            "count".to_string(),
        ];
        let system_last_event = vec!["system".to_string(), "last_event".to_string()];
        let system_events_count = vec![
            "system".to_string(),
            "events".to_string(),
            "count".to_string(),
        ];
        let tooling_build_ready = vec![
            "tooling".to_string(),
            "build".to_string(),
            "ready".to_string(),
        ];
        let tooling_events_count = vec![
            "tooling".to_string(),
            "events".to_string(),
            "count".to_string(),
        ];
        let compiler_parse_ready = vec![
            "compiler".to_string(),
            "parse".to_string(),
            "ready".to_string(),
        ];
        let compiler_events_count = vec![
            "compiler".to_string(),
            "events".to_string(),
            "count".to_string(),
        ];
        let kernel_scheduler_open = vec![
            "kernel".to_string(),
            "scheduler".to_string(),
            "open".to_string(),
        ];
        let kernel_events_count = vec![
            "kernel".to_string(),
            "events".to_string(),
            "count".to_string(),
        ];
        let ai_session_active = vec![
            "ai".to_string(),
            "session".to_string(),
            "active".to_string(),
        ];
        let ai_events_count = vec!["ai".to_string(), "events".to_string(), "count".to_string()];

        assert_eq!(
            inspect_runtime_path_from_segments(&heap_tail),
            Some(InspectRuntimePath::HeapEventsTail)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&heap_count),
            Some(InspectRuntimePath::HeapEventsCount)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&net_connected),
            Some(InspectRuntimePath::NetConnected)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&security_last_action),
            Some(InspectRuntimePath::SecurityLastAction)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&security_deny_count),
            Some(InspectRuntimePath::SecurityDenyCount)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&system_last_event),
            Some(InspectRuntimePath::SystemLastEvent)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&system_events_count),
            Some(InspectRuntimePath::SystemEventsCount)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&tooling_build_ready),
            Some(InspectRuntimePath::ToolingBuildReady)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&tooling_events_count),
            Some(InspectRuntimePath::ToolingEventsCount)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&compiler_parse_ready),
            Some(InspectRuntimePath::CompilerParseReady)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&compiler_events_count),
            Some(InspectRuntimePath::CompilerEventsCount)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&kernel_scheduler_open),
            Some(InspectRuntimePath::KernelSchedulerOpen)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&kernel_events_count),
            Some(InspectRuntimePath::KernelEventsCount)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&ai_session_active),
            Some(InspectRuntimePath::AiSessionActive)
        );
        assert_eq!(
            inspect_runtime_path_from_segments(&ai_events_count),
            Some(InspectRuntimePath::AiEventsCount)
        );
    }

    #[test]
    fn inspect_runtime_path_from_segments_rejects_unknown_paths() {
        let unknown_heap = vec![
            "heap".to_string(),
            "events".to_string(),
            "ghost".to_string(),
        ];
        let unknown_net = vec!["net".to_string(), "ghost".to_string()];
        let unknown_security = vec![
            "security".to_string(),
            "unknown".to_string(),
            "count".to_string(),
        ];
        let unknown_system = vec!["system".to_string(), "ghost".to_string()];
        let unknown_tooling = vec!["tooling".to_string(), "ghost".to_string()];
        let unknown_compiler = vec!["compiler".to_string(), "ghost".to_string()];
        let unknown_kernel = vec!["kernel".to_string(), "ghost".to_string()];
        let unknown_ai = vec!["ai".to_string(), "ghost".to_string()];

        assert_eq!(inspect_runtime_path_from_segments(&unknown_heap), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_net), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_security), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_system), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_tooling), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_compiler), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_kernel), None);
        assert_eq!(inspect_runtime_path_from_segments(&unknown_ai), None);
    }
}
