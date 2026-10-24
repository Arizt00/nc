# Basal Bridges

Basal bridges declare relationships to other layers. They do not execute those layers.

| Bridge | Rule |
|---|---|
| `[@]` to `[@compiler]` | allowed for verification |
| `[@]` to `[@runtime]` | allowed only for authorized reporting |
| `[@]` to `[@machine]` | deferred until machine is closed |
| `[@]` to `primitive[@]` | immutable witness reference only |

Primitive mutation from basal is rejected with `E_BASAL_BRIDGE_PRIMITIVE_MUTATION_001`.

Active machine lowering is rejected with `E_BASAL_BRIDGE_MACHINE_DEFERRED_001`.
