use std::fmt::Write as _;

const COMMON_RUNTIME: &str = r#"
@__ncc_kernel_physical_cpu_count = internal global i64 0, align 8
@__ncc_kernel_physical_smp_discovered = internal global i1 false, align 1
@__ncc_kernel_physical_cpu_used = internal global [256 x i1] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_hardware_id = internal global [256 x i64] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_firmware_id = internal global [256 x i64] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_numa = internal global [256 x i32] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_package = internal global [256 x i16] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_core = internal global [256 x i16] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_thread = internal global [256 x i16] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_state = internal global [256 x i8] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_stack_top = internal global [256 x i64] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_entry = internal global [256 x i64] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_context = internal global [256 x i64] zeroinitializer, align 16
@__ncc_kernel_physical_cpu_ipi_vector = internal global [256 x i64] zeroinitializer, align 16

@__ncc_kernel_smp_primary_base = internal global i64 0, align 8
@__ncc_kernel_smp_secondary_base = internal global i64 0, align 8
@__ncc_kernel_smp_tertiary_base = internal global i64 0, align 8
@__ncc_kernel_smp_timer_hz = internal global i64 0, align 8
@__ncc_kernel_smp_firmware_mode = internal global i64 0, align 8
@__ncc_kernel_smp_interrupt_model = internal global i64 0, align 8
@__ncc_kernel_smp_redistributor_stride = internal global i64 131072, align 8
@__ncc_kernel_smp_context_stride = internal global i64 4096, align 8
@__ncc_kernel_smp_source_count = internal global i64 0, align 8
@__ncc_kernel_smp_context_count = internal global i64 0, align 8
@__ncc_kernel_smp_feature_flags = internal global i64 0, align 8
@__ncc_kernel_riscv_imsic_base = internal global i64 0, align 8

@__ncc_kernel_tlb_generation = internal global i64 0, align 8
@__ncc_kernel_tlb_address = internal global i64 0, align 8
@__ncc_kernel_tlb_length = internal global i64 0, align 8
@__ncc_kernel_tlb_asid = internal global i64 0, align 8
@__ncc_kernel_tlb_pending = internal global [4 x i64] zeroinitializer, align 16
@__ncc_kernel_tlb_seen_generation = internal global [256 x i64] zeroinitializer, align 16
@__ncc_kernel_tlb_lock = internal global i8 0, align 1

define internal ptr @__ncc_kernel_boot_info_header() {
entry:
  %address = load i64, ptr @__ncc_kernel_boot_info_slot, align 8
  %present = icmp ne i64 %address, 0
  br i1 %present, label %inspect, label %fail
inspect:
  %header = inttoptr i64 %address to ptr
  %magic = load i64, ptr %header, align 8
  %version.ptr = getelementptr i8, ptr %header, i64 8
  %version = load i64, ptr %version.ptr, align 8
  %magic.ok = icmp eq i64 %magic, 13879475691537230
  %version.ok = icmp eq i64 %version, 1
  %valid = and i1 %magic.ok, %version.ok
  br i1 %valid, label %hit, label %fail
hit:
  ret ptr %header
fail:
  ret ptr null
}

define i1 @__ncc_kernel_boot_valid() {
entry:
  %firmware.ok = call i1 @__ncc_kernel_firmware_adapt()
  br i1 %firmware.ok, label %boot.header, label %fail
boot.header:
  %header = call ptr @__ncc_kernel_boot_info_header()
  %base.ok = icmp ne ptr %header, null
  br i1 %base.ok, label %inspect, label %fail
inspect:
  %flags.ptr = getelementptr i8, ptr %header, i64 16
  %flags = load i64, ptr %flags.ptr, align 8
  %mmap.ptr.ptr = getelementptr i8, ptr %header, i64 24
  %mmap.ptr = load i64, ptr %mmap.ptr.ptr, align 8
  %mmap.len.ptr = getelementptr i8, ptr %header, i64 32
  %mmap.len = load i64, ptr %mmap.len.ptr, align 8
  %fb.ptr.ptr = getelementptr i8, ptr %header, i64 40
  %fb.ptr = load i64, ptr %fb.ptr.ptr, align 8
  %serial.ptr.ptr = getelementptr i8, ptr %header, i64 48
  %serial.ptr = load i64, ptr %serial.ptr.ptr, align 8
  %platform.ptr.ptr = getelementptr i8, ptr %header, i64 56
  %platform.ptr = load i64, ptr %platform.ptr.ptr, align 8
  %mmap.flag.raw = and i64 %flags, 1
  %mmap.flag = icmp ne i64 %mmap.flag.raw, 0
  %mmap.ptr.ok = icmp ne i64 %mmap.ptr, 0
  %mmap.len.ok = icmp ne i64 %mmap.len, 0
  %mmap.payload.ok = and i1 %mmap.ptr.ok, %mmap.len.ok
  %mmap.not.flag = xor i1 %mmap.flag, true
  %mmap.ok = or i1 %mmap.not.flag, %mmap.payload.ok
  %serial.flag.raw = and i64 %flags, 2
  %serial.flag = icmp ne i64 %serial.flag.raw, 0
  %serial.ptr.ok = icmp ne i64 %serial.ptr, 0
  %serial.not.flag = xor i1 %serial.flag, true
  %serial.ok = or i1 %serial.not.flag, %serial.ptr.ok
  %platform.flag.raw = and i64 %flags, 4
  %platform.flag = icmp ne i64 %platform.flag.raw, 0
  %platform.ptr.ok = icmp ne i64 %platform.ptr, 0
  %platform.not.flag = xor i1 %platform.flag, true
  %platform.ok = or i1 %platform.not.flag, %platform.ptr.ok
  %fb.flag.raw = and i64 %flags, 8
  %fb.flag = icmp ne i64 %fb.flag.raw, 0
  %fb.ptr.ok = icmp ne i64 %fb.ptr, 0
  %fb.not.flag = xor i1 %fb.flag, true
  %fb.ok = or i1 %fb.not.flag, %fb.ptr.ok
  %a = and i1 %mmap.ok, %serial.ok
  %b = and i1 %platform.ok, %fb.ok
  %valid = and i1 %a, %b
  ret i1 %valid
fail:
  ret i1 false
}

define i64 @__ncc_kernel_boot_flags() {
entry:
  %valid = call i1 @__ncc_kernel_boot_valid()
  br i1 %valid, label %read, label %fail
read:
  %header = call ptr @__ncc_kernel_boot_info_header()
  %ptr = getelementptr i8, ptr %header, i64 16
  %flags = load i64, ptr %ptr, align 8
  ret i64 %flags
fail:
  ret i64 0
}

define internal ptr @__ncc_kernel_boot_memory_entry(i64 %index) {
entry:
  %valid = call i1 @__ncc_kernel_boot_valid()
  br i1 %valid, label %bounds, label %fail
bounds:
  %header = call ptr @__ncc_kernel_boot_info_header()
  %map.ptr.ptr = getelementptr i8, ptr %header, i64 24
  %map.address = load i64, ptr %map.ptr.ptr, align 8
  %count.ptr = getelementptr i8, ptr %header, i64 32
  %count = load i64, ptr %count.ptr, align 8
  %nonnegative = icmp sge i64 %index, 0
  %inside = icmp ult i64 %index, %count
  %ok = and i1 %nonnegative, %inside
  br i1 %ok, label %address, label %fail
address:
  %map = inttoptr i64 %map.address to ptr
  %offset = mul i64 %index, 32
  %entry.ptr = getelementptr i8, ptr %map, i64 %offset
  ret ptr %entry.ptr
fail:
  ret ptr null
}

define i64 @__ncc_kernel_boot_memory_count() {
entry:
  %valid = call i1 @__ncc_kernel_boot_valid()
  br i1 %valid, label %read, label %fail
read:
  %header = call ptr @__ncc_kernel_boot_info_header()
  %ptr = getelementptr i8, ptr %header, i64 32
  %count = load i64, ptr %ptr, align 8
  ret i64 %count
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_memory_base(i64 %index) {
entry:
  %entry.ptr = call ptr @__ncc_kernel_boot_memory_entry(i64 %index)
  %ok = icmp ne ptr %entry.ptr, null
  br i1 %ok, label %read, label %fail
read:
  %value = load i64, ptr %entry.ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_memory_size(i64 %index) {
entry:
  %entry.ptr = call ptr @__ncc_kernel_boot_memory_entry(i64 %index)
  %ok = icmp ne ptr %entry.ptr, null
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_memory_kind(i64 %index) {
entry:
  %entry.ptr = call ptr @__ncc_kernel_boot_memory_entry(i64 %index)
  %ok = icmp ne ptr %entry.ptr, null
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr i8, ptr %entry.ptr, i64 16
  %raw = load i32, ptr %ptr, align 4
  %value = zext i32 %raw to i64
  ret i64 %value
fail:
  ret i64 0
}

define internal ptr @__ncc_kernel_boot_framebuffer_header() {
entry:
  %flags = call i64 @__ncc_kernel_boot_flags()
  %flag = and i64 %flags, 8
  %present = icmp ne i64 %flag, 0
  br i1 %present, label %read, label %fail
read:
  %header = call ptr @__ncc_kernel_boot_info_header()
  %ptr = getelementptr i8, ptr %header, i64 40
  %value = load i64, ptr %ptr, align 8
  %value.ptr = inttoptr i64 %value to ptr
  ret ptr %value.ptr
fail:
  ret ptr null
}

define i64 @__ncc_kernel_boot_framebuffer() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %value = ptrtoint ptr %header to i64
  ret i64 %value
}

define i64 @__ncc_kernel_boot_framebuffer_base() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %present = icmp ne ptr %header, null
  br i1 %present, label %read, label %fail
read:
  %value = load i64, ptr %header, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_framebuffer_width() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %present = icmp ne ptr %header, null
  br i1 %present, label %read, label %fail
read:
  %slot = getelementptr i8, ptr %header, i64 8
  %raw = load i32, ptr %slot, align 4
  %value = zext i32 %raw to i64
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_framebuffer_height() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %present = icmp ne ptr %header, null
  br i1 %present, label %read, label %fail
read:
  %slot = getelementptr i8, ptr %header, i64 12
  %raw = load i32, ptr %slot, align 4
  %value = zext i32 %raw to i64
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_framebuffer_pitch() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %present = icmp ne ptr %header, null
  br i1 %present, label %read, label %fail
read:
  %slot = getelementptr i8, ptr %header, i64 16
  %raw = load i32, ptr %slot, align 4
  %value = zext i32 %raw to i64
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_framebuffer_format() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %present = icmp ne ptr %header, null
  br i1 %present, label %read, label %fail
read:
  %slot = getelementptr i8, ptr %header, i64 20
  %raw = load i32, ptr %slot, align 4
  %value = zext i32 %raw to i64
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_framebuffer_flags() {
entry:
  %header = call ptr @__ncc_kernel_boot_framebuffer_header()
  %present = icmp ne ptr %header, null
  br i1 %present, label %read, label %fail
read:
  %slot = getelementptr i8, ptr %header, i64 24
  %value = load i64, ptr %slot, align 8
  ret i64 %value
fail:
  ret i64 0
}

define internal ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 %kind) {
entry:
  %table.ptr.ptr = getelementptr i8, ptr %boot, i64 56
  %table.address = load i64, ptr %table.ptr.ptr, align 8
  %present = icmp ne i64 %table.address, 0
  br i1 %present, label %table, label %fail
table:
  %table.ptr = inttoptr i64 %table.address to ptr
  %magic = load i64, ptr %table.ptr, align 8
  %version.ptr = getelementptr i8, ptr %table.ptr, i64 8
  %version = load i32, ptr %version.ptr, align 4
  %count.ptr = getelementptr i8, ptr %table.ptr, i64 12
  %count = load i32, ptr %count.ptr, align 4
  %magic.ok = icmp eq i64 %magic, 13884960599655246
  %version.ok = icmp eq i32 %version, 1
  %count.ok = icmp ule i32 %count, 4096
  %a = and i1 %magic.ok, %version.ok
  %ok = and i1 %a, %count.ok
  br i1 %ok, label %loop, label %fail
loop:
  %i = phi i32 [ 0, %table ], [ %next, %miss ]
  %done = icmp uge i32 %i, %count
  br i1 %done, label %fail, label %inspect
inspect:
  %wide = zext i32 %i to i64
  %offset.body = mul i64 %wide, 32
  %offset = add i64 %offset.body, 16
  %entry.ptr = getelementptr i8, ptr %table.ptr, i64 %offset
  %candidate.raw = load i32, ptr %entry.ptr, align 4
  %candidate = zext i32 %candidate.raw to i64
  %same = icmp eq i64 %candidate, %kind
  br i1 %same, label %hit, label %miss
miss:
  %next = add nuw i32 %i, 1
  br label %loop
hit:
  ret ptr %entry.ptr
fail:
  ret ptr null
}

define internal ptr @__ncc_kernel_boot_platform_entry(i64 %kind) {
entry:
  %valid = call i1 @__ncc_kernel_boot_valid()
  br i1 %valid, label %header, label %fail
header:
  %boot = call ptr @__ncc_kernel_boot_info_header()
  %entry.ptr = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 %kind)
  ret ptr %entry.ptr
fail:
  ret ptr null
}

define i64 @__ncc_kernel_boot_platform(i64 %kind) {
entry:
  %entry.ptr = call ptr @__ncc_kernel_boot_platform_entry(i64 %kind)
  %ok = icmp ne ptr %entry.ptr, null
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_platform_size(i64 %kind) {
entry:
  %entry.ptr = call ptr @__ncc_kernel_boot_platform_entry(i64 %kind)
  %ok = icmp ne ptr %entry.ptr, null
  br i1 %ok, label %read, label %fail
read:
  %ptr = getelementptr i8, ptr %entry.ptr, i64 16
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_boot_command_line() {
entry:
  %valid = call i1 @__ncc_kernel_boot_valid()
  br i1 %valid, label %read, label %fail
read:
  %header = call ptr @__ncc_kernel_boot_info_header()
  %ptr = getelementptr i8, ptr %header, i64 64
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define internal void @__ncc_kernel_physical_smp_clear() {
entry:
  store i1 false, ptr @__ncc_kernel_physical_smp_discovered, align 1
  store i64 0, ptr @__ncc_kernel_physical_cpu_count, align 8
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i32 %i, 256
  br i1 %done, label %finish, label %body
body:
  %used = getelementptr [256 x i1], ptr @__ncc_kernel_physical_cpu_used, i32 0, i32 %i
  %state = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %i
  %stack = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_stack_top, i32 0, i32 %i
  %entry.slot = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_entry, i32 0, i32 %i
  %context.slot = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_context, i32 0, i32 %i
  %ipi.slot = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_ipi_vector, i32 0, i32 %i
  %seen.slot = getelementptr [256 x i64], ptr @__ncc_kernel_tlb_seen_generation, i32 0, i32 %i
  store i1 false, ptr %used, align 1
  store i8 0, ptr %state, align 1
  store i64 0, ptr %stack, align 8
  store i64 0, ptr %entry.slot, align 8
  store i64 0, ptr %context.slot, align 8
  store atomic i64 0, ptr %ipi.slot release, align 8
  store atomic i64 0, ptr %seen.slot release, align 8
  %next = add nuw i32 %i, 1
  br label %loop
finish:
  ret void
}

define internal i1 @__ncc_kernel_physical_hwid_exists(i64 %hardware) {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %continue ]
  %done = icmp uge i32 %i, 256
  br i1 %done, label %miss, label %inspect
inspect:
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_physical_cpu_used, i32 0, i32 %i
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %compare, label %continue
compare:
  %id.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %i
  %id = load i64, ptr %id.ptr, align 8
  %same = icmp eq i64 %id, %hardware
  br i1 %same, label %hit, label %continue
continue:
  %next = add nuw i32 %i, 1
  br label %loop
hit:
  ret i1 true
miss:
  ret i1 false
}

define i1 @__ncc_kernel_smp_discover() {
entry:
  %firmware.ok = call i1 @__ncc_kernel_firmware_adapt()
  br i1 %firmware.ok, label %clear, label %fail
clear:
  call void @__ncc_kernel_physical_smp_clear()
  %platform.entry = call ptr @__ncc_kernel_boot_platform_entry(i64 10)
  %has.entry = icmp ne ptr %platform.entry, null
  br i1 %has.entry, label %topology.address, label %acpi.fallback
topology.address:
  %address.ptr = getelementptr i8, ptr %platform.entry, i64 8
  %address = load i64, ptr %address.ptr, align 8
  %present = icmp ne i64 %address, 0
  br i1 %present, label %topology.explicit, label %acpi.fallback
topology.explicit:
  %explicit.topology = inttoptr i64 %address to ptr
  br label %header
acpi.fallback:
  %boot = call ptr @__ncc_kernel_boot_info_header()
  %acpi.topology = call ptr @__ncc_kernel_acpi_topology_from_boot(ptr %boot)
  %acpi.present = icmp ne ptr %acpi.topology, null
  br i1 %acpi.present, label %header, label %fail
header:
  %topology = phi ptr [ %explicit.topology, %topology.explicit ], [ %acpi.topology, %acpi.fallback ]
  %magic = load i64, ptr %topology, align 8
  %version.ptr = getelementptr i8, ptr %topology, i64 8
  %version = load i32, ptr %version.ptr, align 4
  %count.ptr = getelementptr i8, ptr %topology, i64 12
  %count = load i32, ptr %count.ptr, align 4
  %magic.ok = icmp eq i64 %magic, 13886077073048398
  %version.ok = icmp eq i32 %version, 1
  %count.nonzero = icmp ne i32 %count, 0
  %count.bounded = icmp ule i32 %count, 256
  %a = and i1 %magic.ok, %version.ok
  %b = and i1 %count.nonzero, %count.bounded
  %valid = and i1 %a, %b
  br i1 %valid, label %loop, label %fail
loop:
  %i = phi i32 [ 0, %header ], [ %next, %store ]
  %bootstrap.count = phi i32 [ 0, %header ], [ %bootstrap.next, %store ]
  %bootstrap.logical = phi i32 [ -1, %header ], [ %bootstrap.logical.next, %store ]
  %done = icmp uge i32 %i, %count
  br i1 %done, label %finish, label %inspect
inspect:
  %wide = zext i32 %i to i64
  %entry.body.offset = mul i64 %wide, 48
  %entry.offset = add i64 %entry.body.offset, 32
  %entry.ptr = getelementptr i8, ptr %topology, i64 %entry.offset
  %logical = load i32, ptr %entry.ptr, align 4
  %flags.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %flags = load i32, ptr %flags.ptr, align 4
  %hardware.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %hardware = load i64, ptr %hardware.ptr, align 8
  %logical.ok = icmp ult i32 %logical, 256
  br i1 %logical.ok, label %unique.logical, label %fail
unique.logical:
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_physical_cpu_used, i32 0, i32 %logical
  %used = load i1, ptr %used.ptr, align 1
  %unused = xor i1 %used, true
  br i1 %unused, label %unique.hardware, label %fail
unique.hardware:
  %duplicate = call i1 @__ncc_kernel_physical_hwid_exists(i64 %hardware)
  %unique = xor i1 %duplicate, true
  br i1 %unique, label %flags.check, label %fail
flags.check:
  %enabled.raw = and i32 %flags, 1
  %bootstrap.raw = and i32 %flags, 2
  %enabled = icmp ne i32 %enabled.raw, 0
  %bootstrap = icmp ne i32 %bootstrap.raw, 0
  %enabled.not = xor i1 %enabled, true
  %bootstrap.disabled = and i1 %bootstrap, %enabled.not
  br i1 %bootstrap.disabled, label %fail, label %store
store:
  %firmware.ptr.src = getelementptr i8, ptr %entry.ptr, i64 16
  %firmware = load i64, ptr %firmware.ptr.src, align 8
  %numa.ptr.src = getelementptr i8, ptr %entry.ptr, i64 24
  %numa = load i32, ptr %numa.ptr.src, align 4
  %package.ptr.src = getelementptr i8, ptr %entry.ptr, i64 28
  %package = load i16, ptr %package.ptr.src, align 2
  %core.ptr.src = getelementptr i8, ptr %entry.ptr, i64 30
  %core = load i16, ptr %core.ptr.src, align 2
  %thread.ptr.src = getelementptr i8, ptr %entry.ptr, i64 32
  %thread = load i16, ptr %thread.ptr.src, align 2
  %stack.ptr.src = getelementptr i8, ptr %entry.ptr, i64 40
  %stack = load i64, ptr %stack.ptr.src, align 8
  store i1 true, ptr %used.ptr, align 1
  %hardware.dst = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %logical
  %firmware.dst = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_firmware_id, i32 0, i32 %logical
  %numa.dst = getelementptr [256 x i32], ptr @__ncc_kernel_physical_cpu_numa, i32 0, i32 %logical
  %package.dst = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_package, i32 0, i32 %logical
  %core.dst = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_core, i32 0, i32 %logical
  %thread.dst = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_thread, i32 0, i32 %logical
  %stack.dst = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_stack_top, i32 0, i32 %logical
  %state.dst = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %logical
  store i64 %hardware, ptr %hardware.dst, align 8
  store i64 %firmware, ptr %firmware.dst, align 8
  store i32 %numa, ptr %numa.dst, align 4
  store i16 %package, ptr %package.dst, align 2
  store i16 %core, ptr %core.dst, align 2
  store i16 %thread, ptr %thread.dst, align 2
  store i64 %stack, ptr %stack.dst, align 8
  %enabled.state = select i1 %bootstrap, i8 3, i8 1
  %state = select i1 %enabled, i8 %enabled.state, i8 0
  store i8 %state, ptr %state.dst, align 1
  %bootstrap.increment = select i1 %bootstrap, i32 1, i32 0
  %bootstrap.next = add i32 %bootstrap.count, %bootstrap.increment
  %bootstrap.logical.next = select i1 %bootstrap, i32 %logical, i32 %bootstrap.logical
  %next = add nuw i32 %i, 1
  br label %loop
finish:
  %one.bootstrap = icmp eq i32 %bootstrap.count, 1
  br i1 %one.bootstrap, label %commit, label %fail
commit:
  %count.wide = zext i32 %count to i64
  store i64 %count.wide, ptr @__ncc_kernel_physical_cpu_count, align 8
  store i1 true, ptr @__ncc_kernel_physical_smp_discovered, align 1
  %bootstrap.wide = zext i32 %bootstrap.logical to i64
  call void @__ncc_kernel_arch_cpu_local_set(i64 %bootstrap.wide)
  %marked = call i1 @__ncc_kernel_cpu_mark_online(i64 %bootstrap.wide)
  ret i1 %marked
fail:
  store i1 false, ptr @__ncc_kernel_physical_smp_discovered, align 1
  ret i1 false
}

define internal i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu) {
entry:
  %discovered = load i1, ptr @__ncc_kernel_physical_smp_discovered, align 1
  %nonnegative = icmp sge i64 %cpu, 0
  %bounded = icmp ult i64 %cpu, 256
  %range.a = and i1 %discovered, %nonnegative
  %range = and i1 %range.a, %bounded
  br i1 %range, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i1], ptr @__ncc_kernel_physical_cpu_used, i32 0, i32 %idx
  %used = load i1, ptr %ptr, align 1
  ret i1 %used
fail:
  ret i1 false
}

define i64 @__ncc_kernel_cpu_current_physical() {
entry:
  %local = call i64 @__ncc_kernel_arch_cpu_local_get()
  %local.nonnegative = icmp sge i64 %local, 0
  %local.bounded = icmp ult i64 %local, 256
  %local.range = and i1 %local.nonnegative, %local.bounded
  br i1 %local.range, label %local.validate, label %hardware
local.validate:
  %local.valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %local)
  br i1 %local.valid, label %local.hit, label %hardware
local.hit:
  ret i64 %local
hardware:
  %hardware.id = call i64 @__ncc_kernel_arch_bootstrap_hardware_id()
  br label %scan
scan:
  %i = phi i32 [ 0, %hardware ], [ %next, %continue ]
  %done = icmp uge i32 %i, 256
  br i1 %done, label %fail, label %inspect
inspect:
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_physical_cpu_used, i32 0, i32 %i
  %used = load i1, ptr %used.ptr, align 1
  br i1 %used, label %compare, label %continue
compare:
  %id.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %i
  %id = load i64, ptr %id.ptr, align 8
  %same = icmp eq i64 %id, %hardware.id
  br i1 %same, label %hit, label %continue
continue:
  %next = add nuw i32 %i, 1
  br label %scan
hit:
  %logical = zext i32 %i to i64
  call void @__ncc_kernel_arch_cpu_local_set(i64 %logical)
  ret i64 %logical
fail:
  ret i64 -1
}

define i64 @__ncc_kernel_cpu_count_physical() {
entry:
  %discovered = load i1, ptr @__ncc_kernel_physical_smp_discovered, align 1
  br i1 %discovered, label %read, label %fail
read:
  %count = load i64, ptr @__ncc_kernel_physical_cpu_count, align 8
  ret i64 %count
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_hardware_id(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %idx
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_firmware_id(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_firmware_id, i32 0, i32 %idx
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define internal i64 @__ncc_kernel_cpu_topology_field(i64 %cpu, i32 %field) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %select, label %fail
select:
  %idx = trunc i64 %cpu to i32
  switch i32 %field, label %fail [
    i32 0, label %numa
    i32 1, label %package
    i32 2, label %core
    i32 3, label %thread
  ]
numa:
  %numa.ptr = getelementptr [256 x i32], ptr @__ncc_kernel_physical_cpu_numa, i32 0, i32 %idx
  %numa.raw = load i32, ptr %numa.ptr, align 4
  %numa.value = zext i32 %numa.raw to i64
  ret i64 %numa.value
package:
  %package.ptr = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_package, i32 0, i32 %idx
  %package.raw = load i16, ptr %package.ptr, align 2
  %package.value = zext i16 %package.raw to i64
  ret i64 %package.value
core:
  %core.ptr = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_core, i32 0, i32 %idx
  %core.raw = load i16, ptr %core.ptr, align 2
  %core.value = zext i16 %core.raw to i64
  ret i64 %core.value
thread:
  %thread.ptr = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_thread, i32 0, i32 %idx
  %thread.raw = load i16, ptr %thread.ptr, align 2
  %thread.value = zext i16 %thread.raw to i64
  ret i64 %thread.value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_numa_node(i64 %cpu) {
entry:
  %v = call i64 @__ncc_kernel_cpu_topology_field(i64 %cpu, i32 0)
  ret i64 %v
}

define i64 @__ncc_kernel_cpu_package_id(i64 %cpu) {
entry:
  %v = call i64 @__ncc_kernel_cpu_topology_field(i64 %cpu, i32 1)
  ret i64 %v
}

define i64 @__ncc_kernel_cpu_core_id(i64 %cpu) {
entry:
  %v = call i64 @__ncc_kernel_cpu_topology_field(i64 %cpu, i32 2)
  ret i64 %v
}

define i64 @__ncc_kernel_cpu_thread_id(i64 %cpu) {
entry:
  %v = call i64 @__ncc_kernel_cpu_topology_field(i64 %cpu, i32 3)
  ret i64 %v
}

define i64 @__ncc_kernel_cpu_state_physical(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %raw = load i8, ptr %ptr, align 1
  %value = zext i8 %raw to i64
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_stack_physical(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_stack_top, i32 0, i32 %idx
  %value = load i64, ptr %ptr, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_entry_physical(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_entry, i32 0, i32 %idx
  %value = load atomic i64, ptr %ptr acquire, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i64 @__ncc_kernel_cpu_context_physical(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %read, label %fail
read:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_context, i32 0, i32 %idx
  %value = load atomic i64, ptr %ptr acquire, align 8
  ret i64 %value
fail:
  ret i64 0
}

define i1 @__ncc_kernel_cpu_set_stack_physical(i64 %cpu, i64 %stack) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  %present = icmp ne i64 %stack, 0
  %alignment.raw = and i64 %stack, 15
  %aligned = icmp eq i64 %alignment.raw, 0
  %a = and i1 %valid, %present
  %ok = and i1 %a, %aligned
  br i1 %ok, label %store, label %fail
store:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_stack_top, i32 0, i32 %idx
  store i64 %stack, ptr %ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_smp_configure(i64 %primary, i64 %secondary, i64 %timer_hz, i64 %firmware_mode) {
entry:
  %timer.ok = icmp ne i64 %timer_hz, 0
  %mode.ok = icmp ule i64 %firmware_mode, 2
  %valid = and i1 %timer.ok, %mode.ok
  br i1 %valid, label %store, label %fail
store:
  store i64 %primary, ptr @__ncc_kernel_smp_primary_base, align 8
  store i64 %secondary, ptr @__ncc_kernel_smp_secondary_base, align 8
  store i64 %timer_hz, ptr @__ncc_kernel_smp_timer_hz, align 8
  store i64 %firmware_mode, ptr @__ncc_kernel_smp_firmware_mode, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_smp_configure_firmware() {
entry:
  %firmware.ok = call i1 @__ncc_kernel_firmware_adapt()
  br i1 %firmware.ok, label %boot, label %fail
boot:
  %boot.info = call ptr @__ncc_kernel_boot_info_header()
  %boot.ok = icmp ne ptr %boot.info, null
  br i1 %boot.ok, label %platform.entry, label %fail
platform.entry:
  %entry.ptr = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot.info, i64 25)
  %has.entry = icmp ne ptr %entry.ptr, null
  br i1 %has.entry, label %platform.address, label %acpi
platform.address:
  %address.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %address = load i64, ptr %address.ptr, align 8
  %present = icmp ne i64 %address, 0
  br i1 %present, label %explicit, label %acpi
explicit:
  %explicit.platform = inttoptr i64 %address to ptr
  br label %validate
acpi:
  %acpi.platform = call ptr @__ncc_kernel_acpi_platform_from_boot(ptr %boot.info)
  %acpi.present = icmp ne ptr %acpi.platform, null
  br i1 %acpi.present, label %validate, label %fail
validate:
  %platform = phi ptr [ %explicit.platform, %explicit ], [ %acpi.platform, %acpi ]
  %magic = load i64, ptr %platform, align 8
  %version.ptr = getelementptr i8, ptr %platform, i64 8
  %version = load i32, ptr %version.ptr, align 4
  %size.ptr = getelementptr i8, ptr %platform, i64 12
  %size = load i32, ptr %size.ptr, align 4
  %magic.ok = icmp eq i64 %magic, 13880566898443086
  %version.ok = icmp eq i32 %version, 1
  %size.ok = icmp eq i32 %size, 104
  %a = and i1 %magic.ok, %version.ok
  %header.ok = and i1 %a, %size.ok
  br i1 %header.ok, label %fields, label %fail
fields:
  %arch.ptr = getelementptr i8, ptr %platform, i64 16
  %arch = load i32, ptr %arch.ptr, align 4
  %start.ptr = getelementptr i8, ptr %platform, i64 24
  %start = load i32, ptr %start.ptr, align 4
  %model.ptr = getelementptr i8, ptr %platform, i64 28
  %model = load i32, ptr %model.ptr, align 4
  %primary.ptr = getelementptr i8, ptr %platform, i64 32
  %primary = load i64, ptr %primary.ptr, align 8
  %secondary.ptr = getelementptr i8, ptr %platform, i64 40
  %secondary = load i64, ptr %secondary.ptr, align 8
  %tertiary.ptr = getelementptr i8, ptr %platform, i64 48
  %tertiary = load i64, ptr %tertiary.ptr, align 8
  %timer.ptr = getelementptr i8, ptr %platform, i64 56
  %timer.raw = load i64, ptr %timer.ptr, align 8
  %redistributor.ptr = getelementptr i8, ptr %platform, i64 64
  %redistributor.raw = load i64, ptr %redistributor.ptr, align 8
  %context.stride.ptr = getelementptr i8, ptr %platform, i64 72
  %context.stride.raw = load i64, ptr %context.stride.ptr, align 8
  %source.ptr = getelementptr i8, ptr %platform, i64 80
  %source.raw = load i32, ptr %source.ptr, align 4
  %context.count.ptr = getelementptr i8, ptr %platform, i64 84
  %context.count.raw = load i32, ptr %context.count.ptr, align 4
  %features.ptr = getelementptr i8, ptr %platform, i64 88
  %features = load i64, ptr %features.ptr, align 8
  %timer.missing = icmp eq i64 %timer.raw, 0
  br i1 %timer.missing, label %timer.probe, label %mode.select
timer.probe:
  %timer.probed = call i64 @__ncc_kernel_arch_timer_frequency()
  br label %mode.select
mode.select:
  %timer = phi i64 [ %timer.raw, %fields ], [ %timer.probed, %timer.probe ]
  %x86 = icmp eq i32 %arch, 1
  %arm = icmp eq i32 %arch, 2
  %riscv = icmp eq i32 %arch, 3
  %known.a = or i1 %x86, %arm
  %known = or i1 %known.a, %riscv
  %x2 = icmp eq i32 %model, 2
  %hvc = icmp eq i32 %start, 3
  %x86.start = icmp eq i32 %start, 1
  %xapic = icmp eq i32 %model, 1
  %x86.model = or i1 %xapic, %x2
  %x86.pair = and i1 %x86.start, %x86.model
  %psci.smc = icmp eq i32 %start, 2
  %psci.hvc = icmp eq i32 %start, 3
  %psci = or i1 %psci.smc, %psci.hvc
  %gic = icmp eq i32 %model, 3
  %arm.pair = and i1 %psci, %gic
  %sbi = icmp eq i32 %start, 4
  %rv.model.low = icmp uge i32 %model, 4
  %rv.model.high = icmp ule i32 %model, 8
  %rv.model = and i1 %rv.model.low, %rv.model.high
  %rv.pair = and i1 %sbi, %rv.model
  %x86.valid = and i1 %x86, %x86.pair
  %arm.valid = and i1 %arm, %arm.pair
  %rv.valid = and i1 %riscv, %rv.pair
  %pair.a = or i1 %x86.valid, %arm.valid
  %pair.ok = or i1 %pair.a, %rv.valid
  %x86.mode = select i1 %x2, i64 1, i64 0
  %arm.mode = select i1 %hvc, i64 2, i64 1
  %firmware.mode = select i1 %arm, i64 %arm.mode, i64 %x86.mode
  %primary.missing = icmp eq i64 %primary, 0
  %rv.secondary = and i1 %riscv, %primary.missing
  %effective.primary = select i1 %rv.secondary, i64 %secondary, i64 %primary
  %primary.ok = icmp ne i64 %effective.primary, 0
  %timer.ok = icmp ne i64 %timer, 0
  %known.pair = and i1 %known, %pair.ok
  %b = and i1 %known.pair, %primary.ok
  %ready = and i1 %b, %timer.ok
  br i1 %ready, label %configure, label %fail
configure:
  %ok = call i1 @__ncc_kernel_smp_configure(i64 %effective.primary, i64 %secondary, i64 %timer, i64 %firmware.mode)
  br i1 %ok, label %metadata, label %fail
metadata:
  %redistributor.missing = icmp eq i64 %redistributor.raw, 0
  %redistributor = select i1 %redistributor.missing, i64 131072, i64 %redistributor.raw
  %context.stride.missing = icmp eq i64 %context.stride.raw, 0
  %context.stride = select i1 %context.stride.missing, i64 4096, i64 %context.stride.raw
  %source = zext i32 %source.raw to i64
  %context.count = zext i32 %context.count.raw to i64
  %model.wide = zext i32 %model to i64
  store i64 %tertiary, ptr @__ncc_kernel_smp_tertiary_base, align 8
  store i64 %model.wide, ptr @__ncc_kernel_smp_interrupt_model, align 8
  store i64 %redistributor, ptr @__ncc_kernel_smp_redistributor_stride, align 8
  store i64 %context.stride, ptr @__ncc_kernel_smp_context_stride, align 8
  store i64 %source, ptr @__ncc_kernel_smp_source_count, align 8
  store i64 %context.count, ptr @__ncc_kernel_smp_context_count, align 8
  store i64 %features, ptr @__ncc_kernel_smp_feature_flags, align 8
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_start_physical(i64 %cpu, i64 %entry.address, i64 %context) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %inspect, label %fail
inspect:
  %idx = trunc i64 %cpu to i32
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %state = load atomic i8, ptr %state.ptr acquire, align 1
  %offline = icmp eq i8 %state, 1
  %entry.ok = icmp ne i64 %entry.address, 0
  %stack.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_stack_top, i32 0, i32 %idx
  %stack = load i64, ptr %stack.ptr, align 8
  %stack.ok = icmp ne i64 %stack, 0
  %a = and i1 %offline, %entry.ok
  %ready = and i1 %a, %stack.ok
  br i1 %ready, label %claim, label %fail
claim:
  %transition = cmpxchg ptr %state.ptr, i8 1, i8 2 acq_rel acquire
  %claimed = extractvalue { i8, i1 } %transition, 1
  br i1 %claimed, label %start, label %fail
start:
  %hardware.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %idx
  %hardware = load i64, ptr %hardware.ptr, align 8
  %entry.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_entry, i32 0, i32 %idx
  %context.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_context, i32 0, i32 %idx
  store atomic i64 %entry.address, ptr %entry.ptr release, align 8
  store atomic i64 %context, ptr %context.ptr release, align 8
  fence release
  %started = call i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry.address, i64 %context)
  br i1 %started, label %commit, label %rollback
commit:
  ret i1 true
rollback:
  call void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu)
  %rolled = cmpxchg ptr %state.ptr, i8 2, i8 1 acq_rel acquire
  store atomic i64 0, ptr %entry.ptr release, align 8
  store atomic i64 0, ptr %context.ptr release, align 8
  ret i1 false
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_wait_online(i64 %cpu, i64 %spins) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  %budget.ok = icmp ugt i64 %spins, 0
  %ready = and i1 %valid, %budget.ok
  br i1 %ready, label %loop, label %fail
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %again ]
  %idx = trunc i64 %cpu to i32
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %state = load atomic i8, ptr %state.ptr acquire, align 1
  %online = icmp eq i8 %state, 3
  br i1 %online, label %ok, label %inspect
inspect:
  %failed = icmp eq i8 %state, 5
  %disabled = icmp eq i8 %state, 0
  %terminal = or i1 %failed, %disabled
  br i1 %terminal, label %fail, label %again
again:
  call void @__ncc_kernel_arch_smp_relax()
  %next = add nuw i64 %i, 1
  %timeout = icmp uge i64 %next, %spins
  br i1 %timeout, label %fail, label %loop
ok:
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_start_physical_wait(i64 %cpu, i64 %entry.address, i64 %context, i64 %spins) {
entry:
  %started = call i1 @__ncc_kernel_cpu_start_physical(i64 %cpu, i64 %entry.address, i64 %context)
  br i1 %started, label %wait, label %fail
wait:
  %online = call i1 @__ncc_kernel_cpu_wait_online(i64 %cpu, i64 %spins)
  br i1 %online, label %ok, label %timeout
timeout:
  call void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu)
  %idx = trunc i64 %cpu to i32
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %failed = cmpxchg ptr %state.ptr, i8 2, i8 5 acq_rel acquire
  ret i1 false
ok:
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_cpu_mark_online(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %inspect, label %fail
inspect:
  %idx = trunc i64 %cpu to i32
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %state = load i8, ptr %state.ptr, align 1
  %starting = icmp eq i8 %state, 2
  %online = icmp eq i8 %state, 3
  %allowed = or i1 %starting, %online
  br i1 %allowed, label %commit, label %fail
commit:
  %hardware.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %idx
  %hardware = load i64, ptr %hardware.ptr, align 8
  %topology = call i64 @__ncc_kernel_arch_topology_pack(i64 %hardware)
  %package.raw = and i64 %topology, 65535
  %core.shift = lshr i64 %topology, 16
  %core.raw = and i64 %core.shift, 65535
  %thread.shift = lshr i64 %topology, 32
  %thread.raw = and i64 %thread.shift, 65535
  %package = trunc i64 %package.raw to i16
  %core = trunc i64 %core.raw to i16
  %thread = trunc i64 %thread.raw to i16
  %package.ptr = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_package, i32 0, i32 %idx
  %core.ptr = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_core, i32 0, i32 %idx
  %thread.ptr = getelementptr [256 x i16], ptr @__ncc_kernel_physical_cpu_thread, i32 0, i32 %idx
  store i16 %package, ptr %package.ptr, align 2
  store i16 %core, ptr %core.ptr, align 2
  store i16 %thread, ptr %thread.ptr, align 2
  store atomic i8 3, ptr %state.ptr release, align 1
  call void @__ncc_kernel_arch_cpu_local_set(i64 %cpu)
  call void @__ncc_kernel_arch_cpu_online_commit(i64 %cpu)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_cpu_mark_parked(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %park, label %fail
park:
  %idx = trunc i64 %cpu to i32
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %transition = cmpxchg ptr %state.ptr, i8 3, i8 4 acq_rel acquire
  %parked = extractvalue { i8, i1 } %transition, 1
  ret i1 %parked
fail:
  ret i1 false
}

define i1 @__ncc_kernel_ipi_send_physical(i64 %cpu, i64 %vector) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  %vector.positive = icmp ugt i64 %vector, 0
  %vector.bounded = icmp ult i64 %vector, 256
  %vector.ok = and i1 %vector.positive, %vector.bounded
  %a = and i1 %valid, %vector.ok
  br i1 %a, label %state, label %fail
state:
  %idx = trunc i64 %cpu to i32
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %idx
  %cpu.state = load i8, ptr %state.ptr, align 1
  %online = icmp eq i8 %cpu.state, 3
  br i1 %online, label %send, label %fail
send:
  %hardware.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_hardware_id, i32 0, i32 %idx
  %hardware = load i64, ptr %hardware.ptr, align 8
  %ipi.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_ipi_vector, i32 0, i32 %idx
  store atomic i64 %vector, ptr %ipi.ptr release, align 8
  %sent = call i1 @__ncc_kernel_arch_ipi_send_physical(i64 %hardware, i64 %vector)
  br i1 %sent, label %ok, label %clear
clear:
  store atomic i64 0, ptr %ipi.ptr release, align 8
  ret i1 false
ok:
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_ipi_take_physical(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %take, label %fail
take:
  %idx = trunc i64 %cpu to i32
  %ptr = getelementptr [256 x i64], ptr @__ncc_kernel_physical_cpu_ipi_vector, i32 0, i32 %idx
  %value = atomicrmw xchg ptr %ptr, i64 0 acq_rel
  ret i64 %value
fail:
  ret i64 0
}

define i1 @__ncc_kernel_timer_arm_physical(i64 %deadline, i64 %vector) {
entry:
  %valid = icmp ult i64 %vector, 256
  br i1 %valid, label %arm, label %fail
arm:
  %armed = call i1 @__ncc_kernel_arch_timer_arm_physical(i64 %deadline, i64 %vector)
  ret i1 %armed
fail:
  ret i1 false
}

define i1 @__ncc_kernel_tlb_invalidate(i64 %address, i64 %length, i64 %asid) {
entry:
  %ok = call i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid)
  ret i1 %ok
}

define internal void @__ncc_kernel_tlb_clear_pending() {
entry:
  store atomic i64 0, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 0) seq_cst, align 8
  store atomic i64 0, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 1) seq_cst, align 8
  store atomic i64 0, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 2) seq_cst, align 8
  store atomic i64 0, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 3) seq_cst, align 8
  ret void
}

define i1 @__ncc_kernel_tlb_shootdown(i64 %address, i64 %length, i64 %asid, i64 %vector) {
entry:
  %vector.ok = icmp ult i64 %vector, 256
  %range.end = add i64 %address, %length
  %range.ok = icmp uge i64 %range.end, %address
  %valid = and i1 %vector.ok, %range.ok
  br i1 %valid, label %lock, label %fail
lock:
  %lock.result = cmpxchg ptr @__ncc_kernel_tlb_lock, i8 0, i8 1 acq_rel acquire
  %locked = extractvalue { i8, i1 } %lock.result, 1
  br i1 %locked, label %prepare, label %fail
prepare:
  %generation.old = atomicrmw add ptr @__ncc_kernel_tlb_generation, i64 1 seq_cst
  %generation = add i64 %generation.old, 1
  store atomic i64 %address, ptr @__ncc_kernel_tlb_address release, align 8
  store atomic i64 %length, ptr @__ncc_kernel_tlb_length release, align 8
  store atomic i64 %asid, ptr @__ncc_kernel_tlb_asid release, align 8
  call void @__ncc_kernel_tlb_clear_pending()
  fence seq_cst
  %local.ok = call i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid)
  %current = call i64 @__ncc_kernel_cpu_current_physical()
  %current.ok = icmp sge i64 %current, 0
  %start.ok = and i1 %local.ok, %current.ok
  br i1 %start.ok, label %loop, label %abort
loop:
  %cpu = phi i32 [ 0, %prepare ], [ %next, %continue ]
  %all.ok = phi i1 [ %local.ok, %prepare ], [ %next.ok, %continue ]
  %done = icmp uge i32 %cpu, 256
  br i1 %done, label %finish, label %inspect
inspect:
  %cpu.wide = zext i32 %cpu to i64
  %is.local = icmp eq i64 %cpu.wide, %current
  %used.ptr = getelementptr [256 x i1], ptr @__ncc_kernel_physical_cpu_used, i32 0, i32 %cpu
  %used = load i1, ptr %used.ptr, align 1
  %state.ptr = getelementptr [256 x i8], ptr @__ncc_kernel_physical_cpu_state, i32 0, i32 %cpu
  %state = load i8, ptr %state.ptr, align 1
  %online = icmp eq i8 %state, 3
  %remote = xor i1 %is.local, true
  %active = and i1 %used, %online
  %target = and i1 %active, %remote
  br i1 %target, label %send, label %skip
send:
  %word = lshr i32 %cpu, 6
  %bit.index = and i32 %cpu, 63
  %bit.index.wide = zext i32 %bit.index to i64
  %bit = shl i64 1, %bit.index.wide
  %word.ptr = getelementptr [4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 %word
  %pending.old = atomicrmw or ptr %word.ptr, i64 %bit seq_cst
  %sent = call i1 @__ncc_kernel_ipi_send_physical(i64 %cpu.wide, i64 %vector)
  br i1 %sent, label %sent.ok, label %send.fail
send.fail:
  %not.bit = xor i64 %bit, -1
  %cleared = atomicrmw and ptr %word.ptr, i64 %not.bit seq_cst
  br label %sent.ok
sent.ok:
  %send.ok = phi i1 [ true, %send ], [ false, %send.fail ]
  %combined = and i1 %all.ok, %send.ok
  br label %continue
skip:
  br label %continue
continue:
  %next.ok = phi i1 [ %combined, %sent.ok ], [ %all.ok, %skip ]
  %next = add nuw i32 %cpu, 1
  br label %loop
finish:
  br i1 %all.ok, label %wait, label %abort
wait:
  br label %wait.loop
wait.loop:
  %spin = phi i64 [ 0, %wait ], [ %spin.next, %wait.again ]
  %p0 = load atomic i64, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 0) acquire, align 8
  %p1 = load atomic i64, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 1) acquire, align 8
  %p2 = load atomic i64, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 2) acquire, align 8
  %p3 = load atomic i64, ptr getelementptr ([4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 3) acquire, align 8
  %p01 = or i64 %p0, %p1
  %p23 = or i64 %p2, %p3
  %pending = or i64 %p01, %p23
  %complete = icmp eq i64 %pending, 0
  br i1 %complete, label %success, label %wait.again
wait.again:
  call void @__ncc_kernel_arch_smp_relax()
  %spin.next = add nuw i64 %spin, 1
  %timeout = icmp uge i64 %spin.next, 10000000
  br i1 %timeout, label %abort, label %wait.loop
success:
  store atomic i8 0, ptr @__ncc_kernel_tlb_lock release, align 1
  ret i1 true
abort:
  call void @__ncc_kernel_tlb_clear_pending()
  store atomic i8 0, ptr @__ncc_kernel_tlb_lock release, align 1
  ret i1 false
fail:
  ret i1 false
}

define i1 @__ncc_kernel_tlb_shootdown_handle(i64 %cpu) {
entry:
  %valid = call i1 @__ncc_kernel_physical_cpu_valid(i64 %cpu)
  br i1 %valid, label %pending, label %fail
pending:
  %idx = trunc i64 %cpu to i32
  %word = lshr i32 %idx, 6
  %bit.index = and i32 %idx, 63
  %bit.index.wide = zext i32 %bit.index to i64
  %bit = shl i64 1, %bit.index.wide
  %word.ptr = getelementptr [4 x i64], ptr @__ncc_kernel_tlb_pending, i32 0, i32 %word
  %mask = load atomic i64, ptr %word.ptr acquire, align 8
  %present.raw = and i64 %mask, %bit
  %present = icmp ne i64 %present.raw, 0
  br i1 %present, label %invalidate, label %fail
invalidate:
  %generation = load atomic i64, ptr @__ncc_kernel_tlb_generation acquire, align 8
  %seen.ptr = getelementptr [256 x i64], ptr @__ncc_kernel_tlb_seen_generation, i32 0, i32 %idx
  %seen = load atomic i64, ptr %seen.ptr acquire, align 8
  %fresh = icmp ne i64 %seen, %generation
  br i1 %fresh, label %flush, label %fail
flush:
  %address = load atomic i64, ptr @__ncc_kernel_tlb_address acquire, align 8
  %length = load atomic i64, ptr @__ncc_kernel_tlb_length acquire, align 8
  %asid = load atomic i64, ptr @__ncc_kernel_tlb_asid acquire, align 8
  %ok = call i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid)
  br i1 %ok, label %ack, label %fail
ack:
  store atomic i64 %generation, ptr %seen.ptr release, align 8
  %not.bit = xor i64 %bit, -1
  %old = atomicrmw and ptr %word.ptr, i64 %not.bit release
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss) {
entry:
  %ok = call i1 @__ncc_kernel_arch_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss)
  ret i1 %ok
}

define i1 @__ncc_kernel_gic_init(i64 %gicd, i64 %gicr) {
entry:
  %ok = call i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr)
  ret i1 %ok
}

define i1 @__ncc_kernel_gic_init_firmware() {
entry:
  %gicd = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %gicr.base = load i64, ptr @__ncc_kernel_smp_secondary_base, align 8
  %stride.raw = load i64, ptr @__ncc_kernel_smp_redistributor_stride, align 8
  %stride.missing = icmp eq i64 %stride.raw, 0
  %stride = select i1 %stride.missing, i64 131072, i64 %stride.raw
  %gicd.ok = icmp ne i64 %gicd, 0
  %gicr.ok = icmp ne i64 %gicr.base, 0
  %valid = and i1 %gicd.ok, %gicr.ok
  br i1 %valid, label %affinity, label %fail
affinity:
  %mpidr = call i64 @__ncc_kernel_arch_bootstrap_hardware_id()
  %aff.low = and i64 %mpidr, 16777215
  %aff3.raw = lshr i64 %mpidr, 32
  %aff3.byte = and i64 %aff3.raw, 255
  %aff3 = shl i64 %aff3.byte, 24
  %expected = or i64 %aff.low, %aff3
  br label %scan
scan:
  %i = phi i64 [ 0, %affinity ], [ %next, %continue ]
  %offset = mul i64 %i, %stride
  %gicr = add i64 %gicr.base, %offset
  %typer.address = add i64 %gicr, 8
  %typer.ptr = inttoptr i64 %typer.address to ptr
  %typer = load volatile i64, ptr %typer.ptr, align 8
  %affinity.raw = lshr i64 %typer, 32
  %observed = and i64 %affinity.raw, 4294967295
  %same = icmp eq i64 %observed, %expected
  br i1 %same, label %hit, label %miss
hit:
  %ok = call i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr)
  ret i1 %ok
miss:
  %last.raw = and i64 %typer, 16
  %last = icmp ne i64 %last.raw, 0
  br i1 %last, label %fail, label %continue
continue:
  %next = add nuw i64 %i, 1
  %bounded = icmp ult i64 %next, 256
  br i1 %bounded, label %scan, label %fail
fail:
  ret i1 false
}

define i1 @__ncc_kernel_smp_interrupt_init_firmware(i64 %context) {
entry:
  %model = load i64, ptr @__ncc_kernel_smp_interrupt_model, align 8
  switch i64 %model, label %fail [
    i64 1, label %apic
    i64 2, label %apic
    i64 3, label %gic
    i64 4, label %plic
    i64 5, label %imsic.primary
    i64 6, label %imsic.secondary
    i64 7, label %plic.imsic
  ]
apic:
  ret i1 true
gic:
  %gic.ok = call i1 @__ncc_kernel_gic_init_firmware()
  ret i1 %gic.ok
plic:
  %plic.base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %plic.ok = call i1 @__ncc_kernel_plic_init(i64 %plic.base, i64 %context)
  ret i1 %plic.ok
imsic.primary:
  %imsic.base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %imsic.ok = call i1 @__ncc_kernel_arch_imsic_init(i64 %imsic.base)
  ret i1 %imsic.ok
imsic.secondary:
  %imsic.secondary.base = load i64, ptr @__ncc_kernel_smp_secondary_base, align 8
  %imsic.secondary.ok = call i1 @__ncc_kernel_arch_imsic_init(i64 %imsic.secondary.base)
  ret i1 %imsic.secondary.ok
plic.imsic:
  %combined.plic.base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %combined.imsic.base = load i64, ptr @__ncc_kernel_smp_secondary_base, align 8
  %combined.plic.ok = call i1 @__ncc_kernel_plic_init(i64 %combined.plic.base, i64 %context)
  %combined.imsic.ok = call i1 @__ncc_kernel_arch_imsic_init(i64 %combined.imsic.base)
  %combined.ok = and i1 %combined.plic.ok, %combined.imsic.ok
  ret i1 %combined.ok
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_plic_context_valid(i64 %context) {
entry:
  %count = load i64, ptr @__ncc_kernel_smp_context_count, align 8
  %unknown = icmp eq i64 %count, 0
  %inside = icmp ult i64 %context, %count
  %valid = or i1 %unknown, %inside
  ret i1 %valid
}

define internal i1 @__ncc_kernel_plic_source_valid(i64 %source) {
entry:
  %positive = icmp ugt i64 %source, 0
  %count = load i64, ptr @__ncc_kernel_smp_source_count, align 8
  %unknown = icmp eq i64 %count, 0
  %inside = icmp ule i64 %source, %count
  %bounded = or i1 %unknown, %inside
  %valid = and i1 %positive, %bounded
  ret i1 %valid
}

define i1 @__ncc_kernel_plic_init(i64 %base, i64 %context) {
entry:
  %valid = call i1 @__ncc_kernel_plic_context_valid(i64 %context)
  br i1 %valid, label %init, label %fail
init:
  %ok = call i1 @__ncc_kernel_arch_plic_init(i64 %base, i64 %context)
  ret i1 %ok
fail:
  ret i1 false
}

define i1 @__ncc_kernel_imsic_init(i64 %base) {
entry:
  %ok = call i1 @__ncc_kernel_arch_imsic_init(i64 %base)
  ret i1 %ok
}

define i1 @__ncc_kernel_gic_enable(i64 %interrupt, i64 %priority, i64 %edge) {
entry:
  %ok = call i1 @__ncc_kernel_arch_gic_enable(i64 %interrupt, i64 %priority, i64 %edge)
  ret i1 %ok
}

define i64 @__ncc_kernel_gic_ack() {
entry:
  %interrupt = call i64 @__ncc_kernel_arch_gic_ack()
  ret i64 %interrupt
}

define i1 @__ncc_kernel_gic_eoi(i64 %interrupt) {
entry:
  %ok = call i1 @__ncc_kernel_arch_gic_eoi(i64 %interrupt)
  ret i1 %ok
}

define i1 @__ncc_kernel_plic_enable(i64 %source, i64 %priority, i64 %context) {
entry:
  %source.ok = call i1 @__ncc_kernel_plic_source_valid(i64 %source)
  %context.ok = call i1 @__ncc_kernel_plic_context_valid(i64 %context)
  %valid = and i1 %source.ok, %context.ok
  br i1 %valid, label %enable, label %fail
enable:
  %ok = call i1 @__ncc_kernel_arch_plic_enable(i64 %source, i64 %priority, i64 %context)
  ret i1 %ok
fail:
  ret i1 false
}

define i64 @__ncc_kernel_plic_claim(i64 %context) {
entry:
  %valid = call i1 @__ncc_kernel_plic_context_valid(i64 %context)
  br i1 %valid, label %claim, label %fail
claim:
  %source = call i64 @__ncc_kernel_arch_plic_claim(i64 %context)
  ret i64 %source
fail:
  ret i64 0
}

define i1 @__ncc_kernel_plic_complete(i64 %context, i64 %source) {
entry:
  %source.ok = call i1 @__ncc_kernel_plic_source_valid(i64 %source)
  %context.ok = call i1 @__ncc_kernel_plic_context_valid(i64 %context)
  %valid = and i1 %source.ok, %context.ok
  br i1 %valid, label %complete, label %fail
complete:
  %ok = call i1 @__ncc_kernel_arch_plic_complete(i64 %context, i64 %source)
  ret i1 %ok
fail:
  ret i1 false
}

define i1 @__ncc_kernel_imsic_enable(i64 %interrupt) {
entry:
  %ok = call i1 @__ncc_kernel_arch_imsic_enable(i64 %interrupt)
  ret i1 %ok
}

define i64 @__ncc_kernel_imsic_claim() {
entry:
  %interrupt = call i64 @__ncc_kernel_arch_imsic_claim()
  ret i64 %interrupt
}

define i1 @__ncc_kernel_imsic_complete(i64 %interrupt) {
entry:
  %ok = call i1 @__ncc_kernel_arch_imsic_complete(i64 %interrupt)
  ret i1 %ok
}
"#;

pub(crate) fn render_kernel_firmware_smp_runtime(target_triple: &str) -> String {
    use crate::kernel_firmware_acpi_llvm::render_kernel_firmware_acpi_runtime;
    use crate::kernel_firmware_fdt_llvm::render_kernel_firmware_fdt_runtime;

    let triple = target_triple.trim().to_ascii_lowercase();
    let hosted = triple.contains("-linux-")
        || triple.contains("-windows-")
        || triple.contains("-darwin")
        || triple.contains("-freebsd");
    let arch = if hosted {
        HOSTED_REFERENCE_RUNTIME
    } else if triple.starts_with("x86_64") {
        X86_64_RUNTIME
    } else if triple.starts_with("i386") || triple.starts_with("i686") {
        I386_RUNTIME
    } else if triple.starts_with("aarch64") {
        AARCH64_RUNTIME
    } else if triple.starts_with("riscv64") {
        RISCV64_RUNTIME
    } else if triple.starts_with("riscv32") {
        RISCV32_RUNTIME
    } else {
        UNSUPPORTED_RUNTIME
    };
    let fdt = render_kernel_firmware_fdt_runtime();
    let acpi = render_kernel_firmware_acpi_runtime();
    let trampoline =
        if triple.starts_with("x86_64") || triple.starts_with("i386") || triple.starts_with("i686")
        {
            render_x86_64_ap_trampoline_llvm()
        } else {
            String::new()
        };
    let mut output = String::with_capacity(
        COMMON_RUNTIME.len() + fdt.len() + acpi.len() + trampoline.len() + arch.len(),
    );
    output.push_str(COMMON_RUNTIME);
    output.push_str(fdt);
    output.push_str(acpi);
    output.push_str(&trampoline);
    output.push_str(arch);
    output
}

fn render_x86_64_ap_trampoline_llvm() -> String {
    let image = ncc_machine::x86_64_ap_trampoline_image()
        .expect("the compile-time x86_64 AP trampoline must remain valid");
    let mut encoded = String::with_capacity(image.bytes.len() * 3);
    for byte in image.bytes {
        write!(&mut encoded, "\\{byte:02X}").expect("String writes cannot fail");
    }
    format!(
        "\n@__ncc_kernel_x86_ap_trampoline = private unnamed_addr constant [{} x i8] c\"{}\", align 16\n",
        image.bytes.len(),
        encoded
    )
}

const UNSUPPORTED_RUNTIME: &str = r#"
define internal i64 @__ncc_kernel_arch_bootstrap_hardware_id() { ret i64 0 }
define internal i64 @__ncc_kernel_arch_topology_pack(i64 %hardware) {
entry:
  %core = shl i64 %hardware, 16
  ret i64 %core
}
define internal i64 @__ncc_kernel_arch_timer_frequency() { ret i64 0 }
define internal void @__ncc_kernel_arch_smp_relax() { ret void }
define internal i64 @__ncc_kernel_arch_cpu_local_get() { ret i64 -1 }
define internal void @__ncc_kernel_arch_cpu_local_set(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_cpu_online_commit(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu) { ret void }
define internal i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry, i64 %context) { ret i1 false }
define internal i1 @__ncc_kernel_arch_ipi_send_physical(i64 %hardware, i64 %vector) { ret i1 false }
define internal i1 @__ncc_kernel_arch_timer_arm_physical(i64 %deadline, i64 %vector) { ret i1 false }
define internal i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid) { ret i1 false }
define internal i1 @__ncc_kernel_arch_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr) { ret i1 false }
define internal i1 @__ncc_kernel_arch_plic_init(i64 %base, i64 %context) { ret i1 false }
define internal i1 @__ncc_kernel_arch_imsic_init(i64 %base) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_enable(i64 %interrupt, i64 %priority, i64 %edge) { ret i1 false }
define internal i64 @__ncc_kernel_arch_gic_ack() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_gic_eoi(i64 %interrupt) { ret i1 false }
define internal i1 @__ncc_kernel_arch_plic_enable(i64 %source, i64 %priority, i64 %context) { ret i1 false }
define internal i64 @__ncc_kernel_arch_plic_claim(i64 %context) { ret i64 0 }
define internal i1 @__ncc_kernel_arch_plic_complete(i64 %context, i64 %source) { ret i1 false }
define internal i1 @__ncc_kernel_arch_imsic_enable(i64 %interrupt) { ret i1 false }
define internal i64 @__ncc_kernel_arch_imsic_claim() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_imsic_complete(i64 %interrupt) { ret i1 false }
"#;

// Hosted tests exercise firmware parsing and SMP state transitions without
// touching the bare-metal AP mailbox, interrupt controllers or timer MMIO.
const HOSTED_REFERENCE_RUNTIME: &str = UNSUPPORTED_RUNTIME;

const X86_64_RUNTIME: &str = r#"
@__ncc_kernel_x86_ap_mailbox_lock = internal global i8 0, align 1
@__ncc_kernel_x86_ap_generation = internal global i64 0, align 8

define internal { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 %leaf, i32 %subleaf) {
entry:
  %result = call { i32, i32, i32, i32 } asm sideeffect "cpuid", "={ax},={bx},={cx},={dx},{ax},{cx},~{dirflag},~{fpsr},~{flags}"(i32 %leaf, i32 %subleaf)
  ret { i32, i32, i32, i32 } %result
}

define internal i32 @__ncc_kernel_smp_x86_topology_leaf() {
entry:
  %root = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 0, i32 0)
  %maximum = extractvalue { i32, i32, i32, i32 } %root, 0
  %has.v2 = icmp uge i32 %maximum, 31
  br i1 %has.v2, label %probe.v2, label %legacy
probe.v2:
  %v2 = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 31, i32 0)
  %v2.count = extractvalue { i32, i32, i32, i32 } %v2, 1
  %v2.valid = icmp ne i32 %v2.count, 0
  br i1 %v2.valid, label %modern, label %legacy
modern:
  ret i32 31
legacy:
  ret i32 11
}

define internal i64 @__ncc_kernel_arch_bootstrap_hardware_id() {
entry:
  %leaf = call i32 @__ncc_kernel_smp_x86_topology_leaf()
  %topology = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 %leaf, i32 0)
  %count = extractvalue { i32, i32, i32, i32 } %topology, 1
  %x2id = extractvalue { i32, i32, i32, i32 } %topology, 3
  %valid = icmp ne i32 %count, 0
  br i1 %valid, label %x2apic, label %legacy
x2apic:
  %x2id64 = zext i32 %x2id to i64
  ret i64 %x2id64
legacy:
  %leaf1 = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 1, i32 0)
  %ebx = extractvalue { i32, i32, i32, i32 } %leaf1, 1
  %apic.shift = lshr i32 %ebx, 24
  %apic = and i32 %apic.shift, 255
  %apic64 = zext i32 %apic to i64
  ret i64 %apic64
}

define internal i64 @__ncc_kernel_arch_topology_pack(i64 %hardware) {
entry:
  %leaf = call i32 @__ncc_kernel_smp_x86_topology_leaf()
  %level0 = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 %leaf, i32 0)
  %level1 = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 %leaf, i32 1)
  %eax0 = extractvalue { i32, i32, i32, i32 } %level0, 0
  %ebx0 = extractvalue { i32, i32, i32, i32 } %level0, 1
  %eax1 = extractvalue { i32, i32, i32, i32 } %level1, 0
  %ebx1 = extractvalue { i32, i32, i32, i32 } %level1, 1
  %smt.shift32 = and i32 %eax0, 31
  %core.shift32 = and i32 %eax1, 31
  %has.smt = icmp ne i32 %ebx0, 0
  %has.core = icmp ne i32 %ebx1, 0
  %ordered = icmp uge i32 %core.shift32, %smt.shift32
  %a = and i1 %has.smt, %has.core
  %valid = and i1 %a, %ordered
  br i1 %valid, label %decode, label %fallback
decode:
  %smt.shift = zext i32 %smt.shift32 to i64
  %core.shift = zext i32 %core.shift32 to i64
  %one.smt = shl i64 1, %smt.shift
  %smt.mask = sub i64 %one.smt, 1
  %core.width = sub i64 %core.shift, %smt.shift
  %one.core = shl i64 1, %core.width
  %core.mask = sub i64 %one.core, 1
  %thread = and i64 %hardware, %smt.mask
  %core.raw = lshr i64 %hardware, %smt.shift
  %core = and i64 %core.raw, %core.mask
  %package = lshr i64 %hardware, %core.shift
  %package.mask = and i64 %package, 65535
  %core.masked = and i64 %core, 65535
  %thread.masked = and i64 %thread, 65535
  %core.field = shl i64 %core.masked, 16
  %thread.field = shl i64 %thread.masked, 32
  %low = or i64 %package.mask, %core.field
  %packed = or i64 %low, %thread.field
  ret i64 %packed
fallback:
  %fallback.core = and i64 %hardware, 65535
  %fallback.packed = shl i64 %fallback.core, 16
  ret i64 %fallback.packed
}

define internal i64 @__ncc_kernel_arch_timer_frequency() {
entry:
  %leaf15 = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 21, i32 0)
  %denominator32 = extractvalue { i32, i32, i32, i32 } %leaf15, 0
  %numerator32 = extractvalue { i32, i32, i32, i32 } %leaf15, 1
  %crystal32 = extractvalue { i32, i32, i32, i32 } %leaf15, 2
  %denominator.ok = icmp ne i32 %denominator32, 0
  %numerator.ok = icmp ne i32 %numerator32, 0
  %crystal.ok = icmp ne i32 %crystal32, 0
  %a = and i1 %denominator.ok, %numerator.ok
  %ratio.ok = and i1 %a, %crystal.ok
  br i1 %ratio.ok, label %ratio, label %leaf16
ratio:
  %denominator = zext i32 %denominator32 to i64
  %numerator = zext i32 %numerator32 to i64
  %crystal = zext i32 %crystal32 to i64
  %product = mul i64 %crystal, %numerator
  %frequency = udiv i64 %product, %denominator
  ret i64 %frequency
leaf16:
  %base = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 22, i32 0)
  %mhz32 = extractvalue { i32, i32, i32, i32 } %base, 0
  %mhz = zext i32 %mhz32 to i64
  %frequency.fallback = mul i64 %mhz, 1000000
  ret i64 %frequency.fallback
}

define internal i64 @__ncc_kernel_arch_cpu_local_get() { ret i64 -1 }
define internal void @__ncc_kernel_arch_cpu_local_set(i64 %cpu) { ret void }

define internal i1 @__ncc_kernel_x86_ap_mailbox_owned(i64 %cpu) {
entry:
  %magic.ptr = inttoptr i64 24576 to ptr
  %magic = load volatile i64, ptr %magic.ptr, align 8
  %magic.ok = icmp eq i64 %magic, 13877280946471758
  %cpu.ptr = inttoptr i64 24592 to ptr
  %cpu.raw = load volatile i32, ptr %cpu.ptr, align 4
  %cpu.wide = zext i32 %cpu.raw to i64
  %cpu.ok = icmp eq i64 %cpu.wide, %cpu
  %owned = and i1 %magic.ok, %cpu.ok
  ret i1 %owned
}

define internal void @__ncc_kernel_arch_cpu_online_commit(i64 %cpu) {
entry:
  %owned = call i1 @__ncc_kernel_x86_ap_mailbox_owned(i64 %cpu)
  br i1 %owned, label %commit, label %finish
commit:
  %state.ptr = inttoptr i64 24596 to ptr
  store atomic i32 3, ptr %state.ptr release, align 4
  store atomic i8 0, ptr @__ncc_kernel_x86_ap_mailbox_lock release, align 1
  br label %finish
finish:
  ret void
}

define internal void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu) {
entry:
  %owned = call i1 @__ncc_kernel_x86_ap_mailbox_owned(i64 %cpu)
  br i1 %owned, label %abort, label %unlock
abort:
  %state.ptr = inttoptr i64 24596 to ptr
  store atomic i32 5, ptr %state.ptr release, align 4
  br label %unlock
unlock:
  store atomic i8 0, ptr @__ncc_kernel_x86_ap_mailbox_lock release, align 1
  ret void
}

define internal i1 @__ncc_kernel_x86_ap_prepare(i64 %cpu, i64 %stack, i64 %entry.address, i64 %context) {
entry:
  %lock.result = cmpxchg ptr @__ncc_kernel_x86_ap_mailbox_lock, i8 0, i8 1 acq_rel acquire
  %locked = extractvalue { i8, i1 } %lock.result, 1
  br i1 %locked, label %validate, label %fail
validate:
  %cr3.raw = call i32 asm sideeffect ".byte 0x0f,0x20,0xd8", "={ax},~{memory}"()
  %cr3 = zext i32 %cr3.raw to i64
  %cr3.present = icmp ne i64 %cr3, 0
  %stack.present = icmp ne i64 %stack, 0
  %entry.present = icmp ne i64 %entry.address, 0
  %a = and i1 %cr3.present, %stack.present
  %valid = and i1 %a, %entry.present
  br i1 %valid, label %copy, label %release
copy:
  br label %copy.loop
copy.loop:
  %index = phi i64 [ 0, %copy ], [ %next, %copy.body ]
  %done = icmp uge i64 %index, 314
  br i1 %done, label %mailbox, label %copy.body
copy.body:
  %source = getelementptr [314 x i8], ptr @__ncc_kernel_x86_ap_trampoline, i64 0, i64 %index
  %byte = load i8, ptr %source, align 1
  %destination.address = add i64 28672, %index
  %destination = inttoptr i64 %destination.address to ptr
  store volatile i8 %byte, ptr %destination, align 1
  %next = add nuw i64 %index, 1
  br label %copy.loop
mailbox:
  %generation.old = atomicrmw add ptr @__ncc_kernel_x86_ap_generation, i64 1 acq_rel
  %generation = add i64 %generation.old, 1
  %magic.ptr = inttoptr i64 24576 to ptr
  %generation.ptr = inttoptr i64 24584 to ptr
  %cpu.ptr = inttoptr i64 24592 to ptr
  %state.ptr = inttoptr i64 24596 to ptr
  %stack.ptr = inttoptr i64 24600 to ptr
  %entry.ptr = inttoptr i64 24608 to ptr
  %context.ptr = inttoptr i64 24616 to ptr
  %cr3.ptr = inttoptr i64 24624 to ptr
  %gdt.ptr = inttoptr i64 24632 to ptr
  %idt.ptr = inttoptr i64 24640 to ptr
  %park.ptr = inttoptr i64 24648 to ptr
  %cpu.narrow = trunc i64 %cpu to i32
  %park.callback = ptrtoint ptr @__ncc_kernel_cpu_mark_parked to i64
  store volatile i32 0, ptr %state.ptr, align 4
  store volatile i64 13877280946471758, ptr %magic.ptr, align 8
  store volatile i64 %generation, ptr %generation.ptr, align 8
  store volatile i32 %cpu.narrow, ptr %cpu.ptr, align 4
  store volatile i64 %stack, ptr %stack.ptr, align 8
  store volatile i64 %entry.address, ptr %entry.ptr, align 8
  store volatile i64 %context, ptr %context.ptr, align 8
  store volatile i64 %cr3, ptr %cr3.ptr, align 8
  store volatile i64 0, ptr %gdt.ptr, align 8
  store volatile i64 0, ptr %idt.ptr, align 8
  store volatile i64 %park.callback, ptr %park.ptr, align 8
  fence release
  store atomic i32 1, ptr %state.ptr release, align 4
  ret i1 true
release:
  store atomic i8 0, ptr @__ncc_kernel_x86_ap_mailbox_lock release, align 1
  ret i1 false
fail:
  ret i1 false
}

define internal void @__ncc_kernel_arch_smp_relax() {
entry:
  call void asm sideeffect "pause", "~{memory}"()
  ret void
}

define internal void @__ncc_kernel_x86_wrmsr(i64 %msr, i64 %value) {
entry:
  %low = trunc i64 %value to i32
  %high.shift = lshr i64 %value, 32
  %high = trunc i64 %high.shift to i32
  %msr32 = trunc i64 %msr to i32
  call void asm sideeffect "wrmsr", "{cx},{ax},{dx},~{dirflag},~{fpsr},~{flags},~{memory}"(i32 %msr32, i32 %low, i32 %high)
  ret void
}

define internal void @__ncc_kernel_x86_delay_us(i64 %microseconds) {
entry:
  %frequency = load i64, ptr @__ncc_kernel_smp_timer_hz, align 8
  %scaled = mul i64 %frequency, %microseconds
  %cycles = udiv i64 %scaled, 1000000
  %start = call i64 @llvm.readcyclecounter()
  br label %loop
loop:
  call void asm sideeffect "pause", "~{memory}"()
  %now = call i64 @llvm.readcyclecounter()
  %elapsed = sub i64 %now, %start
  %done = icmp uge i64 %elapsed, %cycles
  br i1 %done, label %finish, label %loop
finish:
  ret void
}

define internal i1 @__ncc_kernel_x86_xapic_wait(i64 %base) {
entry:
  %icr.address = add i64 %base, 768
  %icr.ptr = inttoptr i64 %icr.address to ptr
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %again ]
  %value = load volatile i32, ptr %icr.ptr, align 4
  %pending.raw = and i32 %value, 4096
  %ready = icmp eq i32 %pending.raw, 0
  br i1 %ready, label %ok, label %again
again:
  call void asm sideeffect "pause", "~{memory}"()
  %next = add nuw i32 %i, 1
  %timeout = icmp uge i32 %next, 1000000
  br i1 %timeout, label %fail, label %loop
ok:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_x86_xapic_command(i64 %hardware, i64 %command) {
entry:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %present = icmp ne i64 %base, 0
  br i1 %present, label %write, label %fail
write:
  %high.address = add i64 %base, 784
  %low.address = add i64 %base, 768
  %high.ptr = inttoptr i64 %high.address to ptr
  %low.ptr = inttoptr i64 %low.address to ptr
  %destination.low = trunc i64 %hardware to i32
  %destination = shl i32 %destination.low, 24
  %command32 = trunc i64 %command to i32
  store volatile i32 %destination, ptr %high.ptr, align 4
  store volatile i32 %command32, ptr %low.ptr, align 4
  %ok = call i1 @__ncc_kernel_x86_xapic_wait(i64 %base)
  ret i1 %ok
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_x86_apic_command(i64 %hardware, i64 %command) {
entry:
  %mode = load i64, ptr @__ncc_kernel_smp_firmware_mode, align 8
  %x2 = icmp eq i64 %mode, 1
  br i1 %x2, label %x2apic, label %xapic
x2apic:
  %destination = shl i64 %hardware, 32
  %value = or i64 %destination, %command
  call void @__ncc_kernel_x86_wrmsr(i64 2096, i64 %value)
  ret i1 true
xapic:
  %ok = call i1 @__ncc_kernel_x86_xapic_command(i64 %hardware, i64 %command)
  ret i1 %ok
}

define internal i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry.address, i64 %context) {
entry:
  %prepared = call i1 @__ncc_kernel_x86_ap_prepare(i64 %cpu, i64 %stack, i64 %entry.address, i64 %context)
  br i1 %prepared, label %init.assert, label %fail
init.assert:
  %init1 = call i1 @__ncc_kernel_x86_apic_command(i64 %hardware, i64 50432)
  br i1 %init1, label %init.delay, label %abort
init.delay:
  call void @__ncc_kernel_x86_delay_us(i64 10000)
  %init2 = call i1 @__ncc_kernel_x86_apic_command(i64 %hardware, i64 34048)
  br i1 %init2, label %sipi.delay, label %abort
sipi.delay:
  call void @__ncc_kernel_x86_delay_us(i64 200)
  %vector = lshr i64 28672, 12
  %sipi.command = or i64 %vector, 17920
  %sipi1 = call i1 @__ncc_kernel_x86_apic_command(i64 %hardware, i64 %sipi.command)
  br i1 %sipi1, label %second.delay, label %abort
second.delay:
  call void @__ncc_kernel_x86_delay_us(i64 200)
  %sipi2 = call i1 @__ncc_kernel_x86_apic_command(i64 %hardware, i64 %sipi.command)
  br i1 %sipi2, label %ok, label %abort
ok:
  ret i1 true
abort:
  call void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu)
  ret i1 false
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_ipi_send_physical(i64 %hardware, i64 %vector) {
entry:
  %command = or i64 %vector, 16384
  %ok = call i1 @__ncc_kernel_x86_apic_command(i64 %hardware, i64 %command)
  ret i1 %ok
}

define internal i1 @__ncc_kernel_arch_timer_arm_physical(i64 %deadline, i64 %vector) {
entry:
  %vector.low = icmp uge i64 %vector, 16
  %vector.high = icmp ult i64 %vector, 256
  %vector.ok = and i1 %vector.low, %vector.high
  br i1 %vector.ok, label %feature, label %fail
feature:
  %leaf1 = call { i32, i32, i32, i32 } @__ncc_kernel_smp_x86_cpuid(i32 1, i32 0)
  %ecx = extractvalue { i32, i32, i32, i32 } %leaf1, 2
  %deadline.raw = and i32 %ecx, 16777216
  %deadline.supported = icmp ne i32 %deadline.raw, 0
  br i1 %deadline.supported, label %mode.read, label %fail
mode.read:
  %mode = load i64, ptr @__ncc_kernel_smp_firmware_mode, align 8
  %x2 = icmp eq i64 %mode, 1
  br i1 %x2, label %x2apic, label %xapic
x2apic:
  %vector.mode = or i64 %vector, 262144
  call void @__ncc_kernel_x86_wrmsr(i64 2098, i64 %vector.mode)
  br label %deadline.write
xapic:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %present = icmp ne i64 %base, 0
  br i1 %present, label %lvt, label %fail
lvt:
  %lvt.address = add i64 %base, 800
  %lvt.ptr = inttoptr i64 %lvt.address to ptr
  %lvt.value64 = or i64 %vector, 262144
  %lvt.value = trunc i64 %lvt.value64 to i32
  store volatile i32 %lvt.value, ptr %lvt.ptr, align 4
  br label %deadline.write
deadline.write:
  call void @__ncc_kernel_x86_wrmsr(i64 1760, i64 %deadline)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid) {
entry:
  %empty = icmp eq i64 %length, 0
  %effective = select i1 %empty, i64 1, i64 %length
  %end = add i64 %address, %effective
  %base = and i64 %address, -4096
  br label %loop
loop:
  %cursor = phi i64 [ %base, %entry ], [ %next, %body ]
  %done = icmp uge i64 %cursor, %end
  br i1 %done, label %finish, label %body
body:
  %ptr = inttoptr i64 %cursor to ptr
  call void asm sideeffect "invlpg ($0)", "r,~{memory}"(ptr %ptr)
  %next = add i64 %cursor, 4096
  br label %loop
finish:
  ret i1 true
}

define internal i1 @__ncc_kernel_arch_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss) {
entry:
  %gdt.ok = icmp ne i64 %gdt, 0
  %idt.ok = icmp ne i64 %idt, 0
  %a = and i1 %gdt.ok, %idt.ok
  br i1 %a, label %load, label %fail
load:
  %gdt.ptr = inttoptr i64 %gdt to ptr
  %idt.ptr = inttoptr i64 %idt to ptr
  %selector = trunc i64 %tss to i16
  call void asm sideeffect "lgdt ($0); lidt ($1); ltr $2", "r,r,r,~{memory}"(ptr %gdt.ptr, ptr %idt.ptr, i16 %selector)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr) { ret i1 false }
define internal i1 @__ncc_kernel_arch_plic_init(i64 %base, i64 %context) { ret i1 false }
define internal i1 @__ncc_kernel_arch_imsic_init(i64 %base) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_enable(i64 %interrupt, i64 %priority, i64 %edge) { ret i1 false }
define internal i64 @__ncc_kernel_arch_gic_ack() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_gic_eoi(i64 %interrupt) { ret i1 false }
define internal i1 @__ncc_kernel_arch_plic_enable(i64 %source, i64 %priority, i64 %context) { ret i1 false }
define internal i64 @__ncc_kernel_arch_plic_claim(i64 %context) { ret i64 0 }
define internal i1 @__ncc_kernel_arch_plic_complete(i64 %context, i64 %source) { ret i1 false }
define internal i1 @__ncc_kernel_arch_imsic_enable(i64 %interrupt) { ret i1 false }
define internal i64 @__ncc_kernel_arch_imsic_claim() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_imsic_complete(i64 %interrupt) { ret i1 false }
"#;

const I386_RUNTIME: &str = X86_64_RUNTIME;

const AARCH64_RUNTIME: &str = r#"
define internal i64 @__ncc_kernel_arch_bootstrap_hardware_id() {
entry:
  %mpidr = call i64 asm sideeffect "mrs $0, mpidr_el1", "=r"()
  %masked = and i64 %mpidr, 1095233437695
  ret i64 %masked
}

define internal i64 @__ncc_kernel_arch_topology_pack(i64 %hardware) {
entry:
  %aff0 = and i64 %hardware, 255
  %aff1.raw = lshr i64 %hardware, 8
  %aff1 = and i64 %aff1.raw, 255
  %aff2.raw = lshr i64 %hardware, 16
  %aff2 = and i64 %aff2.raw, 255
  %aff3.raw = lshr i64 %hardware, 32
  %aff3 = and i64 %aff3.raw, 255
  %package.high = shl i64 %aff3, 8
  %package = or i64 %package.high, %aff2
  %core = shl i64 %aff1, 16
  %thread = shl i64 %aff0, 32
  %low = or i64 %package, %core
  %packed = or i64 %low, %thread
  ret i64 %packed
}

define internal i64 @__ncc_kernel_arch_timer_frequency() {
entry:
  %frequency = call i64 asm sideeffect "mrs $0, cntfrq_el0", "=r"()
  ret i64 %frequency
}

define internal i64 @__ncc_kernel_arch_cpu_local_get() {
entry:
  %cpu = call i64 asm sideeffect "mrs $0, tpidr_el1", "=r"()
  ret i64 %cpu
}

define internal void @__ncc_kernel_arch_cpu_local_set(i64 %cpu) {
entry:
  call void asm sideeffect "msr tpidr_el1, $0; isb", "r,~{memory}"(i64 %cpu)
  ret void
}
define internal void @__ncc_kernel_arch_cpu_online_commit(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu) { ret void }

define internal void @__ncc_kernel_arch_smp_relax() {
entry:
  call void asm sideeffect "yield", "~{memory}"()
  ret void
}

define internal i64 @__ncc_kernel_arm_psci(i64 %function, i64 %a1, i64 %a2, i64 %a3) {
entry:
  %mode = load i64, ptr @__ncc_kernel_smp_firmware_mode, align 8
  %hvc = icmp eq i64 %mode, 2
  br i1 %hvc, label %via.hvc, label %via.smc
via.hvc:
  %hvc.result = call i64 asm sideeffect "hvc #0", "={x0},0,{x1},{x2},{x3},~{memory}"(i64 %function, i64 %a1, i64 %a2, i64 %a3)
  ret i64 %hvc.result
via.smc:
  %smc.result = call i64 asm sideeffect "smc #0", "={x0},0,{x1},{x2},{x3},~{memory}"(i64 %function, i64 %a1, i64 %a2, i64 %a3)
  ret i64 %smc.result
}

define internal i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry.address, i64 %context) {
entry:
  %result = call i64 @__ncc_kernel_arm_psci(i64 3288334339, i64 %hardware, i64 %entry.address, i64 %context)
  %success = icmp eq i64 %result, 0
  %already = icmp eq i64 %result, -4
  %accepted = or i1 %success, %already
  br i1 %accepted, label %poll, label %fail
poll:
  %i = phi i32 [ 0, %entry ], [ %next, %again ]
  %status = call i64 @__ncc_kernel_arm_psci(i64 3288334340, i64 %hardware, i64 0, i64 0)
  %online = icmp eq i64 %status, 0
  br i1 %online, label %ok, label %status.check
status.check:
  %off = icmp eq i64 %status, 1
  %pending = icmp eq i64 %status, 2
  %retry = or i1 %off, %pending
  br i1 %retry, label %again, label %fail
again:
  call void asm sideeffect "yield", "~{memory}"()
  %next = add nuw i32 %i, 1
  %timeout = icmp uge i32 %next, 1000000
  br i1 %timeout, label %fail, label %poll
ok:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_ipi_send_physical(i64 %hardware, i64 %vector) {
entry:
  %vector.ok = icmp ult i64 %vector, 16
  br i1 %vector.ok, label %encode, label %fail
encode:
  %aff0 = and i64 %hardware, 255
  %target.index = and i64 %aff0, 15
  %target = shl i64 1, %target.index
  %rs.raw = lshr i64 %aff0, 4
  %rs = shl i64 %rs.raw, 44
  %aff1.raw = lshr i64 %hardware, 8
  %aff1.mask = and i64 %aff1.raw, 255
  %aff1 = shl i64 %aff1.mask, 16
  %aff2.raw = lshr i64 %hardware, 16
  %aff2.mask = and i64 %aff2.raw, 255
  %aff2 = shl i64 %aff2.mask, 32
  %aff3.raw = lshr i64 %hardware, 32
  %aff3.mask = and i64 %aff3.raw, 255
  %aff3 = shl i64 %aff3.mask, 48
  %intid.mask = and i64 %vector, 15
  %intid = shl i64 %intid.mask, 24
  %a = or i64 %target, %rs
  %b = or i64 %aff1, %aff2
  %c = or i64 %a, %b
  %d = or i64 %c, %aff3
  %sgi = or i64 %d, %intid
  call void asm sideeffect "msr icc_sgi1r_el1, $0; isb", "r,~{memory}"(i64 %sgi)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_timer_arm_physical(i64 %deadline, i64 %vector) {
entry:
  %vector.ok = icmp eq i64 %vector, 27
  br i1 %vector.ok, label %arm, label %fail
arm:
  call void asm sideeffect "msr cntv_cval_el0, $0; msr cntv_ctl_el0, $1; isb", "r,r,~{memory}"(i64 %deadline, i64 1)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid) {
entry:
  %empty = icmp eq i64 %length, 0
  br i1 %empty, label %all, label %range
all:
  call void asm sideeffect "dsb ishst; tlbi vmalle1is; dsb ish; isb", "~{memory}"()
  ret i1 true
range:
  %end = add i64 %address, %length
  %base = and i64 %address, -4096
  br label %loop
loop:
  %cursor = phi i64 [ %base, %range ], [ %next, %body ]
  %done = icmp uge i64 %cursor, %end
  br i1 %done, label %finish, label %body
body:
  %page = lshr i64 %cursor, 12
  %asid.mask = and i64 %asid, 65535
  %asid.field = shl i64 %asid.mask, 48
  %operand = or i64 %page, %asid.field
  call void asm sideeffect "dsb ishst; tlbi vae1is, $0; dsb ish; isb", "r,~{memory}"(i64 %operand)
  %next = add i64 %cursor, 4096
  br label %loop
finish:
  ret i1 true
}

define internal i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr) {
entry:
  %gicd.ok = icmp ne i64 %gicd, 0
  %gicr.ok = icmp ne i64 %gicr, 0
  %valid = and i1 %gicd.ok, %gicr.ok
  br i1 %valid, label %distributor.disable, label %fail
distributor.disable:
  %gicd.ptr = inttoptr i64 %gicd to ptr
  store volatile i32 0, ptr %gicd.ptr, align 4
  br label %distributor.wait
distributor.wait:
  %dw = phi i32 [ 0, %distributor.disable ], [ %dw.next, %distributor.again ]
  %ctlr = load volatile i32, ptr %gicd.ptr, align 4
  %rwp.raw = and i32 %ctlr, -2147483648
  %rwp.clear = icmp eq i32 %rwp.raw, 0
  br i1 %rwp.clear, label %distributor.lines, label %distributor.again
distributor.again:
  call void asm sideeffect "yield", "~{memory}"()
  %dw.next = add nuw i32 %dw, 1
  %dw.timeout = icmp uge i32 %dw.next, 1000000
  br i1 %dw.timeout, label %fail, label %distributor.wait
distributor.lines:
  %typer.address = add i64 %gicd, 4
  %typer.ptr = inttoptr i64 %typer.address to ptr
  %typer = load volatile i32, ptr %typer.ptr, align 4
  %line.raw = and i32 %typer, 31
  %line.count = add i32 %line.raw, 1
  br label %distributor.configure
distributor.configure:
  %line = phi i32 [ 1, %distributor.lines ], [ %line.next, %distributor.body ]
  %line.done = icmp uge i32 %line, %line.count
  br i1 %line.done, label %redistributor.wake, label %distributor.body
distributor.body:
  %line.wide = zext i32 %line to i64
  %word.offset = mul i64 %line.wide, 4
  %group.base = add i64 %gicd, 128
  %group.address = add i64 %group.base, %word.offset
  %group.ptr = inttoptr i64 %group.address to ptr
  store volatile i32 -1, ptr %group.ptr, align 4
  %disable.base = add i64 %gicd, 384
  %disable.address = add i64 %disable.base, %word.offset
  %disable.ptr = inttoptr i64 %disable.address to ptr
  store volatile i32 -1, ptr %disable.ptr, align 4
  %clear.base = add i64 %gicd, 640
  %clear.address = add i64 %clear.base, %word.offset
  %clear.ptr = inttoptr i64 %clear.address to ptr
  store volatile i32 -1, ptr %clear.ptr, align 4
  %line.next = add nuw i32 %line, 1
  br label %distributor.configure
redistributor.wake:
  %waker.address = add i64 %gicr, 20
  %waker.ptr = inttoptr i64 %waker.address to ptr
  %waker = load volatile i32, ptr %waker.ptr, align 4
  %awake = and i32 %waker, -3
  store volatile i32 %awake, ptr %waker.ptr, align 4
  br label %wait
wait:
  %i = phi i32 [ 0, %redistributor.wake ], [ %next, %again ]
  %state = load volatile i32, ptr %waker.ptr, align 4
  %children.raw = and i32 %state, 4
  %ready = icmp eq i32 %children.raw, 0
  br i1 %ready, label %redistributor.configure, label %again
again:
  call void asm sideeffect "yield", "~{memory}"()
  %next = add nuw i32 %i, 1
  %timeout = icmp uge i32 %next, 1000000
  br i1 %timeout, label %fail, label %wait
redistributor.configure:
  %sgi.base = add i64 %gicr, 65536
  %sgi.group.address = add i64 %sgi.base, 128
  %sgi.group.ptr = inttoptr i64 %sgi.group.address to ptr
  store volatile i32 -1, ptr %sgi.group.ptr, align 4
  %sgi.disable.address = add i64 %sgi.base, 384
  %sgi.disable.ptr = inttoptr i64 %sgi.disable.address to ptr
  store volatile i32 -1, ptr %sgi.disable.ptr, align 4
  %sgi.clear.address = add i64 %sgi.base, 640
  %sgi.clear.ptr = inttoptr i64 %sgi.clear.address to ptr
  store volatile i32 -1, ptr %sgi.clear.ptr, align 4
  store volatile i32 18, ptr %gicd.ptr, align 4
  br label %enable.wait
enable.wait:
  %ew = phi i32 [ 0, %redistributor.configure ], [ %ew.next, %enable.again ]
  %enabled.ctlr = load volatile i32, ptr %gicd.ptr, align 4
  %enabled.rwp = and i32 %enabled.ctlr, -2147483648
  %enabled.ready = icmp eq i32 %enabled.rwp, 0
  br i1 %enabled.ready, label %cpu.interface, label %enable.again
enable.again:
  call void asm sideeffect "yield", "~{memory}"()
  %ew.next = add nuw i32 %ew, 1
  %ew.timeout = icmp uge i32 %ew.next, 1000000
  br i1 %ew.timeout, label %fail, label %enable.wait
cpu.interface:
  call void asm sideeffect "msr icc_sre_el1, $0; isb; msr icc_pmr_el1, $1; msr icc_igrpen1_el1, $2; isb", "r,r,r,~{memory}"(i64 7, i64 255, i64 1)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_gic_enable(i64 %interrupt, i64 %priority, i64 %edge) {
entry:
  %id.ok = icmp ult i64 %interrupt, 1020
  %priority.ok = icmp ult i64 %priority, 256
  %valid = and i1 %id.ok, %priority.ok
  br i1 %valid, label %select, label %fail
select:
  %private = icmp ult i64 %interrupt, 32
  %gicd = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %gicr = load i64, ptr @__ncc_kernel_smp_secondary_base, align 8
  %sgi = add i64 %gicr, 65536
  %base = select i1 %private, i64 %sgi, i64 %gicd
  %word = lshr i64 %interrupt, 5
  %word.offset = mul i64 %word, 4
  %group.base = add i64 %base, 128
  %group.address = add i64 %group.base, %word.offset
  %group.ptr = inttoptr i64 %group.address to ptr
  %group.old = load volatile i32, ptr %group.ptr, align 4
  %bit.index = and i64 %interrupt, 31
  %bit32 = trunc i64 %bit.index to i32
  %bit = shl i32 1, %bit32
  %group.new = or i32 %group.old, %bit
  store volatile i32 %group.new, ptr %group.ptr, align 4
  %priority.address0 = add i64 %base, 1024
  %priority.address = add i64 %priority.address0, %interrupt
  %priority.ptr = inttoptr i64 %priority.address to ptr
  %priority8 = trunc i64 %priority to i8
  store volatile i8 %priority8, ptr %priority.ptr, align 1
  %cfg.word = lshr i64 %interrupt, 4
  %cfg.word.offset = mul i64 %cfg.word, 4
  %cfg.base = add i64 %base, 3072
  %cfg.address = add i64 %cfg.base, %cfg.word.offset
  %cfg.ptr = inttoptr i64 %cfg.address to ptr
  %cfg.old = load volatile i32, ptr %cfg.ptr, align 4
  %cfg.index = and i64 %interrupt, 15
  %cfg.shift64 = mul i64 %cfg.index, 2
  %cfg.shift.add = add i64 %cfg.shift64, 1
  %cfg.shift = trunc i64 %cfg.shift.add to i32
  %edge.bit = shl i32 1, %cfg.shift
  %edge.enabled = icmp ne i64 %edge, 0
  %edge.set = or i32 %cfg.old, %edge.bit
  %edge.not = xor i32 %edge.bit, -1
  %edge.clear = and i32 %cfg.old, %edge.not
  %cfg.new = select i1 %edge.enabled, i32 %edge.set, i32 %edge.clear
  store volatile i32 %cfg.new, ptr %cfg.ptr, align 4
  %enable.base = add i64 %base, 256
  %enable.address = add i64 %enable.base, %word.offset
  %enable.ptr = inttoptr i64 %enable.address to ptr
  store volatile i32 %bit, ptr %enable.ptr, align 4
  call void asm sideeffect "dsb sy; isb", "~{memory}"()
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_kernel_arch_gic_ack() {
entry:
  %raw = call i64 asm sideeffect "mrs $0, icc_iar1_el1", "=r,~{memory}"()
  %id = and i64 %raw, 16777215
  ret i64 %id
}

define internal i1 @__ncc_kernel_arch_gic_eoi(i64 %interrupt) {
entry:
  %valid = icmp ult i64 %interrupt, 1024
  br i1 %valid, label %write, label %fail
write:
  call void asm sideeffect "msr icc_eoir1_el1, $0; isb", "r,~{memory}"(i64 %interrupt)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss) { ret i1 false }
define internal i1 @__ncc_kernel_arch_plic_init(i64 %base, i64 %context) { ret i1 false }
define internal i1 @__ncc_kernel_arch_imsic_init(i64 %base) { ret i1 false }
define internal i1 @__ncc_kernel_arch_plic_enable(i64 %source, i64 %priority, i64 %context) { ret i1 false }
define internal i64 @__ncc_kernel_arch_plic_claim(i64 %context) { ret i64 0 }
define internal i1 @__ncc_kernel_arch_plic_complete(i64 %context, i64 %source) { ret i1 false }
define internal i1 @__ncc_kernel_arch_imsic_enable(i64 %interrupt) { ret i1 false }
define internal i64 @__ncc_kernel_arch_imsic_claim() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_imsic_complete(i64 %interrupt) { ret i1 false }
"#;

const RISCV64_RUNTIME: &str = r#"
define internal i64 @__ncc_kernel_arch_bootstrap_hardware_id() { ret i64 0 }
define internal i64 @__ncc_kernel_arch_topology_pack(i64 %hardware) {
entry:
  %core = and i64 %hardware, 65535
  %packed = shl i64 %core, 16
  ret i64 %packed
}
define internal i64 @__ncc_kernel_arch_timer_frequency() { ret i64 0 }
define internal i64 @__ncc_kernel_arch_cpu_local_get() {
entry:
  %cpu = call i64 asm sideeffect "mv $0, tp", "=r"()
  ret i64 %cpu
}
define internal void @__ncc_kernel_arch_cpu_local_set(i64 %cpu) {
entry:
  call void asm sideeffect "mv tp, $0", "r,~{memory}"(i64 %cpu)
  ret void
}
define internal void @__ncc_kernel_arch_cpu_online_commit(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_smp_relax() {
entry:
  call void asm sideeffect "nop", "~{memory}"()
  ret void
}

define internal { i64, i64 } @__ncc_kernel_riscv_sbi(i64 %a0, i64 %a1, i64 %a2, i64 %a3, i64 %a4, i64 %a5, i64 %fid, i64 %eid) {
entry:
  %result = call { i64, i64 } asm sideeffect "ecall", "={x10},={x11},0,1,{x12},{x13},{x14},{x15},{x16},{x17},~{memory}"(i64 %a0, i64 %a1, i64 %a2, i64 %a3, i64 %a4, i64 %a5, i64 %fid, i64 %eid)
  ret { i64, i64 } %result
}

define internal i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry.address, i64 %context) {
entry:
  %result = call { i64, i64 } @__ncc_kernel_riscv_sbi(i64 %hardware, i64 %entry.address, i64 %context, i64 0, i64 0, i64 0, i64 0, i64 4739917)
  %error = extractvalue { i64, i64 } %result, 0
  %success = icmp eq i64 %error, 0
  %already = icmp eq i64 %error, -7
  %accepted = or i1 %success, %already
  br i1 %accepted, label %poll, label %fail
poll:
  %i = phi i32 [ 0, %entry ], [ %next, %again ]
  %status.result = call { i64, i64 } @__ncc_kernel_riscv_sbi(i64 %hardware, i64 0, i64 0, i64 0, i64 0, i64 0, i64 2, i64 4739917)
  %status.error = extractvalue { i64, i64 } %status.result, 0
  %status = extractvalue { i64, i64 } %status.result, 1
  %status.ok = icmp eq i64 %status.error, 0
  %online = icmp eq i64 %status, 0
  %ready = and i1 %status.ok, %online
  br i1 %ready, label %ok, label %status.check
status.check:
  %stopped = icmp eq i64 %status, 1
  %pending = icmp eq i64 %status, 2
  %retry.state = or i1 %stopped, %pending
  %retry = and i1 %status.ok, %retry.state
  br i1 %retry, label %again, label %fail
again:
  call void asm sideeffect "nop", "~{memory}"()
  %next = add nuw i32 %i, 1
  %timeout = icmp uge i32 %next, 1000000
  br i1 %timeout, label %fail, label %poll
ok:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_ipi_send_physical(i64 %hardware, i64 %vector) {
entry:
  %offset = and i64 %hardware, 63
  %mask = shl i64 1, %offset
  %base = and i64 %hardware, -64
  %result = call { i64, i64 } @__ncc_kernel_riscv_sbi(i64 %mask, i64 %base, i64 0, i64 0, i64 0, i64 0, i64 0, i64 7553097)
  %error = extractvalue { i64, i64 } %result, 0
  %ok = icmp eq i64 %error, 0
  ret i1 %ok
}

define internal i1 @__ncc_kernel_arch_timer_arm_physical(i64 %deadline, i64 %vector) {
entry:
  %vector.ok = icmp eq i64 %vector, 5
  br i1 %vector.ok, label %arm, label %fail
arm:
  %result = call { i64, i64 } @__ncc_kernel_riscv_sbi(i64 %deadline, i64 0, i64 0, i64 0, i64 0, i64 0, i64 0, i64 1414090053)
  %error = extractvalue { i64, i64 } %result, 0
  %ok = icmp eq i64 %error, 0
  ret i1 %ok
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid) {
entry:
  call void asm sideeffect "sfence.vma $0, $1", "r,r,~{memory}"(i64 %address, i64 %asid)
  ret i1 true
}

define internal i1 @__ncc_kernel_arch_plic_init(i64 %base, i64 %context) {
entry:
  %present = icmp ne i64 %base, 0
  br i1 %present, label %configure, label %fail
configure:
  %context.offset = mul i64 %context, 4096
  %threshold.address0 = add i64 %base, 2097152
  %threshold.address = add i64 %threshold.address0, %context.offset
  %threshold.ptr = inttoptr i64 %threshold.address to ptr
  store volatile i32 0, ptr %threshold.ptr, align 4
  store i64 %base, ptr @__ncc_kernel_smp_primary_base, align 8
  store i64 %context, ptr @__ncc_kernel_smp_secondary_base, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_imsic_init(i64 %base) {
entry:
  %present = icmp ne i64 %base, 0
  br i1 %present, label %configure, label %fail
configure:
  store i64 %base, ptr @__ncc_kernel_riscv_imsic_base, align 8
  call void asm sideeffect "csrw 0x150, $0; csrw 0x151, $1; csrw 0x150, $2; csrw 0x151, $3", "r,r,r,r,~{memory}"(i64 112, i64 1, i64 114, i64 0)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_plic_enable(i64 %source, i64 %priority, i64 %context) {
entry:
  %source.positive = icmp ugt i64 %source, 0
  %source.bound = icmp ult i64 %source, 1024
  %priority.ok = icmp ule i64 %priority, 7
  %a = and i1 %source.positive, %source.bound
  %valid = and i1 %a, %priority.ok
  br i1 %valid, label %configure, label %fail
configure:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %priority.offset = mul i64 %source, 4
  %priority.address = add i64 %base, %priority.offset
  %priority.ptr = inttoptr i64 %priority.address to ptr
  %priority32 = trunc i64 %priority to i32
  store volatile i32 %priority32, ptr %priority.ptr, align 4
  %context.offset = mul i64 %context, 128
  %enable.base0 = add i64 %base, 8192
  %enable.base = add i64 %enable.base0, %context.offset
  %word = lshr i64 %source, 5
  %word.offset = mul i64 %word, 4
  %enable.address = add i64 %enable.base, %word.offset
  %enable.ptr = inttoptr i64 %enable.address to ptr
  %old = load volatile i32, ptr %enable.ptr, align 4
  %bit.index64 = and i64 %source, 31
  %bit.index = trunc i64 %bit.index64 to i32
  %bit = shl i32 1, %bit.index
  %new = or i32 %old, %bit
  store volatile i32 %new, ptr %enable.ptr, align 4
  fence seq_cst
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_kernel_arch_plic_claim(i64 %context) {
entry:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %context.offset = mul i64 %context, 4096
  %claim.base = add i64 %base, 2097156
  %claim.address = add i64 %claim.base, %context.offset
  %claim.ptr = inttoptr i64 %claim.address to ptr
  %source32 = load volatile i32, ptr %claim.ptr, align 4
  %source = zext i32 %source32 to i64
  ret i64 %source
}

define internal i1 @__ncc_kernel_arch_plic_complete(i64 %context, i64 %source) {
entry:
  %valid = icmp ult i64 %source, 1024
  br i1 %valid, label %write, label %fail
write:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %context.offset = mul i64 %context, 4096
  %complete.base = add i64 %base, 2097156
  %complete.address = add i64 %complete.base, %context.offset
  %complete.ptr = inttoptr i64 %complete.address to ptr
  %source32 = trunc i64 %source to i32
  store volatile i32 %source32, ptr %complete.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_imsic_enable(i64 %interrupt) {
entry:
  %positive = icmp ugt i64 %interrupt, 0
  %bounded = icmp ult i64 %interrupt, 2048
  %valid = and i1 %positive, %bounded
  br i1 %valid, label %select, label %fail
select:
  %word = lshr i64 %interrupt, 6
  %selector = add i64 %word, 192
  %old = call i64 asm sideeffect "csrw 0x150, $1; csrr $0, 0x151", "=r,r,~{memory}"(i64 %selector)
  %bit.index = and i64 %interrupt, 63
  %bit = shl i64 1, %bit.index
  %new = or i64 %old, %bit
  call void asm sideeffect "csrw 0x150, $0; csrw 0x151, $1", "r,r,~{memory}"(i64 %selector, i64 %new)
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_kernel_arch_imsic_claim() {
entry:
  %raw = call i64 asm sideeffect "csrrw $0, 0x15c, zero", "=r,~{memory}"()
  %shift = lshr i64 %raw, 16
  %interrupt = and i64 %shift, 2047
  ret i64 %interrupt
}

define internal i1 @__ncc_kernel_arch_imsic_complete(i64 %interrupt) {
entry:
  %positive = icmp ugt i64 %interrupt, 0
  %bounded = icmp ult i64 %interrupt, 2048
  %valid = and i1 %positive, %bounded
  br i1 %valid, label %write, label %fail
write:
  %value = shl i64 %interrupt, 16
  call void asm sideeffect "csrw 0x15c, $0", "r,~{memory}"(i64 %value)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_enable(i64 %interrupt, i64 %priority, i64 %edge) { ret i1 false }
define internal i64 @__ncc_kernel_arch_gic_ack() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_gic_eoi(i64 %interrupt) { ret i1 false }
"#;

const RISCV32_RUNTIME: &str = r#"
define internal i64 @__ncc_kernel_arch_bootstrap_hardware_id() { ret i64 0 }
define internal i64 @__ncc_kernel_arch_topology_pack(i64 %hardware) {
entry:
  %core = and i64 %hardware, 65535
  %packed = shl i64 %core, 16
  ret i64 %packed
}
define internal i64 @__ncc_kernel_arch_timer_frequency() { ret i64 0 }
define internal i64 @__ncc_kernel_arch_cpu_local_get() {
entry:
  %cpu = call i32 asm sideeffect "mv $0, tp", "=r"()
  %wide = zext i32 %cpu to i64
  ret i64 %wide
}
define internal void @__ncc_kernel_arch_cpu_local_set(i64 %cpu) {
entry:
  %low = trunc i64 %cpu to i32
  call void asm sideeffect "mv tp, $0", "r,~{memory}"(i32 %low)
  ret void
}
define internal void @__ncc_kernel_arch_cpu_online_commit(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_cpu_start_abort(i64 %cpu) { ret void }
define internal void @__ncc_kernel_arch_smp_relax() {
entry:
  call void asm sideeffect "nop", "~{memory}"()
  ret void
}

define internal { i32, i32 } @__ncc_kernel_riscv_sbi32(i32 %a0, i32 %a1, i32 %a2, i32 %a3, i32 %a4, i32 %a5, i32 %fid, i32 %eid) {
entry:
  %result = call { i32, i32 } asm sideeffect "ecall", "={x10},={x11},0,1,{x12},{x13},{x14},{x15},{x16},{x17},~{memory}"(i32 %a0, i32 %a1, i32 %a2, i32 %a3, i32 %a4, i32 %a5, i32 %fid, i32 %eid)
  ret { i32, i32 } %result
}

define internal i1 @__ncc_kernel_arch_cpu_start_physical(i64 %cpu, i64 %hardware, i64 %stack, i64 %entry.address, i64 %context) {
entry:
  %hart32 = trunc i64 %hardware to i32
  %entry32 = trunc i64 %entry.address to i32
  %context32 = trunc i64 %context to i32
  %result = call { i32, i32 } @__ncc_kernel_riscv_sbi32(i32 %hart32, i32 %entry32, i32 %context32, i32 0, i32 0, i32 0, i32 0, i32 4739917)
  %error = extractvalue { i32, i32 } %result, 0
  %success = icmp eq i32 %error, 0
  %already = icmp eq i32 %error, -7
  %accepted = or i1 %success, %already
  br i1 %accepted, label %poll, label %fail
poll:
  %i = phi i32 [ 0, %entry ], [ %next, %again ]
  %status.result = call { i32, i32 } @__ncc_kernel_riscv_sbi32(i32 %hart32, i32 0, i32 0, i32 0, i32 0, i32 0, i32 2, i32 4739917)
  %status.error = extractvalue { i32, i32 } %status.result, 0
  %status = extractvalue { i32, i32 } %status.result, 1
  %status.ok = icmp eq i32 %status.error, 0
  %online = icmp eq i32 %status, 0
  %ready = and i1 %status.ok, %online
  br i1 %ready, label %ok, label %status.check
status.check:
  %stopped = icmp eq i32 %status, 1
  %pending = icmp eq i32 %status, 2
  %retry.state = or i1 %stopped, %pending
  %retry = and i1 %status.ok, %retry.state
  br i1 %retry, label %again, label %fail
again:
  call void asm sideeffect "nop", "~{memory}"()
  %next = add nuw i32 %i, 1
  %timeout = icmp uge i32 %next, 1000000
  br i1 %timeout, label %fail, label %poll
ok:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_ipi_send_physical(i64 %hardware, i64 %vector) {
entry:
  %offset64 = and i64 %hardware, 31
  %offset = trunc i64 %offset64 to i32
  %mask = shl i32 1, %offset
  %base64 = and i64 %hardware, -32
  %base = trunc i64 %base64 to i32
  %result = call { i32, i32 } @__ncc_kernel_riscv_sbi32(i32 %mask, i32 %base, i32 0, i32 0, i32 0, i32 0, i32 0, i32 7553097)
  %error = extractvalue { i32, i32 } %result, 0
  %ok = icmp eq i32 %error, 0
  ret i1 %ok
}

define internal i1 @__ncc_kernel_arch_timer_arm_physical(i64 %deadline, i64 %vector) {
entry:
  %vector.ok = icmp eq i64 %vector, 5
  br i1 %vector.ok, label %arm, label %fail
arm:
  %low = trunc i64 %deadline to i32
  %high64 = lshr i64 %deadline, 32
  %high = trunc i64 %high64 to i32
  %result = call { i32, i32 } @__ncc_kernel_riscv_sbi32(i32 %low, i32 %high, i32 0, i32 0, i32 0, i32 0, i32 0, i32 1414090053)
  %error = extractvalue { i32, i32 } %result, 0
  %ok = icmp eq i32 %error, 0
  ret i1 %ok
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_tlb_invalidate(i64 %address, i64 %length, i64 %asid) {
entry:
  %address32 = trunc i64 %address to i32
  %asid32 = trunc i64 %asid to i32
  call void asm sideeffect "sfence.vma $0, $1", "r,r,~{memory}"(i32 %address32, i32 %asid32)
  ret i1 true
}

define internal i1 @__ncc_kernel_arch_plic_init(i64 %base, i64 %context) {
entry:
  %present = icmp ne i64 %base, 0
  br i1 %present, label %configure, label %fail
configure:
  %context.offset = mul i64 %context, 4096
  %threshold.address0 = add i64 %base, 2097152
  %threshold.address = add i64 %threshold.address0, %context.offset
  %threshold.ptr = inttoptr i64 %threshold.address to ptr
  store volatile i32 0, ptr %threshold.ptr, align 4
  store i64 %base, ptr @__ncc_kernel_smp_primary_base, align 8
  store i64 %context, ptr @__ncc_kernel_smp_secondary_base, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_imsic_init(i64 %base) {
entry:
  %present = icmp ne i64 %base, 0
  br i1 %present, label %configure, label %fail
configure:
  store i64 %base, ptr @__ncc_kernel_riscv_imsic_base, align 8
  call void asm sideeffect "csrw 0x150, $0; csrw 0x151, $1; csrw 0x150, $2; csrw 0x151, $3", "r,r,r,r,~{memory}"(i32 112, i32 1, i32 114, i32 0)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_plic_enable(i64 %source, i64 %priority, i64 %context) {
entry:
  %source.positive = icmp ugt i64 %source, 0
  %source.bound = icmp ult i64 %source, 1024
  %priority.ok = icmp ule i64 %priority, 7
  %a = and i1 %source.positive, %source.bound
  %valid = and i1 %a, %priority.ok
  br i1 %valid, label %configure, label %fail
configure:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %priority.offset = mul i64 %source, 4
  %priority.address = add i64 %base, %priority.offset
  %priority.ptr = inttoptr i64 %priority.address to ptr
  %priority32 = trunc i64 %priority to i32
  store volatile i32 %priority32, ptr %priority.ptr, align 4
  %context.offset = mul i64 %context, 128
  %enable.base0 = add i64 %base, 8192
  %enable.base = add i64 %enable.base0, %context.offset
  %word = lshr i64 %source, 5
  %word.offset = mul i64 %word, 4
  %enable.address = add i64 %enable.base, %word.offset
  %enable.ptr = inttoptr i64 %enable.address to ptr
  %old = load volatile i32, ptr %enable.ptr, align 4
  %bit.index64 = and i64 %source, 31
  %bit.index = trunc i64 %bit.index64 to i32
  %bit = shl i32 1, %bit.index
  %new = or i32 %old, %bit
  store volatile i32 %new, ptr %enable.ptr, align 4
  fence seq_cst
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_kernel_arch_plic_claim(i64 %context) {
entry:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %context.offset = mul i64 %context, 4096
  %claim.base = add i64 %base, 2097156
  %claim.address = add i64 %claim.base, %context.offset
  %claim.ptr = inttoptr i64 %claim.address to ptr
  %source32 = load volatile i32, ptr %claim.ptr, align 4
  %source = zext i32 %source32 to i64
  ret i64 %source
}

define internal i1 @__ncc_kernel_arch_plic_complete(i64 %context, i64 %source) {
entry:
  %valid = icmp ult i64 %source, 1024
  br i1 %valid, label %write, label %fail
write:
  %base = load i64, ptr @__ncc_kernel_smp_primary_base, align 8
  %context.offset = mul i64 %context, 4096
  %complete.base = add i64 %base, 2097156
  %complete.address = add i64 %complete.base, %context.offset
  %complete.ptr = inttoptr i64 %complete.address to ptr
  %source32 = trunc i64 %source to i32
  store volatile i32 %source32, ptr %complete.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_imsic_enable(i64 %interrupt) {
entry:
  %positive = icmp ugt i64 %interrupt, 0
  %bounded = icmp ult i64 %interrupt, 2048
  %valid = and i1 %positive, %bounded
  br i1 %valid, label %select, label %fail
select:
  %word64 = lshr i64 %interrupt, 5
  %selector64 = add i64 %word64, 192
  %selector = trunc i64 %selector64 to i32
  %old = call i32 asm sideeffect "csrw 0x150, $1; csrr $0, 0x151", "=r,r,~{memory}"(i32 %selector)
  %bit.index64 = and i64 %interrupt, 31
  %bit.index = trunc i64 %bit.index64 to i32
  %bit = shl i32 1, %bit.index
  %new = or i32 %old, %bit
  call void asm sideeffect "csrw 0x150, $0; csrw 0x151, $1", "r,r,~{memory}"(i32 %selector, i32 %new)
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_kernel_arch_imsic_claim() {
entry:
  %raw = call i32 asm sideeffect "csrrw $0, 0x15c, zero", "=r,~{memory}"()
  %shift = lshr i32 %raw, 16
  %interrupt32 = and i32 %shift, 2047
  %interrupt = zext i32 %interrupt32 to i64
  ret i64 %interrupt
}

define internal i1 @__ncc_kernel_arch_imsic_complete(i64 %interrupt) {
entry:
  %positive = icmp ugt i64 %interrupt, 0
  %bounded = icmp ult i64 %interrupt, 2048
  %valid = and i1 %positive, %bounded
  br i1 %valid, label %write, label %fail
write:
  %value64 = shl i64 %interrupt, 16
  %value = trunc i64 %value64 to i32
  call void asm sideeffect "csrw 0x15c, $0", "r,~{memory}"(i32 %value)
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_arch_x86_tables_load(i64 %gdt, i64 %idt, i64 %tss) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_init(i64 %gicd, i64 %gicr) { ret i1 false }
define internal i1 @__ncc_kernel_arch_gic_enable(i64 %interrupt, i64 %priority, i64 %edge) { ret i1 false }
define internal i64 @__ncc_kernel_arch_gic_ack() { ret i64 0 }
define internal i1 @__ncc_kernel_arch_gic_eoi(i64 %interrupt) { ret i1 false }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::process::Command;

    fn push_be32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_be64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn fdt_align(bytes: &mut Vec<u8>) {
        while bytes.len() & 3 != 0 {
            bytes.push(0);
        }
    }

    fn fdt_begin_node(structure: &mut Vec<u8>, name: &str) {
        push_be32(structure, 1);
        structure.extend_from_slice(name.as_bytes());
        structure.push(0);
        fdt_align(structure);
    }

    fn fdt_end_node(structure: &mut Vec<u8>) {
        push_be32(structure, 2);
    }

    fn fdt_property(
        structure: &mut Vec<u8>,
        offsets: &BTreeMap<&'static str, u32>,
        name: &'static str,
        value: &[u8],
    ) {
        push_be32(structure, 3);
        push_be32(structure, value.len() as u32);
        push_be32(structure, offsets[name]);
        structure.extend_from_slice(value);
        fdt_align(structure);
    }

    fn physical_fdt_fixture() -> Vec<u8> {
        let names = [
            "#address-cells",
            "#size-cells",
            "device_type",
            "reg",
            "bootargs",
            "compatible",
            "width",
            "height",
            "stride",
            "format",
        ];
        let mut strings = Vec::new();
        let mut offsets = BTreeMap::new();
        for name in names {
            offsets.insert(name, strings.len() as u32);
            strings.extend_from_slice(name.as_bytes());
            strings.push(0);
        }

        let mut structure = Vec::new();
        fdt_begin_node(&mut structure, "");
        fdt_property(
            &mut structure,
            &offsets,
            "#address-cells",
            &2_u32.to_be_bytes(),
        );
        fdt_property(
            &mut structure,
            &offsets,
            "#size-cells",
            &2_u32.to_be_bytes(),
        );

        fdt_begin_node(&mut structure, "memory@80000000");
        fdt_property(&mut structure, &offsets, "device_type", b"memory\0");
        let mut memory_reg = Vec::new();
        push_be64(&mut memory_reg, 0x8000_0000);
        push_be64(&mut memory_reg, 0x0800_0000);
        fdt_property(&mut structure, &offsets, "reg", &memory_reg);
        fdt_end_node(&mut structure);

        fdt_begin_node(&mut structure, "cpus");
        fdt_property(
            &mut structure,
            &offsets,
            "#address-cells",
            &1_u32.to_be_bytes(),
        );
        fdt_property(
            &mut structure,
            &offsets,
            "#size-cells",
            &0_u32.to_be_bytes(),
        );
        for hart in 0..2_u32 {
            fdt_begin_node(&mut structure, &format!("cpu@{hart}"));
            fdt_property(&mut structure, &offsets, "device_type", b"cpu\0");
            fdt_property(&mut structure, &offsets, "reg", &hart.to_be_bytes());
            fdt_end_node(&mut structure);
        }
        fdt_end_node(&mut structure);

        fdt_begin_node(&mut structure, "chosen");
        fdt_property(
            &mut structure,
            &offsets,
            "bootargs",
            b"console=ttyS0 root=/dev/vda\0",
        );
        fdt_begin_node(&mut structure, "framebuffer@90000000");
        fdt_property(
            &mut structure,
            &offsets,
            "compatible",
            b"simple-framebuffer\0",
        );
        let mut framebuffer_reg = Vec::new();
        push_be64(&mut framebuffer_reg, 0x9000_0000);
        push_be64(&mut framebuffer_reg, 0x0080_0000);
        fdt_property(&mut structure, &offsets, "reg", &framebuffer_reg);
        fdt_property(&mut structure, &offsets, "width", &1024_u32.to_be_bytes());
        fdt_property(&mut structure, &offsets, "height", &768_u32.to_be_bytes());
        fdt_property(&mut structure, &offsets, "stride", &4096_u32.to_be_bytes());
        fdt_property(&mut structure, &offsets, "format", b"a8r8g8b8\0");
        fdt_end_node(&mut structure);
        fdt_end_node(&mut structure);
        fdt_end_node(&mut structure);
        push_be32(&mut structure, 9);

        let reserve_offset = 40_u32;
        let structure_offset = 72_u32;
        let strings_offset = structure_offset + structure.len() as u32;
        let total_size = strings_offset + strings.len() as u32;
        let mut fdt = Vec::with_capacity(total_size as usize);
        push_be32(&mut fdt, 0xd00d_feed);
        push_be32(&mut fdt, total_size);
        push_be32(&mut fdt, structure_offset);
        push_be32(&mut fdt, strings_offset);
        push_be32(&mut fdt, reserve_offset);
        push_be32(&mut fdt, 17);
        push_be32(&mut fdt, 16);
        push_be32(&mut fdt, 0);
        push_be32(&mut fdt, strings.len() as u32);
        push_be32(&mut fdt, structure.len() as u32);
        push_be64(&mut fdt, 0x8100_0000);
        push_be64(&mut fdt, 0x1000);
        push_be64(&mut fdt, 0);
        push_be64(&mut fdt, 0);
        fdt.extend_from_slice(&structure);
        fdt.extend_from_slice(&strings);
        assert_eq!(fdt.len(), total_size as usize);
        fdt
    }

    fn llvm_bytes(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("\\{byte:02x}")).collect()
    }

    #[test]
    fn common_runtime_materializes_boot_topology_and_shootdown() {
        let llvm = render_kernel_firmware_smp_runtime("x86_64-unknown-none");
        assert!(llvm.contains("define i1 @__ncc_kernel_boot_valid"));
        assert!(llvm.contains("define i1 @__ncc_kernel_smp_discover"));
        assert!(llvm.contains("define i64 @__ncc_kernel_boot_framebuffer_base"));
        assert!(llvm.contains("define i64 @__ncc_kernel_boot_framebuffer_width"));
        assert!(llvm.contains("define i64 @__ncc_kernel_boot_framebuffer_height"));
        assert!(llvm.contains("define i64 @__ncc_kernel_boot_framebuffer_pitch"));
        assert!(llvm.contains("define i64 @__ncc_kernel_boot_framebuffer_format"));
        assert!(llvm.contains("define i64 @__ncc_kernel_boot_framebuffer_flags"));
        assert!(llvm.contains("@__ncc_kernel_physical_cpu_stack_top"));
        assert!(llvm.contains("define i1 @__ncc_kernel_tlb_shootdown"));
        assert!(llvm.contains("@__ncc_kernel_tlb_lock"));
        assert!(llvm.contains("define i1 @__ncc_kernel_cpu_wait_online"));
        assert!(llvm.contains("define i1 @__ncc_kernel_smp_configure_firmware"));
        assert!(llvm.contains("define i1 @__ncc_kernel_smp_interrupt_init_firmware"));
        assert!(llvm.contains("define internal i1 @__ncc_kernel_cpu_mark_parked"));
        assert!(llvm.contains("INIT") == false);
    }

    #[test]
    fn architecture_runtimes_materialize_physical_protocols() {
        let x86 = render_kernel_firmware_smp_runtime("x86_64-unknown-none");
        assert!(x86.contains("@__ncc_kernel_x86_apic_command"));
        assert!(x86.contains("@__ncc_kernel_x86_ap_trampoline"));
        assert!(x86.contains("constant [314 x i8]"));
        assert!(x86.contains("@__ncc_kernel_x86_ap_mailbox_lock"));
        assert!(x86.contains("@__ncc_kernel_x86_ap_prepare"));
        assert!(x86.contains("add i64 28672, %index"));
        assert!(x86.contains("lshr i64 28672, 12"));
        assert!(x86.contains("wrmsr"));
        let arm = render_kernel_firmware_smp_runtime("aarch64-unknown-none");
        assert!(arm.contains("smc #0"));
        assert!(arm.contains("icc_sgi1r_el1"));
        assert!(arm.contains("icc_iar1_el1"));
        assert!(arm.contains("icc_eoir1_el1"));
        let riscv = render_kernel_firmware_smp_runtime("riscv64-unknown-elf");
        assert!(riscv.contains("ecall"));
        assert!(riscv.contains("sfence.vma"));
        assert!(riscv.contains("csrw 0x150"));
        assert!(riscv.contains("csrrw $0, 0x15c, zero"));
    }

    #[test]
    fn architecture_runtimes_are_accepted_by_llvm_backends() {
        if Command::new("clang").arg("--version").output().is_err() {
            return;
        }
        for (target, runtime_target) in [
            ("x86_64-unknown-none-elf", "x86_64-unknown-none"),
            ("i386-unknown-none-elf", "i386-unknown-none-elf"),
            ("aarch64-unknown-none-elf", "aarch64-unknown-none"),
            ("riscv64-unknown-elf", "riscv64-unknown-elf"),
            ("riscv32-unknown-elf", "riscv32imac-unknown-elf"),
        ] {
            let stem = target.replace('-', "_");
            let input = std::env::temp_dir().join(format!(
                "ncc_kernel_firmware_smp_{}_{}.ll",
                stem,
                std::process::id()
            ));
            let output = input.with_extension("o");
            let module = format!(
                "target triple = \"{target}\"\n@__ncc_kernel_boot_info_slot = internal global i64 0, align 8\n@__ncc_kernel_current_cpu = internal global i64 0, align 8\ndeclare i64 @llvm.readcyclecounter()\n{}",
                render_kernel_firmware_smp_runtime(runtime_target)
            );
            std::fs::write(&input, module).expect("write LLVM fixture");
            let result = Command::new("clang")
                .args(["-c", "-O2", "-ffreestanding", "-target", target])
                .arg(&input)
                .arg("-o")
                .arg(&output)
                .output()
                .expect("run clang");
            let _ = std::fs::remove_file(&input);
            let _ = std::fs::remove_file(&output);
            assert!(
                result.status.success(),
                "LLVM rejected {target}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }

    #[test]
    fn fdt_adapter_executes_outside_qemu() {
        if Command::new("clang").arg("--version").output().is_err() {
            return;
        }
        let fdt = physical_fdt_fixture();
        let input =
            std::env::temp_dir().join(format!("ncc_kernel_fdt_runtime_{}.ll", std::process::id()));
        let executable = input.with_extension(if cfg!(windows) { "exe" } else { "bin" });
        let module = format!(
            r#"target triple = "x86_64-unknown-linux-gnu"
@__ncc_kernel_boot_info_slot = internal global i64 0, align 8
@__ncc_kernel_current_cpu = internal global i64 0, align 8
@fixture_fdt = private constant [{fdt_len} x i8] c"{fdt_bytes}", align 8
@fixture_boot = internal global [88 x i8] zeroinitializer, align 8
@fixture_memory = internal global [8192 x i8] zeroinitializer, align 8
@fixture_framebuffer = internal global [32 x i8] zeroinitializer, align 8
@fixture_platform = internal global [80 x i8] zeroinitializer, align 8
@fixture_topology = internal global [12320 x i8] zeroinitializer, align 8
declare i64 @llvm.readcyclecounter()
{runtime}

define i32 @main() {{
entry:
  %boot.address = ptrtoint ptr @fixture_boot to i64
  %memory.address = ptrtoint ptr @fixture_memory to i64
  %framebuffer.address = ptrtoint ptr @fixture_framebuffer to i64
  %platform.address = ptrtoint ptr @fixture_platform to i64
  %topology.address = ptrtoint ptr @fixture_topology to i64
  %fdt.address = ptrtoint ptr @fixture_fdt to i64
  store i64 13879475691537230, ptr @fixture_boot, align 8
  store i64 1, ptr getelementptr (i8, ptr @fixture_boot, i64 8), align 8
  store i64 4, ptr getelementptr (i8, ptr @fixture_boot, i64 16), align 8
  store i64 %memory.address, ptr getelementptr (i8, ptr @fixture_boot, i64 24), align 8
  store i64 0, ptr getelementptr (i8, ptr @fixture_boot, i64 32), align 8
  store i64 %framebuffer.address, ptr getelementptr (i8, ptr @fixture_boot, i64 40), align 8
  store i64 0, ptr getelementptr (i8, ptr @fixture_boot, i64 48), align 8
  store i64 %platform.address, ptr getelementptr (i8, ptr @fixture_boot, i64 56), align 8
  store i64 0, ptr getelementptr (i8, ptr @fixture_boot, i64 64), align 8
  store i64 13884960599655246, ptr @fixture_platform, align 8
  store i32 1, ptr getelementptr (i8, ptr @fixture_platform, i64 8), align 4
  store i32 2, ptr getelementptr (i8, ptr @fixture_platform, i64 12), align 4
  store i32 6, ptr getelementptr (i8, ptr @fixture_platform, i64 16), align 4
  store i32 105, ptr getelementptr (i8, ptr @fixture_platform, i64 20), align 4
  store i64 %fdt.address, ptr getelementptr (i8, ptr @fixture_platform, i64 24), align 8
  store i32 10, ptr getelementptr (i8, ptr @fixture_platform, i64 48), align 4
  store i32 7, ptr getelementptr (i8, ptr @fixture_platform, i64 52), align 4
  store i64 %topology.address, ptr getelementptr (i8, ptr @fixture_platform, i64 56), align 8
  store i64 12320, ptr getelementptr (i8, ptr @fixture_platform, i64 64), align 8
  store i64 13886077073048398, ptr @fixture_topology, align 8
  store i32 1, ptr getelementptr (i8, ptr @fixture_topology, i64 8), align 4
  store i64 %boot.address, ptr @__ncc_kernel_boot_info_slot, align 8
  %valid = call i1 @__ncc_kernel_boot_valid()
  %count = call i64 @__ncc_kernel_boot_memory_count()
  %count.ok = icmp eq i64 %count, 2
  %base = call i64 @__ncc_kernel_boot_memory_base(i64 1)
  %base.ok = icmp eq i64 %base, 2147483648
  %command = call i64 @__ncc_kernel_boot_command_line()
  %command.ok = icmp ne i64 %command, 0
  %framebuffer = call i64 @__ncc_kernel_boot_framebuffer()
  %framebuffer.ok = icmp eq i64 %framebuffer, %framebuffer.address
  %smp = call i1 @__ncc_kernel_smp_discover()
  %cpus = call i64 @__ncc_kernel_cpu_count_physical()
  %cpus.ok = icmp eq i64 %cpus, 2
  %valid.fail = xor i1 %valid, true
  %count.fail = xor i1 %count.ok, true
  %base.fail = xor i1 %base.ok, true
  %command.fail = xor i1 %command.ok, true
  %framebuffer.fail = xor i1 %framebuffer.ok, true
  %smp.fail = xor i1 %smp, true
  %cpus.fail = xor i1 %cpus.ok, true
  %valid.bit = zext i1 %valid.fail to i32
  %count.raw = zext i1 %count.fail to i32
  %count.bit = shl i32 %count.raw, 1
  %base.raw = zext i1 %base.fail to i32
  %base.bit = shl i32 %base.raw, 2
  %command.raw = zext i1 %command.fail to i32
  %command.bit = shl i32 %command.raw, 3
  %framebuffer.raw = zext i1 %framebuffer.fail to i32
  %framebuffer.bit = shl i32 %framebuffer.raw, 4
  %smp.raw = zext i1 %smp.fail to i32
  %smp.bit = shl i32 %smp.raw, 5
  %cpus.raw = zext i1 %cpus.fail to i32
  %cpus.bit = shl i32 %cpus.raw, 6
  %ab = or i32 %valid.bit, %count.bit
  %cd = or i32 %base.bit, %command.bit
  %ef = or i32 %framebuffer.bit, %smp.bit
  %abcd = or i32 %ab, %cd
  %efg = or i32 %ef, %cpus.bit
  %status = or i32 %abcd, %efg
  ret i32 %status
}}
"#,
            fdt_len = fdt.len(),
            fdt_bytes = llvm_bytes(&fdt),
            runtime = render_kernel_firmware_smp_runtime("x86_64-unknown-linux-gnu"),
        );
        std::fs::write(&input, module).expect("write executable FDT LLVM fixture");
        let compile = Command::new("clang")
            .args(["-O2", "-x", "ir"])
            .arg(&input)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("compile executable FDT fixture");
        assert!(
            compile.status.success(),
            "LLVM rejected executable FDT fixture: {}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&executable)
            .output()
            .expect("execute FDT fixture outside QEMU");
        assert!(
            run.status.success(),
            "FDT runtime fixture exited with {:?}: input={} executable={} stdout={} stderr={}",
            run.status,
            input.display(),
            executable.display(),
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&executable);
    }
}
