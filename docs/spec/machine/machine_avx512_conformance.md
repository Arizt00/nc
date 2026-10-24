# AVX-512 Conformance

AVX-512 conformance is measured over the nC AVX-512 manifest, not over a smoke subset. Each included mnemonic is tied to an EVEX encoding pattern, feature gate, operand form, diagnostic coverage, example evidence, and byte-pattern hash.

Conformance depends on the Phase 3 EVEX encoding layer. AVX-512 does not duplicate EVEX prefix, mask, tuple, broadcast, rounding, SAE, or compressed displacement logic; it consumes that closed layer and binds subfeature ownership on top.

Fase 4E freezes the audit surface:

- manifest entries: all closed;
- deferred entries: zero;
- unsupported-by-design entries: zero inside the current nC AVX-512 profile;
- native-doctor: AVX-512 closed;
- `asm_complete_total`: false until phases 5-8 close.
