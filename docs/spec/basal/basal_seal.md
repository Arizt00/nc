# Basal Seal

`seal:` closes a basal contract by requiring identity, binary contract, evidence, and hash material.

If `root.authority` is `sealed`, `seal:` is mandatory.

The current CLI can parse seal requirements, build root/site/bind tables, model seal requirements in BasalIR, lower to machine references, write structural `NBB0` artifacts, integrate real machine evidence, and verify a final seal.

Baseical diagnostics:

| Diagnostic | Meaning |
|---|---|
| `E_BASAL_SEAL_REQUIRED_001` | sealed root has no `seal:` section |
| `E_BASAL_SEAL_HASH_001` | seal hash contract is missing |
| `E_BASAL_SEAL_DEFERRED_001` | legacy diagnostic for pre-Phase 6 seal deferral |

Superseded history: seal generation was previously described as waiting for machine closure or Phase 6 evidence. Machine is now available, evidence integration is closed, and final seal verification is closed.
## Phase 4 Boundary

Lowering records a pending evidence boundary but does not create a basal seal.

Any source that claims final machine evidence is rejected before `.nbb` emission.

## Phase 5 Boundary

`NBB0` stores a pending evidence record. It does not execute the final seal.

## Phase 6 Closure

`NBB0` can now store integrated machine evidence and a final seal record.
`ncc basal verify-seal <file.nbb> --json` rejects missing evidence, modified
NBB bytes, hash mismatches, and non-deterministic seal input.

## Phase 7 Release Seal

Basal Phase 7 records the release closure around the Phase 6 seal. It does not
change the seal algorithm or evidence records; it packages conformance and
worktree classification evidence.
