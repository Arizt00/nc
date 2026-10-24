# Runtime Family Test Suite Contract - Priority 12

Status: closed for the current runtime family surface.

Priority 12 makes every runtime family pass through the same evidence lanes. A runtime family is not considered closed only because it appears in the import catalog.

Required lanes per family:

- example `.n`: `examples/runtime/families/<family>_basic.n`
- parser: the family example must lex and parse through `ncc-parser`
- sema: the parsed example must pass `ncc-sema`
- MCT: `ncc mct imports --json` must expose a official `use nC.runtime.<family>;` activation row
- run: `ncc run examples/runtime/families/<family>_basic.n` must execute without runtime import failure
- negative: a generated missing-use fixture must fail strict runtime checking with `E_RUNTIME_USE_001`
- installed binary: `ncc` resolved from PATH must report its executable/hash and run a runtime family check

The official family list is `ncc runtime families --json`. The MCT lane is tied to `ncc mct imports --json`, where every official runtime family must have:

- deterministic activation bytes for `nC.sysv64`, `nC.win64`, `nC.aapcs64`, and `nC.portable`
- `backing_status = closed_real_backing_bytes`
- `runtime_call_lowering = direct_runtime_symbol_call_without_real_flag`
- at least one runtime symbol under the same family path

This catches three classes of regression:

- a family exists in documentation but has no `.n` example
- a family parses but is not accepted by semantic/runtime checks
- a family exists in high-level runtime catalog but has no MCT/backend activation row

Primary test:

```bash
cargo test -p ncc-cli --test runtime_family_priority12 -- --nocapture
```

Supporting command checks:

```bash
ncc runtime families --json
ncc mct imports --json
ncc doctor bins --json
```
