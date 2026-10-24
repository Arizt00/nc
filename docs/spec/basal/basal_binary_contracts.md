# Basal Binary Contracts

`binary:` in `[@]` declares a root binary contract. It does not manufacture executable bytes.

Required fields:

| Field | Requirement |
|---|---|
| `kind` | must be `root` |
| `layout` | stable binary layout |
| `endian` | `little` or `big` |
| `width` | word width |
| `alignment` | alignment requirement |
| `magic` | root binary magic |
| `version` | binary contract version |
| `checksum` | checksum policy |
| `seal` | seal policy |

Root symbols used by a required binary contract must be stable. `str`, `text`, `list<T>`, dynamic arrays, and unconstrained pointers are rejected in binary root symbols.

Machine is closed and available. Basal Phase 4 now lowers BasalIR into a machine-facing plan, but `.nbb` writing and final machine evidence integration remain not started.
