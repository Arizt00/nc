# ghor Lockfile

`ghor.lock` records deterministic resolution for the project manifest.

The primary lockfile is Cargo.lock-style TOML:

```toml
version = 4

[[package]]
name = "dep"
version = "1.0.0"
source = "registry+..."
checksum = "..."
```

Registry dependencies use `registry+...` sources. Git dependencies use `git+...#resolved_head` sources. Path dependencies omit source, matching Cargo path package semantics.

The rich nC lock model is stored next to it:

```text
ghor.lock.meta.json
```

That sidecar keeps package identity, manifest hash, target defaults, capability maps, language adapters, separated dependency sets for nC/Rust/C/C++/ASM, resolver output, and Cargo compatibility metadata.

`ghor lock` writes both files. `ghor verify-lock` compares both the TOML primary lock and the JSON sidecar against the current manifest-derived resolution.

Legacy JSON `ghor.lock` files are treated as drift and must be rewritten with `ghor lock`.
