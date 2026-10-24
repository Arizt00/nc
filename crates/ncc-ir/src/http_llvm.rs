pub(crate) fn render_http_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if !lower.contains("windows") || !(lower.contains("x86_64") || lower.contains("aarch64")) {
        return Err(format!(
            "native HTTP backend currently requires Win64 WinHTTP, got target '{target}'"
        ));
    }
    Ok(WIN64_HTTP_RUNTIME.to_string())
}

const WIN64_HTTP_RUNTIME: &str = r#"
@__ncc_http_agent = private unnamed_addr constant [7 x i16] [i16 110, i16 67, i16 47, i16 49, i16 46, i16 48, i16 0], align 2
@__ncc_http_get_text = private unnamed_addr constant [4 x i8] c"GET\00", align 1
@__ncc_http_post_text = private unnamed_addr constant [5 x i8] c"POST\00", align 1
@__ncc_http_empty = private unnamed_addr constant [1 x i8] zeroinitializer, align 1

declare i32 @MultiByteToWideChar(i32, i32, ptr, i32, ptr, i32)
declare ptr @WinHttpOpen(ptr, i32, ptr, ptr, i32)
declare ptr @WinHttpConnect(ptr, ptr, i16, i32)
declare ptr @WinHttpOpenRequest(ptr, ptr, ptr, ptr, ptr, ptr, i32)
declare i32 @WinHttpSetTimeouts(ptr, i32, i32, i32, i32)
declare i32 @WinHttpCrackUrl(ptr, i32, i32, ptr)
declare i32 @WinHttpSendRequest(ptr, ptr, i32, ptr, i32, i32, i64)
declare i32 @WinHttpReceiveResponse(ptr, ptr)
declare i32 @WinHttpQueryHeaders(ptr, i32, ptr, ptr, ptr, ptr)
declare i32 @WinHttpQueryDataAvailable(ptr, ptr)
declare i32 @WinHttpReadData(ptr, ptr, i32, ptr)
declare i32 @WinHttpCloseHandle(ptr)
define internal ptr @__ncc_http_utf8_to_wide(ptr %text) {
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
  %ignored = call i1 @__ncc_raw_aligned_free(ptr %wide)
  br label %bad
ok:
  ret ptr %wide
bad:
  ret ptr null
}

define internal void @__ncc_http_free_if(ptr %value) {
entry:
  %present = icmp ne ptr %value, null
  br i1 %present, label %free, label %done
free:
  %ignored = call i1 @__ncc_raw_aligned_free(ptr %value)
  br label %done
done:
  ret void
}

define internal void @__ncc_http_close_if(ptr %handle) {
entry:
  %present = icmp ne ptr %handle, null
  br i1 %present, label %close, label %done
close:
  %ignored = call i32 @WinHttpCloseHandle(ptr %handle)
  br label %done
done:
  ret void
}

define internal ptr @__ncc_http_new_response(i32 %status, i32 %error, i64 %capacity) {
entry:
  %cap_nonnegative = icmp sge i64 %capacity, 0
  %cap = select i1 %cap_nonnegative, i64 %capacity, i64 0
  %response = call ptr @__ncc_raw_aligned_alloc(i64 32, i64 8)
  %response_ok = icmp ne ptr %response, null
  br i1 %response_ok, label %body_alloc, label %fail
body_alloc:
  %body_bytes = add i64 %cap, 1
  %body = call ptr @__ncc_raw_aligned_alloc(i64 %body_bytes, i64 8)
  %body_ok = icmp ne ptr %body, null
  br i1 %body_ok, label %init, label %free_response
init:
  %error_slot = getelementptr i8, ptr %response, i64 4
  %len_slot = getelementptr i8, ptr %response, i64 8
  %cap_slot = getelementptr i8, ptr %response, i64 16
  %body_slot = getelementptr i8, ptr %response, i64 24
  store i32 %status, ptr %response, align 8
  store i32 %error, ptr %error_slot, align 4
  store i64 0, ptr %len_slot, align 8
  store i64 %cap, ptr %cap_slot, align 8
  store ptr %body, ptr %body_slot, align 8
  store i8 0, ptr %body, align 1
  ret ptr %response
free_response:
  %ignored = call i1 @__ncc_raw_aligned_free(ptr %response)
  br label %fail
fail:
  ret ptr null
}

define ptr @__ncc_http_request_raw(ptr %method, ptr %url, ptr %headers, ptr %body_data, i64 %body_bytes, i64 %timeout_ms, i64 %max_body_bytes) {
entry:
  %urlw_slot = alloca ptr, align 8
  %methodw_slot = alloca ptr, align 8
  %headersw_slot = alloca ptr, align 8
  %hostw_slot = alloca ptr, align 8
  %objectw_slot = alloca ptr, align 8
  %session_slot = alloca ptr, align 8
  %connect_slot = alloca ptr, align 8
  %request_slot = alloca ptr, align 8
  %response_slot = alloca ptr, align 8
  %error_slot = alloca i32, align 4
  %status_slot = alloca i32, align 4
  %status_bytes_slot = alloca i32, align 4
  %available_slot = alloca i32, align 4
  %read_slot = alloca i32, align 4
  %total_slot = alloca i64, align 8
  %object_pos_slot = alloca i64, align 8
  %url_parts = alloca [104 x i8], align 8
  store ptr null, ptr %urlw_slot, align 8
  store ptr null, ptr %methodw_slot, align 8
  store ptr null, ptr %headersw_slot, align 8
  store ptr null, ptr %hostw_slot, align 8
  store ptr null, ptr %objectw_slot, align 8
  store ptr null, ptr %session_slot, align 8
  store ptr null, ptr %connect_slot, align 8
  store ptr null, ptr %request_slot, align 8
  store ptr null, ptr %response_slot, align 8
  store i32 87, ptr %error_slot, align 4
  store i64 0, ptr %total_slot, align 8
  %method_ok = icmp ne ptr %method, null
  %url_ok = icmp ne ptr %url, null
  %headers_ok = icmp ne ptr %headers, null
  %required_ok0 = and i1 %method_ok, %url_ok
  %required_ok = and i1 %required_ok0, %headers_ok
  %body_len_nonnegative = icmp sge i64 %body_bytes, 0
  %body_len_fits = icmp ule i64 %body_bytes, 4294967295
  %body_len_ok = and i1 %body_len_nonnegative, %body_len_fits
  %body_present = icmp ne ptr %body_data, null
  %body_empty = icmp eq i64 %body_bytes, 0
  %body_ok = or i1 %body_present, %body_empty
  %timeout_positive = icmp sgt i64 %timeout_ms, 0
  %timeout_fits = icmp sle i64 %timeout_ms, 2147483647
  %timeout_ok = and i1 %timeout_positive, %timeout_fits
  %max_positive = icmp sgt i64 %max_body_bytes, 0
  %max_fits = icmp sle i64 %max_body_bytes, 2147483646
  %max_ok = and i1 %max_positive, %max_fits
  %args0 = and i1 %required_ok, %body_len_ok
  %args1 = and i1 %args0, %body_ok
  %args2 = and i1 %args1, %timeout_ok
  %args_ok = and i1 %args2, %max_ok
  br i1 %args_ok, label %convert_url, label %fail

convert_url:
  %urlw = call ptr @__ncc_http_utf8_to_wide(ptr %url)
  store ptr %urlw, ptr %urlw_slot, align 8
  %urlw_ok = icmp ne ptr %urlw, null
  br i1 %urlw_ok, label %convert_method, label %capture_error
convert_method:
  %methodw = call ptr @__ncc_http_utf8_to_wide(ptr %method)
  store ptr %methodw, ptr %methodw_slot, align 8
  %methodw_ok = icmp ne ptr %methodw, null
  br i1 %methodw_ok, label %convert_headers, label %capture_error
convert_headers:
  %headersw = call ptr @__ncc_http_utf8_to_wide(ptr %headers)
  store ptr %headersw, ptr %headersw_slot, align 8
  %headersw_ok = icmp ne ptr %headersw, null
  br i1 %headersw_ok, label %crack_init, label %capture_error

crack_init:
  call void @llvm.memset.p0.i64(ptr %url_parts, i8 0, i64 104, i1 false)
  store i32 104, ptr %url_parts, align 8
  %scheme_len_slot = getelementptr i8, ptr %url_parts, i64 16
  %host_len_slot = getelementptr i8, ptr %url_parts, i64 32
  %user_len_slot = getelementptr i8, ptr %url_parts, i64 48
  %password_len_slot = getelementptr i8, ptr %url_parts, i64 64
  %path_len_slot = getelementptr i8, ptr %url_parts, i64 80
  %extra_len_slot = getelementptr i8, ptr %url_parts, i64 96
  store i32 -1, ptr %scheme_len_slot, align 4
  store i32 -1, ptr %host_len_slot, align 4
  store i32 -1, ptr %user_len_slot, align 4
  store i32 -1, ptr %password_len_slot, align 4
  store i32 -1, ptr %path_len_slot, align 4
  store i32 -1, ptr %extra_len_slot, align 4
  %cracked = call i32 @WinHttpCrackUrl(ptr %urlw, i32 0, i32 0, ptr %url_parts)
  %cracked_ok = icmp ne i32 %cracked, 0
  br i1 %cracked_ok, label %validate_parts, label %capture_error

validate_parts:
  %host_len = load i32, ptr %host_len_slot, align 4
  %user_len = load i32, ptr %user_len_slot, align 4
  %password_len = load i32, ptr %password_len_slot, align 4
  %host_nonempty = icmp sgt i32 %host_len, 0
  %user_empty = icmp eq i32 %user_len, 0
  %password_empty = icmp eq i32 %password_len, 0
  %credentials_empty = and i1 %user_empty, %password_empty
  %parts_ok = and i1 %host_nonempty, %credentials_empty
  br i1 %parts_ok, label %copy_host, label %fail

copy_host:
  %host_ptr_slot = getelementptr i8, ptr %url_parts, i64 24
  %host_ptr = load ptr, ptr %host_ptr_slot, align 8
  %host_len64 = zext i32 %host_len to i64
  %host_chars = add i64 %host_len64, 1
  %host_bytes = shl i64 %host_chars, 1
  %hostw = call ptr @__ncc_raw_aligned_alloc(i64 %host_bytes, i64 8)
  store ptr %hostw, ptr %hostw_slot, align 8
  %hostw_ok = icmp ne ptr %hostw, null
  br i1 %hostw_ok, label %copy_host_bytes, label %capture_error
copy_host_bytes:
  %host_copy_bytes = shl i64 %host_len64, 1
  call void @llvm.memcpy.p0.p0.i64(ptr %hostw, ptr %host_ptr, i64 %host_copy_bytes, i1 false)
  %host_end = getelementptr i16, ptr %hostw, i64 %host_len64
  store i16 0, ptr %host_end, align 2
  br label %build_object

build_object:
  %path_ptr_slot = getelementptr i8, ptr %url_parts, i64 72
  %path_ptr = load ptr, ptr %path_ptr_slot, align 8
  %path_len = load i32, ptr %path_len_slot, align 4
  %extra_ptr_slot = getelementptr i8, ptr %url_parts, i64 88
  %extra_ptr = load ptr, ptr %extra_ptr_slot, align 8
  %extra_len = load i32, ptr %extra_len_slot, align 4
  %path_len64 = zext i32 %path_len to i64
  %extra_len64 = zext i32 %extra_len to i64
  %object_chars0 = add i64 %path_len64, %extra_len64
  %object_chars = add i64 %object_chars0, 2
  %object_bytes = shl i64 %object_chars, 1
  %objectw = call ptr @__ncc_raw_aligned_alloc(i64 %object_bytes, i64 8)
  store ptr %objectw, ptr %objectw_slot, align 8
  %objectw_ok = icmp ne ptr %objectw, null
  br i1 %objectw_ok, label %copy_path_select, label %capture_error
copy_path_select:
  %has_path = icmp sgt i32 %path_len, 0
  br i1 %has_path, label %copy_path, label %default_path
copy_path:
  %path_copy_bytes = shl i64 %path_len64, 1
  call void @llvm.memcpy.p0.p0.i64(ptr %objectw, ptr %path_ptr, i64 %path_copy_bytes, i1 false)
  store i64 %path_len64, ptr %object_pos_slot, align 8
  br label %extra_loop
default_path:
  store i16 47, ptr %objectw, align 2
  store i64 1, ptr %object_pos_slot, align 8
  br label %extra_loop

extra_loop:
  %extra_index = phi i64 [ 0, %copy_path ], [ 0, %default_path ], [ %extra_next, %extra_copy ]
  %extra_more = icmp ult i64 %extra_index, %extra_len64
  br i1 %extra_more, label %extra_check, label %object_finish
extra_check:
  %extra_char_ptr = getelementptr i16, ptr %extra_ptr, i64 %extra_index
  %extra_char = load i16, ptr %extra_char_ptr, align 2
  %is_fragment = icmp eq i16 %extra_char, 35
  br i1 %is_fragment, label %object_finish, label %extra_copy
extra_copy:
  %object_pos = load i64, ptr %object_pos_slot, align 8
  %object_char_ptr = getelementptr i16, ptr %objectw, i64 %object_pos
  store i16 %extra_char, ptr %object_char_ptr, align 2
  %object_next = add i64 %object_pos, 1
  store i64 %object_next, ptr %object_pos_slot, align 8
  %extra_next = add i64 %extra_index, 1
  br label %extra_loop
object_finish:
  %object_final = load i64, ptr %object_pos_slot, align 8
  %object_end = getelementptr i16, ptr %objectw, i64 %object_final
  store i16 0, ptr %object_end, align 2
  br label %open_session

open_session:
  %agent = getelementptr [7 x i16], ptr @__ncc_http_agent, i64 0, i64 0
  %session = call ptr @WinHttpOpen(ptr %agent, i32 4, ptr null, ptr null, i32 0)
  store ptr %session, ptr %session_slot, align 8
  %session_ok = icmp ne ptr %session, null
  br i1 %session_ok, label %set_timeouts, label %capture_error
set_timeouts:
  %timeout32 = trunc i64 %timeout_ms to i32
  %timeouts_set = call i32 @WinHttpSetTimeouts(ptr %session, i32 %timeout32, i32 %timeout32, i32 %timeout32, i32 %timeout32)
  %timeouts_ok = icmp ne i32 %timeouts_set, 0
  br i1 %timeouts_ok, label %open_connection, label %capture_error
open_connection:
  %port_slot = getelementptr i8, ptr %url_parts, i64 36
  %port = load i16, ptr %port_slot, align 2
  %connection = call ptr @WinHttpConnect(ptr %session, ptr %hostw, i16 %port, i32 0)
  store ptr %connection, ptr %connect_slot, align 8
  %connection_ok = icmp ne ptr %connection, null
  br i1 %connection_ok, label %open_request, label %capture_error
open_request:
  %scheme_slot = getelementptr i8, ptr %url_parts, i64 20
  %scheme = load i32, ptr %scheme_slot, align 4
  %secure = icmp eq i32 %scheme, 2
  %request_flags = select i1 %secure, i32 8388608, i32 0
  %request = call ptr @WinHttpOpenRequest(ptr %connection, ptr %methodw, ptr %objectw, ptr null, ptr null, ptr null, i32 %request_flags)
  store ptr %request, ptr %request_slot, align 8
  %request_ok = icmp ne ptr %request, null
  br i1 %request_ok, label %send_request, label %capture_error
send_request:
  %body_bytes32 = trunc i64 %body_bytes to i32
  %body_pointer = select i1 %body_empty, ptr null, ptr %body_data
  %sent = call i32 @WinHttpSendRequest(ptr %request, ptr %headersw, i32 -1, ptr %body_pointer, i32 %body_bytes32, i32 %body_bytes32, i64 0)
  %sent_ok = icmp ne i32 %sent, 0
  br i1 %sent_ok, label %receive_response, label %capture_error
receive_response:
  %received = call i32 @WinHttpReceiveResponse(ptr %request, ptr null)
  %received_ok = icmp ne i32 %received, 0
  br i1 %received_ok, label %query_status, label %capture_error
query_status:
  store i32 0, ptr %status_slot, align 4
  store i32 4, ptr %status_bytes_slot, align 4
  %status_queried = call i32 @WinHttpQueryHeaders(ptr %request, i32 536870931, ptr null, ptr %status_slot, ptr %status_bytes_slot, ptr null)
  %status_ok = icmp ne i32 %status_queried, 0
  br i1 %status_ok, label %allocate_response, label %capture_error
allocate_response:
  %status = load i32, ptr %status_slot, align 4
  %response = call ptr @__ncc_http_new_response(i32 %status, i32 0, i64 %max_body_bytes)
  store ptr %response, ptr %response_slot, align 8
  %response_ok = icmp ne ptr %response, null
  br i1 %response_ok, label %read_loop, label %capture_error

read_loop:
  store i32 0, ptr %available_slot, align 4
  %available_queried = call i32 @WinHttpQueryDataAvailable(ptr %request, ptr %available_slot)
  %available_ok = icmp ne i32 %available_queried, 0
  br i1 %available_ok, label %read_available, label %capture_error
read_available:
  %available = load i32, ptr %available_slot, align 4
  %has_available = icmp ne i32 %available, 0
  br i1 %has_available, label %read_bounds, label %read_done
read_bounds:
  %available64 = zext i32 %available to i64
  %total = load i64, ptr %total_slot, align 8
  %next_total = add i64 %total, %available64
  %within_cap = icmp ule i64 %next_total, %max_body_bytes
  br i1 %within_cap, label %read_chunk, label %body_too_large
body_too_large:
  store i32 122, ptr %error_slot, align 4
  br label %fail
read_chunk:
  %response_body_slot = getelementptr i8, ptr %response, i64 24
  %response_body = load ptr, ptr %response_body_slot, align 8
  %write_at = getelementptr i8, ptr %response_body, i64 %total
  store i32 0, ptr %read_slot, align 4
  %read_ok_raw = call i32 @WinHttpReadData(ptr %request, ptr %write_at, i32 %available, ptr %read_slot)
  %read_ok = icmp ne i32 %read_ok_raw, 0
  br i1 %read_ok, label %read_commit, label %capture_error
read_commit:
  %read = load i32, ptr %read_slot, align 4
  %read64 = zext i32 %read to i64
  %read_zero = icmp eq i32 %read, 0
  %committed_total = add i64 %total, %read64
  store i64 %committed_total, ptr %total_slot, align 8
  br i1 %read_zero, label %read_done, label %read_loop
read_done:
  %final_total = load i64, ptr %total_slot, align 8
  %final_body_slot = getelementptr i8, ptr %response, i64 24
  %final_body = load ptr, ptr %final_body_slot, align 8
  %terminator = getelementptr i8, ptr %final_body, i64 %final_total
  store i8 0, ptr %terminator, align 1
  %final_len_slot = getelementptr i8, ptr %response, i64 8
  store i64 %final_total, ptr %final_len_slot, align 8
  br label %success

capture_error:
  %os_error = call i32 @GetLastError()
  %nonzero_error = icmp ne i32 %os_error, 0
  %captured_error = select i1 %nonzero_error, i32 %os_error, i32 8
  store i32 %captured_error, ptr %error_slot, align 4
  br label %fail

success:
  %success_request = load ptr, ptr %request_slot, align 8
  %success_connect = load ptr, ptr %connect_slot, align 8
  %success_session = load ptr, ptr %session_slot, align 8
  call void @__ncc_http_close_if(ptr %success_request)
  call void @__ncc_http_close_if(ptr %success_connect)
  call void @__ncc_http_close_if(ptr %success_session)
  %success_urlw = load ptr, ptr %urlw_slot, align 8
  %success_methodw = load ptr, ptr %methodw_slot, align 8
  %success_headersw = load ptr, ptr %headersw_slot, align 8
  %success_hostw = load ptr, ptr %hostw_slot, align 8
  %success_objectw = load ptr, ptr %objectw_slot, align 8
  call void @__ncc_http_free_if(ptr %success_urlw)
  call void @__ncc_http_free_if(ptr %success_methodw)
  call void @__ncc_http_free_if(ptr %success_headersw)
  call void @__ncc_http_free_if(ptr %success_hostw)
  call void @__ncc_http_free_if(ptr %success_objectw)
  ret ptr %response

fail:
  %fail_response = load ptr, ptr %response_slot, align 8
  %has_fail_response = icmp ne ptr %fail_response, null
  br i1 %has_fail_response, label %release_fail_response, label %fail_cleanup
release_fail_response:
  %released = call i1 @__ncc_http_release(ptr %fail_response)
  store ptr null, ptr %response_slot, align 8
  br label %fail_cleanup
fail_cleanup:
  %fail_request = load ptr, ptr %request_slot, align 8
  %fail_connect = load ptr, ptr %connect_slot, align 8
  %fail_session = load ptr, ptr %session_slot, align 8
  call void @__ncc_http_close_if(ptr %fail_request)
  call void @__ncc_http_close_if(ptr %fail_connect)
  call void @__ncc_http_close_if(ptr %fail_session)
  %fail_urlw = load ptr, ptr %urlw_slot, align 8
  %fail_methodw = load ptr, ptr %methodw_slot, align 8
  %fail_headersw = load ptr, ptr %headersw_slot, align 8
  %fail_hostw = load ptr, ptr %hostw_slot, align 8
  %fail_objectw = load ptr, ptr %objectw_slot, align 8
  call void @__ncc_http_free_if(ptr %fail_urlw)
  call void @__ncc_http_free_if(ptr %fail_methodw)
  call void @__ncc_http_free_if(ptr %fail_headersw)
  call void @__ncc_http_free_if(ptr %fail_hostw)
  call void @__ncc_http_free_if(ptr %fail_objectw)
  %error = load i32, ptr %error_slot, align 4
  %error_response = call ptr @__ncc_http_new_response(i32 0, i32 %error, i64 0)
  ret ptr %error_response
}

define ptr @__ncc_http_get(ptr %url) {
entry:
  %method = getelementptr [4 x i8], ptr @__ncc_http_get_text, i64 0, i64 0
  %headers = getelementptr [1 x i8], ptr @__ncc_http_empty, i64 0, i64 0
  %response = call ptr @__ncc_http_request_raw(ptr %method, ptr %url, ptr %headers, ptr null, i64 0, i64 30000, i64 16777216)
  ret ptr %response
}

define internal i64 @__ncc_http_strlen(ptr %text) {
entry:
  %null = icmp eq ptr %text, null
  br i1 %null, label %done_null, label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %more ]
  %at = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %at, align 1
  %end = icmp eq i8 %byte, 0
  br i1 %end, label %done, label %more
more:
  %next = add i64 %index, 1
  br label %loop
done:
  ret i64 %index
done_null:
  ret i64 0
}

define ptr @__ncc_http_post(ptr %url, ptr %body) {
entry:
  %method = getelementptr [5 x i8], ptr @__ncc_http_post_text, i64 0, i64 0
  %headers = getelementptr [1 x i8], ptr @__ncc_http_empty, i64 0, i64 0
  %length = call i64 @__ncc_http_strlen(ptr %body)
  %response = call ptr @__ncc_http_request_raw(ptr %method, ptr %url, ptr %headers, ptr %body, i64 %length, i64 30000, i64 16777216)
  ret ptr %response
}

define i64 @__ncc_http_status(ptr %response) {
entry:
  %valid = icmp ne ptr %response, null
  br i1 %valid, label %load, label %invalid
load:
  %status = load i32, ptr %response, align 8
  %status64 = sext i32 %status to i64
  ret i64 %status64
invalid:
  ret i64 -1
}

define ptr @__ncc_http_body(ptr %response) {
entry:
  %valid = icmp ne ptr %response, null
  br i1 %valid, label %load, label %invalid
load:
  %body_slot = getelementptr i8, ptr %response, i64 24
  %body = load ptr, ptr %body_slot, align 8
  ret ptr %body
invalid:
  %empty = getelementptr [1 x i8], ptr @__ncc_http_empty, i64 0, i64 0
  ret ptr %empty
}

define ptr @__ncc_http_body_data(ptr %response) {
entry:
  %body = call ptr @__ncc_http_body(ptr %response)
  ret ptr %body
}

define i64 @__ncc_http_body_bytes(ptr %response) {
entry:
  %valid = icmp ne ptr %response, null
  br i1 %valid, label %load, label %invalid
load:
  %len_slot = getelementptr i8, ptr %response, i64 8
  %length = load i64, ptr %len_slot, align 8
  ret i64 %length
invalid:
  ret i64 -1
}

define i64 @__ncc_http_error(ptr %response) {
entry:
  %valid = icmp ne ptr %response, null
  br i1 %valid, label %load, label %invalid
load:
  %error_slot = getelementptr i8, ptr %response, i64 4
  %error = load i32, ptr %error_slot, align 4
  %error64 = sext i32 %error to i64
  ret i64 %error64
invalid:
  ret i64 -1
}

define i1 @__ncc_http_release(ptr %response) {
entry:
  %valid = icmp ne ptr %response, null
  br i1 %valid, label %release_body, label %invalid
release_body:
  %body_slot = getelementptr i8, ptr %response, i64 24
  %body = load ptr, ptr %body_slot, align 8
  call void @__ncc_http_free_if(ptr %body)
  %released = call i1 @__ncc_raw_aligned_free(ptr %response)
  ret i1 %released
invalid:
  ret i1 false
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win64_http_runtime_is_direct_winhttp_and_caller_owned() {
        let ir = render_http_runtime("x86_64-pc-windows-msvc").expect("Win64 HTTP IR");
        for symbol in [
            "@WinHttpCrackUrl(",
            "@WinHttpSendRequest(",
            "@WinHttpReadData(",
            "@__ncc_http_request_raw(",
            "@__ncc_http_body_data(",
            "@__ncc_http_release(",
        ] {
            assert!(ir.contains(symbol), "missing {symbol}");
        }
        assert!(!ir.contains("malloc("));
        assert!(!ir.contains("curl"));
        assert!(render_http_runtime("x86_64-unknown-linux-gnu").is_err());
    }
}
