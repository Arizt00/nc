# Core, alloc and stoud library layers

Status: canonical.

This document defines the ownership and dependency direction of the three
standard library layers. Import spelling does not change ownership: every
resolved import is classified by the canonical catalog before semantic or
backend work begins.

## Dependency direction

```text
stoud -> alloc -> core
stoud -------> core
```

The reverse edges are forbidden. `core` never depends on an allocator or an
operating system. `alloc` may use `core`, but it never imports hosted I/O,
filesystem, networking, process or hosted-thread services. `stoud` is the
hosted layer and may use both lower layers.

## Layer contract

| Layer | Owns | May require an OS | May allocate |
| --- | --- | --- | --- |
| `core` | scalar types, pointers, volatile access, atomics, fences and OS-free math | no | no |
| `alloc` | heap, regions, owned strings and allocating collections | no | yes, through the selected allocator provider |
| `stoud` | console, files, network, process, hosted threads and runtime services | yes | yes |

Canonical import families include:

```text
use core.memory { ptr, region };
use core.sync { atomic };
use alloc.heap { heap };
use alloc.collections { list, queue };
use stoud.io { input, output, render };
```

Historical `stoud::memory` spellings remain compatibility aliases. The import
catalog expands them to canonical functions and classifies each resolved item
by its owner. An alias therefore cannot bypass a kernel capability check.

## Hosted programs

Standard hosted profiles admit all three layers. The allocator provider may be
the native runtime, while OS services are materialized as typed runtime imports
or direct native calls according to the selected backend.

## Kernel programs

Kernel profiles always admit `core`. They admit `alloc` only when the build
selects an explicit freestanding allocator provider with `--kernel-alloc`.
The MCT backend then embeds the internal allocator only when an allocating
operation is reachable.

Kernel profiles reject all `stoud` services before object materialization.
This includes aliases that resolve to filesystem, console, network, process,
hosted thread or libc-backed symbols. The resulting object cannot acquire a
hosted dependency accidentally.

The freestanding allocator provides `malloc`, `calloc`, `realloc` and `free`
semantics over an internal heap with aligned headers, first-fit selection,
splitting and adjacent-block coalescing. Its zero and copy operations are
internal loops; they do not synthesize `memset`, `memcpy` or libc imports.

## Required diagnostics

- `E_KERNEL_ALLOC_UNAVAILABLE_001`: allocating code has no selected kernel
  allocator provider.
- `E_KERNEL_HOSTED_DEPENDENCY_001`: a kernel import resolves to `stoud` or a
  hosted ABI symbol.
- `E_KERNEL_ALLOC_BODY_NOT_MATERIALIZED_001`: allocation was admitted but the
  internal provider was not emitted.

These are hard build errors. The compiler does not downgrade them to warnings
or defer them to the system linker.

## Evidence

- Model and import classification: `crates/ncc-ast/src/memory_model.rs` and
  `crates/ncc-ast/src/import_catalog.rs`.
- Freestanding allocator lowering:
  `crates/ncc-ir/src/freestanding_alloc_llvm.rs`.
- End-to-end kernel gates: `crates/ncc-cli/tests/kernel_library_layers.rs`.
- Freestanding fixture: `examples/11_machine/kernel_alloc_freestanding.n`.

The normative regression command is:

```text
cargo test -p ncc-cli --test kernel_library_layers -- --nocapture
```
