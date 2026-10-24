# RootIR

Status: closed for Basal Fase 3.

`BasalRootIr` models the verified root identity from the root table:

- root id.
- root kind.
- version.
- profile.
- metadata.
- declared sections.
- source span.
- table origin.
- semantic status.

The verifier requires exactly one root, a materialized root id, coherent profile/version data inherited from Phase 2, and preserved spans.
