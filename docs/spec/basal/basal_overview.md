# Basal Overview

Basal is the semantic-binary root layer of nC. It covers `[@]` as the root mode, `.nb` as readable basal source, and `.nbb` as verifiable basal binary source.

Basal does not touch hardware directly. Basal does not execute runtime calls directly. Basal does not emit machine artifacts without an authorized lowering backend. Basal declares the origin contract from which other layers can be verified, derived, referenced, or sealed.

Current industrial status:

| Surface | Status |
|---|---|
| `[@]` root container recognition | closed for basal CLI verification |
| `.nb` classification | closed |
| `.nbb` writer | closed in Basal Fase 5 |
| `.nbb` reader | closed in Basal Fase 5 |
| `.nbb` verifier | closed for NBB0 structure and records |
| root contract sections | closed for report/check gates |
| `.nb` grammar/parser/AST/spans | closed in Basal Fase 1 |
| root/site/bind tables | closed in Basal Fase 2 |
| BasalIR / RootIR / semantic verifier | closed in Basal Fase 3 |
| machine dependency | available_closed |
| machine lowering | closed in Basal Fase 4 |
| machine evidence integration | closed in Basal Fase 6 |
| final seal | closed in Basal Fase 6 |
| release seal / conformance | closed in Basal Fase 7 |

The current declared conformance is `basal_phase7_release_seal_closed`.
Basal release closure is claimed for the Phase 1 through Phase 7 scope.

Superseded history: old status said machine lowering was deferred until `[@machine]` closed. Machine is now closed, lowering is closed, machine evidence is integrated, and the final release evidence bundle is generated in Phase 7.
## Phase 4 Update

Basal now has a closed lowering planning layer from BasalIR/RootIR to `[@machine]`.

The layer resolves local site references, resolves declared machine symbols, records explicit pending machine emission, and keeps final evidence integration pending.

## Phase 5 Update

Basal now writes and reads structural `NBB0` artifacts. The binary format carries root/site/bind/lowering records and a pending evidence section. Final evidence and final seal execution remain out of scope.

## Phase 5.5 Update

Basal now officializes `const:`, `routine:`, `raw`, `verify:`, `probe:` alias compatibility, `report:`, fixed-width types, `mem[...]`, terminal authority, and NBB0 executable surface records. Machine evidence integration and final seal execution remain out of scope.

## Phase 6 Update

Basal now integrates real machine evidence into NBB0, records evidence hashes,
and verifies a final SHA-256 seal. Pending evidence remains valid for structural
NBB checks, but final seal verification requires integrated evidence.

## Phase 7 Update

Basal is release-sealed with a final conformance matrix, evidence bundle
manifest, CLI matrix, examples matrices, search gate evidence, and dirty
worktree classification. Phase 7 does not add features or change basal
semantics.
