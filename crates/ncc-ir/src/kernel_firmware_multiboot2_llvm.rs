pub(crate) fn render_kernel_firmware_multiboot2_runtime(target_triple: &str) -> String {
    let target = target_triple.to_ascii_lowercase();
    if target.starts_with("x86_64") || target.starts_with("i386") {
        MULTIBOOT2_RUNTIME.to_string()
    } else if target.starts_with("aarch64") {
        format!("{RAW_FDT_RUNTIME}\n{AARCH64_FDT_ENTRY_RUNTIME}")
    } else if target.starts_with("riscv64") {
        format!("{RAW_FDT_RUNTIME}\n{RISCV64_OPENSBI_ENTRY_RUNTIME}")
    } else if target.starts_with("riscv32") {
        format!("{RAW_FDT_RUNTIME}\n{RISCV32_OPENSBI_ENTRY_RUNTIME}")
    } else {
        PASSTHROUGH_RUNTIME.to_string()
    }
}

const PASSTHROUGH_RUNTIME: &str = r#"
define i64 @__ncc_kernel_normalize_boot_info(i64 %raw.address) {
entry:
  ret i64 %raw.address
}
"#;

const RAW_FDT_RUNTIME: &str = r#"
@__ncc_kernel_raw_fdt_boot = internal global [88 x i8] zeroinitializer, align 16
@__ncc_kernel_raw_fdt_memory = internal global [8192 x i8] zeroinitializer, align 16
@__ncc_kernel_raw_fdt_platform = internal global [112 x i8] zeroinitializer, align 16
@__ncc_kernel_raw_fdt_topology = internal global [12320 x i8] zeroinitializer, align 16
@__ncc_kernel_opensbi_hart = internal global i64 0, align 8

define internal i64 @__ncc_kernel_build_fdt_boot_info(i64 %fdt.address, i64 %hart, i1 %include.hart) {
entry:
  %zero.boot = call ptr @memset(ptr @__ncc_kernel_raw_fdt_boot, i32 0, i64 88)
  %zero.memory = call ptr @memset(ptr @__ncc_kernel_raw_fdt_memory, i32 0, i64 8192)
  %zero.platform = call ptr @memset(ptr @__ncc_kernel_raw_fdt_platform, i32 0, i64 112)
  %zero.topology = call ptr @memset(ptr @__ncc_kernel_raw_fdt_topology, i32 0, i64 12320)

  store i64 13879475691537230, ptr @__ncc_kernel_raw_fdt_boot, align 16
  %boot.version = getelementptr i8, ptr @__ncc_kernel_raw_fdt_boot, i64 8
  store i64 1, ptr %boot.version, align 8
  %boot.flags = getelementptr i8, ptr @__ncc_kernel_raw_fdt_boot, i64 16
  store i64 20, ptr %boot.flags, align 8
  %memory.address = ptrtoint ptr @__ncc_kernel_raw_fdt_memory to i64
  %boot.memory = getelementptr i8, ptr @__ncc_kernel_raw_fdt_boot, i64 24
  store i64 %memory.address, ptr %boot.memory, align 8
  %platform.address = ptrtoint ptr @__ncc_kernel_raw_fdt_platform to i64
  %boot.platform = getelementptr i8, ptr @__ncc_kernel_raw_fdt_boot, i64 56
  store i64 %platform.address, ptr %boot.platform, align 8

  store i64 13884960599655246, ptr @__ncc_kernel_raw_fdt_platform, align 16
  %platform.version = getelementptr i8, ptr @__ncc_kernel_raw_fdt_platform, i64 8
  store i32 1, ptr %platform.version, align 4
  %platform.count.ptr = getelementptr i8, ptr @__ncc_kernel_raw_fdt_platform, i64 12
  %platform.count = select i1 %include.hart, i32 3, i32 2
  store i32 %platform.count, ptr %platform.count.ptr, align 4

  %fdt.entry = getelementptr i8, ptr @__ncc_kernel_raw_fdt_platform, i64 16
  store i32 6, ptr %fdt.entry, align 16
  %fdt.flags = getelementptr i8, ptr %fdt.entry, i64 4
  store i32 73, ptr %fdt.flags, align 4
  %fdt.ptr = getelementptr i8, ptr %fdt.entry, i64 8
  store i64 %fdt.address, ptr %fdt.ptr, align 8

  %topology.entry = getelementptr i8, ptr @__ncc_kernel_raw_fdt_platform, i64 48
  store i32 10, ptr %topology.entry, align 16
  %topology.entry.flags = getelementptr i8, ptr %topology.entry, i64 4
  store i32 5, ptr %topology.entry.flags, align 4
  %topology.address = ptrtoint ptr @__ncc_kernel_raw_fdt_topology to i64
  %topology.entry.ptr = getelementptr i8, ptr %topology.entry, i64 8
  store i64 %topology.address, ptr %topology.entry.ptr, align 8
  %topology.entry.length = getelementptr i8, ptr %topology.entry, i64 16
  store i64 12320, ptr %topology.entry.length, align 8
  %topology.entry.revision = getelementptr i8, ptr %topology.entry, i64 24
  store i64 1, ptr %topology.entry.revision, align 8

  store i64 13886077073048398, ptr @__ncc_kernel_raw_fdt_topology, align 16
  %topology.version = getelementptr i8, ptr @__ncc_kernel_raw_fdt_topology, i64 8
  store i32 1, ptr %topology.version, align 4
  %topology.bootstrap = getelementptr i8, ptr @__ncc_kernel_raw_fdt_topology, i64 24
  store i64 %hart, ptr %topology.bootstrap, align 8
  br i1 %include.hart, label %hart.entry, label %finish

hart.entry:
  %hart.slot = getelementptr i8, ptr @__ncc_kernel_raw_fdt_platform, i64 80
  store i32 7, ptr %hart.slot, align 16
  %hart.flags = getelementptr i8, ptr %hart.slot, i64 4
  store i32 9, ptr %hart.flags, align 4
  %hart.ptr = getelementptr i8, ptr %hart.slot, i64 8
  store i64 %hart, ptr %hart.ptr, align 8
  %hart.length = getelementptr i8, ptr %hart.slot, i64 16
  store i64 8, ptr %hart.length, align 8
  br label %finish

finish:
  %result = ptrtoint ptr @__ncc_kernel_raw_fdt_boot to i64
  ret i64 %result
}

define internal i64 @__ncc_kernel_normalize_fdt_boot_info(i64 %raw.address, i64 %hart, i1 %include.hart) {
entry:
  %present = icmp uge i64 %raw.address, 4096
  br i1 %present, label %inspect, label %fail
inspect:
  %raw = inttoptr i64 %raw.address to ptr
  %magic = load i64, ptr %raw, align 1
  %canonical = icmp eq i64 %magic, 13879475691537230
  br i1 %canonical, label %already, label %fdt.inspect
already:
  ret i64 %raw.address
fdt.inspect:
  %fdt.magic = load i32, ptr %raw, align 1
  %fdt.valid = icmp eq i32 %fdt.magic, -302117424
  br i1 %fdt.valid, label %build, label %fail
build:
  %result = call i64 @__ncc_kernel_build_fdt_boot_info(i64 %raw.address, i64 %hart, i1 %include.hart)
  ret i64 %result
fail:
  ret i64 0
}
"#;

const AARCH64_FDT_ENTRY_RUNTIME: &str = r#"
define i64 @__ncc_kernel_normalize_boot_info(i64 %raw.address) {
entry:
  %normalized = call i64 @__ncc_kernel_normalize_fdt_boot_info(i64 %raw.address, i64 0, i1 false)
  ret i64 %normalized
}
"#;

const RISCV64_OPENSBI_ENTRY_RUNTIME: &str = r#"
define i64 @__ncc_kernel_normalize_boot_info(i64 %raw.address) {
entry:
  %hart = load i64, ptr @__ncc_kernel_opensbi_hart, align 8
  %normalized = call i64 @__ncc_kernel_normalize_fdt_boot_info(i64 %raw.address, i64 %hart, i1 true)
  ret i64 %normalized
}

define void @__ncc_kernel_set_opensbi_boot_info(i64 %hart, i64 %fdt) {
entry:
  store i64 %hart, ptr @__ncc_kernel_opensbi_hart, align 8
  call void @__ncc_kernel_set_boot_info(i64 %fdt)
  ret void
}
"#;

const RISCV32_OPENSBI_ENTRY_RUNTIME: &str = r#"
define i64 @__ncc_kernel_normalize_boot_info(i64 %raw.address) {
entry:
  %hart = load i64, ptr @__ncc_kernel_opensbi_hart, align 8
  %normalized = call i64 @__ncc_kernel_normalize_fdt_boot_info(i64 %raw.address, i64 %hart, i1 true)
  ret i64 %normalized
}

define void @__ncc_kernel_set_opensbi_boot_info(i32 %hart.raw, i32 %fdt.raw) {
entry:
  %hart = zext i32 %hart.raw to i64
  %fdt = zext i32 %fdt.raw to i64
  store i64 %hart, ptr @__ncc_kernel_opensbi_hart, align 8
  call void @__ncc_kernel_set_boot_info(i64 %fdt)
  ret void
}
"#;

const MULTIBOOT2_RUNTIME: &str = r#"
@__ncc_kernel_mb2_boot = internal global [88 x i8] zeroinitializer, align 16
@__ncc_kernel_mb2_memory = internal global [8192 x i8] zeroinitializer, align 16
@__ncc_kernel_mb2_framebuffer = internal global [32 x i8] zeroinitializer, align 16
@__ncc_kernel_mb2_platform = internal global [1040 x i8] zeroinitializer, align 16
@__ncc_kernel_mb2_command = internal global [4096 x i8] zeroinitializer, align 16
@__ncc_kernel_mb2_memory_count = internal global i64 0, align 8
@__ncc_kernel_mb2_efi_memory = internal global [8192 x i8] zeroinitializer, align 16
@__ncc_kernel_mb2_efi_memory_count = internal global i64 0, align 8
@__ncc_kernel_mb2_detailed_memory = internal global i1 false, align 1
@__ncc_kernel_mb2_basic_lower = internal global i32 0, align 4
@__ncc_kernel_mb2_basic_upper = internal global i32 0, align 4

define internal i64 @__ncc_kernel_mb2_uefi_system_table_size(i64 %address) {
entry:
  %present = icmp uge i64 %address, 4096
  br i1 %present, label %mapping, label %fail
mapping:
  %identity.mapped = icmp ule i64 %address, 4294967272
  br i1 %identity.mapped, label %inspect, label %external.minimum
inspect:
  %table = inttoptr i64 %address to ptr
  %signature = load i64, ptr %table, align 1
  %signature.valid = icmp eq i64 %signature, 6076298535811760713
  br i1 %signature.valid, label %header, label %fail
header:
  %header.size.ptr = getelementptr i8, ptr %table, i64 12
  %header.size.raw = load i32, ptr %header.size.ptr, align 1
  %header.size = zext i32 %header.size.raw to i64
  %header.minimum = icmp uge i64 %header.size, 24
  %header.maximum = icmp ule i64 %header.size, 4096
  %header.valid = and i1 %header.minimum, %header.maximum
  br i1 %header.valid, label %success, label %fail
success:
  ret i64 %header.size
external.minimum:
  ret i64 24
fail:
  ret i64 0
}

define internal i1 @__ncc_kernel_mb2_platform_upsert(i32 %kind, i32 %flags, i64 %address, i64 %length, i64 %revision) {
entry:
  %count.ptr = getelementptr i8, ptr @__ncc_kernel_mb2_platform, i64 12
  %count = load i32, ptr %count.ptr, align 4
  br label %scan
scan:
  %index = phi i32 [ 0, %entry ], [ %next, %miss ]
  %done = icmp uge i32 %index, %count
  br i1 %done, label %append, label %inspect
inspect:
  %wide = zext i32 %index to i64
  %body = mul nuw i64 %wide, 32
  %offset = add nuw i64 %body, 16
  %slot = getelementptr i8, ptr @__ncc_kernel_mb2_platform, i64 %offset
  %candidate = load i32, ptr %slot, align 4
  %same = icmp eq i32 %candidate, %kind
  br i1 %same, label %replace.test, label %miss
replace.test:
  %old.revision.ptr = getelementptr i8, ptr %slot, i64 24
  %old.revision = load i64, ptr %old.revision.ptr, align 8
  %newer = icmp ugt i64 %revision, %old.revision
  br i1 %newer, label %store, label %success
miss:
  %next = add nuw i32 %index, 1
  br label %scan
append:
  %room = icmp ult i32 %count, 32
  br i1 %room, label %append.slot, label %fail
append.slot:
  %count.wide = zext i32 %count to i64
  %append.body = mul nuw i64 %count.wide, 32
  %append.offset = add nuw i64 %append.body, 16
  %append.ptr = getelementptr i8, ptr @__ncc_kernel_mb2_platform, i64 %append.offset
  %new.count = add nuw i32 %count, 1
  store i32 %new.count, ptr %count.ptr, align 4
  br label %store.append
store.append:
  br label %store.common
store:
  br label %store.common
store.common:
  %destination = phi ptr [ %append.ptr, %store.append ], [ %slot, %store ]
  store i32 %kind, ptr %destination, align 4
  %flags.ptr = getelementptr i8, ptr %destination, i64 4
  store i32 %flags, ptr %flags.ptr, align 4
  %address.ptr = getelementptr i8, ptr %destination, i64 8
  store i64 %address, ptr %address.ptr, align 8
  %length.ptr = getelementptr i8, ptr %destination, i64 16
  store i64 %length, ptr %length.ptr, align 8
  %revision.ptr = getelementptr i8, ptr %destination, i64 24
  store i64 %revision, ptr %revision.ptr, align 8
  br label %success
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_memory_append(i64 %base, i64 %length, i32 %kind, i32 %flags) {
entry:
  %nonzero = icmp ne i64 %length, 0
  %end = add i64 %base, %length
  %end.ok = icmp uge i64 %end, %base
  %valid = and i1 %nonzero, %end.ok
  br i1 %valid, label %capacity, label %fail
capacity:
  %count = load i64, ptr @__ncc_kernel_mb2_memory_count, align 8
  %room = icmp ult i64 %count, 256
  br i1 %room, label %store, label %fail
store:
  %offset = mul nuw i64 %count, 32
  %slot = getelementptr i8, ptr @__ncc_kernel_mb2_memory, i64 %offset
  store i64 %base, ptr %slot, align 8
  %length.ptr = getelementptr i8, ptr %slot, i64 8
  store i64 %length, ptr %length.ptr, align 8
  %kind.ptr = getelementptr i8, ptr %slot, i64 16
  store i32 %kind, ptr %kind.ptr, align 4
  %flags.ptr = getelementptr i8, ptr %slot, i64 20
  store i32 %flags, ptr %flags.ptr, align 4
  %source.ptr = getelementptr i8, ptr %slot, i64 24
  store i32 5, ptr %source.ptr, align 4
  %reserved.ptr = getelementptr i8, ptr %slot, i64 28
  store i32 0, ptr %reserved.ptr, align 4
  %next = add nuw i64 %count, 1
  store i64 %next, ptr @__ncc_kernel_mb2_memory_count, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_efi_memory_append(i64 %base, i64 %length, i32 %kind, i32 %flags) {
entry:
  %nonzero = icmp ne i64 %length, 0
  %end = add i64 %base, %length
  %end.ok = icmp uge i64 %end, %base
  %valid = and i1 %nonzero, %end.ok
  br i1 %valid, label %capacity, label %fail
capacity:
  %count = load i64, ptr @__ncc_kernel_mb2_efi_memory_count, align 8
  %room = icmp ult i64 %count, 256
  br i1 %room, label %store, label %fail
store:
  %offset = mul nuw i64 %count, 32
  %slot = getelementptr i8, ptr @__ncc_kernel_mb2_efi_memory, i64 %offset
  store i64 %base, ptr %slot, align 8
  %length.ptr = getelementptr i8, ptr %slot, i64 8
  store i64 %length, ptr %length.ptr, align 8
  %kind.ptr = getelementptr i8, ptr %slot, i64 16
  store i32 %kind, ptr %kind.ptr, align 4
  %flags.ptr = getelementptr i8, ptr %slot, i64 20
  store i32 %flags, ptr %flags.ptr, align 4
  %source.ptr = getelementptr i8, ptr %slot, i64 24
  store i32 4, ptr %source.ptr, align 4
  %reserved.ptr = getelementptr i8, ptr %slot, i64 28
  store i32 0, ptr %reserved.ptr, align 4
  %next = add nuw i64 %count, 1
  store i64 %next, ptr @__ncc_kernel_mb2_efi_memory_count, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_checksum(ptr %bytes, i64 %length) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %sum = phi i8 [ 0, %entry ], [ %new.sum, %body ]
  %done = icmp uge i64 %index, %length
  br i1 %done, label %result, label %body
body:
  %slot = getelementptr i8, ptr %bytes, i64 %index
  %value = load i8, ptr %slot, align 1
  %new.sum = add i8 %sum, %value
  %next = add nuw i64 %index, 1
  br label %loop
result:
  %valid = icmp eq i8 %sum, 0
  ret i1 %valid
}

define internal i1 @__ncc_kernel_mb2_c_string_validate(ptr %bytes, i64 %length) {
entry:
  %nonzero = icmp ne i64 %length, 0
  br i1 %nonzero, label %loop, label %fail
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %continue ]
  %inside = icmp ult i64 %index, %length
  br i1 %inside, label %read, label %fail
read:
  %slot = getelementptr i8, ptr %bytes, i64 %index
  %value = load i8, ptr %slot, align 1
  %end = icmp eq i8 %value, 0
  br i1 %end, label %success, label %continue
continue:
  %next = add nuw i64 %index, 1
  br label %loop
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_acpi_validate(ptr %payload, i64 %length, i1 %modern) {
entry:
  %old.large = icmp uge i64 %length, 20
  br i1 %old.large, label %magic, label %fail
magic:
  %signature = load i64, ptr %payload, align 1
  %signature.ok = icmp eq i64 %signature, 2329016660419433298
  br i1 %signature.ok, label %old.checksum, label %fail
old.checksum:
  %old.valid = call i1 @__ncc_kernel_mb2_checksum(ptr %payload, i64 20)
  br i1 %old.valid, label %version, label %fail
version:
  br i1 %modern, label %modern.header, label %success
modern.header:
  %modern.large = icmp uge i64 %length, 36
  br i1 %modern.large, label %modern.read, label %fail
modern.read:
  %revision.ptr = getelementptr i8, ptr %payload, i64 15
  %revision = load i8, ptr %revision.ptr, align 1
  %revision.ok = icmp uge i8 %revision, 2
  %declared.ptr = getelementptr i8, ptr %payload, i64 20
  %declared.raw = load i32, ptr %declared.ptr, align 1
  %declared = zext i32 %declared.raw to i64
  %declared.minimum = icmp uge i64 %declared, 36
  %declared.inside = icmp ule i64 %declared, %length
  %declared.a = and i1 %revision.ok, %declared.minimum
  %declared.valid = and i1 %declared.a, %declared.inside
  br i1 %declared.valid, label %modern.checksum, label %fail
modern.checksum:
  %modern.valid = call i1 @__ncc_kernel_mb2_checksum(ptr %payload, i64 %declared)
  br i1 %modern.valid, label %success, label %fail
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_efi_memory_map(ptr %payload, i64 %length) {
entry:
  %header.ok = icmp uge i64 %length, 8
  br i1 %header.ok, label %header, label %fail
header:
  %descriptor.size.raw = load i32, ptr %payload, align 4
  %descriptor.size = zext i32 %descriptor.size.raw to i64
  %size.ok = icmp uge i64 %descriptor.size, 40
  br i1 %size.ok, label %shape, label %fail
shape:
  %entries.length.wide = sub i64 %length, 8
  %entries.length = trunc i64 %entries.length.wide to i32
  %remainder = urem i32 %entries.length, %descriptor.size.raw
  %exact = icmp eq i32 %remainder, 0
  br i1 %exact, label %loop, label %fail
loop:
  %offset = phi i64 [ 8, %shape ], [ %next, %stored ]
  %done = icmp uge i64 %offset, %length
  br i1 %done, label %success, label %read
read:
  %slot = getelementptr i8, ptr %payload, i64 %offset
  %raw.kind = load i32, ptr %slot, align 4
  %base.ptr = getelementptr i8, ptr %slot, i64 8
  %base = load i64, ptr %base.ptr, align 8
  %pages.ptr = getelementptr i8, ptr %slot, i64 24
  %pages = load i64, ptr %pages.ptr, align 8
  %pages.ok = icmp ule i64 %pages, 4503599627370495
  br i1 %pages.ok, label %classify, label %fail
classify:
  %region.length = shl i64 %pages, 12
  %loader.low = icmp uge i32 %raw.kind, 1
  %loader.high = icmp ule i32 %raw.kind, 4
  %loader = and i1 %loader.low, %loader.high
  %runtime.code = icmp eq i32 %raw.kind, 5
  %runtime.data = icmp eq i32 %raw.kind, 6
  %runtime = or i1 %runtime.code, %runtime.data
  %usable = icmp eq i32 %raw.kind, 7
  %bad = icmp eq i32 %raw.kind, 8
  %acpi.reclaim = icmp eq i32 %raw.kind, 9
  %acpi.nvs = icmp eq i32 %raw.kind, 10
  %mmio.a = icmp eq i32 %raw.kind, 11
  %mmio.b = icmp eq i32 %raw.kind, 12
  %mmio = or i1 %mmio.a, %mmio.b
  %persistent = icmp eq i32 %raw.kind, 14
  %kind.persistent = select i1 %persistent, i32 11, i32 0
  %kind.mmio = select i1 %mmio, i32 10, i32 %kind.persistent
  %kind.nvs = select i1 %acpi.nvs, i32 7, i32 %kind.mmio
  %kind.acpi = select i1 %acpi.reclaim, i32 6, i32 %kind.nvs
  %kind.bad = select i1 %bad, i32 8, i32 %kind.acpi
  %kind.usable = select i1 %usable, i32 5, i32 %kind.bad
  %kind.runtime = select i1 %runtime, i32 9, i32 %kind.usable
  %kind = select i1 %loader, i32 1, i32 %kind.runtime
  %attributes.ptr = getelementptr i8, ptr %slot, i64 32
  %attributes = load i64, ptr %attributes.ptr, align 8
  %flags = trunc i64 %attributes to i32
  %stored.ok = call i1 @__ncc_kernel_mb2_efi_memory_append(i64 %base, i64 %region.length, i32 %kind, i32 %flags)
  br i1 %stored.ok, label %stored, label %fail
stored:
  %next = add nuw i64 %offset, %descriptor.size
  br label %loop
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_memory_map(ptr %payload, i64 %length) {
entry:
  %header.ok = icmp uge i64 %length, 8
  br i1 %header.ok, label %header, label %fail
header:
  %entry.size.raw = load i32, ptr %payload, align 4
  %entry.size = zext i32 %entry.size.raw to i64
  %size.ok = icmp uge i64 %entry.size, 24
  br i1 %size.ok, label %shape, label %fail
shape:
  %entries.length.wide = sub i64 %length, 8
  %entries.length = trunc i64 %entries.length.wide to i32
  %remainder = urem i32 %entries.length, %entry.size.raw
  %exact = icmp eq i32 %remainder, 0
  br i1 %exact, label %loop, label %fail
loop:
  %offset = phi i64 [ 8, %shape ], [ %next, %stored ]
  %done = icmp uge i64 %offset, %length
  br i1 %done, label %success, label %read
read:
  %slot = getelementptr i8, ptr %payload, i64 %offset
  %base = load i64, ptr %slot, align 8
  %length.ptr = getelementptr i8, ptr %slot, i64 8
  %region.length = load i64, ptr %length.ptr, align 8
  %raw.kind.ptr = getelementptr i8, ptr %slot, i64 16
  %raw.kind = load i32, ptr %raw.kind.ptr, align 4
  %usable = icmp eq i32 %raw.kind, 1
  %acpi.reclaim = icmp eq i32 %raw.kind, 3
  %acpi.nvs = icmp eq i32 %raw.kind, 4
  %bad = icmp eq i32 %raw.kind, 5
  %kind.bad = select i1 %bad, i32 8, i32 0
  %kind.nvs = select i1 %acpi.nvs, i32 7, i32 %kind.bad
  %kind.acpi = select i1 %acpi.reclaim, i32 6, i32 %kind.nvs
  %kind = select i1 %usable, i32 5, i32 %kind.acpi
  %stored.ok = call i1 @__ncc_kernel_mb2_memory_append(i64 %base, i64 %region.length, i32 %kind, i32 %raw.kind)
  br i1 %stored.ok, label %stored, label %fail
stored:
  %next = add nuw i64 %offset, %entry.size
  br label %loop
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_command_line(ptr %payload, i64 %length) {
entry:
  %nonzero = icmp ne i64 %length, 0
  br i1 %nonzero, label %loop, label %fail
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy ]
  %inside = icmp ult i64 %index, %length
  %room = icmp ult i64 %index, 4096
  %valid = and i1 %inside, %room
  br i1 %valid, label %copy, label %fail
copy:
  %source = getelementptr i8, ptr %payload, i64 %index
  %value = load i8, ptr %source, align 1
  %destination = getelementptr i8, ptr @__ncc_kernel_mb2_command, i64 %index
  store i8 %value, ptr %destination, align 1
  %end = icmp eq i8 %value, 0
  %next = add nuw i64 %index, 1
  br i1 %end, label %success, label %loop
success:
  %address = ptrtoint ptr @__ncc_kernel_mb2_command to i64
  %slot = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 64
  store i64 %address, ptr %slot, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_mb2_framebuffer_capture(ptr %payload, i64 %length) {
entry:
  %large = icmp uge i64 %length, 28
  br i1 %large, label %read, label %fail
read:
  %base = load i64, ptr %payload, align 8
  %pitch.ptr = getelementptr i8, ptr %payload, i64 8
  %pitch = load i32, ptr %pitch.ptr, align 4
  %width.ptr = getelementptr i8, ptr %payload, i64 12
  %width = load i32, ptr %width.ptr, align 4
  %height.ptr = getelementptr i8, ptr %payload, i64 16
  %height = load i32, ptr %height.ptr, align 4
  %bpp.ptr = getelementptr i8, ptr %payload, i64 20
  %bpp = load i8, ptr %bpp.ptr, align 1
  %type.ptr = getelementptr i8, ptr %payload, i64 21
  %type = load i8, ptr %type.ptr, align 1
  %red.position.ptr = getelementptr i8, ptr %payload, i64 22
  %red.position = load i8, ptr %red.position.ptr, align 1
  %red.mask.ptr = getelementptr i8, ptr %payload, i64 23
  %red.mask = load i8, ptr %red.mask.ptr, align 1
  %green.position.ptr = getelementptr i8, ptr %payload, i64 24
  %green.position = load i8, ptr %green.position.ptr, align 1
  %green.mask.ptr = getelementptr i8, ptr %payload, i64 25
  %green.mask = load i8, ptr %green.mask.ptr, align 1
  %blue.position.ptr = getelementptr i8, ptr %payload, i64 26
  %blue.position = load i8, ptr %blue.position.ptr, align 1
  %blue.mask.ptr = getelementptr i8, ptr %payload, i64 27
  %blue.mask = load i8, ptr %blue.mask.ptr, align 1
  %base.ok = icmp ne i64 %base, 0
  %width.ok = icmp ne i32 %width, 0
  %height.ok = icmp ne i32 %height, 0
  %bpp.ok = icmp eq i8 %bpp, 32
  %type.ok = icmp eq i8 %type, 1
  %width.wide = zext i32 %width to i64
  %pitch.wide = zext i32 %pitch to i64
  %minimum = mul nuw i64 %width.wide, 4
  %pitch.ok = icmp uge i64 %pitch.wide, %minimum
  %a = and i1 %base.ok, %width.ok
  %b = and i1 %height.ok, %bpp.ok
  %c = and i1 %type.ok, %pitch.ok
  %d = and i1 %a, %b
  %valid = and i1 %c, %d
  br i1 %valid, label %store, label %fail
store:
  %red.zero = icmp eq i8 %red.position, 0
  %red.sixteen = icmp eq i8 %red.position, 16
  %red.eight = icmp eq i8 %red.mask, 8
  %green.eight.position = icmp eq i8 %green.position, 8
  %green.eight.mask = icmp eq i8 %green.mask, 8
  %blue.zero = icmp eq i8 %blue.position, 0
  %blue.sixteen = icmp eq i8 %blue.position, 16
  %blue.eight = icmp eq i8 %blue.mask, 8
  %rgb.a = and i1 %red.zero, %red.eight
  %rgb.b = and i1 %green.eight.position, %green.eight.mask
  %rgb.c = and i1 %blue.sixteen, %blue.eight
  %rgb.ab = and i1 %rgb.a, %rgb.b
  %rgb = and i1 %rgb.ab, %rgb.c
  %bgr.a = and i1 %red.sixteen, %red.eight
  %bgr.c = and i1 %blue.zero, %blue.eight
  %bgr.ab = and i1 %bgr.a, %rgb.b
  %bgr = and i1 %bgr.ab, %bgr.c
  %format.not.rgb = select i1 %bgr, i32 1, i32 2
  %format = select i1 %rgb, i32 0, i32 %format.not.rgb
  store i64 %base, ptr @__ncc_kernel_mb2_framebuffer, align 16
  %dst.width = getelementptr i8, ptr @__ncc_kernel_mb2_framebuffer, i64 8
  store i32 %width, ptr %dst.width, align 4
  %dst.height = getelementptr i8, ptr @__ncc_kernel_mb2_framebuffer, i64 12
  store i32 %height, ptr %dst.height, align 4
  %dst.pitch = getelementptr i8, ptr @__ncc_kernel_mb2_framebuffer, i64 16
  store i32 %pitch, ptr %dst.pitch, align 4
  %dst.format = getelementptr i8, ptr @__ncc_kernel_mb2_framebuffer, i64 20
  store i32 %format, ptr %dst.format, align 4
  %dst.flags = getelementptr i8, ptr @__ncc_kernel_mb2_framebuffer, i64 24
  store i64 3, ptr %dst.flags, align 8
  %address = ptrtoint ptr @__ncc_kernel_mb2_framebuffer to i64
  %slot = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 40
  store i64 %address, ptr %slot, align 8
  %flags.ptr = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 16
  %flags = load i64, ptr %flags.ptr, align 8
  %with.framebuffer = or i64 %flags, 8
  store i64 %with.framebuffer, ptr %flags.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define i64 @__ncc_kernel_normalize_boot_info(i64 %raw.address) {
entry:
  %present = icmp ne i64 %raw.address, 0
  br i1 %present, label %inspect, label %fail
inspect:
  %raw = inttoptr i64 %raw.address to ptr
  %magic = load i64, ptr %raw, align 4
  %canonical = icmp eq i64 %magic, 13879475691537230
  br i1 %canonical, label %already, label %multiboot
already:
  ret i64 %raw.address
multiboot:
  %total.raw = load i32, ptr %raw, align 4
  %total = zext i32 %total.raw to i64
  %reserved.ptr = getelementptr i8, ptr %raw, i64 4
  %reserved = load i32, ptr %reserved.ptr, align 4
  %minimum = icmp uge i64 %total, 16
  %maximum = icmp ule i64 %total, 16777216
  %aligned.raw = and i64 %total, 7
  %aligned = icmp eq i64 %aligned.raw, 0
  %reserved.ok = icmp eq i32 %reserved, 0
  %a = and i1 %minimum, %maximum
  %b = and i1 %aligned, %reserved.ok
  %valid = and i1 %a, %b
  br i1 %valid, label %initialize, label %fail
initialize:
  %zero.boot = call ptr @memset(ptr @__ncc_kernel_mb2_boot, i32 0, i64 88)
  %zero.memory = call ptr @memset(ptr @__ncc_kernel_mb2_memory, i32 0, i64 8192)
  %zero.framebuffer = call ptr @memset(ptr @__ncc_kernel_mb2_framebuffer, i32 0, i64 32)
  %zero.platform = call ptr @memset(ptr @__ncc_kernel_mb2_platform, i32 0, i64 1040)
  %zero.command = call ptr @memset(ptr @__ncc_kernel_mb2_command, i32 0, i64 4096)
  %zero.efi.memory = call ptr @memset(ptr @__ncc_kernel_mb2_efi_memory, i32 0, i64 8192)
  store i64 0, ptr @__ncc_kernel_mb2_memory_count, align 8
  store i64 0, ptr @__ncc_kernel_mb2_efi_memory_count, align 8
  store i1 false, ptr @__ncc_kernel_mb2_detailed_memory, align 1
  store i32 0, ptr @__ncc_kernel_mb2_basic_lower, align 4
  store i32 0, ptr @__ncc_kernel_mb2_basic_upper, align 4
  store i64 13879475691537230, ptr @__ncc_kernel_mb2_boot, align 16
  %version.ptr = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 8
  store i64 1, ptr %version.ptr, align 8
  %platform.address = ptrtoint ptr @__ncc_kernel_mb2_platform to i64
  %platform.slot = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 56
  store i64 %platform.address, ptr %platform.slot, align 8
  %flags.ptr = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 16
  store i64 4, ptr %flags.ptr, align 8
  store i64 13884960599655246, ptr @__ncc_kernel_mb2_platform, align 16
  %platform.version.ptr = getelementptr i8, ptr @__ncc_kernel_mb2_platform, i64 8
  store i32 1, ptr %platform.version.ptr, align 4
  %raw.recorded = call i1 @__ncc_kernel_mb2_platform_upsert(i32 4, i32 11, i64 %raw.address, i64 %total, i64 2)
  br i1 %raw.recorded, label %loop, label %fail
loop:
  %cursor = phi i64 [ 8, %initialize ], [ %next, %advance ]
  %header.end = add i64 %cursor, 8
  %header.inside = icmp ule i64 %header.end, %total
  br i1 %header.inside, label %tag, label %fail
tag:
  %tag.ptr = getelementptr i8, ptr %raw, i64 %cursor
  %tag.kind = load i32, ptr %tag.ptr, align 4
  %tag.size.ptr = getelementptr i8, ptr %tag.ptr, i64 4
  %tag.size.raw = load i32, ptr %tag.size.ptr, align 4
  %tag.size = zext i32 %tag.size.raw to i64
  %tag.minimum = icmp uge i64 %tag.size, 8
  %tag.end = add i64 %cursor, %tag.size
  %tag.no.overflow = icmp uge i64 %tag.end, %cursor
  %tag.inside = icmp ule i64 %tag.end, %total
  %tag.a = and i1 %tag.minimum, %tag.no.overflow
  %tag.valid = and i1 %tag.a, %tag.inside
  br i1 %tag.valid, label %dispatch, label %fail
dispatch:
  %payload = getelementptr i8, ptr %tag.ptr, i64 8
  %payload.length = sub i64 %tag.size, 8
  switch i32 %tag.kind, label %advance [
    i32 0, label %end
    i32 1, label %command
    i32 2, label %boot.loader
    i32 3, label %module
    i32 4, label %basic.memory
    i32 5, label %boot.device
    i32 6, label %memory
    i32 7, label %vbe
    i32 8, label %framebuffer
    i32 9, label %elf.sections
    i32 10, label %apm
    i32 11, label %efi32
    i32 12, label %efi64
    i32 13, label %smbios
    i32 14, label %acpi.old
    i32 15, label %acpi.new
    i32 16, label %network
    i32 17, label %efi.memory
    i32 18, label %efi.services
    i32 19, label %efi.handle32
    i32 20, label %efi.handle
    i32 21, label %load.base
  ]
command:
  %command.ok = call i1 @__ncc_kernel_mb2_command_line(ptr %payload, i64 %payload.length)
  br i1 %command.ok, label %advance, label %fail
boot.loader:
  %boot.loader.valid = call i1 @__ncc_kernel_mb2_c_string_validate(ptr %payload, i64 %payload.length)
  br i1 %boot.loader.valid, label %boot.loader.store, label %fail
boot.loader.store:
  %boot.loader.address = ptrtoint ptr %payload to i64
  %boot.loader.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 24, i32 73, i64 %boot.loader.address, i64 %payload.length, i64 2)
  br i1 %boot.loader.ok, label %advance, label %fail
module:
  %module.large = icmp ugt i64 %payload.length, 8
  br i1 %module.large, label %module.read, label %fail
module.read:
  %module.start.raw = load i32, ptr %payload, align 4
  %module.end.ptr = getelementptr i8, ptr %payload, i64 4
  %module.end.raw = load i32, ptr %module.end.ptr, align 4
  %module.start = zext i32 %module.start.raw to i64
  %module.end = zext i32 %module.end.raw to i64
  %module.range = icmp uge i64 %module.end, %module.start
  %module.length = sub i64 %module.end, %module.start
  br i1 %module.range, label %module.command, label %fail
module.command:
  %module.command.ptr = getelementptr i8, ptr %payload, i64 8
  %module.command.length = sub i64 %payload.length, 8
  %module.command.valid = call i1 @__ncc_kernel_mb2_c_string_validate(ptr %module.command.ptr, i64 %module.command.length)
  br i1 %module.command.valid, label %module.store, label %fail
module.store:
  %module.ok = call i1 @__ncc_kernel_mb2_memory_append(i64 %module.start, i64 %module.length, i32 2, i32 0)
  br i1 %module.ok, label %advance, label %fail
basic.memory:
  %basic.large = icmp uge i64 %payload.length, 8
  br i1 %basic.large, label %basic.read, label %fail
basic.read:
  %basic.lower = load i32, ptr %payload, align 4
  %basic.upper.ptr = getelementptr i8, ptr %payload, i64 4
  %basic.upper = load i32, ptr %basic.upper.ptr, align 4
  store i32 %basic.lower, ptr @__ncc_kernel_mb2_basic_lower, align 4
  store i32 %basic.upper, ptr @__ncc_kernel_mb2_basic_upper, align 4
  br label %advance
boot.device:
  %boot.device.large = icmp uge i64 %payload.length, 12
  br i1 %boot.device.large, label %boot.device.store, label %fail
boot.device.store:
  %boot.device.address = ptrtoint ptr %payload to i64
  %boot.device.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 14, i32 73, i64 %boot.device.address, i64 12, i64 1)
  br i1 %boot.device.ok, label %advance, label %fail
memory:
  %memory.ok = call i1 @__ncc_kernel_mb2_memory_map(ptr %payload, i64 %payload.length)
  br i1 %memory.ok, label %memory.mark, label %fail
memory.mark:
  store i1 true, ptr @__ncc_kernel_mb2_detailed_memory, align 1
  br label %advance
vbe:
  %vbe.address = ptrtoint ptr %payload to i64
  %vbe.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 18, i32 73, i64 %vbe.address, i64 %payload.length, i64 2)
  br i1 %vbe.ok, label %advance, label %fail
framebuffer:
  %framebuffer.ok = call i1 @__ncc_kernel_mb2_framebuffer_capture(ptr %payload, i64 %payload.length)
  %framebuffer.address = ptrtoint ptr %payload to i64
  %framebuffer.raw.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 23, i32 73, i64 %framebuffer.address, i64 %payload.length, i64 2)
  br i1 %framebuffer.raw.ok, label %advance, label %fail
elf.sections:
  %elf.sections.address = ptrtoint ptr %payload to i64
  %elf.sections.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 19, i32 73, i64 %elf.sections.address, i64 %payload.length, i64 2)
  br i1 %elf.sections.ok, label %advance, label %fail
apm:
  %apm.address = ptrtoint ptr %payload to i64
  %apm.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 20, i32 73, i64 %apm.address, i64 %payload.length, i64 2)
  br i1 %apm.ok, label %advance, label %fail
efi32:
  %efi32.large = icmp uge i64 %payload.length, 4
  br i1 %efi32.large, label %efi32.read, label %fail
efi32.read:
  %efi32.raw = load i32, ptr %payload, align 4
  %efi32.address = zext i32 %efi32.raw to i64
  %efi32.length = call i64 @__ncc_kernel_mb2_uefi_system_table_size(i64 %efi32.address)
  %efi32.valid = icmp ne i64 %efi32.length, 0
  br i1 %efi32.valid, label %efi32.store, label %fail
efi32.store:
  %efi32.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 3, i32 73, i64 %efi32.address, i64 %efi32.length, i64 1)
  br i1 %efi32.ok, label %advance, label %fail
efi64:
  %efi64.large = icmp uge i64 %payload.length, 8
  br i1 %efi64.large, label %efi64.read, label %fail
efi64.read:
  %efi64.address = load i64, ptr %payload, align 8
  %efi64.length = call i64 @__ncc_kernel_mb2_uefi_system_table_size(i64 %efi64.address)
  %efi64.valid = icmp ne i64 %efi64.length, 0
  br i1 %efi64.valid, label %efi64.store, label %fail
efi64.store:
  %efi64.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 3, i32 73, i64 %efi64.address, i64 %efi64.length, i64 2)
  br i1 %efi64.ok, label %advance, label %fail
smbios:
  %smbios.large = icmp uge i64 %payload.length, 8
  br i1 %smbios.large, label %smbios.read, label %fail
smbios.read:
  %smbios.major = load i8, ptr %payload, align 1
  %smbios.major.wide = zext i8 %smbios.major to i64
  %smbios.minor.ptr = getelementptr i8, ptr %payload, i64 1
  %smbios.minor = load i8, ptr %smbios.minor.ptr, align 1
  %smbios.minor.wide = zext i8 %smbios.minor to i64
  %smbios.major.shifted = shl i64 %smbios.major.wide, 32
  %smbios.revision = or i64 %smbios.major.shifted, %smbios.minor.wide
  %smbios.body = getelementptr i8, ptr %payload, i64 8
  %smbios.address = ptrtoint ptr %smbios.body to i64
  %smbios.length = sub i64 %payload.length, 8
  %smbios.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 13, i32 73, i64 %smbios.address, i64 %smbios.length, i64 %smbios.revision)
  br i1 %smbios.ok, label %advance, label %fail
acpi.old:
  %acpi.old.valid = call i1 @__ncc_kernel_mb2_acpi_validate(ptr %payload, i64 %payload.length, i1 false)
  br i1 %acpi.old.valid, label %acpi.old.store, label %fail
acpi.old.store:
  %acpi.old.address = ptrtoint ptr %payload to i64
  %acpi.old.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 2, i32 9, i64 %acpi.old.address, i64 %payload.length, i64 1)
  br i1 %acpi.old.ok, label %advance, label %fail
acpi.new:
  %acpi.new.valid = call i1 @__ncc_kernel_mb2_acpi_validate(ptr %payload, i64 %payload.length, i1 true)
  br i1 %acpi.new.valid, label %acpi.new.store, label %fail
acpi.new.store:
  %acpi.new.address = ptrtoint ptr %payload to i64
  %acpi.new.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 2, i32 9, i64 %acpi.new.address, i64 %payload.length, i64 2)
  br i1 %acpi.new.ok, label %advance, label %fail
network:
  %network.address = ptrtoint ptr %payload to i64
  %network.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 21, i32 73, i64 %network.address, i64 %payload.length, i64 2)
  br i1 %network.ok, label %advance, label %fail
efi.memory:
  %efi.memory.address = ptrtoint ptr %payload to i64
  %efi.memory.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 15, i32 73, i64 %efi.memory.address, i64 %payload.length, i64 1)
  br i1 %efi.memory.ok, label %efi.memory.parse, label %fail
efi.memory.parse:
  %efi.memory.parsed = call i1 @__ncc_kernel_mb2_efi_memory_map(ptr %payload, i64 %payload.length)
  br i1 %efi.memory.parsed, label %advance, label %fail
efi.services:
  %efi.services.address = ptrtoint ptr %tag.ptr to i64
  %efi.services.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 22, i32 73, i64 %efi.services.address, i64 %tag.size, i64 1)
  br i1 %efi.services.ok, label %advance, label %fail
efi.handle32:
  %efi.handle32.large = icmp uge i64 %payload.length, 4
  br i1 %efi.handle32.large, label %efi.handle32.read, label %fail
efi.handle32.read:
  %efi.handle32.raw = load i32, ptr %payload, align 4
  %efi.handle32.value = zext i32 %efi.handle32.raw to i64
  %efi.handle32.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 16, i32 9, i64 %efi.handle32.value, i64 0, i64 1)
  br i1 %efi.handle32.ok, label %advance, label %fail
efi.handle:
  %efi.handle.large = icmp uge i64 %payload.length, 8
  br i1 %efi.handle.large, label %efi.handle.read, label %fail
efi.handle.read:
  %efi.handle.value = load i64, ptr %payload, align 8
  %efi.handle.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 16, i32 9, i64 %efi.handle.value, i64 0, i64 2)
  br i1 %efi.handle.ok, label %advance, label %fail
load.base:
  %load.base.large = icmp uge i64 %payload.length, 4
  br i1 %load.base.large, label %load.base.read, label %fail
load.base.read:
  %load.base.raw = load i32, ptr %payload, align 4
  %load.base.value = zext i32 %load.base.raw to i64
  %payload.slot = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 72
  store i64 %load.base.value, ptr %payload.slot, align 8
  %load.base.ok = call i1 @__ncc_kernel_mb2_platform_upsert(i32 17, i32 9, i64 %load.base.value, i64 0, i64 1)
  br i1 %load.base.ok, label %advance, label %fail
advance:
  %aligned.end = add i64 %tag.end, 7
  %next = and i64 %aligned.end, -8
  %next.forward = icmp ugt i64 %next, %cursor
  br i1 %next.forward, label %loop, label %fail
end:
  %end.size.ok = icmp eq i64 %tag.size, 8
  br i1 %end.size.ok, label %finalize, label %fail
finalize:
  %detailed.memory = load i1, ptr @__ncc_kernel_mb2_detailed_memory, align 1
  br i1 %detailed.memory, label %memory.store, label %fallback.efi
fallback.efi:
  %efi.memory.count = load i64, ptr @__ncc_kernel_mb2_efi_memory_count, align 8
  %has.efi.memory = icmp ne i64 %efi.memory.count, 0
  br i1 %has.efi.memory, label %fallback.efi.capacity, label %fallback.basic
fallback.efi.capacity:
  %existing.memory.count = load i64, ptr @__ncc_kernel_mb2_memory_count, align 8
  %combined.memory.count = add i64 %existing.memory.count, %efi.memory.count
  %combined.memory.room = icmp ule i64 %combined.memory.count, 256
  br i1 %combined.memory.room, label %fallback.efi.loop, label %fail
fallback.efi.loop:
  %efi.index = phi i64 [ 0, %fallback.efi.capacity ], [ %efi.next, %fallback.efi.copy ]
  %efi.done = icmp uge i64 %efi.index, %efi.memory.count
  br i1 %efi.done, label %fallback.efi.complete, label %fallback.efi.copy
fallback.efi.copy:
  %efi.source.offset = mul nuw i64 %efi.index, 32
  %efi.source = getelementptr i8, ptr @__ncc_kernel_mb2_efi_memory, i64 %efi.source.offset
  %destination.index = add nuw i64 %existing.memory.count, %efi.index
  %destination.offset = mul nuw i64 %destination.index, 32
  %destination = getelementptr i8, ptr @__ncc_kernel_mb2_memory, i64 %destination.offset
  %word0 = load i64, ptr %efi.source, align 8
  %source.word1 = getelementptr i8, ptr %efi.source, i64 8
  %word1 = load i64, ptr %source.word1, align 8
  %source.word2 = getelementptr i8, ptr %efi.source, i64 16
  %word2 = load i64, ptr %source.word2, align 8
  %source.word3 = getelementptr i8, ptr %efi.source, i64 24
  %word3 = load i64, ptr %source.word3, align 8
  store i64 %word0, ptr %destination, align 8
  %destination.word1 = getelementptr i8, ptr %destination, i64 8
  store i64 %word1, ptr %destination.word1, align 8
  %destination.word2 = getelementptr i8, ptr %destination, i64 16
  store i64 %word2, ptr %destination.word2, align 8
  %destination.word3 = getelementptr i8, ptr %destination, i64 24
  store i64 %word3, ptr %destination.word3, align 8
  %efi.next = add nuw i64 %efi.index, 1
  br label %fallback.efi.loop
fallback.efi.complete:
  store i64 %combined.memory.count, ptr @__ncc_kernel_mb2_memory_count, align 8
  br label %memory.store
fallback.basic:
  %basic.lower.raw = load i32, ptr @__ncc_kernel_mb2_basic_lower, align 4
  %basic.lower.present = icmp ne i32 %basic.lower.raw, 0
  br i1 %basic.lower.present, label %fallback.basic.lower, label %fallback.basic.upper
fallback.basic.lower:
  %basic.lower.wide = zext i32 %basic.lower.raw to i64
  %basic.lower.length = mul nuw i64 %basic.lower.wide, 1024
  %basic.lower.ok = call i1 @__ncc_kernel_mb2_memory_append(i64 0, i64 %basic.lower.length, i32 5, i32 1)
  br i1 %basic.lower.ok, label %fallback.basic.upper, label %fail
fallback.basic.upper:
  %basic.upper.raw = load i32, ptr @__ncc_kernel_mb2_basic_upper, align 4
  %basic.upper.present = icmp ne i32 %basic.upper.raw, 0
  br i1 %basic.upper.present, label %fallback.basic.upper.store, label %memory.store
fallback.basic.upper.store:
  %basic.upper.wide = zext i32 %basic.upper.raw to i64
  %basic.upper.length = mul nuw i64 %basic.upper.wide, 1024
  %basic.upper.ok = call i1 @__ncc_kernel_mb2_memory_append(i64 1048576, i64 %basic.upper.length, i32 5, i32 1)
  br i1 %basic.upper.ok, label %memory.store, label %fail
memory.store:
  %memory.count = load i64, ptr @__ncc_kernel_mb2_memory_count, align 8
  %has.memory = icmp ne i64 %memory.count, 0
  br i1 %has.memory, label %memory.commit, label %fail
memory.commit:
  %memory.address = ptrtoint ptr @__ncc_kernel_mb2_memory to i64
  %memory.slot = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 24
  store i64 %memory.address, ptr %memory.slot, align 8
  %memory.count.slot = getelementptr i8, ptr @__ncc_kernel_mb2_boot, i64 32
  store i64 %memory.count, ptr %memory.count.slot, align 8
  %old.flags = load i64, ptr %flags.ptr, align 8
  %with.memory = or i64 %old.flags, 1
  store i64 %with.memory, ptr %flags.ptr, align 8
  %result = ptrtoint ptr @__ncc_kernel_mb2_boot to i64
  ret i64 %result
fail:
  ret i64 0
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_covers_canonical_tags_and_rejects_unbounded_input() {
        let llvm = render_kernel_firmware_multiboot2_runtime("x86_64-unknown-none");
        assert!(llvm.contains("define i64 @__ncc_kernel_normalize_boot_info"));
        assert!(llvm.contains("i32 6, label %memory"));
        assert!(llvm.contains("i32 8, label %framebuffer"));
        assert!(llvm.contains("i32 15, label %acpi.new"));
        assert!(llvm.contains("i32 17, label %efi.memory"));
        assert!(llvm.contains("i32 2, label %boot.loader"));
        assert!(llvm.contains("@__ncc_kernel_mb2_c_string_validate"));
        assert!(llvm.contains("i32 24, i32 73, i64 %boot.loader.address"));
        assert!(llvm.contains("i32 22, i32 73, i64 %efi.services.address"));
        assert!(llvm.contains("@__ncc_kernel_mb2_uefi_system_table_size"));
        assert!(llvm.contains("icmp ule i64 %address, 4294967272"));
        assert!(llvm.contains("icmp eq i64 %signature, 6076298535811760713"));
        assert!(llvm.contains("i32 3, i32 73, i64 %efi64.address, i64 %efi64.length"));
        assert!(llvm.contains("%maximum = icmp ule i64 %total, 16777216"));
        assert!(llvm.contains("urem i32"));
        assert!(!llvm.contains("urem i64"));
        let arm = render_kernel_firmware_multiboot2_runtime("aarch64-unknown-none");
        assert!(arm.contains("@__ncc_kernel_raw_fdt_boot"));
        assert!(arm.contains("%fdt.valid = icmp eq i32 %fdt.magic, -302117424"));
        assert!(!arm.contains("@__ncc_kernel_set_opensbi_boot_info"));

        let rv64 = render_kernel_firmware_multiboot2_runtime("riscv64-unknown-elf");
        assert!(
            rv64.contains("define void @__ncc_kernel_set_opensbi_boot_info(i64 %hart, i64 %fdt)")
        );
        assert!(rv64.contains("store i32 7, ptr %hart.slot"));

        let rv32 = render_kernel_firmware_multiboot2_runtime("riscv32imac-unknown-elf");
        assert!(rv32.contains(
            "define void @__ncc_kernel_set_opensbi_boot_info(i32 %hart.raw, i32 %fdt.raw)"
        ));
    }
}
