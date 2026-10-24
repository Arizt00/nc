# Universal native-fast and C-performance contract

## Scope

Normal hosted `.n` execution selects the native route unless the caller requests
`--interpreted`. Project children are lowered with the mother program. The
material pipeline is:

```text
.n -> typed IR -> optimized MCT LLVM -> machine entry -> object -> OS executable
```

Both the direct backend and the MCT route are native backends. Falling from the
direct encoder to MCT because a runtime symbol is needed is not an interpreter
fallback.

## High-level surface

The native route admits scalar values, structs and aggregate ABI layouts,
strings, arrays, lists, queues, vectors, raw pointers, byte-addressed heap
allocation, control flow, closures and catalogued runtime calls. The retained
manifest records the route, target ABI, object, `.nm` bridge, executable and
runtime imports.

Root containers that define a direct `to main` body are material functions in
MCT. Their `stc` globals, local projections, initialization calls, runtime
calls and exact source spans are emitted even when the source declares no
ordinary `fn`. The route therefore does not reject a valid source merely
because its entry body is structural rather than a named function.

Interactive input inherits stdin. Native stdout and stderr remain attached to
the invoking terminal. `ncc run file.n` therefore preserves `readInt`,
`readStr`, prints and process exit status without selecting the evaluator.

Filesystem, process and thread calls lower to the stable `ncc_rt_*` ABI. Their
target implementation is compiled as a native object and linked with the
program. `Result`/`Option` pattern matching and thread function pointers are
decoded and invoked in generated code rather than by the interpreter.

## Optimization

`release-fast` selects O3 and whole-program optimization automatically. ThinLTO
is the normal LTO mode. Full LTO and disabling LTO remain explicit controls.
PGO supports training, profile merge, profile-use relinking and automatic reuse
of a retained per-program profile under `.ncc/pgo`.

The x86_64 scalar selector keeps call-free loop locals and raw pointers in
registers, rotates `while` loops to a single back-edge condition, selects
immediate arithmetic/comparisons and removes a signed `dword` extension only
when def-use analysis proves that every consumer observes low bits before the
value is overwritten. Signed comparisons, divisions and wider stores remain
barriers, so the optimization does not reinterpret negative values.

On the internal PE path, where Clang's profiling runtime cannot be embedded by
the PE writer, the manifest records O3 whole-module IPO instead of claiming a
ThinLTO link that did not happen.

Cross-Darwin builds use the internal Mach-O x86_64/AArch64 finalizer. Native
Darwin builds use Clang LTO and the system linker so the exact host loader and
SDK remain part of certification.

The standard MCT route preserves the requested target instead of replacing it
with the host default. Entry, program, JSON and hosted runtime objects are
passed to either the internal final linker or the explicitly configured linker
as one typed object set. ELF32 i386 execution is checked with the real system
loader when it is installed.

## Performance evidence

Performance is a measured property, not a language-wide assertion. CI compares
the nC corpus against GCC and Clang optimization matrices and retains timings,
artifact sizes, host identity and trend budgets. A regression is closed only by
the benchmark report for the exact source revision and host.

The contract does not claim that every possible nC program is faster than, or
identical in speed to, every C compiler output. It requires comparable native
code generation and retained differential evidence.
