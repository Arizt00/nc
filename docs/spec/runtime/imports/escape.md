# Runtime family: `text::escape`

## Availability

`escape` belongs to the hosted runtime prelude. Root and runtime programs call
its functions without a source import. An explicit declaration remains valid
for intent or disambiguation:

```n
use <text:escape>;
```

The compatibility spelling `use::stoud::text::<escape>;` selects the same
family. Kernel, System, Machine and Primitive code do not receive this hosted
runtime implicitly.

## Functions

| Function | Contract | Behavior |
|---|---|---|
| `escapeAnsi` | `escapeAnsi(str) -> str` | Renders ESC and standalone C1 controls as lowercase `\\xNN`. |
| `escapeJson` | `escapeJson(str) -> str` | Escapes JSON content without adding surrounding quotes. |
| `escapeControl` | `escapeControl(str) -> str` | Renders C0, DEL and standalone C1 controls visibly. |
| `sanitizeTerminal` | `sanitizeTerminal(str) -> str` | Removes ECMA-48 sequences and dangerous controls; keeps tab and line boundaries. |
| `sanitizeLog` | `sanitizeLog(str) -> str` | Removes ECMA-48 sequences and escapes every remaining control. |

The ECMA-48 parser handles CSI, OSC, DCS, SOS, PM and APC sequences; OSC may
end with BEL or ST, and control strings end with ST. ESC intermediate/final
forms and standalone C1 introducers are recognized. Valid UTF-8 sequences are
copied atomically so a continuation byte is never mistaken for a raw C1
control.

`escapeJson` emits standard short escapes for quote, backslash, backspace,
form-feed, newline, carriage return and tab. Other C0 bytes use `\\u00NN`.

## Memory and ABI

Every operation allocates an owned NUL-terminated UTF-8 result. The caller
releases it with `strRelease`. A null source pointer is treated as an empty
string by the native ABI; allocation failure returns null.

LLVM/MCT calls `ncc_rt_escape_ansi`, `ncc_rt_escape_json`,
`ncc_rt_escape_control`, `ncc_rt_sanitize_terminal` or
`ncc_rt_sanitize_log`. Machine emits those calls directly with the target ABI.

## Diagnostics

- Wrong arity: `E_RUNTIME_PRELUDE_ARITY_001`.
- Wrong argument type: semantic argument mismatch.
- Native allocation failure: `E_RUNTIME_ESCAPE_ALLOC_001` contract.

## Evidence

- Positive example: `examples/runtime/families/escape_basic.n`.
- Negative example: `examples/runtime/families/escape_wrong_arity_fail.n`.
- Material artifacts: `target/evidence/imports/text-escape/`.
- Native Windows path: `.n -> .nm -> LLVM/MCT -> COFF -> PE64 -> execution`.
- Cross-target object compilation: Win64 COFF and Linux x86_64 ELF.
