pub(crate) struct MemoryViewRuntimeCall {
    pub canonical: &'static str,
    pub material: &'static str,
    pub args: &'static [&'static str],
    pub result: &'static str,
}

pub(crate) const MEMORY_VIEW_MANUAL_CALLS: &[MemoryViewRuntimeCall] = &[
    call(
        "nC.runtime.slice.sliceStride",
        "__ncc_slice_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.slice.sliceAt",
        "__ncc_slice_at",
        &["ptr", "ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.slice.sliceData",
        "__ncc_slice_data",
        &["ptr"],
        "ptr",
    ),
    call(
        "nC.runtime.slice.sliceLen",
        "__ncc_slice_len",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.slice.sliceFlags",
        "__ncc_slice_flags",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.slice.sliceRead8",
        "__ncc_slice_read8",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.slice.sliceWrite8",
        "__ncc_slice_write8",
        &["ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.slice.sliceView",
        "__ncc_slice_view",
        &["ptr", "ptr", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.slice.sliceIsEmpty",
        "__ncc_slice_is_empty",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.slice.sliceClear",
        "__ncc_slice_clear",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.span.spanStride",
        "__ncc_span_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.span.spanSegmentStride",
        "__ncc_span_segment_stride",
        &[],
        "i64",
    ),
    call(
        "nC.runtime.span.spanSegmentAt",
        "__ncc_span_segment_at",
        &["ptr", "i64", "ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.span.spanAt",
        "__ncc_span_at",
        &["ptr", "ptr", "i64", "i64", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.span.spanSegments",
        "__ncc_span_segments",
        &["ptr"],
        "ptr",
    ),
    call(
        "nC.runtime.span.spanCount",
        "__ncc_span_count",
        &["ptr"],
        "i64",
    ),
    call("nC.runtime.span.spanLen", "__ncc_span_len", &["ptr"], "i64"),
    call(
        "nC.runtime.span.spanFlags",
        "__ncc_span_flags",
        &["ptr"],
        "i64",
    ),
    call(
        "nC.runtime.span.spanRead8",
        "__ncc_span_read8",
        &["ptr", "i64"],
        "i64",
    ),
    call(
        "nC.runtime.span.spanContains8",
        "__ncc_span_contains8",
        &["ptr", "i64"],
        "i1",
    ),
    call(
        "nC.runtime.span.spanIsEmpty",
        "__ncc_span_is_empty",
        &["ptr"],
        "i1",
    ),
    call(
        "nC.runtime.span.spanClear",
        "__ncc_span_clear",
        &["ptr"],
        "i1",
    ),
];

const fn call(
    canonical: &'static str,
    material: &'static str,
    args: &'static [&'static str],
    result: &'static str,
) -> MemoryViewRuntimeCall {
    MemoryViewRuntimeCall {
        canonical,
        material,
        args,
        result,
    }
}

pub(crate) fn call_signature(target: &str, arity: usize) -> Option<&'static MemoryViewRuntimeCall> {
    MEMORY_VIEW_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target) && call.args.len() == arity)
}

pub(crate) fn material_symbol(target: &str) -> Option<&'static str> {
    MEMORY_VIEW_MANUAL_CALLS
        .iter()
        .find(|call| call.canonical.eq_ignore_ascii_case(target))
        .map(|call| call.material)
}

pub(crate) fn render_memory_views_manual_runtime() -> &'static str {
    r#"
define i64 @__ncc_slice_stride() { ret i64 24 }
define i1 @__ncc_slice_at(ptr %descriptor, ptr %data, i64 %bytes, i64 %flags) {
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 8
  %flags_slot = getelementptr i8, ptr %descriptor, i64 16
  store ptr %data, ptr %descriptor, align 1
  store i64 %bytes, ptr %bytes_slot, align 1
  store i64 %flags, ptr %flags_slot, align 1
  ret i1 true
}
define ptr @__ncc_slice_data(ptr %descriptor) { %v = load ptr, ptr %descriptor, align 1 ret ptr %v }
define i64 @__ncc_slice_len(ptr %descriptor) { %s = getelementptr i8, ptr %descriptor, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_slice_flags(ptr %descriptor) { %s = getelementptr i8, ptr %descriptor, i64 16 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_slice_read8(ptr %descriptor, i64 %offset) { %data = load ptr, ptr %descriptor, align 1 %p = getelementptr i8, ptr %data, i64 %offset %raw = load i8, ptr %p, align 1 %v = zext i8 %raw to i64 ret i64 %v }
define i1 @__ncc_slice_write8(ptr %descriptor, i64 %offset, i64 %value) { %data = load ptr, ptr %descriptor, align 1 %p = getelementptr i8, ptr %data, i64 %offset %raw = trunc i64 %value to i8 store i8 %raw, ptr %p, align 1 ret i1 true }
define i1 @__ncc_slice_view(ptr %destination, ptr %source, i64 %offset, i64 %bytes) { %data = load ptr, ptr %source, align 1 %flags_slot = getelementptr i8, ptr %source, i64 16 %flags = load i64, ptr %flags_slot, align 1 %view = getelementptr i8, ptr %data, i64 %offset %ok = call i1 @__ncc_slice_at(ptr %destination, ptr %view, i64 %bytes, i64 %flags) ret i1 %ok }
define i1 @__ncc_slice_is_empty(ptr %descriptor) { %bytes = call i64 @__ncc_slice_len(ptr %descriptor) %empty = icmp eq i64 %bytes, 0 ret i1 %empty }
define i1 @__ncc_slice_clear(ptr %descriptor) { %bytes_slot = getelementptr i8, ptr %descriptor, i64 8 %flags_slot = getelementptr i8, ptr %descriptor, i64 16 store ptr null, ptr %descriptor, align 1 store i64 0, ptr %bytes_slot, align 1 store i64 0, ptr %flags_slot, align 1 ret i1 true }

define i64 @__ncc_span_stride() { ret i64 32 }
define i64 @__ncc_span_segment_stride() { ret i64 16 }
define internal ptr @__ncc_span_segment(ptr %segments, i64 %index) { %offset = mul i64 %index, 16 %segment = getelementptr i8, ptr %segments, i64 %offset ret ptr %segment }
define i1 @__ncc_span_segment_at(ptr %segments, i64 %index, ptr %data, i64 %bytes) { %segment = call ptr @__ncc_span_segment(ptr %segments, i64 %index) %bytes_slot = getelementptr i8, ptr %segment, i64 8 store ptr %data, ptr %segment, align 1 store i64 %bytes, ptr %bytes_slot, align 1 ret i1 true }
define i1 @__ncc_span_at(ptr %descriptor, ptr %segments, i64 %count, i64 %bytes, i64 %flags) { %count_slot = getelementptr i8, ptr %descriptor, i64 8 %bytes_slot = getelementptr i8, ptr %descriptor, i64 16 %flags_slot = getelementptr i8, ptr %descriptor, i64 24 store ptr %segments, ptr %descriptor, align 1 store i64 %count, ptr %count_slot, align 1 store i64 %bytes, ptr %bytes_slot, align 1 store i64 %flags, ptr %flags_slot, align 1 ret i1 true }
define ptr @__ncc_span_segments(ptr %descriptor) { %v = load ptr, ptr %descriptor, align 1 ret ptr %v }
define i64 @__ncc_span_count(ptr %descriptor) { %s = getelementptr i8, ptr %descriptor, i64 8 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_span_len(ptr %descriptor) { %s = getelementptr i8, ptr %descriptor, i64 16 %v = load i64, ptr %s, align 1 ret i64 %v }
define i64 @__ncc_span_flags(ptr %descriptor) { %s = getelementptr i8, ptr %descriptor, i64 24 %v = load i64, ptr %s, align 1 ret i64 %v }

define i64 @__ncc_span_read8(ptr %descriptor, i64 %position) {
  %segments = load ptr, ptr %descriptor, align 1
  %count = call i64 @__ncc_span_count(ptr %descriptor)
  br label %loop
loop:
  %index = phi i64 [ 0, %0 ], [ %next, %advance ]
  %offset = phi i64 [ %position, %0 ], [ %remaining, %advance ]
  %segment = call ptr @__ncc_span_segment(ptr %segments, i64 %index)
  %bytes_slot = getelementptr i8, ptr %segment, i64 8
  %bytes = load i64, ptr %bytes_slot, align 1
  %inside = icmp ult i64 %offset, %bytes
  br i1 %inside, label %read, label %advance
advance:
  %remaining = sub i64 %offset, %bytes
  %next = add i64 %index, 1
  %more = icmp ult i64 %next, %count
  br i1 %more, label %loop, label %read
read:
  %data = load ptr, ptr %segment, align 1
  %byte = getelementptr i8, ptr %data, i64 %offset
  %raw = load i8, ptr %byte, align 1
  %value = zext i8 %raw to i64
  ret i64 %value
}

define i1 @__ncc_span_contains8(ptr %descriptor, i64 %value) {
  %segments = load ptr, ptr %descriptor, align 1
  %count = call i64 @__ncc_span_count(ptr %descriptor)
  %wanted = trunc i64 %value to i8
  br label %segment_loop
segment_loop:
  %segment_index = phi i64 [ 0, %0 ], [ %next_segment, %next ]
  %segment_done = icmp uge i64 %segment_index, %count
  br i1 %segment_done, label %miss, label %byte_setup
byte_setup:
  %segment = call ptr @__ncc_span_segment(ptr %segments, i64 %segment_index)
  %data = load ptr, ptr %segment, align 1
  %bytes_slot = getelementptr i8, ptr %segment, i64 8
  %bytes = load i64, ptr %bytes_slot, align 1
  br label %byte_loop
byte_loop:
  %byte_index = phi i64 [ 0, %byte_setup ], [ %next_byte, %advance_byte ]
  %byte_done = icmp uge i64 %byte_index, %bytes
  br i1 %byte_done, label %next, label %compare
compare:
  %byte_ptr = getelementptr i8, ptr %data, i64 %byte_index
  %byte = load i8, ptr %byte_ptr, align 1
  %equal = icmp eq i8 %byte, %wanted
  br i1 %equal, label %found, label %advance_byte
advance_byte:
  %next_byte = add i64 %byte_index, 1
  br label %byte_loop
next:
  %next_segment = add i64 %segment_index, 1
  br label %segment_loop
found:
  ret i1 true
miss:
  ret i1 false
}

define i1 @__ncc_span_is_empty(ptr %descriptor) { %bytes = call i64 @__ncc_span_len(ptr %descriptor) %empty = icmp eq i64 %bytes, 0 ret i1 %empty }
define i1 @__ncc_span_clear(ptr %descriptor) { %count_slot = getelementptr i8, ptr %descriptor, i64 8 %bytes_slot = getelementptr i8, ptr %descriptor, i64 16 %flags_slot = getelementptr i8, ptr %descriptor, i64 24 store ptr null, ptr %descriptor, align 1 store i64 0, ptr %count_slot, align 1 store i64 0, ptr %bytes_slot, align 1 store i64 0, ptr %flags_slot, align 1 ret i1 true }
"#
}
