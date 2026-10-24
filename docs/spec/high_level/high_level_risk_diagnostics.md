# High Level Risk Diagnostics

Fase 15 diagnostics are official and high-level specific. They do not reuse machine, basal, primitive, runtime, or compiler diagnostics for risk authority errors.

Core diagnostics:

- `E_HIGH_RISK_POLICY_001`
- `E_HIGH_RISK_CAPABILITY_REQUIRED_001`
- `E_HIGH_RISK_RACE_REQUIRES_ALLOW_001`
- `E_HIGH_RISK_ALIAS_MUTATION_001`
- `E_HIGH_RISK_STREAM_BACKPRESSURE_001`
- `E_HIGH_RISK_CHANNEL_BACKPRESSURE_001`
- `E_HIGH_RISK_NO_ROLLBACK_001`
- `E_HIGH_RISK_PARTIAL_COMMIT_001`
- `E_HIGH_RISK_PARTIAL_MATCH_001`
- `E_HIGH_RISK_UNCHECKED_UNWRAP_001`
- `E_HIGH_RISK_UNCHECKED_CAST_001`
- `E_HIGH_RISK_EFFECT_ERASURE_001`
- `E_HIGH_RISK_PRIVATE_METADATA_001`
- `E_HIGH_RISK_ANY_LEAK_001`
- `E_HIGH_RISK_CONSTRUCTION_LEAK_001`
- `E_HIGH_RISK_RAW_PTR_001` (legacy strict-policy only under Pointer Model V2)
- `E_HIGH_RISK_RAW_ADDR_001` (legacy strict-policy only under Pointer Model V2)
- `E_HIGH_RISK_RAW_SITE_001` (legacy strict-policy only under Pointer Model V2)
- `E_HIGH_RISK_BRIDGE_AUTHORITY_001`
- `E_HIGH_RISK_COMPTIME_UNSAFE_001`
- `E_HIGH_RISK_UNSAFE_MACRO_001`
- `E_HIGH_RISK_NOLIMIT_001`
- `E_HIGH_RISK_UNBOUNDED_LOOP_001`
- `E_HIGH_RISK_UNBOUNDED_RECURSION_001`
- `E_HIGH_RISK_FAKE_SAFETY_001`
- `E_HIGH_RISK_SILENT_FALLBACK_001`

Every negative Phase15 example is expected to fail with one of these diagnostics. Pointer Model V2 moved raw pointer/addr/site missing-allow fixtures out of `negative/`, because those paths are now accepted as C/C++-grade raw pointer intent with diagnostics/evidence for demonstrable hazards.
