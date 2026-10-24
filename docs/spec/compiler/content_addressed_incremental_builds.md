# Content-addressed incremental builds

## Contract

Incremental compilation is enabled by default for normal hosted builds. A cache
hit is valid only when the complete key material and every retained artifact
match. Timestamps alone never authorize reuse.

The implementation has five namespaces:

- `frontend-ast`: parsed AST keyed by exact source bytes and compiler identity;
- `frontend-sema`: successful warnings/result keyed by the linked AST;
- `nbuild-standard-mct-o3`: optimized LLVM IR, target object and direct backend
  object-IR audit;
- `nbuild-runtime-object`: target runtime objects keyed by runtime source,
  target, flags and Clang identity;
- `compiler-native-o3`: the complete native build set, including optimized IR,
  object, runtime objects, machine entry object and linked executable.

The partial Stage2 compiler written in nC owns an independent implementation
of the same contract for the material it can already produce. It uses two
namespaces:

- `frontend-ast`: one parsed material AST per exact source content;
- `project-ir-mct-object`: one IR, MCT and target object set per source, target,
  unit identity and imported-symbol binding fingerprint.

An implementation change in an imported unit invalidates that unit without
invalidating callers whose imported symbol contract did not change. A target
change preserves AST hits and invalidates material hits.

Only successful parser/sema/build products are stored. Parser and semantic
errors are recomputed so diagnostics always retain their current source spans.
PGO training runs are never restored from cache.

## Key material

Native keys include all inputs capable of changing material output:

- exact source SHA-256;
- linked AST SHA-256, which invalidates on imported project changes;
- compiler image canonical path, size and modification time;
- cache schema epoch;
- backend profile and full target data layout;
- target triple, ABI and calling convention;
- optimization level and pass pipeline;
- LTO request/applied mode;
- PGO mode and profile SHA-256;
- CPU and fine-tuning profile;
- embedded runtime source SHA-256;
- Clang canonical path, size and modification time.

The source path remains part of native MCT keys because debug/source-map output
is path-sensitive. Frontend AST entries are portable between paths with the
same extension and content.

## Publication and restore

Writers acquire a per-namespace/per-key directory lock. They copy artifacts to
a private temporary directory, hash the copied material, write the manifest and
publish the complete directory with one rename. Failed publication removes its
temporary directory.

Readers hold the same key lock and verify before copying:

- schema, namespace and key;
- artifact kind;
- non-zero size;
- exact byte count;
- SHA-256.

Missing, partial or corrupt entries are cache misses and are rebuilt. Concurrent
writers converge on one immutable valid entry. Cache pruning skips entries that
are currently locked.

## Storage

The hosted Stage0 default root is `.ncc/cache/content-v1`. The partial native
Stage2 compiler uses `.ncc/cache/c2`; its short physical directories keep
Windows paths bounded, while every manifest retains and validates the full
SHA-256 logical key. Normal clean operations preserve both roots;
`ncc clean --cache` and `ncc clean --all` remove them.

Defaults:

- maximum bytes: 1 GiB;
- maximum entries: 512;
- stale writer lock: 15 minutes;
- lock wait timeout: 30 seconds.

Configuration:

```text
--incremental
--no-incremental
--content-cache-dir <path>
NCC_CONTENT_CACHE_DIR=<path>
NCC_DISABLE_CONTENT_CACHE=1
NCC_CONTENT_CACHE_MAX_BYTES=<bytes>
NCC_CONTENT_CACHE_MAX_ENTRIES=<count>
```

## Build behavior

`nbuild` restores the expensive frontend/IR/MCT/O3 products, then regenerates
output-path-dependent `.nm` bridges and launchers and relinks the requested
output. `compiler build-native` can restore the complete verified executable
set when its entire key matches.

Both JSON manifests expose:

```json
{
  "incremental": {
    "mode": "content-addressed",
    "cache_hit": true,
    "namespace": "nbuild-standard-mct-o3",
    "key": "<sha256>",
    "restored_artifacts": 3,
    "restored_bytes": 1924014
  }
}
```

This is compilation-unit reuse. The current optimized LLVM module is a linked
program unit; imported child changes invalidate that unit through the linked
AST hash. Function-level object partitioning is a separate future optimization,
not a property claimed by this contract.
