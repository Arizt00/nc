# Import `use <text:string>;`

## Contract

`text:string` is a manual library. A program imports it explicitly; its implementation is written entirely in nC. This route does not generate, compile or link C source.

```nc
use <text:string>;
```

The compiler resolves this import to `runtime/nc/text_format.n`, attaches the provider to the linked AST and lowers it through:

```text
nC -> sema -> IR -> LLVM/MCT -> target object -> machine linker -> executable
```

## API surface

| Group | Functions |
| --- | --- |
| Length | `strLen`, `len`, `strByteLen` |
| Construction | `strEmpty`, `strConcat`, `repeatChar`, `charFromCode`, `charToStr` |
| Slices | `strSlice`, `strSubstring`, `substring` |
| Comparison | `strCompare`, `strEq`, `strStartsWith`, `strEndsWith` |
| Search | `strContains`, `strIndexOf` |
| Conversion | `strFromInt`, `strFromBool`, `strFromFt64` |
| Transformation | `strToUpper`, `strToLower`, `strTrim` |
| Splitting | `strSplit`, `strSplitRelease` |
| Manual lifetime | `strRelease` |
| Character code | `charCode` |

`strLen` and `strByteLen` scan NUL-terminated bytes. Unicode operations remain a separate family so decoding, normalization and allocation are explicit. Case conversion in this family is ASCII.

`charFromCode(code >> int) -> str` constructs an owned character string from an integer code. It is a declared runtime export, distinct from `charToStr(char) -> str`. The import-lowering catalog must match the complete typed string family, including this export and the release functions. Catalog tests compare symbol sets and reject duplicates.

## Manual memory

Text-producing operations return owned storage:

- `strEmpty`, `strConcat`, `repeatChar`, `charFromCode`, `charToStr`;
- `strSlice`, `strSubstring`, `substring`;
- `strFromInt`, `strFromBool`, `strFromFt64`;
- `strToUpper`, `strToLower`, `strTrim`.

The caller releases each owned result explicitly with `strRelease`. The compiler does not insert destructors, ownership checks, lifetime tracking or hidden cleanup. Releasing a literal or borrowed alias, releasing twice, or accessing released storage retains the language's raw semantics.

`strSplit` allocates a pointer table and a buffer for each fragment. Its descriptor is `array<str>` with `data` and `len`. Destroy it explicitly with:

```nc
strSplitRelease(cast<byte>(&parts));
```

## Native ABI

| nC type | LLVM/MCT representation |
| --- | --- |
| `int` | `i32` |
| `u64`, `i64`, scalar `addr` | `i64` |
| `bool` | `i1` |
| `char`, `byte` | `i8` |
| `ft64` | `double` |
| `str`, `p<T>` | `ptr` |

For this manual provider route, the final object does not import `ncc_rt_str_*`, a C runtime or libc string implementations. It may import operating-system primitives for page allocation/release and output when the program also uses `io:output`.

## Example and evidence

- Example: `examples/runtime/families/string_basic.n`.
- Provider: `runtime/nc/text_format.n`.
- Semantic tests: `crates/ncc-sema/tests/compact_text_string.rs`.
- IR/ABI test: `mct_llvm_lowers_runtime_string_ops_to_host_bytes`.
- Compiler test: `nc_string_provider_is_manual_and_never_reclassified_as_hosted_c`.
- PE64 example: exits with code `0` without linking a C runtime object.

Invoke the already-built Stage0 CLI to check this route:

```bash
./target/debug/ncc check examples/runtime/families/string_basic.n
./target/debug/ncc emit-ir examples/runtime/families/string_basic.n
./target/debug/ncc run examples/runtime/families/string_basic.n
```

On Windows, use `.\target\debug\ncc.exe`. Range checks, null handling, double release and use-after-free remain the program's responsibility. An auditing tool may diagnose them without changing generated code.