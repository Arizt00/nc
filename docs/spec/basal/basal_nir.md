# Basal nIR

Basal nIR preserves root contracts as reportable contract nodes:

| Node | Preserved contract |
|---|---|
| `BASAL_ROOT` | root identity, authority, binary, evidence, lowering |
| `BASAL_BINARY` | binary layout contract |
| `BASAL_SITE` | semantic-material site declarations |
| `BASAL_SEAL` | seal requirements |
| `BASAL_EVIDENCE` | evidence requirements |
| `BASAL_BRIDGE` | layer bridge intent |
| `BASAL_NBB_HEADER` | binary header verification |

The current CLI reports these as contract projections. Final machine-level lowering is deferred.

Verifier rejection rules include missing identity, required binary without `binary:`, required evidence without `evidence:`, sealed root without `seal:`, primitive mutation bridge, and active machine lowering.
