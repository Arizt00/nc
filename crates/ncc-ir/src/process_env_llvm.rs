pub(crate) fn render_process_env_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if lower.contains("windows") && (lower.contains("x86_64") || lower.contains("aarch64")) {
        return Ok(WIN64_PROCESS_ENV_RUNTIME.to_string());
    }
    if lower.contains("linux") || lower.contains("android") {
        return Ok(render_posix_process_env_runtime(false));
    }
    if lower.contains("darwin") || lower.contains("apple") {
        return Ok(render_posix_process_env_runtime(true));
    }
    Err(format!(
        "native raw process-environment backend has no direct OS ABI for target '{target}'"
    ))
}

fn render_posix_process_env_runtime(darwin: bool) -> String {
    let seed = if darwin {
        r#"declare ptr @_NSGetEnviron()

define internal void @__ncc_env_platform_seed() {
entry:
  %native_slot = call ptr @_NSGetEnviron()
  %native_slot_missing = icmp eq ptr %native_slot, null
  br i1 %native_slot_missing, label %done, label %load
load:
  %native = load ptr, ptr %native_slot, align 8
  store ptr %native, ptr @__ncc_process_environ, align 8
  br label %done
done:
  ret void
}"#
    } else {
        r#"@environ = external global ptr

define internal void @__ncc_env_platform_seed() {
entry:
  %captured = load ptr, ptr @__ncc_process_environ, align 8
  %has_captured = icmp ne ptr %captured, null
  br i1 %has_captured, label %done, label %native_seed
native_seed:
  %native = load ptr, ptr @environ, align 8
  store ptr %native, ptr @__ncc_process_environ, align 8
  br label %done
done:
  ret void
}"#
    };
    // Linux project objects must observe one environment vector after growth.
    let linkage = if darwin { "internal" } else { "weak" };
    LINUX_PROCESS_ENV_RUNTIME
        .replace(
            "@__ncc_process_environ = internal global ptr null",
            &format!("@__ncc_process_environ = {linkage} global ptr null\n{seed}"),
        )
        .replace(
            "define internal ptr @__ncc_env_find_slot(ptr %name) {\nentry:",
            "define internal ptr @__ncc_env_find_slot(ptr %name) {\nentry:\n  call void @__ncc_env_platform_seed()",
        )
        .replace(
            "define ptr @__ncc_env_open() {\nentry:",
            "define ptr @__ncc_env_open() {\nentry:\n  call void @__ncc_env_platform_seed()",
        )
}

const LINUX_PROCESS_ENV_RUNTIME: &str = r#"
@__ncc_process_environ = internal global ptr null
@__ncc_env_last_error_slot = internal global i64 0

define internal i64 @__ncc_env_strlen(ptr %text) {
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

define internal i1 @__ncc_env_name_matches(ptr %entry_text, ptr %name) {
entry:
  %missing_entry = icmp eq ptr %entry_text, null
  %missing_name = icmp eq ptr %name, null
  %missing = or i1 %missing_entry, %missing_name
  br i1 %missing, label %no, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %more ]
  %name_slot = getelementptr i8, ptr %name, i64 %index
  %name_byte = load i8, ptr %name_slot, align 1
  %name_done = icmp eq i8 %name_byte, 0
  br i1 %name_done, label %separator, label %compare
compare:
  %entry_slot = getelementptr i8, ptr %entry_text, i64 %index
  %entry_byte = load i8, ptr %entry_slot, align 1
  %same = icmp eq i8 %entry_byte, %name_byte
  br i1 %same, label %more, label %no
more:
  %next = add i64 %index, 1
  br label %loop
separator:
  %separator_slot = getelementptr i8, ptr %entry_text, i64 %index
  %separator_byte = load i8, ptr %separator_slot, align 1
  %matched = icmp eq i8 %separator_byte, 61
  ret i1 %matched
no:
  ret i1 false
}

define internal ptr @__ncc_env_find_slot(ptr %name) {
entry:
  %vector = load ptr, ptr @__ncc_process_environ, align 1
  %missing = icmp eq ptr %vector, null
  br i1 %missing, label %not_found, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %more ]
  %slot = getelementptr ptr, ptr %vector, i64 %index
  %value = load ptr, ptr %slot, align 1
  %done = icmp eq ptr %value, null
  br i1 %done, label %not_found, label %compare
compare:
  %same = call i1 @__ncc_env_name_matches(ptr %value, ptr %name)
  br i1 %same, label %found, label %more
more:
  %next = add i64 %index, 1
  br label %loop
found:
  ret ptr %slot
not_found:
  ret ptr null
}

define internal ptr @__ncc_env_find_value(ptr %name) {
entry:
  %slot = call ptr @__ncc_env_find_slot(ptr %name)
  %missing = icmp eq ptr %slot, null
  br i1 %missing, label %not_found, label %value
value:
  %entry_text = load ptr, ptr %slot, align 1
  %name_bytes = call i64 @__ncc_env_strlen(ptr %name)
  %value_offset = add i64 %name_bytes, 1
  %value_text = getelementptr i8, ptr %entry_text, i64 %value_offset
  ret ptr %value_text
not_found:
  ret ptr null
}

define ptr @__ncc_env_open() {
entry:
  %native = load ptr, ptr @__ncc_process_environ, align 1
  %native_missing = icmp eq ptr %native, null
  br i1 %native_missing, label %empty_vector, label %measure
measure:
  br label %measure_loop
measure_loop:
  %index = phi i64 [ 0, %measure ], [ %next, %measure_more ]
  %bytes = phi i64 [ 0, %measure ], [ %bytes_next, %measure_more ]
  %slot = getelementptr ptr, ptr %native, i64 %index
  %value = load ptr, ptr %slot, align 1
  %done = icmp eq ptr %value, null
  br i1 %done, label %allocate_descriptor, label %measure_more
measure_more:
  %length = call i64 @__ncc_env_strlen(ptr %value)
  %with_nul = add i64 %length, 1
  %bytes_next = add i64 %bytes, %with_nul
  %next = add i64 %index, 1
  br label %measure_loop
empty_vector:
  br label %allocate_descriptor
allocate_descriptor:
  %count = phi i64 [ %index, %measure_loop ], [ 0, %empty_vector ]
  %total = phi i64 [ %bytes, %measure_loop ], [ 0, %empty_vector ]
  %descriptor = call ptr @__ncc_raw_aligned_alloc(i64 32, i64 8)
  %descriptor_ok = icmp ne ptr %descriptor, null
  br i1 %descriptor_ok, label %allocate_snapshot, label %oom
allocate_snapshot:
  %raw_vector_bytes = mul i64 %count, 8
  %has_entries = icmp ugt i64 %count, 0
  %vector_bytes = select i1 %has_entries, i64 %raw_vector_bytes, i64 8
  %snapshot = call ptr @__ncc_raw_aligned_alloc(i64 %vector_bytes, i64 8)
  %snapshot_ok = icmp ne ptr %snapshot, null
  br i1 %snapshot_ok, label %copy_loop, label %descriptor_error
copy_loop:
  %copy_index = phi i64 [ 0, %allocate_snapshot ], [ %copy_next, %copy_one ]
  %copy_more = icmp ult i64 %copy_index, %count
  br i1 %copy_more, label %copy_one, label %success
copy_one:
  %source_slot = getelementptr ptr, ptr %native, i64 %copy_index
  %source_value = load ptr, ptr %source_slot, align 1
  %target_slot = getelementptr ptr, ptr %snapshot, i64 %copy_index
  store ptr %source_value, ptr %target_slot, align 1
  %copy_next = add i64 %copy_index, 1
  br label %copy_loop
success:
  store i64 %count, ptr %descriptor, align 8
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  store ptr %snapshot, ptr %data_slot, align 8
  store i64 %total, ptr %bytes_slot, align 8
  store i64 0, ptr %error_slot, align 8
  store i64 0, ptr @__ncc_env_last_error_slot, align 8
  ret ptr %descriptor
descriptor_error:
  %released_descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  br label %oom
oom:
  store i64 12, ptr @__ncc_env_last_error_slot, align 8
  ret ptr null
}

define i64 @__ncc_env_count(ptr %descriptor) {
entry:
  %missing = icmp eq ptr %descriptor, null
  br i1 %missing, label %empty, label %load
load:
  %count = load i64, ptr %descriptor, align 8
  ret i64 %count
empty:
  ret i64 0
}

define ptr @__ncc_env_at(ptr %descriptor, i64 %index) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  %value_slot = getelementptr ptr, ptr %data, i64 %index
  %value = load ptr, ptr %value_slot, align 1
  ret ptr %value
}

define ptr @__ncc_env_data(ptr %descriptor) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  ret ptr %data
}

define i64 @__ncc_env_bytes(ptr %descriptor) {
entry:
  %bytes_slot = getelementptr i8, ptr %descriptor, i64 16
  %bytes = load i64, ptr %bytes_slot, align 8
  ret i64 %bytes
}

define i64 @__ncc_env_error(ptr %descriptor) {
entry:
  %missing = icmp eq ptr %descriptor, null
  br i1 %missing, label %last, label %load
load:
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  %error = load i64, ptr %error_slot, align 8
  ret i64 %error
last:
  %last_error = load i64, ptr @__ncc_env_last_error_slot, align 8
  ret i64 %last_error
}

define i1 @__ncc_env_release(ptr %descriptor) {
entry:
  %missing = icmp eq ptr %descriptor, null
  br i1 %missing, label %done, label %release
release:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  %released_data = call i1 @__ncc_raw_aligned_free(ptr %data)
  %released_descriptor = call i1 @__ncc_raw_aligned_free(ptr %descriptor)
  br label %done
done:
  %result = phi i1 [ true, %entry ], [ %released_descriptor, %release ]
  ret i1 %result
}

define i64 @__ncc_env_get(ptr %name, ptr %destination, i64 %capacity) {
entry:
  %value = call ptr @__ncc_env_find_value(ptr %name)
  %missing = icmp eq ptr %value, null
  br i1 %missing, label %not_found, label %measure
measure:
  %length = call i64 @__ncc_env_strlen(ptr %value)
  %required = add i64 %length, 1
  %destination_ok = icmp ne ptr %destination, null
  %capacity_ok = icmp uge i64 %capacity, %required
  %can_copy = and i1 %destination_ok, %capacity_ok
  br i1 %can_copy, label %copy_loop, label %report_required
copy_loop:
  %index = phi i64 [ 0, %measure ], [ %next, %copy_more ]
  %more = icmp ult i64 %index, %required
  br i1 %more, label %copy_more, label %copied
copy_more:
  %source_slot = getelementptr i8, ptr %value, i64 %index
  %byte = load i8, ptr %source_slot, align 1
  %target_slot = getelementptr i8, ptr %destination, i64 %index
  store i8 %byte, ptr %target_slot, align 1
  %next = add i64 %index, 1
  br label %copy_loop
copied:
  store i64 0, ptr @__ncc_env_last_error_slot, align 8
  ret i64 %length
report_required:
  store i64 0, ptr @__ncc_env_last_error_slot, align 8
  ret i64 %required
not_found:
  store i64 2, ptr @__ncc_env_last_error_slot, align 8
  ret i64 -2
}

define i1 @__ncc_env_has(ptr %name) {
entry:
  %value = call ptr @__ncc_env_find_value(ptr %name)
  %present = icmp ne ptr %value, null
  ret i1 %present
}

define i1 @__ncc_env_set(ptr %name, ptr %value) {
entry:
  %name_missing = icmp eq ptr %name, null
  %value_missing = icmp eq ptr %value, null
  %missing = or i1 %name_missing, %value_missing
  br i1 %missing, label %invalid, label %allocate
allocate:
  %name_bytes = call i64 @__ncc_env_strlen(ptr %name)
  %value_bytes = call i64 @__ncc_env_strlen(ptr %value)
  %prefix_bytes = add i64 %name_bytes, 1
  %entry_bytes = add i64 %prefix_bytes, %value_bytes
  %storage_bytes = add i64 %entry_bytes, 1
  %new_entry = call ptr @__ncc_raw_aligned_alloc(i64 %storage_bytes, i64 8)
  %allocated = icmp ne ptr %new_entry, null
  br i1 %allocated, label %copy_name_loop, label %oom
copy_name_loop:
  %name_index = phi i64 [ 0, %allocate ], [ %name_next, %copy_name ]
  %name_more = icmp ult i64 %name_index, %name_bytes
  br i1 %name_more, label %copy_name, label %separator
copy_name:
  %name_source = getelementptr i8, ptr %name, i64 %name_index
  %name_byte = load i8, ptr %name_source, align 1
  %name_target = getelementptr i8, ptr %new_entry, i64 %name_index
  store i8 %name_byte, ptr %name_target, align 1
  %name_next = add i64 %name_index, 1
  br label %copy_name_loop
separator:
  %separator_slot = getelementptr i8, ptr %new_entry, i64 %name_bytes
  store i8 61, ptr %separator_slot, align 1
  br label %copy_value_loop
copy_value_loop:
  %value_index = phi i64 [ 0, %separator ], [ %value_next, %copy_value ]
  %value_more = icmp ule i64 %value_index, %value_bytes
  br i1 %value_more, label %copy_value, label %install
copy_value:
  %value_source = getelementptr i8, ptr %value, i64 %value_index
  %value_byte = load i8, ptr %value_source, align 1
  %target_index = add i64 %prefix_bytes, %value_index
  %value_target = getelementptr i8, ptr %new_entry, i64 %target_index
  store i8 %value_byte, ptr %value_target, align 1
  %value_next = add i64 %value_index, 1
  br label %copy_value_loop
install:
  %existing_slot = call ptr @__ncc_env_find_slot(ptr %name)
  %exists = icmp ne ptr %existing_slot, null
  br i1 %exists, label %replace, label %grow_measure
replace:
  store ptr %new_entry, ptr %existing_slot, align 1
  br label %success
grow_measure:
  %old_vector = load ptr, ptr @__ncc_process_environ, align 1
  %old_missing = icmp eq ptr %old_vector, null
  br i1 %old_missing, label %grow_allocate_empty, label %grow_count_loop
grow_count_loop:
  %count = phi i64 [ 0, %grow_measure ], [ %count_next, %grow_count_more ]
  %old_slot = getelementptr ptr, ptr %old_vector, i64 %count
  %old_value = load ptr, ptr %old_slot, align 1
  %count_done = icmp eq ptr %old_value, null
  br i1 %count_done, label %grow_allocate, label %grow_count_more
grow_count_more:
  %count_next = add i64 %count, 1
  br label %grow_count_loop
grow_allocate_empty:
  br label %grow_allocate
grow_allocate:
  %old_count = phi i64 [ %count, %grow_count_loop ], [ 0, %grow_allocate_empty ]
  %new_count = add i64 %old_count, 2
  %vector_bytes = mul i64 %new_count, 8
  %new_vector = call ptr @__ncc_raw_aligned_alloc(i64 %vector_bytes, i64 8)
  %vector_ok = icmp ne ptr %new_vector, null
  br i1 %vector_ok, label %grow_copy_loop, label %entry_oom
grow_copy_loop:
  %copy_index = phi i64 [ 0, %grow_allocate ], [ %copy_next, %grow_copy ]
  %copy_more = icmp ult i64 %copy_index, %old_count
  br i1 %copy_more, label %grow_copy, label %grow_append
grow_copy:
  %copy_source_slot = getelementptr ptr, ptr %old_vector, i64 %copy_index
  %copied_pointer = load ptr, ptr %copy_source_slot, align 1
  %copy_target_slot = getelementptr ptr, ptr %new_vector, i64 %copy_index
  store ptr %copied_pointer, ptr %copy_target_slot, align 1
  %copy_next = add i64 %copy_index, 1
  br label %grow_copy_loop
grow_append:
  %append_slot = getelementptr ptr, ptr %new_vector, i64 %old_count
  store ptr %new_entry, ptr %append_slot, align 1
  %terminator_index = add i64 %old_count, 1
  %terminator_slot = getelementptr ptr, ptr %new_vector, i64 %terminator_index
  store ptr null, ptr %terminator_slot, align 1
  store ptr %new_vector, ptr @__ncc_process_environ, align 1
  br label %success
success:
  store i64 0, ptr @__ncc_env_last_error_slot, align 8
  ret i1 true
entry_oom:
  %released_entry = call i1 @__ncc_raw_aligned_free(ptr %new_entry)
  br label %oom
oom:
  store i64 12, ptr @__ncc_env_last_error_slot, align 8
  ret i1 false
invalid:
  store i64 22, ptr @__ncc_env_last_error_slot, align 8
  ret i1 false
}

define i1 @__ncc_env_remove(ptr %name) {
entry:
  %slot = call ptr @__ncc_env_find_slot(ptr %name)
  %missing = icmp eq ptr %slot, null
  br i1 %missing, label %success, label %shift_loop
shift_loop:
  %current = phi ptr [ %slot, %entry ], [ %next_slot, %shift_more ]
  %next_slot = getelementptr ptr, ptr %current, i64 1
  %next_value = load ptr, ptr %next_slot, align 1
  store ptr %next_value, ptr %current, align 1
  %done = icmp eq ptr %next_value, null
  br i1 %done, label %success, label %shift_more
shift_more:
  br label %shift_loop
success:
  store i64 0, ptr @__ncc_env_last_error_slot, align 8
  ret i1 true
}

define internal i64 @__ncc_env_emit(ptr %destination, i64 %capacity, i64 %position, i8 %byte) {
entry:
  %destination_ok = icmp ne ptr %destination, null
  %capacity_ok = icmp ult i64 %position, %capacity
  %can_write = and i1 %destination_ok, %capacity_ok
  br i1 %can_write, label %write, label %done
write:
  %slot = getelementptr i8, ptr %destination, i64 %position
  store i8 %byte, ptr %slot, align 1
  br label %done
done:
  %next = add i64 %position, 1
  ret i64 %next
}

define i64 @__ncc_env_expand(ptr %source, ptr %destination, i64 %capacity) {
entry:
  %missing = icmp eq ptr %source, null
  br i1 %missing, label %invalid, label %loop
loop:
  %source_at = phi i64 [ 0, %entry ], [ %plain_next_source, %plain ], [ %literal_next_source, %literal_done ], [ %value_next_source, %value_done ]
  %output_at = phi i64 [ 0, %entry ], [ %plain_next_output, %plain ], [ %literal_next_output, %literal_done ], [ %value_next_output, %value_done ]
  %source_slot = getelementptr i8, ptr %source, i64 %source_at
  %byte = load i8, ptr %source_slot, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %finish, label %classify
classify:
  %is_percent = icmp eq i8 %byte, 37
  br i1 %is_percent, label %find_close, label %plain
plain:
  %plain_next_output = call i64 @__ncc_env_emit(ptr %destination, i64 %capacity, i64 %output_at, i8 %byte)
  %plain_next_source = add i64 %source_at, 1
  br label %loop
find_close:
  %name_start = add i64 %source_at, 1
  br label %close_loop
close_loop:
  %close_at = phi i64 [ %name_start, %find_close ], [ %close_next, %close_more ]
  %close_slot = getelementptr i8, ptr %source, i64 %close_at
  %close_byte = load i8, ptr %close_slot, align 1
  %close_end = icmp eq i8 %close_byte, 0
  %close_found = icmp eq i8 %close_byte, 37
  br i1 %close_found, label %allocate_name, label %close_check_end
close_check_end:
  br i1 %close_end, label %plain, label %close_more
close_more:
  %close_next = add i64 %close_at, 1
  br label %close_loop
allocate_name:
  %name_length = sub i64 %close_at, %name_start
  %name_bytes = add i64 %name_length, 1
  %name = call ptr @__ncc_raw_aligned_alloc(i64 %name_bytes, i64 8)
  %name_ok = icmp ne ptr %name, null
  br i1 %name_ok, label %copy_name_loop, label %oom
copy_name_loop:
  %name_index = phi i64 [ 0, %allocate_name ], [ %name_next, %copy_name ]
  %name_more = icmp ult i64 %name_index, %name_length
  br i1 %name_more, label %copy_name, label %terminate_name
copy_name:
  %name_source_index = add i64 %name_start, %name_index
  %name_source_slot = getelementptr i8, ptr %source, i64 %name_source_index
  %name_byte = load i8, ptr %name_source_slot, align 1
  %name_target_slot = getelementptr i8, ptr %name, i64 %name_index
  store i8 %name_byte, ptr %name_target_slot, align 1
  %name_next = add i64 %name_index, 1
  br label %copy_name_loop
terminate_name:
  %name_terminator = getelementptr i8, ptr %name, i64 %name_length
  store i8 0, ptr %name_terminator, align 1
  %value = call ptr @__ncc_env_find_value(ptr %name)
  %released_name = call i1 @__ncc_raw_aligned_free(ptr %name)
  %value_missing = icmp eq ptr %value, null
  br i1 %value_missing, label %literal_loop, label %value_loop
literal_loop:
  %literal_at = phi i64 [ %source_at, %terminate_name ], [ %literal_next_at, %literal_more ]
  %literal_output = phi i64 [ %output_at, %terminate_name ], [ %literal_written, %literal_more ]
  %literal_slot = getelementptr i8, ptr %source, i64 %literal_at
  %literal_byte = load i8, ptr %literal_slot, align 1
  %literal_written = call i64 @__ncc_env_emit(ptr %destination, i64 %capacity, i64 %literal_output, i8 %literal_byte)
  %literal_complete = icmp eq i64 %literal_at, %close_at
  br i1 %literal_complete, label %literal_done, label %literal_more
literal_more:
  %literal_next_at = add i64 %literal_at, 1
  br label %literal_loop
literal_done:
  %literal_next_source = add i64 %close_at, 1
  %literal_next_output = add i64 %literal_output, 1
  br label %loop
value_loop:
  %value_at = phi i64 [ 0, %terminate_name ], [ %value_more_at, %value_more ]
  %value_output = phi i64 [ %output_at, %terminate_name ], [ %value_written, %value_more ]
  %value_slot = getelementptr i8, ptr %value, i64 %value_at
  %value_byte = load i8, ptr %value_slot, align 1
  %value_complete = icmp eq i8 %value_byte, 0
  br i1 %value_complete, label %value_done, label %value_more
value_more:
  %value_written = call i64 @__ncc_env_emit(ptr %destination, i64 %capacity, i64 %value_output, i8 %value_byte)
  %value_more_at = add i64 %value_at, 1
  br label %value_loop
value_done:
  %value_next_output = phi i64 [ %value_output, %value_loop ]
  %value_next_source = add i64 %close_at, 1
  br label %loop
finish:
  %required = add i64 %output_at, 1
  %destination_ok = icmp ne ptr %destination, null
  %capacity_ok = icmp uge i64 %capacity, %required
  %can_finish = and i1 %destination_ok, %capacity_ok
  br i1 %can_finish, label %write_nul, label %report_required
write_nul:
  %nul_slot = getelementptr i8, ptr %destination, i64 %output_at
  store i8 0, ptr %nul_slot, align 1
  store i64 0, ptr @__ncc_env_last_error_slot, align 8
  ret i64 %output_at
report_required:
  ret i64 %required
oom:
  store i64 12, ptr @__ncc_env_last_error_slot, align 8
  ret i64 -12
invalid:
  store i64 22, ptr @__ncc_env_last_error_slot, align 8
  ret i64 -22
}

define i64 @__ncc_env_last_error() {
entry:
  %error = load i64, ptr @__ncc_env_last_error_slot, align 8
  ret i64 %error
}
"#;

const WIN64_PROCESS_ENV_RUNTIME: &str = r#"
declare ptr @GetEnvironmentStringsW()
declare i32 @FreeEnvironmentStringsW(ptr)
declare i32 @GetEnvironmentVariableW(ptr, ptr, i32)
declare i32 @SetEnvironmentVariableW(ptr, ptr)
declare i32 @ExpandEnvironmentStringsW(ptr, ptr, i32)
declare void @SetLastError(i32)
declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare i32 @WideCharToMultiByte(i32, i32, ptr, i32, ptr, i32, ptr, ptr)

define internal ptr @__ncc_env_utf8_to_wide(ptr %text) {
entry:
  %is_null = icmp eq ptr %text, null
  br i1 %is_null, label %bad, label %measure
measure:
  %chars = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %text, i32 -1, ptr null, i32 0)
  %chars_ok = icmp sgt i32 %chars, 0
  br i1 %chars_ok, label %allocate, label %bad
allocate:
  %chars64 = zext i32 %chars to i64
  %bytes = shl i64 %chars64, 1
  %wide = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %wide_ok = icmp ne ptr %wide, null
  br i1 %wide_ok, label %convert, label %bad
convert:
  %written = call i32 @MultiByteToWideChar(i32 65001, i32 8, ptr %text, i32 -1, ptr %wide, i32 %chars)
  %converted = icmp eq i32 %written, %chars
  br i1 %converted, label %ok, label %free_bad
free_bad:
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide)
  br label %bad
ok:
  ret ptr %wide
bad:
  ret ptr null
}

define internal ptr @__ncc_env_wide_next(ptr %text) {
entry:
  br label %scan
scan:
  %cursor = phi ptr [ %text, %entry ], [ %next, %more ]
  %unit = load i16, ptr %cursor, align 2
  %done = icmp eq i16 %unit, 0
  br i1 %done, label %found, label %more
more:
  %next = getelementptr i16, ptr %cursor, i64 1
  br label %scan
found:
  %after = getelementptr i16, ptr %cursor, i64 1
  ret ptr %after
}

define internal ptr @__ncc_env_set_error(ptr %descriptor, i64 %error) {
entry:
  %error_slot = getelementptr i8, ptr %descriptor, i64 24
  store i64 %error, ptr %error_slot, align 8
  ret ptr %descriptor
}

define ptr @__ncc_env_open() {
entry:
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
  %native_block = call ptr @GetEnvironmentStringsW()
  %block_ok = icmp ne ptr %native_block, null
  br i1 %block_ok, label %count_loop, label %native_error
count_loop:
  %count_cursor = phi ptr [ %native_block, %initialize ], [ %count_next_cursor, %count_advance ]
  %count = phi i64 [ 0, %initialize ], [ %count_next, %count_advance ]
  %first = load i16, ptr %count_cursor, align 2
  %block_end = icmp eq i16 %first, 0
  br i1 %block_end, label %allocate_vector, label %count_advance
count_advance:
  %count_next_cursor = call ptr @__ncc_env_wide_next(ptr %count_cursor)
  %count_next = add i64 %count, 1
  br label %count_loop
allocate_vector:
  %vector_bytes_raw = mul i64 %count, 8
  %has_values = icmp ugt i64 %count, 0
  %vector_bytes = select i1 %has_values, i64 %vector_bytes_raw, i64 8
  %vector = call ptr @__ncc_raw_aligned_alloc(i64 %vector_bytes, i64 8)
  %vector_ok = icmp ne ptr %vector, null
  br i1 %vector_ok, label %convert_loop, label %vector_error
convert_loop:
  %cursor = phi ptr [ %native_block, %allocate_vector ], [ %next_cursor, %store_value ]
  %index = phi i64 [ 0, %allocate_vector ], [ %next_index, %store_value ]
  %total = phi i64 [ 0, %allocate_vector ], [ %total_next, %store_value ]
  %more_values = icmp ult i64 %index, %count
  br i1 %more_values, label %measure_value, label %success
measure_value:
  %required32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %cursor, i32 -1, ptr null, i32 0, ptr null, ptr null)
  %required_ok = icmp sgt i32 %required32, 0
  br i1 %required_ok, label %allocate_value, label %convert_error
allocate_value:
  %required = zext i32 %required32 to i64
  %value = call ptr @__ncc_raw_aligned_alloc(i64 %required, i64 8)
  %value_ok = icmp ne ptr %value, null
  br i1 %value_ok, label %convert_value, label %alloc_error
convert_value:
  %written32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %cursor, i32 -1, ptr %value, i32 %required32, ptr null, ptr null)
  %converted = icmp eq i32 %written32, %required32
  br i1 %converted, label %store_value, label %value_error
store_value:
  %value_slot = getelementptr ptr, ptr %vector, i64 %index
  store ptr %value, ptr %value_slot, align 8
  %next_cursor = call ptr @__ncc_env_wide_next(ptr %cursor)
  %next_index = add i64 %index, 1
  %total_next = add i64 %total, %required
  br label %convert_loop
success:
  store i64 %count, ptr %descriptor, align 8
  store ptr %vector, ptr %data_slot, align 8
  store i64 %total, ptr %bytes_slot, align 8
  %released_native = call i32 @FreeEnvironmentStringsW(ptr %native_block)
  ret ptr %descriptor
value_error:
  %released_bad_value = call i1 @__ncc_raw_aligned_free(ptr %value)
  %value_code32 = call i32 @GetLastError()
  %value_code = zext i32 %value_code32 to i64
  br label %cleanup_start
convert_error:
  %convert_code32 = call i32 @GetLastError()
  %convert_code = zext i32 %convert_code32 to i64
  br label %cleanup_start
alloc_error:
  br label %cleanup_start
cleanup_start:
  %failure = phi i64 [ %value_code, %value_error ], [ %convert_code, %convert_error ], [ 8, %alloc_error ]
  br label %cleanup_loop
cleanup_loop:
  %cleanup_index = phi i64 [ 0, %cleanup_start ], [ %cleanup_next, %cleanup_value ]
  %cleanup_more = icmp ult i64 %cleanup_index, %index
  br i1 %cleanup_more, label %cleanup_value, label %cleanup_done
cleanup_value:
  %cleanup_slot = getelementptr ptr, ptr %vector, i64 %cleanup_index
  %cleanup_ptr = load ptr, ptr %cleanup_slot, align 8
  %released_value = call i1 @__ncc_raw_aligned_free(ptr %cleanup_ptr)
  %cleanup_next = add i64 %cleanup_index, 1
  br label %cleanup_loop
cleanup_done:
  %released_vector = call i1 @__ncc_raw_aligned_free(ptr %vector)
  %released_block = call i32 @FreeEnvironmentStringsW(ptr %native_block)
  %failed_descriptor = call ptr @__ncc_env_set_error(ptr %descriptor, i64 %failure)
  ret ptr %failed_descriptor
vector_error:
  %released_vector_block = call i32 @FreeEnvironmentStringsW(ptr %native_block)
  %vector_failed = call ptr @__ncc_env_set_error(ptr %descriptor, i64 8)
  ret ptr %vector_failed
native_error:
  %native_code32 = call i32 @GetLastError()
  %native_code = zext i32 %native_code32 to i64
  %native_failed = call ptr @__ncc_env_set_error(ptr %descriptor, i64 %native_code)
  ret ptr %native_failed
out_of_memory:
  ret ptr null
}

define i64 @__ncc_env_count(ptr %descriptor) {
entry:
  %value = load i64, ptr %descriptor, align 8
  ret i64 %value
}

define ptr @__ncc_env_at(ptr %descriptor, i64 %index) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  %value_slot = getelementptr ptr, ptr %data, i64 %index
  %value = load ptr, ptr %value_slot, align 8
  ret ptr %value
}

define ptr @__ncc_env_data(ptr %descriptor) {
entry:
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  ret ptr %data
}

define i64 @__ncc_env_bytes(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 16
  %value = load i64, ptr %slot, align 8
  ret i64 %value
}

define i64 @__ncc_env_error(ptr %descriptor) {
entry:
  %slot = getelementptr i8, ptr %descriptor, i64 24
  %value = load i64, ptr %slot, align 8
  ret i64 %value
}

define i1 @__ncc_env_release(ptr %descriptor) {
entry:
  %is_null = icmp eq ptr %descriptor, null
  br i1 %is_null, label %done, label %load_descriptor
load_descriptor:
  %count = load i64, ptr %descriptor, align 8
  %data_slot = getelementptr i8, ptr %descriptor, i64 8
  %data = load ptr, ptr %data_slot, align 8
  br label %release_loop
release_loop:
  %index = phi i64 [ 0, %load_descriptor ], [ %next, %release_value ]
  %more = icmp ult i64 %index, %count
  br i1 %more, label %release_value, label %release_vector
release_value:
  %value_slot = getelementptr ptr, ptr %data, i64 %index
  %value = load ptr, ptr %value_slot, align 8
  %released_value = call i1 @__ncc_raw_aligned_free(ptr %value)
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

define i64 @__ncc_env_get(ptr %name, ptr %destination, i64 %capacity64) {
entry:
  %wide_name = call ptr @__ncc_env_utf8_to_wide(ptr %name)
  %name_ok = icmp ne ptr %wide_name, null
  br i1 %name_ok, label %measure, label %convert_error
measure:
  call void @SetLastError(i32 0)
  %wide_required = call i32 @GetEnvironmentVariableW(ptr %wide_name, ptr null, i32 0)
  %has_value = icmp ugt i32 %wide_required, 0
  br i1 %has_value, label %allocate, label %zero_value
zero_value:
  %zero_error = call i32 @GetLastError()
  %zero_ok = icmp eq i32 %zero_error, 0
  br i1 %zero_ok, label %empty_value, label %native_error
empty_value:
  %capacity_positive = icmp sgt i64 %capacity64, 0
  %destination_present = icmp ne ptr %destination, null
  %can_write_empty = and i1 %capacity_positive, %destination_present
  br i1 %can_write_empty, label %write_empty, label %empty_done
write_empty:
  store i8 0, ptr %destination, align 1
  br label %empty_done
empty_done:
  %released_empty_name = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  ret i64 0
allocate:
  %wide_required64 = zext i32 %wide_required to i64
  %wide_bytes = shl i64 %wide_required64, 1
  %wide_value = call ptr @__ncc_raw_aligned_alloc(i64 %wide_bytes, i64 8)
  %wide_ok = icmp ne ptr %wide_value, null
  br i1 %wide_ok, label %read, label %oom
read:
  %wide_written = call i32 @GetEnvironmentVariableW(ptr %wide_name, ptr %wide_value, i32 %wide_required)
  %read_ok = icmp ult i32 %wide_written, %wide_required
  br i1 %read_ok, label %measure_utf8, label %read_error
measure_utf8:
  %utf8_required32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %wide_value, i32 -1, ptr null, i32 0, ptr null, ptr null)
  %utf8_ok = icmp sgt i32 %utf8_required32, 0
  br i1 %utf8_ok, label %capacity, label %utf8_error
capacity:
  %utf8_required = zext i32 %utf8_required32 to i64
  %destination_present2 = icmp ne ptr %destination, null
  %capacity_ok = icmp uge i64 %capacity64, %utf8_required
  %can_write = and i1 %destination_present2, %capacity_ok
  br i1 %can_write, label %convert, label %report_required
convert:
  %written32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %wide_value, i32 -1, ptr %destination, i32 %utf8_required32, ptr null, ptr null)
  %converted = icmp eq i32 %written32, %utf8_required32
  br i1 %converted, label %success, label %utf8_error
success:
  %bytes_without_nul = sub i64 %utf8_required, 1
  br label %release_success
report_required:
  br label %release_success
release_success:
  %result = phi i64 [ %bytes_without_nul, %success ], [ %utf8_required, %report_required ]
  %released_value = call i1 @__ncc_raw_aligned_free(ptr %wide_value)
  %released_name = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  ret i64 %result
read_error:
  %read_code = call i32 @GetLastError()
  br label %release_error
utf8_error:
  %utf8_code = call i32 @GetLastError()
  br label %release_error
release_error:
  %error32 = phi i32 [ %read_code, %read_error ], [ %utf8_code, %utf8_error ]
  %released_error_value = call i1 @__ncc_raw_aligned_free(ptr %wide_value)
  %released_error_name = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  %error64 = zext i32 %error32 to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
native_error:
  %released_native_name = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  %native_error64 = zext i32 %zero_error to i64
  %native_negative = sub i64 0, %native_error64
  ret i64 %native_negative
oom:
  %released_oom_name = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  ret i64 -8
convert_error:
  %convert_code = call i32 @GetLastError()
  %convert_code64 = zext i32 %convert_code to i64
  %convert_negative = sub i64 0, %convert_code64
  ret i64 %convert_negative
}

define i1 @__ncc_env_has(ptr %name) {
entry:
  %wide_name = call ptr @__ncc_env_utf8_to_wide(ptr %name)
  %name_ok = icmp ne ptr %wide_name, null
  br i1 %name_ok, label %query, label %bad
query:
  call void @SetLastError(i32 0)
  %required = call i32 @GetEnvironmentVariableW(ptr %wide_name, ptr null, i32 0)
  %error = call i32 @GetLastError()
  %has_bytes = icmp ugt i32 %required, 0
  %empty_exists = icmp eq i32 %error, 0
  %exists = or i1 %has_bytes, %empty_exists
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  ret i1 %exists
bad:
  ret i1 false
}

define i1 @__ncc_env_set(ptr %name, ptr %value) {
entry:
  %wide_name = call ptr @__ncc_env_utf8_to_wide(ptr %name)
  %wide_value = call ptr @__ncc_env_utf8_to_wide(ptr %value)
  %name_ok = icmp ne ptr %wide_name, null
  %value_ok = icmp ne ptr %wide_value, null
  %inputs_ok = and i1 %name_ok, %value_ok
  br i1 %inputs_ok, label %set, label %cleanup_bad
set:
  %status = call i32 @SetEnvironmentVariableW(ptr %wide_name, ptr %wide_value)
  %ok = icmp ne i32 %status, 0
  br label %cleanup
cleanup_bad:
  br label %cleanup
cleanup:
  %result = phi i1 [ %ok, %set ], [ false, %cleanup_bad ]
  %released_name = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  %released_value = call i1 @__ncc_raw_aligned_free(ptr %wide_value)
  ret i1 %result
}

define i1 @__ncc_env_remove(ptr %name) {
entry:
  %wide_name = call ptr @__ncc_env_utf8_to_wide(ptr %name)
  %name_ok = icmp ne ptr %wide_name, null
  br i1 %name_ok, label %remove, label %bad
remove:
  %status = call i32 @SetEnvironmentVariableW(ptr %wide_name, ptr null)
  %released = call i1 @__ncc_raw_aligned_free(ptr %wide_name)
  %ok = icmp ne i32 %status, 0
  ret i1 %ok
bad:
  ret i1 false
}

define i64 @__ncc_env_expand(ptr %source, ptr %destination, i64 %capacity64) {
entry:
  %wide_source = call ptr @__ncc_env_utf8_to_wide(ptr %source)
  %source_ok = icmp ne ptr %wide_source, null
  br i1 %source_ok, label %measure, label %convert_error
measure:
  %wide_required = call i32 @ExpandEnvironmentStringsW(ptr %wide_source, ptr null, i32 0)
  %required_ok = icmp ugt i32 %wide_required, 0
  br i1 %required_ok, label %allocate, label %native_error
allocate:
  %wide_required64 = zext i32 %wide_required to i64
  %wide_bytes = shl i64 %wide_required64, 1
  %wide_value = call ptr @__ncc_raw_aligned_alloc(i64 %wide_bytes, i64 8)
  %wide_ok = icmp ne ptr %wide_value, null
  br i1 %wide_ok, label %expand, label %oom
expand:
  %wide_written = call i32 @ExpandEnvironmentStringsW(ptr %wide_source, ptr %wide_value, i32 %wide_required)
  %expanded = icmp eq i32 %wide_written, %wide_required
  br i1 %expanded, label %measure_utf8, label %expand_error
measure_utf8:
  %utf8_required32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %wide_value, i32 -1, ptr null, i32 0, ptr null, ptr null)
  %utf8_ok = icmp sgt i32 %utf8_required32, 0
  br i1 %utf8_ok, label %capacity, label %utf8_error
capacity:
  %utf8_required = zext i32 %utf8_required32 to i64
  %destination_present = icmp ne ptr %destination, null
  %capacity_ok = icmp uge i64 %capacity64, %utf8_required
  %can_write = and i1 %destination_present, %capacity_ok
  br i1 %can_write, label %convert, label %report_required
convert:
  %written32 = call i32 @WideCharToMultiByte(i32 65001, i32 128, ptr %wide_value, i32 -1, ptr %destination, i32 %utf8_required32, ptr null, ptr null)
  %converted = icmp eq i32 %written32, %utf8_required32
  br i1 %converted, label %success, label %utf8_error
success:
  %bytes_without_nul = sub i64 %utf8_required, 1
  br label %release_success
report_required:
  br label %release_success
release_success:
  %result = phi i64 [ %bytes_without_nul, %success ], [ %utf8_required, %report_required ]
  %released_value = call i1 @__ncc_raw_aligned_free(ptr %wide_value)
  %released_source = call i1 @__ncc_raw_aligned_free(ptr %wide_source)
  ret i64 %result
expand_error:
  %expand_code = call i32 @GetLastError()
  br label %release_error
utf8_error:
  %utf8_code = call i32 @GetLastError()
  br label %release_error
release_error:
  %error32 = phi i32 [ %expand_code, %expand_error ], [ %utf8_code, %utf8_error ]
  %released_error_value = call i1 @__ncc_raw_aligned_free(ptr %wide_value)
  %released_error_source = call i1 @__ncc_raw_aligned_free(ptr %wide_source)
  %error64 = zext i32 %error32 to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
native_error:
  %native_code = call i32 @GetLastError()
  %released_native_source = call i1 @__ncc_raw_aligned_free(ptr %wide_source)
  %native_code64 = zext i32 %native_code to i64
  %native_negative = sub i64 0, %native_code64
  ret i64 %native_negative
oom:
  %released_oom_source = call i1 @__ncc_raw_aligned_free(ptr %wide_source)
  ret i64 -8
convert_error:
  %convert_code = call i32 @GetLastError()
  %convert_code64 = zext i32 %convert_code to i64
  %convert_negative = sub i64 0, %convert_code64
  ret i64 %convert_negative
}

define i64 @__ncc_env_last_error() {
entry:
  %error32 = call i32 @GetLastError()
  %error = zext i32 %error32 to i64
  ret i64 %error
}
"#;

#[cfg(test)]
mod tests {
    use super::render_process_env_runtime;

    #[test]
    fn win64_env_runtime_is_manual_crt_free_and_uses_native_environment_apis() {
        let ir = render_process_env_runtime("x86_64-pc-windows-msvc")
            .expect("Win64 process environment runtime");
        for material in [
            "__ncc_env_open",
            "__ncc_env_count",
            "__ncc_env_at",
            "__ncc_env_data",
            "__ncc_env_bytes",
            "__ncc_env_error",
            "__ncc_env_release",
            "__ncc_env_get",
            "__ncc_env_has",
            "__ncc_env_set",
            "__ncc_env_remove",
            "__ncc_env_expand",
            "__ncc_env_last_error",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        for native in [
            "GetEnvironmentStringsW",
            "FreeEnvironmentStringsW",
            "GetEnvironmentVariableW",
            "SetEnvironmentVariableW",
            "ExpandEnvironmentStringsW",
        ] {
            assert!(ir.contains(&format!("@{native}(")));
        }
        for forbidden in [
            "@getenv(",
            "@putenv(",
            "@_wgetenv(",
            "@malloc(",
            "@calloc(",
            "@realloc(",
            "@free(",
            "msvcrt",
        ] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }

    #[test]
    fn linux_env_runtime_uses_the_process_vector_without_c_memory_or_lookup() {
        let ir = render_process_env_runtime("x86_64-unknown-linux-gnu")
            .expect("Linux process environment runtime");
        for material in [
            "__ncc_env_open",
            "__ncc_env_count",
            "__ncc_env_at",
            "__ncc_env_data",
            "__ncc_env_bytes",
            "__ncc_env_error",
            "__ncc_env_release",
            "__ncc_env_get",
            "__ncc_env_has",
            "__ncc_env_set",
            "__ncc_env_remove",
            "__ncc_env_expand",
            "__ncc_env_last_error",
        ] {
            assert!(ir.contains(&format!("@{material}(")));
        }
        assert!(ir.contains("@__ncc_process_environ = weak global ptr null"));
        assert!(!ir.contains("@__ncc_process_environ = internal global ptr null"));
        assert!(ir.contains("@environ = external global ptr"));
        assert!(ir.contains("%has_captured = icmp ne ptr %captured, null"));
        assert!(ir.contains("br i1 %has_captured, label %done, label %native_seed"));
        assert!(ir.contains("call void @__ncc_env_platform_seed()"));
        for forbidden in [
            "@getenv(",
            "@putenv(",
            "@setenv(",
            "@unsetenv(",
            "@malloc(",
            "@calloc(",
            "@realloc(",
            "@free(",
        ] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }

    #[test]
    fn darwin_env_runtime_reads_the_native_environment_vector_without_libc_lookup() {
        let ir = render_process_env_runtime("x86_64-apple-darwin")
            .expect("Darwin process environment runtime");
        assert!(ir.contains("declare ptr @_NSGetEnviron()"));
        assert!(ir.contains("define internal void @__ncc_env_platform_seed()"));
        assert!(ir.contains("call void @__ncc_env_platform_seed()"));
        assert!(ir.contains("define i64 @__ncc_env_get("));
        for forbidden in ["@getenv(", "@setenv(", "@unsetenv(", "@malloc(", "@free("] {
            assert!(!ir.contains(forbidden), "unexpected C provider {forbidden}");
        }
    }
}
