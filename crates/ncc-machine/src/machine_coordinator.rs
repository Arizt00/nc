use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MachineFeature {
    PrivilegeLevels,
    UserMode,
    Mmu,
    Mpu,
    Atomics,
    Interrupts,
    Ipi,
    TlbInvalidation,
    SyscallTrap,
    ElfLoader,
    Pcie,
    Mmio,
    Dma,
    CacheMaintenance,
    BlockIo,
    NetworkIo,
    UsbHost,
    Framebuffer,
    FirmwareVectorTable,
    Thumb2,
    ArmState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineEndian {
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineBinaryFormat {
    Elf32,
    Elf64,
    Coff64,
    MachO64,
    RawFirmware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineProfile {
    pub target: &'static str,
    pub architecture: &'static str,
    pub abi: &'static str,
    pub pointer_width: u8,
    pub endian: MachineEndian,
    pub object_format: MachineBinaryFormat,
    pub executable_format: MachineBinaryFormat,
    pub registers: &'static [&'static str],
    pub instructions: &'static [&'static str],
    pub relocations: &'static [&'static str],
    pub features: &'static [MachineFeature],
}

impl MachineProfile {
    pub fn supports(self, feature: MachineFeature) -> bool {
        self.features.contains(&feature)
    }
}

impl MachineFeature {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::PrivilegeLevels => "privilege-levels",
            Self::UserMode => "user-mode",
            Self::Mmu => "mmu",
            Self::Mpu => "mpu",
            Self::Atomics => "atomics",
            Self::Interrupts => "interrupts",
            Self::Ipi => "ipi",
            Self::TlbInvalidation => "tlb-invalidation",
            Self::SyscallTrap => "syscall-trap",
            Self::ElfLoader => "elf-loader",
            Self::Pcie => "pcie",
            Self::Mmio => "mmio",
            Self::Dma => "dma",
            Self::CacheMaintenance => "cache-maintenance",
            Self::BlockIo => "block-io",
            Self::NetworkIo => "network-io",
            Self::UsbHost => "usb-host",
            Self::Framebuffer => "framebuffer",
            Self::FirmwareVectorTable => "firmware-vector-table",
            Self::Thumb2 => "thumb2",
            Self::ArmState => "arm-state",
        }
    }
}

const FULL_KERNEL_FEATURES: &[MachineFeature] = &[
    MachineFeature::PrivilegeLevels,
    MachineFeature::UserMode,
    MachineFeature::Mmu,
    MachineFeature::Atomics,
    MachineFeature::Interrupts,
    MachineFeature::Ipi,
    MachineFeature::TlbInvalidation,
    MachineFeature::SyscallTrap,
    MachineFeature::ElfLoader,
    MachineFeature::Pcie,
    MachineFeature::Mmio,
    MachineFeature::Dma,
    MachineFeature::CacheMaintenance,
    MachineFeature::BlockIo,
    MachineFeature::NetworkIo,
    MachineFeature::UsbHost,
    MachineFeature::Framebuffer,
    MachineFeature::FirmwareVectorTable,
];

const ARMV7A_FEATURES: &[MachineFeature] = &[
    MachineFeature::PrivilegeLevels,
    MachineFeature::UserMode,
    MachineFeature::Mmu,
    MachineFeature::Atomics,
    MachineFeature::Interrupts,
    MachineFeature::Ipi,
    MachineFeature::TlbInvalidation,
    MachineFeature::SyscallTrap,
    MachineFeature::ElfLoader,
    MachineFeature::Pcie,
    MachineFeature::Mmio,
    MachineFeature::Dma,
    MachineFeature::CacheMaintenance,
    MachineFeature::BlockIo,
    MachineFeature::NetworkIo,
    MachineFeature::UsbHost,
    MachineFeature::Framebuffer,
    MachineFeature::FirmwareVectorTable,
    MachineFeature::Thumb2,
    MachineFeature::ArmState,
];

const ARMV7M_FEATURES: &[MachineFeature] = &[
    MachineFeature::PrivilegeLevels,
    MachineFeature::Mpu,
    MachineFeature::Atomics,
    MachineFeature::Interrupts,
    MachineFeature::SyscallTrap,
    MachineFeature::Mmio,
    MachineFeature::Dma,
    MachineFeature::CacheMaintenance,
    MachineFeature::BlockIo,
    MachineFeature::NetworkIo,
    MachineFeature::UsbHost,
    MachineFeature::Framebuffer,
    MachineFeature::FirmwareVectorTable,
    MachineFeature::Thumb2,
];

const X86_64_REGISTERS: &[&str] = &[
    "rax",
    "rbx",
    "rcx",
    "rdx",
    "rsi",
    "rdi",
    "rbp",
    "rsp",
    "r8",
    "r9",
    "r10",
    "r11",
    "r12",
    "r13",
    "r14",
    "r15",
    "rip",
    "rflags",
    "cr0",
    "cr2",
    "cr3",
    "cr4",
    "cr8",
    "fs_base",
    "gs_base",
    "kernel_gs_base",
    "xmm0..xmm31",
];
const AARCH64_REGISTERS: &[&str] = &[
    "x0..x30",
    "sp",
    "pc",
    "pstate",
    "ttbr0_el1",
    "ttbr1_el1",
    "tpidr_el0",
    "tpidrro_el0",
    "sp_el0",
    "elr_el1",
    "spsr_el1",
    "cntv_cval_el0",
    "cntv_ctl_el0",
    "v0..v31",
];
const RISCV64_REGISTERS: &[&str] = &[
    "x0..x31",
    "pc",
    "f0..f31",
    "sstatus/sie/stvec/scounteren/sscratch/sepc/scause/stval/sip/satp",
    "mstatus/misa/medeleg/mideleg/mie/mtvec/mcounteren/mscratch/mepc/mcause/mtval/mip",
    "hstatus/hedeleg/hideleg/hie/hcounteren/hgeie/htval/hip/hvip/htinst/hgatp",
    "vsstatus/vsie/vstvec/vsscratch/vsepc/vscause/vstval/vsip/vsatp",
];
const I386_REGISTERS: &[&str] = &[
    "eax",
    "ebx",
    "ecx",
    "edx",
    "esi",
    "edi",
    "ebp",
    "esp",
    "eip",
    "eflags",
    "cr0",
    "cr2",
    "cr3",
    "cr4",
    "xmm0..xmm7",
];
const RISCV32_REGISTERS: &[&str] = &[
    "x0..x31",
    "pc",
    "f0..f31",
    "sstatus/sie/stvec/scounteren/sscratch/sepc/scause/stval/sip/satp",
    "mstatus/misa/medeleg/mideleg/mie/mtvec/mcounteren/mscratch/mepc/mcause/mtval/mip",
    "hstatus/hedeleg/hideleg/hie/hcounteren/hgeie/htval/hip/hvip/htinst/hgatp",
    "vsstatus/vsie/vstvec/vsscratch/vsepc/vscause/vstval/vsip/vsatp",
];
const ARMV7_REGISTERS: &[&str] = &["r0..r12", "sp", "lr", "pc", "cpsr", "s0..s31"];
const THUMBV7M_REGISTERS: &[&str] = &[
    "r0..r12",
    "sp",
    "lr",
    "pc",
    "xpsr",
    "control",
    "primask",
    "basepri",
    "faultmask",
    "s0..s31",
];

const X86_64_KERNEL_INSTRUCTIONS: &[&str] = &[
    "mov",
    "lea",
    "push",
    "pop",
    "call",
    "ret",
    "jmp",
    "jcc",
    "cmp",
    "test",
    "setcc",
    "cmovcc",
    "add",
    "sub",
    "imul",
    "and",
    "or",
    "xor",
    "shl",
    "shr",
    "sar",
    "lock",
    "cmpxchg",
    "xadd",
    "pause",
    "mfence",
    "lfence",
    "sfence",
    "invlpg",
    "invpcid",
    "cli",
    "sti",
    "hlt",
    "int",
    "iretq",
    "syscall",
    "sysretq",
    "swapgs",
    "rdmsr",
    "wrmsr",
    "lgdt",
    "lidt",
    "ltr",
    "in",
    "out",
    "insb",
    "insw",
    "insd",
    "outsb",
    "outsw",
    "outsd",
    "xsave",
    "xrstor",
    "rdfsbase",
    "rdgsbase",
    "wrfsbase",
    "wrgsbase",
    "umonitor",
    "umwait",
    "tpause",
    "serialize",
];
const I386_KERNEL_INSTRUCTIONS: &[&str] = &[
    "mov", "lea", "push", "pop", "call", "ret", "jmp", "jcc", "cmp", "test", "setcc", "add", "sub",
    "imul", "and", "or", "xor", "shl", "shr", "sar", "lock", "cmpxchg", "xadd", "pause", "mfence",
    "invlpg", "cli", "sti", "hlt", "int", "iret", "sysenter", "sysexit", "rdmsr", "wrmsr", "lgdt",
    "lidt", "ltr", "in", "out", "insb", "insw", "insd", "outsb", "outsw", "outsd", "fxsave",
    "fxrstor",
];
const AARCH64_KERNEL_INSTRUCTIONS: &[&str] = &[
    "adrp", "add", "sub", "mov", "ldr", "str", "ldp", "stp", "bl", "br", "ret", "b", "b.cond",
    "cbz", "cbnz", "mrs", "msr", "svc", "eret", "dmb", "dsb", "isb", "tlbi", "ldxr", "stxr",
    "ldaxr", "stlxr", "cas", "casa", "casl", "casal", "swp", "swpa", "swpl", "swpal", "ldadd",
    "ldadda", "ldaddl", "ldaddal", "ldclr", "ldclra", "ldclrl", "ldclral", "ldeor", "ldeora",
    "ldeorl", "ldeoral", "ldset", "ldseta", "ldsetl", "ldsetal", "wfi", "wfe", "sev", "sevl",
    "yield", "dc", "ic",
];
const RISCV_KERNEL_INSTRUCTIONS: &[&str] = &[
    "lui",
    "auipc",
    "addi",
    "add",
    "sub",
    "and",
    "or",
    "xor",
    "sll",
    "srl",
    "sra",
    "lw",
    "sw",
    "jal",
    "jalr",
    "beq",
    "bne",
    "blt",
    "bge",
    "ecall",
    "sret",
    "csrrw",
    "csrrs",
    "sfence.vma",
    "fence",
    "fence.i",
    "cbo.clean",
    "cbo.flush",
    "cbo.inval",
    "cbo.zero",
    "lr.w",
    "sc.w",
    "amoadd.w",
    "amoswap.w",
    "amoxor.w",
    "amoand.w",
    "amoor.w",
    "amomin.w",
    "amomax.w",
    "amominu.w",
    "amomaxu.w",
    "wfi",
];
const RISCV64_KERNEL_INSTRUCTIONS: &[&str] = &[
    "lui",
    "auipc",
    "addi",
    "add",
    "sub",
    "and",
    "or",
    "xor",
    "sll",
    "srl",
    "sra",
    "ld",
    "sd",
    "lw",
    "sw",
    "jal",
    "jalr",
    "beq",
    "bne",
    "blt",
    "bge",
    "ecall",
    "sret",
    "csrrw",
    "csrrs",
    "sfence.vma",
    "fence",
    "fence.i",
    "cbo.clean",
    "cbo.flush",
    "cbo.inval",
    "cbo.zero",
    "lr.d",
    "sc.d",
    "amoadd.d",
    "amoswap.d",
    "amoxor.d",
    "amoand.d",
    "amoor.d",
    "amomin.d",
    "amomax.d",
    "amominu.d",
    "amomaxu.d",
    "wfi",
];
const ARMV7A_KERNEL_INSTRUCTIONS: &[&str] = &[
    "mov", "movw", "movt", "add", "sub", "cmp", "tst", "ldr", "str", "ldm", "stm", "push", "pop",
    "b", "bl", "bx", "blx", "svc", "rfe", "mrs", "msr", "mrc", "mcr", "cpsid", "cpsie", "dmb",
    "dsb", "isb", "ldrex", "strex", "wfi", "wfe", "sev",
];
const THUMBV7M_KERNEL_INSTRUCTIONS: &[&str] = &[
    "nop", "movs", "movw", "movt", "cmp", "adds", "subs", "ldr", "str", "ldrb", "strb", "ldrh",
    "strh", "push", "pop", "b", "b.cond", "bl", "bx", "svc", "bkpt", "udf", "mrs", "msr", "cpsid",
    "cpsie", "dmb", "dsb", "isb", "ldrex", "strex", "wfi", "wfe", "sev",
];

const X86_RELOCS: &[&str] = &[
    "R_X86_64_64",
    "R_X86_64_PC32",
    "R_X86_64_PLT32",
    "R_X86_64_GOTPCRELX",
    "R_X86_64_RELATIVE",
];
const AARCH64_RELOCS: &[&str] = &[
    "R_AARCH64_ABS64",
    "R_AARCH64_PREL32",
    "R_AARCH64_CALL26",
    "R_AARCH64_JUMP26",
    "R_AARCH64_ADR_PREL_PG_HI21",
    "R_AARCH64_ADD_ABS_LO12_NC",
];
const RISCV_RELOCS: &[&str] = &[
    "R_RISCV_64",
    "R_RISCV_RELATIVE",
    "R_RISCV_CALL_PLT",
    "R_RISCV_PCREL_HI20",
    "R_RISCV_PCREL_LO12_I",
    "R_RISCV_RELAX",
];
const I386_RELOCS: &[&str] = &[
    "R_386_32",
    "R_386_PC32",
    "R_386_PLT32",
    "R_386_GOT32",
    "R_386_RELATIVE",
];
const RISCV32_RELOCS: &[&str] = &[
    "R_RISCV_32",
    "R_RISCV_BRANCH",
    "R_RISCV_JAL",
    "R_RISCV_CALL",
    "R_RISCV_HI20",
    "R_RISCV_LO12_I",
    "R_RISCV_PCREL_HI20",
    "R_RISCV_PCREL_LO12_I",
    "R_RISCV_RELAX",
];
const ARM_RELOCS: &[&str] = &[
    "R_ARM_ABS32",
    "R_ARM_REL32",
    "R_ARM_THM_CALL",
    "R_ARM_THM_JUMP24",
    "R_ARM_MOVW_ABS_NC",
    "R_ARM_MOVT_ABS",
    "R_ARM_PREL31",
];

pub fn machine_profile(target: &str) -> Option<MachineProfile> {
    let target = target.trim().to_ascii_lowercase();
    if target.starts_with("aarch64-linux-android") {
        return Some(MachineProfile {
            target: "aarch64-linux-android",
            architecture: "aarch64",
            abi: "AAPCS64-android",
            pointer_width: 64,
            endian: MachineEndian::Little,
            object_format: MachineBinaryFormat::Elf64,
            executable_format: MachineBinaryFormat::Elf64,
            registers: AARCH64_REGISTERS,
            instructions: AARCH64_KERNEL_INSTRUCTIONS,
            relocations: AARCH64_RELOCS,
            features: FULL_KERNEL_FEATURES,
        });
    }
    match target.as_str() {
        "x86_64-unknown-none" | "x86_64-unknown-none-elf" | "x86_64-unknown-linux-gnu" => {
            Some(MachineProfile {
                target: "x86_64-unknown-none",
                architecture: "x86_64",
                abi: "nC-kernel-sysv64",
                pointer_width: 64,
                endian: MachineEndian::Little,
                object_format: MachineBinaryFormat::Elf64,
                executable_format: MachineBinaryFormat::Elf64,
                registers: X86_64_REGISTERS,
                instructions: X86_64_KERNEL_INSTRUCTIONS,
                relocations: X86_RELOCS,
                features: FULL_KERNEL_FEATURES,
            })
        }
        "i386-unknown-none" | "i386-unknown-none-elf" | "i386-unknown-linux-gnu" => {
            Some(MachineProfile {
                target: "i386-unknown-none-elf",
                architecture: "i386",
                abi: "i386-sysv-kernel",
                pointer_width: 32,
                endian: MachineEndian::Little,
                object_format: MachineBinaryFormat::Elf32,
                executable_format: MachineBinaryFormat::Elf32,
                registers: I386_REGISTERS,
                instructions: I386_KERNEL_INSTRUCTIONS,
                relocations: I386_RELOCS,
                features: FULL_KERNEL_FEATURES,
            })
        }
        "aarch64-unknown-none" | "aarch64-unknown-none-elf" | "aarch64-unknown-linux-gnu" => {
            Some(MachineProfile {
                target: "aarch64-unknown-none",
                architecture: "aarch64",
                abi: "AAPCS64-kernel",
                pointer_width: 64,
                endian: MachineEndian::Little,
                object_format: MachineBinaryFormat::Elf64,
                executable_format: MachineBinaryFormat::Elf64,
                registers: AARCH64_REGISTERS,
                instructions: AARCH64_KERNEL_INSTRUCTIONS,
                relocations: AARCH64_RELOCS,
                features: FULL_KERNEL_FEATURES,
            })
        }
        "riscv64-unknown-elf" | "riscv64gc-unknown-none-elf" => Some(MachineProfile {
            target: "riscv64-unknown-elf",
            architecture: "riscv64",
            abi: "lp64d-kernel",
            pointer_width: 64,
            endian: MachineEndian::Little,
            object_format: MachineBinaryFormat::Elf64,
            executable_format: MachineBinaryFormat::Elf64,
            registers: RISCV64_REGISTERS,
            instructions: RISCV64_KERNEL_INSTRUCTIONS,
            relocations: RISCV_RELOCS,
            features: FULL_KERNEL_FEATURES,
        }),
        "riscv32imac-unknown-elf" | "riscv32imac-unknown-none-elf" => Some(MachineProfile {
            target: "riscv32imac-unknown-none-elf",
            architecture: "riscv32imac",
            abi: "ilp32-kernel",
            pointer_width: 32,
            endian: MachineEndian::Little,
            object_format: MachineBinaryFormat::Elf32,
            executable_format: MachineBinaryFormat::Elf32,
            registers: RISCV32_REGISTERS,
            instructions: RISCV_KERNEL_INSTRUCTIONS,
            relocations: RISCV32_RELOCS,
            features: FULL_KERNEL_FEATURES,
        }),
        "armv7a-unknown-none-eabi" | "armv7-unknown-none-eabi" | "armv7a-none-eabi" => {
            Some(MachineProfile {
                target: "armv7a-unknown-none-eabi",
                architecture: "armv7a-thumb2",
                abi: "AAPCS32-kernel",
                pointer_width: 32,
                endian: MachineEndian::Little,
                object_format: MachineBinaryFormat::Elf32,
                executable_format: MachineBinaryFormat::Elf32,
                registers: ARMV7_REGISTERS,
                instructions: ARMV7A_KERNEL_INSTRUCTIONS,
                relocations: ARM_RELOCS,
                features: ARMV7A_FEATURES,
            })
        }
        "armv7em-none-eabi" | "thumbv7em-none-eabi" => Some(MachineProfile {
            target: "armv7em-none-eabi",
            architecture: "thumbv7em",
            abi: "AAPCS32-M",
            pointer_width: 32,
            endian: MachineEndian::Little,
            object_format: MachineBinaryFormat::Elf32,
            executable_format: MachineBinaryFormat::RawFirmware,
            registers: THUMBV7M_REGISTERS,
            instructions: THUMBV7M_KERNEL_INSTRUCTIONS,
            relocations: ARM_RELOCS,
            features: ARMV7M_FEATURES,
        }),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KernelSubsystem {
    PhysicalMemory,
    VirtualMemory,
    Processes,
    ElfLoader,
    Cow,
    Syscalls,
    MulticoreScheduler,
    Synchronization,
    InterruptController,
    Timer,
    Serial,
    Keyboard,
    Vfs,
    Initramfs,
    Storage,
    BlockCache,
    Fat,
    Ext2,
    Pcie,
    Nvme,
    Ahci,
    Virtio,
    NicDrivers,
    Ethernet,
    Arp,
    Ipv4,
    Ipv6,
    Udp,
    Tcp,
    Sockets,
    Network,
    Usb,
    Hid,
    Framebuffer,
    Firmware,
}

impl KernelSubsystem {
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::PhysicalMemory => "physical-memory",
            Self::VirtualMemory => "virtual-memory",
            Self::Processes => "processes",
            Self::ElfLoader => "elf-loader",
            Self::Cow => "cow",
            Self::Syscalls => "syscalls",
            Self::MulticoreScheduler => "multicore-scheduler",
            Self::Synchronization => "synchronization",
            Self::InterruptController => "interrupt-controller",
            Self::Timer => "timer",
            Self::Serial => "serial",
            Self::Keyboard => "keyboard",
            Self::Vfs => "vfs",
            Self::Initramfs => "initramfs",
            Self::Storage => "storage",
            Self::BlockCache => "block-cache",
            Self::Fat => "fat",
            Self::Ext2 => "ext2",
            Self::Pcie => "pcie",
            Self::Nvme => "nvme",
            Self::Ahci => "ahci",
            Self::Virtio => "virtio",
            Self::NicDrivers => "nic-drivers",
            Self::Ethernet => "ethernet",
            Self::Arp => "arp",
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
            Self::Udp => "udp",
            Self::Tcp => "tcp",
            Self::Sockets => "sockets",
            Self::Network => "network",
            Self::Usb => "usb",
            Self::Hid => "hid",
            Self::Framebuffer => "framebuffer",
            Self::Firmware => "firmware",
        }
    }
}

pub const COMPLETE_KERNEL_SUBSYSTEMS: &[KernelSubsystem] = &[
    KernelSubsystem::PhysicalMemory,
    KernelSubsystem::VirtualMemory,
    KernelSubsystem::Processes,
    KernelSubsystem::ElfLoader,
    KernelSubsystem::Cow,
    KernelSubsystem::Syscalls,
    KernelSubsystem::MulticoreScheduler,
    KernelSubsystem::Synchronization,
    KernelSubsystem::InterruptController,
    KernelSubsystem::Timer,
    KernelSubsystem::Serial,
    KernelSubsystem::Keyboard,
    KernelSubsystem::Vfs,
    KernelSubsystem::Initramfs,
    KernelSubsystem::Storage,
    KernelSubsystem::BlockCache,
    KernelSubsystem::Fat,
    KernelSubsystem::Ext2,
    KernelSubsystem::Pcie,
    KernelSubsystem::Nvme,
    KernelSubsystem::Ahci,
    KernelSubsystem::Virtio,
    KernelSubsystem::NicDrivers,
    KernelSubsystem::Ethernet,
    KernelSubsystem::Arp,
    KernelSubsystem::Ipv4,
    KernelSubsystem::Ipv6,
    KernelSubsystem::Udp,
    KernelSubsystem::Tcp,
    KernelSubsystem::Sockets,
    KernelSubsystem::Network,
    KernelSubsystem::Usb,
    KernelSubsystem::Hid,
    KernelSubsystem::Framebuffer,
    KernelSubsystem::Firmware,
];

fn subsystem_dependencies(subsystem: KernelSubsystem) -> &'static [KernelSubsystem] {
    use KernelSubsystem::*;
    match subsystem {
        PhysicalMemory | Firmware => &[],
        VirtualMemory => &[PhysicalMemory],
        Processes => &[VirtualMemory, ElfLoader],
        ElfLoader => &[VirtualMemory],
        Cow => &[Processes, VirtualMemory],
        Syscalls => &[Processes],
        MulticoreScheduler => &[Processes, Syscalls, Synchronization],
        Synchronization => &[InterruptController],
        InterruptController => &[PhysicalMemory, Firmware],
        Timer => &[InterruptController],
        Serial => &[PhysicalMemory, Firmware],
        Keyboard => &[InterruptController],
        Vfs => &[Processes, Syscalls],
        Initramfs => &[Vfs],
        Pcie => &[PhysicalMemory],
        Storage => &[Vfs, PhysicalMemory],
        BlockCache => &[Storage],
        Fat | Ext2 => &[Vfs, Storage, BlockCache],
        Nvme | Ahci => &[Storage, BlockCache, Pcie],
        Virtio => &[Storage, Pcie, InterruptController],
        NicDrivers => &[Pcie, InterruptController, Synchronization],
        Ethernet => &[NicDrivers],
        Arp => &[Ethernet, Timer],
        Ipv4 => &[Ethernet, Timer],
        Ipv6 => &[Ethernet, Timer],
        Udp => &[Ipv4, Ipv6],
        Tcp => &[Ipv4, Ipv6, Timer, Synchronization],
        Sockets => &[Udp, Tcp, Processes, Synchronization],
        Network => &[Sockets, Arp],
        Usb => &[Pcie],
        Hid => &[Usb],
        Framebuffer => &[PhysicalMemory],
    }
}

fn subsystem_features(subsystem: KernelSubsystem) -> &'static [MachineFeature] {
    use KernelSubsystem::*;
    match subsystem {
        PhysicalMemory => &[MachineFeature::Atomics],
        VirtualMemory | Cow => &[MachineFeature::Mmu, MachineFeature::TlbInvalidation],
        Processes => &[MachineFeature::UserMode, MachineFeature::PrivilegeLevels],
        ElfLoader => &[MachineFeature::ElfLoader],
        Syscalls => &[MachineFeature::SyscallTrap],
        MulticoreScheduler => &[MachineFeature::Atomics, MachineFeature::Ipi],
        Synchronization => &[
            MachineFeature::Atomics,
            MachineFeature::Interrupts,
            MachineFeature::Ipi,
        ],
        InterruptController => &[MachineFeature::Interrupts, MachineFeature::Mmio],
        Timer => &[MachineFeature::Interrupts],
        Serial | Keyboard => &[MachineFeature::Mmio, MachineFeature::Interrupts],
        Vfs => &[],
        Initramfs => &[],
        Storage | BlockCache | Fat | Ext2 | Nvme | Ahci => &[
            MachineFeature::Dma,
            MachineFeature::CacheMaintenance,
            MachineFeature::BlockIo,
        ],
        Virtio => &[
            MachineFeature::Dma,
            MachineFeature::Mmio,
            MachineFeature::Interrupts,
        ],
        Pcie => &[
            MachineFeature::Pcie,
            MachineFeature::Mmio,
            MachineFeature::Interrupts,
        ],
        NicDrivers => &[
            MachineFeature::Pcie,
            MachineFeature::Dma,
            MachineFeature::NetworkIo,
            MachineFeature::Interrupts,
        ],
        Ethernet | Arp | Ipv4 | Ipv6 | Udp | Tcp | Sockets | Network => {
            &[MachineFeature::NetworkIo]
        }
        Usb | Hid => &[MachineFeature::UsbHost, MachineFeature::Interrupts],
        Framebuffer => &[MachineFeature::Framebuffer, MachineFeature::Mmio],
        Firmware => &[MachineFeature::FirmwareVectorTable],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineCoordinationError {
    UnknownTarget(String),
    UnsupportedFeature {
        target: String,
        subsystem: KernelSubsystem,
        feature: MachineFeature,
    },
}

impl fmt::Display for MachineCoordinationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTarget(target) => write!(f, "unknown machine target '{target}'"),
            Self::UnsupportedFeature {
                target,
                subsystem,
                feature,
            } => write!(
                f,
                "target '{target}' cannot materialize {subsystem:?}: missing {feature:?}"
            ),
        }
    }
}

impl Error for MachineCoordinationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelMachinePlan {
    pub profile: MachineProfile,
    pub ordered_subsystems: Vec<KernelSubsystem>,
    pub required_features: BTreeSet<MachineFeature>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualKernelAuthority {
    pub profile: MachineProfile,
    pub source_coordinator: &'static str,
    pub source_mode: &'static str,
    pub compact_surface: bool,
    pub raw_memory: bool,
    pub unchecked_out_of_range: bool,
    pub unchecked_aliasing: bool,
    pub explicit_resource_lifetime: bool,
    pub explicit_interrupt_state: bool,
    pub explicit_register_state: bool,
    pub explicit_abi: bool,
    pub explicit_relocations: bool,
    pub hidden_bounds_checks: bool,
    pub hidden_overflow_checks: bool,
    pub hidden_heap: bool,
    pub hidden_cleanup: bool,
    pub hidden_scheduler: bool,
    pub hosted_runtime: bool,
}

impl ManualKernelAuthority {
    pub fn canonical_manifest(self) -> String {
        format!(
            "version=1 coordinator={} mode={} target={} abi={} compact={} raw_memory={} unchecked_out_of_range={} unchecked_aliasing={} explicit_resource_lifetime={} explicit_interrupt_state={} explicit_register_state={} explicit_abi={} explicit_relocations={} hidden_bounds_checks={} hidden_overflow_checks={} hidden_heap={} hidden_cleanup={} hidden_scheduler={} hosted_runtime={}",
            self.source_coordinator,
            self.source_mode,
            self.profile.target,
            self.profile.abi,
            self.compact_surface,
            self.raw_memory,
            self.unchecked_out_of_range,
            self.unchecked_aliasing,
            self.explicit_resource_lifetime,
            self.explicit_interrupt_state,
            self.explicit_register_state,
            self.explicit_abi,
            self.explicit_relocations,
            self.hidden_bounds_checks,
            self.hidden_overflow_checks,
            self.hidden_heap,
            self.hidden_cleanup,
            self.hidden_scheduler,
            self.hosted_runtime,
        )
    }
}

impl KernelMachinePlan {
    pub fn manual_authority(&self) -> ManualKernelAuthority {
        ManualKernelAuthority {
            profile: self.profile,
            source_coordinator: ".n",
            source_mode: "manual_raw",
            compact_surface: true,
            raw_memory: true,
            unchecked_out_of_range: true,
            unchecked_aliasing: true,
            explicit_resource_lifetime: true,
            explicit_interrupt_state: true,
            explicit_register_state: true,
            explicit_abi: true,
            explicit_relocations: true,
            hidden_bounds_checks: false,
            hidden_overflow_checks: false,
            hidden_heap: false,
            hidden_cleanup: false,
            hidden_scheduler: false,
            hosted_runtime: false,
        }
    }

    pub fn canonical_manifest(&self) -> String {
        let subsystems = self
            .ordered_subsystems
            .iter()
            .map(|subsystem| subsystem.canonical_name())
            .collect::<Vec<_>>()
            .join(",");
        let features = self
            .required_features
            .iter()
            .map(|feature| feature.canonical_name())
            .collect::<Vec<_>>()
            .join(",");
        let registers = self.profile.registers.join(",");
        let instructions = self.profile.instructions.join(",");
        let relocations = self.profile.relocations.join(",");
        format!(
            "version=1 target={} architecture={} abi={} pointer_width={} object={:?} executable={:?} subsystems=[{}] features=[{}] registers=[{}] instructions=[{}] relocations=[{}]",
            self.profile.target,
            self.profile.architecture,
            self.profile.abi,
            self.profile.pointer_width,
            self.profile.object_format,
            self.profile.executable_format,
            subsystems,
            features,
            registers,
            instructions,
            relocations
        )
    }

    pub fn llvm_authority_comment(&self) -> String {
        format!("; ncc-machine-authority {}\n", self.canonical_manifest())
    }
}

fn visit_subsystem(
    profile: MachineProfile,
    subsystem: KernelSubsystem,
    visited: &mut BTreeSet<KernelSubsystem>,
    ordered: &mut Vec<KernelSubsystem>,
    features: &mut BTreeSet<MachineFeature>,
) -> Result<(), MachineCoordinationError> {
    if visited.contains(&subsystem) {
        return Ok(());
    }
    for dependency in subsystem_dependencies(subsystem) {
        visit_subsystem(profile, *dependency, visited, ordered, features)?;
    }
    for feature in subsystem_features(subsystem) {
        if !profile.supports(*feature) {
            return Err(MachineCoordinationError::UnsupportedFeature {
                target: profile.target.to_string(),
                subsystem,
                feature: *feature,
            });
        }
        features.insert(*feature);
    }
    visited.insert(subsystem);
    ordered.push(subsystem);
    Ok(())
}

pub fn coordinate_kernel(
    target: &str,
    requested: &[KernelSubsystem],
) -> Result<KernelMachinePlan, MachineCoordinationError> {
    let profile = machine_profile(target)
        .ok_or_else(|| MachineCoordinationError::UnknownTarget(target.to_string()))?;
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();
    let mut features = BTreeSet::new();
    for subsystem in requested {
        visit_subsystem(
            profile,
            *subsystem,
            &mut visited,
            &mut ordered,
            &mut features,
        )?;
    }
    Ok(KernelMachinePlan {
        profile,
        ordered_subsystems: ordered,
        required_features: features,
    })
}

pub fn coordinate_complete_kernel(
    target: &str,
) -> Result<KernelMachinePlan, MachineCoordinationError> {
    coordinate_kernel(target, COMPLETE_KERNEL_SUBSYSTEMS)
}

pub fn coordinate_target_kernel(
    target: &str,
) -> Result<KernelMachinePlan, MachineCoordinationError> {
    let profile = machine_profile(target)
        .ok_or_else(|| MachineCoordinationError::UnknownTarget(target.to_string()))?;
    if profile.supports(MachineFeature::Mmu)
        && profile.supports(MachineFeature::UserMode)
        && profile.supports(MachineFeature::ElfLoader)
    {
        coordinate_complete_kernel(target)
    } else {
        coordinate_kernel(
            target,
            &[
                KernelSubsystem::PhysicalMemory,
                KernelSubsystem::Framebuffer,
                KernelSubsystem::Firmware,
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_kernel_profiles_close_every_dependency_in_order() {
        for target in [
            "x86_64-unknown-none",
            "aarch64-unknown-none",
            "riscv64-unknown-elf",
            "riscv32imac-unknown-none-elf",
            "i386-unknown-none-elf",
            "armv7a-unknown-none-eabi",
        ] {
            let plan = coordinate_complete_kernel(target).unwrap();
            assert_eq!(
                plan.ordered_subsystems.len(),
                COMPLETE_KERNEL_SUBSYSTEMS.len()
            );
            assert_eq!(plan.ordered_subsystems[0], KernelSubsystem::PhysicalMemory);
            assert!(plan.required_features.contains(&MachineFeature::Mmu));
            assert!(plan.required_features.contains(&MachineFeature::Pcie));
        }
    }

    #[test]
    fn cortex_m_accepts_firmware_but_rejects_hidden_mmu_requirements() {
        let firmware = coordinate_kernel(
            "thumbv7em-none-eabi",
            &[KernelSubsystem::Firmware, KernelSubsystem::Framebuffer],
        )
        .unwrap();
        assert!(firmware.profile.supports(MachineFeature::Thumb2));

        assert!(matches!(
            coordinate_kernel("thumbv7em-none-eabi", &[KernelSubsystem::Processes]),
            Err(MachineCoordinationError::UnsupportedFeature {
                feature: MachineFeature::Mmu,
                ..
            })
        ));
        let default = coordinate_target_kernel("thumbv7em-none-eabi").unwrap();
        assert_eq!(
            default.ordered_subsystems,
            vec![
                KernelSubsystem::PhysicalMemory,
                KernelSubsystem::Framebuffer,
                KernelSubsystem::Firmware,
            ]
        );
        assert!(default.profile.supports(MachineFeature::Mpu));
    }

    #[test]
    fn arm_profiles_publish_real_abi_register_and_relocation_contracts() {
        let profile = machine_profile("armv7em-none-eabi").unwrap();
        assert_eq!(profile.abi, "AAPCS32-M");
        assert!(profile.registers.contains(&"xpsr"));
        assert!(profile.relocations.contains(&"R_ARM_THM_CALL"));
        assert_eq!(profile.executable_format, MachineBinaryFormat::RawFirmware);
    }

    #[test]
    fn android_arm64_profile_is_aapcs64_elf_for_every_api_level() {
        for target in ["aarch64-linux-android29", "aarch64-linux-android35"] {
            let profile = machine_profile(target).expect("Android AArch64 profile");
            assert_eq!(profile.target, "aarch64-linux-android");
            assert_eq!(profile.abi, "AAPCS64-android");
            assert_eq!(profile.pointer_width, 64);
            assert_eq!(profile.object_format, MachineBinaryFormat::Elf64);
            assert!(profile.relocations.contains(&"R_AARCH64_CALL26"));
        }
    }

    #[test]
    fn authority_manifest_is_canonical_and_records_machine_owned_contracts() {
        let plan = coordinate_complete_kernel("x86_64-unknown-none-elf").unwrap();
        let manifest = plan.canonical_manifest();
        assert!(manifest.starts_with("version=1 target=x86_64-unknown-none"));
        assert!(manifest.contains("subsystems=[physical-memory,virtual-memory"));
        assert!(manifest.contains("features=[privilege-levels,user-mode,mmu"));
        assert!(manifest.contains("registers=[rax,rbx,rcx"));
        assert!(manifest.contains("instructions=[mov,lea,push,pop,call,ret"));
        assert!(manifest.contains("relocations=[R_X86_64_64,R_X86_64_PC32"));
        assert_eq!(
            plan.llvm_authority_comment(),
            format!("; ncc-machine-authority {manifest}\n")
        );
        let manual = plan.manual_authority();
        assert_eq!(manual.source_coordinator, ".n");
        assert_eq!(manual.source_mode, "manual_raw");
        assert!(manual.raw_memory && manual.unchecked_out_of_range);
        assert!(manual.explicit_abi && manual.explicit_relocations);
        assert!(!manual.hidden_heap && !manual.hidden_scheduler && !manual.hosted_runtime);
        assert!(manual
            .canonical_manifest()
            .contains("mode=manual_raw target=x86_64-unknown-none"));
    }
}
