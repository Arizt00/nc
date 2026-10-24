pub(crate) struct MemoryBufferRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const MEMORY_BUFFER_MANUAL_CALLS: &[MemoryBufferRuntimeCall] = &[
    call(
        "nC.runtime.buffer.bufferStride",
        "__ncc_buffer_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferAt",
        "__ncc_buffer_at",
        &["ptr", "ptr", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferData",
        "__ncc_buffer_data",
        &["ptr"],
        "ptr",
    ),
    call(
        "nC.runtime.buffer.bufferLen",
        "__ncc_buffer_len",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferCapacity",
        "__ncc_buffer_capacity",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferFlags",
        "__ncc_buffer_flags",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferSetLen",
        "__ncc_buffer_set_len",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferRead8",
        "__ncc_buffer_read8",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferRead16",
        "__ncc_buffer_read16",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferRead32",
        "__ncc_buffer_read32",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferRead64",
        "__ncc_buffer_read64",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.buffer.bufferWrite8",
        "__ncc_buffer_write8",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferWrite16",
        "__ncc_buffer_write16",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferWrite32",
        "__ncc_buffer_write32",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferWrite64",
        "__ncc_buffer_write64",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferFill",
        "__ncc_buffer_fill",
        &["ptr", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferCopy",
        "__ncc_buffer_copy",
        &["ptr", "i64", "ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferView",
        "__ncc_buffer_view",
        &["ptr", "ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.buffer.bufferClear",
        "__ncc_buffer_clear",
        &["ptr"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> MemoryBufferRuntimeCall {
    MemoryBufferRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(
    target: &str,
    arity: usize,
) -> Option<&'static MemoryBufferRuntimeCall> {
    MEMORY_BUFFER_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    MEMORY_BUFFER_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

pub(crate) fn render_memory_buffer_manual_runtime() -> &'static str {
    r#"
define internal ptr @__ncc_buffer_slot(ptr %descriptor, i64 %offset) {
  %slot = getelementptr i8, ptr %descriptor, i64 %offset
  ret ptr %slot
}

define i64 @__ncc_buffer_stride() { ret i64 32 }

define i1 @__ncc_buffer_at(ptr %descriptor, ptr %data, i64 %length, i64 %capacity, i64 %flags) {
  %len_slot = getelementptr i8, ptr %descriptor, i64 8
  %cap_slot = getelementptr i8, ptr %descriptor, i64 16
  %flags_slot = getelementptr i8, ptr %descriptor, i64 24
  store ptr %data, ptr %descriptor, align 1
  store i64 %length, ptr %len_slot, align 1
  store i64 %capacity, ptr %cap_slot, align 1
  store i64 %flags, ptr %flags_slot, align 1
  ret i1 true
}

define ptr @__ncc_buffer_data(ptr %descriptor) {
  %value = load ptr, ptr %descriptor, align 1
  ret ptr %value
}

define i64 @__ncc_buffer_len(ptr %descriptor) {
  %slot = getelementptr i8, ptr %descriptor, i64 8
  %value = load i64, ptr %slot, align 1
  ret i64 %value
}

define i64 @__ncc_buffer_capacity(ptr %descriptor) {
  %slot = getelementptr i8, ptr %descriptor, i64 16
  %value = load i64, ptr %slot, align 1
  ret i64 %value
}

define i64 @__ncc_buffer_flags(ptr %descriptor) {
  %slot = getelementptr i8, ptr %descriptor, i64 24
  %value = load i64, ptr %slot, align 1
  ret i64 %value
}

define i1 @__ncc_buffer_set_len(ptr %descriptor, i64 %length) {
  %slot = getelementptr i8, ptr %descriptor, i64 8
  store i64 %length, ptr %slot, align 1
  ret i1 true
}

define internal ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset) {
  %data = load ptr, ptr %descriptor, align 1
  %byte = getelementptr i8, ptr %data, i64 %offset
  ret ptr %byte
}

define i64 @__ncc_buffer_read8(ptr %descriptor, i64 %offset) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %raw = load i8, ptr %byte, align 1
  %value = zext i8 %raw to i64
  ret i64 %value
}

define i64 @__ncc_buffer_read16(ptr %descriptor, i64 %offset) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %raw = load i16, ptr %byte, align 1
  %value = zext i16 %raw to i64
  ret i64 %value
}

define i64 @__ncc_buffer_read32(ptr %descriptor, i64 %offset) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %raw = load i32, ptr %byte, align 1
  %value = zext i32 %raw to i64
  ret i64 %value
}

define i64 @__ncc_buffer_read64(ptr %descriptor, i64 %offset) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %value = load i64, ptr %byte, align 1
  ret i64 %value
}

define i1 @__ncc_buffer_write8(ptr %descriptor, i64 %offset, i64 %value) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %raw = trunc i64 %value to i8
  store i8 %raw, ptr %byte, align 1
  ret i1 true
}

define i1 @__ncc_buffer_write16(ptr %descriptor, i64 %offset, i64 %value) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %raw = trunc i64 %value to i16
  store i16 %raw, ptr %byte, align 1
  ret i1 true
}

define i1 @__ncc_buffer_write32(ptr %descriptor, i64 %offset, i64 %value) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  %raw = trunc i64 %value to i32
  store i32 %raw, ptr %byte, align 1
  ret i1 true
}

define i1 @__ncc_buffer_write64(ptr %descriptor, i64 %offset, i64 %value) {
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %offset)
  store i64 %value, ptr %byte, align 1
  ret i1 true
}

define i1 @__ncc_buffer_fill(ptr %descriptor, i64 %offset, i64 %bytes, i64 %value) {
  %raw = trunc i64 %value to i8
  br label %loop
loop:
  %index = phi i64 [ 0, %0 ], [ %next, %write ]
  %done = icmp uge i64 %index, %bytes
  br i1 %done, label %exit, label %write
write:
  %position = add i64 %offset, %index
  %byte = call ptr @__ncc_buffer_byte(ptr %descriptor, i64 %position)
  store i8 %raw, ptr %byte, align 1
  %next = add i64 %index, 1
  br label %loop
exit:
  ret i1 true
}

define i1 @__ncc_buffer_copy(ptr %destination, i64 %destination_offset, ptr %source, i64 %source_offset, i64 %bytes) {
  %destination_data = load ptr, ptr %destination, align 1
  %source_data = load ptr, ptr %source, align 1
  %destination_start = getelementptr i8, ptr %destination_data, i64 %destination_offset
  %source_start = getelementptr i8, ptr %source_data, i64 %source_offset
  %destination_address = ptrtoint ptr %destination_start to i64
  %source_address = ptrtoint ptr %source_start to i64
  %source_end = add i64 %source_address, %bytes
  %after_source = icmp uge i64 %destination_address, %source_end
  %before_source = icmp ule i64 %destination_address, %source_address
  %forward = or i1 %after_source, %before_source
  br i1 %forward, label %forward_loop, label %backward_loop
forward_loop:
  %forward_index = phi i64 [ 0, %0 ], [ %forward_next, %forward_write ]
  %forward_done = icmp uge i64 %forward_index, %bytes
  br i1 %forward_done, label %exit, label %forward_write
forward_write:
  %forward_source = getelementptr i8, ptr %source_start, i64 %forward_index
  %forward_destination = getelementptr i8, ptr %destination_start, i64 %forward_index
  %forward_byte = load i8, ptr %forward_source, align 1
  store i8 %forward_byte, ptr %forward_destination, align 1
  %forward_next = add i64 %forward_index, 1
  br label %forward_loop
backward_loop:
  %backward_index = phi i64 [ %bytes, %0 ], [ %backward_next, %backward_write ]
  %backward_done = icmp eq i64 %backward_index, 0
  br i1 %backward_done, label %exit, label %backward_write
backward_write:
  %backward_next = sub i64 %backward_index, 1
  %backward_source = getelementptr i8, ptr %source_start, i64 %backward_next
  %backward_destination = getelementptr i8, ptr %destination_start, i64 %backward_next
  %backward_byte = load i8, ptr %backward_source, align 1
  store i8 %backward_byte, ptr %backward_destination, align 1
  br label %backward_loop
exit:
  ret i1 true
}

define i1 @__ncc_buffer_view(ptr %destination, ptr %source, i64 %offset, i64 %bytes) {
  %source_data = load ptr, ptr %source, align 1
  %source_flags_slot = getelementptr i8, ptr %source, i64 24
  %source_flags = load i64, ptr %source_flags_slot, align 1
  %view_data = getelementptr i8, ptr %source_data, i64 %offset
  %ok = call i1 @__ncc_buffer_at(ptr %destination, ptr %view_data, i64 %bytes, i64 %bytes, i64 %source_flags)
  ret i1 %ok
}

define i1 @__ncc_buffer_clear(ptr %descriptor) {
  %len_slot = getelementptr i8, ptr %descriptor, i64 8
  %cap_slot = getelementptr i8, ptr %descriptor, i64 16
  %flags_slot = getelementptr i8, ptr %descriptor, i64 24
  store ptr null, ptr %descriptor, align 1
  store i64 0, ptr %len_slot, align 1
  store i64 0, ptr %cap_slot, align 1
  store i64 0, ptr %flags_slot, align 1
  ret i1 true
}
"#
}
