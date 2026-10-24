# ghor Artifacts And Evidence

`ghor artifact --json` writes an artifact manifest under `target/ghor/artifacts` unless `--out` is provided.

Artifact records include package identity, version, target, artifact kind, graph hash, and manifest hash. Evidence commands include machine and basal command hashes plus target/toolchain matrix hashes.
