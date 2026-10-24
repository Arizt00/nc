# Runtime Network Priority 4 Contract

Status: closed_real_backing_bytes.

This contract closes the real runtime network surface exposed through:

```n
use stoud.net { dns, tcp, tls, udp };
```

`ncc run archivo.n` calls these APIs directly. No `--real` flag is required.

## TCP

| Function | Return | Real effect |
| --- | --- | --- |
| `tcpListen(port)` | `Result<TcpHandle,NetworkError>` | binds `127.0.0.1:port` |
| `tcpListenOn(host, port)` | `Result<TcpHandle,NetworkError>` | binds concrete host/IP and port |
| `tcpPort(listener)` | `Result<int,NetworkError>` | reads local port |
| `tcpConnect(host, port)` | `Result<TcpHandle,NetworkError>` | opens real TCP stream |
| `tcpConnectTimeout(host, port, timeoutMs)` | `Result<TcpHandle,NetworkError>` | opens real TCP stream with connect timeout |
| `tcpAccept(listener)` | `Result<TcpHandle,NetworkError>` | accepts one real stream |
| `tcpSend(stream, payload)` | `Result<int,NetworkError>` | writes bytes and flushes |
| `tcpRecv(stream, maxBytes, timeoutMs)` | `Result<str,NetworkError>` | reads with timeout |
| `tcpSetNonBlocking(handle, enabled)` | `Result<bool,NetworkError>` | sets OS nonblocking mode on listener or stream |
| `tcpSetTimeout(stream, readMs, writeMs)` | `Result<bool,NetworkError>` | sets read/write timeouts |
| `tcpShutdownRead(stream)` | `Result<bool,NetworkError>` | half-closes read side |
| `tcpShutdownWrite(stream)` | `Result<bool,NetworkError>` | half-closes write side |
| `tcpClose(handle)` | `bool` | closes listener, TCP stream, UDP socket or TLS stream handle if present |

## UDP

| Function | Return | Real effect |
| --- | --- | --- |
| `udpBind(port)` | `Result<TcpHandle,NetworkError>` | binds `127.0.0.1:port` UDP socket |
| `udpBindOn(host, port)` | `Result<TcpHandle,NetworkError>` | binds concrete host/IP and port |
| `udpPort(handle)` | `Result<int,NetworkError>` | reads local UDP port |
| `udpSetNonBlocking(handle, enabled)` | `Result<bool,NetworkError>` | sets OS nonblocking mode |
| `udpSetTimeout(handle, readMs, writeMs)` | `Result<bool,NetworkError>` | sets read/write timeouts |
| `udpSendTo(handle, host, port, payload)` | `Result<int,NetworkError>` | sends a datagram |
| `udpRecvFrom(handle, maxBytes, timeoutMs)` | `Result<str,NetworkError>` | returns `addr|payload` |
| `udpClose(handle)` | `bool` | closes UDP socket |

## DNS

| Function | Return | Real effect |
| --- | --- | --- |
| `resolve(host)` | `Result<str,NetworkError>` | returns first IP from host resolver |
| `resolveAll(host)` | `Result<str,NetworkError>` | returns sorted newline-separated IPs |
| `reverseLookup(ip)` | `Result<str,NetworkError>` | asks OS resolver command when available |

## TLS

TLS is backed by `rustls` plus `webpki-roots`.

| Function | Return | Real effect |
| --- | --- | --- |
| `tlsConnect(host, port, timeoutMs)` | `Result<TcpHandle,NetworkError>` | TCP connect plus TLS client handshake |
| `tlsSend(handle, payload)` | `Result<int,NetworkError>` | writes encrypted bytes |
| `tlsRecv(handle, maxBytes, timeoutMs)` | `Result<str,NetworkError>` | reads decrypted bytes |
| `tlsPeerCert(handle)` | `Result<str,NetworkError>` | returns certificate count and first DER byte length |
| `tlsClose(handle)` | `bool` | closes TLS stream |

## Error Contract

Network fallible calls return `Result<T,NetworkError>`.

`NetworkError` is a nominal runtime error:

```text
__runtimeFamily  = runtime.net.error
__runtimeVariant = error
value            = str message with E_RUNTIME_TCP/UDP/DNS/TLS prefix
native lowering  = { code: dword, message: str }
```

This keeps `isOk`, `isErr`, `match` and `unwrap` semantics consistent while avoiding raw string-only network errors.

## Validation Fixtures

Deterministic examples:

- `examples/runtime/runtime_real_tcp_extended_network.n`
- `examples/runtime/runtime_real_udp_dns_network.n`
- `examples/runtime/negative/runtime_real_network_errors_result.n`

External TLS example:

- `examples/runtime/runtime_real_tls_https_network.n`

Tests:

- `cargo test -p ncc-cli --test runtime_process_thread_contract -- --nocapture`
- `cargo test -p ncc-cli runtime_symbol_abi -- --nocapture`
- `cargo test -p ncc-cli --test import_surface_stoud -- --nocapture`
- `cargo test -p ncc-cli --test backend_mct_runtime_symbols -- --nocapture`
