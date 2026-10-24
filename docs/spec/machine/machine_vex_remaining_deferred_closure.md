# Machine VEX Remaining Deferred Closure

This note records Phase 1E of the native machine assembler plan. The phase closes the remaining valid Phase 1 VEX operand forms by native descriptor-based encoding while keeping the global VEX claim open.

Phase 1E does not start AVX2, EVEX, AVX-512, object-aware disassembly, or roundtrip closure. It only updates the VEX Phase 1 table, verifier, encoding status, byte-pattern evidence, examples, reports, and doctor gates.

## Scope

Phase 1E covers the remaining Phase 1 VEX forms across shuffle and permute, move, conversion, arithmetic, compare, logical, and miscellaneous families. Each valid form uses the existing VexInstructionDef and VexOperandForm metadata created in Phase 1A and Phase 1B.

The phase keeps five instructions outside this closure by design:

- vpermps, owned by the future AVX2 phase.
- vgatherdps, owned by the future AVX2 phase.
- vpmulld, owned by the future AVX2 phase.
- vfmadd132ps, owned by the future FMA phase.
- vcvtph2ps, owned by the future F16C phase.

Those entries remain in the manifest and surface explicit diagnostics. They are not removed and they do not count as Phase 1 VEX closed.

## Encoding Contract

Every newly closed valid form records:

- VEX2 or VEX3 prefix kind.
- opcode map, opcode byte, pp, W, L, and vvvv fields.
- ModRM policy.
- SIB, displacement, and immediate policy when the form requires them.
- feature gate.
- positive coverage.
- invalid-form coverage.
- byte-pattern evidence hash.

The encoder emits deterministic native bytes. It does not use textual assembly export as a substitute for encoding.

## Verifier Contract

The verifier remains closed for all 207 operand forms. It validates operand count, vector register class, xmm versus ymm width, memory width, immediate width, feature availability, unsupported-by-design instructions, VEX field legality, and invalid destination/source shapes before emission.

## Status

Phase 1E is closed when all valid Phase 1 VEX forms are encoded and no valid form remains deferred. The global Phase 1 VEX status remains not_closed because ten operand forms belong to future extension domains and are explicitly unsupported by design in this phase.

The expected doctor state is:

```json
{
  "phase1e_vex_deferred_closure": "closed",
  "phase1_vex_complete": "not_closed",
  "phase2_avx2_complete": "not_started",
  "asm_complete_total": false
}
```
