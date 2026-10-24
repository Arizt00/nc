# Runtime errors priority 9 contract

Estado: cerrado para `ncc run` sin `--real` adicional.

Esta fase elimina el error runtime como string desnudo en las rutas principales y lo convierte en valor nominal con payload estable:

- `FsError`
- `TcpError`
- `ProcessError`
- `ThreadError`
- `MemoryError`

## Layout

Todos los errores nominales bajan en ABI/MCT como un payload de 32 bytes conceptuales:

- `code: dword`
- `message: str_ref`
- `osCause: str_ref`
- `kind: str_ref`

En el ejecutor, el valor se representa como record etiquetado:

```text
{
  __runtimeFamily: runtime.<family>.error,
  __runtimeVariant: error,
  value: {
    code: int,
    message: str,
    osCause: str,
    kind: str
  }
}
```

`osCause` conserva causa de host cuando el error la expone (`os error N`, permisos, not found, timeout). Si el host no ofrece causa verificable, el runtime crea `nc.runtime.synthetic_cause`.

## APIs cerradas

Imports:

- `use stoud.collections { error };`
- `use stoud.collections { result };`

Funciones:

- `errorCode(err) -> int`
- `errorMessage(err) -> str`
- `errorOsCause(err) -> str`
- `errorKind(err) -> str`
- `raiseRuntime(err) -> result`
- `lastError() -> option`
- `clearError() -> bool`

Retornos tipados principales:

- `fsread(path) -> Result<str, FsError>`
- `fswrite(path, text) -> Result<bool, FsError>`
- `fsappend(path, text) -> Result<bool, FsError>`
- `fsync(path) -> Result<bool, FsError>`
- `readBytes(path) -> Result<array<byte>, FsError>`
- `writeBytes(path, bytes) -> Result<int, FsError>`
- `tcpConnect(host, port) -> Result<TcpHandle, TcpError>`
- `tcpRecv(handle, maxBytes, timeoutMs) -> Result<str, TcpError>`
- `threadJoin(handle) -> Result<any, ThreadError>`
- `wait(handle) -> Result<int, ProcessError>`
- `bufferRead<T>(buffer, index) -> Result<T, MemoryError>`

UDP, DNS y TLS conservan `NetworkError` como familia general de red. TCP queda separado como `TcpError`.

## Validacion

Ejemplo:

- `examples/runtime/runtime_real_errors_priority9.n`

Tests:

- `crates/ncc-cli/tests/runtime_errors_priority9_contract.rs`

Comandos esperados:

```bash
cargo test -p ncc-cli --test runtime_errors_priority9_contract -- --nocapture
ncc run examples/runtime/runtime_real_errors_priority9.n
```
