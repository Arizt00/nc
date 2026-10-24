# Proc / Routine / Block

Machine routines are declared as `proc:` or `routine:` entries. They are symbol-backed object-level routines.

Required supporting contracts:

| Contract | Purpose |
|---|---|
| `symbol:` | binds routine to object symbol |
| `entry:` | declares entry symbol |
| `frame:` | declares ABI and stack contract |
| `instr:` | declares instruction stream |
| `verify:` | declares gates to prove |
| `seal:` | final artifact contract |

Blocks and labels are resolved by the verifier. Conditional branches require an earlier flag producer.
