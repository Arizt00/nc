#[derive(Debug, Clone, Copy)]
pub(crate) struct KernelCoreSignature {
    pub symbol: &'static str,
    pub params: &'static [&'static str],
    pub return_type: &'static str,
}

pub(crate) fn kernel_core_signature(target: &str) -> Option<KernelCoreSignature> {
    let name = target.trim().to_ascii_lowercase();
    let sig = match name.as_str() {
        "kernel.task.core.register" | "taskregister" => {
            ("__ncc_kernel_task_register", &["i64", "ptr"][..], "i1")
        }
        "kernel.task.core.spawn" | "taskspawn" => {
            ("__ncc_kernel_task_spawn", &["i64", "i64", "i64"][..], "i1")
        }
        "kernel.task.core.block" | "taskblock" => ("__ncc_kernel_task_block", &["i64"][..], "i1"),
        "kernel.task.core.wake" | "taskwake" => ("__ncc_kernel_task_wake", &["i64"][..], "i1"),
        "kernel.task.core.exit" | "taskexit" => {
            ("__ncc_kernel_task_exit", &["i64", "i64"][..], "i1")
        }
        "kernel.task.core.state" | "taskstate" => ("__ncc_kernel_task_state", &["i64"][..], "i64"),
        "kernel.task.core.current" | "taskcurrent" => ("__ncc_kernel_task_current", &[][..], "i64"),
        "kernel.schedule.core.open" | "scheduleropen" => {
            ("__ncc_kernel_schedule_open", &[][..], "i1")
        }
        "kernel.schedule.core.close" | "schedulerclose" => {
            ("__ncc_kernel_schedule_close", &[][..], "i1")
        }
        "kernel.schedule.core.next" | "schedulernext" => {
            ("__ncc_kernel_schedule_next", &[][..], "i1")
        }
        "kernel.schedule.core.tick" | "schedulertick" => {
            ("__ncc_kernel_schedule_tick", &[][..], "i64")
        }
        "kernel.schedule.core.yieldnow" | "scheduleryield" => {
            ("__ncc_kernel_schedule_yield", &[][..], "i64")
        }
        "kernel.schedule.core.bind" | "schedulerbind" => {
            ("__ncc_kernel_schedule_bind", &["i64", "i64"][..], "i1")
        }
        "kernel.schedule.core.nexton" | "schedulernexton" => {
            ("__ncc_kernel_schedule_next_on", &["i64"][..], "i1")
        }
        "kernel.schedule.core.currenton" | "schedulercurrenton" => {
            ("__ncc_kernel_schedule_current_on", &["i64"][..], "i64")
        }
        "kernel.cpu.core.register" | "cpuregister" => {
            ("__ncc_kernel_cpu_register", &["i64", "i1", "i64"][..], "i1")
        }
        "kernel.cpu.core.start" | "cpustart" => ("__ncc_kernel_cpu_start", &["i64"][..], "i1"),
        "kernel.cpu.core.online" | "cpuonline" => ("__ncc_kernel_cpu_online", &["i64"][..], "i1"),
        "kernel.cpu.core.current" | "cpucurrent" => ("__ncc_kernel_cpu_current", &[][..], "i64"),
        "kernel.cpu.core.count" | "cpucount" => ("__ncc_kernel_cpu_count", &[][..], "i64"),
        "kernel.cpu.core.onlinecount" | "cpuonlinecount" => {
            ("__ncc_kernel_cpu_online_count", &[][..], "i64")
        }
        "kernel.cpu.core.setcurrent" | "cpusetcurrent" => {
            ("__ncc_kernel_cpu_set_current", &["i64"][..], "i1")
        }
        "kernel.cpu.core.localset" | "cpulocalset" => (
            "__ncc_kernel_cpu_local_set",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.cpu.core.localget" | "cpulocalget" => {
            ("__ncc_kernel_cpu_local_get", &["i64", "i64"][..], "i64")
        }
        "kernel.cpu.core.stack" | "cpustack" => ("__ncc_kernel_cpu_stack_get", &["i64"][..], "i64"),
        "kernel.ipi.core.send" | "ipisend" => ("__ncc_kernel_ipi_send", &["i64", "i64"][..], "i1"),
        "kernel.ipi.core.broadcast" | "ipibroadcast" => {
            ("__ncc_kernel_ipi_broadcast", &["i64"][..], "i64")
        }
        "kernel.ipi.core.pending" | "ipipending" => {
            ("__ncc_kernel_ipi_pending", &["i64"][..], "i64")
        }
        "kernel.ipi.core.peek" | "ipipeek" => ("__ncc_kernel_ipi_peek", &["i64"][..], "i64"),
        "kernel.ipi.core.ack" | "ipiack" => ("__ncc_kernel_ipi_ack", &["i64", "i64"][..], "i1"),
        "kernel.control.core.quarantine" | "kernelquarantine" => {
            ("__ncc_kernel_quarantine", &["i64"][..], "i1")
        }
        "kernel.control.core.panic" | "kernelpanic" => ("__ncc_kernel_panic", &["ptr"][..], "i1"),
        "kernel.control.core.halt" | "kernelhalt" => ("__ncc_kernel_halt", &[][..], "i1"),
        "kernel.control.core.panicstate" | "kernelpanicstate" => {
            ("__ncc_kernel_panic_state", &[][..], "i1")
        }
        "kernel.memory.core.frameinit" | "frameinit" => {
            ("__ncc_kernel_frame_init", &["i64", "i64"][..], "i1")
        }
        "kernel.memory.core.framealloc" | "framealloc" => {
            ("__ncc_kernel_frame_alloc", &["i64"][..], "i64")
        }
        "kernel.memory.core.framefree" | "framefree" => {
            ("__ncc_kernel_frame_free", &["i64", "i64"][..], "i1")
        }
        "kernel.memory.core.pagemap" | "pagemap" => {
            ("__ncc_kernel_page_map", &["i64", "i64", "i64"][..], "i1")
        }
        "kernel.memory.core.pageunmap" | "pageunmap" => {
            ("__ncc_kernel_page_unmap", &["i64"][..], "i1")
        }
        "kernel.memory.core.pageprotect" | "pageprotect" => {
            ("__ncc_kernel_page_protect", &["i64", "i64"][..], "i1")
        }
        "kernel.memory.core.pagetranslate" | "pagetranslate" => {
            ("__ncc_kernel_page_translate", &["i64"][..], "i64")
        }
        "kernel.interrupt.core.register" | "irqregister" => {
            ("__ncc_kernel_irq_register", &["i64", "i64"][..], "i1")
        }
        "kernel.interrupt.core.enable" | "irqenable" => {
            ("__ncc_kernel_irq_enable", &["i64"][..], "i1")
        }
        "kernel.interrupt.core.disable" | "irqdisable" => {
            ("__ncc_kernel_irq_disable", &["i64"][..], "i1")
        }
        "kernel.interrupt.core.isenabled" | "irqisenabled" => {
            ("__ncc_kernel_irq_is_enabled", &["i64"][..], "i1")
        }
        "kernel.interrupt.core.dispatch" | "irqdispatch" => {
            ("__ncc_kernel_irq_dispatch", &["i64", "i64"][..], "i1")
        }
        "kernel.interrupt.core.eoi" | "irqeoi" => ("__ncc_kernel_irq_eoi", &["i64"][..], "i1"),
        "kernel.syscall.core.register" | "syscallregister" => {
            ("__ncc_kernel_syscall_register", &["i64", "i64"][..], "i1")
        }
        "kernel.syscall.core.available" | "syscallavailable" => {
            ("__ncc_kernel_syscall_available", &["i64"][..], "i1")
        }
        "kernel.syscall.core.invoke" | "syscallinvoke" => (
            "__ncc_kernel_syscall_invoke",
            &["i64", "i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.driver.core.register" | "driverregister" => (
            "__ncc_kernel_driver_register",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.driver.core.start" | "driverstart" => {
            ("__ncc_kernel_driver_start", &["i64"][..], "i1")
        }
        "kernel.driver.core.stop" | "driverstop" => {
            ("__ncc_kernel_driver_stop", &["i64"][..], "i1")
        }
        "kernel.driver.core.read" | "driverread" => {
            ("__ncc_kernel_driver_read", &["i64", "i64"][..], "i64")
        }
        "kernel.driver.core.write" | "driverwrite" => (
            "__ncc_kernel_driver_write",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.driver.core.status" | "driverstatus" => {
            ("__ncc_kernel_driver_status", &["i64"][..], "i64")
        }
        "kernel.user.core.copyfromuser" | "copyfromuser" => (
            "__ncc_kernel_copy_from_user",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.user.core.copytouser" | "copytouser" => (
            "__ncc_kernel_copy_to_user",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.boot.core.info" | "kernelbootinfo" => ("__ncc_kernel_boot_info", &[][..], "i64"),
        "kernel.boot.physical.valid" | "bootvalid" | "boot_valid" => {
            ("__ncc_kernel_boot_valid", &[][..], "i1")
        }
        "kernel.boot.physical.flags" | "bootflags" | "boot_flags" => {
            ("__ncc_kernel_boot_flags", &[][..], "i64")
        }
        "kernel.boot.physical.memorycount" | "bootmemorycount" | "boot_memory_count" => {
            ("__ncc_kernel_boot_memory_count", &[][..], "i64")
        }
        "kernel.boot.physical.memorybase" | "bootmemorybase" | "boot_memory_base" => {
            ("__ncc_kernel_boot_memory_base", &["i64"][..], "i64")
        }
        "kernel.boot.physical.memorysize" | "bootmemorysize" | "boot_memory_size" => {
            ("__ncc_kernel_boot_memory_size", &["i64"][..], "i64")
        }
        "kernel.boot.physical.memorykind" | "bootmemorykind" | "boot_memory_kind" => {
            ("__ncc_kernel_boot_memory_kind", &["i64"][..], "i64")
        }
        "kernel.boot.physical.framebuffer" | "bootframebuffer" | "boot_framebuffer" => {
            ("__ncc_kernel_boot_framebuffer", &[][..], "i64")
        }
        "kernel.boot.physical.framebufferbase"
        | "bootframebufferbase"
        | "boot_framebuffer_base" => ("__ncc_kernel_boot_framebuffer_base", &[][..], "i64"),
        "kernel.boot.physical.framebufferwidth"
        | "bootframebufferwidth"
        | "boot_framebuffer_width" => ("__ncc_kernel_boot_framebuffer_width", &[][..], "i64"),
        "kernel.boot.physical.framebufferheight"
        | "bootframebufferheight"
        | "boot_framebuffer_height" => ("__ncc_kernel_boot_framebuffer_height", &[][..], "i64"),
        "kernel.boot.physical.framebufferpitch"
        | "bootframebufferpitch"
        | "boot_framebuffer_pitch" => ("__ncc_kernel_boot_framebuffer_pitch", &[][..], "i64"),
        "kernel.boot.physical.framebufferformat"
        | "bootframebufferformat"
        | "boot_framebuffer_format" => ("__ncc_kernel_boot_framebuffer_format", &[][..], "i64"),
        "kernel.boot.physical.framebufferflags"
        | "bootframebufferflags"
        | "boot_framebuffer_flags" => ("__ncc_kernel_boot_framebuffer_flags", &[][..], "i64"),
        "kernel.boot.physical.platform" | "bootplatform" | "boot_platform" => {
            ("__ncc_kernel_boot_platform", &["i64"][..], "i64")
        }
        "kernel.boot.physical.platformsize" | "bootplatformsize" | "boot_platform_size" => {
            ("__ncc_kernel_boot_platform_size", &["i64"][..], "i64")
        }
        "kernel.boot.physical.commandline" | "bootcommandline" | "boot_command_line" => {
            ("__ncc_kernel_boot_command_line", &[][..], "i64")
        }
        "kernel.smp.physical.discover" | "smpdiscover" | "smp_discover" => {
            ("__ncc_kernel_smp_discover", &[][..], "i1")
        }
        "kernel.smp.physical.count" | "cpucountphysical" | "cpu_count_physical" => {
            ("__ncc_kernel_cpu_count_physical", &[][..], "i64")
        }
        "kernel.smp.physical.configure" | "smpconfigure" | "smp_configure" => (
            "__ncc_kernel_smp_configure",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.smp.physical.configurefirmware"
        | "smpconfigurefirmware"
        | "smp_configure_firmware" => ("__ncc_kernel_smp_configure_firmware", &[][..], "i1"),
        "kernel.smp.physical.current" | "cpucurrentphysical" | "cpu_current_physical" => {
            ("__ncc_kernel_cpu_current_physical", &[][..], "i64")
        }
        "kernel.smp.physical.hardwareid" | "cpuhardwareid" | "cpu_hardware_id" => {
            ("__ncc_kernel_cpu_hardware_id", &["i64"][..], "i64")
        }
        "kernel.smp.physical.firmwareid" | "cpufirmwareid" | "cpu_firmware_id" => {
            ("__ncc_kernel_cpu_firmware_id", &["i64"][..], "i64")
        }
        "kernel.smp.physical.numanode" | "cpunumanode" | "cpu_numa_node" => {
            ("__ncc_kernel_cpu_numa_node", &["i64"][..], "i64")
        }
        "kernel.smp.physical.packageid" | "cpupackageid" | "cpu_package_id" => {
            ("__ncc_kernel_cpu_package_id", &["i64"][..], "i64")
        }
        "kernel.smp.physical.coreid" | "cpucoreid" | "cpu_core_id" => {
            ("__ncc_kernel_cpu_core_id", &["i64"][..], "i64")
        }
        "kernel.smp.physical.threadid" | "cputhreadid" | "cpu_thread_id" => {
            ("__ncc_kernel_cpu_thread_id", &["i64"][..], "i64")
        }
        "kernel.smp.physical.state" | "cpustatephysical" | "cpu_state_physical" => {
            ("__ncc_kernel_cpu_state_physical", &["i64"][..], "i64")
        }
        "kernel.smp.physical.stack" | "cpustackphysical" | "cpu_stack_physical" => {
            ("__ncc_kernel_cpu_stack_physical", &["i64"][..], "i64")
        }
        "kernel.smp.physical.entry" | "cpuentryphysical" | "cpu_entry_physical" => {
            ("__ncc_kernel_cpu_entry_physical", &["i64"][..], "i64")
        }
        "kernel.smp.physical.context" | "cpucontextphysical" | "cpu_context_physical" => {
            ("__ncc_kernel_cpu_context_physical", &["i64"][..], "i64")
        }
        "kernel.smp.physical.setstack" | "cpusetstackphysical" | "cpu_set_stack_physical" => (
            "__ncc_kernel_cpu_set_stack_physical",
            &["i64", "i64"][..],
            "i1",
        ),
        "kernel.smp.physical.start" | "cpustartphysical" | "cpu_start_physical" => (
            "__ncc_kernel_cpu_start_physical",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.smp.physical.startwait" | "cpustartphysicalwait" | "cpu_start_physical_wait" => (
            "__ncc_kernel_cpu_start_physical_wait",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.smp.physical.waitonline" | "cpuwaitonline" | "cpu_wait_online" => {
            ("__ncc_kernel_cpu_wait_online", &["i64", "i64"][..], "i1")
        }
        "kernel.smp.physical.online" | "cpumarkonline" | "cpu_mark_online" => {
            ("__ncc_kernel_cpu_mark_online", &["i64"][..], "i1")
        }
        "kernel.smp.physical.ipi" | "ipisendphysical" | "ipi_send_physical" => {
            ("__ncc_kernel_ipi_send_physical", &["i64", "i64"][..], "i1")
        }
        "kernel.smp.physical.ipitake" | "ipitakephysical" | "ipi_take_physical" => {
            ("__ncc_kernel_ipi_take_physical", &["i64"][..], "i64")
        }
        "kernel.smp.physical.timer" | "timerarmphysical" | "timer_arm_physical" => {
            ("__ncc_kernel_timer_arm_physical", &["i64", "i64"][..], "i1")
        }
        "kernel.smp.physical.tlbinvalidate" | "tlbinvalidate" | "tlb_invalidate" => (
            "__ncc_kernel_tlb_invalidate",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.smp.physical.tlbshootdown" | "tlbshootdown" | "tlb_shootdown" => (
            "__ncc_kernel_tlb_shootdown",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.smp.physical.tlbhandle" | "tlbshootdownhandle" | "tlb_shootdown_handle" => {
            ("__ncc_kernel_tlb_shootdown_handle", &["i64"][..], "i1")
        }
        "kernel.arch.x86.tablesload" | "x86tablesload" | "x86_tables_load" => (
            "__ncc_kernel_x86_tables_load",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.arch.arm.gicinit" | "gicinit" | "gic_init" => {
            ("__ncc_kernel_gic_init", &["i64", "i64"][..], "i1")
        }
        "kernel.arch.arm.gicinitfirmware" | "gicinitfirmware" | "gic_init_firmware" => {
            ("__ncc_kernel_gic_init_firmware", &[][..], "i1")
        }
        "kernel.smp.physical.interruptinitfirmware"
        | "smpinterruptinitfirmware"
        | "smp_interrupt_init_firmware" => (
            "__ncc_kernel_smp_interrupt_init_firmware",
            &["i64"][..],
            "i1",
        ),
        "kernel.arch.arm.gicenable" | "gicenable" | "gic_enable" => {
            ("__ncc_kernel_gic_enable", &["i64", "i64", "i64"][..], "i1")
        }
        "kernel.arch.arm.gicack" | "gicack" | "gic_ack" => ("__ncc_kernel_gic_ack", &[][..], "i64"),
        "kernel.arch.arm.giceoi" | "giceoi" | "gic_eoi" => {
            ("__ncc_kernel_gic_eoi", &["i64"][..], "i1")
        }
        "kernel.arch.riscv.plicinit" | "plicinit" | "plic_init" => {
            ("__ncc_kernel_plic_init", &["i64", "i64"][..], "i1")
        }
        "kernel.arch.riscv.plicenable" | "plicenable" | "plic_enable" => {
            ("__ncc_kernel_plic_enable", &["i64", "i64", "i64"][..], "i1")
        }
        "kernel.arch.riscv.plicclaim" | "plicclaim" | "plic_claim" => {
            ("__ncc_kernel_plic_claim", &["i64"][..], "i64")
        }
        "kernel.arch.riscv.pliccomplete" | "pliccomplete" | "plic_complete" => {
            ("__ncc_kernel_plic_complete", &["i64", "i64"][..], "i1")
        }
        "kernel.arch.riscv.imsicinit" | "imsicinit" | "imsic_init" => {
            ("__ncc_kernel_imsic_init", &["i64"][..], "i1")
        }
        "kernel.arch.riscv.imsicenable" | "imsicenable" | "imsic_enable" => {
            ("__ncc_kernel_imsic_enable", &["i64"][..], "i1")
        }
        "kernel.arch.riscv.imsicclaim" | "imsicclaim" | "imsic_claim" => {
            ("__ncc_kernel_imsic_claim", &[][..], "i64")
        }
        "kernel.arch.riscv.imsiccomplete" | "imsiccomplete" | "imsic_complete" => {
            ("__ncc_kernel_imsic_complete", &["i64"][..], "i1")
        }
        "kernel.arch.core.capability" | "archcapability" | "arch_capability" => {
            ("__ncc_kernel_arch_capability", &["i64"][..], "i1")
        }
        "kernel.arch.core.relax" | "cpurelax" | "cpu_relax" => {
            ("__ncc_kernel_arch_cpu_relax", &[][..], "i1")
        }
        "kernel.arch.core.memorybarrier" | "memorybarrier" | "memory_barrier" => {
            ("__ncc_kernel_arch_memory_barrier", &[][..], "i1")
        }
        "kernel.arch.core.instructionbarrier" | "instructionbarrier" | "instruction_barrier" => {
            ("__ncc_kernel_arch_instruction_barrier", &[][..], "i1")
        }
        "kernel.arch.core.interruptenable" | "interruptenable" | "interrupt_enable" => {
            ("__ncc_kernel_arch_interrupt_enable", &[][..], "i1")
        }
        "kernel.arch.core.interruptdisable" | "interruptdisable" | "interrupt_disable" => {
            ("__ncc_kernel_arch_interrupt_disable", &[][..], "i1")
        }
        "kernel.arch.core.halt" | "cpuhalt" | "cpu_halt" => {
            ("__ncc_kernel_arch_halt", &[][..], "i1")
        }
        "kernel.arch.core.cyclecounter" | "cyclecounter" | "cycle_counter" => {
            ("__ncc_kernel_arch_cycle_counter", &[][..], "i64")
        }
        "kernel.virtualization.intel.vmxon" | "vmxon" | "vmx_on" => {
            ("__ncc_kernel_vmx_on", &["i64"][..], "i1")
        }
        "kernel.virtualization.intel.vmxoff" | "vmxoff" | "vmx_off" => {
            ("__ncc_kernel_vmx_off", &[][..], "i1")
        }
        "kernel.virtualization.intel.vmcsclear" | "vmcsclear" | "vmcs_clear" => {
            ("__ncc_kernel_vmcs_clear", &["i64"][..], "i1")
        }
        "kernel.virtualization.intel.vmcsload" | "vmcsload" | "vmcs_load" => {
            ("__ncc_kernel_vmcs_load", &["i64"][..], "i1")
        }
        "kernel.virtualization.intel.vmcsstore" | "vmcsstore" | "vmcs_store" => {
            ("__ncc_kernel_vmcs_store", &["i64"][..], "i1")
        }
        "kernel.virtualization.intel.vmcsread" | "vmcsread" | "vmcs_read" => {
            ("__ncc_kernel_vmcs_read", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.intel.vmcswrite" | "vmcswrite" | "vmcs_write" => {
            ("__ncc_kernel_vmcs_write", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.intel.invept" | "vminvalidateept" | "vm_invalidate_ept" => {
            ("__ncc_kernel_invept", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.intel.invvpid" | "vminvalidatevpid" | "vm_invalidate_vpid" => {
            ("__ncc_kernel_invvpid", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.intel.vmcall" | "vmcall" | "vm_call" => {
            ("__ncc_kernel_vm_call", &[][..], "i1")
        }
        "kernel.virtualization.intel.vmlaunch" | "vmlaunch" | "vm_launch" => {
            ("__ncc_kernel_vm_launch", &[][..], "i1")
        }
        "kernel.virtualization.intel.vmresume" | "vmresume" | "vm_resume" => {
            ("__ncc_kernel_vm_resume", &[][..], "i1")
        }
        "kernel.virtualization.amd.vmrun" | "svmrun" | "svm_run" => {
            ("__ncc_kernel_svm_run", &["i64"][..], "i1")
        }
        "kernel.virtualization.amd.vmmcall" | "vmmcall" | "svm_call" => {
            ("__ncc_kernel_svm_call", &[][..], "i1")
        }
        "kernel.virtualization.amd.vmload" | "svmload" | "svm_load" => {
            ("__ncc_kernel_svm_load", &["i64"][..], "i1")
        }
        "kernel.virtualization.amd.vmsave" | "svmsave" | "svm_save" => {
            ("__ncc_kernel_svm_save", &["i64"][..], "i1")
        }
        "kernel.virtualization.arm.el2readhcr" | "el2readhcr" | "el2_read_hcr" => {
            ("__ncc_kernel_el2_read_hcr", &[][..], "i64")
        }
        "kernel.virtualization.arm.el2writehcr" | "el2writehcr" | "el2_write_hcr" => {
            ("__ncc_kernel_el2_write_hcr", &["i64"][..], "i1")
        }
        "kernel.virtualization.arm.el2writevtcr" | "el2writevtcr" | "el2_write_vtcr" => {
            ("__ncc_kernel_el2_write_vtcr", &["i64"][..], "i1")
        }
        "kernel.virtualization.arm.el2writevttbr" | "el2writevttbr" | "el2_write_vttbr" => {
            ("__ncc_kernel_el2_write_vttbr", &["i64"][..], "i1")
        }
        "kernel.virtualization.arm.el2tlbiall" | "el2tlbiall" | "el2_tlbi_all" => {
            ("__ncc_kernel_el2_tlbi_all", &[][..], "i1")
        }
        "kernel.virtualization.arm.el2hvc" | "el2hvc" | "el2_hvc" => {
            ("__ncc_kernel_el2_hvc", &["i64"][..], "i64")
        }
        "kernel.virtualization.arm.el2exitsnapshot" | "el2exitsnapshot" | "el2_exit_snapshot" => {
            ("__ncc_kernel_el2_exit_snapshot", &["i64"][..], "i1")
        }
        "kernel.virtualization.riscv.hreadstatus" | "riscvhreadstatus" | "riscv_h_read_hstatus" => {
            ("__ncc_kernel_riscv_h_read_hstatus", &[][..], "i64")
        }
        "kernel.virtualization.riscv.hwritestatus"
        | "riscvhwritestatus"
        | "riscv_h_write_hstatus" => ("__ncc_kernel_riscv_h_write_hstatus", &["i64"][..], "i1"),
        "kernel.virtualization.riscv.hwritehgatp" | "riscvhwritehgatp" | "riscv_h_write_hgatp" => {
            ("__ncc_kernel_riscv_h_write_hgatp", &["i64"][..], "i1")
        }
        "kernel.virtualization.riscv.hfencegvma" | "riscvhfencegvma" | "riscv_h_hfence_gvma" => (
            "__ncc_kernel_riscv_h_hfence_gvma",
            &["i64", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.riscv.hfencevvma" | "riscvhfencevvma" | "riscv_h_hfence_vvma" => (
            "__ncc_kernel_riscv_h_hfence_vvma",
            &["i64", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.riscv.hlv" | "riscvhlv" | "riscv_h_hlv" => {
            ("__ncc_kernel_riscv_h_hlv", &["i64"][..], "i64")
        }
        "kernel.virtualization.riscv.hsv" | "riscvhsv" | "riscv_h_hsv" => {
            ("__ncc_kernel_riscv_h_hsv", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.riscv.hexitsnapshot"
        | "riscvhexitsnapshot"
        | "riscv_h_exit_snapshot" => ("__ncc_kernel_riscv_h_exit_snapshot", &["i64"][..], "i1"),
        "kernel.iommu.core.mapentry" | "iommumapentry" | "iommu_map_entry" => (
            "__ncc_kernel_iommu_map_entry",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.iommu.core.unmapentry" | "iommuunmapentry" | "iommu_unmap_entry" => {
            ("__ncc_kernel_iommu_unmap_entry", &["i64", "i64"][..], "i1")
        }
        "kernel.iommu.core.invalidate" | "iommuinvalidate" | "iommu_invalidate" => {
            ("__ncc_kernel_iommu_invalidate", &["i64", "i64"][..], "i1")
        }
        "kernel.dma.core.descriptorwrite" | "dmadescriptorwrite" | "dma_descriptor_write" => (
            "__ncc_kernel_dma_descriptor_write",
            &["i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.pinpages" | "dmapinpages" | "dma_pin_pages" => (
            "__ncc_kernel_dma_pin_pages",
            &["i64", "i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.unpinpages" | "dmaunpinpages" | "dma_unpin_pages" => (
            "__ncc_kernel_dma_unpin_pages",
            &["i64", "i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.sgwrite" | "dmasgwrite" | "dma_sg_write" => (
            "__ncc_kernel_dma_sg_write",
            &["i64", "i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.bouncetodevice" | "dmabouncetodevice" | "dma_bounce_to_device" => (
            "__ncc_kernel_dma_bounce_to_device",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.bouncefromdevice" | "dmabouncefromdevice" | "dma_bounce_from_device" => (
            "__ncc_kernel_dma_bounce_from_device",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.cacheclean" | "dmacacheclean" | "dma_cache_clean" => (
            "__ncc_kernel_dma_cache_clean",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.cacheinvalidate" | "dmacacheinvalidate" | "dma_cache_invalidate" => (
            "__ncc_kernel_dma_cache_invalidate",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.cacheflush" | "dmacacheflush" | "dma_cache_flush" => (
            "__ncc_kernel_dma_cache_flush",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.iommu.core.commandwrite" | "iommucommandwrite" | "iommu_command_write" => (
            "__ncc_kernel_iommu_command_write",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.iommu.core.mapentry128" | "iommumapentry128" | "iommu_map_entry128" => (
            "__ncc_kernel_iommu_map_entry128",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.iommu.core.faultcopy" | "iommufaultcopy" | "iommu_fault_copy" => (
            "__ncc_kernel_iommu_fault_copy",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.dma.core.fence" | "dmafence" | "dma_fence" => {
            ("__ncc_kernel_dma_fence", &[][..], "i1")
        }
        "kernel.virtualization.memory.read64" | "guestphysread64" | "guest_phys_read64" => {
            ("__ncc_kernel_guest_phys_read64", &["i64"][..], "i64")
        }
        "kernel.virtualization.memory.write64" | "guestphyswrite64" | "guest_phys_write64" => {
            ("__ncc_kernel_guest_phys_write64", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.memory.zeropage" | "guestphyszeropage" | "guest_phys_zero_page" => {
            ("__ncc_kernel_guest_phys_zero_page", &["i64"][..], "i1")
        }
        "kernel.virtualization.intel.eptpointer" | "eptpointer" | "ept_pointer" => {
            ("__ncc_kernel_ept_pointer", &["i64", "i64", "i1"][..], "i64")
        }
        "kernel.virtualization.intel.eptsetentry" | "eptsetentry" | "ept_set_entry" => (
            "__ncc_kernel_ept_set_entry",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.amd.nptsetentry" | "nptsetentry" | "npt_set_entry" => (
            "__ncc_kernel_npt_set_entry",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.intel.exitsnapshot"
        | "vmexitintelsnapshot"
        | "vmexit_intel_snapshot" => ("__ncc_kernel_vmexit_intel_snapshot", &["i64"][..], "i1"),
        "kernel.virtualization.amd.exitsnapshot" | "vmexitamdsnapshot" | "vmexit_amd_snapshot" => (
            "__ncc_kernel_vmexit_amd_snapshot",
            &["i64", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.intel.inject" | "vmexitintelinject" | "vmexit_intel_inject" => (
            "__ncc_kernel_vmexit_intel_inject",
            &["i64", "i64", "i1", "i64", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.amd.inject" | "vmexitamdinject" | "vmexit_amd_inject" => (
            "__ncc_kernel_vmexit_amd_inject",
            &["i64", "i64", "i64", "i1", "i64"][..],
            "i1",
        ),
        "kernel.virtualization.intel.advancerip"
        | "vmexitinteladvance"
        | "vmexit_intel_advance" => ("__ncc_kernel_vmexit_intel_advance", &["i64"][..], "i1"),
        "kernel.virtualization.amd.advancerip" | "vmexitamdadvance" | "vmexit_amd_advance" => {
            ("__ncc_kernel_vmexit_amd_advance", &["i64"][..], "i1")
        }
        "kernel.arch.x86.readcr0" | "cpureadcr0" | "cpu_read_cr0" => {
            ("__ncc_kernel_cpu_read_cr0", &[][..], "i64")
        }
        "kernel.arch.x86.readcr3" | "cpureadcr3" | "cpu_read_cr3" => {
            ("__ncc_kernel_cpu_read_cr3", &[][..], "i64")
        }
        "kernel.arch.x86.readcr4" | "cpureadcr4" | "cpu_read_cr4" => {
            ("__ncc_kernel_cpu_read_cr4", &[][..], "i64")
        }
        "kernel.arch.x86.writecr0" | "cpuwritecr0" | "cpu_write_cr0" => {
            ("__ncc_kernel_cpu_write_cr0", &["i64"][..], "i1")
        }
        "kernel.arch.x86.writecr3" | "cpuwritecr3" | "cpu_write_cr3" => {
            ("__ncc_kernel_cpu_write_cr3", &["i64"][..], "i1")
        }
        "kernel.arch.x86.writecr4" | "cpuwritecr4" | "cpu_write_cr4" => {
            ("__ncc_kernel_cpu_write_cr4", &["i64"][..], "i1")
        }
        "kernel.arch.x86.readmsr" | "cpureadmsr" | "cpu_read_msr" => {
            ("__ncc_kernel_cpu_read_msr", &["i64"][..], "i64")
        }
        "kernel.arch.x86.writemsr" | "cpuwritemsr" | "cpu_write_msr" => {
            ("__ncc_kernel_cpu_write_msr", &["i64", "i64"][..], "i1")
        }
        "kernel.arch.x86.cpuidwrite" | "cpuidwrite" | "cpuid_write" => {
            ("__ncc_kernel_cpuid_write", &["i64", "i64", "i64"][..], "i1")
        }
        "kernel.io.port.read8" | "portread8" | "port_read8" => {
            ("__ncc_kernel_port_read8", &["i64"][..], "i64")
        }
        "kernel.io.port.read16" | "portread16" | "port_read16" => {
            ("__ncc_kernel_port_read16", &["i64"][..], "i64")
        }
        "kernel.io.port.read32" | "portread32" | "port_read32" => {
            ("__ncc_kernel_port_read32", &["i64"][..], "i64")
        }
        "kernel.io.port.write8" | "portwrite8" | "port_write8" => {
            ("__ncc_kernel_port_write8", &["i64", "i64"][..], "i1")
        }
        "kernel.io.port.write16" | "portwrite16" | "port_write16" => {
            ("__ncc_kernel_port_write16", &["i64", "i64"][..], "i1")
        }
        "kernel.io.port.write32" | "portwrite32" | "port_write32" => {
            ("__ncc_kernel_port_write32", &["i64", "i64"][..], "i1")
        }
        "kernel.pci.config.read32" | "pciconfigread32" | "pci_config_read32" => (
            "__ncc_kernel_pci_config_read32",
            &["i64", "i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.pci.config.write32" | "pciconfigwrite32" | "pci_config_write32" => (
            "__ncc_kernel_pci_config_write32",
            &["i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.pci.ecam.read32" | "pciecamread32" | "pci_ecam_read32" => (
            "__ncc_kernel_pci_ecam_read32",
            &["i64", "i64", "i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.pci.ecam.write32" | "pciecamwrite32" | "pci_ecam_write32" => (
            "__ncc_kernel_pci_ecam_write32",
            &["i64", "i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.virtio.queue.notify" | "virtionotify" | "virtio_notify" => {
            ("__ncc_kernel_virtio_notify", &["i64", "i64"][..], "i1")
        }
        "kernel.debug.break" | "debugbreak" | "debug_break" => {
            ("__ncc_kernel_debug_break", &[][..], "i1")
        }
        "kernel.debug.registersnapshot" | "debugregistersnapshot" | "debug_register_snapshot" => (
            "__ncc_kernel_debug_register_snapshot",
            &["i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.debug.memorydump" | "debugmemorydump" | "debug_memory_dump" => (
            "__ncc_kernel_debug_memory_dump",
            &["i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.debug.stacktrace" | "debugstacktrace" | "debug_stack_trace" => (
            "__ncc_kernel_debug_stack_trace",
            &["i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.debug.symbollookup" | "debugsymbollookup" | "debug_symbol_lookup" => (
            "__ncc_kernel_debug_symbol_lookup",
            &["i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.debug.rspchecksum" | "debugrspchecksum" | "debug_rsp_checksum" => (
            "__ncc_kernel_debug_rsp_checksum",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.debug.rspframe" | "debugrspframe" | "debug_rsp_frame" => (
            "__ncc_kernel_debug_rsp_frame",
            &["i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.debug.breakpointinstall"
        | "debugbreakpointinstall"
        | "debug_breakpoint_install" => (
            "__ncc_kernel_debug_breakpoint_install",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.debug.breakpointremove" | "debugbreakpointremove" | "debug_breakpoint_remove" => (
            "__ncc_kernel_debug_breakpoint_remove",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.debug.crashreport" | "debugcrashreport" | "debug_crash_report" => (
            "__ncc_kernel_debug_crash_report",
            &["i64", "i64", "i64", "i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.debug.panicraw" | "panicraw" | "panic_raw" => {
            ("__ncc_kernel_panic_raw", &["i64", "i64"][..], "i1")
        }
        "kernel.hardening.kaslrrelocate" | "kaslrrelocate" | "kaslr_relocate" => {
            ("__ncc_kernel_kaslr_relocate", &["i64", "i64"][..], "i64")
        }
        "kernel.hardening.canarywrite" | "stackcanarywrite" | "stack_canary_write" => {
            ("__ncc_kernel_stack_canary_write", &["i64", "i64"][..], "i1")
        }
        "kernel.hardening.canarycheck" | "stackcanarycheck" | "stack_canary_check" => {
            ("__ncc_kernel_stack_canary_check", &["i64", "i64"][..], "i1")
        }
        "kernel.hardening.sanitizerpoison" | "sanitizerpoison" | "sanitizer_poison" => (
            "__ncc_kernel_sanitizer_poison",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.hardening.sanitizerunpoison" | "sanitizerunpoison" | "sanitizer_unpoison" => {
            ("__ncc_kernel_sanitizer_unpoison", &["i64", "i64"][..], "i1")
        }
        "kernel.hardening.sanitizercheck" | "sanitizercheck" | "sanitizer_check" => {
            ("__ncc_kernel_sanitizer_check", &["i64", "i64"][..], "i1")
        }
        "kernel.virtualization.amd.clgi" | "svmclgi" | "svm_clgi" => {
            ("__ncc_kernel_svm_clgi", &[][..], "i1")
        }
        "kernel.virtualization.amd.stgi" | "svmstgi" | "svm_stgi" => {
            ("__ncc_kernel_svm_stgi", &[][..], "i1")
        }
        "kernel.virtualization.amd.invlpga" | "svminvlpga" | "svm_invlpga" => {
            ("__ncc_kernel_svm_invlpga", &["i64", "i64"][..], "i1")
        }
        "kernel.atomic.core.load32" | "atomic32load" | "atomic32_load" => {
            ("__ncc_kernel_atomic32_load", &["i64"][..], "i64")
        }
        "kernel.atomic.core.store32" | "atomic32store" | "atomic32_store" => {
            ("__ncc_kernel_atomic32_store", &["i64", "i64"][..], "i1")
        }
        "kernel.atomic.core.exchange32" | "atomic32exchange" | "atomic32_exchange" => {
            ("__ncc_kernel_atomic32_exchange", &["i64", "i64"][..], "i64")
        }
        "kernel.atomic.core.compareexchange32"
        | "atomic32compareexchange"
        | "atomic32_compare_exchange" => (
            "__ncc_kernel_atomic32_compare_exchange",
            &["i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchadd32" | "atomic32fetchadd" | "atomic32_fetch_add" => (
            "__ncc_kernel_atomic32_fetch_add",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchsub32" | "atomic32fetchsub" | "atomic32_fetch_sub" => (
            "__ncc_kernel_atomic32_fetch_sub",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchand32" | "atomic32fetchand" | "atomic32_fetch_and" => (
            "__ncc_kernel_atomic32_fetch_and",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchor32" | "atomic32fetchor" | "atomic32_fetch_or" => {
            ("__ncc_kernel_atomic32_fetch_or", &["i64", "i64"][..], "i64")
        }
        "kernel.atomic.core.fetchxor32" | "atomic32fetchxor" | "atomic32_fetch_xor" => (
            "__ncc_kernel_atomic32_fetch_xor",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.load64" | "atomic64load" | "atomic64_load" => {
            ("__ncc_kernel_atomic64_load", &["i64"][..], "i64")
        }
        "kernel.atomic.core.store64" | "atomic64store" | "atomic64_store" => {
            ("__ncc_kernel_atomic64_store", &["i64", "i64"][..], "i1")
        }
        "kernel.atomic.core.exchange64" | "atomic64exchange" | "atomic64_exchange" => {
            ("__ncc_kernel_atomic64_exchange", &["i64", "i64"][..], "i64")
        }
        "kernel.atomic.core.compareexchange64"
        | "atomic64compareexchange"
        | "atomic64_compare_exchange" => (
            "__ncc_kernel_atomic64_compare_exchange",
            &["i64", "i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchadd64" | "atomic64fetchadd" | "atomic64_fetch_add" => (
            "__ncc_kernel_atomic64_fetch_add",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchsub64" | "atomic64fetchsub" | "atomic64_fetch_sub" => (
            "__ncc_kernel_atomic64_fetch_sub",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchand64" | "atomic64fetchand" | "atomic64_fetch_and" => (
            "__ncc_kernel_atomic64_fetch_and",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fetchor64" | "atomic64fetchor" | "atomic64_fetch_or" => {
            ("__ncc_kernel_atomic64_fetch_or", &["i64", "i64"][..], "i64")
        }
        "kernel.atomic.core.fetchxor64" | "atomic64fetchxor" | "atomic64_fetch_xor" => (
            "__ncc_kernel_atomic64_fetch_xor",
            &["i64", "i64"][..],
            "i64",
        ),
        "kernel.atomic.core.fenceacquire" | "atomicfenceacquire" | "atomic_fence_acquire" => {
            ("__ncc_kernel_atomic_fence_acquire", &[][..], "i1")
        }
        "kernel.atomic.core.fencerelease" | "atomicfencerelease" | "atomic_fence_release" => {
            ("__ncc_kernel_atomic_fence_release", &[][..], "i1")
        }
        "kernel.atomic.core.fenceseqcst" | "atomicfenceseqcst" | "atomic_fence_seq_cst" => {
            ("__ncc_kernel_atomic_fence_seq_cst", &[][..], "i1")
        }
        "kernel.atomic.core.spintrylock" | "spintrylock" | "spin_try_lock" => {
            ("__ncc_kernel_spin_try_lock", &["i64"][..], "i1")
        }
        "kernel.atomic.core.spinlock" | "spinlock" | "spin_lock" => {
            ("__ncc_kernel_spin_lock", &["i64"][..], "i1")
        }
        "kernel.atomic.core.spinunlock" | "spinunlock" | "spin_unlock" => {
            ("__ncc_kernel_spin_unlock", &["i64"][..], "i1")
        }
        "kernel.mmio.core.read8" | "mmioread8" | "mmio_read8" => {
            ("__ncc_kernel_mmio_read8", &["i64"][..], "i64")
        }
        "kernel.mmio.core.read16" | "mmioread16" | "mmio_read16" => {
            ("__ncc_kernel_mmio_read16", &["i64"][..], "i64")
        }
        "kernel.mmio.core.read32" | "mmioread32" | "mmio_read32" => {
            ("__ncc_kernel_mmio_read32", &["i64"][..], "i64")
        }
        "kernel.mmio.core.read64" | "mmioread64" | "mmio_read64" => {
            ("__ncc_kernel_mmio_read64", &["i64"][..], "i64")
        }
        "kernel.mmio.core.read128" | "mmioread128" | "mmio_read128" => {
            ("__ncc_kernel_mmio_read128", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.read256" | "mmioread256" | "mmio_read256" => {
            ("__ncc_kernel_mmio_read256", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.read512" | "mmioread512" | "mmio_read512" => {
            ("__ncc_kernel_mmio_read512", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write8" | "mmiowrite8" | "mmio_write8" => {
            ("__ncc_kernel_mmio_write8", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write16" | "mmiowrite16" | "mmio_write16" => {
            ("__ncc_kernel_mmio_write16", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write32" | "mmiowrite32" | "mmio_write32" => {
            ("__ncc_kernel_mmio_write32", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write64" | "mmiowrite64" | "mmio_write64" => {
            ("__ncc_kernel_mmio_write64", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write128" | "mmiowrite128" | "mmio_write128" => {
            ("__ncc_kernel_mmio_write128", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write256" | "mmiowrite256" | "mmio_write256" => {
            ("__ncc_kernel_mmio_write256", &["i64", "i64"][..], "i1")
        }
        "kernel.mmio.core.write512" | "mmiowrite512" | "mmio_write512" => {
            ("__ncc_kernel_mmio_write512", &["i64", "i64"][..], "i1")
        }
        "kernel.apic.core.read32" | "apicread32" | "apic_read32" => {
            ("__ncc_kernel_mmio_read32", &["i64"][..], "i64")
        }
        "kernel.apic.core.write32" | "apicwrite32" | "apic_write32" => {
            ("__ncc_kernel_mmio_write32", &["i64", "i64"][..], "i1")
        }
        "kernel.apic.core.eoi" | "apiceoi" | "apic_eoi" => {
            ("__ncc_kernel_apic_eoi", &["i64"][..], "i1")
        }
        "kernel.apic.core.sendipi" | "apicsendipi" | "apic_send_ipi" => (
            "__ncc_kernel_apic_send_ipi",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.apic.core.x2read" | "x2apicread" | "x2apic_read" => {
            ("__ncc_kernel_cpu_read_msr", &["i64"][..], "i64")
        }
        "kernel.apic.core.x2write" | "x2apicwrite" | "x2apic_write" => {
            ("__ncc_kernel_cpu_write_msr", &["i64", "i64"][..], "i1")
        }
        "kernel.ioapic.core.read" | "ioapicread" | "ioapic_read" => {
            ("__ncc_kernel_ioapic_read", &["i64", "i64"][..], "i64")
        }
        "kernel.ioapic.core.write" | "ioapicwrite" | "ioapic_write" => (
            "__ncc_kernel_ioapic_write",
            &["i64", "i64", "i64"][..],
            "i1",
        ),
        "kernel.hpet.core.read64" | "hpetread64" | "hpet_read64" => {
            ("__ncc_kernel_mmio_read64", &["i64"][..], "i64")
        }
        "kernel.hpet.core.write64" | "hpetwrite64" | "hpet_write64" => {
            ("__ncc_kernel_mmio_write64", &["i64", "i64"][..], "i1")
        }
        "kernel.timer.core.tscread" | "tscread" | "tsc_read" => {
            ("__ncc_kernel_arch_cycle_counter", &[][..], "i64")
        }
        "kernel.timer.core.tscdeadlinewrite" | "tscdeadlinewrite" | "tsc_deadline_write" => {
            ("__ncc_kernel_tsc_deadline_write", &["i64"][..], "i1")
        }
        "kernel.arch.core.readbarrier" | "readbarrier" | "read_barrier" => {
            ("__ncc_kernel_arch_memory_barrier", &[][..], "i1")
        }
        "kernel.arch.core.writebarrier" | "writebarrier" | "write_barrier" => {
            ("__ncc_kernel_arch_memory_barrier", &[][..], "i1")
        }
        "memcpy" => ("memcpy", &["ptr", "ptr", "i64"][..], "ptr"),
        "memset" => ("memset", &["ptr", "i32", "i64"][..], "ptr"),
        "memmove" => ("memmove", &["ptr", "ptr", "i64"][..], "ptr"),
        "memcmp" => ("memcmp", &["ptr", "ptr", "i64"][..], "i32"),
        _ => return None,
    };
    Some(KernelCoreSignature {
        symbol: sig.0,
        params: sig.1,
        return_type: sig.2,
    })
}

pub(crate) fn render_kernel_core_runtime(target_triple: &str) -> String {
    let authority = ncc_machine::machine_coordinator::coordinate_target_kernel(target_triple)
        .map(|plan| plan.llvm_authority_comment())
        .unwrap_or_else(|error| {
            format!(
                "; ncc-machine-authority status=not-applicable target={} reason={}\n",
                target_triple,
                error.to_string().replace('\n', " ")
            )
        });
    let mut llvm = format!(
        "{authority}{}",
        r#"
@__ncc_kernel_boot_info_slot = internal global i64 0, align 8
@__ncc_kernel_scheduler_open_slot = internal global i1 false, align 1
@__ncc_kernel_panic_slot = internal global i1 false, align 1
@__ncc_kernel_halted_slot = internal global i1 false, align 1
@__ncc_kernel_ticks_slot = internal global i64 0, align 8
@__ncc_kernel_current_slot = internal global i32 -1, align 4
@__ncc_kernel_quarantine_slot = internal global i64 0, align 8

@__ncc_kernel_task_pid = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_task_states = internal global [64 x i32] zeroinitializer, align 4
@__ncc_kernel_task_priority = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_task_stack = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_task_name = internal global [64 x ptr] zeroinitializer, align 8
@__ncc_kernel_task_exit_code = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_task_affinity_valid = internal global [64 x i1] zeroinitializer, align 8
@__ncc_kernel_task_affinity = internal global [64 x i64] zeroinitializer, align 8

@__ncc_kernel_cpu_used = internal global [64 x i1] zeroinitializer, align 8
@__ncc_kernel_cpu_online_slots = internal global [64 x i1] zeroinitializer, align 8
@__ncc_kernel_cpu_bootstrap = internal global [64 x i1] zeroinitializer, align 8
@__ncc_kernel_cpu_stack = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_cpu_current_valid = internal global [64 x i1] zeroinitializer, align 8
@__ncc_kernel_cpu_current_task = internal global [64 x i32] zeroinitializer, align 8
@__ncc_kernel_current_cpu = internal global i64 0, align 8
@__ncc_kernel_cpu_bootstrap_registered = internal global i1 false, align 1
@__ncc_kernel_cpu_local = internal global [2048 x i64] zeroinitializer, align 16

@__ncc_kernel_ipi_queue = internal global [16384 x i64] zeroinitializer, align 16
@__ncc_kernel_ipi_head = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_ipi_tail = internal global [64 x i64] zeroinitializer, align 8
@__ncc_kernel_ipi_count = internal global [64 x i64] zeroinitializer, align 8

@__ncc_kernel_frame_base = internal global i64 0, align 8
@__ncc_kernel_frame_count = internal global i64 0, align 8
@__ncc_kernel_frame_bitmap = internal global [4096 x i8] zeroinitializer, align 16

@__ncc_kernel_page_used = internal global [256 x i1] zeroinitializer, align 8
@__ncc_kernel_page_virt = internal global [256 x i64] zeroinitializer, align 8
@__ncc_kernel_page_phys = internal global [256 x i64] zeroinitializer, align 8
@__ncc_kernel_page_flags = internal global [256 x i64] zeroinitializer, align 8

@__ncc_kernel_irq_handler = internal global [256 x i64] zeroinitializer, align 8
@__ncc_kernel_irq_enabled = internal global [256 x i1] zeroinitializer, align 8
@__ncc_kernel_irq_dispatch_count = internal global [256 x i64] zeroinitializer, align 8
@__ncc_kernel_irq_eoi_count = internal global [256 x i64] zeroinitializer, align 8

@__ncc_kernel_syscall_handler = internal global [256 x i64] zeroinitializer, align 8
@__ncc_kernel_driver_used = internal global [32 x i1] zeroinitializer, align 8
@__ncc_kernel_driver_state = internal global [32 x i32] zeroinitializer, align 4
@__ncc_kernel_driver_base = internal global [32 x i64] zeroinitializer, align 8
@__ncc_kernel_driver_irq = internal global [32 x i64] zeroinitializer, align 8

define void @__ncc_kernel_set_boot_info(i64 %boot_info) {
entry:
  %normalized = call i64 @__ncc_kernel_normalize_boot_info(i64 %boot_info)
  store i64 %normalized, ptr @__ncc_kernel_boot_info_slot, align 8
  ret void
}

define internal i32 @__ncc_kernel_find_task(i64 %pid) {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %cont ]
  %done = icmp uge i32 %i, 64
  br i1 %done, label %miss, label %check
check:
  %state.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %i
  %state = load i32, ptr %state.ptr, align 4
  %live = icmp ne i32 %state, 0
  %pid.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_pid, i32 0, i32 %i
  %candidate = load i64, ptr %pid.ptr, align 8
  %same = icmp eq i64 %candidate, %pid
  %found = and i1 %live, %same
  br i1 %found, label %hit, label %cont
cont:
  %next = add nuw i32 %i, 1
  br label %loop
hit:
  ret i32 %i
miss:
  ret i32 -1
}

define i1 @__ncc_kernel_task_spawn(i64 %pid, i64 %priority, i64 %stack) {
entry:
  %pid.ok = icmp sgt i64 %pid, 0
  %panic = load i1, ptr @__ncc_kernel_panic_slot, align 1
  %halted = load i1, ptr @__ncc_kernel_halted_slot, align 1
  %blocked = or i1 %panic, %halted
  %state.ok = xor i1 %blocked, true
  %valid = and i1 %pid.ok, %state.ok
  br i1 %valid, label %unique, label %fail
unique:
  %existing = call i32 @__ncc_kernel_find_task(i64 %pid)
  %is.new = icmp slt i32 %existing, 0
  br i1 %is.new, label %scan, label %fail
scan:
  %i = phi i32 [ 0, %unique ], [ %next, %occupied ]
  %done = icmp uge i32 %i, 64
  br i1 %done, label %fail, label %slot
slot:
  %state.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %i
  %state = load i32, ptr %state.ptr, align 4
  %free = icmp eq i32 %state, 0
  br i1 %free, label %insert, label %occupied
occupied:
  %next = add nuw i32 %i, 1
  br label %scan
insert:
  %pid.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_pid, i32 0, i32 %i
  %priority.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_priority, i32 0, i32 %i
  %stack.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_stack, i32 0, i32 %i
  store i64 %pid, ptr %pid.ptr, align 8
  store i64 %priority, ptr %priority.ptr, align 8
  store i64 %stack, ptr %stack.ptr, align 8
  store i32 1, ptr %state.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_task_register(i64 %pid, ptr %name) {
entry:
  %ok = call i1 @__ncc_kernel_task_spawn(i64 %pid, i64 0, i64 0)
  br i1 %ok, label %store, label %done
store:
  %idx = call i32 @__ncc_kernel_find_task(i64 %pid)
  %name.ptr = getelementptr [64 x ptr], ptr @__ncc_kernel_task_name, i32 0, i32 %idx
  store ptr %name, ptr %name.ptr, align 8
  br label %done
done:
  ret i1 %ok
}

define i1 @__ncc_kernel_task_block(i64 %pid) {
entry:
  %idx = call i32 @__ncc_kernel_find_task(i64 %pid)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %set, label %fail
set:
  %ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %idx
  store i32 2, ptr %ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_task_wake(i64 %pid) {
entry:
  %idx = call i32 @__ncc_kernel_find_task(i64 %pid)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %check, label %fail
check:
  %ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %idx
  %state = load i32, ptr %ptr, align 4
  %blocked = icmp eq i32 %state, 2
  br i1 %blocked, label %set, label %fail
set:
  store i32 1, ptr %ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_task_exit(i64 %pid, i64 %code) {
entry:
  %idx = call i32 @__ncc_kernel_find_task(i64 %pid)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %set, label %fail
set:
  %state.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %idx
  %code.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_exit_code, i32 0, i32 %idx
  store i32 3, ptr %state.ptr, align 4
  store i64 %code, ptr %code.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_task_state(i64 %pid) {
entry:
  %idx = call i32 @__ncc_kernel_find_task(i64 %pid)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %idx
  %state = load i32, ptr %ptr, align 4
  %wide = zext i32 %state to i64
  ret i64 %wide
fail:
  ret i64 -1
}

define i64 @__ncc_kernel_task_current() {
entry:
  %idx = load i32, ptr @__ncc_kernel_current_slot, align 4
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %read, label %none
read:
  %ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_pid, i32 0, i32 %idx
  %pid = load i64, ptr %ptr, align 8
  ret i64 %pid
none:
  ret i64 0
}

define i1 @__ncc_kernel_schedule_open() {
entry:
  %panic = load i1, ptr @__ncc_kernel_panic_slot, align 1
  %halted = load i1, ptr @__ncc_kernel_halted_slot, align 1
  %bad = or i1 %panic, %halted
  br i1 %bad, label %fail, label %open
open:
  store i1 true, ptr @__ncc_kernel_scheduler_open_slot, align 1
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_schedule_close() {
entry:
  store i1 false, ptr @__ncc_kernel_scheduler_open_slot, align 1
  ret i1 true
}

define i1 @__ncc_kernel_schedule_next() {
entry:
  %cpu = load i64, ptr @__ncc_kernel_current_cpu, align 8
  %ok = call i1 @__ncc_kernel_schedule_next_on(i64 %cpu)
  ret i1 %ok
}

define i64 @__ncc_kernel_schedule_tick() {
entry:
  %old = load i64, ptr @__ncc_kernel_ticks_slot, align 8
  %next = add i64 %old, 1
  store i64 %next, ptr @__ncc_kernel_ticks_slot, align 8
  %ignored = call i1 @__ncc_kernel_schedule_next()
  ret i64 %next
}

define i64 @__ncc_kernel_schedule_yield() {
entry:
  %ignored = call i1 @__ncc_kernel_schedule_next()
  %pid = call i64 @__ncc_kernel_task_current()
  ret i64 %pid
}

define i1 @__ncc_kernel_cpu_register(i64 %cpu, i1 %bootstrap, i64 %stack) {
entry:
  %low = icmp sge i64 %cpu, 0
  %high = icmp ult i64 %cpu, 64
  %range = and i1 %low, %high
  %stack.ok = icmp sge i64 %stack, 0
  %valid = and i1 %range, %stack.ok
  br i1 %valid, label %check, label %fail
check:
  %idx = trunc i64 %cpu to i32
  %used.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %idx
  %used = load i1, ptr %used.ptr, align 1
  %free = xor i1 %used, true
  %bootstrap.used = load i1, ptr @__ncc_kernel_cpu_bootstrap_registered, align 1
  %bootstrap.conflict = and i1 %bootstrap, %bootstrap.used
  %bootstrap.ok = xor i1 %bootstrap.conflict, true
  %ok = and i1 %free, %bootstrap.ok
  br i1 %ok, label %store, label %fail
store:
  %online.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_online_slots, i32 0, i32 %idx
  %bootstrap.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_bootstrap, i32 0, i32 %idx
  %stack.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_cpu_stack, i32 0, i32 %idx
  store i1 true, ptr %used.ptr, align 1
  store i1 %bootstrap, ptr %online.ptr, align 1
  store i1 %bootstrap, ptr %bootstrap.ptr, align 1
  store i64 %stack, ptr %stack.ptr, align 8
  br i1 %bootstrap, label %store.bootstrap, label %done
store.bootstrap:
  store i1 true, ptr @__ncc_kernel_cpu_bootstrap_registered, align 1
  store i64 %cpu, ptr @__ncc_kernel_current_cpu, align 8
  br label %done
done:
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_start(i64 %cpu) {
entry:
  %low = icmp sge i64 %cpu, 0
  %high = icmp ult i64 %cpu, 64
  %range = and i1 %low, %high
  br i1 %range, label %check, label %fail
check:
  %idx = trunc i64 %cpu to i32
  %used.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %idx
  %used = load i1, ptr %used.ptr, align 1
  %panic = load i1, ptr @__ncc_kernel_panic_slot, align 1
  %halted = load i1, ptr @__ncc_kernel_halted_slot, align 1
  %blocked = or i1 %panic, %halted
  %progress = xor i1 %blocked, true
  %ok = and i1 %used, %progress
  br i1 %ok, label %start, label %fail
start:
  %online.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_online_slots, i32 0, i32 %idx
  store i1 true, ptr %online.ptr, align 1
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_online(i64 %cpu) {
entry:
  %low = icmp sge i64 %cpu, 0
  %high = icmp ult i64 %cpu, 64
  %range = and i1 %low, %high
  br i1 %range, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_online_slots, i32 0, i32 %idx
  %online = load i1, ptr %ptr, align 1
  ret i1 %online
fail:
  ret i1 false
}

define i64 @__ncc_kernel_cpu_current() {
entry:
  %cpu = load i64, ptr @__ncc_kernel_current_cpu, align 8
  ret i64 %cpu
}

define i64 @__ncc_kernel_cpu_count() {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %body ]
  %count = phi i64 [ 0, %entry ], [ %count.next, %body ]
  %done = icmp uge i32 %i, 64
  br i1 %done, label %exit, label %body
body:
  %ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %i
  %used = load i1, ptr %ptr, align 1
  %delta = zext i1 %used to i64
  %count.next = add i64 %count, %delta
  %next = add nuw i32 %i, 1
  br label %loop
exit:
  ret i64 %count
}

define i64 @__ncc_kernel_cpu_online_count() {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %body ]
  %count = phi i64 [ 0, %entry ], [ %count.next, %body ]
  %done = icmp uge i32 %i, 64
  br i1 %done, label %exit, label %body
body:
  %ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_online_slots, i32 0, i32 %i
  %online = load i1, ptr %ptr, align 1
  %delta = zext i1 %online to i64
  %count.next = add i64 %count, %delta
  %next = add nuw i32 %i, 1
  br label %loop
exit:
  ret i64 %count
}

define i1 @__ncc_kernel_cpu_set_current(i64 %cpu) {
entry:
  %online = call i1 @__ncc_kernel_cpu_online(i64 %cpu)
  br i1 %online, label %set, label %fail
set:
  store i64 %cpu, ptr @__ncc_kernel_current_cpu, align 8
  %idx = trunc i64 %cpu to i32
  %valid.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_current_valid, i32 0, i32 %idx
  %valid = load i1, ptr %valid.ptr, align 1
  br i1 %valid, label %copy, label %clear
copy:
  %task.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_cpu_current_task, i32 0, i32 %idx
  %task = load i32, ptr %task.ptr, align 4
  store i32 %task, ptr @__ncc_kernel_current_slot, align 4
  br label %done
clear:
  store i32 -1, ptr @__ncc_kernel_current_slot, align 4
  br label %done
done:
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_local_set(i64 %cpu, i64 %slot, i64 %value) {
entry:
  %cpu.low = icmp sge i64 %cpu, 0
  %cpu.high = icmp ult i64 %cpu, 64
  %cpu.range = and i1 %cpu.low, %cpu.high
  %slot.low = icmp sge i64 %slot, 0
  %slot.high = icmp ult i64 %slot, 32
  %slot.range = and i1 %slot.low, %slot.high
  %range = and i1 %cpu.range, %slot.range
  br i1 %range, label %check, label %fail
check:
  %cpu.idx = trunc i64 %cpu to i32
  %used.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %cpu.idx
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %store, label %fail
store:
  %base = mul nuw i64 %cpu, 32
  %index = add nuw i64 %base, %slot
  %ptr = getelementptr [2048 x i64], ptr @__ncc_kernel_cpu_local, i64 0, i64 %index
  store i64 %value, ptr %ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_cpu_local_get(i64 %cpu, i64 %slot) {
entry:
  %cpu.low = icmp sge i64 %cpu, 0
  %cpu.high = icmp ult i64 %cpu, 64
  %cpu.range = and i1 %cpu.low, %cpu.high
  %slot.low = icmp sge i64 %slot, 0
  %slot.high = icmp ult i64 %slot, 32
  %slot.range = and i1 %slot.low, %slot.high
  %range = and i1 %cpu.range, %slot.range
  br i1 %range, label %check, label %fail
check:
  %cpu.idx = trunc i64 %cpu to i32
  %used.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %cpu.idx
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %read, label %fail
read:
  %base = mul nuw i64 %cpu, 32
  %index = add nuw i64 %base, %slot
  %ptr = getelementptr [2048 x i64], ptr @__ncc_kernel_cpu_local, i64 0, i64 %index
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_stack_get(i64 %cpu) {
entry:
  %low = icmp sge i64 %cpu, 0
  %high = icmp ult i64 %cpu, 64
  %range = and i1 %low, %high
  br i1 %range, label %check, label %none
check:
  %idx = trunc i64 %cpu to i32
  %used.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %idx
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %read, label %none
read:
  %stack.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_cpu_stack, i32 0, i32 %idx
  %stack = load i64, ptr %stack.ptr, align 8
  ret i64 %stack
none:
  ret i64 0
}

define i1 @__ncc_kernel_schedule_bind(i64 %pid, i64 %cpu) {
entry:
  %online = call i1 @__ncc_kernel_cpu_online(i64 %cpu)
  %task = call i32 @__ncc_kernel_find_task(i64 %pid)
  %task.ok = icmp sge i32 %task, 0
  %ok = and i1 %online, %task.ok
  br i1 %ok, label %store, label %fail
store:
  %valid.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_task_affinity_valid, i32 0, i32 %task
  %cpu.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_affinity, i32 0, i32 %task
  store i1 true, ptr %valid.ptr, align 1
  store i64 %cpu, ptr %cpu.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_schedule_next_on(i64 %cpu) {
entry:
  %cpu.low = icmp sge i64 %cpu, 0
  %cpu.high = icmp ult i64 %cpu, 64
  %cpu.range = and i1 %cpu.low, %cpu.high
  br i1 %cpu.range, label %state, label %fail
state:
  %open = load i1, ptr @__ncc_kernel_scheduler_open_slot, align 1
  %panic = load i1, ptr @__ncc_kernel_panic_slot, align 1
  %halted = load i1, ptr @__ncc_kernel_halted_slot, align 1
  %blocked = or i1 %panic, %halted
  %progress = xor i1 %blocked, true
  %kernel.ready = and i1 %open, %progress
  %cpu.idx = trunc i64 %cpu to i32
  %used.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_used, i32 0, i32 %cpu.idx
  %used = load i1, ptr %used.ptr, align 1
  %online.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_online_slots, i32 0, i32 %cpu.idx
  %online = load i1, ptr %online.ptr, align 1
  %legacy.cpu = icmp eq i64 %cpu, 0
  %not.used = xor i1 %used, true
  %legacy.allowed = and i1 %legacy.cpu, %not.used
  %cpu.ready = or i1 %online, %legacy.allowed
  %ready = and i1 %kernel.ready, %cpu.ready
  br i1 %ready, label %prepare, label %fail
prepare:
  %current.valid.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_current_valid, i32 0, i32 %cpu.idx
  %current.valid = load i1, ptr %current.valid.ptr, align 1
  %current.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_cpu_current_task, i32 0, i32 %cpu.idx
  %cpu.current = load i32, ptr %current.ptr, align 4
  %legacy.current = load i32, ptr @__ncc_kernel_current_slot, align 4
  %current = select i1 %current.valid, i32 %cpu.current, i32 %legacy.current
  br label %scan
scan:
  %step = phi i32 [ 1, %prepare ], [ %next.step, %miss ]
  %done = icmp ugt i32 %step, 64
  br i1 %done, label %fail, label %candidate
candidate:
  %sum = add i32 %current, %step
  %idx = and i32 %sum, 63
  %state.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %idx
  %task.state = load i32, ptr %state.ptr, align 4
  %is.ready = icmp eq i32 %task.state, 1
  %aff.valid.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_task_affinity_valid, i32 0, i32 %idx
  %aff.valid = load i1, ptr %aff.valid.ptr, align 1
  %aff.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_affinity, i32 0, i32 %idx
  %aff = load i64, ptr %aff.ptr, align 8
  %aff.same = icmp eq i64 %aff, %cpu
  %aff.ok = select i1 %aff.valid, i1 %aff.same, i1 true
  %eligible = and i1 %is.ready, %aff.ok
  br i1 %eligible, label %select, label %miss
miss:
  %next.step = add nuw i32 %step, 1
  br label %scan
select:
  %had.current = icmp sge i32 %current, 0
  br i1 %had.current, label %release, label %activate
release:
  %old.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %current
  %old.state = load i32, ptr %old.ptr, align 4
  %was.running = icmp eq i32 %old.state, 4
  br i1 %was.running, label %release.store, label %activate
release.store:
  store i32 1, ptr %old.ptr, align 4
  br label %activate
activate:
  store i32 4, ptr %state.ptr, align 4
  store i1 true, ptr %current.valid.ptr, align 1
  store i32 %idx, ptr %current.ptr, align 4
  store i64 %cpu, ptr @__ncc_kernel_current_cpu, align 8
  store i32 %idx, ptr @__ncc_kernel_current_slot, align 4
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_schedule_current_on(i64 %cpu) {
entry:
  %low = icmp sge i64 %cpu, 0
  %high = icmp ult i64 %cpu, 64
  %range = and i1 %low, %high
  br i1 %range, label %check, label %none
check:
  %idx = trunc i64 %cpu to i32
  %valid.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_cpu_current_valid, i32 0, i32 %idx
  %valid = load i1, ptr %valid.ptr, align 1
  br i1 %valid, label %read, label %none
read:
  %task.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_cpu_current_task, i32 0, i32 %idx
  %task = load i32, ptr %task.ptr, align 4
  %pid.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_task_pid, i32 0, i32 %task
  %pid = load i64, ptr %pid.ptr, align 8
  ret i64 %pid
none:
  ret i64 0
}

define i1 @__ncc_kernel_ipi_send(i64 %cpu, i64 %vector) {
entry:
  %vector.low = icmp sge i64 %vector, 0
  %vector.high = icmp ult i64 %vector, 256
  %vector.ok = and i1 %vector.low, %vector.high
  %online = call i1 @__ncc_kernel_cpu_online(i64 %cpu)
  %ready = and i1 %vector.ok, %online
  br i1 %ready, label %queue, label %fail
queue:
  %cpu.idx = trunc i64 %cpu to i32
  %count.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_ipi_count, i32 0, i32 %cpu.idx
  %count = load i64, ptr %count.ptr, align 8
  %space = icmp ult i64 %count, 256
  br i1 %space, label %store, label %fail
store:
  %tail.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_ipi_tail, i32 0, i32 %cpu.idx
  %tail = load i64, ptr %tail.ptr, align 8
  %base = mul nuw i64 %cpu, 256
  %queue.index = add nuw i64 %base, %tail
  %queue.ptr = getelementptr [16384 x i64], ptr @__ncc_kernel_ipi_queue, i64 0, i64 %queue.index
  store i64 %vector, ptr %queue.ptr, align 8
  %tail.add = add nuw i64 %tail, 1
  %tail.next = and i64 %tail.add, 255
  %count.next = add nuw i64 %count, 1
  store i64 %tail.next, ptr %tail.ptr, align 8
  store i64 %count.next, ptr %count.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_ipi_broadcast(i64 %vector) {
entry:
  %current = load i64, ptr @__ncc_kernel_current_cpu, align 8
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %cont ]
  %sent = phi i64 [ 0, %entry ], [ %sent.next, %cont ]
  %done = icmp uge i64 %i, 64
  br i1 %done, label %exit, label %try
try:
  %self = icmp eq i64 %i, %current
  br i1 %self, label %skip, label %send
send:
  %ok = call i1 @__ncc_kernel_ipi_send(i64 %i, i64 %vector)
  %delta = zext i1 %ok to i64
  %sent.add = add i64 %sent, %delta
  br label %cont
skip:
  br label %cont
cont:
  %sent.next = phi i64 [ %sent, %skip ], [ %sent.add, %send ]
  %next = add nuw i64 %i, 1
  br label %loop
exit:
  ret i64 %sent
}

define i64 @__ncc_kernel_ipi_pending(i64 %cpu) {
entry:
  %low = icmp sge i64 %cpu, 0
  %high = icmp ult i64 %cpu, 64
  %range = and i1 %low, %high
  br i1 %range, label %read, label %none
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [64 x i64], ptr @__ncc_kernel_ipi_count, i32 0, i32 %idx
  %count = load i64, ptr %ptr, align 8
  ret i64 %count
none:
  ret i64 0
}

define i64 @__ncc_kernel_ipi_peek(i64 %cpu) {
entry:
  %count = call i64 @__ncc_kernel_ipi_pending(i64 %cpu)
  %has = icmp ugt i64 %count, 0
  br i1 %has, label %read, label %none
read:
  %idx = trunc i64 %cpu to i32
  %head.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_ipi_head, i32 0, i32 %idx
  %head = load i64, ptr %head.ptr, align 8
  %base = mul nuw i64 %cpu, 256
  %queue.index = add nuw i64 %base, %head
  %queue.ptr = getelementptr [16384 x i64], ptr @__ncc_kernel_ipi_queue, i64 0, i64 %queue.index
  %vector = load i64, ptr %queue.ptr, align 8
  ret i64 %vector
none:
  ret i64 -1
}

define i1 @__ncc_kernel_ipi_ack(i64 %cpu, i64 %vector) {
entry:
  %front = call i64 @__ncc_kernel_ipi_peek(i64 %cpu)
  %same = icmp eq i64 %front, %vector
  br i1 %same, label %pop, label %fail
pop:
  %idx = trunc i64 %cpu to i32
  %head.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_ipi_head, i32 0, i32 %idx
  %count.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_ipi_count, i32 0, i32 %idx
  %head = load i64, ptr %head.ptr, align 8
  %count = load i64, ptr %count.ptr, align 8
  %head.add = add nuw i64 %head, 1
  %head.next = and i64 %head.add, 255
  %count.next = sub nuw i64 %count, 1
  store i64 %head.next, ptr %head.ptr, align 8
  store i64 %count.next, ptr %count.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_quarantine(i64 %pid) {
entry:
  %idx = call i32 @__ncc_kernel_find_task(i64 %pid)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %set, label %fail
set:
  %ptr = getelementptr [64 x i32], ptr @__ncc_kernel_task_states, i32 0, i32 %idx
  store i32 5, ptr %ptr, align 4
  %old = load i64, ptr @__ncc_kernel_quarantine_slot, align 8
  %next = add i64 %old, 1
  store i64 %next, ptr @__ncc_kernel_quarantine_slot, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_panic(ptr %reason) {
entry:
  %valid = icmp ne ptr %reason, null
  br i1 %valid, label %set, label %fail
set:
  store i1 true, ptr @__ncc_kernel_panic_slot, align 1
  store i1 false, ptr @__ncc_kernel_scheduler_open_slot, align 1
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_halt() {
entry:
  store i1 true, ptr @__ncc_kernel_halted_slot, align 1
  store i1 false, ptr @__ncc_kernel_scheduler_open_slot, align 1
  ret i1 true
}

define i1 @__ncc_kernel_panic_state() {
entry:
  %state = load i1, ptr @__ncc_kernel_panic_slot, align 1
  ret i1 %state
}

define i1 @__ncc_kernel_frame_init(i64 %base, i64 %bytes) {
entry:
  %positive = icmp sgt i64 %bytes, 0
  %safe.bytes = select i1 %positive, i64 %bytes, i64 0
  %pages.raw = udiv i64 %safe.bytes, 4096
  %nonzero = icmp ugt i64 %pages.raw, 0
  %too.large = icmp ugt i64 %pages.raw, 4096
  %pages = select i1 %too.large, i64 4096, i64 %pages.raw
  store i64 %base, ptr @__ncc_kernel_frame_base, align 8
  store i64 %pages, ptr @__ncc_kernel_frame_count, align 8
  br i1 %nonzero, label %zero, label %fail
zero:
  %i = phi i64 [ 0, %entry ], [ %next, %zero.body ]
  %done = icmp uge i64 %i, 4096
  br i1 %done, label %ok, label %zero.body
zero.body:
  %ptr = getelementptr [4096 x i8], ptr @__ncc_kernel_frame_bitmap, i64 0, i64 %i
  store i8 0, ptr %ptr, align 1
  %next = add nuw i64 %i, 1
  br label %zero
ok:
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_frame_alloc(i64 %pages) {
entry:
  %count = load i64, ptr @__ncc_kernel_frame_count, align 8
  %positive = icmp ugt i64 %pages, 0
  %fits = icmp ule i64 %pages, %count
  %valid = and i1 %positive, %fits
  br i1 %valid, label %outer, label %fail
outer:
  %start = phi i64 [ 0, %entry ], [ %start.next, %advance ]
  %end = add i64 %start, %pages
  %room = icmp ule i64 %end, %count
  br i1 %room, label %verify, label %fail
verify:
  %off = phi i64 [ 0, %outer ], [ %off.next, %verify.free ]
  %checked = icmp uge i64 %off, %pages
  br i1 %checked, label %mark, label %verify.load
verify.load:
  %idx = add i64 %start, %off
  %ptr = getelementptr [4096 x i8], ptr @__ncc_kernel_frame_bitmap, i64 0, i64 %idx
  %used = load i8, ptr %ptr, align 1
  %free = icmp eq i8 %used, 0
  br i1 %free, label %verify.free, label %advance
verify.free:
  %off.next = add nuw i64 %off, 1
  br label %verify
advance:
  %start.next = add nuw i64 %start, 1
  br label %outer
mark:
  br label %mark.loop
mark.loop:
  %mark.off = phi i64 [ 0, %mark ], [ %mark.next, %mark.body ]
  %mark.done = icmp uge i64 %mark.off, %pages
  br i1 %mark.done, label %success, label %mark.body
mark.body:
  %mark.idx = add i64 %start, %mark.off
  %mark.ptr = getelementptr [4096 x i8], ptr @__ncc_kernel_frame_bitmap, i64 0, i64 %mark.idx
  store i8 1, ptr %mark.ptr, align 1
  %mark.next = add nuw i64 %mark.off, 1
  br label %mark.loop
success:
  %base = load i64, ptr @__ncc_kernel_frame_base, align 8
  %delta = mul i64 %start, 4096
  %address = add i64 %base, %delta
  ret i64 %address
fail:
  ret i64 0
}

define i1 @__ncc_kernel_frame_free(i64 %address, i64 %pages) {
entry:
  %base = load i64, ptr @__ncc_kernel_frame_base, align 8
  %count = load i64, ptr @__ncc_kernel_frame_count, align 8
  %after = icmp uge i64 %address, %base
  %positive = icmp ugt i64 %pages, 0
  %pre = and i1 %after, %positive
  br i1 %pre, label %range, label %fail
range:
  %delta = sub i64 %address, %base
  %remainder = urem i64 %delta, 4096
  %aligned = icmp eq i64 %remainder, 0
  %start = udiv i64 %delta, 4096
  %end = add i64 %start, %pages
  %fits = icmp ule i64 %end, %count
  %valid = and i1 %aligned, %fits
  br i1 %valid, label %verify, label %fail
verify:
  %off = phi i64 [ 0, %range ], [ %next, %verify.used ]
  %done = icmp uge i64 %off, %pages
  br i1 %done, label %clear, label %verify.load
verify.load:
  %idx = add i64 %start, %off
  %ptr = getelementptr [4096 x i8], ptr @__ncc_kernel_frame_bitmap, i64 0, i64 %idx
  %used = load i8, ptr %ptr, align 1
  %is.used = icmp ne i8 %used, 0
  br i1 %is.used, label %verify.used, label %fail
verify.used:
  %next = add nuw i64 %off, 1
  br label %verify
clear:
  br label %clear.loop
clear.loop:
  %clear.off = phi i64 [ 0, %clear ], [ %clear.next, %clear.body ]
  %clear.done = icmp uge i64 %clear.off, %pages
  br i1 %clear.done, label %ok, label %clear.body
clear.body:
  %clear.idx = add i64 %start, %clear.off
  %clear.ptr = getelementptr [4096 x i8], ptr @__ncc_kernel_frame_bitmap, i64 0, i64 %clear.idx
  store i8 0, ptr %clear.ptr, align 1
  %clear.next = add nuw i64 %clear.off, 1
  br label %clear.loop
ok:
  ret i1 true
fail:
  ret i1 false
}

define internal i32 @__ncc_kernel_find_page(i64 %virtual_page) {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %cont ]
  %done = icmp uge i32 %i, 256
  br i1 %done, label %miss, label %check
check:
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_page_used, i32 0, i32 %i
  %used = load i1, ptr %used.ptr, align 1
  %virt.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_page_virt, i32 0, i32 %i
  %virt = load i64, ptr %virt.ptr, align 8
  %same = icmp eq i64 %virt, %virtual_page
  %found = and i1 %used, %same
  br i1 %found, label %hit, label %cont
cont:
  %next = add nuw i32 %i, 1
  br label %loop
hit:
  ret i32 %i
miss:
  ret i32 -1
}

define i1 @__ncc_kernel_page_map(i64 %virtual, i64 %physical, i64 %flags) {
entry:
  %virt.page = and i64 %virtual, -4096
  %phys.page = and i64 %physical, -4096
  %existing = call i32 @__ncc_kernel_find_page(i64 %virt.page)
  %found = icmp sge i32 %existing, 0
  br i1 %found, label %store, label %scan
scan:
  %i = phi i32 [ 0, %entry ], [ %next, %occupied ]
  %done = icmp uge i32 %i, 256
  br i1 %done, label %fail, label %slot
slot:
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_page_used, i32 0, i32 %i
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %occupied, label %insert
occupied:
  %next = add nuw i32 %i, 1
  br label %scan
insert:
  store i1 true, ptr %used.ptr, align 1
  br label %store
store:
  %idx = phi i32 [ %existing, %entry ], [ %i, %insert ]
  %virt.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_page_virt, i32 0, i32 %idx
  %phys.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_page_phys, i32 0, i32 %idx
  %flags.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_page_flags, i32 0, i32 %idx
  store i64 %virt.page, ptr %virt.ptr, align 8
  store i64 %phys.page, ptr %phys.ptr, align 8
  store i64 %flags, ptr %flags.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_page_unmap(i64 %virtual) {
entry:
  %page = and i64 %virtual, -4096
  %idx = call i32 @__ncc_kernel_find_page(i64 %page)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %clear, label %fail
clear:
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_page_used, i32 0, i32 %idx
  store i1 false, ptr %used.ptr, align 1
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_page_protect(i64 %virtual, i64 %flags) {
entry:
  %page = and i64 %virtual, -4096
  %idx = call i32 @__ncc_kernel_find_page(i64 %page)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %set, label %fail
set:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_page_flags, i32 0, i32 %idx
  store i64 %flags, ptr %ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_page_translate(i64 %virtual) {
entry:
  %page = and i64 %virtual, -4096
  %offset = and i64 %virtual, 4095
  %idx = call i32 @__ncc_kernel_find_page(i64 %page)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_page_phys, i32 0, i32 %idx
  %physical = load i64, ptr %ptr, align 8
  %address = add i64 %physical, %offset
  ret i64 %address
fail:
  ret i64 0
}

define i1 @__ncc_kernel_irq_register(i64 %vector, i64 %handler) {
entry:
  %range = icmp ult i64 %vector, 256
  %valid.handler = icmp ne i64 %handler, 0
  %valid = and i1 %range, %valid.handler
  br i1 %valid, label %store, label %fail
store:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_irq_handler, i64 0, i64 %vector
  store i64 %handler, ptr %ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_irq_enable(i64 %vector) {
entry:
  %ok = icmp ult i64 %vector, 256
  br i1 %ok, label %set, label %fail
set:
  %ptr = getelementptr [256 x i1], ptr @__ncc_kernel_irq_enabled, i64 0, i64 %vector
  store i1 true, ptr %ptr, align 1
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_irq_disable(i64 %vector) {
entry:
  %ok = icmp ult i64 %vector, 256
  br i1 %ok, label %set, label %fail
set:
  %ptr = getelementptr [256 x i1], ptr @__ncc_kernel_irq_enabled, i64 0, i64 %vector
  store i1 false, ptr %ptr, align 1
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_irq_is_enabled(i64 %vector) {
entry:
  %ok = icmp ult i64 %vector, 256
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr [256 x i1], ptr @__ncc_kernel_irq_enabled, i64 0, i64 %vector
  %enabled = load i1, ptr %ptr, align 1
  ret i1 %enabled
fail:
  ret i1 false
}

define i1 @__ncc_kernel_irq_dispatch(i64 %vector, i64 %frame) {
entry:
  %range = icmp ult i64 %vector, 256
  br i1 %range, label %load, label %fail
load:
  %enabled.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_irq_enabled, i64 0, i64 %vector
  %enabled = load i1, ptr %enabled.ptr, align 1
  %handler.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_irq_handler, i64 0, i64 %vector
  %handler = load i64, ptr %handler.ptr, align 8
  %has.handler = icmp ne i64 %handler, 0
  %ready = and i1 %enabled, %has.handler
  br i1 %ready, label %call, label %fail
call:
  %fn = inttoptr i64 %handler to ptr
  call void %fn(i64 %vector, i64 %frame)
  %count.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_irq_dispatch_count, i64 0, i64 %vector
  %old = load i64, ptr %count.ptr, align 8
  %next = add i64 %old, 1
  store i64 %next, ptr %count.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_irq_eoi(i64 %vector) {
entry:
  %ok = icmp ult i64 %vector, 256
  br i1 %ok, label %set, label %fail
set:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_irq_eoi_count, i64 0, i64 %vector
  %old = load i64, ptr %ptr, align 8
  %next = add i64 %old, 1
  store i64 %next, ptr %ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_syscall_register(i64 %number, i64 %handler) {
entry:
  %range = icmp ult i64 %number, 256
  %has.handler = icmp ne i64 %handler, 0
  %ok = and i1 %range, %has.handler
  br i1 %ok, label %store, label %fail
store:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_syscall_handler, i64 0, i64 %number
  store i64 %handler, ptr %ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_syscall_available(i64 %number) {
entry:
  %range = icmp ult i64 %number, 256
  br i1 %range, label %load, label %fail
load:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_syscall_handler, i64 0, i64 %number
  %handler = load i64, ptr %ptr, align 8
  %available = icmp ne i64 %handler, 0
  ret i1 %available
fail:
  ret i1 false
}

define i64 @__ncc_kernel_syscall_invoke(i64 %number, i64 %a0, i64 %a1, i64 %a2) {
entry:
  %range = icmp ult i64 %number, 256
  br i1 %range, label %load, label %fail
load:
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_syscall_handler, i64 0, i64 %number
  %handler = load i64, ptr %ptr, align 8
  %available = icmp ne i64 %handler, 0
  br i1 %available, label %call, label %fail
call:
  %fn = inttoptr i64 %handler to ptr
  %result = call i64 %fn(i64 %a0, i64 %a1, i64 %a2)
  ret i64 %result
fail:
  ret i64 -38
}

define internal i32 @__ncc_kernel_driver_index(i64 %id) {
entry:
  %range = icmp ult i64 %id, 32
  %idx = trunc i64 %id to i32
  %out = select i1 %range, i32 %idx, i32 -1
  ret i32 %out
}

define i1 @__ncc_kernel_driver_register(i64 %id, i64 %base, i64 %irq) {
entry:
  %idx = call i32 @__ncc_kernel_driver_index(i64 %id)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %store, label %fail
store:
  %used.ptr = getelementptr [32 x i1], ptr @__ncc_kernel_driver_used, i32 0, i32 %idx
  %base.ptr = getelementptr [32 x i64], ptr @__ncc_kernel_driver_base, i32 0, i32 %idx
  %irq.ptr = getelementptr [32 x i64], ptr @__ncc_kernel_driver_irq, i32 0, i32 %idx
  %state.ptr = getelementptr [32 x i32], ptr @__ncc_kernel_driver_state, i32 0, i32 %idx
  store i1 true, ptr %used.ptr, align 1
  store i64 %base, ptr %base.ptr, align 8
  store i64 %irq, ptr %irq.ptr, align 8
  store i32 1, ptr %state.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_driver_start(i64 %id) {
entry:
  %idx = call i32 @__ncc_kernel_driver_index(i64 %id)
  %range = icmp sge i32 %idx, 0
  br i1 %range, label %check, label %fail
check:
  %used.ptr = getelementptr [32 x i1], ptr @__ncc_kernel_driver_used, i32 0, i32 %idx
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %set, label %fail
set:
  %state.ptr = getelementptr [32 x i32], ptr @__ncc_kernel_driver_state, i32 0, i32 %idx
  store i32 2, ptr %state.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_driver_stop(i64 %id) {
entry:
  %idx = call i32 @__ncc_kernel_driver_index(i64 %id)
  %range = icmp sge i32 %idx, 0
  br i1 %range, label %check, label %fail
check:
  %used.ptr = getelementptr [32 x i1], ptr @__ncc_kernel_driver_used, i32 0, i32 %idx
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %set, label %fail
set:
  %state.ptr = getelementptr [32 x i32], ptr @__ncc_kernel_driver_state, i32 0, i32 %idx
  store i32 3, ptr %state.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_driver_status(i64 %id) {
entry:
  %idx = call i32 @__ncc_kernel_driver_index(i64 %id)
  %ok = icmp sge i32 %idx, 0
  br i1 %ok, label %read, label %fail
read:
  %used.ptr = getelementptr [32 x i1], ptr @__ncc_kernel_driver_used, i32 0, i32 %idx
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %state, label %fail
state:
  %state.ptr = getelementptr [32 x i32], ptr @__ncc_kernel_driver_state, i32 0, i32 %idx
  %value = load i32, ptr %state.ptr, align 4
  %wide = zext i32 %value to i64
  ret i64 %wide
fail:
  ret i64 -1
}

define i64 @__ncc_kernel_driver_read(i64 %id, i64 %offset) {
entry:
  %status = call i64 @__ncc_kernel_driver_status(i64 %id)
  %active = icmp eq i64 %status, 2
  br i1 %active, label %read, label %fail
read:
  %idx = trunc i64 %id to i32
  %base.ptr = getelementptr [32 x i64], ptr @__ncc_kernel_driver_base, i32 0, i32 %idx
  %base = load i64, ptr %base.ptr, align 8
  %address = add i64 %base, %offset
  %ptr = inttoptr i64 %address to ptr
  %value = load volatile i64, ptr %ptr, align 1
  ret i64 %value
fail:
  ret i64 -1
}

define i1 @__ncc_kernel_driver_write(i64 %id, i64 %offset, i64 %value) {
entry:
  %status = call i64 @__ncc_kernel_driver_status(i64 %id)
  %active = icmp eq i64 %status, 2
  br i1 %active, label %write, label %fail
write:
  %idx = trunc i64 %id to i32
  %base.ptr = getelementptr [32 x i64], ptr @__ncc_kernel_driver_base, i32 0, i32 %idx
  %base = load i64, ptr %base.ptr, align 8
  %address = add i64 %base, %offset
  %ptr = inttoptr i64 %address to ptr
  store volatile i64 %value, ptr %ptr, align 1
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_copy_bytes(i64 %destination, i64 %source, i64 %bytes) {
entry:
  %dst.ok = icmp ne i64 %destination, 0
  %src.ok = icmp ne i64 %source, 0
  %ptrs.ok = and i1 %dst.ok, %src.ok
  br i1 %ptrs.ok, label %loop, label %fail
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %i, %bytes
  br i1 %done, label %ok, label %body
body:
  %src.address = add i64 %source, %i
  %dst.address = add i64 %destination, %i
  %src.ptr = inttoptr i64 %src.address to ptr
  %dst.ptr = inttoptr i64 %dst.address to ptr
  %value = load volatile i8, ptr %src.ptr, align 1
  store volatile i8 %value, ptr %dst.ptr, align 1
  %next = add nuw i64 %i, 1
  br label %loop
ok:
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_copy_from_user(i64 %kernel_dst, i64 %user_src, i64 %bytes) {
entry:
  %ok = call i1 @__ncc_kernel_copy_bytes(i64 %kernel_dst, i64 %user_src, i64 %bytes)
  ret i1 %ok
}

define i1 @__ncc_kernel_copy_to_user(i64 %user_dst, i64 %kernel_src, i64 %bytes) {
entry:
  %ok = call i1 @__ncc_kernel_copy_bytes(i64 %user_dst, i64 %kernel_src, i64 %bytes)
  ret i1 %ok
}

define ptr @memcpy(ptr %destination, ptr %source, i64 %bytes) noinline optnone {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %i, %bytes
  br i1 %done, label %finish, label %body
body:
  %src.ptr = getelementptr i8, ptr %source, i64 %i
  %dst.ptr = getelementptr i8, ptr %destination, i64 %i
  %byte = load i8, ptr %src.ptr, align 1
  store i8 %byte, ptr %dst.ptr, align 1
  %next = add nuw i64 %i, 1
  br label %loop
finish:
  ret ptr %destination
}

define ptr @memset(ptr %destination, i32 %value, i64 %bytes) noinline optnone {
entry:
  %byte = trunc i32 %value to i8
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %i, %bytes
  br i1 %done, label %finish, label %body
body:
  %ptr = getelementptr i8, ptr %destination, i64 %i
  store i8 %byte, ptr %ptr, align 1
  %next = add nuw i64 %i, 1
  br label %loop
finish:
  ret ptr %destination
}

define ptr @memmove(ptr %destination, ptr %source, i64 %bytes) noinline optnone {
entry:
  %dst.address = ptrtoint ptr %destination to i64
  %src.address = ptrtoint ptr %source to i64
  %same = icmp eq ptr %destination, %source
  %empty = icmp eq i64 %bytes, 0
  %skip = or i1 %same, %empty
  br i1 %skip, label %finish, label %direction
direction:
  %src.end = add i64 %src.address, %bytes
  %before = icmp ult i64 %dst.address, %src.address
  %after = icmp uge i64 %dst.address, %src.end
  %forward = or i1 %before, %after
  br i1 %forward, label %forward.loop, label %backward.loop
forward.loop:
  %forward.i = phi i64 [ 0, %direction ], [ %forward.next, %forward.body ]
  %forward.done = icmp uge i64 %forward.i, %bytes
  br i1 %forward.done, label %finish, label %forward.body
forward.body:
  %forward.src = getelementptr i8, ptr %source, i64 %forward.i
  %forward.dst = getelementptr i8, ptr %destination, i64 %forward.i
  %forward.value = load i8, ptr %forward.src, align 1
  store i8 %forward.value, ptr %forward.dst, align 1
  %forward.next = add nuw i64 %forward.i, 1
  br label %forward.loop
backward.loop:
  %backward.remaining = phi i64 [ %bytes, %direction ], [ %backward.index, %backward.body ]
  %backward.done = icmp eq i64 %backward.remaining, 0
  br i1 %backward.done, label %finish, label %backward.body
backward.body:
  %backward.index = sub i64 %backward.remaining, 1
  %backward.src = getelementptr i8, ptr %source, i64 %backward.index
  %backward.dst = getelementptr i8, ptr %destination, i64 %backward.index
  %backward.value = load i8, ptr %backward.src, align 1
  store i8 %backward.value, ptr %backward.dst, align 1
  br label %backward.loop
finish:
  ret ptr %destination
}

define i32 @memcmp(ptr %left, ptr %right, i64 %bytes) noinline optnone {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %equal ]
  %done = icmp uge i64 %i, %bytes
  br i1 %done, label %same, label %compare
compare:
  %left.ptr = getelementptr i8, ptr %left, i64 %i
  %right.ptr = getelementptr i8, ptr %right, i64 %i
  %left.byte = load i8, ptr %left.ptr, align 1
  %right.byte = load i8, ptr %right.ptr, align 1
  %matches = icmp eq i8 %left.byte, %right.byte
  br i1 %matches, label %equal, label %different
equal:
  %next = add nuw i64 %i, 1
  br label %loop
different:
  %left.wide = zext i8 %left.byte to i32
  %right.wide = zext i8 %right.byte to i32
  %delta = sub i32 %left.wide, %right.wide
  ret i32 %delta
same:
  ret i32 0
}

define i64 @__ncc_kernel_boot_info() {
entry:
  %address = load i64, ptr @__ncc_kernel_boot_info_slot, align 8
  ret i64 %address
}
"#
    );
    llvm.push_str(&render_kernel_atomic_runtime(target_triple));
    llvm.push_str(render_kernel_mmio_runtime());
    llvm.push_str(render_kernel_x86_interrupt_platform_runtime(target_triple));
    llvm.push_str(render_kernel_virtualization_runtime(target_triple));
    llvm.push_str(&render_kernel_hypervisor_iommu_runtime(target_triple));
    llvm.push_str(&render_kernel_device_runtime(target_triple));
    llvm.push_str(
        &crate::kernel_firmware_multiboot2_llvm::render_kernel_firmware_multiboot2_runtime(
            target_triple,
        ),
    );
    llvm.push_str(
        &crate::kernel_firmware_smp_llvm::render_kernel_firmware_smp_runtime(target_triple),
    );
    llvm.push_str(render_kernel_arm_eabi_runtime(target_triple));
    llvm.push_str(render_kernel_thumb_atomic64_runtime(target_triple));
    llvm.push_str(render_kernel_arch_runtime(target_triple));
    // LLVM uses ';' for comments, so expand legacy compact trap bodies before
    // the combined module is handed to Clang/LLVM.
    let llvm = llvm.replace(
        "call void @llvm.trap(); unreachable }",
        "call void @llvm.trap()\n  unreachable\n}",
    );
    debug_assert!(!llvm.contains("call void @llvm.trap(); unreachable }"));
    llvm
}

fn render_kernel_arm_eabi_runtime(target_triple: &str) -> &'static str {
    let triple = target_triple.to_ascii_lowercase();
    if !(triple.starts_with("armv7") || triple.starts_with("thumbv7")) {
        return "";
    }

    // LLVM legalizes fills and 32-bit division to these AAPCS32 entry points.
    // Keeping them here prevents an implicit compiler-rt or libc dependency.
    r#"
define void @__aeabi_memclr(ptr %destination, i32 %bytes) nounwind noinline {
entry:
  %wide = zext i32 %bytes to i64
  %ignored = call ptr @memset(ptr %destination, i32 0, i64 %wide)
  ret void
}

@__aeabi_memclr4 = alias void (ptr, i32), ptr @__aeabi_memclr
@__aeabi_memclr8 = alias void (ptr, i32), ptr @__aeabi_memclr

define void @__aeabi_memset(ptr %destination, i32 %bytes, i32 %value) nounwind noinline {
entry:
  %wide = zext i32 %bytes to i64
  %ignored = call ptr @memset(ptr %destination, i32 %value, i64 %wide)
  ret void
}

@__aeabi_memset4 = alias void (ptr, i32, i32), ptr @__aeabi_memset
@__aeabi_memset8 = alias void (ptr, i32, i32), ptr @__aeabi_memset

define void @__aeabi_memcpy(ptr %destination, ptr %source, i32 %bytes) nounwind noinline {
entry:
  %wide = zext i32 %bytes to i64
  %ignored = call ptr @memcpy(ptr %destination, ptr %source, i64 %wide)
  ret void
}

@__aeabi_memcpy4 = alias void (ptr, ptr, i32), ptr @__aeabi_memcpy
@__aeabi_memcpy8 = alias void (ptr, ptr, i32), ptr @__aeabi_memcpy

define void @__aeabi_memmove(ptr %destination, ptr %source, i32 %bytes) nounwind noinline {
entry:
  %wide = zext i32 %bytes to i64
  %ignored = call ptr @memmove(ptr %destination, ptr %source, i64 %wide)
  ret void
}

@__aeabi_memmove4 = alias void (ptr, ptr, i32), ptr @__aeabi_memmove
@__aeabi_memmove8 = alias void (ptr, ptr, i32), ptr @__aeabi_memmove

define { i32, i32 } @__aeabi_uidivmod(i32 %numerator, i32 %denominator) nounwind noinline {
entry:
  %division.by.zero = icmp eq i32 %denominator, 0
  br i1 %division.by.zero, label %zero, label %divide
zero:
  %zero.quotient = insertvalue { i32, i32 } poison, i32 0, 0
  %zero.result = insertvalue { i32, i32 } %zero.quotient, i32 %numerator, 1
  ret { i32, i32 } %zero.result
divide:
  br label %loop
loop:
  %bit.index = phi i32 [ 31, %divide ], [ %next.index, %advance ]
  %quotient = phi i32 [ 0, %divide ], [ %next.quotient, %advance ]
  %remainder = phi i32 [ 0, %divide ], [ %next.remainder, %advance ]
  %shifted.numerator = lshr i32 %numerator, %bit.index
  %incoming.bit = and i32 %shifted.numerator, 1
  %shifted.remainder = shl i32 %remainder, 1
  %candidate = or i32 %shifted.remainder, %incoming.bit
  %fits = icmp uge i32 %candidate, %denominator
  %reduced = sub i32 %candidate, %denominator
  %next.remainder = select i1 %fits, i32 %reduced, i32 %candidate
  %quotient.bit = shl i32 1, %bit.index
  %with.bit = or i32 %quotient, %quotient.bit
  %next.quotient = select i1 %fits, i32 %with.bit, i32 %quotient
  %last = icmp eq i32 %bit.index, 0
  br i1 %last, label %done, label %advance
advance:
  %next.index = sub nuw i32 %bit.index, 1
  br label %loop
done:
  %partial = insertvalue { i32, i32 } poison, i32 %next.quotient, 0
  %result = insertvalue { i32, i32 } %partial, i32 %next.remainder, 1
  ret { i32, i32 } %result
}

define i32 @__aeabi_uidiv(i32 %numerator, i32 %denominator) nounwind noinline {
entry:
  %pair = call { i32, i32 } @__aeabi_uidivmod(i32 %numerator, i32 %denominator)
  %quotient = extractvalue { i32, i32 } %pair, 0
  ret i32 %quotient
}

define i32 @__aeabi_idiv0(i32 %value) nounwind noinline {
entry:
  ret i32 %value
}

define void @__aeabi_unwind_cpp_pr0() nounwind { ret void }
define void @__aeabi_unwind_cpp_pr1() nounwind { ret void }
define void @__aeabi_unwind_cpp_pr2() nounwind { ret void }
"#
}

fn render_kernel_thumb_atomic64_runtime(target_triple: &str) -> &'static str {
    let triple = target_triple.to_ascii_lowercase();
    if !(triple.starts_with("thumbv7") || triple.starts_with("armv7em")) {
        return "";
    }

    // ARMv7-M has 32-bit exclusive accesses but no architectural 64-bit
    // compare/exchange. LLVM names the required out-of-line ABI entry points
    // __atomic_*_8; serialize their two-word accesses with a native i32 lock.
    r#"
@__ncc_arm_atomic64_lock = internal global i32 0, align 4

define internal void @__ncc_arm_atomic64_acquire() nounwind noinline {
entry:
  br label %spin
spin:
  %attempt = cmpxchg volatile ptr @__ncc_arm_atomic64_lock, i32 0, i32 1 seq_cst seq_cst
  %won = extractvalue { i32, i1 } %attempt, 1
  br i1 %won, label %locked, label %spin
locked:
  ret void
}

define internal void @__ncc_arm_atomic64_release() nounwind noinline {
entry:
  store atomic volatile i32 0, ptr @__ncc_arm_atomic64_lock release, align 4
  ret void
}

define internal i64 @__ncc_arm_load_pair(ptr %address) nounwind noinline optnone {
entry:
  %low = load volatile i32, ptr %address, align 4
  %high.address = getelementptr i8, ptr %address, i32 4
  %high = load volatile i32, ptr %high.address, align 4
  %low.wide = zext i32 %low to i64
  %high.wide = zext i32 %high to i64
  %shifted = shl i64 %high.wide, 32
  %value = or i64 %shifted, %low.wide
  ret i64 %value
}

define internal void @__ncc_arm_store_pair(ptr %address, i64 %value) nounwind noinline optnone {
entry:
  %low = trunc i64 %value to i32
  %shifted = lshr i64 %value, 32
  %high = trunc i64 %shifted to i32
  store volatile i32 %low, ptr %address, align 4
  %high.address = getelementptr i8, ptr %address, i32 4
  store volatile i32 %high, ptr %high.address, align 4
  ret void
}

define i64 @__atomic_load_8(ptr %address, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %value = call i64 @__ncc_arm_load_pair(ptr %address)
  call void @__ncc_arm_atomic64_release()
  ret i64 %value
}

define void @__atomic_store_8(ptr %address, i64 %value, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  call void @__ncc_arm_store_pair(ptr %address, i64 %value)
  call void @__ncc_arm_atomic64_release()
  ret void
}

define i64 @__atomic_exchange_8(ptr %address, i64 %desired, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  call void @__ncc_arm_store_pair(ptr %address, i64 %desired)
  call void @__ncc_arm_atomic64_release()
  ret i64 %old
}

define i1 @__atomic_compare_exchange_8(ptr %address, ptr %expected.address, i64 %desired, i32 %success.order, i32 %failure.order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  %expected = call i64 @__ncc_arm_load_pair(ptr %expected.address)
  %matches = icmp eq i64 %old, %expected
  br i1 %matches, label %store.desired, label %store.observed
store.desired:
  call void @__ncc_arm_store_pair(ptr %address, i64 %desired)
  br label %finish
store.observed:
  call void @__ncc_arm_store_pair(ptr %expected.address, i64 %old)
  br label %finish
finish:
  call void @__ncc_arm_atomic64_release()
  ret i1 %matches
}

define i64 @__atomic_fetch_add_8(ptr %address, i64 %operand, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  %next = add i64 %old, %operand
  call void @__ncc_arm_store_pair(ptr %address, i64 %next)
  call void @__ncc_arm_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_sub_8(ptr %address, i64 %operand, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  %next = sub i64 %old, %operand
  call void @__ncc_arm_store_pair(ptr %address, i64 %next)
  call void @__ncc_arm_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_and_8(ptr %address, i64 %operand, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  %next = and i64 %old, %operand
  call void @__ncc_arm_store_pair(ptr %address, i64 %next)
  call void @__ncc_arm_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_or_8(ptr %address, i64 %operand, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  %next = or i64 %old, %operand
  call void @__ncc_arm_store_pair(ptr %address, i64 %next)
  call void @__ncc_arm_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_xor_8(ptr %address, i64 %operand, i32 %order) nounwind noinline {
entry:
  call void @__ncc_arm_atomic64_acquire()
  %old = call i64 @__ncc_arm_load_pair(ptr %address)
  %next = xor i64 %old, %operand
  call void @__ncc_arm_store_pair(ptr %address, i64 %next)
  call void @__ncc_arm_atomic64_release()
  ret i64 %old
}
"#
}

fn render_kernel_atomic_runtime(target_triple: &str) -> String {
    if target_triple.to_ascii_lowercase().starts_with("wasm32") {
        return r#"
define i64 @__ncc_kernel_atomic32_load(i64 %address) { ret i64 0 }
define i1 @__ncc_kernel_atomic32_store(i64 %address, i64 %value) { ret i1 false }
define i64 @__ncc_kernel_atomic32_exchange(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic32_compare_exchange(i64 %address, i64 %expected, i64 %desired) { ret i64 0 }
define i64 @__ncc_kernel_atomic32_fetch_add(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic32_fetch_sub(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic32_fetch_and(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic32_fetch_or(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic32_fetch_xor(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_load(i64 %address) { ret i64 0 }
define i1 @__ncc_kernel_atomic64_store(i64 %address, i64 %value) { ret i1 false }
define i64 @__ncc_kernel_atomic64_exchange(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_compare_exchange(i64 %address, i64 %expected, i64 %desired) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_fetch_add(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_fetch_sub(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_fetch_and(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_fetch_or(i64 %address, i64 %value) { ret i64 0 }
define i64 @__ncc_kernel_atomic64_fetch_xor(i64 %address, i64 %value) { ret i64 0 }
define i1 @__ncc_kernel_atomic_fence_acquire() { ret i1 false }
define i1 @__ncc_kernel_atomic_fence_release() { ret i1 false }
define i1 @__ncc_kernel_atomic_fence_seq_cst() { ret i1 false }
define i1 @__ncc_kernel_spin_try_lock(i64 %address) { ret i1 false }
define i1 @__ncc_kernel_spin_lock(i64 %address) { ret i1 false }
define i1 @__ncc_kernel_spin_unlock(i64 %address) { ret i1 false }
"#
        .to_string();
    }
    let native = r#"
define i64 @__ncc_kernel_atomic32_load(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %value = load atomic i32, ptr %ptr acquire, align 4
  %wide = sext i32 %value to i64
  ret i64 %wide
}

define i1 @__ncc_kernel_atomic32_store(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  store atomic i32 %narrow, ptr %ptr release, align 4
  ret i1 true
}

define i64 @__ncc_kernel_atomic32_exchange(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  %old = atomicrmw xchg ptr %ptr, i32 %narrow acq_rel, align 4
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic32_compare_exchange(i64 %address, i64 %expected, i64 %desired) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %expected.narrow = trunc i64 %expected to i32
  %desired.narrow = trunc i64 %desired to i32
  %pair = cmpxchg ptr %ptr, i32 %expected.narrow, i32 %desired.narrow acq_rel acquire, align 4
  %old = extractvalue { i32, i1 } %pair, 0
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic32_fetch_add(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  %old = atomicrmw add ptr %ptr, i32 %narrow acq_rel, align 4
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic32_fetch_sub(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  %old = atomicrmw sub ptr %ptr, i32 %narrow acq_rel, align 4
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic32_fetch_and(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  %old = atomicrmw and ptr %ptr, i32 %narrow acq_rel, align 4
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic32_fetch_or(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  %old = atomicrmw or ptr %ptr, i32 %narrow acq_rel, align 4
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic32_fetch_xor(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  %old = atomicrmw xor ptr %ptr, i32 %narrow acq_rel, align 4
  %wide = sext i32 %old to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_atomic64_load(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %value = load atomic i64, ptr %ptr acquire, align 8
  ret i64 %value
}

define i1 @__ncc_kernel_atomic64_store(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  store atomic i64 %value, ptr %ptr release, align 8
  ret i1 true
}

define i64 @__ncc_kernel_atomic64_exchange(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %old = atomicrmw xchg ptr %ptr, i64 %value acq_rel, align 8
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_compare_exchange(i64 %address, i64 %expected, i64 %desired) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %pair = cmpxchg ptr %ptr, i64 %expected, i64 %desired acq_rel acquire, align 8
  %old = extractvalue { i64, i1 } %pair, 0
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_add(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %old = atomicrmw add ptr %ptr, i64 %value acq_rel, align 8
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_sub(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %old = atomicrmw sub ptr %ptr, i64 %value acq_rel, align 8
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_and(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %old = atomicrmw and ptr %ptr, i64 %value acq_rel, align 8
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_or(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %old = atomicrmw or ptr %ptr, i64 %value acq_rel, align 8
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_xor(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %old = atomicrmw xor ptr %ptr, i64 %value acq_rel, align 8
  ret i64 %old
}

define i1 @__ncc_kernel_atomic_fence_acquire() {
entry:
  fence acquire
  ret i1 true
}

define i1 @__ncc_kernel_atomic_fence_release() {
entry:
  fence release
  ret i1 true
}

define i1 @__ncc_kernel_atomic_fence_seq_cst() {
entry:
  fence seq_cst
  ret i1 true
}

define i1 @__ncc_kernel_spin_try_lock(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %pair = cmpxchg ptr %ptr, i32 0, i32 1 acquire monotonic, align 4
  %locked = extractvalue { i32, i1 } %pair, 1
  ret i1 %locked
}

define i1 @__ncc_kernel_spin_lock(i64 %address) {
entry:
  br label %attempt
attempt:
  %locked = call i1 @__ncc_kernel_spin_try_lock(i64 %address)
  br i1 %locked, label %acquired, label %wait
wait:
  %relaxed = call i1 @__ncc_kernel_arch_cpu_relax()
  br label %attempt
acquired:
  ret i1 true
}

define i1 @__ncc_kernel_spin_unlock(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  store atomic i32 0, ptr %ptr release, align 4
  ret i1 true
}
"#;
    if !target_triple.to_ascii_lowercase().starts_with("riscv32") {
        return native.to_string();
    }

    let atomic64_start = native
        .find("define i64 @__ncc_kernel_atomic64_load")
        .expect("native atomic runtime must contain atomic64 load");
    let atomic64_end = native
        .find("define i1 @__ncc_kernel_atomic_fence_acquire")
        .expect("native atomic runtime must contain fence section");
    let mut rv32 = String::with_capacity(native.len() + 3_000);
    rv32.push_str(&native[..atomic64_start]);
    rv32.push_str(RISCV32_SOFTWARE_ATOMIC64_RUNTIME);
    rv32.push_str(&native[atomic64_end..]);
    rv32
}

const RISCV32_SOFTWARE_ATOMIC64_RUNTIME: &str = r#"
@__ncc_kernel_rv32_atomic64_lock = internal global i32 0, align 4

define internal void @__ncc_kernel_rv32_atomic64_acquire() {
entry:
  br label %attempt
attempt:
  %pair = cmpxchg ptr @__ncc_kernel_rv32_atomic64_lock, i32 0, i32 1 acquire monotonic, align 4
  %locked = extractvalue { i32, i1 } %pair, 1
  br i1 %locked, label %acquired, label %wait
wait:
  %relaxed = call i1 @__ncc_kernel_arch_cpu_relax()
  br label %attempt
acquired:
  ret void
}

define internal void @__ncc_kernel_rv32_atomic64_release() {
entry:
  store atomic i32 0, ptr @__ncc_kernel_rv32_atomic64_lock release, align 4
  ret void
}

define i64 @__ncc_kernel_atomic64_load(i64 %address) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %value = load volatile i64, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %value
}

define i1 @__ncc_kernel_atomic64_store(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  store volatile i64 %value, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i1 true
}

define i64 @__ncc_kernel_atomic64_exchange(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  store volatile i64 %value, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_compare_exchange(i64 %address, i64 %expected, i64 %desired) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  %equal = icmp eq i64 %old, %expected
  br i1 %equal, label %replace, label %complete
replace:
  store volatile i64 %desired, ptr %ptr, align 8
  br label %complete
complete:
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_add(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  %next = add i64 %old, %value
  store volatile i64 %next, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_sub(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  %next = sub i64 %old, %value
  store volatile i64 %next, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_and(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  %next = and i64 %old, %value
  store volatile i64 %next, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_or(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  %next = or i64 %old, %value
  store volatile i64 %next, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__ncc_kernel_atomic64_fetch_xor(i64 %address, i64 %value) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %ptr = inttoptr i64 %address to ptr
  %old = load volatile i64, ptr %ptr, align 8
  %next = xor i64 %old, %value
  store volatile i64 %next, ptr %ptr, align 8
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_load_8(ptr %address, i32 %order) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %value = load volatile i64, ptr %address, align 1
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %value
}

define void @__atomic_store_8(ptr %address, i64 %value, i32 %order) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  store volatile i64 %value, ptr %address, align 1
  call void @__ncc_kernel_rv32_atomic64_release()
  ret void
}

define i64 @__atomic_exchange_8(ptr %address, i64 %value, i32 %order) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %old = load volatile i64, ptr %address, align 1
  store volatile i64 %value, ptr %address, align 1
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_add_8(ptr %address, i64 %value, i32 %order) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %old = load volatile i64, ptr %address, align 1
  %next = add i64 %old, %value
  store volatile i64 %next, ptr %address, align 1
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_and_8(ptr %address, i64 %value, i32 %order) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %old = load volatile i64, ptr %address, align 1
  %next = and i64 %old, %value
  store volatile i64 %next, ptr %address, align 1
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

define i64 @__atomic_fetch_or_8(ptr %address, i64 %value, i32 %order) {
entry:
  call void @__ncc_kernel_rv32_atomic64_acquire()
  %old = load volatile i64, ptr %address, align 1
  %next = or i64 %old, %value
  store volatile i64 %next, ptr %address, align 1
  call void @__ncc_kernel_rv32_atomic64_release()
  ret i64 %old
}

"#;

fn render_kernel_mmio_runtime() -> &'static str {
    r#"
define i64 @__ncc_kernel_mmio_read8(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %value = load volatile i8, ptr %ptr, align 1
  %wide = zext i8 %value to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_mmio_read16(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %value = load volatile i16, ptr %ptr, align 2
  %wide = zext i16 %value to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_mmio_read32(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %value = load volatile i32, ptr %ptr, align 4
  %wide = zext i32 %value to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_mmio_read64(i64 %address) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %value = load volatile i64, ptr %ptr, align 8
  ret i64 %value
}

define i1 @__ncc_kernel_mmio_read128(i64 %address, i64 %destination_address) {
entry:
  %mmio = inttoptr i64 %address to ptr
  %destination = inttoptr i64 %destination_address to ptr
  %value = load volatile i128, ptr %mmio, align 1
  store i128 %value, ptr %destination, align 1
  ret i1 true
}

define i1 @__ncc_kernel_mmio_read256(i64 %address, i64 %destination_address) {
entry:
  %mmio = inttoptr i64 %address to ptr
  %destination = inttoptr i64 %destination_address to ptr
  %value = load volatile i256, ptr %mmio, align 1
  store i256 %value, ptr %destination, align 1
  ret i1 true
}

define i1 @__ncc_kernel_mmio_read512(i64 %address, i64 %destination_address) {
entry:
  %mmio = inttoptr i64 %address to ptr
  %destination = inttoptr i64 %destination_address to ptr
  %value = load volatile i512, ptr %mmio, align 1
  store i512 %value, ptr %destination, align 1
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write8(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i8
  store volatile i8 %narrow, ptr %ptr, align 1
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write16(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i16
  store volatile i16 %narrow, ptr %ptr, align 2
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write32(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  %narrow = trunc i64 %value to i32
  store volatile i32 %narrow, ptr %ptr, align 4
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write64(i64 %address, i64 %value) {
entry:
  %ptr = inttoptr i64 %address to ptr
  store volatile i64 %value, ptr %ptr, align 8
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write128(i64 %address, i64 %source_address) {
entry:
  %mmio = inttoptr i64 %address to ptr
  %source = inttoptr i64 %source_address to ptr
  %value = load i128, ptr %source, align 1
  store volatile i128 %value, ptr %mmio, align 1
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write256(i64 %address, i64 %source_address) {
entry:
  %mmio = inttoptr i64 %address to ptr
  %source = inttoptr i64 %source_address to ptr
  %value = load i256, ptr %source, align 1
  store volatile i256 %value, ptr %mmio, align 1
  ret i1 true
}

define i1 @__ncc_kernel_mmio_write512(i64 %address, i64 %source_address) {
entry:
  %mmio = inttoptr i64 %address to ptr
  %source = inttoptr i64 %source_address to ptr
  %value = load i512, ptr %source, align 1
  store volatile i512 %value, ptr %mmio, align 1
  ret i1 true
}
"#
}

fn render_kernel_x86_interrupt_platform_runtime(target_triple: &str) -> &'static str {
    if !target_triple.to_ascii_lowercase().starts_with("x86_64") {
        return r#"
define i1 @__ncc_kernel_apic_eoi(i64 %base) { ret i1 false }
define i1 @__ncc_kernel_apic_send_ipi(i64 %base, i64 %destination, i64 %command) { ret i1 false }
define i64 @__ncc_kernel_ioapic_read(i64 %base, i64 %register) { ret i64 0 }
define i1 @__ncc_kernel_ioapic_write(i64 %base, i64 %register, i64 %value) { ret i1 false }
define i1 @__ncc_kernel_tsc_deadline_write(i64 %value) { ret i1 false }
"#;
    }
    r#"
define i1 @__ncc_kernel_apic_eoi(i64 %base) {
entry:
  %register = add i64 %base, 176
  %written = call i1 @__ncc_kernel_mmio_write32(i64 %register, i64 0)
  ret i1 %written
}

define i1 @__ncc_kernel_apic_send_ipi(i64 %base, i64 %destination, i64 %command) {
entry:
  %high.address = add i64 %base, 784
  %low.address = add i64 %base, 768
  %destination.low = and i64 %destination, 255
  %destination.high = shl i64 %destination.low, 24
  %high = call i1 @__ncc_kernel_mmio_write32(i64 %high.address, i64 %destination.high)
  %low = call i1 @__ncc_kernel_mmio_write32(i64 %low.address, i64 %command)
  %complete = and i1 %high, %low
  ret i1 %complete
}

define i64 @__ncc_kernel_ioapic_read(i64 %base, i64 %register) {
entry:
  %select = call i1 @__ncc_kernel_mmio_write32(i64 %base, i64 %register)
  %window = add i64 %base, 16
  %value = call i64 @__ncc_kernel_mmio_read32(i64 %window)
  ret i64 %value
}

define i1 @__ncc_kernel_ioapic_write(i64 %base, i64 %register, i64 %value) {
entry:
  %select = call i1 @__ncc_kernel_mmio_write32(i64 %base, i64 %register)
  %window = add i64 %base, 16
  %written = call i1 @__ncc_kernel_mmio_write32(i64 %window, i64 %value)
  %complete = and i1 %select, %written
  ret i1 %complete
}

define i1 @__ncc_kernel_tsc_deadline_write(i64 %value) {
entry:
  %written = call i1 @__ncc_kernel_cpu_write_msr(i64 1760, i64 %value)
  ret i1 %written
}
"#
}

fn render_kernel_virtualization_runtime(target_triple: &str) -> &'static str {
    if target_triple.to_ascii_lowercase().starts_with("x86_64") {
        return r#"
define i1 @__ncc_kernel_vmx_on(i64 %physical_slot) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0xf3,0x0f,0xc7,0x30; pushfq; popq $0", "=r,{rax},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %physical_slot)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vmx_off() {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x0f,0x01,0xc4; pushfq; popq $0", "=r,~{dirflag},~{fpsr},~{flags},~{memory}"()
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vmcs_clear(i64 %physical_slot) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x66,0x0f,0xc7,0x30; pushfq; popq $0", "=r,{rax},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %physical_slot)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vmcs_load(i64 %physical_slot) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x0f,0xc7,0x30; pushfq; popq $0", "=r,{rax},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %physical_slot)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vmcs_store(i64 %destination) {
entry:
  call void asm sideeffect ".byte 0x0f,0xc7,0x38", "{rax},~{memory}"(i64 %destination)
  ret i1 true
}

define i1 @__ncc_kernel_vmcs_read(i64 %field, i64 %destination) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x0f,0x78,0x0a; pushfq; popq $0", "=r,{rcx},{rdx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %field, i64 %destination)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vmcs_write(i64 %field, i64 %value) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x0f,0x79,0xd1; pushfq; popq $0", "=r,{rcx},{rdx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %field, i64 %value)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_invept(i64 %kind, i64 %descriptor) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x66,0x0f,0x38,0x80,0x0a; pushfq; popq $0", "=r,{rcx},{rdx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %kind, i64 %descriptor)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_invvpid(i64 %kind, i64 %descriptor) {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x66,0x0f,0x38,0x81,0x0a; pushfq; popq $0", "=r,{rcx},{rdx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %kind, i64 %descriptor)
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vm_call() {
entry:
  call void asm sideeffect ".byte 0x0f,0x01,0xc1", "~{dirflag},~{fpsr},~{flags},~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_vm_launch() {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x0f,0x01,0xc2; pushfq; popq $0", "=r,~{dirflag},~{fpsr},~{flags},~{memory}"()
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_vm_resume() {
entry:
  %rflags = call i64 asm sideeffect ".byte 0x0f,0x01,0xc3; pushfq; popq $0", "=r,~{dirflag},~{fpsr},~{flags},~{memory}"()
  %status.bits = and i64 %rflags, 65
  %status = icmp eq i64 %status.bits, 0
  ret i1 %status
}

define i1 @__ncc_kernel_svm_run(i64 %vmcb_physical) {
entry:
  %rax.after = call i64 asm sideeffect ".byte 0x0f,0x01,0xd8", "={rax},0,~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %vmcb_physical)
  ret i1 true
}

define i1 @__ncc_kernel_svm_call() {
entry:
  call void asm sideeffect ".byte 0x0f,0x01,0xd9", "~{dirflag},~{fpsr},~{flags},~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_svm_load(i64 %vmcb_physical) {
entry:
  %rax.after = call i64 asm sideeffect ".byte 0x0f,0x01,0xda", "={rax},0,~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %vmcb_physical)
  ret i1 true
}

define i1 @__ncc_kernel_svm_save(i64 %vmcb_physical) {
entry:
  %rax.after = call i64 asm sideeffect ".byte 0x0f,0x01,0xdb", "={rax},0,~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %vmcb_physical)
  ret i1 true
}

define i64 @__ncc_kernel_guest_phys_read64(i64 %address) {
entry:
  %pointer = inttoptr i64 %address to ptr
  %value = load volatile i64, ptr %pointer, align 1
  ret i64 %value
}

define i1 @__ncc_kernel_guest_phys_write64(i64 %address, i64 %value) {
entry:
  %pointer = inttoptr i64 %address to ptr
  store volatile i64 %value, ptr %pointer, align 1
  ret i1 true
}

define i1 @__ncc_kernel_guest_phys_zero_page(i64 %address) {
entry:
  %aligned.bits = and i64 %address, 4095
  %aligned = icmp eq i64 %aligned.bits, 0
  br i1 %aligned, label %zero, label %done
zero:
  %pointer = inttoptr i64 %address to ptr
  %ignored = call ptr @memset(ptr %pointer, i32 0, i64 4096)
  br label %done
done:
  ret i1 %aligned
}

define i64 @__ncc_kernel_ept_pointer(i64 %root, i64 %memory_type, i1 %accessed_dirty) {
entry:
  %root.masked = and i64 %root, 4503599627366400
  %type.masked = and i64 %memory_type, 7
  %base = or i64 %root.masked, %type.masked
  %walk = or i64 %base, 24
  %ad = zext i1 %accessed_dirty to i64
  %ad.bit = shl i64 %ad, 6
  %result = or i64 %walk, %ad.bit
  ret i64 %result
}

define i1 @__ncc_kernel_ept_set_entry(i64 %table, i64 %index, i64 %entry_value) {
entry:
  %alignment.bits = and i64 %table, 4095
  %aligned = icmp eq i64 %alignment.bits, 0
  %index.valid = icmp ult i64 %index, 512
  %valid = and i1 %aligned, %index.valid
  br i1 %valid, label %write, label %done
write:
  %table.ptr = inttoptr i64 %table to ptr
  %entry.ptr = getelementptr i64, ptr %table.ptr, i64 %index
  store volatile i64 %entry_value, ptr %entry.ptr, align 8
  br label %done
done:
  ret i1 %valid
}

define i1 @__ncc_kernel_npt_set_entry(i64 %table, i64 %index, i64 %entry_value) {
entry:
  %status = call i1 @__ncc_kernel_ept_set_entry(i64 %table, i64 %index, i64 %entry_value)
  ret i1 %status
}

define i1 @__ncc_kernel_vmexit_intel_snapshot(i64 %destination) {
entry:
  %p0 = add i64 %destination, 0
  %p1 = add i64 %destination, 8
  %p2 = add i64 %destination, 16
  %p3 = add i64 %destination, 24
  %p4 = add i64 %destination, 32
  %p5 = add i64 %destination, 40
  %p6 = add i64 %destination, 48
  %p7 = add i64 %destination, 56
  %p8 = add i64 %destination, 64
  %p9 = add i64 %destination, 72
  %p10 = add i64 %destination, 80
  %ok0 = call i1 @__ncc_kernel_vmcs_read(i64 17410, i64 %p0)
  %ok1 = call i1 @__ncc_kernel_vmcs_read(i64 17420, i64 %p1)
  %ok2 = call i1 @__ncc_kernel_vmcs_read(i64 25600, i64 %p2)
  %ok3 = call i1 @__ncc_kernel_vmcs_read(i64 9216, i64 %p3)
  %ok4 = call i1 @__ncc_kernel_vmcs_read(i64 25610, i64 %p4)
  %ok5 = call i1 @__ncc_kernel_vmcs_read(i64 17412, i64 %p5)
  %ok6 = call i1 @__ncc_kernel_vmcs_read(i64 17414, i64 %p6)
  %ok7 = call i1 @__ncc_kernel_vmcs_read(i64 17416, i64 %p7)
  %ok8 = call i1 @__ncc_kernel_vmcs_read(i64 17418, i64 %p8)
  %ok9 = call i1 @__ncc_kernel_vmcs_read(i64 26654, i64 %p9)
  %ok10 = call i1 @__ncc_kernel_vmcs_read(i64 26656, i64 %p10)
  %ok01 = and i1 %ok0, %ok1
  %ok23 = and i1 %ok2, %ok3
  %ok45 = and i1 %ok4, %ok5
  %ok67 = and i1 %ok6, %ok7
  %ok89 = and i1 %ok8, %ok9
  %ok0123 = and i1 %ok01, %ok23
  %ok4567 = and i1 %ok45, %ok67
  %ok09 = and i1 %ok0123, %ok4567
  %ok010 = and i1 %ok09, %ok89
  %status = and i1 %ok010, %ok10
  ret i1 %status
}

define i1 @__ncc_kernel_vmexit_amd_snapshot(i64 %vmcb, i64 %destination) {
entry:
  %vmcb.ptr = inttoptr i64 %vmcb to ptr
  %out.ptr = inttoptr i64 %destination to ptr
  %exit.code.ptr = getelementptr i8, ptr %vmcb.ptr, i64 112
  %exit.info1.ptr = getelementptr i8, ptr %vmcb.ptr, i64 120
  %exit.info2.ptr = getelementptr i8, ptr %vmcb.ptr, i64 128
  %exit.int.ptr = getelementptr i8, ptr %vmcb.ptr, i64 136
  %next.rip.ptr = getelementptr i8, ptr %vmcb.ptr, i64 200
  %guest.rip.ptr = getelementptr i8, ptr %vmcb.ptr, i64 1400
  %guest.rflags.ptr = getelementptr i8, ptr %vmcb.ptr, i64 1392
  %exit.code = load volatile i64, ptr %exit.code.ptr, align 8
  %exit.info1 = load volatile i64, ptr %exit.info1.ptr, align 8
  %exit.info2 = load volatile i64, ptr %exit.info2.ptr, align 8
  %exit.int = load volatile i64, ptr %exit.int.ptr, align 8
  %next.rip = load volatile i64, ptr %next.rip.ptr, align 8
  %guest.rip = load volatile i64, ptr %guest.rip.ptr, align 8
  %guest.rflags = load volatile i64, ptr %guest.rflags.ptr, align 8
  %out0 = getelementptr i64, ptr %out.ptr, i64 0
  %out1 = getelementptr i64, ptr %out.ptr, i64 1
  %out2 = getelementptr i64, ptr %out.ptr, i64 2
  %out3 = getelementptr i64, ptr %out.ptr, i64 3
  %out4 = getelementptr i64, ptr %out.ptr, i64 4
  %out5 = getelementptr i64, ptr %out.ptr, i64 5
  %out6 = getelementptr i64, ptr %out.ptr, i64 6
  store volatile i64 %exit.code, ptr %out0, align 8
  store volatile i64 %exit.info1, ptr %out1, align 8
  store volatile i64 %exit.info2, ptr %out2, align 8
  store volatile i64 %exit.int, ptr %out3, align 8
  store volatile i64 %next.rip, ptr %out4, align 8
  store volatile i64 %guest.rip, ptr %out5, align 8
  store volatile i64 %guest.rflags, ptr %out6, align 8
  ret i1 true
}

define i1 @__ncc_kernel_vmexit_intel_inject(i64 %vector, i64 %kind, i1 %error_valid, i64 %error_code, i64 %instruction_length) {
entry:
  %vector.masked = and i64 %vector, 255
  %kind.masked = and i64 %kind, 7
  %kind.shifted = shl i64 %kind.masked, 8
  %base = or i64 %vector.masked, %kind.shifted
  %error.bit.raw = zext i1 %error_valid to i64
  %error.bit = shl i64 %error.bit.raw, 11
  %with.error = or i64 %base, %error.bit
  %information = or i64 %with.error, 2147483648
  %ok0 = call i1 @__ncc_kernel_vmcs_write(i64 16406, i64 %information)
  %ok1 = call i1 @__ncc_kernel_vmcs_write(i64 16408, i64 %error_code)
  %ok2 = call i1 @__ncc_kernel_vmcs_write(i64 16410, i64 %instruction_length)
  %ok01 = and i1 %ok0, %ok1
  %status = and i1 %ok01, %ok2
  ret i1 %status
}

define i1 @__ncc_kernel_vmexit_amd_inject(i64 %vmcb, i64 %vector, i64 %kind, i1 %error_valid, i64 %error_code) {
entry:
  %vector.masked = and i64 %vector, 255
  %kind.masked = and i64 %kind, 7
  %kind.shifted = shl i64 %kind.masked, 8
  %base = or i64 %vector.masked, %kind.shifted
  %error.bit.raw = zext i1 %error_valid to i64
  %error.bit = shl i64 %error.bit.raw, 11
  %with.error = or i64 %base, %error.bit
  %valid = or i64 %with.error, 2147483648
  %error.masked = and i64 %error_code, 4294967295
  %error.shifted = shl i64 %error.masked, 32
  %event = or i64 %valid, %error.shifted
  %vmcb.ptr = inttoptr i64 %vmcb to ptr
  %event.ptr = getelementptr i8, ptr %vmcb.ptr, i64 168
  store volatile i64 %event, ptr %event.ptr, align 8
  ret i1 true
}

define i1 @__ncc_kernel_vmexit_intel_advance(i64 %instruction_length) {
entry:
  %slot = alloca i64, align 8
  %slot.address = ptrtoint ptr %slot to i64
  %read.ok = call i1 @__ncc_kernel_vmcs_read(i64 26654, i64 %slot.address)
  %rip = load i64, ptr %slot, align 8
  %next = add i64 %rip, %instruction_length
  %write.ok = call i1 @__ncc_kernel_vmcs_write(i64 26654, i64 %next)
  %status = and i1 %read.ok, %write.ok
  ret i1 %status
}

define i1 @__ncc_kernel_vmexit_amd_advance(i64 %vmcb) {
entry:
  %vmcb.ptr = inttoptr i64 %vmcb to ptr
  %next.ptr = getelementptr i8, ptr %vmcb.ptr, i64 200
  %rip.ptr = getelementptr i8, ptr %vmcb.ptr, i64 1400
  %next = load volatile i64, ptr %next.ptr, align 8
  store volatile i64 %next, ptr %rip.ptr, align 8
  ret i1 true
}

define i64 @__ncc_kernel_cpu_read_cr0() {
entry:
  %value = call i64 asm sideeffect ".byte 0x0f,0x20,0xc0", "={rax},~{memory}"()
  ret i64 %value
}

define i64 @__ncc_kernel_cpu_read_cr3() {
entry:
  %value = call i64 asm sideeffect ".byte 0x0f,0x20,0xd8", "={rax},~{memory}"()
  ret i64 %value
}

define i64 @__ncc_kernel_cpu_read_cr4() {
entry:
  %value = call i64 asm sideeffect ".byte 0x0f,0x20,0xe0", "={rax},~{memory}"()
  ret i64 %value
}

define i1 @__ncc_kernel_cpu_write_cr0(i64 %value) {
entry:
  call void asm sideeffect ".byte 0x0f,0x22,0xc0", "{rax},~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_cpu_write_cr3(i64 %value) {
entry:
  call void asm sideeffect ".byte 0x0f,0x22,0xd8", "{rax},~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_cpu_write_cr4(i64 %value) {
entry:
  call void asm sideeffect ".byte 0x0f,0x22,0xe0", "{rax},~{memory}"(i64 %value)
  ret i1 true
}

define i64 @__ncc_kernel_cpu_read_msr(i64 %msr) {
entry:
  %value = call i64 asm sideeffect ".byte 0x0f,0x32,0x48,0xc1,0xe2,0x20,0x48,0x09,0xd0", "={rax},{rcx},~{rdx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %msr)
  ret i64 %value
}

define i1 @__ncc_kernel_cpu_write_msr(i64 %msr, i64 %value) {
entry:
  %high = lshr i64 %value, 32
  call void asm sideeffect ".byte 0x0f,0x30", "{rcx},{rax},{rdx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %msr, i64 %value, i64 %high)
  ret i1 true
}

define i1 @__ncc_kernel_cpuid_write(i64 %leaf, i64 %subleaf, i64 %destination) {
entry:
  %leaf32 = trunc i64 %leaf to i32
  %subleaf32 = trunc i64 %subleaf to i32
  %result = call { i32, i32, i32, i32 } asm sideeffect "cpuid", "={ax},={bx},={cx},={dx},{ax},{cx},~{dirflag},~{fpsr},~{flags}"(i32 %leaf32, i32 %subleaf32)
  %eax = extractvalue { i32, i32, i32, i32 } %result, 0
  %ebx = extractvalue { i32, i32, i32, i32 } %result, 1
  %ecx = extractvalue { i32, i32, i32, i32 } %result, 2
  %edx = extractvalue { i32, i32, i32, i32 } %result, 3
  %out = inttoptr i64 %destination to ptr
  %out0 = getelementptr i32, ptr %out, i64 0
  %out1 = getelementptr i32, ptr %out, i64 1
  %out2 = getelementptr i32, ptr %out, i64 2
  %out3 = getelementptr i32, ptr %out, i64 3
  store volatile i32 %eax, ptr %out0, align 4
  store volatile i32 %ebx, ptr %out1, align 4
  store volatile i32 %ecx, ptr %out2, align 4
  store volatile i32 %edx, ptr %out3, align 4
  ret i1 true
}

define i64 @__ncc_kernel_port_read8(i64 %port) {
entry:
  %port16 = trunc i64 %port to i16
  %value = call i8 asm sideeffect ".byte 0xec", "={al},{dx},~{memory}"(i16 %port16)
  %wide = zext i8 %value to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_port_read16(i64 %port) {
entry:
  %port16 = trunc i64 %port to i16
  %value = call i16 asm sideeffect ".byte 0x66,0xed", "={ax},{dx},~{memory}"(i16 %port16)
  %wide = zext i16 %value to i64
  ret i64 %wide
}

define i64 @__ncc_kernel_port_read32(i64 %port) {
entry:
  %port16 = trunc i64 %port to i16
  %value = call i32 asm sideeffect ".byte 0xed", "={ax},{dx},~{memory}"(i16 %port16)
  %wide = zext i32 %value to i64
  ret i64 %wide
}

define i1 @__ncc_kernel_port_write8(i64 %port, i64 %value) {
entry:
  %port16 = trunc i64 %port to i16
  %value8 = trunc i64 %value to i8
  call void asm sideeffect ".byte 0xee", "{al},{dx},~{memory}"(i8 %value8, i16 %port16)
  ret i1 true
}

define i1 @__ncc_kernel_port_write16(i64 %port, i64 %value) {
entry:
  %port16 = trunc i64 %port to i16
  %value16 = trunc i64 %value to i16
  call void asm sideeffect ".byte 0x66,0xef", "{ax},{dx},~{memory}"(i16 %value16, i16 %port16)
  ret i1 true
}

define i1 @__ncc_kernel_port_write32(i64 %port, i64 %value) {
entry:
  %port16 = trunc i64 %port to i16
  %value32 = trunc i64 %value to i32
  call void asm sideeffect ".byte 0xef", "{ax},{dx},~{memory}"(i32 %value32, i16 %port16)
  ret i1 true
}

define i1 @__ncc_kernel_svm_clgi() {
entry:
  call void asm sideeffect ".byte 0x0f,0x01,0xdd", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_svm_stgi() {
entry:
  call void asm sideeffect ".byte 0x0f,0x01,0xdc", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_svm_invlpga(i64 %linear_address, i64 %asid) {
entry:
  %rax.after = call i64 asm sideeffect ".byte 0x0f,0x01,0xdf", "={rax},0,{rcx},~{dirflag},~{fpsr},~{flags},~{memory}"(i64 %linear_address, i64 %asid)
  ret i1 true
}
"#;
    }

    r#"
define i1 @__ncc_kernel_vmx_on(i64 %physical_slot) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmx_off() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmcs_clear(i64 %physical_slot) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmcs_load(i64 %physical_slot) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmcs_store(i64 %destination) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmcs_read(i64 %field, i64 %destination) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmcs_write(i64 %field, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_invept(i64 %kind, i64 %descriptor) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_invvpid(i64 %kind, i64 %descriptor) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vm_call() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vm_launch() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vm_resume() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_run(i64 %vmcb_physical) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_call() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_load(i64 %vmcb_physical) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_save(i64 %vmcb_physical) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_guest_phys_read64(i64 %address) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_guest_phys_write64(i64 %address, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_guest_phys_zero_page(i64 %address) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_ept_pointer(i64 %root, i64 %memory_type, i1 %accessed_dirty) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_ept_set_entry(i64 %table, i64 %index, i64 %entry_value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_npt_set_entry(i64 %table, i64 %index, i64 %entry_value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmexit_intel_snapshot(i64 %destination) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmexit_amd_snapshot(i64 %vmcb, i64 %destination) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmexit_intel_inject(i64 %vector, i64 %kind, i1 %error_valid, i64 %error_code, i64 %instruction_length) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmexit_amd_inject(i64 %vmcb, i64 %vector, i64 %kind, i1 %error_valid, i64 %error_code) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmexit_intel_advance(i64 %instruction_length) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_vmexit_amd_advance(i64 %vmcb) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_cpu_read_cr0() { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_cpu_read_cr3() { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_cpu_read_cr4() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_cpu_write_cr0(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_cpu_write_cr3(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_cpu_write_cr4(i64 %value) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_cpu_read_msr(i64 %msr) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_cpu_write_msr(i64 %msr, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_cpuid_write(i64 %leaf, i64 %subleaf, i64 %destination) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_port_read8(i64 %port) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_port_read16(i64 %port) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_port_read32(i64 %port) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_port_write8(i64 %port, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_port_write16(i64 %port, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_port_write32(i64 %port, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_clgi() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_stgi() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_svm_invlpga(i64 %linear_address, i64 %asid) { call void @llvm.trap(); unreachable }
"#
}

fn render_kernel_hypervisor_iommu_runtime(target_triple: &str) -> String {
    let triple = target_triple.to_ascii_lowercase();
    let mut llvm = r#"
define i1 @__ncc_kernel_iommu_map_entry(i64 %table, i64 %index, i64 %entry_value) {
entry:
  %table.ptr = inttoptr i64 %table to ptr
  %entry.ptr = getelementptr i64, ptr %table.ptr, i64 %index
  store volatile i64 %entry_value, ptr %entry.ptr, align 1
  ret i1 true
}

define i1 @__ncc_kernel_iommu_unmap_entry(i64 %table, i64 %index) {
entry:
  %table.ptr = inttoptr i64 %table to ptr
  %entry.ptr = getelementptr i64, ptr %table.ptr, i64 %index
  store volatile i64 0, ptr %entry.ptr, align 1
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_iommu_invalidate(i64 %command_address, i64 %command) {
entry:
  %command.ptr = inttoptr i64 %command_address to ptr
  store volatile i64 %command, ptr %command.ptr, align 1
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_descriptor_write(i64 %descriptor, i64 %iova, i64 %host_address, i64 %length, i64 %permissions) {
entry:
  %descriptor.ptr = inttoptr i64 %descriptor to ptr
  %iova.ptr = getelementptr i64, ptr %descriptor.ptr, i64 0
  %host.ptr = getelementptr i64, ptr %descriptor.ptr, i64 1
  %length.ptr = getelementptr i64, ptr %descriptor.ptr, i64 2
  %permissions.ptr = getelementptr i64, ptr %descriptor.ptr, i64 3
  store volatile i64 %iova, ptr %iova.ptr, align 1
  store volatile i64 %host_address, ptr %host.ptr, align 1
  store volatile i64 %length, ptr %length.ptr, align 1
  store volatile i64 %permissions, ptr %permissions.ptr, align 1
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_pin_pages(i64 %metadata, i64 %first_page, i64 %page_count, i64 %stride, i64 %flags_offset, i64 %pin_mask) {
entry:
  %empty = icmp eq i64 %page_count, 0
  br i1 %empty, label %done, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %loop ]
  %page = add i64 %first_page, %index
  %record.offset = mul i64 %page, %stride
  %record = add i64 %metadata, %record.offset
  %flags.address = add i64 %record, %flags_offset
  %flags.ptr = inttoptr i64 %flags.address to ptr
  %old = load volatile i64, ptr %flags.ptr, align 1
  %new = or i64 %old, %pin_mask
  store volatile i64 %new, ptr %flags.ptr, align 1
  %next = add i64 %index, 1
  %more = icmp ult i64 %next, %page_count
  br i1 %more, label %loop, label %done
done:
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_unpin_pages(i64 %metadata, i64 %first_page, i64 %page_count, i64 %stride, i64 %flags_offset, i64 %pin_mask) {
entry:
  %inverse = xor i64 %pin_mask, -1
  %empty = icmp eq i64 %page_count, 0
  br i1 %empty, label %done, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %loop ]
  %page = add i64 %first_page, %index
  %record.offset = mul i64 %page, %stride
  %record = add i64 %metadata, %record.offset
  %flags.address = add i64 %record, %flags_offset
  %flags.ptr = inttoptr i64 %flags.address to ptr
  %old = load volatile i64, ptr %flags.ptr, align 1
  %new = and i64 %old, %inverse
  store volatile i64 %new, ptr %flags.ptr, align 1
  %next = add i64 %index, 1
  %more = icmp ult i64 %next, %page_count
  br i1 %more, label %loop, label %done
done:
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_sg_write(i64 %table, i64 %index, i64 %iova, i64 %physical, i64 %length, i64 %flags) {
entry:
  %table.ptr = inttoptr i64 %table to ptr
  %word.index = mul i64 %index, 4
  %iova.ptr = getelementptr i64, ptr %table.ptr, i64 %word.index
  %physical.index = add i64 %word.index, 1
  %physical.ptr = getelementptr i64, ptr %table.ptr, i64 %physical.index
  %length.index = add i64 %word.index, 2
  %length.ptr = getelementptr i64, ptr %table.ptr, i64 %length.index
  %flags.index = add i64 %word.index, 3
  %flags.ptr = getelementptr i64, ptr %table.ptr, i64 %flags.index
  store volatile i64 %iova, ptr %iova.ptr, align 1
  store volatile i64 %physical, ptr %physical.ptr, align 1
  store volatile i64 %length, ptr %length.ptr, align 1
  store volatile i64 %flags, ptr %flags.ptr, align 1
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define internal void @__ncc_kernel_dma_copy(i64 %destination, i64 %source, i64 %bytes) {
entry:
  %empty = icmp eq i64 %bytes, 0
  br i1 %empty, label %done, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %loop ]
  %source.address = add i64 %source, %index
  %destination.address = add i64 %destination, %index
  %source.ptr = inttoptr i64 %source.address to ptr
  %destination.ptr = inttoptr i64 %destination.address to ptr
  %value = load volatile i8, ptr %source.ptr, align 1
  store volatile i8 %value, ptr %destination.ptr, align 1
  %next = add i64 %index, 1
  %more = icmp ult i64 %next, %bytes
  br i1 %more, label %loop, label %done
done:
  ret void
}

define i1 @__ncc_kernel_dma_bounce_to_device(i64 %bounce, i64 %source, i64 %bytes) {
entry:
  call void @__ncc_kernel_dma_copy(i64 %bounce, i64 %source, i64 %bytes)
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_bounce_from_device(i64 %destination, i64 %bounce, i64 %bytes) {
entry:
  call void @__ncc_kernel_dma_copy(i64 %destination, i64 %bounce, i64 %bytes)
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define internal i1 @__ncc_kernel_dma_cache_range(i64 %address, i64 %bytes, i64 %line_bytes, i64 %operation) {
entry:
  %empty = icmp eq i64 %bytes, 0
  br i1 %empty, label %done, label %prepare
prepare:
  br label %loop
loop:
  %offset = phi i64 [ 0, %prepare ], [ %next.offset, %dispatch.done ]
  %line = add i64 %address, %offset
  %is.clean = icmp eq i64 %operation, 0
  br i1 %is.clean, label %clean, label %not.clean
clean:
  %cleaned = call i1 @__ncc_kernel_dma_cache_clean_line(i64 %line)
  br label %dispatch.done
not.clean:
  %is.invalidate = icmp eq i64 %operation, 1
  br i1 %is.invalidate, label %invalidate, label %flush
invalidate:
  %invalidated = call i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line)
  br label %dispatch.done
flush:
  %flushed = call i1 @__ncc_kernel_dma_cache_flush_line(i64 %line)
  br label %dispatch.done
dispatch.done:
  %next.offset = add i64 %offset, %line_bytes
  %more = icmp ult i64 %next.offset, %bytes
  br i1 %more, label %loop, label %done
done:
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_cache_clean(i64 %address, i64 %bytes, i64 %line_bytes) {
entry:
  %result = call i1 @__ncc_kernel_dma_cache_range(i64 %address, i64 %bytes, i64 %line_bytes, i64 0)
  ret i1 %result
}

define i1 @__ncc_kernel_dma_cache_invalidate(i64 %address, i64 %bytes, i64 %line_bytes) {
entry:
  %result = call i1 @__ncc_kernel_dma_cache_range(i64 %address, i64 %bytes, i64 %line_bytes, i64 1)
  ret i1 %result
}

define i1 @__ncc_kernel_dma_cache_flush(i64 %address, i64 %bytes, i64 %line_bytes) {
entry:
  %result = call i1 @__ncc_kernel_dma_cache_range(i64 %address, i64 %bytes, i64 %line_bytes, i64 2)
  ret i1 %result
}

define i1 @__ncc_kernel_iommu_command_write(i64 %queue, i64 %index, i64 %low, i64 %high) {
entry:
  %queue.ptr = inttoptr i64 %queue to ptr
  %word.index = mul i64 %index, 2
  %low.ptr = getelementptr i64, ptr %queue.ptr, i64 %word.index
  %high.index = add i64 %word.index, 1
  %high.ptr = getelementptr i64, ptr %queue.ptr, i64 %high.index
  store volatile i64 %low, ptr %low.ptr, align 1
  store volatile i64 %high, ptr %high.ptr, align 1
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_iommu_map_entry128(i64 %table, i64 %index, i64 %low, i64 %high) {
entry:
  %written = call i1 @__ncc_kernel_iommu_command_write(i64 %table, i64 %index, i64 %low, i64 %high)
  ret i1 %written
}

define i1 @__ncc_kernel_iommu_fault_copy(i64 %queue, i64 %index, i64 %destination, i64 %entry_bytes) {
entry:
  %offset = mul i64 %index, %entry_bytes
  %source = add i64 %queue, %offset
  call void @__ncc_kernel_dma_copy(i64 %destination, i64 %source, i64 %entry_bytes)
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}

define i1 @__ncc_kernel_dma_fence() {
entry:
  %fenced = call i1 @__ncc_kernel_arch_memory_barrier()
  ret i1 %fenced
}
"#
    .to_string();

    if triple.starts_with("x86_64") {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) {
entry:
  call void asm sideeffect ".byte 0x0f,0xae,0x38", "{rax},~{memory}"(i64 %line)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) {
entry:
  call void asm sideeffect ".byte 0x0f,0xae,0x38", "{rax},~{memory}"(i64 %line)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) {
entry:
  call void asm sideeffect ".byte 0x0f,0xae,0x38", "{rax},~{memory}"(i64 %line)
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("i386")
        || triple.starts_with("i486")
        || triple.starts_with("i586")
        || triple.starts_with("i686")
    {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect ".byte 0x0f,0xae,0x38", "{eax},~{memory}"(i32 %line32)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect ".byte 0x0f,0xae,0x38", "{eax},~{memory}"(i32 %line32)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect ".byte 0x0f,0xae,0x38", "{eax},~{memory}"(i32 %line32)
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("aarch64") {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) {
entry:
  call void asm sideeffect "dc cvac, $0", "r,~{memory}"(i64 %line)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) {
entry:
  call void asm sideeffect "dc ivac, $0", "r,~{memory}"(i64 %line)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) {
entry:
  call void asm sideeffect "dc civac, $0", "r,~{memory}"(i64 %line)
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("armv7") || triple.starts_with("thumbv7") {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect "mcr p15, 0, $0, c7, c10, 1", "r,~{memory}"(i32 %line32)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect "mcr p15, 0, $0, c7, c6, 1", "r,~{memory}"(i32 %line32)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect "mcr p15, 0, $0, c7, c14, 1", "r,~{memory}"(i32 %line32)
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("riscv64") {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) {
entry:
  call void asm sideeffect ".word 0x0015200f", "{x10},~{memory}"(i64 %line)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) {
entry:
  call void asm sideeffect ".word 0x0005200f", "{x10},~{memory}"(i64 %line)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) {
entry:
  call void asm sideeffect ".word 0x0025200f", "{x10},~{memory}"(i64 %line)
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("riscv32") {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect ".word 0x0015200f", "{x10},~{memory}"(i32 %line32)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect ".word 0x0005200f", "{x10},~{memory}"(i32 %line32)
  ret i1 true
}
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) {
entry:
  %line32 = trunc i64 %line to i32
  call void asm sideeffect ".word 0x0025200f", "{x10},~{memory}"(i32 %line32)
  ret i1 true
}
"#,
        );
    } else {
        llvm.push_str(
            r#"
define internal i1 @__ncc_kernel_dma_cache_clean_line(i64 %line) { ret i1 false }
define internal i1 @__ncc_kernel_dma_cache_invalidate_line(i64 %line) { ret i1 false }
define internal i1 @__ncc_kernel_dma_cache_flush_line(i64 %line) { ret i1 false }
"#,
        );
    }

    if triple.starts_with("aarch64") {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_el2_read_hcr() {
entry:
  %value = call i64 asm sideeffect "mrs $0, hcr_el2", "=r,~{memory}"()
  ret i64 %value
}

define i1 @__ncc_kernel_el2_write_hcr(i64 %value) {
entry:
  call void asm sideeffect "msr hcr_el2, $0; isb", "r,~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_el2_write_vtcr(i64 %value) {
entry:
  call void asm sideeffect "msr vtcr_el2, $0; isb", "r,~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_el2_write_vttbr(i64 %value) {
entry:
  call void asm sideeffect "msr vttbr_el2, $0; isb", "r,~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_el2_tlbi_all() {
entry:
  call void asm sideeffect "dsb ish; tlbi vmalls12e1is; dsb ish; isb", "~{memory}"()
  ret i1 true
}

define i64 @__ncc_kernel_el2_hvc(i64 %argument) {
entry:
  %result = call i64 asm sideeffect "hvc #0", "={x0},0,~{memory}"(i64 %argument)
  ret i64 %result
}

define i1 @__ncc_kernel_el2_exit_snapshot(i64 %destination) {
entry:
  %esr = call i64 asm sideeffect "mrs $0, esr_el2", "=r,~{memory}"()
  %far = call i64 asm sideeffect "mrs $0, far_el2", "=r,~{memory}"()
  %hpfar = call i64 asm sideeffect "mrs $0, hpfar_el2", "=r,~{memory}"()
  %elr = call i64 asm sideeffect "mrs $0, elr_el2", "=r,~{memory}"()
  %spsr = call i64 asm sideeffect "mrs $0, spsr_el2", "=r,~{memory}"()
  %out = inttoptr i64 %destination to ptr
  %out0 = getelementptr i64, ptr %out, i64 0
  %out1 = getelementptr i64, ptr %out, i64 1
  %out2 = getelementptr i64, ptr %out, i64 2
  %out3 = getelementptr i64, ptr %out, i64 3
  %out4 = getelementptr i64, ptr %out, i64 4
  store volatile i64 %esr, ptr %out0, align 1
  store volatile i64 %far, ptr %out1, align 1
  store volatile i64 %hpfar, ptr %out2, align 1
  store volatile i64 %elr, ptr %out3, align 1
  store volatile i64 %spsr, ptr %out4, align 1
  ret i1 true
}
"#,
        );
    } else {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_el2_read_hcr() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_el2_write_hcr(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_el2_write_vtcr(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_el2_write_vttbr(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_el2_tlbi_all() { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_el2_hvc(i64 %argument) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_el2_exit_snapshot(i64 %destination) { call void @llvm.trap(); unreachable }
"#,
        );
    }

    if triple.starts_with("riscv64") {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_riscv_h_read_hstatus() {
entry:
  %value = call i64 asm sideeffect ".word 0x60002573", "={x10},~{memory}"()
  ret i64 %value
}

define i1 @__ncc_kernel_riscv_h_write_hstatus(i64 %value) {
entry:
  call void asm sideeffect ".word 0x60051073", "{x10},~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_write_hgatp(i64 %value) {
entry:
  call void asm sideeffect ".word 0x68051073", "{x10},~{memory}"(i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_hfence_gvma(i64 %guest_address, i64 %vmid) {
entry:
  call void asm sideeffect ".word 0x62b50073", "{x10},{x11},~{memory}"(i64 %guest_address, i64 %vmid)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_hfence_vvma(i64 %virtual_address, i64 %asid) {
entry:
  call void asm sideeffect ".word 0x22b50073", "{x10},{x11},~{memory}"(i64 %virtual_address, i64 %asid)
  ret i1 true
}

define i64 @__ncc_kernel_riscv_h_hlv(i64 %guest_address) {
entry:
  %value = call i64 asm sideeffect ".word 0x6c05c573", "={x10},{x11},~{memory}"(i64 %guest_address)
  ret i64 %value
}

define i1 @__ncc_kernel_riscv_h_hsv(i64 %guest_address, i64 %value) {
entry:
  call void asm sideeffect ".word 0x6ea5c073", "{x11},{x10},~{memory}"(i64 %guest_address, i64 %value)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_exit_snapshot(i64 %destination) {
entry:
  %scause = call i64 asm sideeffect ".word 0x14202573", "={x10},~{memory}"()
  %sepc = call i64 asm sideeffect ".word 0x14102573", "={x10},~{memory}"()
  %stval = call i64 asm sideeffect ".word 0x14302573", "={x10},~{memory}"()
  %htval = call i64 asm sideeffect ".word 0x64302573", "={x10},~{memory}"()
  %htinst = call i64 asm sideeffect ".word 0x64a02573", "={x10},~{memory}"()
  %hstatus = call i64 asm sideeffect ".word 0x60002573", "={x10},~{memory}"()
  %out = inttoptr i64 %destination to ptr
  %out0 = getelementptr i64, ptr %out, i64 0
  %out1 = getelementptr i64, ptr %out, i64 1
  %out2 = getelementptr i64, ptr %out, i64 2
  %out3 = getelementptr i64, ptr %out, i64 3
  %out4 = getelementptr i64, ptr %out, i64 4
  %out5 = getelementptr i64, ptr %out, i64 5
  store volatile i64 %scause, ptr %out0, align 1
  store volatile i64 %sepc, ptr %out1, align 1
  store volatile i64 %stval, ptr %out2, align 1
  store volatile i64 %htval, ptr %out3, align 1
  store volatile i64 %htinst, ptr %out4, align 1
  store volatile i64 %hstatus, ptr %out5, align 1
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("riscv32") {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_riscv_h_read_hstatus() {
entry:
  %raw = call i32 asm sideeffect ".word 0x60002573", "={x10},~{memory}"()
  %value = zext i32 %raw to i64
  ret i64 %value
}

define i1 @__ncc_kernel_riscv_h_write_hstatus(i64 %value) {
entry:
  %value32 = trunc i64 %value to i32
  call void asm sideeffect ".word 0x60051073", "{x10},~{memory}"(i32 %value32)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_write_hgatp(i64 %value) {
entry:
  %value32 = trunc i64 %value to i32
  call void asm sideeffect ".word 0x68051073", "{x10},~{memory}"(i32 %value32)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_hfence_gvma(i64 %guest_address, i64 %vmid) {
entry:
  %address32 = trunc i64 %guest_address to i32
  %vmid32 = trunc i64 %vmid to i32
  call void asm sideeffect ".word 0x62b50073", "{x10},{x11},~{memory}"(i32 %address32, i32 %vmid32)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_hfence_vvma(i64 %virtual_address, i64 %asid) {
entry:
  %address32 = trunc i64 %virtual_address to i32
  %asid32 = trunc i64 %asid to i32
  call void asm sideeffect ".word 0x22b50073", "{x10},{x11},~{memory}"(i32 %address32, i32 %asid32)
  ret i1 true
}

define i64 @__ncc_kernel_riscv_h_hlv(i64 %guest_address) {
entry:
  %address32 = trunc i64 %guest_address to i32
  %raw = call i32 asm sideeffect ".word 0x6805c573", "={x10},{x11},~{memory}"(i32 %address32)
  %value = sext i32 %raw to i64
  ret i64 %value
}

define i1 @__ncc_kernel_riscv_h_hsv(i64 %guest_address, i64 %value) {
entry:
  %address32 = trunc i64 %guest_address to i32
  %value32 = trunc i64 %value to i32
  call void asm sideeffect ".word 0x6aa5c073", "{x11},{x10},~{memory}"(i32 %address32, i32 %value32)
  ret i1 true
}

define i1 @__ncc_kernel_riscv_h_exit_snapshot(i64 %destination) {
entry:
  %scause.raw = call i32 asm sideeffect ".word 0x14202573", "={x10},~{memory}"()
  %sepc.raw = call i32 asm sideeffect ".word 0x14102573", "={x10},~{memory}"()
  %stval.raw = call i32 asm sideeffect ".word 0x14302573", "={x10},~{memory}"()
  %htval.raw = call i32 asm sideeffect ".word 0x64302573", "={x10},~{memory}"()
  %htinst.raw = call i32 asm sideeffect ".word 0x64a02573", "={x10},~{memory}"()
  %hstatus.raw = call i32 asm sideeffect ".word 0x60002573", "={x10},~{memory}"()
  %scause = zext i32 %scause.raw to i64
  %sepc = zext i32 %sepc.raw to i64
  %stval = zext i32 %stval.raw to i64
  %htval = zext i32 %htval.raw to i64
  %htinst = zext i32 %htinst.raw to i64
  %hstatus = zext i32 %hstatus.raw to i64
  %out = inttoptr i64 %destination to ptr
  %out0 = getelementptr i64, ptr %out, i64 0
  %out1 = getelementptr i64, ptr %out, i64 1
  %out2 = getelementptr i64, ptr %out, i64 2
  %out3 = getelementptr i64, ptr %out, i64 3
  %out4 = getelementptr i64, ptr %out, i64 4
  %out5 = getelementptr i64, ptr %out, i64 5
  store volatile i64 %scause, ptr %out0, align 1
  store volatile i64 %sepc, ptr %out1, align 1
  store volatile i64 %stval, ptr %out2, align 1
  store volatile i64 %htval, ptr %out3, align 1
  store volatile i64 %htinst, ptr %out4, align 1
  store volatile i64 %hstatus, ptr %out5, align 1
  ret i1 true
}
"#,
        );
    } else {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_riscv_h_read_hstatus() { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_riscv_h_write_hstatus(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_riscv_h_write_hgatp(i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_riscv_h_hfence_gvma(i64 %guest_address, i64 %vmid) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_riscv_h_hfence_vvma(i64 %virtual_address, i64 %asid) { call void @llvm.trap(); unreachable }
define i64 @__ncc_kernel_riscv_h_hlv(i64 %guest_address) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_riscv_h_hsv(i64 %guest_address, i64 %value) { call void @llvm.trap(); unreachable }
define i1 @__ncc_kernel_riscv_h_exit_snapshot(i64 %destination) { call void @llvm.trap(); unreachable }
"#,
        );
    }
    llvm
}

fn render_kernel_device_runtime(target_triple: &str) -> String {
    let triple = target_triple.to_ascii_lowercase();
    let mut llvm = r#"
define i64 @__ncc_kernel_pci_ecam_read32(i64 %base, i64 %bus, i64 %device, i64 %function, i64 %offset) {
entry:
  %bus.masked = and i64 %bus, 255
  %device.masked = and i64 %device, 31
  %function.masked = and i64 %function, 7
  %offset.masked = and i64 %offset, 4092
  %bus.shifted = shl i64 %bus.masked, 20
  %device.shifted = shl i64 %device.masked, 15
  %function.shifted = shl i64 %function.masked, 12
  %bus.address = add i64 %base, %bus.shifted
  %device.address = add i64 %bus.address, %device.shifted
  %function.address = add i64 %device.address, %function.shifted
  %address = add i64 %function.address, %offset.masked
  %value = call i64 @__ncc_kernel_mmio_read32(i64 %address)
  ret i64 %value
}

define i1 @__ncc_kernel_pci_ecam_write32(i64 %base, i64 %bus, i64 %device, i64 %function, i64 %offset, i64 %value) {
entry:
  %bus.masked = and i64 %bus, 255
  %device.masked = and i64 %device, 31
  %function.masked = and i64 %function, 7
  %offset.masked = and i64 %offset, 4092
  %bus.shifted = shl i64 %bus.masked, 20
  %device.shifted = shl i64 %device.masked, 15
  %function.shifted = shl i64 %function.masked, 12
  %bus.address = add i64 %base, %bus.shifted
  %device.address = add i64 %bus.address, %device.shifted
  %function.address = add i64 %device.address, %function.shifted
  %address = add i64 %function.address, %offset.masked
  %written = call i1 @__ncc_kernel_mmio_write32(i64 %address, i64 %value)
  ret i1 %written
}

define i1 @__ncc_kernel_virtio_notify(i64 %notify_address, i64 %queue_index) {
entry:
  %written = call i1 @__ncc_kernel_mmio_write16(i64 %notify_address, i64 %queue_index)
  ret i1 %written
}
"#
    .to_string();

    if triple.starts_with("x86_64") || triple.starts_with("i386") || triple.starts_with("i686") {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_pci_config_read32(i64 %bus, i64 %device, i64 %function, i64 %offset) {
entry:
  %bus.masked = and i64 %bus, 255
  %device.masked = and i64 %device, 31
  %function.masked = and i64 %function, 7
  %offset.masked = and i64 %offset, 252
  %bus.shifted = shl i64 %bus.masked, 16
  %device.shifted = shl i64 %device.masked, 11
  %function.shifted = shl i64 %function.masked, 8
  %address.0 = or i64 %bus.shifted, %device.shifted
  %address.1 = or i64 %address.0, %function.shifted
  %address.2 = or i64 %address.1, %offset.masked
  %address = or i64 %address.2, 2147483648
  %selected = call i1 @__ncc_kernel_port_write32(i64 3320, i64 %address)
  %value = call i64 @__ncc_kernel_port_read32(i64 3324)
  ret i64 %value
}

define i1 @__ncc_kernel_pci_config_write32(i64 %bus, i64 %device, i64 %function, i64 %offset, i64 %value) {
entry:
  %bus.masked = and i64 %bus, 255
  %device.masked = and i64 %device, 31
  %function.masked = and i64 %function, 7
  %offset.masked = and i64 %offset, 252
  %bus.shifted = shl i64 %bus.masked, 16
  %device.shifted = shl i64 %device.masked, 11
  %function.shifted = shl i64 %function.masked, 8
  %address.0 = or i64 %bus.shifted, %device.shifted
  %address.1 = or i64 %address.0, %function.shifted
  %address.2 = or i64 %address.1, %offset.masked
  %address = or i64 %address.2, 2147483648
  %selected = call i1 @__ncc_kernel_port_write32(i64 3320, i64 %address)
  %written = call i1 @__ncc_kernel_port_write32(i64 3324, i64 %value)
  %complete = and i1 %selected, %written
  ret i1 %complete
}
"#,
        );
        if triple.starts_with("x86_64") {
            llvm.push_str(
                r#"

@__ncc_kernel_debug_idt = internal global [256 x i128] zeroinitializer, align 16
@__ncc_kernel_debug_saved_idtr = internal global <{ i16, i64 }> zeroinitializer, align 1
@__ncc_kernel_debug_active_idtr = internal global <{ i16, i64 }> <{ i16 4095, i64 ptrtoint (ptr @__ncc_kernel_debug_idt to i64) }>, align 1

define void @__ncc_kernel_debug_break_handler() naked nounwind {
entry:
  call void asm sideeffect "iretq", "~{memory}"()
  unreachable
}

define i1 @__ncc_kernel_debug_break() {
entry:
  %handler.address = ptrtoint ptr @__ncc_kernel_debug_break_handler to i64
  %gate = getelementptr i8, ptr @__ncc_kernel_debug_idt, i64 48
  %offset.low = trunc i64 %handler.address to i16
  store i16 %offset.low, ptr %gate, align 16
  %selector.slot = getelementptr i8, ptr %gate, i64 2
  store i16 8, ptr %selector.slot, align 2
  %ist.slot = getelementptr i8, ptr %gate, i64 4
  store i8 0, ptr %ist.slot, align 1
  %attributes.slot = getelementptr i8, ptr %gate, i64 5
  store i8 -113, ptr %attributes.slot, align 1
  %offset.mid.shifted = lshr i64 %handler.address, 16
  %offset.mid = trunc i64 %offset.mid.shifted to i16
  %offset.mid.slot = getelementptr i8, ptr %gate, i64 6
  store i16 %offset.mid, ptr %offset.mid.slot, align 2
  %offset.high.shifted = lshr i64 %handler.address, 32
  %offset.high = trunc i64 %offset.high.shifted to i32
  %offset.high.slot = getelementptr i8, ptr %gate, i64 8
  store i32 %offset.high, ptr %offset.high.slot, align 4
  %reserved.slot = getelementptr i8, ptr %gate, i64 12
  store i32 0, ptr %reserved.slot, align 4
  call void asm sideeffect ".byte 0x0f,0x01,0x08", "{rax},~{memory}"(ptr @__ncc_kernel_debug_saved_idtr)
  call void asm sideeffect ".byte 0x0f,0x01,0x18", "{rax},~{memory}"(ptr @__ncc_kernel_debug_active_idtr)
  call void asm sideeffect "int3", "~{memory}"()
  call void asm sideeffect ".byte 0x0f,0x01,0x18", "{rax},~{memory}"(ptr @__ncc_kernel_debug_saved_idtr)
  ret i1 true
}
"#,
            );
        } else {
            llvm.push_str(
                r#"
@__ncc_kernel_debug_idt32 = internal global [256 x i64] zeroinitializer, align 8
@__ncc_kernel_debug_saved_idtr32 = internal global <{ i16, i32 }> zeroinitializer, align 1
@__ncc_kernel_debug_active_idtr32 = internal global <{ i16, i32 }> <{ i16 2047, i32 0 }>, align 1

define void @__ncc_kernel_debug_break_handler() naked nounwind {
entry:
  call void asm sideeffect ".byte 0xcf", "~{memory}"()
  unreachable
}

define i1 @__ncc_kernel_debug_break() {
entry:
  %handler.address = ptrtoint ptr @__ncc_kernel_debug_break_handler to i32
  %gate = getelementptr i8, ptr @__ncc_kernel_debug_idt32, i32 24
  %offset.low = trunc i32 %handler.address to i16
  store i16 %offset.low, ptr %gate, align 8
  %selector.slot = getelementptr i8, ptr %gate, i32 2
  store i16 8, ptr %selector.slot, align 2
  %zero.slot = getelementptr i8, ptr %gate, i32 4
  store i8 0, ptr %zero.slot, align 1
  %attributes.slot = getelementptr i8, ptr %gate, i32 5
  store i8 -113, ptr %attributes.slot, align 1
  %offset.high.shifted = lshr i32 %handler.address, 16
  %offset.high = trunc i32 %offset.high.shifted to i16
  %offset.high.slot = getelementptr i8, ptr %gate, i32 6
  store i16 %offset.high, ptr %offset.high.slot, align 2
  %idt.address = ptrtoint ptr @__ncc_kernel_debug_idt32 to i32
  %active.base.slot = getelementptr i8, ptr @__ncc_kernel_debug_active_idtr32, i32 2
  store i32 %idt.address, ptr %active.base.slot, align 1
  call void asm sideeffect ".byte 0x0f,0x01,0x08", "{eax},~{memory}"(ptr @__ncc_kernel_debug_saved_idtr32)
  call void asm sideeffect ".byte 0x0f,0x01,0x18", "{eax},~{memory}"(ptr @__ncc_kernel_debug_active_idtr32)
  call void asm sideeffect "int3", "~{memory}"()
  call void asm sideeffect ".byte 0x0f,0x01,0x18", "{eax},~{memory}"(ptr @__ncc_kernel_debug_saved_idtr32)
  ret i1 true
}
"#,
            );
        }
    } else if triple.starts_with("aarch64") {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_pci_config_read32(i64 %bus, i64 %device, i64 %function, i64 %offset) { ret i64 4294967295 }
define i1 @__ncc_kernel_pci_config_write32(i64 %bus, i64 %device, i64 %function, i64 %offset, i64 %value) { ret i1 false }
define i1 @__ncc_kernel_debug_break() {
entry:
  call void asm sideeffect ".inst 0xd4200000", "~{memory}"()
  ret i1 true
}
"#,
        );
    } else if triple.starts_with("riscv") {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_pci_config_read32(i64 %bus, i64 %device, i64 %function, i64 %offset) { ret i64 4294967295 }
define i1 @__ncc_kernel_pci_config_write32(i64 %bus, i64 %device, i64 %function, i64 %offset, i64 %value) { ret i1 false }
define i1 @__ncc_kernel_debug_break() {
entry:
  call void asm sideeffect ".word 0x00100073", "~{memory}"()
  ret i1 true
}
"#,
        );
    } else {
        llvm.push_str(
            r#"
define i64 @__ncc_kernel_pci_config_read32(i64 %bus, i64 %device, i64 %function, i64 %offset) { ret i64 4294967295 }
define i1 @__ncc_kernel_pci_config_write32(i64 %bus, i64 %device, i64 %function, i64 %offset, i64 %value) { ret i1 false }
define i1 @__ncc_kernel_debug_break() { ret i1 false }
"#,
        );
    }
    llvm.push_str(
        r#"

define i64 @__ncc_kernel_debug_register_snapshot(i64 %source, i64 %output, i64 %slots) {
entry:
  br label %copy.check
copy.check:
  %index = phi i64 [ 0, %entry ], [ %next, %copy.body ]
  %more = icmp ult i64 %index, %slots
  br i1 %more, label %copy.body, label %copy.done
copy.body:
  %offset = mul i64 %index, 8
  %source.address = add i64 %source, %offset
  %output.address = add i64 %output, %offset
  %source.pointer = inttoptr i64 %source.address to ptr
  %output.pointer = inttoptr i64 %output.address to ptr
  %value = load volatile i64, ptr %source.pointer, align 1
  store volatile i64 %value, ptr %output.pointer, align 1
  %next = add i64 %index, 1
  br label %copy.check
copy.done:
  ret i64 %index
}

define i64 @__ncc_kernel_debug_memory_dump(i64 %source, i64 %output, i64 %bytes) {
entry:
  br label %copy.check
copy.check:
  %index = phi i64 [ 0, %entry ], [ %next, %copy.body ]
  %more = icmp ult i64 %index, %bytes
  br i1 %more, label %copy.body, label %copy.done
copy.body:
  %source.address = add i64 %source, %index
  %output.address = add i64 %output, %index
  %source.pointer = inttoptr i64 %source.address to ptr
  %output.pointer = inttoptr i64 %output.address to ptr
  %value = load volatile i8, ptr %source.pointer, align 1
  store volatile i8 %value, ptr %output.pointer, align 1
  %next = add i64 %index, 1
  br label %copy.check
copy.done:
  ret i64 %index
}

define i1 @__ncc_kernel_debug_symbol_lookup(i64 %table, i64 %count, i64 %program_counter, i64 %output) {
entry:
  br label %lookup.check
lookup.check:
  %index = phi i64 [ 0, %entry ], [ %next, %lookup.next ]
  %more = icmp ult i64 %index, %count
  br i1 %more, label %lookup.body, label %lookup.missing
lookup.body:
  %record.offset = mul i64 %index, 32
  %record.address = add i64 %table, %record.offset
  %record.pointer = inttoptr i64 %record.address to ptr
  %start = load volatile i64, ptr %record.pointer, align 1
  %size.pointer = getelementptr i8, ptr %record.pointer, i64 8
  %size = load volatile i64, ptr %size.pointer, align 1
  %end = add i64 %start, %size
  %after.start = icmp uge i64 %program_counter, %start
  %before.end = icmp ult i64 %program_counter, %end
  %inside = and i1 %after.start, %before.end
  br i1 %inside, label %lookup.found, label %lookup.next
lookup.found:
  %output.pointer = inttoptr i64 %output to ptr
  %name.pointer.slot = getelementptr i8, ptr %record.pointer, i64 16
  %name.length.slot = getelementptr i8, ptr %record.pointer, i64 24
  %name.pointer = load volatile i64, ptr %name.pointer.slot, align 1
  %name.length = load volatile i64, ptr %name.length.slot, align 1
  store volatile i64 %start, ptr %output.pointer, align 1
  %out.size = getelementptr i8, ptr %output.pointer, i64 8
  %out.name = getelementptr i8, ptr %output.pointer, i64 16
  %out.length = getelementptr i8, ptr %output.pointer, i64 24
  store volatile i64 %size, ptr %out.size, align 1
  store volatile i64 %name.pointer, ptr %out.name, align 1
  store volatile i64 %name.length, ptr %out.length, align 1
  ret i1 true
lookup.next:
  %next = add i64 %index, 1
  br label %lookup.check
lookup.missing:
  ret i1 false
}

define i64 @__ncc_kernel_debug_rsp_checksum(i64 %data, i64 %bytes) {
entry:
  br label %checksum.check
checksum.check:
  %index = phi i64 [ 0, %entry ], [ %next, %checksum.body ]
  %sum = phi i64 [ 0, %entry ], [ %sum.next, %checksum.body ]
  %more = icmp ult i64 %index, %bytes
  br i1 %more, label %checksum.body, label %checksum.done
checksum.body:
  %address = add i64 %data, %index
  %pointer = inttoptr i64 %address to ptr
  %byte = load volatile i8, ptr %pointer, align 1
  %wide = zext i8 %byte to i64
  %sum.next = add i64 %sum, %wide
  %next = add i64 %index, 1
  br label %checksum.check
checksum.done:
  %checksum = and i64 %sum, 255
  ret i64 %checksum
}

define i64 @__ncc_kernel_debug_rsp_frame(i64 %data, i64 %bytes, i64 %output) {
entry:
  %output.pointer = inttoptr i64 %output to ptr
  store volatile i8 36, ptr %output.pointer, align 1
  br label %frame.check
frame.check:
  %index = phi i64 [ 0, %entry ], [ %next, %frame.body ]
  %more = icmp ult i64 %index, %bytes
  br i1 %more, label %frame.body, label %frame.tail
frame.body:
  %source.address = add i64 %data, %index
  %source.pointer = inttoptr i64 %source.address to ptr
  %byte = load volatile i8, ptr %source.pointer, align 1
  %output.offset = add i64 %index, 1
  %output.address = add i64 %output, %output.offset
  %destination.pointer = inttoptr i64 %output.address to ptr
  store volatile i8 %byte, ptr %destination.pointer, align 1
  %next = add i64 %index, 1
  br label %frame.check
frame.tail:
  %checksum = call i64 @__ncc_kernel_debug_rsp_checksum(i64 %data, i64 %bytes)
  %hash.offset = add i64 %bytes, 1
  %high.offset = add i64 %bytes, 2
  %low.offset = add i64 %bytes, 3
  %hash.address = add i64 %output, %hash.offset
  %high.address = add i64 %output, %high.offset
  %low.address = add i64 %output, %low.offset
  %hash.pointer = inttoptr i64 %hash.address to ptr
  %high.pointer = inttoptr i64 %high.address to ptr
  %low.pointer = inttoptr i64 %low.address to ptr
  store volatile i8 35, ptr %hash.pointer, align 1
  %high.nibble.wide = lshr i64 %checksum, 4
  %high.nibble = trunc i64 %high.nibble.wide to i8
  %low.nibble.wide = and i64 %checksum, 15
  %low.nibble = trunc i64 %low.nibble.wide to i8
  %high.alpha = icmp ugt i8 %high.nibble, 9
  %low.alpha = icmp ugt i8 %low.nibble, 9
  %high.base = select i1 %high.alpha, i8 87, i8 48
  %low.base = select i1 %low.alpha, i8 87, i8 48
  %high.character = add i8 %high.nibble, %high.base
  %low.character = add i8 %low.nibble, %low.base
  store volatile i8 %high.character, ptr %high.pointer, align 1
  store volatile i8 %low.character, ptr %low.pointer, align 1
  %total = add i64 %bytes, 4
  ret i64 %total
}

define i1 @__ncc_kernel_debug_crash_report(i64 %report, i64 %code, i64 %pc, i64 %sp, i64 %fault, i64 %flags) {
entry:
  %report.pointer = inttoptr i64 %report to ptr
  store volatile i64 3551179846199755598, ptr %report.pointer, align 1
  %code.pointer = getelementptr i8, ptr %report.pointer, i64 8
  %pc.pointer = getelementptr i8, ptr %report.pointer, i64 16
  %sp.pointer = getelementptr i8, ptr %report.pointer, i64 24
  %fault.pointer = getelementptr i8, ptr %report.pointer, i64 32
  %flags.pointer = getelementptr i8, ptr %report.pointer, i64 40
  %version.pointer = getelementptr i8, ptr %report.pointer, i64 48
  %checksum.pointer = getelementptr i8, ptr %report.pointer, i64 56
  store volatile i64 %code, ptr %code.pointer, align 1
  store volatile i64 %pc, ptr %pc.pointer, align 1
  store volatile i64 %sp, ptr %sp.pointer, align 1
  store volatile i64 %fault, ptr %fault.pointer, align 1
  store volatile i64 %flags, ptr %flags.pointer, align 1
  store volatile i64 1, ptr %version.pointer, align 1
  %checksum.0 = xor i64 3551179846199755598, %code
  %checksum.1 = xor i64 %checksum.0, %pc
  %checksum.2 = xor i64 %checksum.1, %sp
  %checksum.3 = xor i64 %checksum.2, %fault
  %checksum.4 = xor i64 %checksum.3, %flags
  %checksum.5 = xor i64 %checksum.4, 1
  store volatile i64 %checksum.5, ptr %checksum.pointer, align 1
  ret i1 true
}

define i1 @__ncc_kernel_panic_raw(i64 %code, i64 %report) {
entry:
  %written = call i1 @__ncc_kernel_debug_crash_report(i64 %report, i64 %code, i64 0, i64 0, i64 0, i64 0)
  %stopped = call i1 @__ncc_kernel_debug_break()
  %complete = and i1 %written, %stopped
  ret i1 %complete
}

define i64 @__ncc_kernel_kaslr_relocate(i64 %value, i64 %slide) {
entry:
  %relocated = add i64 %value, %slide
  ret i64 %relocated
}

define i1 @__ncc_kernel_stack_canary_write(i64 %slot, i64 %value) {
entry:
  %pointer = inttoptr i64 %slot to ptr
  store volatile i64 %value, ptr %pointer, align 1
  ret i1 true
}

define i1 @__ncc_kernel_stack_canary_check(i64 %slot, i64 %expected) {
entry:
  %pointer = inttoptr i64 %slot to ptr
  %observed = load volatile i64, ptr %pointer, align 1
  %matches = icmp eq i64 %observed, %expected
  ret i1 %matches
}

define i1 @__ncc_kernel_sanitizer_poison(i64 %shadow, i64 %index, i64 %tag) {
entry:
  %address = add i64 %shadow, %index
  %pointer = inttoptr i64 %address to ptr
  %tag.byte = trunc i64 %tag to i8
  store volatile i8 %tag.byte, ptr %pointer, align 1
  ret i1 true
}

define i1 @__ncc_kernel_sanitizer_unpoison(i64 %shadow, i64 %index) {
entry:
  %address = add i64 %shadow, %index
  %pointer = inttoptr i64 %address to ptr
  store volatile i8 0, ptr %pointer, align 1
  ret i1 true
}

define i1 @__ncc_kernel_sanitizer_check(i64 %shadow, i64 %index) {
entry:
  %address = add i64 %shadow, %index
  %pointer = inttoptr i64 %address to ptr
  %tag = load volatile i8, ptr %pointer, align 1
  %clean = icmp eq i8 %tag, 0
  ret i1 %clean
}
"#,
    );

    let (word_type, previous_offset, return_offset) =
        if triple.starts_with("i386") || triple.starts_with("i686") {
            ("i32", 0i64, 4i64)
        } else if triple.starts_with("armv7") || triple.starts_with("thumbv7") {
            ("i32", -4i64, 0i64)
        } else if triple.starts_with("riscv32") {
            ("i32", -8i64, -4i64)
        } else if triple.starts_with("riscv64") {
            ("i64", -16i64, -8i64)
        } else {
            ("i64", 0i64, 8i64)
        };
    let (wide_values, previous_value, return_value) = if word_type == "i32" {
        (
            "%previous.wide = zext i32 %previous.raw to i64\n  %return.wide = zext i32 %return.raw to i64",
            "%previous.wide",
            "%return.wide",
        )
    } else {
        ("", "%previous.raw", "%return.raw")
    };
    llvm.push_str(&format!(
        r#"

define i64 @__ncc_kernel_debug_stack_trace(i64 %frame, i64 %output, i64 %maximum) {{
entry:
  br label %walk.check
walk.check:
  %index = phi i64 [ 0, %entry ], [ %next.index, %walk.body ]
  %current = phi i64 [ %frame, %entry ], [ {previous}, %walk.body ]
  %has.frame = icmp ne i64 %current, 0
  %has.space = icmp ult i64 %index, %maximum
  %continue = and i1 %has.frame, %has.space
  br i1 %continue, label %walk.body, label %walk.done
walk.body:
  %previous.address = add i64 %current, {previous_offset}
  %return.address = add i64 %current, {return_offset}
  %previous.pointer = inttoptr i64 %previous.address to ptr
  %return.pointer = inttoptr i64 %return.address to ptr
  %previous.raw = load volatile {word_type}, ptr %previous.pointer, align 1
  %return.raw = load volatile {word_type}, ptr %return.pointer, align 1
  {wide_values}
  %output.offset = mul i64 %index, 8
  %output.address = add i64 %output, %output.offset
  %output.pointer = inttoptr i64 %output.address to ptr
  store volatile i64 {return_address}, ptr %output.pointer, align 1
  %next.index = add i64 %index, 1
  br label %walk.check
walk.done:
  ret i64 %index
}}
"#,
        previous = previous_value,
        previous_offset = previous_offset,
        return_offset = return_offset,
        word_type = word_type,
        wide_values = wide_values,
        return_address = return_value,
    ));

    let (trap_type, trap_value, trap_width, trap_address) =
        if triple.starts_with("x86_64") || triple.starts_with("i386") || triple.starts_with("i686")
        {
            ("i8", "204", 1i64, "%address")
        } else if triple.starts_with("aarch64") {
            ("i32", "-736100352", 4i64, "%address")
        } else if triple.starts_with("armv7") || triple.starts_with("thumbv7") {
            ("i16", "48640", 2i64, "%normalized.address")
        } else {
            ("i32", "1048691", 4i64, "%address")
        };
    let normalize_address = if triple.starts_with("armv7") || triple.starts_with("thumbv7") {
        "  %normalized.address = and i64 %address, -2\n"
    } else {
        ""
    };
    let instruction_sync = if triple.starts_with("aarch64") {
        "  call void asm sideeffect \"dc cvau, $0; dsb ish; ic ivau, $0; dsb ish; isb\", \"r,~{memory}\"(ptr %code.pointer)\n"
    } else if triple.starts_with("riscv") {
        "  call void asm sideeffect \"fence.i\", \"~{memory}\"()\n"
    } else {
        ""
    };
    llvm.push_str(&format!(
        r#"

define i64 @__ncc_kernel_debug_breakpoint_install(i64 %address, i64 %saved) {{
entry:
{normalize_address}  %code.pointer = inttoptr i64 {trap_address} to ptr
  %saved.pointer = inttoptr i64 %saved to ptr
  %original = load volatile {trap_type}, ptr %code.pointer, align 1
  store volatile {trap_type} %original, ptr %saved.pointer, align 1
  store volatile {trap_type} {trap_value}, ptr %code.pointer, align 1
{instruction_sync}  ret i64 {trap_width}
}}

define i1 @__ncc_kernel_debug_breakpoint_remove(i64 %address, i64 %saved, i64 %width) {{
entry:
  %width.matches = icmp eq i64 %width, {trap_width}
  br i1 %width.matches, label %restore, label %invalid
restore:
{normalize_address}  %code.pointer = inttoptr i64 {trap_address} to ptr
  %saved.pointer = inttoptr i64 %saved to ptr
  %original = load volatile {trap_type}, ptr %saved.pointer, align 1
  store volatile {trap_type} %original, ptr %code.pointer, align 1
{instruction_sync}  ret i1 true
invalid:
  ret i1 false
}}
"#,
        normalize_address = normalize_address,
        trap_address = trap_address,
        trap_type = trap_type,
        trap_value = trap_value,
        instruction_sync = instruction_sync,
        trap_width = trap_width,
    ));
    llvm
}

fn render_kernel_arch_runtime(target_triple: &str) -> &'static str {
    let triple = target_triple.to_ascii_lowercase();
    if triple.starts_with("x86_64") || triple.starts_with("i386") || triple.starts_with("i686") {
        return r#"
declare i64 @llvm.readcyclecounter()

define i1 @__ncc_kernel_arch_capability(i64 %id) {
entry:
  %low = icmp uge i64 %id, 1
  %high = icmp ule i64 %id, 9
  %supported = and i1 %low, %high
  ret i1 %supported
}

define i1 @__ncc_kernel_arch_cpu_relax() {
entry:
  call void asm sideeffect "pause", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_memory_barrier() {
entry:
  call void asm sideeffect "mfence", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_instruction_barrier() {
entry:
  call void asm sideeffect "lfence", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_enable() {
entry:
  call void asm sideeffect "sti", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_disable() {
entry:
  call void asm sideeffect "cli", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_halt() {
entry:
  call void asm sideeffect "hlt", "~{memory}"()
  ret i1 true
}

define i64 @__ncc_kernel_arch_cycle_counter() {
entry:
  %cycles = call i64 @llvm.readcyclecounter()
  ret i64 %cycles
}
"#;
    }
    if triple.starts_with("armv7a") {
        return r#"
define i1 @__ncc_kernel_arch_capability(i64 %id) {
entry:
  %low = icmp uge i64 %id, 1
  %high = icmp ule i64 %id, 9
  %supported = and i1 %low, %high
  ret i1 %supported
}

define i1 @__ncc_kernel_arch_cpu_relax() {
entry:
  call void asm sideeffect "yield", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_memory_barrier() {
entry:
  call void asm sideeffect "dmb sy", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_instruction_barrier() {
entry:
  call void asm sideeffect "isb sy", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_enable() {
entry:
  call void asm sideeffect "cpsie i", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_disable() {
entry:
  call void asm sideeffect "cpsid i", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_halt() {
entry:
  call void asm sideeffect "wfi", "~{memory}"()
  ret i1 true
}

define i64 @__ncc_kernel_arch_cycle_counter() {
entry:
  %cycles32 = call i32 asm sideeffect "mrc p15, 0, $0, c9, c13, 0", "=r"()
  %cycles = zext i32 %cycles32 to i64
  ret i64 %cycles
}
"#;
    }
    if triple.starts_with("thumbv7") || triple.starts_with("armv7em") {
        return r#"
define i1 @__ncc_kernel_arch_capability(i64 %id) {
entry:
  %low = icmp uge i64 %id, 1
  %high = icmp ule i64 %id, 9
  %supported = and i1 %low, %high
  ret i1 %supported
}

define i1 @__ncc_kernel_arch_cpu_relax() {
entry:
  call void asm sideeffect "yield", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_memory_barrier() {
entry:
  call void asm sideeffect "dmb sy", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_instruction_barrier() {
entry:
  call void asm sideeffect "isb sy", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_enable() {
entry:
  call void asm sideeffect "cpsie i", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_disable() {
entry:
  call void asm sideeffect "cpsid i", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_halt() {
entry:
  call void asm sideeffect "wfi", "~{memory}"()
  ret i1 true
}

define i64 @__ncc_kernel_arch_cycle_counter() {
entry:
  %dwt = inttoptr i32 -536866812 to ptr
  %cycles32 = load volatile i32, ptr %dwt, align 4
  %cycles = zext i32 %cycles32 to i64
  ret i64 %cycles
}
"#;
    }
    if triple.starts_with("aarch64") {
        return r#"
declare i64 @llvm.readcyclecounter()

define i1 @__ncc_kernel_arch_capability(i64 %id) {
entry:
  %low = icmp uge i64 %id, 1
  %high = icmp ule i64 %id, 9
  %supported = and i1 %low, %high
  ret i1 %supported
}

define i1 @__ncc_kernel_arch_cpu_relax() {
entry:
  call void asm sideeffect "yield", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_memory_barrier() {
entry:
  call void asm sideeffect "dmb ish", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_instruction_barrier() {
entry:
  call void asm sideeffect "isb", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_enable() {
entry:
  call void asm sideeffect "msr daifclr, #2", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_disable() {
entry:
  call void asm sideeffect "msr daifset, #2", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_halt() {
entry:
  call void asm sideeffect "wfi", "~{memory}"()
  ret i1 true
}

define i64 @__ncc_kernel_arch_cycle_counter() {
entry:
  %cycles = call i64 @llvm.readcyclecounter()
  ret i64 %cycles
}
"#;
    }
    if triple.starts_with("riscv64") || triple.starts_with("riscv32") {
        return r#"
declare i64 @llvm.readcyclecounter()

define i1 @__ncc_kernel_arch_capability(i64 %id) {
entry:
  %low = icmp uge i64 %id, 1
  %high = icmp ule i64 %id, 9
  %supported = and i1 %low, %high
  ret i1 %supported
}

define i1 @__ncc_kernel_arch_cpu_relax() {
entry:
  call void asm sideeffect ".word 0x0100000f", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_memory_barrier() {
entry:
  call void asm sideeffect ".word 0x0ff0000f", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_instruction_barrier() {
entry:
  call void asm sideeffect ".word 0x0000100f", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_enable() {
entry:
  call void asm sideeffect ".word 0x10016073", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_interrupt_disable() {
entry:
  call void asm sideeffect ".word 0x10017073", "~{memory}"()
  ret i1 true
}

define i1 @__ncc_kernel_arch_halt() {
entry:
  call void asm sideeffect ".word 0x10500073", "~{memory}"()
  ret i1 true
}

define i64 @__ncc_kernel_arch_cycle_counter() {
entry:
  %cycles = call i64 @llvm.readcyclecounter()
  ret i64 %cycles
}
"#;
    }
    r#"
define i1 @__ncc_kernel_arch_capability(i64 %id) {
entry:
  ret i1 false
}

define i1 @__ncc_kernel_arch_cpu_relax() {
entry:
  ret i1 false
}

define i1 @__ncc_kernel_arch_memory_barrier() {
entry:
  ret i1 false
}

define i1 @__ncc_kernel_arch_instruction_barrier() {
entry:
  ret i1 false
}

define i1 @__ncc_kernel_arch_interrupt_enable() {
entry:
  ret i1 false
}

define i1 @__ncc_kernel_arch_interrupt_disable() {
entry:
  ret i1 false
}

define i1 @__ncc_kernel_arch_halt() {
entry:
  ret i1 false
}

define i64 @__ncc_kernel_arch_cycle_counter() {
entry:
  ret i64 0
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operational_profile_exposes_native_signatures_and_runtime_bodies() {
        for name in [
            "taskSpawn",
            "schedulerTick",
            "cpuRegister",
            "cpuLocalGet",
            "ipiBroadcast",
            "schedulerNextOn",
            "frameAlloc",
            "pageTranslate",
            "irqDispatch",
            "syscallInvoke",
            "driverWrite",
            "copyFromUser",
            "kernelBootInfo",
            "cpu_relax",
            "memory_barrier",
            "instruction_barrier",
            "interrupt_enable",
            "interrupt_disable",
            "cpu_halt",
            "cycle_counter",
            "atomic32_load",
            "atomic32_store",
            "atomic32_exchange",
            "atomic32_compare_exchange",
            "atomic32_fetch_add",
            "atomic32_fetch_sub",
            "atomic32_fetch_and",
            "atomic32_fetch_or",
            "atomic32_fetch_xor",
            "atomic64_load",
            "atomic64_store",
            "atomic64_exchange",
            "atomic64_compare_exchange",
            "atomic64_fetch_add",
            "atomic64_fetch_sub",
            "atomic64_fetch_and",
            "atomic64_fetch_or",
            "atomic64_fetch_xor",
            "atomic_fence_acquire",
            "atomic_fence_release",
            "atomic_fence_seq_cst",
            "spin_try_lock",
            "spin_lock",
            "spin_unlock",
            "mmio_read8",
            "mmio_read16",
            "mmio_read32",
            "mmio_read64",
            "mmio_read128",
            "mmio_read256",
            "mmio_read512",
            "mmio_write8",
            "mmio_write16",
            "mmio_write32",
            "mmio_write64",
            "mmio_write128",
            "mmio_write256",
            "mmio_write512",
            "apic_read32",
            "apic_write32",
            "apic_eoi",
            "apic_send_ipi",
            "x2apic_read",
            "x2apic_write",
            "ioapic_read",
            "ioapic_write",
            "hpet_read64",
            "hpet_write64",
            "tsc_read",
            "tsc_deadline_write",
            "read_barrier",
            "write_barrier",
            "memcpy",
            "memset",
            "memmove",
            "memcmp",
            "vmx_on",
            "vmx_off",
            "vmcs_clear",
            "vmcs_load",
            "vmcs_store",
            "vmcs_read",
            "vmcs_write",
            "vm_invalidate_ept",
            "vm_invalidate_vpid",
            "vm_launch",
            "vm_resume",
            "svm_run",
            "svm_load",
            "svm_save",
            "el2_read_hcr",
            "el2_write_hcr",
            "el2_write_vtcr",
            "el2_write_vttbr",
            "el2_tlbi_all",
            "el2_hvc",
            "el2_exit_snapshot",
            "riscv_h_read_hstatus",
            "riscv_h_write_hstatus",
            "riscv_h_write_hgatp",
            "riscv_h_hfence_gvma",
            "riscv_h_hfence_vvma",
            "riscv_h_hlv",
            "riscv_h_hsv",
            "riscv_h_exit_snapshot",
            "iommu_map_entry",
            "iommu_map_entry128",
            "iommu_unmap_entry",
            "iommu_invalidate",
            "iommu_command_write",
            "iommu_fault_copy",
            "dma_descriptor_write",
            "dma_pin_pages",
            "dma_unpin_pages",
            "dma_sg_write",
            "dma_bounce_to_device",
            "dma_bounce_from_device",
            "dma_cache_clean",
            "dma_cache_invalidate",
            "dma_cache_flush",
            "dma_fence",
            "pci_config_read32",
            "pci_config_write32",
            "pci_ecam_read32",
            "pci_ecam_write32",
            "virtio_notify",
            "debug_break",
            "debug_register_snapshot",
            "debug_memory_dump",
            "debug_stack_trace",
            "debug_symbol_lookup",
            "debug_rsp_checksum",
            "debug_rsp_frame",
            "debug_breakpoint_install",
            "debug_breakpoint_remove",
            "debug_crash_report",
            "panic_raw",
            "kaslr_relocate",
            "stack_canary_write",
            "stack_canary_check",
            "sanitizer_poison",
            "sanitizer_unpoison",
            "sanitizer_check",
        ] {
            assert!(kernel_core_signature(name).is_some(), "missing {name}");
        }
        for name in [
            "mmio_read128",
            "mmio_read256",
            "mmio_read512",
            "mmio_write128",
            "mmio_write256",
            "mmio_write512",
        ] {
            let signature = kernel_core_signature(name).expect("wide MMIO signature");
            assert_eq!(signature.params, &["i64", "i64"], "{name}");
            assert_eq!(signature.return_type, "i1", "{name}");
        }
        let llvm = render_kernel_core_runtime("x86_64-unknown-none");
        assert!(llvm.starts_with(
            "; ncc-machine-authority version=1 target=x86_64-unknown-none architecture=x86_64"
        ));
        assert!(llvm.contains("subsystems=[physical-memory,virtual-memory,elf-loader,processes"));
        assert!(llvm.contains("relocations=[R_X86_64_64,R_X86_64_PC32"));
        assert!(llvm.contains("define void @__ncc_kernel_set_boot_info"));
        assert!(llvm.contains("define i1 @__ncc_kernel_cpu_register"));
        assert!(llvm.contains("define i1 @__ncc_kernel_schedule_next_on"));
        assert!(llvm.contains("define i1 @__ncc_kernel_ipi_send"));
        assert!(llvm.contains("@__ncc_kernel_cpu_local"));
        assert!(llvm.contains("call void %fn(i64 %vector, i64 %frame)"));
        assert!(llvm.contains("call i64 %fn(i64 %a0, i64 %a1, i64 %a2)"));
        assert!(llvm.contains("load volatile i64"));
        assert!(llvm.contains("store volatile i8"));
        assert!(llvm.contains("load volatile i128"));
        assert!(llvm.contains("load volatile i256"));
        assert!(llvm.contains("load volatile i512"));
        assert!(llvm.contains("store volatile i128"));
        assert!(llvm.contains("store volatile i256"));
        assert!(llvm.contains("store volatile i512"));
        assert!(llvm.contains("%positive = icmp sgt i64 %bytes, 0"));
        assert!(llvm.contains("%safe.bytes = select i1 %positive, i64 %bytes, i64 0"));
        assert!(llvm.contains("define ptr @memcpy"));
        assert!(llvm.contains("define ptr @memmove"));
        assert!(llvm.contains("define i32 @memcmp"));
        assert!(llvm.contains("atomicrmw xchg"));
        assert!(llvm.contains("atomicrmw xor ptr %ptr, i64 %value acq_rel"));
        assert!(llvm.contains("define i64 @__ncc_kernel_atomic64_compare_exchange"));
        assert!(llvm.contains("fence seq_cst"));
        assert!(llvm.contains("cmpxchg ptr"));
        assert!(llvm.contains("load volatile i8"));
        assert!(llvm.contains("store volatile i64"));
        assert!(llvm.contains("define i1 @__ncc_kernel_apic_eoi"));
        assert!(llvm.contains("%high.address = add i64 %base, 784"));
        assert!(llvm.contains("%low.address = add i64 %base, 768"));
        assert!(llvm.contains("define i64 @__ncc_kernel_ioapic_read"));
        assert!(llvm.contains("%window = add i64 %base, 16"));
        assert!(llvm.contains("call i1 @__ncc_kernel_cpu_write_msr(i64 1760"));
        assert!(llvm.contains("asm sideeffect \"pause\""));
        assert!(llvm.contains("@llvm.readcyclecounter"));
        assert!(llvm.contains(".byte 0xf3,0x0f,0xc7,0x30; pushfq; popq $0"));
        assert!(llvm.contains(".byte 0x0f,0x78,0x0a; pushfq; popq $0"));
        assert!(llvm.contains(".byte 0x0f,0x79,0xd1; pushfq; popq $0"));
        assert!(llvm.contains(".byte 0x66,0x0f,0x38,0x80,0x0a; pushfq; popq $0"));
        assert!(llvm.contains(".byte 0x66,0x0f,0x38,0x81,0x0a; pushfq; popq $0"));
        assert!(llvm.contains("%status.bits = and i64 %rflags, 65"));
        assert!(llvm.contains(".byte 0x0f,0x01,0xd8"));
        assert!(llvm.contains(".byte 0x0f,0x01,0xda"));
        assert!(llvm.contains(".byte 0x0f,0x01,0xdb"));
        assert!(llvm.contains("define i64 @__ncc_kernel_pci_config_read32"));
        assert!(llvm.contains("call i1 @__ncc_kernel_port_write32(i64 3320"));
        assert!(llvm.contains("define i64 @__ncc_kernel_pci_ecam_read32"));
        assert!(llvm.contains("%bus.shifted = shl i64 %bus.masked, 20"));
        assert!(llvm.contains("define i1 @__ncc_kernel_virtio_notify"));
        assert!(llvm.contains("asm sideeffect \"int3\""));
        assert!(llvm.contains("@__ncc_kernel_debug_idt = internal global [256 x i128]"));
        assert!(llvm.contains("define void @__ncc_kernel_debug_break_handler() naked nounwind"));
        assert!(llvm.contains("asm sideeffect \"iretq\""));
        assert!(llvm.contains(".byte 0x0f,0x01,0x08"));
        assert_eq!(llvm.matches(".byte 0x0f,0x01,0x18").count(), 2);
        assert!(llvm.contains("define i64 @__ncc_kernel_debug_register_snapshot"));
        assert!(llvm.contains("define i64 @__ncc_kernel_debug_memory_dump"));
        assert!(llvm.contains("define i64 @__ncc_kernel_debug_stack_trace"));
        assert!(llvm.contains("define i1 @__ncc_kernel_debug_symbol_lookup"));
        assert!(llvm.contains("define i64 @__ncc_kernel_debug_rsp_frame"));
        assert!(llvm.contains("define i64 @__ncc_kernel_debug_breakpoint_install"));
        assert!(llvm.contains("store volatile i8 204, ptr %code.pointer"));
        assert!(llvm.contains("define i1 @__ncc_kernel_debug_crash_report"));
        assert!(llvm.contains("store volatile i64 3551179846199755598"));
        assert!(llvm.contains("define i64 @__ncc_kernel_kaslr_relocate"));
        assert!(llvm.contains("define i1 @__ncc_kernel_stack_canary_check"));
        assert!(llvm.contains("define i1 @__ncc_kernel_sanitizer_check"));
        let i386_runtime = render_kernel_core_runtime("i386-unknown-none-elf");
        assert!(i386_runtime.contains("@__ncc_kernel_debug_idt32 = internal global [256 x i64]"));
        assert!(i386_runtime.contains("call void asm sideeffect \".byte 0xcf\""));
        assert!(i386_runtime.contains("\"{eax},~{memory}\""));
        assert!(!i386_runtime.contains("iretq"));
        assert!(!i386_runtime.contains("{rax}"));
        assert!(
            render_kernel_core_runtime("aarch64-unknown-none").contains("asm sideeffect \"wfi\"")
        );
        assert!(render_kernel_core_runtime("aarch64-unknown-none")
            .contains("asm sideeffect \".inst 0xd4200000\""));
        assert!(render_kernel_core_runtime("riscv64-unknown-elf").contains(".word 0x10500073"));
        assert!(render_kernel_core_runtime("riscv64-unknown-elf")
            .contains("asm sideeffect \".word 0x00100073\""));
        let wasm_runtime = render_kernel_core_runtime("wasm32-unknown-unknown");
        assert!(wasm_runtime.starts_with(
            "; ncc-machine-authority status=not-applicable target=wasm32-unknown-unknown"
        ));
        assert!(wasm_runtime.contains("ret i1 false"));
        assert!(wasm_runtime.contains(
            "define internal i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry, i64 %context) { ret i1 false }"
        ));
        let arm_runtime = render_kernel_core_runtime("aarch64-unknown-none");
        assert!(arm_runtime.contains("define i1 @__ncc_kernel_vmx_on(i64 %physical_slot)"));
        assert!(!arm_runtime.contains("call void @llvm.trap(); unreachable }"));
        let arm = render_kernel_core_runtime("aarch64-unknown-none");
        assert!(arm.contains("mrs $0, hcr_el2"));
        assert!(arm.contains("msr vtcr_el2, $0; isb"));
        assert!(arm.contains("tlbi vmalls12e1is"));
        assert!(arm.contains("hvc #0"));
        assert!(arm.contains("mrs $0, hpfar_el2"));
        let rv64 = render_kernel_core_runtime("riscv64-unknown-elf");
        assert!(rv64.contains(".word 0x60002573"));
        assert!(rv64.contains(".word 0x62b50073"));
        assert!(rv64.contains(".word 0x6c05c573"));
        assert!(rv64.contains(".word 0x6ea5c073"));
        let rv32 = render_kernel_core_runtime("riscv32imac-unknown-none-elf");
        assert!(rv32.contains(".word 0x6805c573"));
        assert!(rv32.contains(".word 0x6aa5c073"));
        assert!(rv32.contains("@__ncc_kernel_rv32_atomic64_lock"));
        assert!(rv32.contains("define internal void @__ncc_kernel_rv32_atomic64_acquire"));
        assert!(rv32.contains("define i64 @__atomic_load_8"));
        assert!(rv32.contains("define void @__atomic_store_8"));
        assert!(rv32.contains("define i64 @__atomic_exchange_8"));
        assert!(rv32.contains("define i64 @__atomic_fetch_add_8"));
        assert!(rv32.contains("define i64 @__atomic_fetch_and_8"));
        assert!(rv32.contains("define i64 @__atomic_fetch_or_8"));
        assert!(rv32.contains("atomicrmw add ptr @__ncc_kernel_tlb_generation, i64 1 seq_cst"));
        let armv7 = render_kernel_core_runtime("armv7a-none-eabi");
        assert!(
            armv7.starts_with("; ncc-machine-authority version=1 target=armv7a-unknown-none-eabi")
        );
        assert!(armv7.contains("define { i32, i32 } @__aeabi_uidivmod"));
        assert!(armv7.contains("asm sideeffect \"mrc p15, 0, $0, c9, c13, 0\""));
        let thumb = render_kernel_core_runtime("thumbv7em-none-eabi");
        assert!(thumb.contains("@__ncc_arm_atomic64_lock"));
        assert!(thumb.contains("define i1 @__atomic_compare_exchange_8"));
        assert!(thumb.contains("define i64 @__atomic_fetch_xor_8"));
        assert!(thumb.contains("asm sideeffect \"cpsid i\""));
        assert!(thumb.contains("inttoptr i32 -536866812 to ptr"));
        assert!(llvm.contains("define i1 @__ncc_kernel_iommu_map_entry"));
        assert!(llvm.contains("define i1 @__ncc_kernel_dma_descriptor_write"));
        assert!(llvm.contains("define i1 @__ncc_kernel_dma_pin_pages"));
        assert!(llvm.contains("define i1 @__ncc_kernel_dma_unpin_pages"));
        assert!(llvm.contains("define i1 @__ncc_kernel_dma_sg_write"));
        assert!(llvm.contains("define i1 @__ncc_kernel_dma_bounce_to_device"));
        assert!(llvm.contains("define internal i1 @__ncc_kernel_dma_cache_range"));
        assert!(llvm.contains("define i1 @__ncc_kernel_iommu_command_write"));
        assert!(llvm.contains("define i1 @__ncc_kernel_iommu_fault_copy"));
        assert!(llvm.contains(".byte 0x0f,0xae,0x38"));
        assert!(arm.contains("dc cvac, $0"));
        assert!(arm.contains("dc ivac, $0"));
        assert!(arm.contains("dc civac, $0"));
        assert!(rv64.contains(".word 0x0015200f"));
        assert!(rv64.contains(".word 0x0005200f"));
        assert!(rv64.contains(".word 0x0025200f"));
        assert!(llvm.contains("store volatile i64 %permissions"));
    }
}
