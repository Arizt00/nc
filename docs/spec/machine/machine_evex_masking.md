# Machine EVEX Masking

EVEX uses P2 `aaa` to encode opmask registers.

- no explicit mask uses `aaa=0`.
- explicit masks require `k1..k7`.
- explicit `k0` is rejected because no-mask semantics must be expressed by omitting the mask.
- zeroing requires an explicit nonzero mask.
- merging is the default when a mask is present without zeroing.

Diagnostics:
- `E_MACHINE_EVEX_MASK_001`
- `E_MACHINE_EVEX_ZEROING_001`
