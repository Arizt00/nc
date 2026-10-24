pub(crate) fn render_process_args_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if lower.contains("windows") && (lower.contains("x86_64") || lower.contains("aarch64")) {
        return Ok(WIN64_PROCESS_ARGS_RUNTIME.to_string());
    }
    let pointer_bytes = if lower.contains("i386")
        || lower.contains("i486")
        || lower.contains("i586")
        || lower.contains("i686")
        || lower.contains("armv7")
        || lower.contains("thumbv7")
        || lower.contains("riscv32")
    {
        "4"
    } else {
        "8"
    };
    if lower.contains("linux") || lower.contains("android") {
        return Ok(
            format!("{POSIX_PROCESS_ARGS_RUNTIME}\n{LINUX_PROCESS_ARGS_RUNTIME}")
                .replace("@POINTER_BYTES@", pointer_bytes),
        );
    }
    if lower.contains("darwin") || lower.contains("apple") {
        return Ok(
            format!("{POSIX_PROCESS_ARGS_RUNTIME}\n{DARWIN_PROCESS_ARGS_RUNTIME}")
                .replace("@POINTER_BYTES@", pointer_bytes),
        );
    }
    Err(format!(
        "native raw process-args backend has no direct OS ABI for target '{target}'"
    ))
}

const POSIX_PROCESS_ARGS_RUNTIME: &str = r#"
define internal i64 @__ncc_args_strlen(ptr %text) {
entry:
  %missing = icmp eq ptr %text, null
  br i1 %missing, label %empty, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %more ]
  %slot = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %slot, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %finish, label %more
more:
  %next = add i64 %index, 1
  br label %loop
finish:
  ret i64 %index
empty:
  ret i64 0
}

define internal ptr @__ncc_args_error_descriptor(i64 %code) {
entry:
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 32, i64 8)
  %ok = icmp ne ptr %descriptor, null
  br i1 %ok, label %initialize, label %failed
initialize:
  store i64 0, ptr %descriptor, align 8
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  store ptr null, ptr %data_slot, align 8
  store i64 0, ptr %bytes_slot, align 8
  store i64 %code, ptr %error_slot, align 8
  ret ptr %descriptor
failed:
  ret ptr null
}

define internal ptr @__ncc_args_clone_vector(i64 %count, ptr %source) {
entry:
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 32, i64 8)
  %descriptor_ok = icmp ne ptr %descriptor, null
  br i1 %descriptor_ok, label %allocate_vector, label %failed
allocate_vector:
  %raw_vector_bytes = mul i64 %count, @POINTER_BYTES@
  %has_values = icmp ugt i64 %count, 0
  %vector_bytes = select i1 %has_values, i64 %raw_vector_bytes, i64 @POINTER_BYTES@
  %vector = call ptr @__ncc_raw_aligned_alloc(i64 %vector_bytes, i64 8)
  %vector_ok = icmp ne ptr %vector, null
  br i1 %vector_ok, label %copy_loop, label %descriptor_error
copy_loop:
  %index = phi i64 [ 0, %allocate_vector ], [ %next, %stored ]
  %total = phi i64 [ 0, %allocate_vector ], [ %total_next, %stored ]
  %more = icmp ult i64 %index, %count
  br i1 %more, label %copy_one, label %success
copy_one:
  %source_slot = getelementptr ptr, ptr %source, i64 %index
  %source_text = load ptr, ptr %source_slot, align 1
  %length = call i64 @__ncc_args_strlen(ptr %source_text)
  %bytes = add i64 %length, 1
  %text = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %text_ok = icmp ne ptr %text, null
  br i1 %text_ok, label %copy_bytes, label %cleanup_start
copy_bytes:
  br label %byte_loop
byte_loop:
  %byte_index = phi i64 [ 0, %copy_bytes ], [ %byte_next, %byte_more ]
  %byte_more_test = icmp ult i64 %byte_index, %bytes
  br i1 %byte_more_test, label %byte_more, label %stored
byte_more:
  %source_byte_slot = getelementptr i8, ptr %source_text, i64 %byte_index
  %source_byte = load i8, ptr %source_byte_slot, align 1
  %target_byte_slot = getelementptr i8, ptr %text, i64 %byte_index
  store i8 %source_byte, ptr %target_byte_slot, align 1
  %byte_next = add i64 %byte_index, 1
  br label %byte_loop
stored:
  %target_slot = getelementptr ptr, ptr %vector, i64 %index
  store ptr %text, ptr %target_slot, align 1
  %next = add i64 %index, 1
  %total_next = add i64 %total, %bytes
  br label %copy_loop
success:
  store i64 %count, ptr %descriptor, align 8
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  store ptr %vector, ptr %data_slot, align 8
  store i64 %total, ptr %bytes_slot, align 8
  store i64 0, ptr %error_slot, align 8
  ret ptr %descriptor
cleanup_start:
  br label %cleanup_loop
cleanup_loop:
  %cleanup_index = phi i64 [ 0, %cleanup_start ], [ %cleanup_next, %cleanup_one ]
  %cleanup_more = icmp ult i64 %cleanup_index, %index
  br i1 %cleanup_more, label %cleanup_one, label %cleanup_vector
cleanup_one:
  %cleanup_slot = getelementptr ptr, ptr %vector, i64 %cleanup_index
  %cleanup_text = load ptr, ptr %cleanup_slot, align 1
  %released_text = call i1 @__ncc_raw_aligned_free(ptr %cleanup_text)
  %cleanup_next = add i64 %cleanup_index, 1
  br label %cleanup_loop
cleanup_vector:
  %released_vector = call i1 @__ncc_raw_aligned_free(ptr %vector)
  br label %descriptor_error
descriptor_error:
  %released_descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  br label %failed
failed:
  ret ptr null
}

define i64 @__ncc_args_count(ptr %descriptor) {
entry:
  %missing = icmp eq ptr %descriptor, null
  br i1 %missing, label %empty, label %load
load:
  %count = load i64, ptr %descriptor, align 8
  ret i64 %count
empty:
  ret i64 0
}

define ptr @__ncc_args_at(ptr %descriptor, i64 %index) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  %argument_slot = getelementptr ptr, ptr %data, i64 %index
  %argument = load ptr, ptr %argument_slot, align 1
  ret ptr %argument
}

define ptr @__ncc_args_data(ptr %descriptor) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  ret ptr %data
}

define i64 @__ncc_args_bytes(ptr %descriptor) {
entry:
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %bytes = load i64, ptr %bytes_slot, align 8
  ret i64 %bytes
}

define i64 @__ncc_args_error(ptr %descriptor) {
entry:
  %missing = icmp eq ptr %descriptor, null
  br i1 %missing, label %failed, label %load
load:
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  %error = load i64, ptr %error_slot, align 8
  ret i64 %error
failed:
  ret i64 12
}

define i1 @__ncc_args_release(ptr %descriptor) {
entry:
  %missing = icmp eq ptr %descriptor, null
  br i1 %missing, label %done, label %load
load:
  %count = load i64, ptr %descriptor, align 8
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  br label %loop
loop:
  %index = phi i64 [ 0, %load ], [ %next, %release_one ]
  %more = icmp ult i64 %index, %count
  br i1 %more, label %release_one, label %release_vector
release_one:
  %slot = getelementptr ptr, ptr %data, i64 %index
  %text = load ptr, ptr %slot, align 1
  %released_text = call i1 @__ncc_raw_aligned_free(ptr %text)
  %next = add i64 %index, 1
  br label %loop
release_vector:
  %released_data = call i1 @__ncc_raw_aligned_free(ptr %data)
  %released_descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  br label %done
done:
  %result = phi i1 [ true, %entry ], [ %released_descriptor, %release_vector ]
  ret i1 %result
}
"#;

const LINUX_PROCESS_ARGS_RUNTIME: &str = r#"
@__ncc_args_proc_path = private unnamed_addr constant [19 x i8] c"/proc/self/cmdline\00"
declare i32 @open(ptr, i32, ...)
declare i64 @read(i32, ptr, i64)
declare i32 @close(i32)

define ptr @__ncc_args_open() {
entry:
  %fd = call i32 (ptr, i32, ...) @open(ptr @__ncc_args_proc_path, i32 0)
  %opened = icmp sge i32 %fd, 0
  br i1 %opened, label %allocate, label %open_error
allocate:
  %initial = call ptr @__ncc_raw_aligned_alloc(i64 4096, i64 8)
  %allocated = icmp ne ptr %initial, null
  br i1 %allocated, label %read_loop, label %allocation_error
read_loop:
  %buffer = phi ptr [ %initial, %allocate ], [ %grown, %grow_done ], [ %buffer, %read_more ]
  %capacity = phi i64 [ 4096, %allocate ], [ %next_capacity, %grow_done ], [ %capacity, %read_more ]
  %used = phi i64 [ 0, %allocate ], [ %used, %grow_done ], [ %used_next, %read_more ]
  %available = sub i64 %capacity, %used
  %target = getelementptr i8, ptr %buffer, i64 %used
  %amount = call i64 @read(i32 %fd, ptr %target, i64 %available)
  %failed = icmp slt i64 %amount, 0
  br i1 %failed, label %read_error, label %read_result
read_result:
  %eof = icmp eq i64 %amount, 0
  br i1 %eof, label %parse, label %read_more
read_more:
  %used_next = add i64 %used, %amount
  %full = icmp eq i64 %used_next, %capacity
  br i1 %full, label %grow, label %read_loop
grow:
  %next_capacity = shl i64 %capacity, 1
  %grown = call ptr @__ncc_raw_aligned_realloc(ptr %buffer, i64 %capacity, i64 %next_capacity, i64 8)
  %grow_ok = icmp ne ptr %grown, null
  br i1 %grow_ok, label %grow_done, label %grow_error
grow_done:
  br label %read_loop
parse:
  %closed = call i32 @close(i32 %fd)
  br label %count_loop
count_loop:
  %count_at = phi i64 [ 0, %parse ], [ %count_next_at, %count_step ]
  %count = phi i64 [ 0, %parse ], [ %count_after, %count_step ]
  %count_more = icmp ult i64 %count_at, %used
  br i1 %count_more, label %count_byte, label %allocate_vector
count_byte:
  %count_slot = getelementptr i8, ptr %buffer, i64 %count_at
  %count_value = load i8, ptr %count_slot, align 1
  %separator = icmp eq i8 %count_value, 0
  %count_increment = select i1 %separator, i64 1, i64 0
  %count_total = add i64 %count, %count_increment
  br label %count_step
count_step:
  %count_after = phi i64 [ %count_total, %count_byte ]
  %count_next_at = add i64 %count_at, 1
  br label %count_loop
allocate_vector:
  %raw_vector_bytes = mul i64 %count, @POINTER_BYTES@
  %has_values = icmp ugt i64 %count, 0
  %vector_bytes = select i1 %has_values, i64 %raw_vector_bytes, i64 @POINTER_BYTES@
  %vector = call ptr @__ncc_raw_aligned_alloc(i64 %vector_bytes, i64 8)
  %vector_ok = icmp ne ptr %vector, null
  br i1 %vector_ok, label %vector_loop, label %vector_error
vector_loop:
  %byte_at = phi i64 [ 0, %allocate_vector ], [ %byte_next, %vector_step ]
  %arg_at = phi i64 [ 0, %allocate_vector ], [ %arg_next, %vector_step ]
  %start = phi ptr [ %buffer, %allocate_vector ], [ %start_next, %vector_step ]
  %vector_more = icmp ult i64 %byte_at, %used
  br i1 %vector_more, label %vector_byte, label %clone
vector_byte:
  %byte_slot = getelementptr i8, ptr %buffer, i64 %byte_at
  %byte = load i8, ptr %byte_slot, align 1
  %at_separator = icmp eq i8 %byte, 0
  br i1 %at_separator, label %store_arg, label %vector_continue
store_arg:
  %arg_slot = getelementptr ptr, ptr %vector, i64 %arg_at
  store ptr %start, ptr %arg_slot, align 1
  %arg_stored = add i64 %arg_at, 1
  %after = getelementptr i8, ptr %byte_slot, i64 1
  br label %vector_step
vector_continue:
  br label %vector_step
vector_step:
  %arg_next = phi i64 [ %arg_stored, %store_arg ], [ %arg_at, %vector_continue ]
  %start_next = phi ptr [ %after, %store_arg ], [ %start, %vector_continue ]
  %byte_next = add i64 %byte_at, 1
  br label %vector_loop
clone:
  %descriptor = call ptr @__ncc_args_clone_vector(i64 %count, ptr %vector)
  %released_vector = call i1 @__ncc_raw_aligned_free(ptr %vector)
  %released_buffer = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  ret ptr %descriptor
read_error:
  %closed_read_error = call i32 @close(i32 %fd)
  %released_read_buffer = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %read_failure = call ptr @__ncc_args_error_descriptor(i64 5)
  ret ptr %read_failure
grow_error:
  %closed_grow_error = call i32 @close(i32 %fd)
  %released_grow_buffer = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %grow_failure = call ptr @__ncc_args_error_descriptor(i64 12)
  ret ptr %grow_failure
vector_error:
  %released_vector_buffer = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %vector_failure = call ptr @__ncc_args_error_descriptor(i64 12)
  ret ptr %vector_failure
allocation_error:
  %closed_allocation_error = call i32 @close(i32 %fd)
  %allocation_failure = call ptr @__ncc_args_error_descriptor(i64 12)
  ret ptr %allocation_failure
open_error:
  %open_failure = call ptr @__ncc_args_error_descriptor(i64 2)
  ret ptr %open_failure
}
"#;

const DARWIN_PROCESS_ARGS_RUNTIME: &str = r#"
declare ptr @_NSGetArgc()
declare ptr @_NSGetArgv()

define ptr @__ncc_args_open() {
entry:
  %argc_address = call ptr @_NSGetArgc()
  %argv_address = call ptr @_NSGetArgv()
  %argc32 = load i32, ptr %argc_address, align 4
  %argc = zext i32 %argc32 to i64
  %argv = load ptr, ptr %argv_address, align 1
  %descriptor = call ptr @__ncc_args_clone_vector(i64 %argc, ptr %argv)
  ret ptr %descriptor
}
"#;

const WIN64_PROCESS_ARGS_RUNTIME: &str = r#"
declare ptr @GetCommandLineW()
declare ptr @CommandLineToArgvW(ptr, ptr)
declare i32 @WideCharToMultiByte(i32, i32, ptr, i32, ptr, i32, ptr, ptr)
declare ptr @LocalFree(ptr)

define internal ptr @__ncc_args_set_error(ptr %descriptor, i64 %error) {
entry:
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  store i64 %error, ptr %error_slot, align 8
  ret ptr %descriptor
}

define ptr @__ncc_args_open() {
entry:
  %argc_slot = alloca i32, align 4
  %allocated_slot = alloca i64, align 8
  %failure_slot = alloca i64, align 8
  store i32 0, ptr %argc_slot, align 4
  store i64 0, ptr %allocated_slot, align 8
  store i64 0, ptr %failure_slot, align 8
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 32, i64 8)
  %descriptor_ok = icmp ne ptr %descriptor, null
  br i1 %descriptor_ok, label %initialize, label %out_of_memory
initialize:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  store i64 0, ptr %descriptor, align 8
  store ptr null, ptr %data_slot, align 8
  store i64 0, ptr %bytes_slot, align 8
  store i64 0, ptr %error_slot, align 8
  %command_line = call ptr @GetCommandLineW()
  %command_ok = icmp ne ptr %command_line, null
  br i1 %command_ok, label %split, label %command_error
split:
  %wide_vector = call ptr @CommandLineToArgvW(ptr %command_line, ptr %argc_slot)
  %wide_ok = icmp ne ptr %wide_vector, null
  br i1 %wide_ok, label %validate_count, label %split_error
validate_count:
  %argc32 = load i32, ptr %argc_slot, align 4
  %argc_valid = icmp sge i32 %argc32, 0
  br i1 %argc_valid, label %allocate_vector, label %invalid_count
allocate_vector:
  %argc = zext i32 %argc32 to i64
  %vector_bytes_raw = mul i64 %argc, 8
  %has_args = icmp ugt i64 %argc, 0
  %vector_bytes = select i1 %has_args, i64 %vector_bytes_raw, i64 8
  %vector = call ptr @__ncc_raw_aligned_alloc(i64 %vector_bytes, i64 8)
  %vector_ok = icmp ne ptr %vector, null
  br i1 %vector_ok, label %convert_loop, label %vector_error
convert_loop:
  %index = phi i64 [ 0, %allocate_vector ], [ %next, %store_argument ]
  %total_bytes = phi i64 [ 0, %allocate_vector ], [ %total_next, %store_argument ]
  %more = icmp ult i64 %index, %argc
  br i1 %more, label %measure_argument, label %success
measure_argument:
  %wide_slot = getelementptr ptr, ptr %wide_vector, i64 %index
  %wide_argument = load ptr, ptr %wide_slot, align 8
  %required32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %wide_argument, i32 -1, ptr null, i32 0, ptr null, ptr null)
  %required_ok = icmp sgt i32 %required32, 0
  br i1 %required_ok, label %allocate_argument, label %measure_error
allocate_argument:
  %required = zext i32 %required32 to i64
  %argument = call ptr @__ncc_raw_aligned_alloc(i64 %required, i64 8)
  %argument_ok = icmp ne ptr %argument, null
  br i1 %argument_ok, label %convert_argument, label %argument_alloc_error
convert_argument:
  %written32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %wide_argument, i32 -1, ptr %argument, i32 %required32, ptr null, ptr null)
  %converted = icmp eq i32 %written32, %required32
  br i1 %converted, label %store_argument, label %conversion_error
store_argument:
  %argument_slot = getelementptr ptr, ptr %vector, i64 %index
  store ptr %argument, ptr %argument_slot, align 8
  %next = add i64 %index, 1
  %total_next = add i64 %total_bytes, %required
  store i64 %next, ptr %allocated_slot, align 8
  br label %convert_loop
success:
  store i64 %argc, ptr %descriptor, align 8
  store ptr %vector, ptr %data_slot, align 8
  store i64 %total_bytes, ptr %bytes_slot, align 8
  %released_native = call ptr @LocalFree(ptr %wide_vector)
  ret ptr %descriptor
conversion_error:
  %conversion_code32 = call i32 @GetLastError()
  %conversion_code = zext i32 %conversion_code32 to i64
  store i64 %conversion_code, ptr %failure_slot, align 8
  %released_argument = call i1 @__ncc_raw_aligned_free(ptr %argument)
  br label %cleanup_start
measure_error:
  %measure_code32 = call i32 @GetLastError()
  %measure_code = zext i32 %measure_code32 to i64
  store i64 %measure_code, ptr %failure_slot, align 8
  br label %cleanup_start
argument_alloc_error:
  store i64 8, ptr %failure_slot, align 8
  br label %cleanup_start
cleanup_start:
  %allocated = load i64, ptr %allocated_slot, align 8
  br label %cleanup_loop
cleanup_loop:
  %cleanup_index = phi i64 [ 0, %cleanup_start ], [ %cleanup_next, %cleanup_argument ]
  %cleanup_more = icmp ult i64 %cleanup_index, %allocated
  br i1 %cleanup_more, label %cleanup_argument, label %cleanup_vector
cleanup_argument:
  %cleanup_slot = getelementptr ptr, ptr %vector, i64 %cleanup_index
  %cleanup_value = load ptr, ptr %cleanup_slot, align 8
  %released_value = call i1 @__ncc_raw_aligned_free(ptr %cleanup_value)
  %cleanup_next = add i64 %cleanup_index, 1
  br label %cleanup_loop
cleanup_vector:
  %released_vector = call i1 @__ncc_raw_aligned_free(ptr %vector)
  %released_wide = call ptr @LocalFree(ptr %wide_vector)
  %failure = load i64, ptr %failure_slot, align 8
  %failed_descriptor = call ptr @__ncc_args_set_error(ptr %descriptor, i64 %failure)
  ret ptr %failed_descriptor
vector_error:
  %released_wide_vector = call ptr @LocalFree(ptr %wide_vector)
  %vector_failed_descriptor = call ptr @__ncc_args_set_error(ptr %descriptor, i64 8)
  ret ptr %vector_failed_descriptor
invalid_count:
  %released_invalid_vector = call ptr @LocalFree(ptr %wide_vector)
  %invalid_descriptor = call ptr @__ncc_args_set_error(ptr %descriptor, i64 87)
  ret ptr %invalid_descriptor
split_error:
  %split_code32 = call i32 @GetLastError()
  %split_code = zext i32 %split_code32 to i64
  %split_descriptor = call ptr @__ncc_args_set_error(ptr %descriptor, i64 %split_code)
  ret ptr %split_descriptor
command_error:
  %command_code32 = call i32 @GetLastError()
  %command_code = zext i32 %command_code32 to i64
  %command_descriptor = call ptr @__ncc_args_set_error(ptr %descriptor, i64 %command_code)
  ret ptr %command_descriptor
out_of_memory:
  ret ptr null
}

define i64 @__ncc_args_count(ptr %descriptor) {
entry:
  %count = load i64, ptr %descriptor, align 8
  ret i64 %count
}

define ptr @__ncc_args_at(ptr %descriptor, i64 %index) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  %argument_slot = getelementptr ptr, ptr %data, i64 %index
  %argument = load ptr, ptr %argument_slot, align 8
  ret ptr %argument
}

define ptr @__ncc_args_data(ptr %descriptor) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  ret ptr %data
}

define i64 @__ncc_args_bytes(ptr %descriptor) {
entry:
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %bytes = load i64, ptr %bytes_slot, align 8
  ret i64 %bytes
}

define i64 @__ncc_args_error(ptr %descriptor) {
entry:
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  %error = load i64, ptr %error_slot, align 8
  ret i64 %error
}

define i1 @__ncc_args_release(ptr %descriptor) {
entry:
  %is_null = icmp eq ptr %descriptor, null
  br i1 %is_null, label %done, label %load_descriptor
load_descriptor:
  %count = load i64, ptr %descriptor, align 8
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  br label %release_loop
release_loop:
  %index = phi i64 [ 0, %load_descriptor ], [ %next, %release_argument ]
  %more = icmp ult i64 %index, %count
  br i1 %more, label %release_argument, label %release_vector
release_argument:
  %argument_slot = getelementptr ptr, ptr %data, i64 %index
  %argument = load ptr, ptr %argument_slot, align 8
  %released_argument = call i1 @__ncc_raw_aligned_free(ptr %argument)
  %next = add i64 %index, 1
  br label %release_loop
release_vector:
  %released_data = call i1 @__ncc_raw_aligned_free(ptr %data)
  %released_descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  br label %done
done:
  %result = phi i1 [ true, %entry ], [ %released_descriptor, %release_vector ]
  ret i1 %result
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_args_runtime;

    #[test]
    fn win64_args_runtime_is_manual_crt_free_and_uses_native_process_apis() {
        let ir = render_process_args_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process args runtime");
        for material in [
            "__ncc_args_open",
            "__ncc_args_count",
            "__ncc_args_at",
            "__ncc_args_data",
            "__ncc_args_bytes",
            "__ncc_args_error",
            "__ncc_args_release",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "GetCommandLineW",
            "CommandLineToArgvW",
            "WideCharToMultiByte",
            "LocalFree",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        for forbidden in ["@malloc(", "@calloc(", "@realloc(", "@free(", "msvcrt"] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }

    #[test]
    fn posix_args_runtime_uses_native_host_argument_contracts() {
        let linux = render_process_args_runtime("x86_64-unknown-linux-gnu")
            .expect("Linux process args runtime");
        for material in [
            "define ptr @__ncc_args_open",
            "define ptr @__ncc_args_at",
            "@__ncc_args_proc_path",
            "call i64 @read",
            "@__ncc_raw_aligned_realloc",
        ] {
            assert!(
                linux.contains(material),
                "missing Linux material {material}"
            );
        }
        let darwin = render_process_args_runtime("aarch64-apple-darwin")
            .expect("Darwin process args runtime");
        for material in ["@_NSGetArgc", "@_NSGetArgv", "@__ncc_args_clone_vector"] {
            assert!(
                darwin.contains(material),
                "missing Darwin material {material}"
            );
        }
    }
}
