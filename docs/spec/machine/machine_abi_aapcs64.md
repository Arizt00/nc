# ABI AAPCS64

AAPCS64 is a declared contract target. The declared AArch64 profile is closed
for machine parity in Phase 8.

| Field | Value |
|---|---|
| args | `x0-x7` |
| return | `x0` |
| stack align | 16 |
| callee-saved | `x19-x28`, FP/LR by contract |

Run `ncc machine abi aapcs64 --json` and `ncc machine cross-arch --json`.
