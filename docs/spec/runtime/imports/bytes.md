# Runtime import: `memory:bytes`

```n
use <memory:bytes>;
```

`bytes` trabaja sobre direcciones y longitudes aportadas por el programa. No
crea strings, buffers, handles ni resultados administrados.

| Operacion | Contrato |
|---|---|
| `byteAt(data, offset)` | `(addr, int) -> int` |
| `bytesToHex(source, bytes, output, capacity)` | `(addr, int, addr, int) -> int` |
| `hexToBytes(source, bytes, output, capacity)` | `(addr, int, addr, int) -> int` |
| `bytesEq(left, leftBytes, right, rightBytes)` | `(addr, int, addr, int) -> bool` |
| `bytesLen(data, bytes)` | `(addr, int) -> int` |

Las conversiones devuelven bytes escritos, `-1` si la capacidad declarada es
insuficiente y `-2` si el hexadecimal es impar o contiene digitos invalidos.
`byteAt` no comprueba rango: null, UAF, desalineacion y offsets invalidos
conservan comportamiento raw.

LLVM/Machine materializa `__ncc_bytes_*` y `__ncc_hex_to_bytes` sin C, CRT ni
allocation. El fixture ejecutable es
`examples/runtime/families/memory_bytes_manual_raw.n`.
