# `.nbb` Format

`.nbb` is a binary basal source. It is accepted only when its binary contract verifies.

Initial header layout:

| Offset | Field | Size |
|---:|---|---:|
| 0 | magic `NBB1` | 4 |
| 4 | version | 2 |
| 6 | flags | 2 |
| 8 | header size | 4 |
| 12 | payload size | 8 |
| 20 | contract hash | 32 |
| 52 | payload hash | 32 |
| 84 | seal hash | 32 |
| 116 | payload | variable |

The verifier checks magic, version, header size, payload size, materialized contract hash, payload hash, and materialized seal hash.

Diagnostics include `E_NBB_MAGIC_001`, `E_NBB_VERSION_001`, `E_NBB_PAYLOAD_HASH_001`, and `E_NBB_TRUNCATED_001`.
