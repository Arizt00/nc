pub(crate) fn render_socket_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    if lower.contains("windows") && (lower.contains("x86_64") || lower.contains("aarch64")) {
        return Ok(WIN64_SOCKET_RUNTIME.to_string());
    }
    let posix_64 = (lower.contains("linux")
        || lower.contains("android")
        || lower.contains("darwin")
        || lower.contains("apple"))
        && (lower.contains("x86_64") || lower.contains("aarch64") || lower.contains("riscv64"));
    if posix_64 {
        let apple = lower.contains("darwin") || lower.contains("apple");
        let errno_symbol = if apple {
            "__error"
        } else if lower.contains("android") {
            "__errno"
        } else {
            "__errno_location"
        };
        let rendered = POSIX64_SOCKET_RUNTIME
            .replace("@ERRNO_SYMBOL@", errno_symbol)
            .replace("@SOL_SOCKET@", if apple { "65535" } else { "1" })
            .replace("@SO_RCVTIMEO@", if apple { "4102" } else { "20" })
            .replace("@SO_SNDTIMEO@", if apple { "4101" } else { "21" })
            .replace("@O_NONBLOCK@", if apple { "4" } else { "2048" })
            .replace("@EINPROGRESS@", if apple { "36" } else { "115" })
            .replace("@AF_INET6@", if apple { "30" } else { "10" })
            .replace("@COUNT_FAMILY_LOAD@", if apple {
                "%count.family.byte.slot = getelementptr i8, ptr %address, i64 1\n  %count.family.byte = load i8, ptr %count.family.byte.slot, align 1\n  %count.family.value = zext i8 %count.family.byte to i16"
            } else {
                "%count.family.value = load i16, ptr %address, align 2"
            })
            .replace("@WRITE_FAMILY_LOAD@", if apple {
                "%write.family.byte.slot = getelementptr i8, ptr %write.address, i64 1\n  %write.family.byte = load i8, ptr %write.family.byte.slot, align 1\n  %write.family.value = zext i8 %write.family.byte to i16"
            } else {
                "%write.family.value = load i16, ptr %write.address, align 2"
            });
        return Ok(rendered);
    }
    Err(format!(
        "native raw socket backend has no material ABI for target '{target}'"
    ))
}

const WIN64_SOCKET_RUNTIME: &str = r#"
declare i32 @WSAStartup(i16, ptr)
declare i32 @WSACleanup()
declare i32 @WSAGetLastError()
declare i64 @socket(i32, i32, i32)
declare i32 @connect(i64, ptr, i32)
declare i32 @bind(i64, ptr, i32)
declare i32 @listen(i64, i32)
declare i64 @accept(i64, ptr, ptr)
declare i32 @send(i64, ptr, i32, i32)
declare i32 @recv(i64, ptr, i32, i32)
declare i32 @shutdown(i64, i32)
declare i32 @closesocket(i64)
declare i32 @setsockopt(i64, i32, i32, ptr, i32)
declare i32 @getsockopt(i64, i32, i32, ptr, ptr)
declare i32 @ioctlsocket(i64, i64, ptr)
declare i32 @WSAPoll(ptr, i32, i32)
declare i16 @htons(i16)
declare i32 @getaddrinfo(ptr, ptr, ptr, ptr)
declare void @freeaddrinfo(ptr)
declare i32 @GetAdaptersAddresses(i32, i32, ptr, ptr, ptr)

define i64 @__ncc_net_startup() {
entry:
  %data = alloca [512 x i8], align 16
  %status = call i32 @WSAStartup(i16 514, ptr %data)
  %wide = sext i32 %status to i64
  ret i64 %wide
}

define i64 @__ncc_net_shutdown() {
entry:
  %status = call i32 @WSACleanup()
  %wide = sext i32 %status to i64
  ret i64 %wide
}

define i64 @__ncc_net_last_error() {
entry:
  %error = call i32 @WSAGetLastError()
  %wide = sext i32 %error to i64
  ret i64 %wide
}

define i1 @__ncc_net_available() {
entry:
  %status = call i64 @__ncc_net_startup()
  %ok = icmp eq i64 %status, 0
  br i1 %ok, label %cleanup, label %done
cleanup:
  %ignored = call i64 @__ncc_net_shutdown()
  br label %done
done:
  ret i1 %ok
}

define internal i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %out, i64 %capacity) {
entry:
  %hints = alloca [48 x i8], align 8
  %result_slot = alloca ptr, align 8
  store ptr null, ptr %result_slot, align 8
  %host_ok = icmp ne ptr %host, null
  %out_ok = icmp ne ptr %out, null
  %port_low = icmp sge i64 %port, 0
  %port_high = icmp sle i64 %port, 65535
  %port_ok = and i1 %port_low, %port_high
  %cap_low = icmp sgt i64 %capacity, 0
  %cap_high = icmp sle i64 %capacity, 2147483647
  %cap_ok = and i1 %cap_low, %cap_high
  %args0 = and i1 %host_ok, %out_ok
  %args1 = and i1 %port_ok, %cap_ok
  %args_ok = and i1 %args0, %args1
  br i1 %args_ok, label %resolve, label %bad_args
resolve:
  call void @llvm.memset.p0.i64(ptr %hints, i8 0, i64 48, i1 false)
  %resolved = call i32 @getaddrinfo(ptr %host, ptr null, ptr %hints, ptr %result_slot)
  %resolve_ok = icmp eq i32 %resolved, 0
  br i1 %resolve_ok, label %first, label %resolve_fail
first:
  %result = load ptr, ptr %result_slot, align 8
  %present = icmp ne ptr %result, null
  br i1 %present, label %validate, label %empty
validate:
  %addrlen_slot = getelementptr i8, ptr %result, i64 16
  %addr_slot = getelementptr i8, ptr %result, i64 32
  %addrlen = load i64, ptr %addrlen_slot, align 8
  %address = load ptr, ptr %addr_slot, align 8
  %len_positive = icmp sgt i64 %addrlen, 0
  %len_fits = icmp ule i64 %addrlen, %capacity
  %address_ok = icmp ne ptr %address, null
  %valid0 = and i1 %len_positive, %len_fits
  %valid = and i1 %valid0, %address_ok
  br i1 %valid, label %set_port, label %too_small
set_port:
  %port16 = trunc i64 %port to i16
  %network_port = call i16 @htons(i16 %port16)
  %port_slot = getelementptr i8, ptr %address, i64 2
  store i16 %network_port, ptr %port_slot, align 2
  br label %copy_loop
copy_loop:
  %index = phi i64 [ 0, %set_port ], [ %next, %copy_byte ]
  %more = icmp ult i64 %index, %addrlen
  br i1 %more, label %copy_byte, label %success
copy_byte:
  %src = getelementptr i8, ptr %address, i64 %index
  %dst = getelementptr i8, ptr %out, i64 %index
  %byte = load i8, ptr %src, align 1
  store i8 %byte, ptr %dst, align 1
  %next = add i64 %index, 1
  br label %copy_loop
success:
  call void @freeaddrinfo(ptr %result)
  ret i64 %addrlen
too_small:
  call void @freeaddrinfo(ptr %result)
  %negative_len = sub i64 0, %addrlen
  ret i64 %negative_len
empty:
  ret i64 -11001
resolve_fail:
  %resolved64 = sext i32 %resolved to i64
  %negative_error = sub i64 0, %resolved64
  ret i64 %negative_error
bad_args:
  ret i64 -10022
}

define i64 @__ncc_net_resolve(ptr %host, i64 %port, ptr %out, i64 %capacity) {
entry:
  %result = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %out, i64 %capacity)
  ret i64 %result
}

define i64 @__ncc_net_address_of(ptr %host, i64 %port, ptr %out, i64 %capacity) {
entry:
  %result = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %out, i64 %capacity)
  ret i64 %result
}

define i64 @__ncc_net_interfaces(ptr %out, i64 %capacity) {
entry:
  %size = alloca i32, align 4
  %out_ok = icmp ne ptr %out, null
  %cap_low = icmp sge i64 %capacity, 0
  %cap_high = icmp ule i64 %capacity, 4294967295
  %cap_ok = and i1 %cap_low, %cap_high
  %args_ok = and i1 %out_ok, %cap_ok
  br i1 %args_ok, label %query, label %bad_args
query:
  %cap32 = trunc i64 %capacity to i32
  store i32 %cap32, ptr %size, align 4
  %status = call i32 @GetAdaptersAddresses(i32 0, i32 16, ptr null, ptr %out, ptr %size)
  %used32 = load i32, ptr %size, align 4
  %used = zext i32 %used32 to i64
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  ret i64 %used
failed:
  %overflow = icmp eq i32 %status, 111
  br i1 %overflow, label %required, label %error
required:
  %negative_required = sub i64 0, %used
  ret i64 %negative_required
error:
  %status64 = zext i32 %status to i64
  %negative_status = sub i64 0, %status64
  ret i64 %negative_status
bad_args:
  ret i64 -87
}

define ptr @__ncc_socket_create(i64 %family, i64 %kind, i64 %protocol) {
entry:
  %family32 = trunc i64 %family to i32
  %kind32 = trunc i64 %kind to i32
  %protocol32 = trunc i64 %protocol to i32
  %raw = call i64 @socket(i32 %family32, i32 %kind32, i32 %protocol32)
  %valid = icmp ne i64 %raw, -1
  %handle = inttoptr i64 %raw to ptr
  %result = select i1 %valid, ptr %handle, ptr null
  ret ptr %result
}

define i64 @__ncc_socket_set_timeout(ptr %handle, i64 %receive_ms, i64 %send_ms) {
entry:
  %receive = alloca i32, align 4
  %send_time = alloca i32, align 4
  %present = icmp ne ptr %handle, null
  %receive_low = icmp sge i64 %receive_ms, 0
  %receive_high = icmp sle i64 %receive_ms, 2147483647
  %send_low = icmp sge i64 %send_ms, 0
  %send_high = icmp sle i64 %send_ms, 2147483647
  %receive_ok = and i1 %receive_low, %receive_high
  %send_ok = and i1 %send_low, %send_high
  %times_ok = and i1 %receive_ok, %send_ok
  %args_ok = and i1 %present, %times_ok
  br i1 %args_ok, label %set_receive, label %bad
set_receive:
  %raw = ptrtoint ptr %handle to i64
  %receive32 = trunc i64 %receive_ms to i32
  %send32 = trunc i64 %send_ms to i32
  store i32 %receive32, ptr %receive, align 4
  store i32 %send32, ptr %send_time, align 4
  %receive_status = call i32 @setsockopt(i64 %raw, i32 65535, i32 4102, ptr %receive, i32 4)
  %receive_set = icmp eq i32 %receive_status, 0
  br i1 %receive_set, label %set_send, label %failed
set_send:
  %send_status = call i32 @setsockopt(i64 %raw, i32 65535, i32 4101, ptr %send_time, i32 4)
  %send_set = icmp eq i32 %send_status, 0
  br i1 %send_set, label %ok, label %failed
ok:
  ret i64 0
failed:
  %error = call i32 @WSAGetLastError()
  %error64 = sext i32 %error to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
bad:
  ret i64 -10022
}

define i64 @__ncc_socket_connect(ptr %handle, ptr %host, i64 %port, i64 %timeout_ms) {
entry:
  %address = alloca [128 x i8], align 16
  %mode = alloca i32, align 4
  %pollfd = alloca [16 x i8], align 8
  %socket_error = alloca i32, align 4
  %socket_error_len = alloca i32, align 4
  %present = icmp ne ptr %handle, null
  %timeout_low = icmp sge i64 %timeout_ms, 0
  %timeout_high = icmp sle i64 %timeout_ms, 2147483647
  %timeout_ok = and i1 %timeout_low, %timeout_high
  %args_ok = and i1 %present, %timeout_ok
  br i1 %args_ok, label %resolve, label %bad
resolve:
  %addrlen = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %address, i64 128)
  %resolved = icmp sgt i64 %addrlen, 0
  br i1 %resolved, label %nonblocking, label %resolve_fail
nonblocking:
  %raw = ptrtoint ptr %handle to i64
  store i32 1, ptr %mode, align 4
  %nonblock_status = call i32 @ioctlsocket(i64 %raw, i64 2147772030, ptr %mode)
  %nonblock_ok = icmp eq i32 %nonblock_status, 0
  br i1 %nonblock_ok, label %try_connect, label %failed
try_connect:
  %addrlen32 = trunc i64 %addrlen to i32
  %connected = call i32 @connect(i64 %raw, ptr %address, i32 %addrlen32)
  %immediate = icmp eq i32 %connected, 0
  br i1 %immediate, label %restore_ok, label %connect_error
connect_error:
  %error = call i32 @WSAGetLastError()
  %would_block = icmp eq i32 %error, 10035
  br i1 %would_block, label %poll_init, label %restore_error
poll_init:
  call void @llvm.memset.p0.i64(ptr %pollfd, i8 0, i64 16, i1 false)
  store i64 %raw, ptr %pollfd, align 8
  %events = getelementptr i8, ptr %pollfd, i64 8
  store i16 16, ptr %events, align 2
  %timeout32 = trunc i64 %timeout_ms to i32
  %polled = call i32 @WSAPoll(ptr %pollfd, i32 1, i32 %timeout32)
  %poll_ready = icmp sgt i32 %polled, 0
  br i1 %poll_ready, label %socket_error_query, label %poll_failed
socket_error_query:
  store i32 0, ptr %socket_error, align 4
  store i32 4, ptr %socket_error_len, align 4
  %queried = call i32 @getsockopt(i64 %raw, i32 65535, i32 4103, ptr %socket_error, ptr %socket_error_len)
  %query_ok = icmp eq i32 %queried, 0
  br i1 %query_ok, label %socket_error_check, label %restore_failed
socket_error_check:
  %completion_error = load i32, ptr %socket_error, align 4
  %completed = icmp eq i32 %completion_error, 0
  br i1 %completed, label %restore_ok, label %restore_completion_error
poll_failed:
  %timed_out = icmp eq i32 %polled, 0
  %poll_error = call i32 @WSAGetLastError()
  %selected_error = select i1 %timed_out, i32 10060, i32 %poll_error
  br label %restore_selected_error
restore_ok:
  store i32 0, ptr %mode, align 4
  %restore0 = call i32 @ioctlsocket(i64 %raw, i64 2147772030, ptr %mode)
  ret i64 0
restore_error:
  store i32 0, ptr %mode, align 4
  %restore1 = call i32 @ioctlsocket(i64 %raw, i64 2147772030, ptr %mode)
  %error64 = sext i32 %error to i64
  %negative_error = sub i64 0, %error64
  ret i64 %negative_error
restore_completion_error:
  store i32 0, ptr %mode, align 4
  %restore2 = call i32 @ioctlsocket(i64 %raw, i64 2147772030, ptr %mode)
  %completion64 = sext i32 %completion_error to i64
  %negative_completion = sub i64 0, %completion64
  ret i64 %negative_completion
restore_selected_error:
  store i32 0, ptr %mode, align 4
  %restore3 = call i32 @ioctlsocket(i64 %raw, i64 2147772030, ptr %mode)
  %selected64 = sext i32 %selected_error to i64
  %negative_selected = sub i64 0, %selected64
  ret i64 %negative_selected
restore_failed:
  store i32 0, ptr %mode, align 4
  %restore4 = call i32 @ioctlsocket(i64 %raw, i64 2147772030, ptr %mode)
  br label %failed
resolve_fail:
  ret i64 %addrlen
failed:
  %late_error = call i32 @WSAGetLastError()
  %late64 = sext i32 %late_error to i64
  %negative_late = sub i64 0, %late64
  ret i64 %negative_late
bad:
  ret i64 -10022
}

define i64 @__ncc_socket_bind(ptr %handle, ptr %host, i64 %port) {
entry:
  %address = alloca [128 x i8], align 16
  %present = icmp ne ptr %handle, null
  br i1 %present, label %resolve, label %bad
resolve:
  %addrlen = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %address, i64 128)
  %resolved = icmp sgt i64 %addrlen, 0
  br i1 %resolved, label %bind_now, label %resolve_fail
bind_now:
  %raw = ptrtoint ptr %handle to i64
  %addrlen32 = trunc i64 %addrlen to i32
  %status = call i32 @bind(i64 %raw, ptr %address, i32 %addrlen32)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  ret i64 0
failed:
  %error = call i32 @WSAGetLastError()
  %error64 = sext i32 %error to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
resolve_fail:
  ret i64 %addrlen
bad:
  ret i64 -10022
}

define i64 @__ncc_socket_listen(ptr %handle, i64 %backlog) {
entry:
  %present = icmp ne ptr %handle, null
  %low = icmp sge i64 %backlog, 0
  %high = icmp sle i64 %backlog, 2147483647
  %range = and i1 %low, %high
  %ok_args = and i1 %present, %range
  br i1 %ok_args, label %call, label %bad
call:
  %raw = ptrtoint ptr %handle to i64
  %backlog32 = trunc i64 %backlog to i32
  %status = call i32 @listen(i64 %raw, i32 %backlog32)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  ret i64 0
failed:
  %error = call i32 @WSAGetLastError()
  %error64 = sext i32 %error to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
bad:
  ret i64 -10022
}

define ptr @__ncc_socket_accept(ptr %handle, ptr %out_address, i64 %capacity) {
entry:
  %length = alloca i32, align 4
  %present = icmp ne ptr %handle, null
  %out_present = icmp ne ptr %out_address, null
  %cap_low = icmp sgt i64 %capacity, 0
  %cap_high = icmp sle i64 %capacity, 2147483647
  %cap_ok = and i1 %cap_low, %cap_high
  %out_ok = and i1 %out_present, %cap_ok
  %args_ok = and i1 %present, %out_ok
  br i1 %args_ok, label %call, label %bad
call:
  %raw = ptrtoint ptr %handle to i64
  %cap32 = trunc i64 %capacity to i32
  store i32 %cap32, ptr %length, align 4
  %accepted = call i64 @accept(i64 %raw, ptr %out_address, ptr %length)
  %valid = icmp ne i64 %accepted, -1
  %accepted_ptr = inttoptr i64 %accepted to ptr
  %result = select i1 %valid, ptr %accepted_ptr, ptr null
  ret ptr %result
bad:
  ret ptr null
}

define i64 @__ncc_socket_send(ptr %handle, ptr %data, i64 %bytes, i64 %flags) {
entry:
  %present = icmp ne ptr %handle, null
  %data_present = icmp ne ptr %data, null
  %empty = icmp eq i64 %bytes, 0
  %data_ok = or i1 %data_present, %empty
  %bytes_low = icmp sge i64 %bytes, 0
  %bytes_high = icmp sle i64 %bytes, 2147483647
  %bytes_ok = and i1 %bytes_low, %bytes_high
  %flags_fit = icmp sle i64 %flags, 2147483647
  %args0 = and i1 %present, %data_ok
  %args1 = and i1 %bytes_ok, %flags_fit
  %args_ok = and i1 %args0, %args1
  br i1 %args_ok, label %call, label %bad
call:
  %raw = ptrtoint ptr %handle to i64
  %bytes32 = trunc i64 %bytes to i32
  %flags32 = trunc i64 %flags to i32
  %sent = call i32 @send(i64 %raw, ptr %data, i32 %bytes32, i32 %flags32)
  %sent64 = sext i32 %sent to i64
  ret i64 %sent64
bad:
  ret i64 -1
}

define i64 @__ncc_socket_recv(ptr %handle, ptr %data, i64 %capacity, i64 %flags) {
entry:
  %present = icmp ne ptr %handle, null
  %data_present = icmp ne ptr %data, null
  %capacity_low = icmp sge i64 %capacity, 0
  %capacity_high = icmp sle i64 %capacity, 2147483647
  %capacity_ok = and i1 %capacity_low, %capacity_high
  %flags_fit = icmp sle i64 %flags, 2147483647
  %args0 = and i1 %present, %data_present
  %args1 = and i1 %capacity_ok, %flags_fit
  %args_ok = and i1 %args0, %args1
  br i1 %args_ok, label %call, label %bad
call:
  %raw = ptrtoint ptr %handle to i64
  %capacity32 = trunc i64 %capacity to i32
  %flags32 = trunc i64 %flags to i32
  %received = call i32 @recv(i64 %raw, ptr %data, i32 %capacity32, i32 %flags32)
  %received64 = sext i32 %received to i64
  ret i64 %received64
bad:
  ret i64 -1
}

define i64 @__ncc_socket_shutdown(ptr %handle, i64 %how) {
entry:
  %present = icmp ne ptr %handle, null
  br i1 %present, label %call, label %bad
call:
  %raw = ptrtoint ptr %handle to i64
  %how32 = trunc i64 %how to i32
  %status = call i32 @shutdown(i64 %raw, i32 %how32)
  %status64 = sext i32 %status to i64
  ret i64 %status64
bad:
  ret i64 -1
}

define i64 @__ncc_socket_close(ptr %handle) {
entry:
  %present = icmp ne ptr %handle, null
  br i1 %present, label %call, label %bad
call:
  %raw = ptrtoint ptr %handle to i64
  %status = call i32 @closesocket(i64 %raw)
  %status64 = sext i32 %status to i64
  ret i64 %status64
bad:
  ret i64 -1
}

define i64 @__ncc_socket_last_error() {
entry:
  %error = call i32 @WSAGetLastError()
  %wide = sext i32 %error to i64
  ret i64 %wide
}
"#;

const POSIX64_SOCKET_RUNTIME: &str = r#"
declare i32 @socket(i32, i32, i32)
declare i32 @connect(i32, ptr, i32)
declare i32 @bind(i32, ptr, i32)
declare i32 @listen(i32, i32)
declare i32 @accept(i32, ptr, ptr)
declare i64 @send(i32, ptr, i64, i32)
declare i64 @recv(i32, ptr, i64, i32)
declare i32 @shutdown(i32, i32)
declare i32 @close(i32)
declare i32 @setsockopt(i32, i32, i32, ptr, i32)
declare i32 @getsockopt(i32, i32, i32, ptr, ptr)
declare i32 @fcntl(i32, i32, ...)
declare i32 @poll(ptr, i64, i32)
declare i16 @htons(i16)
declare i32 @getaddrinfo(ptr, ptr, ptr, ptr)
declare void @freeaddrinfo(ptr)
declare i32 @getifaddrs(ptr)
declare void @freeifaddrs(ptr)
declare ptr @@ERRNO_SYMBOL@()

define internal i32 @__ncc_socket_errno() {
entry:
  %slot = call ptr @@ERRNO_SYMBOL@()
  %value = load i32, ptr %slot, align 4
  ret i32 %value
}

define i64 @__ncc_net_startup() {
entry:
  ret i64 0
}

define i64 @__ncc_net_shutdown() {
entry:
  ret i64 0
}

define i64 @__ncc_net_last_error() {
entry:
  %error = call i32 @__ncc_socket_errno()
  %wide = sext i32 %error to i64
  ret i64 %wide
}

define i1 @__ncc_net_available() {
entry:
  %fd = call i32 @socket(i32 2, i32 1, i32 6)
  %ok = icmp sge i32 %fd, 0
  br i1 %ok, label %close, label %done
close:
  %ignored = call i32 @close(i32 %fd)
  br label %done
done:
  ret i1 %ok
}

define internal i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %out, i64 %capacity) {
entry:
  %hints = alloca [48 x i8], align 8
  %result.slot = alloca ptr, align 8
  store ptr null, ptr %result.slot, align 8
  %host.ok = icmp ne ptr %host, null
  %out.ok = icmp ne ptr %out, null
  %port.low = icmp sge i64 %port, 0
  %port.high = icmp sle i64 %port, 65535
  %port.ok = and i1 %port.low, %port.high
  %cap.ok = icmp uge i64 %capacity, 28
  %args0 = and i1 %host.ok, %out.ok
  %args1 = and i1 %port.ok, %cap.ok
  %args.ok = and i1 %args0, %args1
  br i1 %args.ok, label %resolve, label %bad
resolve:
  call void @llvm.memset.p0.i64(ptr %hints, i8 0, i64 48, i1 false)
  %socktype.slot = getelementptr i8, ptr %hints, i64 8
  %protocol.slot = getelementptr i8, ptr %hints, i64 12
  store i32 1, ptr %socktype.slot, align 4
  store i32 6, ptr %protocol.slot, align 4
  %status = call i32 @getaddrinfo(ptr %host, ptr null, ptr %hints, ptr %result.slot)
  %resolved = icmp eq i32 %status, 0
  br i1 %resolved, label %first, label %resolve.failed
first:
  %result = load ptr, ptr %result.slot, align 8
  %present = icmp ne ptr %result, null
  br i1 %present, label %read, label %empty
read:
  %length.slot = getelementptr i8, ptr %result, i64 16
  %address.slot = getelementptr i8, ptr %result, i64 24
  %length32 = load i32, ptr %length.slot, align 4
  %length = zext i32 %length32 to i64
  %address = load ptr, ptr %address.slot, align 8
  %fits = icmp ule i64 %length, %capacity
  %address.ok = icmp ne ptr %address, null
  %valid = and i1 %fits, %address.ok
  br i1 %valid, label %set.port, label %small
set.port:
  %port16 = trunc i64 %port to i16
  %network.port = call i16 @htons(i16 %port16)
  %port.slot = getelementptr i8, ptr %address, i64 2
  store i16 %network.port, ptr %port.slot, align 2
  br label %copy.loop
copy.loop:
  %index = phi i64 [ 0, %set.port ], [ %next, %copy.byte ]
  %more = icmp ult i64 %index, %length
  br i1 %more, label %copy.byte, label %success
copy.byte:
  %source = getelementptr i8, ptr %address, i64 %index
  %target = getelementptr i8, ptr %out, i64 %index
  %byte = load i8, ptr %source, align 1
  store i8 %byte, ptr %target, align 1
  %next = add i64 %index, 1
  br label %copy.loop
success:
  call void @freeaddrinfo(ptr %result)
  ret i64 %length
small:
  call void @freeaddrinfo(ptr %result)
  %negative = sub i64 0, %length
  ret i64 %negative
empty:
  ret i64 -2
resolve.failed:
  %status64 = sext i32 %status to i64
  %negative.status = sub i64 0, %status64
  ret i64 %negative.status
bad:
  ret i64 -22
}

define i64 @__ncc_net_resolve(ptr %host, i64 %port, ptr %out, i64 %capacity) {
entry:
  %result = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %out, i64 %capacity)
  ret i64 %result
}

define i64 @__ncc_net_address_of(ptr %host, i64 %port, ptr %out, i64 %capacity) {
entry:
  %result = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %out, i64 %capacity)
  ret i64 %result
}

define internal i64 @__ncc_interface_address_length(i16 %family) {
entry:
  %ipv4 = icmp eq i16 %family, 2
  %ipv6 = icmp eq i16 %family, @AF_INET6@
  %v6.length = select i1 %ipv6, i64 28, i64 0
  %result = select i1 %ipv4, i64 16, i64 %v6.length
  ret i64 %result
}

define i64 @__ncc_net_interfaces(ptr %out, i64 %capacity) {
entry:
  %head.slot = alloca ptr, align 8
  store ptr null, ptr %head.slot, align 8
  %cap.ok = icmp sge i64 %capacity, 0
  br i1 %cap.ok, label %query, label %bad
query:
  %status = call i32 @getifaddrs(ptr %head.slot)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %count, label %failed
count:
  %head = load ptr, ptr %head.slot, align 8
  br label %count.loop
count.loop:
  %node = phi ptr [ %head, %count ], [ %next, %count.step ]
  %records = phi i64 [ 0, %count ], [ %records.next, %count.step ]
  %present = icmp ne ptr %node, null
  br i1 %present, label %count.read, label %capacity.check
count.read:
  %address.slot = getelementptr i8, ptr %node, i64 24
  %address = load ptr, ptr %address.slot, align 8
  %has.address = icmp ne ptr %address, null
  br i1 %has.address, label %count.family, label %count.skip
count.family:
  @COUNT_FAMILY_LOAD@
  %address.length = call i64 @__ncc_interface_address_length(i16 %count.family.value)
  %supported = icmp ne i64 %address.length, 0
  %increment = select i1 %supported, i64 1, i64 0
  br label %count.step
count.skip:
  br label %count.step
count.step:
  %record.increment = phi i64 [ %increment, %count.family ], [ 0, %count.skip ]
  %records.next = add i64 %records, %record.increment
  %next = load ptr, ptr %node, align 8
  br label %count.loop
capacity.check:
  %required = mul i64 %records, 144
  %out.present = icmp ne ptr %out, null
  %fits = icmp uge i64 %capacity, %required
  %can.write = and i1 %out.present, %fits
  %none = icmp eq i64 %required, 0
  br i1 %none, label %release.success, label %write.check
write.check:
  br i1 %can.write, label %write, label %release.required
write:
  br label %write.loop
write.loop:
  %write.node = phi ptr [ %head, %write ], [ %write.next, %write.step ]
  %write.index = phi i64 [ 0, %write ], [ %write.index.next, %write.step ]
  %write.present = icmp ne ptr %write.node, null
  br i1 %write.present, label %write.read, label %release.success
write.read:
  %write.flags.slot = getelementptr i8, ptr %write.node, i64 16
  %write.address.slot = getelementptr i8, ptr %write.node, i64 24
  %write.flags = load i32, ptr %write.flags.slot, align 4
  %write.address = load ptr, ptr %write.address.slot, align 8
  %write.has.address = icmp ne ptr %write.address, null
  br i1 %write.has.address, label %write.family, label %write.skip
write.family:
  @WRITE_FAMILY_LOAD@
  %write.address.length = call i64 @__ncc_interface_address_length(i16 %write.family.value)
  %write.supported = icmp ne i64 %write.address.length, 0
  br i1 %write.supported, label %write.record, label %write.skip
write.record:
  %record.offset = mul i64 %write.index, 144
  %record = getelementptr i8, ptr %out, i64 %record.offset
  %family.slot = getelementptr i8, ptr %record, i64 4
  %length.slot = getelementptr i8, ptr %record, i64 8
  %payload = getelementptr i8, ptr %record, i64 16
  %family32 = zext i16 %write.family.value to i32
  store i32 %write.flags, ptr %record, align 4
  store i32 %family32, ptr %family.slot, align 4
  store i64 %write.address.length, ptr %length.slot, align 8
  br label %write.copy.loop
write.copy.loop:
  %byte.index = phi i64 [ 0, %write.record ], [ %byte.next, %write.copy ]
  %byte.more = icmp ult i64 %byte.index, %write.address.length
  br i1 %byte.more, label %write.copy, label %write.copied
write.copy:
  %source = getelementptr i8, ptr %write.address, i64 %byte.index
  %target = getelementptr i8, ptr %payload, i64 %byte.index
  %byte = load i8, ptr %source, align 1
  store i8 %byte, ptr %target, align 1
  %byte.next = add i64 %byte.index, 1
  br label %write.copy.loop
write.copied:
  br label %write.step
write.skip:
  br label %write.step
write.step:
  %index.increment = phi i64 [ 1, %write.copied ], [ 0, %write.skip ]
  %write.index.next = add i64 %write.index, %index.increment
  %write.next = load ptr, ptr %write.node, align 8
  br label %write.loop
release.success:
  call void @freeifaddrs(ptr %head)
  ret i64 %required
release.required:
  call void @freeifaddrs(ptr %head)
  %negative.required = sub i64 0, %required
  ret i64 %negative.required
failed:
  %error = call i32 @__ncc_socket_errno()
  %error64 = sext i32 %error to i64
  %negative.error = sub i64 0, %error64
  ret i64 %negative.error
bad:
  ret i64 -22
}

define ptr @__ncc_socket_create(i64 %family, i64 %kind, i64 %protocol) {
entry:
  %family32 = trunc i64 %family to i32
  %kind32 = trunc i64 %kind to i32
  %protocol32 = trunc i64 %protocol to i32
  %fd = call i32 @socket(i32 %family32, i32 %kind32, i32 %protocol32)
  %valid = icmp sge i32 %fd, 0
  %wide = sext i32 %fd to i64
  %handle = inttoptr i64 %wide to ptr
  %result = select i1 %valid, ptr %handle, ptr null
  ret ptr %result
}

define i64 @__ncc_socket_set_timeout(ptr %handle, i64 %receive.ms, i64 %send.ms) {
entry:
  %receive.timeval = alloca [16 x i8], align 8
  %send.timeval = alloca [16 x i8], align 8
  %present = icmp ne ptr %handle, null
  %receive.ok = icmp sge i64 %receive.ms, 0
  %send.ok = icmp sge i64 %send.ms, 0
  %times.ok = and i1 %receive.ok, %send.ok
  %valid = and i1 %present, %times.ok
  br i1 %valid, label %prepare, label %bad
prepare:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %receive.seconds = sdiv i64 %receive.ms, 1000
  %receive.remainder = srem i64 %receive.ms, 1000
  %receive.usec = mul i64 %receive.remainder, 1000
  %send.seconds = sdiv i64 %send.ms, 1000
  %send.remainder = srem i64 %send.ms, 1000
  %send.usec = mul i64 %send.remainder, 1000
  %receive.usec.slot = getelementptr i8, ptr %receive.timeval, i64 8
  %send.usec.slot = getelementptr i8, ptr %send.timeval, i64 8
  store i64 %receive.seconds, ptr %receive.timeval, align 8
  store i64 %receive.usec, ptr %receive.usec.slot, align 8
  store i64 %send.seconds, ptr %send.timeval, align 8
  store i64 %send.usec, ptr %send.usec.slot, align 8
  %receive.status = call i32 @setsockopt(i32 %fd, i32 @SOL_SOCKET@, i32 @SO_RCVTIMEO@, ptr %receive.timeval, i32 16)
  %receive.set = icmp eq i32 %receive.status, 0
  br i1 %receive.set, label %set.send, label %failed
set.send:
  %send.status = call i32 @setsockopt(i32 %fd, i32 @SOL_SOCKET@, i32 @SO_SNDTIMEO@, ptr %send.timeval, i32 16)
  %send.set = icmp eq i32 %send.status, 0
  br i1 %send.set, label %success, label %failed
success:
  ret i64 0
failed:
  %error = call i32 @__ncc_socket_errno()
  %error64 = sext i32 %error to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
bad:
  ret i64 -22
}

define i64 @__ncc_socket_connect(ptr %handle, ptr %host, i64 %port, i64 %timeout.ms) {
entry:
  %address = alloca [128 x i8], align 16
  %pollfd = alloca [8 x i8], align 4
  %socket.error = alloca i32, align 4
  %socket.error.length = alloca i32, align 4
  %present = icmp ne ptr %handle, null
  %host.present = icmp ne ptr %host, null
  %timeout.ok = icmp sge i64 %timeout.ms, 0
  %args0 = and i1 %present, %host.present
  %args.ok = and i1 %args0, %timeout.ok
  br i1 %args.ok, label %resolve, label %bad
resolve:
  %length = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %address, i64 128)
  %resolved = icmp sgt i64 %length, 0
  br i1 %resolved, label %nonblocking, label %resolve.failed
nonblocking:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %old.flags = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 3)
  %got.flags = icmp sge i32 %old.flags, 0
  br i1 %got.flags, label %set.nonblocking, label %failed
set.nonblocking:
  %new.flags = or i32 %old.flags, @O_NONBLOCK@
  %set.status = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 4, i32 %new.flags)
  %set.ok = icmp eq i32 %set.status, 0
  br i1 %set.ok, label %connect, label %failed
connect:
  %length32 = trunc i64 %length to i32
  %status = call i32 @connect(i32 %fd, ptr %address, i32 %length32)
  %immediate = icmp eq i32 %status, 0
  br i1 %immediate, label %restore.success, label %connect.error
connect.error:
  %error = call i32 @__ncc_socket_errno()
  %in.progress = icmp eq i32 %error, @EINPROGRESS@
  br i1 %in.progress, label %poll.prepare, label %restore.error
poll.prepare:
  store i32 %fd, ptr %pollfd, align 4
  %events.slot = getelementptr i8, ptr %pollfd, i64 4
  %revents.slot = getelementptr i8, ptr %pollfd, i64 6
  store i16 4, ptr %events.slot, align 2
  store i16 0, ptr %revents.slot, align 2
  %timeout32 = trunc i64 %timeout.ms to i32
  %polled = call i32 @poll(ptr %pollfd, i64 1, i32 %timeout32)
  %ready = icmp sgt i32 %polled, 0
  br i1 %ready, label %query.error, label %poll.failed
query.error:
  store i32 0, ptr %socket.error, align 4
  store i32 4, ptr %socket.error.length, align 4
  %queried = call i32 @getsockopt(i32 %fd, i32 @SOL_SOCKET@, i32 4, ptr %socket.error, ptr %socket.error.length)
  %query.ok = icmp eq i32 %queried, 0
  br i1 %query.ok, label %completion, label %restore.failed
completion:
  %completion.error = load i32, ptr %socket.error, align 4
  %completed = icmp eq i32 %completion.error, 0
  br i1 %completed, label %restore.success, label %restore.completion
poll.failed:
  %timeout = icmp eq i32 %polled, 0
  %poll.error = call i32 @__ncc_socket_errno()
  %selected = select i1 %timeout, i32 110, i32 %poll.error
  br label %restore.selected
restore.success:
  %restore0 = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 4, i32 %old.flags)
  ret i64 0
restore.error:
  %restore1 = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 4, i32 %old.flags)
  %error64 = sext i32 %error to i64
  %negative.error = sub i64 0, %error64
  ret i64 %negative.error
restore.completion:
  %restore2 = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 4, i32 %old.flags)
  %completion64 = sext i32 %completion.error to i64
  %negative.completion = sub i64 0, %completion64
  ret i64 %negative.completion
restore.selected:
  %restore3 = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 4, i32 %old.flags)
  %selected64 = sext i32 %selected to i64
  %negative.selected = sub i64 0, %selected64
  ret i64 %negative.selected
restore.failed:
  %restore4 = call i32 (i32, i32, ...) @fcntl(i32 %fd, i32 4, i32 %old.flags)
  br label %failed
resolve.failed:
  ret i64 %length
failed:
  %late.error = call i32 @__ncc_socket_errno()
  %late64 = sext i32 %late.error to i64
  %negative.late = sub i64 0, %late64
  ret i64 %negative.late
bad:
  ret i64 -22
}

define i64 @__ncc_socket_bind(ptr %handle, ptr %host, i64 %port) {
entry:
  %address = alloca [128 x i8], align 16
  %present = icmp ne ptr %handle, null
  br i1 %present, label %resolve, label %bad
resolve:
  %length = call i64 @__ncc_net_resolve_impl(ptr %host, i64 %port, ptr %address, i64 128)
  %resolved = icmp sgt i64 %length, 0
  br i1 %resolved, label %call, label %resolve.failed
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %length32 = trunc i64 %length to i32
  %status = call i32 @bind(i32 %fd, ptr %address, i32 %length32)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  ret i64 0
failed:
  %error = call i32 @__ncc_socket_errno()
  %error64 = sext i32 %error to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
resolve.failed:
  ret i64 %length
bad:
  ret i64 -22
}

define i64 @__ncc_socket_listen(ptr %handle, i64 %backlog) {
entry:
  %present = icmp ne ptr %handle, null
  %range = icmp sge i64 %backlog, 0
  %args.ok = and i1 %present, %range
  br i1 %args.ok, label %call, label %bad
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %backlog32 = trunc i64 %backlog to i32
  %status = call i32 @listen(i32 %fd, i32 %backlog32)
  %ok = icmp eq i32 %status, 0
  br i1 %ok, label %success, label %failed
success:
  ret i64 0
failed:
  %error = call i32 @__ncc_socket_errno()
  %error64 = sext i32 %error to i64
  %negative = sub i64 0, %error64
  ret i64 %negative
bad:
  ret i64 -22
}

define ptr @__ncc_socket_accept(ptr %handle, ptr %out, i64 %capacity) {
entry:
  %length = alloca i32, align 4
  %present = icmp ne ptr %handle, null
  %out.present = icmp ne ptr %out, null
  %cap.ok = icmp sgt i64 %capacity, 0
  %buffer.ok = and i1 %out.present, %cap.ok
  %args.ok = and i1 %present, %buffer.ok
  br i1 %args.ok, label %call, label %bad
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %capacity32 = trunc i64 %capacity to i32
  store i32 %capacity32, ptr %length, align 4
  %accepted = call i32 @accept(i32 %fd, ptr %out, ptr %length)
  %valid = icmp sge i32 %accepted, 0
  %accepted64 = sext i32 %accepted to i64
  %accepted.ptr = inttoptr i64 %accepted64 to ptr
  %result = select i1 %valid, ptr %accepted.ptr, ptr null
  ret ptr %result
bad:
  ret ptr null
}

define i64 @__ncc_socket_send(ptr %handle, ptr %data, i64 %bytes, i64 %flags) {
entry:
  %present = icmp ne ptr %handle, null
  %data.present = icmp ne ptr %data, null
  %empty = icmp eq i64 %bytes, 0
  %data.ok = or i1 %data.present, %empty
  %bytes.ok = icmp sge i64 %bytes, 0
  %args0 = and i1 %present, %data.ok
  %args.ok = and i1 %args0, %bytes.ok
  br i1 %args.ok, label %call, label %bad
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %flags32 = trunc i64 %flags to i32
  %sent = call i64 @send(i32 %fd, ptr %data, i64 %bytes, i32 %flags32)
  ret i64 %sent
bad:
  ret i64 -1
}

define i64 @__ncc_socket_recv(ptr %handle, ptr %data, i64 %capacity, i64 %flags) {
entry:
  %present = icmp ne ptr %handle, null
  %data.present = icmp ne ptr %data, null
  %capacity.ok = icmp sge i64 %capacity, 0
  %args0 = and i1 %present, %data.present
  %args.ok = and i1 %args0, %capacity.ok
  br i1 %args.ok, label %call, label %bad
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %flags32 = trunc i64 %flags to i32
  %received = call i64 @recv(i32 %fd, ptr %data, i64 %capacity, i32 %flags32)
  ret i64 %received
bad:
  ret i64 -1
}

define i64 @__ncc_socket_shutdown(ptr %handle, i64 %how) {
entry:
  %present = icmp ne ptr %handle, null
  br i1 %present, label %call, label %bad
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %how32 = trunc i64 %how to i32
  %status = call i32 @shutdown(i32 %fd, i32 %how32)
  %status64 = sext i32 %status to i64
  ret i64 %status64
bad:
  ret i64 -1
}

define i64 @__ncc_socket_close(ptr %handle) {
entry:
  %present = icmp ne ptr %handle, null
  br i1 %present, label %call, label %bad
call:
  %fd64 = ptrtoint ptr %handle to i64
  %fd = trunc i64 %fd64 to i32
  %status = call i32 @close(i32 %fd)
  %status64 = sext i32 %status to i64
  ret i64 %status64
bad:
  ret i64 -1
}

define i64 @__ncc_socket_last_error() {
entry:
  %error = call i32 @__ncc_socket_errno()
  %wide = sext i32 %error to i64
  ret i64 %wide
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win64_socket_runtime_is_direct_raw_and_has_no_c_provider() {
        let llvm = render_socket_runtime("x86_64-pc-windows-msvc").expect("Win64 renderer");
        for symbol in [
            "@__ncc_net_startup(",
            "@__ncc_net_resolve(",
            "@__ncc_net_interfaces(",
            "@__ncc_socket_connect(",
            "@__ncc_socket_accept(",
            "@__ncc_socket_send(",
            "@__ncc_socket_recv(",
            "@__ncc_socket_close(",
        ] {
            assert!(llvm.contains(symbol), "missing {symbol}");
        }
        assert!(llvm.contains("@WSAPoll("));
        assert!(llvm.contains("@GetAdaptersAddresses("));
        assert!(!llvm.contains("ncc_rt_network_"));
        assert!(!llvm.contains("malloc"));
    }

    #[test]
    fn posix64_socket_runtime_is_material_and_unsupported_targets_are_explicit() {
        let linux = render_socket_runtime("x86_64-unknown-linux-gnu").expect("Linux renderer");
        assert!(linux.contains("@getifaddrs("));
        assert!(linux.contains("@poll("));
        assert!(linux.contains("@__errno_location("));
        let android = render_socket_runtime("aarch64-linux-android24").expect("Android renderer");
        assert!(android.contains("@__errno("));
        let darwin = render_socket_runtime("aarch64-apple-darwin").expect("Darwin renderer");
        assert!(darwin.contains("@__error("));
        assert!(darwin.contains(", 30"));
        let error = render_socket_runtime("wasm32-unknown-unknown").unwrap_err();
        assert!(error.contains("has no material ABI"));
    }
}
