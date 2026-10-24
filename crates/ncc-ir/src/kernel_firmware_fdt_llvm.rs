pub(crate) fn render_kernel_firmware_fdt_runtime() -> &'static str {
    FDT_RUNTIME
}

const FDT_RUNTIME: &str = r##"
@__ncc_kernel_firmware_adapter_state = internal global i8 0, align 1
@__ncc_kernel_fdt_node_kind = internal global [64 x i8] zeroinitializer, align 16
@__ncc_kernel_fdt_node_enabled = internal global [64 x i1] zeroinitializer, align 16
@__ncc_kernel_fdt_parent_address_cells = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_parent_size_cells = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_child_address_cells = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_child_size_cells = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_cpu_index = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_base = internal global [64 x i64] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_size = internal global [64 x i64] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_width = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_height = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_stride = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_format = internal global [64 x i32] zeroinitializer, align 16
@__ncc_kernel_fdt_fb_compatible = internal global [64 x i1] zeroinitializer, align 16
@__ncc_kernel_fdt_replace_memory = internal global i1 false, align 1

@__ncc_fdt_lit_address_cells = private unnamed_addr constant [15 x i8] c"#address-cells\00"
@__ncc_fdt_lit_size_cells = private unnamed_addr constant [12 x i8] c"#size-cells\00"
@__ncc_fdt_lit_reg = private unnamed_addr constant [4 x i8] c"reg\00"
@__ncc_fdt_lit_bootargs = private unnamed_addr constant [9 x i8] c"bootargs\00"
@__ncc_fdt_lit_status = private unnamed_addr constant [7 x i8] c"status\00"
@__ncc_fdt_lit_disabled = private unnamed_addr constant [9 x i8] c"disabled\00"
@__ncc_fdt_lit_device_type = private unnamed_addr constant [12 x i8] c"device_type\00"
@__ncc_fdt_lit_memory = private unnamed_addr constant [7 x i8] c"memory\00"
@__ncc_fdt_lit_cpu = private unnamed_addr constant [4 x i8] c"cpu\00"
@__ncc_fdt_lit_width = private unnamed_addr constant [6 x i8] c"width\00"
@__ncc_fdt_lit_height = private unnamed_addr constant [7 x i8] c"height\00"
@__ncc_fdt_lit_stride = private unnamed_addr constant [7 x i8] c"stride\00"
@__ncc_fdt_lit_format = private unnamed_addr constant [7 x i8] c"format\00"
@__ncc_fdt_lit_compatible = private unnamed_addr constant [11 x i8] c"compatible\00"
@__ncc_fdt_lit_simple_framebuffer = private unnamed_addr constant [19 x i8] c"simple-framebuffer\00"
@__ncc_fdt_lit_cpus = private unnamed_addr constant [5 x i8] c"cpus\00"
@__ncc_fdt_lit_chosen = private unnamed_addr constant [7 x i8] c"chosen\00"
@__ncc_fdt_lit_reserved_memory = private unnamed_addr constant [16 x i8] c"reserved-memory\00"
@__ncc_fdt_lit_memory_prefix = private unnamed_addr constant [7 x i8] c"memory@"
@__ncc_fdt_lit_framebuffer_prefix = private unnamed_addr constant [12 x i8] c"framebuffer@"

define internal i32 @__ncc_kernel_fdt_be32(ptr %address) {
entry:
  %raw = load i32, ptr %address, align 1
  %a = and i32 %raw, 255
  %a.shift = shl i32 %a, 24
  %b.raw = and i32 %raw, 65280
  %b = shl i32 %b.raw, 8
  %c.raw = and i32 %raw, 16711680
  %c = lshr i32 %c.raw, 8
  %d = lshr i32 %raw, 24
  %ab = or i32 %a.shift, %b
  %cd = or i32 %c, %d
  %value = or i32 %ab, %cd
  ret i32 %value
}

define internal i64 @__ncc_kernel_fdt_be64(ptr %address) {
entry:
  %high = call i32 @__ncc_kernel_fdt_be32(ptr %address)
  %low.ptr = getelementptr i8, ptr %address, i64 4
  %low = call i32 @__ncc_kernel_fdt_be32(ptr %low.ptr)
  %high.wide = zext i32 %high to i64
  %low.wide = zext i32 %low to i64
  %high.shift = shl i64 %high.wide, 32
  %value = or i64 %high.shift, %low.wide
  ret i64 %value
}

define internal i1 @__ncc_kernel_fdt_bytes_equal(ptr %value, i32 %available, ptr %literal, i32 %length) {
entry:
  %room = icmp ugt i32 %available, %length
  br i1 %room, label %loop, label %fail
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %same ]
  %done = icmp uge i32 %i, %length
  br i1 %done, label %terminator, label %compare
compare:
  %wide = zext i32 %i to i64
  %a.ptr = getelementptr i8, ptr %value, i64 %wide
  %b.ptr = getelementptr i8, ptr %literal, i64 %wide
  %a = load i8, ptr %a.ptr, align 1
  %b = load i8, ptr %b.ptr, align 1
  %equal = icmp eq i8 %a, %b
  br i1 %equal, label %same, label %fail
same:
  %next = add nuw i32 %i, 1
  br label %loop
terminator:
  %length.wide = zext i32 %length to i64
  %end.ptr = getelementptr i8, ptr %value, i64 %length.wide
  %end = load i8, ptr %end.ptr, align 1
  %zero = icmp eq i8 %end, 0
  ret i1 %zero
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_fdt_bytes_prefix(ptr %value, i32 %available, ptr %literal, i32 %length) {
entry:
  %room = icmp uge i32 %available, %length
  br i1 %room, label %loop, label %fail
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %same ]
  %done = icmp uge i32 %i, %length
  br i1 %done, label %hit, label %compare
compare:
  %wide = zext i32 %i to i64
  %a.ptr = getelementptr i8, ptr %value, i64 %wide
  %b.ptr = getelementptr i8, ptr %literal, i64 %wide
  %a = load i8, ptr %a.ptr, align 1
  %b = load i8, ptr %b.ptr, align 1
  %equal = icmp eq i8 %a, %b
  br i1 %equal, label %same, label %fail
same:
  %next = add nuw i32 %i, 1
  br label %loop
hit:
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_kernel_fdt_cells(ptr %value, i32 %cells) {
entry:
  %valid = icmp ule i32 %cells, 2
  br i1 %valid, label %loop, label %fail
loop:
  %i = phi i32 [ 0, %entry ], [ %next, %body ]
  %result = phi i64 [ 0, %entry ], [ %combined, %body ]
  %done = icmp uge i32 %i, %cells
  br i1 %done, label %finish, label %body
body:
  %wide = zext i32 %i to i64
  %offset = mul i64 %wide, 4
  %cell.ptr = getelementptr i8, ptr %value, i64 %offset
  %cell = call i32 @__ncc_kernel_fdt_be32(ptr %cell.ptr)
  %cell.wide = zext i32 %cell to i64
  %shifted = shl i64 %result, 32
  %combined = or i64 %shifted, %cell.wide
  %next = add nuw i32 %i, 1
  br label %loop
finish:
  ret i64 %result
fail:
  ret i64 0
}

define internal i1 @__ncc_kernel_fdt_append_memory(ptr %boot, i64 %base, i64 %length, i32 %kind) {
entry:
  %replace = load i1, ptr @__ncc_kernel_fdt_replace_memory, align 1
  %nonzero = icmp ne i64 %length, 0
  %allowed = and i1 %replace, %nonzero
  br i1 %allowed, label %capacity, label %skip
capacity:
  %count.ptr = getelementptr i8, ptr %boot, i64 32
  %count = load i64, ptr %count.ptr, align 8
  %room = icmp ult i64 %count, 256
  br i1 %room, label %write, label %fail
write:
  %map.ptr.ptr = getelementptr i8, ptr %boot, i64 24
  %map.address = load i64, ptr %map.ptr.ptr, align 8
  %map.present = icmp ne i64 %map.address, 0
  br i1 %map.present, label %store, label %fail
store:
  %map = inttoptr i64 %map.address to ptr
  %offset = mul i64 %count, 32
  %entry.ptr = getelementptr i8, ptr %map, i64 %offset
  store i64 %base, ptr %entry.ptr, align 8
  %length.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  store i64 %length, ptr %length.ptr, align 8
  %kind.ptr = getelementptr i8, ptr %entry.ptr, i64 16
  store i32 %kind, ptr %kind.ptr, align 4
  %flags.ptr = getelementptr i8, ptr %entry.ptr, i64 20
  %usable = icmp eq i32 %kind, 5
  %flags = select i1 %usable, i32 49, i32 17
  store i32 %flags, ptr %flags.ptr, align 4
  %source.ptr = getelementptr i8, ptr %entry.ptr, i64 24
  store i32 6, ptr %source.ptr, align 4
  %reserved.ptr = getelementptr i8, ptr %entry.ptr, i64 28
  store i32 0, ptr %reserved.ptr, align 4
  %next = add nuw i64 %count, 1
  store i64 %next, ptr %count.ptr, align 8
  %boot.flags.ptr = getelementptr i8, ptr %boot, i64 16
  %boot.flags = load i64, ptr %boot.flags.ptr, align 8
  %with.map = or i64 %boot.flags, 1
  store i64 %with.map, ptr %boot.flags.ptr, align 8
  ret i1 true
skip:
  ret i1 true
fail:
  ret i1 false
}

define internal i32 @__ncc_kernel_fdt_append_cpu(ptr %topology, i64 %hardware, i64 %bootstrap, i1 %enabled) {
entry:
  %count.ptr = getelementptr i8, ptr %topology, i64 12
  %count = load i32, ptr %count.ptr, align 4
  %room = icmp ult i32 %count, 256
  br i1 %room, label %dedupe.loop, label %fail
dedupe.loop:
  %i = phi i32 [ 0, %entry ], [ %next, %dedupe.continue ]
  %done = icmp uge i32 %i, %count
  br i1 %done, label %write, label %dedupe.body
dedupe.body:
  %wide = zext i32 %i to i64
  %entry.offset.body = mul i64 %wide, 48
  %entry.offset = add i64 %entry.offset.body, 32
  %entry.ptr = getelementptr i8, ptr %topology, i64 %entry.offset
  %hardware.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %candidate = load i64, ptr %hardware.ptr, align 8
  %same = icmp eq i64 %candidate, %hardware
  br i1 %same, label %duplicate, label %dedupe.continue
dedupe.continue:
  %next = add nuw i32 %i, 1
  br label %dedupe.loop
duplicate:
  ret i32 %i
write:
  %logical.wide = zext i32 %count to i64
  %offset.body = mul i64 %logical.wide, 48
  %offset = add i64 %offset.body, 32
  %entry.ptr.new = getelementptr i8, ptr %topology, i64 %offset
  store i32 %count, ptr %entry.ptr.new, align 4
  %flags.ptr = getelementptr i8, ptr %entry.ptr.new, i64 4
  %is.bootstrap = icmp eq i64 %hardware, %bootstrap
  %enabled.flags = select i1 %enabled, i32 5, i32 0
  %bootstrap.flag = select i1 %is.bootstrap, i32 2, i32 0
  %flags = or i32 %enabled.flags, %bootstrap.flag
  store i32 %flags, ptr %flags.ptr, align 4
  %hardware.ptr.new = getelementptr i8, ptr %entry.ptr.new, i64 8
  store i64 %hardware, ptr %hardware.ptr.new, align 8
  %firmware.ptr = getelementptr i8, ptr %entry.ptr.new, i64 16
  store i64 %hardware, ptr %firmware.ptr, align 8
  %numa.ptr = getelementptr i8, ptr %entry.ptr.new, i64 24
  store i32 0, ptr %numa.ptr, align 4
  %package.ptr = getelementptr i8, ptr %entry.ptr.new, i64 28
  store i16 0, ptr %package.ptr, align 2
  %core.ptr = getelementptr i8, ptr %entry.ptr.new, i64 30
  %core = trunc i32 %count to i16
  store i16 %core, ptr %core.ptr, align 2
  %thread.ptr = getelementptr i8, ptr %entry.ptr.new, i64 32
  store i16 0, ptr %thread.ptr, align 2
  %reserved.ptr = getelementptr i8, ptr %entry.ptr.new, i64 34
  store i16 0, ptr %reserved.ptr, align 2
  %stack.ptr = getelementptr i8, ptr %entry.ptr.new, i64 40
  store i64 0, ptr %stack.ptr, align 8
  %count.next = add nuw i32 %count, 1
  store i32 %count.next, ptr %count.ptr, align 4
  br i1 %is.bootstrap, label %record.bootstrap, label %finish
record.bootstrap:
  %bootstrap.ptr = getelementptr i8, ptr %topology, i64 24
  store i64 %hardware, ptr %bootstrap.ptr, align 8
  br label %finish
finish:
  ret i32 %count
fail:
  ret i32 -1
}

define internal void @__ncc_kernel_fdt_disable_cpu(ptr %topology, i32 %index) {
entry:
  %valid = icmp sge i32 %index, 0
  br i1 %valid, label %write, label %done
write:
  %wide = zext i32 %index to i64
  %offset.body = mul i64 %wide, 48
  %offset = add i64 %offset.body, 36
  %flags.ptr = getelementptr i8, ptr %topology, i64 %offset
  store i32 0, ptr %flags.ptr, align 4
  br label %done
done:
  ret void
}

define internal i8 @__ncc_kernel_fdt_classify_node(ptr %name, i32 %available, i8 %parent.kind) {
entry:
  %cpus = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %available, ptr @__ncc_fdt_lit_cpus, i32 4)
  br i1 %cpus, label %kind.cpus, label %chosen.test
chosen.test:
  %chosen = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %available, ptr @__ncc_fdt_lit_chosen, i32 6)
  br i1 %chosen, label %kind.chosen, label %reserved.test
reserved.test:
  %reserved = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %available, ptr @__ncc_fdt_lit_reserved_memory, i32 15)
  br i1 %reserved, label %kind.reserved.parent, label %parent.test
parent.test:
  %under.cpus = icmp eq i8 %parent.kind, 2
  br i1 %under.cpus, label %kind.cpu, label %under.reserved.test
under.reserved.test:
  %under.reserved = icmp eq i8 %parent.kind, 5
  br i1 %under.reserved, label %kind.reserved.child, label %memory.test
memory.test:
  %memory = call i1 @__ncc_kernel_fdt_bytes_prefix(ptr %name, i32 %available, ptr @__ncc_fdt_lit_memory_prefix, i32 7)
  br i1 %memory, label %kind.memory, label %framebuffer.test
framebuffer.test:
  %framebuffer = call i1 @__ncc_kernel_fdt_bytes_prefix(ptr %name, i32 %available, ptr @__ncc_fdt_lit_framebuffer_prefix, i32 12)
  br i1 %framebuffer, label %kind.framebuffer, label %kind.other
kind.memory:
  ret i8 1
kind.cpus:
  ret i8 2
kind.cpu:
  ret i8 3
kind.chosen:
  ret i8 4
kind.reserved.parent:
  ret i8 5
kind.reserved.child:
  ret i8 6
kind.framebuffer:
  ret i8 7
kind.other:
  ret i8 0
}

define internal i1 @__ncc_kernel_fdt_handle_reg(ptr %boot, ptr %topology, ptr %value, i32 %length, i32 %depth, i8 %kind, i64 %bootstrap) {
entry:
  %cpu = icmp eq i8 %kind, 3
  %memory = icmp eq i8 %kind, 1
  %reserved = icmp eq i8 %kind, 6
  %framebuffer = icmp eq i8 %kind, 7
  %region = or i1 %memory, %reserved
  %region.or.framebuffer = or i1 %region, %framebuffer
  %accepted = or i1 %cpu, %region.or.framebuffer
  br i1 %accepted, label %cell.contract, label %skip
cell.contract:
  %depth.wide = zext i32 %depth to i64
  %address.cells.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_parent_address_cells, i32 0, i64 %depth.wide
  %size.cells.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_parent_size_cells, i32 0, i64 %depth.wide
  %address.cells = load i32, ptr %address.cells.ptr, align 4
  %size.cells = load i32, ptr %size.cells.ptr, align 4
  %address.valid.low = icmp uge i32 %address.cells, 1
  %address.valid.high = icmp ule i32 %address.cells, 2
  %address.valid = and i1 %address.valid.low, %address.valid.high
  %size.valid = icmp ule i32 %size.cells, 2
  %cells = add i32 %address.cells, %size.cells
  %tuple.bytes = mul i32 %cells, 4
  %tuple.nonzero = icmp ne i32 %tuple.bytes, 0
  %a = and i1 %address.valid, %size.valid
  %valid = and i1 %a, %tuple.nonzero
  br i1 %valid, label %kind.test, label %fail
kind.test:
  br i1 %cpu, label %cpu.reg, label %regions.setup
cpu.reg:
  %cpu.address.bytes = mul i32 %address.cells, 4
  %cpu.enough = icmp uge i32 %length, %cpu.address.bytes
  br i1 %cpu.enough, label %cpu.decode, label %fail
cpu.decode:
  %hardware = call i64 @__ncc_kernel_fdt_cells(ptr %value, i32 %address.cells)
  %enabled.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_fdt_node_enabled, i32 0, i64 %depth.wide
  %enabled = load i1, ptr %enabled.ptr, align 1
  %index = call i32 @__ncc_kernel_fdt_append_cpu(ptr %topology, i64 %hardware, i64 %bootstrap, i1 %enabled)
  %index.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_cpu_index, i32 0, i64 %depth.wide
  store i32 %index, ptr %index.ptr, align 4
  %ok = icmp sge i32 %index, 0
  ret i1 %ok
regions.setup:
  br label %regions.loop
regions.loop:
  %offset = phi i32 [ 0, %regions.setup ], [ %next, %regions.continue ]
  %remaining = sub i32 %length, %offset
  %region.enough = icmp uge i32 %remaining, %tuple.bytes
  br i1 %region.enough, label %regions.decode, label %regions.finish
regions.decode:
  %offset.wide = zext i32 %offset to i64
  %tuple.ptr = getelementptr i8, ptr %value, i64 %offset.wide
  %base = call i64 @__ncc_kernel_fdt_cells(ptr %tuple.ptr, i32 %address.cells)
  %region.address.bytes = mul i32 %address.cells, 4
  %region.address.bytes.wide = zext i32 %region.address.bytes to i64
  %size.ptr = getelementptr i8, ptr %tuple.ptr, i64 %region.address.bytes.wide
  %size = call i64 @__ncc_kernel_fdt_cells(ptr %size.ptr, i32 %size.cells)
  br i1 %framebuffer, label %framebuffer.store, label %region.store
framebuffer.store:
  %fb.base.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_fdt_fb_base, i32 0, i64 %depth.wide
  %fb.size.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_fdt_fb_size, i32 0, i64 %depth.wide
  store i64 %base, ptr %fb.base.ptr, align 8
  store i64 %size, ptr %fb.size.ptr, align 8
  ret i1 true
region.store:
  %region.kind = select i1 %memory, i32 5, i32 0
  %stored = call i1 @__ncc_kernel_fdt_append_memory(ptr %boot, i64 %base, i64 %size, i32 %region.kind)
  br i1 %stored, label %regions.continue, label %fail
regions.continue:
  %next = add i32 %offset, %tuple.bytes
  br label %regions.loop
regions.finish:
  %exact = icmp eq i32 %remaining, 0
  ret i1 %exact
skip:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_fdt_commit_framebuffer(ptr %boot, i32 %depth) {
entry:
  %depth.wide = zext i32 %depth to i64
  %compatible.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_fdt_fb_compatible, i32 0, i64 %depth.wide
  %compatible = load i1, ptr %compatible.ptr, align 1
  %base.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_fdt_fb_base, i32 0, i64 %depth.wide
  %size.ptr = getelementptr [64 x i64], ptr @__ncc_kernel_fdt_fb_size, i32 0, i64 %depth.wide
  %width.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_width, i32 0, i64 %depth.wide
  %height.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_height, i32 0, i64 %depth.wide
  %stride.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_stride, i32 0, i64 %depth.wide
  %format.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_format, i32 0, i64 %depth.wide
  %base = load i64, ptr %base.ptr, align 8
  %size = load i64, ptr %size.ptr, align 8
  %width = load i32, ptr %width.ptr, align 4
  %height = load i32, ptr %height.ptr, align 4
  %stride = load i32, ptr %stride.ptr, align 4
  %format = load i32, ptr %format.ptr, align 4
  %base.ok = icmp ne i64 %base, 0
  %size.ok = icmp ne i64 %size, 0
  %width.ok = icmp ne i32 %width, 0
  %height.ok = icmp ne i32 %height, 0
  %stride.ok = icmp ne i32 %stride, 0
  %a = and i1 %compatible, %base.ok
  %b = and i1 %size.ok, %width.ok
  %c = and i1 %height.ok, %stride.ok
  %d = and i1 %a, %b
  %valid = and i1 %d, %c
  br i1 %valid, label %write, label %skip
write:
  %framebuffer.ptr.ptr = getelementptr i8, ptr %boot, i64 40
  %framebuffer.address = load i64, ptr %framebuffer.ptr.ptr, align 8
  %present = icmp ne i64 %framebuffer.address, 0
  br i1 %present, label %store, label %skip
store:
  %framebuffer = inttoptr i64 %framebuffer.address to ptr
  store i64 %base, ptr %framebuffer, align 8
  %width.dst = getelementptr i8, ptr %framebuffer, i64 8
  store i32 %width, ptr %width.dst, align 4
  %height.dst = getelementptr i8, ptr %framebuffer, i64 12
  store i32 %height, ptr %height.dst, align 4
  %stride.dst = getelementptr i8, ptr %framebuffer, i64 16
  store i32 %stride, ptr %stride.dst, align 4
  %format.dst = getelementptr i8, ptr %framebuffer, i64 20
  store i32 %format, ptr %format.dst, align 4
  %flags.dst = getelementptr i8, ptr %framebuffer, i64 24
  store i64 %size, ptr %flags.dst, align 8
  %boot.flags.ptr = getelementptr i8, ptr %boot, i64 16
  %boot.flags = load i64, ptr %boot.flags.ptr, align 8
  %with.framebuffer = or i64 %boot.flags, 8
  store i64 %with.framebuffer, ptr %boot.flags.ptr, align 8
  ret i1 true
skip:
  ret i1 true
}

define internal i1 @__ncc_kernel_fdt_handle_property(ptr %boot, ptr %topology, ptr %strings, i32 %strings.size, i32 %name.offset, ptr %value, i32 %length, i32 %depth, i64 %bootstrap) {
entry:
  %name.valid = icmp ult i32 %name.offset, %strings.size
  br i1 %name.valid, label %name.address, label %fail
name.address:
  %name.offset.wide = zext i32 %name.offset to i64
  %name = getelementptr i8, ptr %strings, i64 %name.offset.wide
  %name.available = sub i32 %strings.size, %name.offset
  %depth.wide = zext i32 %depth to i64
  %kind.ptr = getelementptr [64 x i8], ptr @__ncc_kernel_fdt_node_kind, i32 0, i64 %depth.wide
  %kind = load i8, ptr %kind.ptr, align 1
  %address.cells = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_address_cells, i32 14)
  br i1 %address.cells, label %set.address.cells, label %size.cells.test
set.address.cells:
  %address.length.ok = icmp eq i32 %length, 4
  br i1 %address.length.ok, label %set.address.cells.value, label %fail
set.address.cells.value:
  %address.value = call i32 @__ncc_kernel_fdt_be32(ptr %value)
  %address.range = icmp ule i32 %address.value, 4
  br i1 %address.range, label %set.address.cells.store, label %fail
set.address.cells.store:
  %address.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_child_address_cells, i32 0, i64 %depth.wide
  store i32 %address.value, ptr %address.dst, align 4
  ret i1 true
size.cells.test:
  %size.cells = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_size_cells, i32 11)
  br i1 %size.cells, label %set.size.cells, label %status.test
set.size.cells:
  %size.length.ok = icmp eq i32 %length, 4
  br i1 %size.length.ok, label %set.size.cells.value, label %fail
set.size.cells.value:
  %size.value = call i32 @__ncc_kernel_fdt_be32(ptr %value)
  %size.range = icmp ule i32 %size.value, 4
  br i1 %size.range, label %set.size.cells.store, label %fail
set.size.cells.store:
  %size.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_child_size_cells, i32 0, i64 %depth.wide
  store i32 %size.value, ptr %size.dst, align 4
  ret i1 true
status.test:
  %status = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_status, i32 6)
  br i1 %status, label %status.value, label %device.test
status.value:
  %disabled = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %value, i32 %length, ptr @__ncc_fdt_lit_disabled, i32 8)
  br i1 %disabled, label %status.disable, label %ok
status.disable:
  %enabled.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_fdt_node_enabled, i32 0, i64 %depth.wide
  store i1 false, ptr %enabled.ptr, align 1
  %index.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_cpu_index, i32 0, i64 %depth.wide
  %index = load i32, ptr %index.ptr, align 4
  call void @__ncc_kernel_fdt_disable_cpu(ptr %topology, i32 %index)
  ret i1 true
device.test:
  %device = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_device_type, i32 11)
  br i1 %device, label %device.value, label %reg.test
device.value:
  %memory = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %value, i32 %length, ptr @__ncc_fdt_lit_memory, i32 6)
  br i1 %memory, label %device.memory, label %device.cpu.test
device.memory:
  store i8 1, ptr %kind.ptr, align 1
  ret i1 true
device.cpu.test:
  %cpu = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %value, i32 %length, ptr @__ncc_fdt_lit_cpu, i32 3)
  br i1 %cpu, label %device.cpu, label %ok
device.cpu:
  store i8 3, ptr %kind.ptr, align 1
  ret i1 true
reg.test:
  %reg = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_reg, i32 3)
  br i1 %reg, label %reg.handle, label %bootargs.test
reg.handle:
  %reg.ok = call i1 @__ncc_kernel_fdt_handle_reg(ptr %boot, ptr %topology, ptr %value, i32 %length, i32 %depth, i8 %kind, i64 %bootstrap)
  ret i1 %reg.ok
bootargs.test:
  %bootargs = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_bootargs, i32 8)
  %chosen = icmp eq i8 %kind, 4
  %is.bootargs = and i1 %bootargs, %chosen
  br i1 %is.bootargs, label %bootargs.store, label %compatible.test
bootargs.store:
  %command.ptr = getelementptr i8, ptr %boot, i64 64
  %command.address = ptrtoint ptr %value to i64
  store i64 %command.address, ptr %command.ptr, align 8
  ret i1 true
compatible.test:
  %compatible.name = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_compatible, i32 10)
  %framebuffer = icmp eq i8 %kind, 7
  %is.compatible = and i1 %compatible.name, %framebuffer
  br i1 %is.compatible, label %compatible.value, label %width.test
compatible.value:
  %compatible = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %value, i32 %length, ptr @__ncc_fdt_lit_simple_framebuffer, i32 18)
  %compatible.ptr = getelementptr [64 x i1], ptr @__ncc_kernel_fdt_fb_compatible, i32 0, i64 %depth.wide
  store i1 %compatible, ptr %compatible.ptr, align 1
  ret i1 true
width.test:
  %width = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_width, i32 5)
  %is.width = and i1 %width, %framebuffer
  br i1 %is.width, label %width.store, label %height.test
width.store:
  %width.length = icmp eq i32 %length, 4
  br i1 %width.length, label %width.value, label %fail
width.value:
  %width.raw = call i32 @__ncc_kernel_fdt_be32(ptr %value)
  %width.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_width, i32 0, i64 %depth.wide
  store i32 %width.raw, ptr %width.ptr, align 4
  ret i1 true
height.test:
  %height = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_height, i32 6)
  %is.height = and i1 %height, %framebuffer
  br i1 %is.height, label %height.store, label %stride.test
height.store:
  %height.length = icmp eq i32 %length, 4
  br i1 %height.length, label %height.value, label %fail
height.value:
  %height.raw = call i32 @__ncc_kernel_fdt_be32(ptr %value)
  %height.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_height, i32 0, i64 %depth.wide
  store i32 %height.raw, ptr %height.ptr, align 4
  ret i1 true
stride.test:
  %stride = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_stride, i32 6)
  %is.stride = and i1 %stride, %framebuffer
  br i1 %is.stride, label %stride.store, label %format.test
stride.store:
  %stride.length = icmp eq i32 %length, 4
  br i1 %stride.length, label %stride.value, label %fail
stride.value:
  %stride.raw = call i32 @__ncc_kernel_fdt_be32(ptr %value)
  %stride.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_stride, i32 0, i64 %depth.wide
  store i32 %stride.raw, ptr %stride.ptr, align 4
  ret i1 true
format.test:
  %format = call i1 @__ncc_kernel_fdt_bytes_equal(ptr %name, i32 %name.available, ptr @__ncc_fdt_lit_format, i32 6)
  %is.format = and i1 %format, %framebuffer
  br i1 %is.format, label %format.store, label %ok
format.store:
  %format.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_fb_format, i32 0, i64 %depth.wide
  store i32 0, ptr %format.ptr, align 4
  ret i1 true
ok:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_kernel_fdt_parse(ptr %boot, ptr %fdt, ptr %topology) {
entry:
  %magic = call i32 @__ncc_kernel_fdt_be32(ptr %fdt)
  %magic.ok = icmp eq i32 %magic, -804389139
  br i1 %magic.ok, label %header, label %fail
header:
  %total.ptr = getelementptr i8, ptr %fdt, i64 4
  %total = call i32 @__ncc_kernel_fdt_be32(ptr %total.ptr)
  %struct.offset.ptr = getelementptr i8, ptr %fdt, i64 8
  %struct.offset = call i32 @__ncc_kernel_fdt_be32(ptr %struct.offset.ptr)
  %strings.offset.ptr = getelementptr i8, ptr %fdt, i64 12
  %strings.offset = call i32 @__ncc_kernel_fdt_be32(ptr %strings.offset.ptr)
  %reserve.offset.ptr = getelementptr i8, ptr %fdt, i64 16
  %reserve.offset = call i32 @__ncc_kernel_fdt_be32(ptr %reserve.offset.ptr)
  %version.ptr = getelementptr i8, ptr %fdt, i64 20
  %version = call i32 @__ncc_kernel_fdt_be32(ptr %version.ptr)
  %boot.cpu.ptr = getelementptr i8, ptr %fdt, i64 28
  %boot.cpu.raw = call i32 @__ncc_kernel_fdt_be32(ptr %boot.cpu.ptr)
  %boot.cpu.fdt = zext i32 %boot.cpu.raw to i64
  %strings.size.ptr = getelementptr i8, ptr %fdt, i64 32
  %strings.size = call i32 @__ncc_kernel_fdt_be32(ptr %strings.size.ptr)
  %struct.size.ptr = getelementptr i8, ptr %fdt, i64 36
  %struct.size = call i32 @__ncc_kernel_fdt_be32(ptr %struct.size.ptr)
  %total.min = icmp uge i32 %total, 40
  %total.max = icmp ule i32 %total, 16777216
  %version.ok = icmp uge i32 %version, 16
  %a = and i1 %total.min, %total.max
  %header.ok = and i1 %a, %version.ok
  br i1 %header.ok, label %ranges, label %fail
ranges:
  %struct.end = add i32 %struct.offset, %struct.size
  %strings.end = add i32 %strings.offset, %strings.size
  %struct.no.overflow = icmp uge i32 %struct.end, %struct.offset
  %strings.no.overflow = icmp uge i32 %strings.end, %strings.offset
  %struct.in = icmp ule i32 %struct.end, %total
  %strings.in = icmp ule i32 %strings.end, %total
  %reserve.in = icmp ult i32 %reserve.offset, %total
  %b = and i1 %struct.no.overflow, %strings.no.overflow
  %c = and i1 %struct.in, %strings.in
  %d = and i1 %b, %c
  %e = and i1 %d, %reserve.in
  br i1 %e, label %platform, label %fail
platform:
  %fdt.entry = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 6)
  %fdt.entry.ok = icmp ne ptr %fdt.entry, null
  br i1 %fdt.entry.ok, label %platform.write, label %fail
platform.write:
  %fdt.length.ptr = getelementptr i8, ptr %fdt.entry, i64 16
  %total.wide = zext i32 %total to i64
  store i64 %total.wide, ptr %fdt.length.ptr, align 8
  %hart.entry = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 7)
  %has.hart = icmp ne ptr %hart.entry, null
  br i1 %has.hart, label %hart.read, label %bootstrap.fdt
hart.read:
  %hart.ptr = getelementptr i8, ptr %hart.entry, i64 8
  %hart = load i64, ptr %hart.ptr, align 8
  br label %bootstrap.ready
bootstrap.fdt:
  br label %bootstrap.ready
bootstrap.ready:
  %bootstrap = phi i64 [ %hart, %hart.read ], [ %boot.cpu.fdt, %bootstrap.fdt ]
  %topology.count.ptr = getelementptr i8, ptr %topology, i64 12
  store i32 0, ptr %topology.count.ptr, align 4
  %topology.bootstrap.ptr = getelementptr i8, ptr %topology, i64 24
  store i64 %bootstrap, ptr %topology.bootstrap.ptr, align 8
  %boot.count.ptr = getelementptr i8, ptr %boot, i64 32
  %boot.count = load i64, ptr %boot.count.ptr, align 8
  %replace.memory = icmp eq i64 %boot.count, 0
  store i1 %replace.memory, ptr @__ncc_kernel_fdt_replace_memory, align 1
  br i1 %replace.memory, label %memory.reset, label %reserve.setup
memory.reset:
  store i64 0, ptr %boot.count.ptr, align 8
  %boot.flags.ptr = getelementptr i8, ptr %boot, i64 16
  %boot.flags = load i64, ptr %boot.flags.ptr, align 8
  %without.map = and i64 %boot.flags, -2
  store i64 %without.map, ptr %boot.flags.ptr, align 8
  br label %reserve.setup
reserve.setup:
  %reserve.offset.wide = zext i32 %reserve.offset to i64
  %reserve = getelementptr i8, ptr %fdt, i64 %reserve.offset.wide
  br label %reserve.loop
reserve.loop:
  %reserve.cursor = phi i32 [ 0, %reserve.setup ], [ %reserve.next, %reserve.store ]
  %reserve.required = add i32 %reserve.cursor, 16
  %reserve.inside = icmp ule i32 %reserve.required, %total
  br i1 %reserve.inside, label %reserve.read, label %fail
reserve.read:
  %reserve.cursor.wide = zext i32 %reserve.cursor to i64
  %reserve.entry = getelementptr i8, ptr %reserve, i64 %reserve.cursor.wide
  %reserve.base = call i64 @__ncc_kernel_fdt_be64(ptr %reserve.entry)
  %reserve.size.ptr = getelementptr i8, ptr %reserve.entry, i64 8
  %reserve.size = call i64 @__ncc_kernel_fdt_be64(ptr %reserve.size.ptr)
  %reserve.base.zero = icmp eq i64 %reserve.base, 0
  %reserve.size.zero = icmp eq i64 %reserve.size, 0
  %reserve.end = and i1 %reserve.base.zero, %reserve.size.zero
  br i1 %reserve.end, label %structure.setup, label %reserve.store
reserve.store:
  %reserve.ok = call i1 @__ncc_kernel_fdt_append_memory(ptr %boot, i64 %reserve.base, i64 %reserve.size, i32 0)
  %reserve.next = add i32 %reserve.cursor, 16
  br i1 %reserve.ok, label %reserve.loop, label %fail
structure.setup:
  %struct.offset.wide = zext i32 %struct.offset to i64
  %structure = getelementptr i8, ptr %fdt, i64 %struct.offset.wide
  %strings.offset.wide = zext i32 %strings.offset to i64
  %strings = getelementptr i8, ptr %fdt, i64 %strings.offset.wide
  br label %token.loop
token.loop:
  %cursor = phi i32 [ 0, %structure.setup ], [ %cursor.next, %token.continue ]
  %depth = phi i32 [ -1, %structure.setup ], [ %depth.next, %token.continue ]
  %token.required = add i32 %cursor, 4
  %token.inside = icmp ule i32 %token.required, %struct.size
  br i1 %token.inside, label %token.read, label %fail
token.read:
  %cursor.wide = zext i32 %cursor to i64
  %token.ptr = getelementptr i8, ptr %structure, i64 %cursor.wide
  %token = call i32 @__ncc_kernel_fdt_be32(ptr %token.ptr)
  %after.token = add i32 %cursor, 4
  switch i32 %token, label %fail [
    i32 1, label %begin.node
    i32 2, label %end.node
    i32 3, label %property
    i32 4, label %nop
    i32 9, label %finish
  ]
begin.node:
  %new.depth = add i32 %depth, 1
  %depth.nonnegative = icmp sge i32 %new.depth, 0
  %depth.bounded = icmp ult i32 %new.depth, 64
  %depth.ok = and i1 %depth.nonnegative, %depth.bounded
  br i1 %depth.ok, label %node.name.loop, label %fail
node.name.loop:
  %name.index = phi i32 [ 0, %begin.node ], [ %name.next, %node.name.continue ]
  %name.position = add i32 %after.token, %name.index
  %name.inside = icmp ult i32 %name.position, %struct.size
  br i1 %name.inside, label %node.name.read, label %fail
node.name.read:
  %name.position.wide = zext i32 %name.position to i64
  %name.byte.ptr = getelementptr i8, ptr %structure, i64 %name.position.wide
  %name.byte = load i8, ptr %name.byte.ptr, align 1
  %name.end = icmp eq i8 %name.byte, 0
  br i1 %name.end, label %node.prepare, label %node.name.continue
node.name.continue:
  %name.next = add nuw i32 %name.index, 1
  %name.limit = icmp ule i32 %name.next, 255
  br i1 %name.limit, label %node.name.loop, label %fail
node.prepare:
  %name.start.wide = zext i32 %after.token to i64
  %name.ptr = getelementptr i8, ptr %structure, i64 %name.start.wide
  %name.available = sub i32 %struct.size, %after.token
  %is.root = icmp eq i32 %new.depth, 0
  br i1 %is.root, label %node.root, label %node.parent
node.root:
  br label %node.classify
node.parent:
  %parent.depth = sub i32 %new.depth, 1
  %parent.depth.wide = zext i32 %parent.depth to i64
  %parent.kind.ptr = getelementptr [64 x i8], ptr @__ncc_kernel_fdt_node_kind, i32 0, i64 %parent.depth.wide
  %parent.kind.value = load i8, ptr %parent.kind.ptr, align 1
  %parent.address.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_child_address_cells, i32 0, i64 %parent.depth.wide
  %parent.size.ptr = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_child_size_cells, i32 0, i64 %parent.depth.wide
  %parent.address.value = load i32, ptr %parent.address.ptr, align 4
  %parent.size.value = load i32, ptr %parent.size.ptr, align 4
  br label %node.classify
node.classify:
  %parent.kind = phi i8 [ 0, %node.root ], [ %parent.kind.value, %node.parent ]
  %parent.address = phi i32 [ 2, %node.root ], [ %parent.address.value, %node.parent ]
  %parent.size = phi i32 [ 1, %node.root ], [ %parent.size.value, %node.parent ]
  %kind = call i8 @__ncc_kernel_fdt_classify_node(ptr %name.ptr, i32 %name.available, i8 %parent.kind)
  %new.depth.wide = zext i32 %new.depth to i64
  %kind.dst = getelementptr [64 x i8], ptr @__ncc_kernel_fdt_node_kind, i32 0, i64 %new.depth.wide
  %enabled.dst = getelementptr [64 x i1], ptr @__ncc_kernel_fdt_node_enabled, i32 0, i64 %new.depth.wide
  %parent.address.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_parent_address_cells, i32 0, i64 %new.depth.wide
  %parent.size.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_parent_size_cells, i32 0, i64 %new.depth.wide
  %child.address.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_child_address_cells, i32 0, i64 %new.depth.wide
  %child.size.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_child_size_cells, i32 0, i64 %new.depth.wide
  %cpu.index.dst = getelementptr [64 x i32], ptr @__ncc_kernel_fdt_cpu_index, i32 0, i64 %new.depth.wide
  store i8 %kind, ptr %kind.dst, align 1
  store i1 true, ptr %enabled.dst, align 1
  store i32 %parent.address, ptr %parent.address.dst, align 4
  store i32 %parent.size, ptr %parent.size.dst, align 4
  store i32 %parent.address, ptr %child.address.dst, align 4
  store i32 %parent.size, ptr %child.size.dst, align 4
  store i32 -1, ptr %cpu.index.dst, align 4
  %name.end.position = add i32 %after.token, %name.index
  %name.after.zero = add i32 %name.end.position, 1
  %name.aligned.add = add i32 %name.after.zero, 3
  %name.aligned = and i32 %name.aligned.add, -4
  br label %token.continue
end.node:
  %depth.valid = icmp sge i32 %depth, 0
  br i1 %depth.valid, label %end.inspect, label %fail
end.inspect:
  %depth.wide.end = zext i32 %depth to i64
  %kind.end.ptr = getelementptr [64 x i8], ptr @__ncc_kernel_fdt_node_kind, i32 0, i64 %depth.wide.end
  %kind.end = load i8, ptr %kind.end.ptr, align 1
  %is.framebuffer = icmp eq i8 %kind.end, 7
  br i1 %is.framebuffer, label %end.framebuffer, label %end.complete
end.framebuffer:
  %framebuffer.ok = call i1 @__ncc_kernel_fdt_commit_framebuffer(ptr %boot, i32 %depth)
  br i1 %framebuffer.ok, label %end.complete, label %fail
end.complete:
  %popped.depth = sub i32 %depth, 1
  br label %token.continue
property:
  %property.header.end = add i32 %after.token, 8
  %property.header.ok = icmp ule i32 %property.header.end, %struct.size
  %property.depth.ok = icmp sge i32 %depth, 0
  %property.pre.ok = and i1 %property.header.ok, %property.depth.ok
  br i1 %property.pre.ok, label %property.header, label %fail
property.header:
  %property.header.offset.wide = zext i32 %after.token to i64
  %property.header.ptr = getelementptr i8, ptr %structure, i64 %property.header.offset.wide
  %property.length = call i32 @__ncc_kernel_fdt_be32(ptr %property.header.ptr)
  %property.name.ptr = getelementptr i8, ptr %property.header.ptr, i64 4
  %property.name = call i32 @__ncc_kernel_fdt_be32(ptr %property.name.ptr)
  %property.value.offset = add i32 %after.token, 8
  %property.value.end = add i32 %property.value.offset, %property.length
  %property.no.overflow = icmp uge i32 %property.value.end, %property.value.offset
  %property.inside = icmp ule i32 %property.value.end, %struct.size
  %property.range.ok = and i1 %property.no.overflow, %property.inside
  br i1 %property.range.ok, label %property.handle, label %fail
property.handle:
  %property.value.offset.wide = zext i32 %property.value.offset to i64
  %property.value = getelementptr i8, ptr %structure, i64 %property.value.offset.wide
  %property.ok = call i1 @__ncc_kernel_fdt_handle_property(ptr %boot, ptr %topology, ptr %strings, i32 %strings.size, i32 %property.name, ptr %property.value, i32 %property.length, i32 %depth, i64 %bootstrap)
  %property.aligned.add = add i32 %property.value.end, 3
  %property.aligned = and i32 %property.aligned.add, -4
  br i1 %property.ok, label %token.continue, label %fail
nop:
  br label %token.continue
token.continue:
  %cursor.next = phi i32 [ %name.aligned, %node.classify ], [ %after.token, %end.complete ], [ %property.aligned, %property.handle ], [ %after.token, %nop ]
  %depth.next = phi i32 [ %new.depth, %node.classify ], [ %popped.depth, %end.complete ], [ %depth, %property.handle ], [ %depth, %nop ]
  br label %token.loop
finish:
  %tree.closed = icmp eq i32 %depth, -1
  br i1 %tree.closed, label %topology.finish, label %fail
topology.finish:
  %topology.count = load i32, ptr %topology.count.ptr, align 4
  %has.cpu = icmp ne i32 %topology.count, 0
  br i1 %has.cpu, label %validate.memory, label %cpu.fallback
cpu.fallback:
  %fallback = call i32 @__ncc_kernel_fdt_append_cpu(ptr %topology, i64 %bootstrap, i64 %bootstrap, i1 true)
  %fallback.ok = icmp sge i32 %fallback, 0
  br i1 %fallback.ok, label %validate.memory, label %fail
validate.memory:
  %replace.final = load i1, ptr @__ncc_kernel_fdt_replace_memory, align 1
  br i1 %replace.final, label %validate.memory.count, label %success
validate.memory.count:
  %memory.count = load i64, ptr %boot.count.ptr, align 8
  %has.memory = icmp ne i64 %memory.count, 0
  br i1 %has.memory, label %success, label %fail
success:
  ret i1 true
fail:
  ret i1 false
}

define i1 @__ncc_kernel_firmware_adapt() {
entry:
  %state = load i8, ptr @__ncc_kernel_firmware_adapter_state, align 1
  switch i8 %state, label %begin [
    i8 1, label %fail
    i8 2, label %success
    i8 3, label %fail
  ]
begin:
  store i8 1, ptr @__ncc_kernel_firmware_adapter_state, align 1
  %boot = call ptr @__ncc_kernel_boot_info_header()
  %boot.ok = icmp ne ptr %boot, null
  br i1 %boot.ok, label %platform, label %mark.fail
platform:
  %fdt.entry = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 6)
  %has.entry = icmp ne ptr %fdt.entry, null
  br i1 %has.entry, label %fdt.address, label %mark.success
fdt.address:
  %fdt.ptr.ptr = getelementptr i8, ptr %fdt.entry, i64 8
  %fdt.value = load i64, ptr %fdt.ptr.ptr, align 8
  %has.fdt = icmp ne i64 %fdt.value, 0
  br i1 %has.fdt, label %topology, label %mark.success
topology:
  %topology.entry = call ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 10)
  %has.topology = icmp ne ptr %topology.entry, null
  br i1 %has.topology, label %topology.address, label %mark.fail
topology.address:
  %topology.ptr.ptr = getelementptr i8, ptr %topology.entry, i64 8
  %topology.value = load i64, ptr %topology.ptr.ptr, align 8
  %topology.present = icmp ne i64 %topology.value, 0
  br i1 %topology.present, label %parse, label %mark.fail
parse:
  %fdt = inttoptr i64 %fdt.value to ptr
  %topology.ptr = inttoptr i64 %topology.value to ptr
  %parsed = call i1 @__ncc_kernel_fdt_parse(ptr %boot, ptr %fdt, ptr %topology.ptr)
  br i1 %parsed, label %mark.success, label %mark.fail
mark.success:
  store i8 2, ptr @__ncc_kernel_firmware_adapter_state, align 1
  br label %success
mark.fail:
  store i8 3, ptr @__ncc_kernel_firmware_adapter_state, align 1
  br label %fail
success:
  ret i1 true
fail:
  ret i1 false
}
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fdt_runtime_materializes_real_boot_fields() {
        let llvm = render_kernel_firmware_fdt_runtime();
        assert!(llvm.contains("define i1 @__ncc_kernel_firmware_adapt"));
        assert!(llvm.contains("define internal i1 @__ncc_kernel_fdt_parse"));
        assert!(llvm.contains("@__ncc_kernel_fdt_append_memory"));
        assert!(llvm.contains("@__ncc_kernel_fdt_append_cpu"));
        assert!(llvm.contains("@__ncc_kernel_fdt_commit_framebuffer"));
        assert!(!llvm.contains("0x47ff0000"));
        assert!(!llvm.contains("0x87ff0000"));
    }

    #[test]
    fn optional_external_fdt_executes_through_the_freestanding_llvm_parser() {
        let Ok(path) = std::env::var("NCC_FDT_FIXTURE") else {
            return;
        };
        if Command::new("clang").arg("--version").output().is_err() {
            return;
        }
        let bytes = fs::read(path).expect("read NCC_FDT_FIXTURE");
        let mut encoded = String::with_capacity(bytes.len() * 3);
        for byte in &bytes {
            write!(encoded, "\\{byte:02X}").unwrap();
        }
        let harness = format!(
            r#"
@__ncc_test_fdt = private constant [{length} x i8] c"{encoded}", align 16
@__ncc_test_boot = global [88 x i8] zeroinitializer, align 16
@__ncc_test_map = global [8192 x i8] zeroinitializer, align 16
@__ncc_test_fb = global [32 x i8] zeroinitializer, align 16
@__ncc_test_topology = global [12320 x i8] zeroinitializer, align 16
@__ncc_test_fdt_entry = global [32 x i8] zeroinitializer, align 16
@__ncc_test_hart_entry = global [32 x i8] zeroinitializer, align 16

define ptr @__ncc_kernel_boot_info_header() {{
entry:
  ret ptr @__ncc_test_boot
}}

define ptr @__ncc_kernel_boot_platform_entry_raw(ptr %boot, i64 %kind) {{
entry:
  %fdt = icmp eq i64 %kind, 6
  br i1 %fdt, label %return.fdt, label %hart.test
hart.test:
  %hart = icmp eq i64 %kind, 7
  br i1 %hart, label %return.hart, label %none
return.fdt:
  ret ptr @__ncc_test_fdt_entry
return.hart:
  ret ptr @__ncc_test_hart_entry
none:
  ret ptr null
}}

define i32 @main() {{
entry:
  %map.slot = getelementptr i8, ptr @__ncc_test_boot, i64 24
  %map.address = ptrtoint ptr @__ncc_test_map to i64
  store i64 %map.address, ptr %map.slot, align 8
  %fb.slot = getelementptr i8, ptr @__ncc_test_boot, i64 40
  %fb.address = ptrtoint ptr @__ncc_test_fb to i64
  store i64 %fb.address, ptr %fb.slot, align 8
  %fdt.slot = getelementptr i8, ptr @__ncc_test_fdt_entry, i64 8
  %fdt.address = ptrtoint ptr @__ncc_test_fdt to i64
  store i64 %fdt.address, ptr %fdt.slot, align 8
  %parsed = call i1 @__ncc_kernel_fdt_parse(ptr @__ncc_test_boot, ptr @__ncc_test_fdt, ptr @__ncc_test_topology)
  br i1 %parsed, label %memory, label %fail.parse
memory:
  %memory.count.ptr = getelementptr i8, ptr @__ncc_test_boot, i64 32
  %memory.count = load i64, ptr %memory.count.ptr, align 8
  %has.memory = icmp ne i64 %memory.count, 0
  br i1 %has.memory, label %cpu, label %fail.memory
cpu:
  %cpu.count.ptr = getelementptr i8, ptr @__ncc_test_topology, i64 12
  %cpu.count = load i32, ptr %cpu.count.ptr, align 4
  %has.cpu = icmp ne i32 %cpu.count, 0
  br i1 %has.cpu, label %success, label %fail.cpu
success:
  ret i32 0
fail.parse:
  ret i32 10
fail.memory:
  ret i32 11
fail.cpu:
  ret i32 12
}}
"#,
            length = bytes.len()
        );
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("ncc-fdt-runtime-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let llvm_path = root.join("fdt-runtime.ll");
        let executable = root.join("fdt-runtime");
        fs::write(
            &llvm_path,
            format!("{}\n{harness}", render_kernel_firmware_fdt_runtime()),
        )
        .unwrap();
        let compile = Command::new("clang")
            .args(["-O2", "-x", "ir"])
            .arg(&llvm_path)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "clang rejected FDT runtime harness:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&executable).output().unwrap();
        assert!(
            run.status.success(),
            "freestanding FDT parser exited with {:?}",
            run.status.code()
        );
        let _ = fs::remove_dir_all(root);
    }
}
