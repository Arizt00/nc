# Runtime family: `text::convert`

## Availability

`convert` belongs to the hosted runtime prelude. Normal root and runtime
programs can call its functions without a source import. The explicit compact
form remains valid when code wants to state intent or resolve an ambiguity:

```n
use <text:convert>;
```

The compatibility spelling `use::stoud::text::<convert>;` resolves to the same
catalog entry. Freestanding kernel, Machine and Primitive profiles do not gain
a hidden hosted runtime.

## Functions

| Function | Contract | Behavior |
|---|---|---|
| `toStr` | `toStr(any) -> str` | Allocates the canonical textual representation. |
| `toText` | `toText(any) -> str` | Alias with the same ownership and representation as `toStr`. |
| `toInt` | `toInt(any) -> Result<int, ConvertError>` | Exact, checked integer conversion. |
| `safeToInt` | `safeToInt(any) -> Result<int, ConvertError>` | Checked alias of `toInt`. |
| `toFloat` | `toFloat(any) -> Result<ft64, ConvertError>` | Checked finite floating-point conversion. |
| `toBool` | `toBool(any) -> Result<bool, ConvertError>` | Checked boolean conversion. |
| `forceToInt` | `forceToInt(any) -> int` | Explicit lossy conversion; invalid text becomes zero. |

Checked conversion accepts integer, floating-point, boolean, character, string
and pointer source categories where the requested result has a defined
meaning. `toInt` rejects fractional, non-finite, overflowing and pointer
inputs. `toBool` recognizes `true/false`, `yes/no`, `on/off`, `y/n` and `1/0`
without case sensitivity. `forceToInt` truncates finite fractions, saturates
finite overflow, converts pointers to their address and returns zero for an
invalid source.

## Memory and ABI

`toStr` and `toText` return owned storage and the caller releases it with
`strRelease`. `Result<ft64, ConvertError>` stores successful floating payloads
in native runtime storage; `unwrap` consumes and releases that payload.
Discarding such a result without consuming or explicitly releasing it leaves
the allocation owned by the caller, consistent with the manual hosted ABI.

The LLVM/MCT route selects an ABI symbol from the static source type. Examples
include `ncc_rt_convert_to_int_str`, `ncc_rt_convert_to_float_i64`,
`ncc_rt_convert_to_bool_str` and `ncc_rt_convert_force_int_f64`. Scalar calls
also have direct Machine lowering; floating ABI calls use the LLVM/MCT route.

On PE64, C runtime imports such as `strtod` are assigned to `msvcrt.dll`, not
`KERNEL32.dll`. ELF and Mach-O objects retain the platform C runtime symbol for
the final hosted linker.

## Diagnostics

- Wrong arity: `E_RUNTIME_PRELUDE_ARITY_001`.
- Unsupported source category: semantic argument mismatch.
- Conversion failure at runtime: `Err(ConvertError)` for checked functions.

## Evidence

- Positive example: `examples/runtime/families/convert_basic.n`.
- Negative example: `examples/runtime/families/convert_wrong_arity_fail.n`.
- Material artifacts: `target/evidence/imports/text-convert/`.
- Native Windows path: `.n -> .nm -> LLVM/MCT -> COFF -> PE64 -> execution`.
- Cross-target object compilation: Win64 COFF and Linux x86_64 ELF.
