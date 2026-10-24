# Runtime Symbol ABI Priority 2

> Fuente canonica actual: `runtime_symbol_abi_canonical_v1.md`. Este documento
> conserva los layouts y el contexto historico de Priority 2.

`ncc runtime abi --json` is the stable ABI manifest for real runtime symbols.

The ABI table exists so a runtime import cannot be "real" while its call shape
is inferred ad hoc by `ncc run`, MCT or a future native backend.

## Command

```bash
ncc runtime abi --json
ncc runtime abi nC.runtime.process.spawn --json
ncc runtime abi --family tcp --json
```

The manifest schema is:

```text
nc.runtime.symbol_abi.v1
```

Each symbol row declares:

- official symbol name;
- aliases, when the runtime accepts more than one spelling;
- input names and input types;
- return type;
- `Result<T,E>` ok/error payload types when applicable;
- error codes;
- ownership and resource lifecycle.

Every linker identity is canonicalized to `nC.runtime.*`; generic source
arguments never form part of the symbol name. The manifest also carries matching
`import_manifest` and `export_manifest` views using
`nc.runtime.link_manifest.v1` and `nC.runtime-value-v1`.

## Stable Layouts

`Result<T,E>` is a tagged runtime value:

- `__runtimeFamily = runtime.result`
- `__runtimeVariant = ok | err`
- `value = T | E`

Native/MCT lowering uses a tagged union formula:

- tag byte at offset `0`;
- `ok = 1`, `err = 2`;
- payload offset is aligned to `max(align(T), align(E))`;
- payload size is `max(sizeof(T), sizeof(E))`.

`ProcessHandle` is a nominal runtime value:

- `__runtimeFamily = runtime.process.handle`
- `__runtimeVariant = handle`
- `value = int registry_slot`

Native/MCT lowering uses one `qword` slot. `wait` and `kill` consume that slot.

`ThreadHandle` follows the same nominal handle layout and is consumed by
`threadJoin`.

`ProcessError` and `ThreadError` are nominal error payloads:

- family tags identify the error domain;
- `value` stores the stable diagnostic message;
- native lowering is `{ code: dword, message: str }`.

`TcpHandle` is currently an integer registry slot. `tcpListen`, `tcpConnect` and
`tcpAccept` create it; `tcpClose` consumes it.

## Closed Symbol Families

The ABI manifest now covers every runnable symbol exported by the generated
runtime API catalog. Family additions are admitted only when their concrete
signature, ownership and import/export rows are generated together.

## Truth Boundary

Closed:

- every runnable runtime symbol exported by the compact import table has an ABI
  row;
- each row declares input types, return type, error codes and ownership;
- no row falls back to `RuntimeValue[] -> RuntimeValue`;
- all linker names use `nC.runtime.*` and omit generic arguments;
- import and export manifests contain the same typed symbol set;
- `Result<T,E>`, `ProcessHandle`, `ThreadHandle`, `ProcessError`,
  `ThreadError`, `TcpHandle` and `ptr<T>` have stable layout rows;
- `ncc runtime doctor --json` and `ncc runtime report --json` expose ABI status.

Not claimed:

- this is not a raw C ABI promise;
- target-specific object ABI still passes through backend/MCT lowering;
- catalog-only imports do not become runnable by appearing in this ABI document.
