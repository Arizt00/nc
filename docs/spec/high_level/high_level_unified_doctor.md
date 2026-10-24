# nC Normal/High-Level Unified Doctor

Status: closed.

`ncc doctor` is the official unified doctor for normal nC and high-level nC only.

Included surfaces:

- normal nC source checking through `ncc check <file.n>`.
- official `local:` surface through `ncc local doctor --json`.
- high-level final surface through `ncc high-level unified-doctor --json`.
- runtime surface as high-level operational support through `ncc runtime doctor --json`.
- compiler surface as high-level compilation support through `ncc compiler doctor --json`.

Excluded release authorities:

- basal remains independent through `ncc basal doctor --json`.
- machine remains independent through `ncc machine doctor --json`.
- primitive remains independent through `ncc primitive doctor --json`.
- future kernel remains independent through its own future doctor.

The unified doctor does not change parser, AST, sema, IR, lowering, NBB, basal seal, machine evidence or primitive semantics. It is a CLI/reporting authority that fixes the scope of normal nC and high-level readiness.

Text mode:

```text
ncc doctor: OK
scope: normal_nc_high_level_only
included: normal_nc, local, high-level, runtime, compiler
excluded_by_design: basal, machine, primitive, kernel
semantic_changes: none
```

JSON mode:

```bash
ncc doctor --json
```

The JSON schema is `nc.doctor.unified.normal_high_level.v1`.
