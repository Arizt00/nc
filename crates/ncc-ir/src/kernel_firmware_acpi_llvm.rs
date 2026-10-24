pub(crate) fn render_kernel_firmware_acpi_runtime() -> &'static str {
    ACPI_RUNTIME
}

const ACPI_RUNTIME: &str = r#"
@__ncc_kernel_acpi_state = internal global i8 0, align 1
@__ncc_kernel_acpi_topology = internal global [12320 x i8] zeroinitializer, align 16
@__ncc_kernel_acpi_platform = internal global [104 x i8] zeroinitializer, align 16

define internal i1 @__ncc_kernel_acpi_checksum(ptr %base, i64 %length) {
entry:
  %nonzero = icmp ne i64 %length, 0
  %bounded = icmp ule i64 %length, 1048576
  %valid = and i1 %nonzero, %bounded
  br i1 %valid, label %loop, label %fail
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %sum = phi i8 [ 0, %entry ], [ %sum.next, %body ]
  %done = icmp uge i64 %i, %length
  br i1 %done, label %finish, label %body
body:
  %ptr = getelementptr i8, ptr %base, i64 %i
  %byte = load i8, ptr %ptr, align 1
  %sum.next = add i8 %sum, %byte
  %next = add nuw i64 %i, 1
  br label %loop
finish:
  %ok = icmp eq i8 %sum, 0
  ret i1 %ok
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_acpi_sdt_valid(ptr %table, i32 %signature) {
entry:
  %present = icmp ne ptr %table, null
  br i1 %present, label %header, label %fail
header:
  %actual = load i32, ptr %table, align 1
  %signature.ok = icmp eq i32 %actual, %signature
  %length.ptr = getelementptr i8, ptr %table, i64 4
  %length32 = load i32, ptr %length.ptr, align 1
  %minimum = icmp uge i32 %length32, 36
  %maximum = icmp ule i32 %length32, 1048576
  %length.ok = and i1 %minimum, %maximum
  %header.ok = and i1 %signature.ok, %length.ok
  br i1 %header.ok, label %checksum, label %fail
checksum:
  %length = zext i32 %length32 to i64
  %ok = call i1 @__ncc_kernel_acpi_checksum(ptr %table, i64 %length)
  ret i1 %ok
fail:
  ret i1 false
}

define internal void @__ncc_kernel_acpi_clear() {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %i, 104
  br i1 %done, label %topology, label %body
body:
  %ptr = getelementptr [104 x i8], ptr @__ncc_kernel_acpi_platform, i32 0, i64 %i
  store i8 0, ptr %ptr, align 1
  %next = add nuw i64 %i, 1
  br label %loop
topology:
  store i64 13886077073048398, ptr @__ncc_kernel_acpi_topology, align 16
  %version.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 8
  store i32 1, ptr %version.ptr, align 4
  %count.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 12
  store i32 0, ptr %count.ptr, align 4
  %flags.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 16
  store i64 0, ptr %flags.ptr, align 8
  %bootstrap.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 24
  store i64 0, ptr %bootstrap.ptr, align 8
  ret void
}

define internal i32 @__ncc_kernel_acpi_append_cpu(i64 %hardware, i64 %firmware, i32 %firmware.flags, i64 %bootstrap) {
entry:
  %count.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 12
  %count = load i32, ptr %count.ptr, align 4
  %room = icmp ult i32 %count, 256
  br i1 %room, label %dedupe.loop, label %fail
dedupe.loop:
  %i = phi i32 [ 0, %entry ], [ %dedupe.next, %dedupe.continue ]
  %dedupe.done = icmp uge i32 %i, %count
  br i1 %dedupe.done, label %write, label %dedupe.body
dedupe.body:
  %i64 = zext i32 %i to i64
  %existing.body = mul i64 %i64, 48
  %existing.offset = add i64 %existing.body, 40
  %existing.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 %existing.offset
  %existing = load i64, ptr %existing.ptr, align 8
  %same = icmp eq i64 %existing, %hardware
  br i1 %same, label %duplicate, label %dedupe.continue
dedupe.continue:
  %dedupe.next = add nuw i32 %i, 1
  br label %dedupe.loop
duplicate:
  ret i32 %i
write:
  %count64 = zext i32 %count to i64
  %entry.body = mul i64 %count64, 48
  %entry.offset = add i64 %entry.body, 32
  %entry.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 %entry.offset
  store i32 %count, ptr %entry.ptr, align 4
  %enabled.raw = and i32 %firmware.flags, 9
  %enabled = icmp ne i32 %enabled.raw, 0
  %hardware.bootstrap = icmp eq i64 %hardware, %bootstrap
  %is.bootstrap = and i1 %enabled, %hardware.bootstrap
  %enabled.flags = select i1 %enabled, i32 5, i32 0
  %bootstrap.flag = select i1 %is.bootstrap, i32 2, i32 0
  %flags = or i32 %enabled.flags, %bootstrap.flag
  %flags.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  store i32 %flags, ptr %flags.ptr, align 4
  %hardware.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  store i64 %hardware, ptr %hardware.ptr, align 8
  %firmware.ptr = getelementptr i8, ptr %entry.ptr, i64 16
  store i64 %firmware, ptr %firmware.ptr, align 8
  %numa.ptr = getelementptr i8, ptr %entry.ptr, i64 24
  store i32 0, ptr %numa.ptr, align 4
  %hierarchy = call i64 @__ncc_kernel_arch_topology_pack(i64 %hardware)
  %package64 = and i64 %hierarchy, 65535
  %package = trunc i64 %package64 to i16
  %package.ptr = getelementptr i8, ptr %entry.ptr, i64 28
  store i16 %package, ptr %package.ptr, align 2
  %core.shift = lshr i64 %hierarchy, 16
  %core = trunc i64 %core.shift to i16
  %core.ptr = getelementptr i8, ptr %entry.ptr, i64 30
  store i16 %core, ptr %core.ptr, align 2
  %thread.shift = lshr i64 %hierarchy, 32
  %thread = trunc i64 %thread.shift to i16
  %thread.ptr = getelementptr i8, ptr %entry.ptr, i64 32
  store i16 %thread, ptr %thread.ptr, align 2
  %stack.ptr = getelementptr i8, ptr %entry.ptr, i64 40
  store i64 0, ptr %stack.ptr, align 8
  %next = add nuw i32 %count, 1
  store i32 %next, ptr %count.ptr, align 4
  ret i32 %count
fail:
  ret i32 -1
}

define internal void @__ncc_kernel_acpi_platform_header(i32 %architecture, i32 %start.method, i32 %interrupt.model, i64 %primary) {
entry:
  store i64 13880566898443086, ptr @__ncc_kernel_acpi_platform, align 16
  %version.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 8
  store i32 1, ptr %version.ptr, align 4
  %size.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 12
  store i32 104, ptr %size.ptr, align 4
  %arch.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 16
  store i32 %architecture, ptr %arch.ptr, align 4
  %flags.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 20
  store i32 1, ptr %flags.ptr, align 4
  %start.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 24
  store i32 %start.method, ptr %start.ptr, align 4
  %model.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 28
  store i32 %interrupt.model, ptr %model.ptr, align 4
  %primary.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 32
  store i64 %primary, ptr %primary.ptr, align 8
  %timer.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 56
  %timer = call i64 @__ncc_kernel_arch_timer_frequency()
  store i64 %timer, ptr %timer.ptr, align 8
  %features.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 88
  store i64 15, ptr %features.ptr, align 8
  ret void
}

define internal i1 @__ncc_kernel_acpi_parse_madt(ptr %madt, i64 %bootstrap) {
entry:
  %valid = call i1 @__ncc_kernel_acpi_sdt_valid(ptr %madt, i32 1128878145)
  br i1 %valid, label %header, label %fail
header:
  %length.ptr = getelementptr i8, ptr %madt, i64 4
  %length32 = load i32, ptr %length.ptr, align 1
  %long.enough = icmp uge i32 %length32, 44
  br i1 %long.enough, label %initialize, label %fail
initialize:
  %lapic.ptr = getelementptr i8, ptr %madt, i64 36
  %lapic32 = load i32, ptr %lapic.ptr, align 1
  %lapic = zext i32 %lapic32 to i64
  call void @__ncc_kernel_acpi_platform_header(i32 1, i32 1, i32 1, i64 %lapic)
  br label %loop
loop:
  %cursor = phi i32 [ 44, %initialize ], [ %next, %continue ]
  %done = icmp uge i32 %cursor, %length32
  br i1 %done, label %finish, label %entry.header
entry.header:
  %room.header = add i32 %cursor, 2
  %header.ok = icmp ule i32 %room.header, %length32
  br i1 %header.ok, label %entry.read, label %fail
entry.read:
  %cursor64 = zext i32 %cursor to i64
  %entry.ptr = getelementptr i8, ptr %madt, i64 %cursor64
  %kind = load i8, ptr %entry.ptr, align 1
  %entry.length.ptr = getelementptr i8, ptr %entry.ptr, i64 1
  %entry.length8 = load i8, ptr %entry.length.ptr, align 1
  %entry.length = zext i8 %entry.length8 to i32
  %minimum = icmp uge i32 %entry.length, 2
  %next = add i32 %cursor, %entry.length
  %inside = icmp ule i32 %next, %length32
  %entry.ok = and i1 %minimum, %inside
  br i1 %entry.ok, label %dispatch, label %fail
dispatch:
  switch i8 %kind, label %continue [
    i8 0, label %lapic.cpu
    i8 1, label %ioapic
    i8 5, label %lapic.override
    i8 9, label %x2apic.cpu
    i8 11, label %gicc.cpu
    i8 12, label %gicd
    i8 14, label %gicr
    i8 15, label %gicits
  ]
lapic.cpu:
  %lapic.size = icmp uge i32 %entry.length, 8
  br i1 %lapic.size, label %lapic.cpu.read, label %fail
lapic.cpu.read:
  %lapic.uid.ptr = getelementptr i8, ptr %entry.ptr, i64 2
  %lapic.uid8 = load i8, ptr %lapic.uid.ptr, align 1
  %lapic.uid = zext i8 %lapic.uid8 to i64
  %lapic.id.ptr = getelementptr i8, ptr %entry.ptr, i64 3
  %lapic.id8 = load i8, ptr %lapic.id.ptr, align 1
  %lapic.id = zext i8 %lapic.id8 to i64
  %lapic.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %lapic.flags = load i32, ptr %lapic.flags.ptr, align 1
  %lapic.index = call i32 @__ncc_kernel_acpi_append_cpu(i64 %lapic.id, i64 %lapic.uid, i32 %lapic.flags, i64 %bootstrap)
  %lapic.added = icmp sge i32 %lapic.index, 0
  br i1 %lapic.added, label %continue, label %fail
ioapic:
  %ioapic.size = icmp uge i32 %entry.length, 12
  br i1 %ioapic.size, label %ioapic.read, label %fail
ioapic.read:
  %ioapic.base.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %ioapic.base32 = load i32, ptr %ioapic.base.ptr, align 1
  %ioapic.base = zext i32 %ioapic.base32 to i64
  %ioapic.dst = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 40
  store i64 %ioapic.base, ptr %ioapic.dst, align 8
  br label %continue
lapic.override:
  %override.size = icmp uge i32 %entry.length, 12
  br i1 %override.size, label %override.read, label %fail
override.read:
  %override.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %override = load i64, ptr %override.ptr, align 1
  %primary.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 32
  store i64 %override, ptr %primary.ptr, align 8
  br label %continue
x2apic.cpu:
  %x2.size = icmp uge i32 %entry.length, 16
  br i1 %x2.size, label %x2.read, label %fail
x2.read:
  %x2.id.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %x2.id32 = load i32, ptr %x2.id.ptr, align 1
  %x2.id = zext i32 %x2.id32 to i64
  %x2.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %x2.flags = load i32, ptr %x2.flags.ptr, align 1
  %x2.uid.ptr = getelementptr i8, ptr %entry.ptr, i64 12
  %x2.uid32 = load i32, ptr %x2.uid.ptr, align 1
  %x2.uid = zext i32 %x2.uid32 to i64
  %x2.index = call i32 @__ncc_kernel_acpi_append_cpu(i64 %x2.id, i64 %x2.uid, i32 %x2.flags, i64 %bootstrap)
  %x2.added = icmp sge i32 %x2.index, 0
  %model.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 28
  store i32 2, ptr %model.ptr, align 4
  br i1 %x2.added, label %continue, label %fail
gicc.cpu:
  %gicc.size = icmp uge i32 %entry.length, 76
  br i1 %gicc.size, label %gicc.read, label %fail
gicc.read:
  %gicc.uid.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %gicc.uid32 = load i32, ptr %gicc.uid.ptr, align 1
  %gicc.uid = zext i32 %gicc.uid32 to i64
  %gicc.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 12
  %gicc.flags = load i32, ptr %gicc.flags.ptr, align 1
  %gicc.mpidr.ptr = getelementptr i8, ptr %entry.ptr, i64 68
  %gicc.mpidr = load i64, ptr %gicc.mpidr.ptr, align 1
  %gicc.index = call i32 @__ncc_kernel_acpi_append_cpu(i64 %gicc.mpidr, i64 %gicc.uid, i32 %gicc.flags, i64 %bootstrap)
  %gicc.added = icmp sge i32 %gicc.index, 0
  br i1 %gicc.added, label %gicc.platform, label %fail
gicc.platform:
  %gicc.gicr.ptr = getelementptr i8, ptr %entry.ptr, i64 60
  %gicc.gicr = load i64, ptr %gicc.gicr.ptr, align 1
  %secondary.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 40
  %secondary.old = load i64, ptr %secondary.ptr, align 8
  %secondary.empty = icmp eq i64 %secondary.old, 0
  %gicc.gicr.present = icmp ne i64 %gicc.gicr, 0
  %take.gicc = and i1 %secondary.empty, %gicc.gicr.present
  br i1 %take.gicc, label %gicc.store, label %gicc.header
gicc.store:
  store i64 %gicc.gicr, ptr %secondary.ptr, align 8
  br label %gicc.header
gicc.header:
  %primary.old.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 32
  %primary.old = load i64, ptr %primary.old.ptr, align 8
  call void @__ncc_kernel_acpi_platform_header(i32 2, i32 2, i32 3, i64 %primary.old)
  br label %continue
gicd:
  %gicd.size = icmp uge i32 %entry.length, 24
  br i1 %gicd.size, label %gicd.read, label %fail
gicd.read:
  %gicd.base.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %gicd.base = load i64, ptr %gicd.base.ptr, align 1
  call void @__ncc_kernel_acpi_platform_header(i32 2, i32 2, i32 3, i64 %gicd.base)
  br label %continue
gicr:
  %gicr.size = icmp uge i32 %entry.length, 16
  br i1 %gicr.size, label %gicr.read, label %fail
gicr.read:
  %gicr.base.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %gicr.base = load i64, ptr %gicr.base.ptr, align 1
  %gicr.dst = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 40
  store i64 %gicr.base, ptr %gicr.dst, align 8
  %gicr.stride.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 64
  store i64 131072, ptr %gicr.stride.ptr, align 8
  %gicr.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 2
  %gicr.flags = load i8, ptr %gicr.flags.ptr, align 1
  %noncoherent.raw = and i8 %gicr.flags, 1
  %coherent = icmp eq i8 %noncoherent.raw, 0
  br i1 %coherent, label %gicr.coherent, label %continue
gicr.coherent:
  %platform.flags.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 20
  %platform.flags = load i32, ptr %platform.flags.ptr, align 4
  %platform.flags.next = or i32 %platform.flags, 4
  store i32 %platform.flags.next, ptr %platform.flags.ptr, align 4
  br label %continue
gicits:
  %its.size = icmp uge i32 %entry.length, 20
  br i1 %its.size, label %its.read, label %fail
its.read:
  %its.base.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %its.base = load i64, ptr %its.base.ptr, align 1
  %its.dst = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 48
  store i64 %its.base, ptr %its.dst, align 8
  %features.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 88
  %features = load i64, ptr %features.ptr, align 8
  %features.next = or i64 %features, 16
  store i64 %features.next, ptr %features.ptr, align 8
  br label %continue
continue:
  br label %loop
finish:
  %count.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 12
  %count = load i32, ptr %count.ptr, align 4
  %has.cpus = icmp ne i32 %count, 0
  br i1 %has.cpus, label %bootstrap.scan, label %fail
bootstrap.scan:
  %scan = phi i32 [ 0, %finish ], [ %scan.next, %bootstrap.continue ]
  %bootstrap.count = phi i32 [ 0, %finish ], [ %bootstrap.count.next, %bootstrap.continue ]
  %scan.done = icmp uge i32 %scan, %count
  br i1 %scan.done, label %validate.platform, label %bootstrap.body
bootstrap.body:
  %scan64 = zext i32 %scan to i64
  %scan.body = mul i64 %scan64, 48
  %scan.offset = add i64 %scan.body, 36
  %scan.flags.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 %scan.offset
  %scan.flags = load i32, ptr %scan.flags.ptr, align 4
  %bootstrap.raw = and i32 %scan.flags, 2
  %is.bootstrap = icmp ne i32 %bootstrap.raw, 0
  %bootstrap.add = select i1 %is.bootstrap, i32 1, i32 0
  %bootstrap.count.next = add i32 %bootstrap.count, %bootstrap.add
  %scan.next = add nuw i32 %scan, 1
  br label %bootstrap.continue
bootstrap.continue:
  br label %bootstrap.scan
validate.platform:
  %one.bootstrap = icmp eq i32 %bootstrap.count, 1
  %arch.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 16
  %arch = load i32, ptr %arch.ptr, align 4
  %primary.ptr.final = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 32
  %primary = load i64, ptr %primary.ptr.final, align 8
  %primary.ok = icmp ne i64 %primary, 0
  %arm = icmp eq i32 %arch, 2
  %secondary.ptr.final = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 40
  %secondary = load i64, ptr %secondary.ptr.final, align 8
  %secondary.ok = icmp ne i64 %secondary, 0
  %not.arm = xor i1 %arm, true
  %controller.ok = or i1 %not.arm, %secondary.ok
  %a = and i1 %one.bootstrap, %primary.ok
  %ok = and i1 %a, %controller.ok
  ret i1 %ok
fail:
  ret i1 false
}

define internal void @__ncc_kernel_acpi_set_numa(i1 %hardware.key, i64 %id, i32 %numa) {
entry:
  %count.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 12
  %count = load i32, ptr %count.ptr, align 4
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %continue ]
  %done = icmp uge i32 %i, %count
  br i1 %done, label %finish, label %body
body:
  %i64 = zext i32 %i to i64
  %body.offset = mul i64 %i64, 48
  %entry.offset = add i64 %body.offset, 32
  %entry.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_topology, i64 %entry.offset
  %hardware.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %hardware = load i64, ptr %hardware.ptr, align 8
  %firmware.ptr = getelementptr i8, ptr %entry.ptr, i64 16
  %firmware = load i64, ptr %firmware.ptr, align 8
  %candidate = select i1 %hardware.key, i64 %hardware, i64 %firmware
  %same = icmp eq i64 %candidate, %id
  br i1 %same, label %store, label %continue
store:
  %numa.ptr = getelementptr i8, ptr %entry.ptr, i64 24
  store i32 %numa, ptr %numa.ptr, align 4
  br label %finish
continue:
  %next = add nuw i32 %i, 1
  br label %loop
finish:
  ret void
}

define internal i1 @__ncc_kernel_acpi_apply_srat(ptr %srat) {
entry:
  %valid = call i1 @__ncc_kernel_acpi_sdt_valid(ptr %srat, i32 1413567059)
  br i1 %valid, label %header, label %fail
header:
  %length.ptr = getelementptr i8, ptr %srat, i64 4
  %length = load i32, ptr %length.ptr, align 1
  %minimum = icmp uge i32 %length, 48
  br i1 %minimum, label %loop, label %fail
loop:
  %cursor = phi i32 [ 48, %header ], [ %next, %continue ]
  %done = icmp uge i32 %cursor, %length
  br i1 %done, label %success, label %entry.header
entry.header:
  %cursor64 = zext i32 %cursor to i64
  %entry.ptr = getelementptr i8, ptr %srat, i64 %cursor64
  %kind = load i8, ptr %entry.ptr, align 1
  %entry.length.ptr = getelementptr i8, ptr %entry.ptr, i64 1
  %entry.length8 = load i8, ptr %entry.length.ptr, align 1
  %entry.length = zext i8 %entry.length8 to i32
  %minimum.entry = icmp uge i32 %entry.length, 2
  %next = add i32 %cursor, %entry.length
  %inside = icmp ule i32 %next, %length
  %entry.ok = and i1 %minimum.entry, %inside
  br i1 %entry.ok, label %dispatch, label %fail
dispatch:
  switch i8 %kind, label %continue [
    i8 0, label %lapic
    i8 2, label %x2apic
    i8 3, label %gicc
  ]
lapic:
  %lapic.size = icmp uge i32 %entry.length, 16
  br i1 %lapic.size, label %lapic.read, label %fail
lapic.read:
  %lapic.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %lapic.flags = load i32, ptr %lapic.flags.ptr, align 1
  %lapic.enabled.raw = and i32 %lapic.flags, 1
  %lapic.enabled = icmp ne i32 %lapic.enabled.raw, 0
  br i1 %lapic.enabled, label %lapic.apply, label %continue
lapic.apply:
  %lapic.id.ptr = getelementptr i8, ptr %entry.ptr, i64 3
  %lapic.id8 = load i8, ptr %lapic.id.ptr, align 1
  %lapic.id = zext i8 %lapic.id8 to i64
  %domain.low.ptr = getelementptr i8, ptr %entry.ptr, i64 2
  %domain.low8 = load i8, ptr %domain.low.ptr, align 1
  %domain.low = zext i8 %domain.low8 to i32
  %domain.b1.ptr = getelementptr i8, ptr %entry.ptr, i64 9
  %domain.b2.ptr = getelementptr i8, ptr %entry.ptr, i64 10
  %domain.b3.ptr = getelementptr i8, ptr %entry.ptr, i64 11
  %domain.b1 = load i8, ptr %domain.b1.ptr, align 1
  %domain.b2 = load i8, ptr %domain.b2.ptr, align 1
  %domain.b3 = load i8, ptr %domain.b3.ptr, align 1
  %domain.b1.wide = zext i8 %domain.b1 to i32
  %domain.b2.wide = zext i8 %domain.b2 to i32
  %domain.b3.wide = zext i8 %domain.b3 to i32
  %domain.b1.shift = shl i32 %domain.b1.wide, 8
  %domain.b2.shift = shl i32 %domain.b2.wide, 16
  %domain.b3.shift = shl i32 %domain.b3.wide, 24
  %domain.a = or i32 %domain.low, %domain.b1.shift
  %domain.b = or i32 %domain.b2.shift, %domain.b3.shift
  %domain = or i32 %domain.a, %domain.b
  call void @__ncc_kernel_acpi_set_numa(i1 true, i64 %lapic.id, i32 %domain)
  br label %continue
x2apic:
  %x2.size = icmp uge i32 %entry.length, 24
  br i1 %x2.size, label %x2.read, label %fail
x2.read:
  %x2.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 12
  %x2.flags = load i32, ptr %x2.flags.ptr, align 1
  %x2.enabled.raw = and i32 %x2.flags, 1
  %x2.enabled = icmp ne i32 %x2.enabled.raw, 0
  br i1 %x2.enabled, label %x2.apply, label %continue
x2.apply:
  %x2.domain.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  %x2.domain = load i32, ptr %x2.domain.ptr, align 1
  %x2.id.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %x2.id32 = load i32, ptr %x2.id.ptr, align 1
  %x2.id = zext i32 %x2.id32 to i64
  call void @__ncc_kernel_acpi_set_numa(i1 true, i64 %x2.id, i32 %x2.domain)
  br label %continue
gicc:
  %gicc.size = icmp uge i32 %entry.length, 18
  br i1 %gicc.size, label %gicc.read, label %fail
gicc.read:
  %gicc.flags.ptr = getelementptr i8, ptr %entry.ptr, i64 10
  %gicc.flags = load i32, ptr %gicc.flags.ptr, align 1
  %gicc.enabled.raw = and i32 %gicc.flags, 1
  %gicc.enabled = icmp ne i32 %gicc.enabled.raw, 0
  br i1 %gicc.enabled, label %gicc.apply, label %continue
gicc.apply:
  %gicc.domain.ptr = getelementptr i8, ptr %entry.ptr, i64 2
  %gicc.domain = load i32, ptr %gicc.domain.ptr, align 1
  %gicc.uid.ptr = getelementptr i8, ptr %entry.ptr, i64 6
  %gicc.uid32 = load i32, ptr %gicc.uid.ptr, align 1
  %gicc.uid = zext i32 %gicc.uid32 to i64
  call void @__ncc_kernel_acpi_set_numa(i1 false, i64 %gicc.uid, i32 %gicc.domain)
  br label %continue
continue:
  br label %loop
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_acpi_apply_fadt(ptr %fadt) {
entry:
  %valid = call i1 @__ncc_kernel_acpi_sdt_valid(ptr %fadt, i32 1346584902)
  br i1 %valid, label %architecture, label %fail
architecture:
  %arch.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 16
  %arch = load i32, ptr %arch.ptr, align 4
  %arm = icmp eq i32 %arch, 2
  br i1 %arm, label %length.check, label %success
length.check:
  %length.ptr = getelementptr i8, ptr %fadt, i64 4
  %length = load i32, ptr %length.ptr, align 1
  %enough = icmp uge i32 %length, 131
  br i1 %enough, label %flags.read, label %fail
flags.read:
  %flags.ptr = getelementptr i8, ptr %fadt, i64 129
  %flags = load i16, ptr %flags.ptr, align 1
  %compliant.raw = and i16 %flags, 1
  %compliant = icmp ne i16 %compliant.raw, 0
  br i1 %compliant, label %method.select, label %fail
method.select:
  %hvc.raw = and i16 %flags, 2
  %hvc = icmp ne i16 %hvc.raw, 0
  %method = select i1 %hvc, i32 3, i32 2
  %method.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 24
  store i32 %method, ptr %method.ptr, align 4
  br label %success
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_acpi_parse_root(ptr %root, i32 %entry.size, i32 %signature, i64 %bootstrap) {
entry:
  %valid = call i1 @__ncc_kernel_acpi_sdt_valid(ptr %root, i32 %signature)
  br i1 %valid, label %header, label %fail
header:
  %length.ptr = getelementptr i8, ptr %root, i64 4
  %length = load i32, ptr %length.ptr, align 1
  %payload = sub i32 %length, 36
  %remainder = urem i32 %payload, %entry.size
  %aligned = icmp eq i32 %remainder, 0
  br i1 %aligned, label %loop, label %fail
loop:
  %cursor = phi i32 [ 36, %header ], [ %next, %continue ]
  %madt = phi ptr [ null, %header ], [ %madt.next, %continue ]
  %srat = phi ptr [ null, %header ], [ %srat.next, %continue ]
  %fadt = phi ptr [ null, %header ], [ %fadt.next, %continue ]
  %done = icmp uge i32 %cursor, %length
  br i1 %done, label %parse.madt, label %read
read:
  %cursor64 = zext i32 %cursor to i64
  %slot = getelementptr i8, ptr %root, i64 %cursor64
  %wide = icmp eq i32 %entry.size, 8
  br i1 %wide, label %read64, label %read32
read64:
  %address64 = load i64, ptr %slot, align 1
  br label %inspect
read32:
  %address32 = load i32, ptr %slot, align 1
  %address32.wide = zext i32 %address32 to i64
  br label %inspect
inspect:
  %address = phi i64 [ %address64, %read64 ], [ %address32.wide, %read32 ]
  %present = icmp ne i64 %address, 0
  br i1 %present, label %signature.read, label %continue.direct
signature.read:
  %table = inttoptr i64 %address to ptr
  %table.signature = load i32, ptr %table, align 1
  %is.madt = icmp eq i32 %table.signature, 1128878145
  %is.srat = icmp eq i32 %table.signature, 1413567059
  %is.fadt = icmp eq i32 %table.signature, 1346584902
  %madt.empty = icmp eq ptr %madt, null
  %srat.empty = icmp eq ptr %srat, null
  %fadt.empty = icmp eq ptr %fadt, null
  %take.madt = and i1 %is.madt, %madt.empty
  %take.srat = and i1 %is.srat, %srat.empty
  %take.fadt = and i1 %is.fadt, %fadt.empty
  %madt.selected = select i1 %take.madt, ptr %table, ptr %madt
  %srat.selected = select i1 %take.srat, ptr %table, ptr %srat
  %fadt.selected = select i1 %take.fadt, ptr %table, ptr %fadt
  br label %continue.direct
continue.direct:
  %madt.direct = phi ptr [ %madt, %inspect ], [ %madt.selected, %signature.read ]
  %srat.direct = phi ptr [ %srat, %inspect ], [ %srat.selected, %signature.read ]
  %fadt.direct = phi ptr [ %fadt, %inspect ], [ %fadt.selected, %signature.read ]
  br label %continue
continue:
  %madt.next = phi ptr [ %madt.direct, %continue.direct ]
  %srat.next = phi ptr [ %srat.direct, %continue.direct ]
  %fadt.next = phi ptr [ %fadt.direct, %continue.direct ]
  %next = add i32 %cursor, %entry.size
  br label %loop
parse.madt:
  %has.madt = icmp ne ptr %madt, null
  br i1 %has.madt, label %madt.call, label %fail
madt.call:
  %madt.ok = call i1 @__ncc_kernel_acpi_parse_madt(ptr %madt, i64 %bootstrap)
  br i1 %madt.ok, label %srat.check, label %fail
srat.check:
  %has.srat = icmp ne ptr %srat, null
  br i1 %has.srat, label %srat.call, label %fadt.check
srat.call:
  %srat.ok = call i1 @__ncc_kernel_acpi_apply_srat(ptr %srat)
  br i1 %srat.ok, label %fadt.check, label %fail
fadt.check:
  %arch.ptr = getelementptr i8, ptr @__ncc_kernel_acpi_platform, i64 16
  %arch = load i32, ptr %arch.ptr, align 4
  %arm = icmp eq i32 %arch, 2
  br i1 %arm, label %fadt.required, label %success
fadt.required:
  %has.fadt = icmp ne ptr %fadt, null
  br i1 %has.fadt, label %fadt.call, label %fail
fadt.call:
  %fadt.ok = call i1 @__ncc_kernel_acpi_apply_fadt(ptr %fadt)
  br i1 %fadt.ok, label %success, label %fail
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_acpi_parse_rsdp(ptr %rsdp, i64 %bootstrap) {
entry:
  %present = icmp ne ptr %rsdp, null
  br i1 %present, label %signature, label %fail
signature:
  %sig0 = load i32, ptr %rsdp, align 1
  %sig1.ptr = getelementptr i8, ptr %rsdp, i64 4
  %sig1 = load i32, ptr %sig1.ptr, align 1
  %sig0.ok = icmp eq i32 %sig0, 541348690
  %sig1.ok = icmp eq i32 %sig1, 542266448
  %signature.ok = and i1 %sig0.ok, %sig1.ok
  br i1 %signature.ok, label %checksum.v1, label %fail
checksum.v1:
  %v1.ok = call i1 @__ncc_kernel_acpi_checksum(ptr %rsdp, i64 20)
  br i1 %v1.ok, label %revision.read, label %fail
revision.read:
  %revision.ptr = getelementptr i8, ptr %rsdp, i64 15
  %revision = load i8, ptr %revision.ptr, align 1
  %extended = icmp uge i8 %revision, 2
  br i1 %extended, label %xsdt, label %rsdt
xsdt:
  %length.ptr = getelementptr i8, ptr %rsdp, i64 20
  %length32 = load i32, ptr %length.ptr, align 1
  %minimum = icmp uge i32 %length32, 36
  %maximum = icmp ule i32 %length32, 4096
  %length.ok = and i1 %minimum, %maximum
  br i1 %length.ok, label %xsdt.checksum, label %fail
xsdt.checksum:
  %length = zext i32 %length32 to i64
  %extended.ok = call i1 @__ncc_kernel_acpi_checksum(ptr %rsdp, i64 %length)
  br i1 %extended.ok, label %xsdt.address.read, label %fail
xsdt.address.read:
  %xsdt.ptr = getelementptr i8, ptr %rsdp, i64 24
  %xsdt.address = load i64, ptr %xsdt.ptr, align 1
  %xsdt.present = icmp ne i64 %xsdt.address, 0
  br i1 %xsdt.present, label %xsdt.parse, label %rsdt
xsdt.parse:
  %xsdt.table = inttoptr i64 %xsdt.address to ptr
  %xsdt.ok = call i1 @__ncc_kernel_acpi_parse_root(ptr %xsdt.table, i32 8, i32 1413763928, i64 %bootstrap)
  ret i1 %xsdt.ok
rsdt:
  %rsdt.ptr = getelementptr i8, ptr %rsdp, i64 16
  %rsdt.address32 = load i32, ptr %rsdt.ptr, align 1
  %rsdt.address = zext i32 %rsdt.address32 to i64
  %rsdt.present = icmp ne i64 %rsdt.address, 0
  br i1 %rsdt.present, label %rsdt.parse, label %fail
rsdt.parse:
  %rsdt.table = inttoptr i64 %rsdt.address to ptr
  %rsdt.ok = call i1 @__ncc_kernel_acpi_parse_root(ptr %rsdt.table, i32 4, i32 1413763922, i64 %bootstrap)
  ret i1 %rsdt.ok
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_acpi_adapt(ptr %boot) {
entry:
  %state = load i8, ptr @__ncc_kernel_acpi_state, align 1
  switch i8 %state, label %begin [
    i8 1, label %fail
    i8 2, label %success
    i8 3, label %fail
  ]
begin:
  store i8 1, ptr @__ncc_kernel_acpi_state, align 1
  call void @__ncc_kernel_acpi_clear()
  %entry.ptr = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 2)
  %has.entry = icmp ne ptr %entry.ptr, null
  br i1 %has.entry, label %address, label %mark.fail
address:
  %rsdp.slot = getelementptr i8, ptr %entry.ptr, i64 8
  %rsdp.address = load i64, ptr %rsdp.slot, align 8
  %present = icmp ne i64 %rsdp.address, 0
  br i1 %present, label %parse, label %mark.fail
parse:
  %rsdp = inttoptr i64 %rsdp.address to ptr
  %bootstrap = call i64 @__ncc_kernel_arch_bootstrap_hardware_id()
  %ok = call i1 @__ncc_kernel_acpi_parse_rsdp(ptr %rsdp, i64 %bootstrap)
  br i1 %ok, label %mark.success, label %mark.fail
mark.success:
  store i8 2, ptr @__ncc_kernel_acpi_state, align 1
  br label %success
mark.fail:
  store i8 3, ptr @__ncc_kernel_acpi_state, align 1
  br label %fail
success:
  ret i1 true
fail:
  ret i1 false
}

define ptr @__ncc_kernel_acpi_topology_from_boot(ptr %boot) {
entry:
  %ok = call i1 @__ncc_kernel_acpi_adapt(ptr %boot)
  br i1 %ok, label %success, label %fail
success:
  ret ptr @__ncc_kernel_acpi_topology
fail:
  ret ptr null
}

define ptr @__ncc_kernel_acpi_platform_from_boot(ptr %boot) {
entry:
  %ok = call i1 @__ncc_kernel_acpi_adapt(ptr %boot)
  br i1 %ok, label %success, label %fail
success:
  ret ptr @__ncc_kernel_acpi_platform
fail:
  ret ptr null
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_contains_checksum_root_madt_srat_fadt_and_canonical_outputs() {
        let llvm = render_kernel_firmware_acpi_runtime();
        assert!(llvm.contains("@__ncc_kernel_acpi_checksum"));
        assert!(llvm.contains("@__ncc_kernel_acpi_parse_root"));
        assert!(llvm.contains("@__ncc_kernel_acpi_parse_madt"));
        assert!(llvm.contains("@__ncc_kernel_acpi_apply_srat"));
        assert!(llvm.contains("@__ncc_kernel_acpi_apply_fadt"));
        assert!(llvm.contains("@__ncc_kernel_acpi_topology_from_boot"));
        assert!(llvm.contains("@__ncc_kernel_acpi_platform_from_boot"));
    }
}
