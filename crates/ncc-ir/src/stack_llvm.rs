pub(crate) fn render_stack_runtime(target: &str) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    let (platform, common) = if target.contains("windows") {
        if is_32_bit(&target) {
            (WINDOWS32, COMMON32)
        } else {
            (WINDOWS64, COMMON64)
        }
    } else if target.contains("darwin") || target.contains("apple") {
        (DARWIN64, COMMON64)
    } else if target.contains("linux") || target.contains("android") {
        let platform = if target.contains("android") {
            if is_32_bit(&target) {
                ANDROID32
            } else {
                ANDROID64
            }
        } else if is_32_bit(&target) {
            LINUX32
        } else {
            LINUX64
        };
        let common = if is_32_bit(&target) {
            COMMON32
        } else {
            COMMON64
        };
        (platform, common)
    } else {
        return Err(format!(
            "stack has no native stack-bound and frame-capture ABI for target '{target}'"
        ));
    };
    Ok(format!("{platform}\n{common}"))
}

fn is_32_bit(target: &str) -> bool {
    target.contains("i386")
        || target.contains("i486")
        || target.contains("i586")
        || target.contains("i686")
        || target.contains("armv7")
        || target.contains("thumbv7")
        || target.contains("riscv32")
}

const WINDOWS64: &str = r#"
declare ptr @VirtualAlloc(ptr, i64, i32, i32)
declare i32 @VirtualFree(ptr, i64, i32)
declare void @GetCurrentThreadStackLimits(ptr, ptr)
declare i16 @RtlCaptureStackBackTrace(i32, i32, ptr, ptr)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call ptr @VirtualAlloc(ptr null, i64 4096, i32 12288, i32 4)
  ret ptr %out
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call i32 @VirtualFree(ptr %memory, i64 0, i32 32768)
  ret i32 %status
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  call void @GetCurrentThreadStackLimits(ptr %low.out, ptr %high.out)
  %current = ptrtoint ptr %marker to i64
  store i64 %current, ptr %current.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %count16 = call i16 @RtlCaptureStackBackTrace(i32 1, i32 64, ptr %frames, ptr null)
  %count = zext i16 %count16 to i64
  ret i64 %count
}
"#;

const WINDOWS32: &str = r#"
declare x86_stdcallcc ptr @VirtualAlloc(ptr, i32, i32, i32)
declare x86_stdcallcc i32 @VirtualFree(ptr, i32, i32)
declare x86_stdcallcc void @GetCurrentThreadStackLimits(ptr, ptr)
declare x86_stdcallcc i16 @RtlCaptureStackBackTrace(i32, i32, ptr, ptr)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call x86_stdcallcc ptr @VirtualAlloc(ptr null, i32 4096, i32 12288, i32 4)
  ret ptr %out
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call x86_stdcallcc i32 @VirtualFree(ptr %memory, i32 0, i32 32768)
  ret i32 %status
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  %low32 = alloca i32, align 4
  %high32 = alloca i32, align 4
  call x86_stdcallcc void @GetCurrentThreadStackLimits(ptr %low32, ptr %high32)
  %low.raw = load i32, ptr %low32, align 4
  %high.raw = load i32, ptr %high32, align 4
  %current.raw = ptrtoint ptr %marker to i32
  %low = zext i32 %low.raw to i64
  %high = zext i32 %high.raw to i64
  %current = zext i32 %current.raw to i64
  store i64 %low, ptr %low.out, align 8
  store i64 %high, ptr %high.out, align 8
  store i64 %current, ptr %current.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %count16 = call x86_stdcallcc i16 @RtlCaptureStackBackTrace(i32 1, i32 64, ptr %frames, ptr null)
  %count = zext i16 %count16 to i64
  ret i64 %count
}
"#;

const LINUX64: &str = r#"
%ncc.stack.pthread.attr = type { [128 x i8] }

declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare i64 @pthread_self()
declare i32 @pthread_getattr_np(i64, ptr)
declare i32 @pthread_attr_getstack(ptr, ptr, ptr)
declare i32 @pthread_attr_destroy(ptr)
declare i32 @backtrace(ptr, i32)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call ptr @mmap(ptr null, i64 4096, i32 3, i32 34, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call i32 @munmap(ptr %memory, i64 4096)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  %attr = alloca %ncc.stack.pthread.attr, align 8
  %base.slot = alloca ptr, align 8
  %size.slot = alloca i64, align 8
  %current = ptrtoint ptr %marker to i64
  store i64 %current, ptr %current.out, align 8
  %thread = call i64 @pthread_self()
  %attr.status = call i32 @pthread_getattr_np(i64 %thread, ptr %attr)
  %attr.ok = icmp eq i32 %attr.status, 0
  br i1 %attr.ok, label %query, label %failure

query:
  %query.status = call i32 @pthread_attr_getstack(ptr %attr, ptr %base.slot, ptr %size.slot)
  %destroy.status = call i32 @pthread_attr_destroy(ptr %attr)
  %query.ok = icmp eq i32 %query.status, 0
  br i1 %query.ok, label %success, label %failure

success:
  %base = load ptr, ptr %base.slot, align 8
  %size = load i64, ptr %size.slot, align 8
  %low = ptrtoint ptr %base to i64
  %high = add i64 %low, %size
  store i64 %low, ptr %low.out, align 8
  store i64 %high, ptr %high.out, align 8
  ret void

failure:
  store i64 %current, ptr %low.out, align 8
  store i64 %current, ptr %high.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %count32 = call i32 @backtrace(ptr %frames, i32 64)
  %enough = icmp sgt i32 %count32, 1
  br i1 %enough, label %shift, label %empty

shift:
  br label %loop
loop:
  %index = phi i32 [ 1, %shift ], [ %next, %copy ]
  %source = getelementptr ptr, ptr %frames, i32 %index
  %destination.index = sub i32 %index, 1
  %destination = getelementptr ptr, ptr %frames, i32 %destination.index
  %frame = load ptr, ptr %source, align 8
  store ptr %frame, ptr %destination, align 8
  %next = add i32 %index, 1
  %done = icmp sge i32 %next, %count32
  br i1 %done, label %result, label %copy
copy:
  br label %loop
result:
  %visible32 = sub i32 %count32, 1
  %visible = zext i32 %visible32 to i64
  ret i64 %visible
empty:
  ret i64 0
}
"#;

const LINUX32: &str = r#"
%ncc.stack.pthread.attr = type { [128 x i8] }

declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare i32 @pthread_self()
declare i32 @pthread_getattr_np(i32, ptr)
declare i32 @pthread_attr_getstack(ptr, ptr, ptr)
declare i32 @pthread_attr_destroy(ptr)
declare i32 @backtrace(ptr, i32)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call ptr @mmap(ptr null, i64 4096, i32 3, i32 34, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i32 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call i32 @munmap(ptr %memory, i64 4096)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  %attr = alloca %ncc.stack.pthread.attr, align 4
  %base.slot = alloca ptr, align 4
  %size.slot = alloca i32, align 4
  %current.raw = ptrtoint ptr %marker to i32
  %current = zext i32 %current.raw to i64
  store i64 %current, ptr %current.out, align 8
  %thread = call i32 @pthread_self()
  %attr.status = call i32 @pthread_getattr_np(i32 %thread, ptr %attr)
  %attr.ok = icmp eq i32 %attr.status, 0
  br i1 %attr.ok, label %query, label %failure

query:
  %query.status = call i32 @pthread_attr_getstack(ptr %attr, ptr %base.slot, ptr %size.slot)
  %destroy.status = call i32 @pthread_attr_destroy(ptr %attr)
  %query.ok = icmp eq i32 %query.status, 0
  br i1 %query.ok, label %success, label %failure

success:
  %base = load ptr, ptr %base.slot, align 4
  %size.raw = load i32, ptr %size.slot, align 4
  %low.raw = ptrtoint ptr %base to i32
  %size = zext i32 %size.raw to i64
  %low = zext i32 %low.raw to i64
  %high = add i64 %low, %size
  store i64 %low, ptr %low.out, align 8
  store i64 %high, ptr %high.out, align 8
  ret void

failure:
  store i64 %current, ptr %low.out, align 8
  store i64 %current, ptr %high.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %count32 = call i32 @backtrace(ptr %frames, i32 64)
  %enough = icmp sgt i32 %count32, 1
  br i1 %enough, label %shift, label %empty

shift:
  br label %loop
loop:
  %index = phi i32 [ 1, %shift ], [ %next, %copy ]
  %source = getelementptr ptr, ptr %frames, i32 %index
  %destination.index = sub i32 %index, 1
  %destination = getelementptr ptr, ptr %frames, i32 %destination.index
  %frame = load ptr, ptr %source, align 4
  store ptr %frame, ptr %destination, align 4
  %next = add i32 %index, 1
  %done = icmp sge i32 %next, %count32
  br i1 %done, label %result, label %copy
copy:
  br label %loop
result:
  %visible32 = sub i32 %count32, 1
  %visible = zext i32 %visible32 to i64
  ret i64 %visible
empty:
  ret i64 0
}
"#;

const DARWIN64: &str = r#"
declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare ptr @pthread_self()
declare ptr @pthread_get_stackaddr_np(ptr)
declare i64 @pthread_get_stacksize_np(ptr)
declare i32 @backtrace(ptr, i32)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call ptr @mmap(ptr null, i64 4096, i32 3, i32 4098, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call i32 @munmap(ptr %memory, i64 4096)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  %thread = call ptr @pthread_self()
  %top = call ptr @pthread_get_stackaddr_np(ptr %thread)
  %size = call i64 @pthread_get_stacksize_np(ptr %thread)
  %high = ptrtoint ptr %top to i64
  %low = sub i64 %high, %size
  %current = ptrtoint ptr %marker to i64
  store i64 %low, ptr %low.out, align 8
  store i64 %high, ptr %high.out, align 8
  store i64 %current, ptr %current.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %count32 = call i32 @backtrace(ptr %frames, i32 64)
  %enough = icmp sgt i32 %count32, 1
  br i1 %enough, label %shift, label %empty

shift:
  br label %loop
loop:
  %index = phi i32 [ 1, %shift ], [ %next, %copy ]
  %source = getelementptr ptr, ptr %frames, i32 %index
  %destination.index = sub i32 %index, 1
  %destination = getelementptr ptr, ptr %frames, i32 %destination.index
  %frame = load ptr, ptr %source, align 8
  store ptr %frame, ptr %destination, align 8
  %next = add i32 %index, 1
  %done = icmp sge i32 %next, %count32
  br i1 %done, label %result, label %copy
copy:
  br label %loop
result:
  %visible32 = sub i32 %count32, 1
  %visible = zext i32 %visible32 to i64
  ret i64 %visible
empty:
  ret i64 0
}
"#;

const ANDROID64: &str = r#"
%ncc.stack.pthread.attr = type { [128 x i8] }

declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare i64 @pthread_self()
declare i32 @pthread_getattr_np(i64, ptr)
declare i32 @pthread_attr_getstack(ptr, ptr, ptr)
declare i32 @pthread_attr_destroy(ptr)
declare ptr @llvm.returnaddress(i32 immarg)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call ptr @mmap(ptr null, i64 4096, i32 3, i32 34, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i64 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call i32 @munmap(ptr %memory, i64 4096)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  %attr = alloca %ncc.stack.pthread.attr, align 8
  %base.slot = alloca ptr, align 8
  %size.slot = alloca i64, align 8
  %current = ptrtoint ptr %marker to i64
  store i64 %current, ptr %current.out, align 8
  %thread = call i64 @pthread_self()
  %attr.status = call i32 @pthread_getattr_np(i64 %thread, ptr %attr)
  %attr.ok = icmp eq i32 %attr.status, 0
  br i1 %attr.ok, label %query, label %failure
query:
  %query.status = call i32 @pthread_attr_getstack(ptr %attr, ptr %base.slot, ptr %size.slot)
  %destroy.status = call i32 @pthread_attr_destroy(ptr %attr)
  %query.ok = icmp eq i32 %query.status, 0
  br i1 %query.ok, label %success, label %failure
success:
  %base = load ptr, ptr %base.slot, align 8
  %size = load i64, ptr %size.slot, align 8
  %low = ptrtoint ptr %base to i64
  %high = add i64 %low, %size
  store i64 %low, ptr %low.out, align 8
  store i64 %high, ptr %high.out, align 8
  ret void
failure:
  store i64 %current, ptr %low.out, align 8
  store i64 %current, ptr %high.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %frame = call ptr @llvm.returnaddress(i32 0)
  store ptr %frame, ptr %frames, align 8
  ret i64 1
}
"#;

const ANDROID32: &str = r#"
%ncc.stack.pthread.attr = type { [128 x i8] }

declare ptr @mmap(ptr, i64, i32, i32, i32, i64)
declare i32 @munmap(ptr, i64)
declare i32 @pthread_self()
declare i32 @pthread_getattr_np(i32, ptr)
declare i32 @pthread_attr_getstack(ptr, ptr, ptr)
declare i32 @pthread_attr_destroy(ptr)
declare ptr @llvm.returnaddress(i32 immarg)

define internal ptr @__ncc_stack_alloc() {
entry:
  %out = call ptr @mmap(ptr null, i64 4096, i32 3, i32 34, i32 -1, i64 0)
  %failed = icmp eq ptr %out, inttoptr (i32 -1 to ptr)
  %result = select i1 %failed, ptr null, ptr %out
  ret ptr %result
}

define internal i32 @__ncc_stack_free(ptr %memory) {
entry:
  %status = call i32 @munmap(ptr %memory, i64 4096)
  %ok = icmp eq i32 %status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
}

define internal void @__ncc_stack_bounds(ptr %low.out, ptr %high.out, ptr %current.out) {
entry:
  %marker = alloca i8, align 1
  %attr = alloca %ncc.stack.pthread.attr, align 4
  %base.slot = alloca ptr, align 4
  %size.slot = alloca i32, align 4
  %current.raw = ptrtoint ptr %marker to i32
  %current = zext i32 %current.raw to i64
  store i64 %current, ptr %current.out, align 8
  %thread = call i32 @pthread_self()
  %attr.status = call i32 @pthread_getattr_np(i32 %thread, ptr %attr)
  %attr.ok = icmp eq i32 %attr.status, 0
  br i1 %attr.ok, label %query, label %failure
query:
  %query.status = call i32 @pthread_attr_getstack(ptr %attr, ptr %base.slot, ptr %size.slot)
  %destroy.status = call i32 @pthread_attr_destroy(ptr %attr)
  %query.ok = icmp eq i32 %query.status, 0
  br i1 %query.ok, label %success, label %failure
success:
  %base = load ptr, ptr %base.slot, align 4
  %size.raw = load i32, ptr %size.slot, align 4
  %low.raw = ptrtoint ptr %base to i32
  %size = zext i32 %size.raw to i64
  %low = zext i32 %low.raw to i64
  %high = add i64 %low, %size
  store i64 %low, ptr %low.out, align 8
  store i64 %high, ptr %high.out, align 8
  ret void
failure:
  store i64 %current, ptr %low.out, align 8
  store i64 %current, ptr %high.out, align 8
  ret void
}

define internal i64 @__ncc_stack_capture(ptr %frames) {
entry:
  %frame = call ptr @llvm.returnaddress(i32 0)
  store ptr %frame, ptr %frames, align 4
  ret i64 1
}
"#;

const COMMON64: &str = r#"
%ncc.stack.trace = type { i64, i64, [64 x ptr] }

define internal i32 @__ncc_stack_valid_bounds(i64 %low, i64 %high, i64 %current) {
entry:
  %ordered = icmp ugt i64 %high, %low
  %above = icmp uge i64 %current, %low
  %below = icmp ule i64 %current, %high
  %inside.0 = and i1 %ordered, %above
  %inside = and i1 %inside.0, %below
  %result = zext i1 %inside to i32
  ret i32 %result
}

define i64 @ncc_rt_stack_depth() {
entry:
  %low = alloca i64, align 8
  %high = alloca i64, align 8
  %current = alloca i64, align 8
  call void @__ncc_stack_bounds(ptr %low, ptr %high, ptr %current)
  %low.value = load i64, ptr %low, align 8
  %high.value = load i64, ptr %high, align 8
  %current.value = load i64, ptr %current, align 8
  %valid32 = call i32 @__ncc_stack_valid_bounds(i64 %low.value, i64 %high.value, i64 %current.value)
  %valid = icmp ne i32 %valid32, 0
  %depth = sub i64 %high.value, %current.value
  %result = select i1 %valid, i64 %depth, i64 0
  ret i64 %result
}

define i64 @ncc_rt_stack_limit() {
entry:
  %low = alloca i64, align 8
  %high = alloca i64, align 8
  %current = alloca i64, align 8
  call void @__ncc_stack_bounds(ptr %low, ptr %high, ptr %current)
  %low.value = load i64, ptr %low, align 8
  %high.value = load i64, ptr %high, align 8
  %ordered = icmp ugt i64 %high.value, %low.value
  %limit = sub i64 %high.value, %low.value
  %result = select i1 %ordered, i64 %limit, i64 0
  ret i64 %result
}

define i64 @ncc_rt_stack_remaining() {
entry:
  %low = alloca i64, align 8
  %high = alloca i64, align 8
  %current = alloca i64, align 8
  call void @__ncc_stack_bounds(ptr %low, ptr %high, ptr %current)
  %low.value = load i64, ptr %low, align 8
  %high.value = load i64, ptr %high, align 8
  %current.value = load i64, ptr %current, align 8
  %valid32 = call i32 @__ncc_stack_valid_bounds(i64 %low.value, i64 %high.value, i64 %current.value)
  %valid = icmp ne i32 %valid32, 0
  %remaining = sub i64 %current.value, %low.value
  %result = select i1 %valid, i64 %remaining, i64 0
  ret i64 %result
}

define i64 @ncc_rt_stack_trace() {
entry:
  %handle = call ptr @__ncc_stack_alloc()
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %capture, label %failure
capture:
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %len.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 1
  %frames.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 2
  store i64 5639442779735280433, ptr %magic.slot, align 8
  %count = call i64 @__ncc_stack_capture(ptr %frames.slot)
  %bounded = icmp ule i64 %count, 64
  %len = select i1 %bounded, i64 %count, i64 64
  store i64 %len, ptr %len.slot, align 8
  %raw = ptrtoint ptr %handle to i64
  ret i64 %raw
failure:
  ret i64 0
}

define i64 @ncc_rt_stack_trace_len(i64 %raw) {
entry:
  %nonzero = icmp ne i64 %raw, 0
  br i1 %nonzero, label %validate, label %failure
validate:
  %handle = inttoptr i64 %raw to ptr
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %magic = load i64, ptr %magic.slot, align 8
  %valid = icmp eq i64 %magic, 5639442779735280433
  br i1 %valid, label %success, label %failure
success:
  %len.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  ret i64 %len
failure:
  ret i64 0
}

define i64 @ncc_rt_stack_trace_frame(i64 %raw, i64 %index) {
entry:
  %nonzero = icmp ne i64 %raw, 0
  br i1 %nonzero, label %validate, label %invalid.handle
validate:
  %handle = inttoptr i64 %raw to ptr
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %magic = load i64, ptr %magic.slot, align 8
  %valid = icmp eq i64 %magic, 5639442779735280433
  br i1 %valid, label %bounds, label %invalid.handle
bounds:
  %len.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %nonnegative = icmp sge i64 %index, 0
  %inside = icmp ult i64 %index, %len
  %in.bounds = and i1 %nonnegative, %inside
  br i1 %in.bounds, label %success, label %invalid.index
success:
  %frame.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 2, i64 %index
  %frame = load ptr, ptr %frame.slot, align 8
  %address = ptrtoint ptr %frame to i64
  %payload = shl i64 %address, 1
  %tagged = or i64 %payload, 1
  ret i64 %tagged
invalid.handle:
  ret i64 2
invalid.index:
  ret i64 6
}

define i32 @ncc_rt_stack_trace_release(i64 %raw) {
entry:
  %nonzero = icmp ne i64 %raw, 0
  br i1 %nonzero, label %validate, label %failure
validate:
  %handle = inttoptr i64 %raw to ptr
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %magic = load i64, ptr %magic.slot, align 8
  %valid = icmp eq i64 %magic, 5639442779735280433
  br i1 %valid, label %release, label %failure
release:
  store i64 0, ptr %magic.slot, align 8
  %status = call i32 @__ncc_stack_free(ptr %handle)
  ret i32 %status
failure:
  ret i32 0
}
"#;

const COMMON32: &str = r#"
%ncc.stack.trace = type { i64, i64, [64 x ptr] }

define internal i32 @__ncc_stack_valid_bounds(i64 %low, i64 %high, i64 %current) {
entry:
  %ordered = icmp ugt i64 %high, %low
  %above = icmp uge i64 %current, %low
  %below = icmp ule i64 %current, %high
  %inside.0 = and i1 %ordered, %above
  %inside = and i1 %inside.0, %below
  %result = zext i1 %inside to i32
  ret i32 %result
}

define i64 @ncc_rt_stack_depth() {
entry:
  %low = alloca i64, align 8
  %high = alloca i64, align 8
  %current = alloca i64, align 8
  call void @__ncc_stack_bounds(ptr %low, ptr %high, ptr %current)
  %low.value = load i64, ptr %low, align 8
  %high.value = load i64, ptr %high, align 8
  %current.value = load i64, ptr %current, align 8
  %valid32 = call i32 @__ncc_stack_valid_bounds(i64 %low.value, i64 %high.value, i64 %current.value)
  %valid = icmp ne i32 %valid32, 0
  %depth = sub i64 %high.value, %current.value
  %result = select i1 %valid, i64 %depth, i64 0
  ret i64 %result
}

define i64 @ncc_rt_stack_limit() {
entry:
  %low = alloca i64, align 8
  %high = alloca i64, align 8
  %current = alloca i64, align 8
  call void @__ncc_stack_bounds(ptr %low, ptr %high, ptr %current)
  %low.value = load i64, ptr %low, align 8
  %high.value = load i64, ptr %high, align 8
  %ordered = icmp ugt i64 %high.value, %low.value
  %limit = sub i64 %high.value, %low.value
  %result = select i1 %ordered, i64 %limit, i64 0
  ret i64 %result
}

define i64 @ncc_rt_stack_remaining() {
entry:
  %low = alloca i64, align 8
  %high = alloca i64, align 8
  %current = alloca i64, align 8
  call void @__ncc_stack_bounds(ptr %low, ptr %high, ptr %current)
  %low.value = load i64, ptr %low, align 8
  %high.value = load i64, ptr %high, align 8
  %current.value = load i64, ptr %current, align 8
  %valid32 = call i32 @__ncc_stack_valid_bounds(i64 %low.value, i64 %high.value, i64 %current.value)
  %valid = icmp ne i32 %valid32, 0
  %remaining = sub i64 %current.value, %low.value
  %result = select i1 %valid, i64 %remaining, i64 0
  ret i64 %result
}

define i64 @ncc_rt_stack_trace() {
entry:
  %handle = call ptr @__ncc_stack_alloc()
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %capture, label %failure
capture:
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %len.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 1
  %frames.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 2
  store i64 5639442779735280433, ptr %magic.slot, align 8
  %count = call i64 @__ncc_stack_capture(ptr %frames.slot)
  %bounded = icmp ule i64 %count, 64
  %len = select i1 %bounded, i64 %count, i64 64
  store i64 %len, ptr %len.slot, align 8
  %raw32 = ptrtoint ptr %handle to i32
  %raw = zext i32 %raw32 to i64
  ret i64 %raw
failure:
  ret i64 0
}

define internal ptr @__ncc_stack_handle_from_raw(i64 %raw) {
entry:
  %raw32 = trunc i64 %raw to i32
  %handle = inttoptr i32 %raw32 to ptr
  ret ptr %handle
}

define i64 @ncc_rt_stack_trace_len(i64 %raw) {
entry:
  %nonzero = icmp ne i64 %raw, 0
  br i1 %nonzero, label %validate, label %failure
validate:
  %handle = call ptr @__ncc_stack_handle_from_raw(i64 %raw)
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %magic = load i64, ptr %magic.slot, align 8
  %valid = icmp eq i64 %magic, 5639442779735280433
  br i1 %valid, label %success, label %failure
success:
  %len.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  ret i64 %len
failure:
  ret i64 0
}

define i64 @ncc_rt_stack_trace_frame(i64 %raw, i64 %index) {
entry:
  %nonzero = icmp ne i64 %raw, 0
  br i1 %nonzero, label %validate, label %invalid.handle
validate:
  %handle = call ptr @__ncc_stack_handle_from_raw(i64 %raw)
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %magic = load i64, ptr %magic.slot, align 8
  %valid = icmp eq i64 %magic, 5639442779735280433
  br i1 %valid, label %bounds, label %invalid.handle
bounds:
  %len.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 1
  %len = load i64, ptr %len.slot, align 8
  %nonnegative = icmp sge i64 %index, 0
  %inside = icmp ult i64 %index, %len
  %in.bounds = and i1 %nonnegative, %inside
  br i1 %in.bounds, label %success, label %invalid.index
success:
  %frame.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 2, i64 %index
  %frame = load ptr, ptr %frame.slot, align 4
  %address32 = ptrtoint ptr %frame to i32
  %address = zext i32 %address32 to i64
  %payload = shl i64 %address, 1
  %tagged = or i64 %payload, 1
  ret i64 %tagged
invalid.handle:
  ret i64 2
invalid.index:
  ret i64 6
}

define i32 @ncc_rt_stack_trace_release(i64 %raw) {
entry:
  %nonzero = icmp ne i64 %raw, 0
  br i1 %nonzero, label %validate, label %failure
validate:
  %handle = call ptr @__ncc_stack_handle_from_raw(i64 %raw)
  %magic.slot = getelementptr %ncc.stack.trace, ptr %handle, i32 0, i32 0
  %magic = load i64, ptr %magic.slot, align 8
  %valid = icmp eq i64 %magic, 5639442779735280433
  br i1 %valid, label %release, label %failure
release:
  store i64 0, ptr %magic.slot, align 8
  %status = call i32 @__ncc_stack_free(ptr %handle)
  ret i32 %status
failure:
  ret i32 0
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_uses_stack_bounds_backtrace_and_page_owned_handle() {
        let llvm = render_stack_runtime("x86_64-pc-windows-msvc").expect("windows stack");
        assert!(llvm.contains("@GetCurrentThreadStackLimits"));
        assert!(llvm.contains("@RtlCaptureStackBackTrace"));
        assert!(llvm.contains("@VirtualAlloc"));
        assert!(llvm.contains("define i64 @ncc_rt_stack_trace()"));
        assert!(!llvm.contains("@malloc"));
        assert!(!llvm.contains("@free"));
    }

    #[test]
    fn windows_i386_uses_the_material_stdcall_os_abi() {
        let llvm = render_stack_runtime("i686-pc-windows-msvc").expect("win32 stack");
        assert!(llvm.contains("declare x86_stdcallcc ptr @VirtualAlloc"));
        assert!(llvm.contains("call x86_stdcallcc ptr @VirtualAlloc"));
        assert!(llvm.contains("declare x86_stdcallcc i32 @VirtualFree"));
        assert!(llvm.contains("call x86_stdcallcc i32 @VirtualFree"));
        assert!(llvm.contains("call x86_stdcallcc void @GetCurrentThreadStackLimits"));
        assert!(llvm.contains("call x86_stdcallcc i16 @RtlCaptureStackBackTrace"));
    }

    #[test]
    fn linux_width_changes_pthread_and_mmap_abi() {
        let elf64 = render_stack_runtime("x86_64-unknown-linux-gnu").expect("linux64");
        let elf32 = render_stack_runtime("i386-unknown-linux-gnu").expect("linux32");
        assert!(elf64.contains("declare i64 @pthread_self()"));
        assert!(elf64.contains("@mmap(ptr, i64"));
        assert!(elf32.contains("declare i32 @pthread_self()"));
        assert!(elf32.contains("@mmap(ptr, i64"));
        assert!(elf32.contains("ptrtoint ptr %handle to i32"));
    }

    #[test]
    fn darwin_uses_native_pthread_stack_contract() {
        let llvm = render_stack_runtime("aarch64-apple-darwin").expect("darwin stack");
        assert!(llvm.contains("@pthread_get_stackaddr_np"));
        assert!(llvm.contains("@pthread_get_stacksize_np"));
        assert!(llvm.contains("i32 4098"));
    }

    #[test]
    fn unsupported_targets_are_rejected_explicitly() {
        let err = render_stack_runtime("wasm32-wasi").expect_err("wasi stack must be explicit");
        assert!(err.contains("no native stack-bound"));
    }
}
