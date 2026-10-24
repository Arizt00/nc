# Internal FFI contract - Priority 3

Status: implemented for hosted real backend execution through `foreign:` contracts and native autolink.

This contract defines the current internal FFI route used by `.n` programs to call host C, C++ and Rust functions through ABI-stable boundaries.

## Source surface

The official source form is:

```n
use foreign.c;
use foreign.cpp;
use foreign.rust;

app[@runtime] {
    foreign:
        c symbol "nc_ffi_c_add" {
            abi: C;
            header: "ffi_c_add.h";
            library: "ffi_c_add";
            linkage: dynamic;
            layout: stable;
            ownership: borrowed;
            sideEffects: none;
            signature: fn(int, int) -> int;
        }

    run :: in() -> bool {
        return nc_ffi_c_add(2, 3) == 5;
    }

    ref main ->> run;

    fa:
        return true;
}
```

The `foreign:` block is not decorative. Parser, sema and runtime/backend checks require ABI, signature, ownership, side-effect and language-specific boundary metadata.

## Real execution

The supported real execution command is:

```bash
ncc run examples/ffi/ffi_real_link_run_user_program.n --real --entry ffiRealLinkRunUserProgram.run
```

`--real` lowers the `.n` entry to backend object IR, compiles declared sidecars when present, links them with the host linker and executes the native artifact. `--native-ffi` remains accepted as an explicit spelling for the same FFI-intent path.

## Covered languages

- C: `foreign.c`, C ABI, C object sidecars and C libraries.
- C++: `foreign.cpp`, `extern "C"` exports, Itanium smoke where supported, exception-boundary contracts.
- Rust: `foreign.rust`, `extern "C"` and `repr(C)` boundary contracts.
- ASM: `foreign.asm` is gated by an explicit `[@machine]` bridge.

## Current boundary

This closes hosted FFI calls into native sidecars. It does not yet mean that every standard runtime family is implemented in `.n` over FFI. The next step is to expose a small OS runtime shim as FFI-safe C/Rust symbols and rewrite selected filesystem/TCP functions as `.n` wrappers over those symbols.

## Verification

Required checks:

- `cargo test -p ncc-cli --test ffi_real_link_run_e2e -- --nocapture`
- `ncc compiler ffi-smoke --json --strict`
- `ncc run examples/ffi/ffi_real_link_run_user_program.n --real --entry ffiRealLinkRunUserProgram.run`
