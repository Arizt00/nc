# Runtime Math

`nC.math` queda registrado como familia de import matemática y sus intrínsecos
principales también están disponibles sin `use` explícito en código normal `.n`.

## Enteros con signo

```n
use <math:sign>;

sign(value: int) -> int
abs(value: int) -> int
```

`sign` devuelve `-1`, `0` o `1`. `abs` es saturante: para el entero mínimo
devuelve el entero máximo, evitando overflow firmado dependiente del host. Las
dos operaciones bajan a `ncc_rt_math_sign` y `ncc_rt_math_abs` en el ABI nativo.

## Intrínsecos `ft64`

| Símbolo | Firma | Bajada |
| --- | --- | --- |
| `sqrtFloat` | `sqrtFloat(value: ft64) -> ft64` | backend/runtime scalar math |
| `expFloat` | `expFloat(value: ft64) -> ft64` | backend/runtime scalar math |
| `sinFloat` | `sinFloat(value: ft64) -> ft64` | backend/runtime scalar math |
| `cosFloat` | `cosFloat(value: ft64) -> ft64` | backend/runtime scalar math |
| `atan2Float` | `atan2Float(y: ft64, x: ft64) -> ft64` | backend/runtime scalar math |
| `logFloat` | `logFloat(value: ft64) -> ft64` | backend/runtime scalar math |

`sqrtFloat`, `expFloat`, `sinFloat`, `cosFloat`, `logFloat` and `atan2Float`
are closed across runtime evaluation and MCT/LLVM lowering. Runtime uses host
`f64` math. MCT/LLVM folds finite literal calls and arithmetic first; dynamic
calls emit LLVM intrinsics for `sqrt/exp/sin/cos/log` and host `atan2`.

Imports equivalentes:

```n
use <math:core, numeric, trig>;
```

El import activa símbolos `nC.math.*`, pero las llamadas compactas anteriores
siguen siendo válidas sin import para no forzar ruido en cálculos normales.

## Constantes

| Símbolo | Firma | Nota |
| --- | --- | --- |
| `pi()` / `piFloat()` | `() -> ft64` | constante ejecutable de pi |
| `e()` / `euler()` | `() -> ft64` | constante ejecutable de Euler |
| `aureo()` / `phiFloat()` | `() -> ft64` | razón áurea |
| `piDigits()` | `() -> str` | prefijo decimal largo para impresión |
| `eulerDigits()` | `() -> str` | prefijo decimal largo para impresión |
| `aureoDigits()` | `() -> str` | prefijo decimal largo para impresión |

Pi, e y la razón áurea son irracionales: no existe una representación finita
con todos sus dígitos. El backend materializa el valor de cálculo como `ft64` y
separadamente expone cadenas decimales largas para salida por pantalla o clases.
