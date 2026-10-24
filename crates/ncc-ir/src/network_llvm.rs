pub(crate) fn render_network_runtime(target: &str) -> Result<String, String> {
    let lower = target.to_ascii_lowercase();
    let win64 =
        lower.contains("windows") && (lower.contains("x86_64") || lower.contains("aarch64"));
    let posix64 = (lower.contains("linux")
        || lower.contains("android")
        || lower.contains("darwin")
        || lower.contains("apple"))
        && (lower.contains("x86_64") || lower.contains("aarch64") || lower.contains("riscv64"));
    let supported = win64 || posix64;
    if !supported {
        return Err(format!(
            "native network backend requires Windows, Linux, Android or Darwin, got target '{target}'"
        ));
    }
    Ok(NETWORK_RUNTIME.to_string())
}

// The public compatibility names remain stable, but their implementation is
// material LLVM. A handle owns one socket plus an optional accepted peer.
const NETWORK_RUNTIME: &str = r#"
@__ncc_network_active = internal global ptr null, align 8
@__ncc_network_response = internal global ptr null, align 8
@__ncc_network_any = private unnamed_addr constant [8 x i8] c"0.0.0.0\00", align 1
@__ncc_network_empty = private unnamed_addr constant [1 x i8] zeroinitializer, align 1

define internal i64 @__ncc_network_strlen(ptr %text) {
entry:
  %present = icmp ne ptr %text, null
  br i1 %present, label %loop, label %done.null
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %step ]
  %slot = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %slot, align 1
  %zero = icmp eq i8 %byte, 0
  br i1 %zero, label %done, label %step
step:
  %next = add i64 %index, 1
  br label %loop
done:
  ret i64 %index
done.null:
  ret i64 0
}

define internal ptr @__ncc_network_copy(ptr %source, i64 %length) {
entry:
  %bytes = add i64 %length, 1
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 8)
  %allocated = icmp ne ptr %out, null
  br i1 %allocated, label %copy.loop, label %failed
copy.loop:
  %index = phi i64 [ 0, %entry ], [ %next, %copy.byte ]
  %more = icmp ult i64 %index, %length
  br i1 %more, label %copy.byte, label %terminate
copy.byte:
  %src = getelementptr i8, ptr %source, i64 %index
  %dst = getelementptr i8, ptr %out, i64 %index
  %byte = load i8, ptr %src, align 1
  store i8 %byte, ptr %dst, align 1
  %next = add i64 %index, 1
  br label %copy.loop
terminate:
  %end = getelementptr i8, ptr %out, i64 %length
  store i8 0, ptr %end, align 1
  ret ptr %out
failed:
  ret ptr null
}

define internal ptr @__ncc_network_handle_new(ptr %socket, i1 %listener) {
entry:
  %handle = call ptr @__ncc_raw_aligned_alloc(i64 24, i64 8)
  %allocated = icmp ne ptr %handle, null
  br i1 %allocated, label %initialize, label %failed
initialize:
  %peer.slot = getelementptr i8, ptr %handle, i64 8
  %listener.slot = getelementptr i8, ptr %handle, i64 16
  store ptr %socket, ptr %handle, align 8
  store ptr null, ptr %peer.slot, align 8
  store i1 %listener, ptr %listener.slot, align 1
  store ptr %handle, ptr @__ncc_network_active, align 8
  ret ptr %handle
failed:
  ret ptr null
}

define internal ptr @__ncc_network_open(ptr %host, i64 %port, i1 %listener) {
entry:
  %port.low = icmp sge i64 %port, 0
  %port.high = icmp sle i64 %port, 65535
  %port.ok = and i1 %port.low, %port.high
  %host.present = icmp ne ptr %host, null
  %args.ok = or i1 %listener, %host.present
  %valid = and i1 %port.ok, %args.ok
  br i1 %valid, label %startup, label %failed
startup:
  %started = call i64 @__ncc_net_startup()
  %startup.ok = icmp eq i64 %started, 0
  br i1 %startup.ok, label %create, label %failed
create:
  %socket = call ptr @__ncc_socket_create(i64 2, i64 1, i64 6)
  %created = icmp ne ptr %socket, null
  br i1 %created, label %configure, label %shutdown.failed
configure:
  br i1 %listener, label %bind, label %connect
bind:
  %bound = call i64 @__ncc_socket_bind(ptr %socket, ptr @__ncc_network_any, i64 %port)
  %bind.ok = icmp eq i64 %bound, 0
  br i1 %bind.ok, label %listen, label %socket.failed
listen:
  %listened = call i64 @__ncc_socket_listen(ptr %socket, i64 16)
  %listen.ok = icmp eq i64 %listened, 0
  br i1 %listen.ok, label %handle, label %socket.failed
connect:
  %connected = call i64 @__ncc_socket_connect(ptr %socket, ptr %host, i64 %port, i64 30000)
  %connect.ok = icmp eq i64 %connected, 0
  br i1 %connect.ok, label %handle, label %socket.failed
handle:
  %out = call ptr @__ncc_network_handle_new(ptr %socket, i1 %listener)
  %handle.ok = icmp ne ptr %out, null
  br i1 %handle.ok, label %success, label %socket.failed
success:
  ret ptr %out
socket.failed:
  %closed = call i64 @__ncc_socket_close(ptr %socket)
  br label %shutdown.failed
shutdown.failed:
  %stopped = call i64 @__ncc_net_shutdown()
  br label %failed
failed:
  ret ptr null
}

define internal ptr @__ncc_network_stream(ptr %handle) {
entry:
  %present = icmp ne ptr %handle, null
  br i1 %present, label %read, label %failed
read:
  %peer.slot = getelementptr i8, ptr %handle, i64 8
  %listener.slot = getelementptr i8, ptr %handle, i64 16
  %socket = load ptr, ptr %handle, align 8
  %listener = load i1, ptr %listener.slot, align 1
  br i1 %listener, label %listener.path, label %client.path
client.path:
  ret ptr %socket
listener.path:
  %peer = load ptr, ptr %peer.slot, align 8
  %accepted.before = icmp ne ptr %peer, null
  br i1 %accepted.before, label %peer.ready, label %accept
accept:
  %address = alloca [128 x i8], align 16
  %accepted = call ptr @__ncc_socket_accept(ptr %socket, ptr %address, i64 128)
  store ptr %accepted, ptr %peer.slot, align 8
  br label %peer.ready
peer.ready:
  %stream = phi ptr [ %peer, %listener.path ], [ %accepted, %accept ]
  ret ptr %stream
failed:
  ret ptr null
}

define internal i64 @__ncc_network_send_all(ptr %handle, ptr %bytes, i64 %length) {
entry:
  %length.ok = icmp sge i64 %length, 0
  %data.present = icmp ne ptr %bytes, null
  %empty = icmp eq i64 %length, 0
  %data.ok = or i1 %data.present, %empty
  %args.ok = and i1 %length.ok, %data.ok
  br i1 %args.ok, label %stream, label %failed
stream:
  %socket = call ptr @__ncc_network_stream(ptr %handle)
  %present = icmp ne ptr %socket, null
  br i1 %present, label %loop, label %failed
loop:
  %offset = phi i64 [ 0, %stream ], [ %next, %written ]
  %more = icmp ult i64 %offset, %length
  br i1 %more, label %send, label %done
send:
  %remaining = sub i64 %length, %offset
  %large = icmp ugt i64 %remaining, 2147483647
  %chunk = select i1 %large, i64 2147483647, i64 %remaining
  %source = getelementptr i8, ptr %bytes, i64 %offset
  %sent = call i64 @__ncc_socket_send(ptr %socket, ptr %source, i64 %chunk, i64 0)
  %sent.ok = icmp sgt i64 %sent, 0
  br i1 %sent.ok, label %written, label %failed
written:
  %next = add i64 %offset, %sent
  br label %loop
done:
  ret i64 %offset
failed:
  ret i64 -1
}

define internal i64 @__ncc_network_receive_into(ptr %handle, ptr %out, i64 %capacity) {
entry:
  %capacity.ok = icmp sge i64 %capacity, 0
  %out.present = icmp ne ptr %out, null
  %empty = icmp eq i64 %capacity, 0
  %out.ok = or i1 %out.present, %empty
  %args.ok = and i1 %capacity.ok, %out.ok
  br i1 %args.ok, label %stream, label %failed
stream:
  %socket = call ptr @__ncc_network_stream(ptr %handle)
  %present = icmp ne ptr %socket, null
  br i1 %present, label %receive, label %failed
receive:
  %large = icmp ugt i64 %capacity, 2147483647
  %chunk = select i1 %large, i64 2147483647, i64 %capacity
  %received = call i64 @__ncc_socket_recv(ptr %socket, ptr %out, i64 %chunk, i64 0)
  ret i64 %received
failed:
  ret i64 -1
}

define i64 @ncc_rt_network_server_open(i64 %port) {
entry:
  %handle = call ptr @__ncc_network_open(ptr null, i64 %port, i1 true)
  %raw = ptrtoint ptr %handle to i64
  ret i64 %raw
}

define i64 @ncc_rt_network_client_open(ptr %host, i64 %port) {
entry:
  %handle = call ptr @__ncc_network_open(ptr %host, i64 %port, i1 false)
  %raw = ptrtoint ptr %handle to i64
  ret i64 %raw
}

define ptr @ncc_rt_network_receive(i64 %raw.handle) {
entry:
  %handle = inttoptr i64 %raw.handle to ptr
  %buffer = call ptr @__ncc_raw_aligned_alloc(i64 65537, i64 8)
  %allocated = icmp ne ptr %buffer, null
  br i1 %allocated, label %receive, label %failed
receive:
  %received = call i64 @__ncc_network_receive_into(ptr %handle, ptr %buffer, i64 65536)
  %some = icmp sgt i64 %received, 0
  br i1 %some, label %terminate, label %empty
terminate:
  %end = getelementptr i8, ptr %buffer, i64 %received
  store i8 0, ptr %end, align 1
  ret ptr %buffer
empty:
  %released = call i1 @__ncc_raw_aligned_free(ptr %buffer)
  %empty.out = call ptr @__ncc_network_copy(ptr @__ncc_network_empty, i64 0)
  ret ptr %empty.out
failed:
  ret ptr null
}

define i64 @ncc_rt_network_send(i64 %raw.handle, ptr %text) {
entry:
  %present = icmp ne ptr %text, null
  br i1 %present, label %send, label %failed
send:
  %handle = inttoptr i64 %raw.handle to ptr
  %length = call i64 @__ncc_network_strlen(ptr %text)
  %sent = call i64 @__ncc_network_send_all(ptr %handle, ptr %text, i64 %length)
  ret i64 %sent
failed:
  ret i64 -1
}

define void @ncc_rt_network_response_clear() {
entry:
  %old = load ptr, ptr @__ncc_network_response, align 8
  %present = icmp ne ptr %old, null
  br i1 %present, label %release, label %done
release:
  %released = call i1 @__ncc_raw_aligned_free(ptr %old)
  br label %done
done:
  store ptr null, ptr @__ncc_network_response, align 8
  ret void
}

define ptr @ncc_rt_network_response_last() {
entry:
  %capture = load ptr, ptr @__ncc_network_response, align 8
  %present = icmp ne ptr %capture, null
  %result = select i1 %present, ptr %capture, ptr @__ncc_network_empty
  ret ptr %result
}

define i64 @ncc_rt_network_send_bytes(ptr %text) {
entry:
  %present = icmp ne ptr %text, null
  br i1 %present, label %capture, label %failed
capture:
  call void @ncc_rt_network_response_clear()
  %length = call i64 @__ncc_network_strlen(ptr %text)
  %copy = call ptr @__ncc_network_copy(ptr %text, i64 %length)
  %copied = icmp ne ptr %copy, null
  br i1 %copied, label %save, label %failed
save:
  store ptr %copy, ptr @__ncc_network_response, align 8
  %active = load ptr, ptr @__ncc_network_active, align 8
  %connected = icmp ne ptr %active, null
  br i1 %connected, label %send, label %captured
send:
  %sent = call i64 @__ncc_network_send_all(ptr %active, ptr %text, i64 %length)
  ret i64 %sent
captured:
  ret i64 %length
failed:
  ret i64 -1
}

define ptr @ncc_rt_network_receive_bytes() {
entry:
  %active = load ptr, ptr @__ncc_network_active, align 8
  %raw = ptrtoint ptr %active to i64
  %out = call ptr @ncc_rt_network_receive(i64 %raw)
  ret ptr %out
}

define i64 @ncc_rt_network_send_raw(ptr %address, i64 %length) {
entry:
  %active = load ptr, ptr @__ncc_network_active, align 8
  %sent = call i64 @__ncc_network_send_all(ptr %active, ptr %address, i64 %length)
  ret i64 %sent
}

define i64 @ncc_rt_network_receive_raw(ptr %address, i64 %capacity) {
entry:
  %active = load ptr, ptr @__ncc_network_active, align 8
  %received = call i64 @__ncc_network_receive_into(ptr %active, ptr %address, i64 %capacity)
  ret i64 %received
}

define i32 @ncc_rt_network_close(i64 %raw.handle) {
entry:
  %handle = inttoptr i64 %raw.handle to ptr
  %present = icmp ne ptr %handle, null
  br i1 %present, label %read, label %absent
read:
  %active = load ptr, ptr @__ncc_network_active, align 8
  %is.active = icmp eq ptr %active, %handle
  br i1 %is.active, label %clear.active, label %close
clear.active:
  store ptr null, ptr @__ncc_network_active, align 8
  br label %close
close:
  %peer.slot = getelementptr i8, ptr %handle, i64 8
  %socket = load ptr, ptr %handle, align 8
  %peer = load ptr, ptr %peer.slot, align 8
  %peer.present = icmp ne ptr %peer, null
  br i1 %peer.present, label %close.peer, label %close.socket
close.peer:
  %peer.closed = call i64 @__ncc_socket_close(ptr %peer)
  br label %close.socket
close.socket:
  %socket.closed = call i64 @__ncc_socket_close(ptr %socket)
  %released = call i1 @__ncc_raw_aligned_free(ptr %handle)
  %stopped = call i64 @__ncc_net_shutdown()
  ret i32 1
absent:
  ret i32 0
}

define i32 @ncc_rt_mobile_network_is_wifi_connected() {
entry:
  %available = call i1 @__ncc_net_available()
  %result = zext i1 %available to i32
  ret i32 %result
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_runtime_owns_the_complete_compatibility_surface() {
        let llvm = render_network_runtime("x86_64-pc-windows-msvc").expect("network renderer");
        for symbol in [
            "@ncc_rt_network_server_open(",
            "@ncc_rt_network_client_open(",
            "@ncc_rt_network_receive(",
            "@ncc_rt_network_send(",
            "@ncc_rt_network_send_bytes(",
            "@ncc_rt_network_receive_bytes(",
            "@ncc_rt_network_send_raw(",
            "@ncc_rt_network_receive_raw(",
            "@ncc_rt_network_close(",
            "@ncc_rt_network_response_clear(",
            "@ncc_rt_network_response_last(",
        ] {
            assert!(llvm.contains(symbol), "missing {symbol}");
        }
        assert!(llvm.contains("@__ncc_network_send_all("));
        assert!(llvm.contains("@__ncc_socket_accept("));
        assert!(!llvm.contains("malloc"));
    }

    #[test]
    fn network_runtime_has_explicit_target_contract() {
        assert!(render_network_runtime("aarch64-linux-android24").is_ok());
        assert!(render_network_runtime("aarch64-apple-darwin").is_ok());
        let error = render_network_runtime("wasm32-unknown-unknown").unwrap_err();
        assert!(error.contains("requires Windows, Linux, Android or Darwin"));
    }
}
