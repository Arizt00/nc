#[cfg(target_os = "macos")]
use serde_json::json;

const HVF_ABI_VERSION: u32 = 1;
#[cfg(target_os = "macos")]
const HVF_ARCH_X86_64: u32 = 1;
#[cfg(target_os = "macos")]
const HVF_ARCH_AARCH64: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
struct HvfSmokeResult {
    abi_version: u32,
    architecture: u32,
    status: i32,
    exit_count: u32,
    marker: u64,
    result_word: u64,
    final_pc: u64,
    exit_reason: u64,
    exit_detail: u64,
    provider_api_executed: u8,
    guest_code_executed: u8,
    final_halt: u8,
    reserved: [u8; 5],
    error: [u8; 256],
}

impl Default for HvfSmokeResult {
    fn default() -> Self {
        Self {
            abi_version: HVF_ABI_VERSION,
            architecture: 0,
            status: 0,
            exit_count: 0,
            marker: 0,
            result_word: 0,
            final_pc: 0,
            exit_reason: 0,
            exit_detail: 0,
            provider_api_executed: 0,
            guest_code_executed: 0,
            final_halt: 0,
            reserved: [0; 5],
            error: [0; 256],
        }
    }
}

#[cfg(target_os = "macos")]
fn record_error(result: &mut HvfSmokeResult, status: i32, operation: &str, value: i64) {
    result.status = status;
    let message = format!("{operation} failed ({value})");
    let bytes = message.as_bytes();
    let count = bytes.len().min(result.error.len() - 1);
    result.error[..count].copy_from_slice(&bytes[..count]);
    result.error[count] = 0;
}

#[cfg(target_os = "macos")]
fn error_text(result: &HvfSmokeResult) -> String {
    let end = result
        .error
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(result.error.len());
    String::from_utf8_lossy(&result.error[..end]).into_owned()
}

#[cfg(target_os = "macos")]
struct GuestMemory {
    address: *mut u8,
    size: usize,
}

#[cfg(target_os = "macos")]
impl GuestMemory {
    fn allocate(size: usize) -> Result<Self, i32> {
        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1));
        }
        Ok(Self {
            address: address.cast(),
            size,
        })
    }

    fn write(&mut self, bytes: &[u8]) {
        assert!(bytes.len() <= self.size);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.address, bytes.len()) };
    }

    #[cfg(target_arch = "x86_64")]
    fn read_u16(&self, offset: usize) -> u16 {
        assert!(offset + size_of::<u16>() <= self.size);
        unsafe { std::ptr::read_volatile(self.address.add(offset).cast::<u16>()) }
    }

    #[cfg(target_arch = "aarch64")]
    fn read_u64(&self, offset: usize) -> u64 {
        assert!(offset + size_of::<u64>() <= self.size);
        unsafe { std::ptr::read_volatile(self.address.add(offset).cast::<u64>()) }
    }
}

#[cfg(target_os = "macos")]
impl Drop for GuestMemory {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.address.cast(), self.size);
        }
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn run_arch(result: &mut HvfSmokeResult) {
    use applevisor_sys as hv;
    use std::ffi::c_void;

    unsafe extern "C" {
        fn sys_icache_invalidate(start: *mut c_void, length: usize);
    }

    const EXPECTED: u64 = 0x1234;
    result.architecture = HVF_ARCH_AARCH64;
    let mut memory = match GuestMemory::allocate(hv::PAGE_SIZE) {
        Ok(memory) => memory,
        Err(error) => {
            record_error(result, error, "mmap", error as i64);
            return;
        }
    };

    // movz x0,#0x1234; str x0,[x1]; hvc #0
    let guest = [0xd282_4680_u32, 0xf900_0020, 0xd400_0002];
    let mut guest_bytes = [0_u8; 12];
    for (index, instruction) in guest.into_iter().enumerate() {
        guest_bytes[index * 4..index * 4 + 4].copy_from_slice(&instruction.to_le_bytes());
    }
    memory.write(&guest_bytes);
    unsafe { sys_icache_invalidate(memory.address.cast(), guest_bytes.len()) };

    let mut vm_created = false;
    let mut memory_mapped = false;
    let mut vcpu_created = false;
    let mut vcpu = 0;
    let mut exit: *const hv::hv_vcpu_exit_t = std::ptr::null();

    let execution = (|| -> Result<(), (i32, &'static str)> {
        let mut status = unsafe { hv::hv_vm_create(std::ptr::null_mut()) };
        if status != 0 {
            return Err((status, "hv_vm_create"));
        }
        vm_created = true;
        result.provider_api_executed = 1;

        status = unsafe {
            hv::hv_vm_map(
                memory.address.cast(),
                0,
                memory.size,
                hv::HV_MEMORY_READ | hv::HV_MEMORY_WRITE | hv::HV_MEMORY_EXEC,
            )
        };
        if status != 0 {
            return Err((status, "hv_vm_map"));
        }
        memory_mapped = true;

        status = unsafe { hv::hv_vcpu_create(&mut vcpu, &mut exit, std::ptr::null_mut()) };
        if status != 0 {
            return Err((status, "hv_vcpu_create"));
        }
        vcpu_created = true;

        for (register, value) in [
            (hv::hv_reg_t::PC, 0),
            (hv::hv_reg_t::X1, 0x100),
            (hv::hv_reg_t::CPSR, 0x3c5),
        ] {
            status = unsafe { hv::hv_vcpu_set_reg(vcpu, register, value) };
            if status != 0 {
                return Err((status, "hv_vcpu_set_reg"));
            }
        }

        status = unsafe { hv::hv_vcpu_run(vcpu) };
        result.exit_count = 1;
        if status != 0 {
            return Err((status, "hv_vcpu_run"));
        }
        result.guest_code_executed = 1;

        status = unsafe { hv::hv_vcpu_get_reg(vcpu, hv::hv_reg_t::X0, &mut result.result_word) };
        if status == 0 {
            status = unsafe { hv::hv_vcpu_get_reg(vcpu, hv::hv_reg_t::PC, &mut result.final_pc) };
        }
        if status != 0 {
            return Err((status, "hv_vcpu_get_reg"));
        }
        result.marker = memory.read_u64(0x100);

        if exit.is_null() {
            return Err((-1, "guest exit evidence"));
        }
        let exit = unsafe { &*exit };
        result.exit_reason = exit.reason as u64;
        result.exit_detail = exit.exception.syndrome;
        let exception_class = (result.exit_detail >> 26) & 0x3f;
        if exit.reason != hv::hv_exit_reason_t::EXCEPTION
            || exception_class != 0x16
            || result.marker != EXPECTED
            || result.result_word != EXPECTED
        {
            return Err((-1, "guest evidence"));
        }

        result.final_halt = 1;
        Ok(())
    })();

    if vcpu_created {
        unsafe { hv::hv_vcpu_destroy(vcpu) };
    }
    if memory_mapped {
        unsafe { hv::hv_vm_unmap(0, memory.size) };
    }
    if vm_created {
        unsafe { hv::hv_vm_destroy() };
    }

    match execution {
        Ok(()) => result.status = 0,
        Err((status, operation)) => record_error(result, status, operation, status as i64),
    }
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn run_arch(result: &mut HvfSmokeResult) {
    use std::ffi::c_void;
    use xhypervisor::x86_64::consts::vmcs::*;
    use xhypervisor::x86_64::consts::vmx_cap::CPU_BASED_HLT;
    use xhypervisor::x86_64::consts::vmx_exit::VMX_REASON_HLT;
    use xhypervisor::x86_64::ffi as hv;
    use xhypervisor::x86_64::Register;

    unsafe fn write_vmcs(vcpu: hv::hv_vcpuid_t, field: u32, value: u64) -> hv::hv_return_t {
        unsafe { hv::hv_vmx_vcpu_write_vmcs(vcpu, field, value) }
    }

    unsafe fn write_segment(
        vcpu: hv::hv_vcpuid_t,
        selector: u32,
        base: u32,
        limit: u32,
        access: u32,
        access_value: u64,
    ) -> hv::hv_return_t {
        let mut status = unsafe { write_vmcs(vcpu, selector, 0) };
        if status == hv::HV_SUCCESS {
            status = unsafe { write_vmcs(vcpu, base, 0) };
        }
        if status == hv::HV_SUCCESS {
            status = unsafe { write_vmcs(vcpu, limit, 0xffff) };
        }
        if status == hv::HV_SUCCESS {
            status = unsafe { write_vmcs(vcpu, access, access_value) };
        }
        status
    }

    const EXPECTED: u64 = 0x1234;
    result.architecture = HVF_ARCH_X86_64;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        record_error(result, -1, "sysconf(_SC_PAGESIZE)", page_size);
        return;
    }
    let mut memory = match GuestMemory::allocate(page_size as usize) {
        Ok(memory) => memory,
        Err(error) => {
            record_error(result, error, "mmap", error as i64);
            return;
        }
    };
    // mov ax,0x1234; mov [0x100],ax; hlt (16-bit unrestricted guest)
    memory.write(&[0xb8, 0x34, 0x12, 0xa3, 0x00, 0x01, 0xf4]);

    let mut vm_created = false;
    let mut memory_mapped = false;
    let mut vcpu_created = false;
    let mut vcpu = 0;

    let execution = (|| -> Result<(), (i32, &'static str)> {
        let mut status = unsafe { hv::hv_vm_create(hv::HV_VM_DEFAULT) };
        if status != hv::HV_SUCCESS {
            return Err((status as i32, "hv_vm_create"));
        }
        vm_created = true;
        result.provider_api_executed = 1;

        status = unsafe {
            hv::hv_vm_map(
                memory.address.cast::<c_void>(),
                0,
                memory.size,
                hv::HV_MEMORY_READ | hv::HV_MEMORY_WRITE | hv::HV_MEMORY_EXEC,
            )
        };
        if status != hv::HV_SUCCESS {
            return Err((status as i32, "hv_vm_map"));
        }
        memory_mapped = true;

        status = unsafe { hv::hv_vcpu_create(&mut vcpu, hv::HV_VCPU_DEFAULT) };
        if status != hv::HV_SUCCESS {
            return Err((status as i32, "hv_vcpu_create"));
        }
        vcpu_created = true;

        for (selector, base, limit, access, value) in [
            (
                VMCS_GUEST_CS,
                VMCS_GUEST_CS_BASE,
                VMCS_GUEST_CS_LIMIT,
                VMCS_GUEST_CS_AR,
                0x9b,
            ),
            (
                VMCS_GUEST_SS,
                VMCS_GUEST_SS_BASE,
                VMCS_GUEST_SS_LIMIT,
                VMCS_GUEST_SS_AR,
                0x93,
            ),
            (
                VMCS_GUEST_DS,
                VMCS_GUEST_DS_BASE,
                VMCS_GUEST_DS_LIMIT,
                VMCS_GUEST_DS_AR,
                0x93,
            ),
            (
                VMCS_GUEST_ES,
                VMCS_GUEST_ES_BASE,
                VMCS_GUEST_ES_LIMIT,
                VMCS_GUEST_ES_AR,
                0x93,
            ),
            (
                VMCS_GUEST_FS,
                VMCS_GUEST_FS_BASE,
                VMCS_GUEST_FS_LIMIT,
                VMCS_GUEST_FS_AR,
                0x93,
            ),
            (
                VMCS_GUEST_GS,
                VMCS_GUEST_GS_BASE,
                VMCS_GUEST_GS_LIMIT,
                VMCS_GUEST_GS_AR,
                0x93,
            ),
            (
                VMCS_GUEST_TR,
                VMCS_GUEST_TR_BASE,
                VMCS_GUEST_TR_LIMIT,
                VMCS_GUEST_TR_AR,
                0x8b,
            ),
            (
                VMCS_GUEST_LDTR,
                VMCS_GUEST_LDTR_BASE,
                VMCS_GUEST_LDTR_LIMIT,
                VMCS_GUEST_LDTR_AR,
                0x10000,
            ),
        ] {
            status = unsafe { write_segment(vcpu, selector, base, limit, access, value) };
            if status != hv::HV_SUCCESS {
                return Err((status as i32, "VMCS segment setup"));
            }
        }

        for (field, value) in [
            (VMCS_GUEST_GDTR_BASE, 0),
            (VMCS_GUEST_GDTR_LIMIT, 0xffff),
            (VMCS_GUEST_IDTR_BASE, 0),
            (VMCS_GUEST_IDTR_LIMIT, 0xffff),
            (VMCS_GUEST_CR0, 0x20),
            (VMCS_GUEST_CR3, 0),
            (VMCS_GUEST_CR4, 0),
            (VMCS_GUEST_DR7, 0x400),
            (VMCS_GUEST_IA32_EFER, 0),
            (VMCS_GUEST_RIP, 0),
            (VMCS_GUEST_RSP, memory.size as u64 - 16),
            (VMCS_GUEST_RFLAGS, 2),
            (VMCS_GUEST_ACTIVITY_STATE, 0),
            (VMCS_GUEST_IGNORE_IRQ, 0),
            (VMCS_GUEST_LINK_POINTER, u64::MAX),
        ] {
            status = unsafe { write_vmcs(vcpu, field, value) };
            if status != hv::HV_SUCCESS {
                return Err((status as i32, "VMCS guest setup"));
            }
        }

        let mut controls = 0;
        status = unsafe { hv::hv_vmx_vcpu_read_vmcs(vcpu, VMCS_CTRL_CPU_BASED, &mut controls) };
        if status == hv::HV_SUCCESS {
            status = unsafe { write_vmcs(vcpu, VMCS_CTRL_CPU_BASED, controls | CPU_BASED_HLT) };
        }
        if status != hv::HV_SUCCESS {
            return Err((status as i32, "VMCS control setup"));
        }

        status = unsafe { hv::hv_vcpu_run(vcpu) };
        result.exit_count = 1;
        if status != hv::HV_SUCCESS {
            return Err((status as i32, "hv_vcpu_run"));
        }
        result.guest_code_executed = 1;

        status = unsafe {
            hv::hv_vmx_vcpu_read_vmcs(vcpu, VMCS_RO_EXIT_REASON, &mut result.exit_reason)
        };
        if status == hv::HV_SUCCESS {
            status =
                unsafe { hv::hv_vmx_vcpu_read_vmcs(vcpu, VMCS_GUEST_RIP, &mut result.final_pc) };
        }
        if status == hv::HV_SUCCESS {
            status =
                unsafe { hv::hv_vcpu_read_register(vcpu, Register::RAX, &mut result.result_word) };
        }
        if status != hv::HV_SUCCESS {
            return Err((status as i32, "read guest evidence"));
        }

        result.exit_reason &= 0xffff;
        result.marker = memory.read_u16(0x100) as u64;
        if result.exit_reason != VMX_REASON_HLT
            || result.marker != EXPECTED
            || result.result_word & 0xffff != EXPECTED
        {
            return Err((-1, "guest evidence"));
        }
        result.result_word &= 0xffff;
        result.final_halt = 1;
        Ok(())
    })();

    if vcpu_created {
        unsafe { hv::hv_vcpu_destroy(vcpu) };
    }
    if memory_mapped {
        unsafe { hv::hv_vm_unmap(0, memory.size) };
    }
    if vm_created {
        unsafe { hv::hv_vm_destroy() };
    }

    match execution {
        Ok(()) => result.status = 0,
        Err((status, operation)) => record_error(result, status, operation, status as i64),
    }
}

#[cfg(target_os = "macos")]
fn run() -> Result<(), String> {
    let mut result = HvfSmokeResult::default();
    run_arch(&mut result);
    if result.status != 0 {
        return Err(format!(
            "[E_MACHINE_HVF_NATIVE_001] {}",
            error_text(&result)
        ));
    }
    if result.abi_version != HVF_ABI_VERSION {
        return Err(format!(
            "[E_MACHINE_HVF_ABI_001] helper ABI {} is not supported",
            result.abi_version
        ));
    }
    let architecture = match result.architecture {
        HVF_ARCH_X86_64 => "x86_64",
        HVF_ARCH_AARCH64 => "aarch64",
        other => {
            return Err(format!(
                "[E_MACHINE_HVF_ARCH_001] unknown architecture {other}"
            ))
        }
    };
    let payload = json!({
        "provider_api_executed": result.provider_api_executed != 0,
        "guest_code_executed": result.guest_code_executed != 0,
        "final_halt": result.final_halt != 0,
        "architecture": architecture,
        "marker_sequence": "HVF",
        "memory_marker": result.marker,
        "result_word": result.result_word,
        "exit_count": result.exit_count,
        "final_pc": result.final_pc,
        "exit_reason": result.exit_reason,
        "exit_detail": result.exit_detail,
        "framework": "/System/Library/Frameworks/Hypervisor.framework",
        "watchdog_fired": false,
    });
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run() -> Result<(), String> {
    let _ = HvfSmokeResult::default();
    Err("[E_MACHINE_HVF_HOST_001] ncc-hvf-smoke requires macOS".to_string())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
