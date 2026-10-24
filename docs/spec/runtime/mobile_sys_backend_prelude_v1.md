# Mobile and sys backend prelude v1

Status: implemented for hosted high-level and `[@runtime]` programs.

## Source contract

The following source modules are compatibility aliases. They remain accepted,
but ordinary hosted code does not need them:

```nc
use stoud.io { output, render, input };
```

`input`, `output` and `render` remain explicit IO capabilities. In particular,
`print` and `println` still require `use stoud.io { output };`. The filesystem
operations listed above are direct hosted-prelude functions. Removing the
mobile, network, filesystem and sys imports does not add synthetic entries to
`Program.uses`; sema resolves typed catalog entries and IR emits canonical
runtime symbols.

Import-free source spellings are:

```nc
fswrite(path, text);
lines = fread_lines(path);
delete_file(path);
rename_file(source, destination);
get_current_time();
cpu_load();
registry_write(key, value);
registry_read(key);
sleep(milliseconds);

telephony::getState();
tts_engine::speak(text);
permissions::request(mask);
network::server_open(port);
network::client_open(host, port);
network::send_bytes(text);
network::receive_bytes();
network::isWiFiConnected();
```

`server` and `client` are compatibility import groups, not hidden objects or
constructors. Their callable operations live under `network::` and return raw
endpoint handles.

Canonical families:

| Source family | Backend family | Operations |
| --- | --- | --- |
| `telephony` | `nC.runtime.telephony` | incoming number, state, availability, dial, answer, hang up |
| `tts_engine` | `nC.runtime.tts` | connect, speak, stop, speaking state, disconnect |
| `network::request` | `nC.runtime.mobileNetwork` | provider request, status, JSON field, body, release |
| `server` / `client` | `nC.runtime.mobileNetwork` | TCP open, accept/connect, send, receive and close |
| `send_bytes` / `receive_bytes` | `nC.runtime.mobileNetwork` | active-socket string and raw-byte transfer |
| `permissions` | `nC.runtime.permission` | request and permission checks |
| `get_current_time` | `nC.runtime.time` | current time through the hosted backend |
| `cpu_load` | `nC.runtime.system` | sampled host CPU utilization from 0 to 100, or -1 on an OS sampling error |
| `registry_write` / `registry_read` | `nC.runtime.system` | per-user persistent key/value storage |
| `delete_file` / `rename_file` | `nC.runtime.fs` | real host file removal and atomic host rename where the OS supports it |
| `sleep` | `nC.runtime.time` | blocking hosted sleep using the established millisecond contract |

`registry_write(key, value)` and `registry_read(key)` use
`HKCU\Software\nC\Runtime` on Windows. Unix targets use a persistent file in
the current user's home directory; the stored key is validated on read so a
hash collision cannot return another key's value.

The constants `PERM_PHONE`, `PERM_NET`, `PERM_MIC`, `PERM_LOCATION`,
`PERM_CONTACTS`, `PERM_SMS`, `PERM_CAMERA`, `PERM_STORAGE` and
`PERM_NOTIFICATIONS` are backend-prelude integer masks.

## Response and memory contract

`network::request(url)` returns a raw `MobileResponse` handle. The language
accepts `result.status`, `result.json(key)`, `result.body()` and
`result.release()`. JSON field strings and the response handle are runtime
allocations. Manual/raw programs remain responsible for releasing values when
their lifetime matters; no hidden ownership is introduced.

## Socket ABI

`network::server_open(port)` and `network::client_open(host, port)` return raw
opaque handles. `receive`, `send` and `close` are available as handle methods.
The free functions `send_bytes` and `receive_bytes` use the explicitly active
handle; `sendBytesRaw(address, length)` and `receiveBytesInto(address,
capacity)` preserve raw byte counts and do not add framing.

The hosted object uses WinSock on Windows and POSIX sockets on Unix. Its
`addrinfo` descriptor follows the target ABI: Win64 uses a 64-bit length before
the canonical-name pointer; POSIX uses `socklen_t` before the address pointer.
PE imports are grouped under `WS2_32.dll`, and Unix targets retain their native
libc socket symbols.

## Provider ABI

The hosted runtime exports `ncc_rt_mobile_install_provider` and consumes an
`NcMobileProviderV2` table, while retaining the V1 installation entry. A
platform host supplies telephony, TTS, mobile request, WiFi/ADB and permission
callbacks. POSIX targets may also link the weak provider symbol. Android/iOS
packaging can therefore bind the same generated nC object to platform services
without source imports.

When no platform provider is installed, deterministic host testing uses:

- `NCC_MOBILE_INCOMING_NUMBER`
- `NCC_MOBILE_CALL_STATE`
- `NCC_MOBILE_TTS_READY`
- `NCC_MOBILE_HTTP_STATUS`
- `NCC_MOBILE_HTTP_JSON`
- `NCC_MOBILE_ALLOW_PERMISSIONS`
- `NCC_MOBILE_PERMISSION_MASK`

The fallback never invents a phone number, location or permission. Missing
values produce idle/empty/denied results.

## Materialization

The normal pipeline is preserved:

```text
.n -> typed AST/sema -> canonical IR -> LLVM/MCT -> .nm -> .obj -> executable
```

The generated executable links the hosted runtime object containing the
provider ABI. On PE64, CRT imports such as `getenv` are assigned to
`msvcrt.dll`; Win32 imports remain in their owning system DLL.

## Examples

- `examples/12_apps/geocaller_mobile_backend.n`: the complete persistent
  service, retaining historical imports as compatibility evidence.
- `examples/12_apps/mobile_backend_smoke_no_imports.n`: finite executable
  proof with only the explicit IO import.
- `examples/12_apps/blockmaster_remote_backend.n`: structs, dynamic aggregate
  arrays, file lines, split/trim, TCP server handles, ADB, call state and TTS.
- `examples/07_runtime/mobile_network_io_sys_material.n`: import-free hosted
  surface except for explicit output, native file lifecycle, CPU sampling,
  registry persistence, clock and WiFi provider query.
