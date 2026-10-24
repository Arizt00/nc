# High Level Contracts

Los contracts de Fase 4 fijan requisitos estaticos ligados a high-level.

```n
contract:
    Positive<T: Truthy<T>> {
        require true;
    }
```

`contract:` es la forma baseica seccionada para contratos high-level. Fase 4 mantiene contracts como verificacion declarativa estatica; no abre effects ni runtime contracts.

Alcance cerrado:

- Nombre nominal del contract.
- Parametros genericos.
- Bounds sobre parametros.
- `require` con expresiones estaticamente verificables.

Fuera de Fase 4:

- `ensure`.
- `invariant`.
- ejecucion dinamica de contracts.
- integracion con un sistema de efectos.

Diagnostics:

- `E_HIGH_CONTRACT_PARSE_001`
- `E_HIGH_CONTRACT_DUP_001`
- `E_HIGH_CONTRACT_REQUIRE_FAIL_001`
- `E_HIGH_CONTRACT_REQUIRE_UNVERIFIABLE_001`
- `E_HIGH_CONTRACT_ENSURE_DEFERRED_001`
- `E_HIGH_CONTRACT_INVARIANT_DEFERRED_001`
