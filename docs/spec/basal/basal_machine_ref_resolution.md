# Basal Machine Ref Resolution

Machine references are resolved against target declarations and declared machine symbols.

Supported target references:

- `x86_64` / `x86_64-unknown-linux-gnu`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64` / `aarch64-unknown-linux-gnu`
- `riscv64` / `riscv64-unknown-elf`
- `wasm32` / `wasm32-unknown-unknown`

Machine symbols are declared in the `target:` section:

```text
target:
    arch: x86_64;
    triple: "x86_64-unknown-linux-gnu";
    profile: basal;
    symbols: entry_start | helper_proc;
```

Resolution states:

- `resolved_to_machine_symbol`
- `resolved_to_local_site`
- `pending_machine_emit`
- `unresolved_error`

`pending_emit.<symbol>` is explicit and auditable. Other unresolved machine references fail lowering verification.
