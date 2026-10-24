# Machine EVEX Broadcast, Rounding, And SAE

Phase 3 models the contextual EVEX P2 `b` bit.

- Memory broadcast is accepted only on carriers that declare a broadcast tuple.
- Current closed broadcast carrier: `vaddps zmm, zmm, m32{1to16}`.
- Embedded rounding and SAE are accepted only on floating-point arithmetic carrier forms.
- Integer carriers reject rounding/SAE decorators.

Diagnostics:
- `E_MACHINE_EVEX_BROADCAST_001`
- `E_MACHINE_EVEX_ROUNDING_001`
- `E_MACHINE_EVEX_SAE_001`
