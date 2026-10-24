pub const CRATE_ROLE: &str = "machine";

pub mod aarch64_el2;
pub mod armv7_thumb;
pub mod dma;
pub mod dynamic_memory;
pub mod elf_loader;
pub mod firmware;
pub mod guest_system;
pub mod hypervisor_backend;
pub mod iommu;
pub mod kernel_autonomous;
pub mod kernel_core;
pub mod kernel_debug;
mod kernel_filesystems;
pub mod kernel_hardening;
pub mod kernel_io;
pub mod kernel_network;
pub mod kernel_storage;
pub mod kernel_stress;
pub mod kernel_sync;
pub mod machine_coordinator;
pub mod manual_allocators;
pub mod nc_abi;
pub mod pci_virtio;
pub mod physical_drivers;
pub mod pointer_lowering;
pub mod raw_memory;
pub mod raw_memory_stress;
pub mod riscv_h;
pub mod smp;
pub mod static_memory;
pub mod uefi_loader;
pub mod virtio_mmio;
pub mod virtual_devices;
pub mod virtual_memory;
pub mod vm_lifecycle;
pub mod vm_migration;
pub mod x86_apic;
pub mod x86_guest_boot;
pub mod x86_guest_memory;
pub mod x86_virtualization;
pub mod x86_vmexit;

pub use aarch64_el2::*;
pub use armv7_thumb::*;
pub use dma::*;
pub use dynamic_memory::*;
pub use elf_loader::*;
pub use firmware::*;
pub use guest_system::*;
pub use hypervisor_backend::*;
pub use iommu::*;
pub use kernel_autonomous::*;
pub use kernel_core::*;
pub use kernel_debug::*;
pub use kernel_hardening::*;
pub use kernel_io::*;
pub use kernel_network::{certify_kernel_network_contracts, KernelNetworkReport};
pub use kernel_storage::*;
pub use kernel_stress::*;
pub use kernel_sync::*;
pub use machine_coordinator::*;
pub use manual_allocators::*;
pub use nc_abi::*;
pub use pci_virtio::*;
pub use physical_drivers::*;
pub use pointer_lowering::*;
pub use raw_memory::*;
pub use raw_memory_stress::*;
pub use riscv_h::*;
pub use smp::*;
pub use static_memory::*;
pub use uefi_loader::*;
pub use virtio_mmio::*;
pub use virtual_devices::*;
pub use virtual_memory::*;
pub use vm_lifecycle::*;
pub use vm_migration::*;
pub use x86_apic::*;
pub use x86_guest_boot::*;
pub use x86_guest_memory::*;
pub use x86_virtualization::*;
pub use x86_vmexit::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineObjectFormat {
    Elf,
    Coff,
    MachO,
    Wasm,
}
pub fn canonical_machine_diagnostic(message: &str) -> ncc_diagnostics::CanonicalDiagnostic {
    ncc_diagnostics::CanonicalDiagnostic::from_legacy(message, "machine", 0, 0)
        .with_domain(ncc_diagnostics::DiagnosticDomain::Machine)
}
