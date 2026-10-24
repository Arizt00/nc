# Machine No Fn/Fa Base

`fn:` and `fa:` are forbidden in `[@machine]`.

Reason: machine routines are object-level symbols and instruction sequences, not high-level language functions. Machine closure is represented through control flow and artifact contracts:

| High-level form | Machine form |
|---|---|
| `fn:` | `symbol:` plus `proc:` / `routine:` |
| function parameters | ABI registers or explicit storage |
| function return | ABI return register and `ret` |
| `fa:` | `verify:` plus `seal:` |

Diagnostics:

| Diagnostic | Meaning |
|---|---|
| `E_MACHINE_HIGH_LEVEL_FN_FORBIDDEN_001` | `fn:` used in `[@machine]` |
| `E_MACHINE_HIGH_LEVEL_FA_FORBIDDEN_001` | `fa:` used in `[@machine]` |
