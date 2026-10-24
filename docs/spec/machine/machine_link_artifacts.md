# Machine Link Artifacts

Phase 12 supports static executable artifacts, shared-library profile artifacts, static archive artifacts, linked WASM modules, map files, and sidecar link reports.

Artifacts are non-empty, contain real emitted object payloads or archive members, and are verified by `ncc machine verify-link`.
