# nC - B1 Legalization Matrix

Status: Active Base (B1.4 + B1.11)
Version: v2.0.0-B1

## 1. Purpose

Legalization is lane-specific in nC and must be modeled explicitly.

This matrix is now typed in `ncc-ir` through:

- `BackendLaneIR`
- `BackendLegalizationIssueIR`
- `BackendLegalizationActionIR`
- `BackendLegalizationRuleIR`
- `BackendLegalizationMatrixIR`

B1.11 deepens the contract with:

- rule lookup by issue (`rule_for_issue(...)`);
- official shape validation (`validate_official_shape(...)`);
- lane-level required issue coverage and duplicate-rule rejection.

## 2. Lane Mapping

Lane is derived from `BackendTargetSpecIR.backend_kind`:

- HostedProduction -> Hosted
- Portable -> Portable
- Freestanding -> Freestanding
- SovereignNc -> Sovereign

## 3. Baseical Baseline Rules (all lanes)

- Aggregate passing legalization: rewrite by-ref.
- Non-native integer width legalization: scalar expansion.
- Unsupported relocation model: reject with official diagnostic.
- Structured control-flow surface: keep native lowering.

## 4. Hosted Rules

- Runtime boundary lowering rule:
  - diagnostic: `profile incompatible with selected runtime model`
  - action: lower to runtime call.

- Primitive bridge conflict:
  - diagnostic: `primitive lowering unavailable for hosted backend`
  - action: reject.

- Pointer/region surface:
  - diagnostic: `hosted lane keeps pointer and region lowering in native backend path`
  - action: keep native.

- Console render surface:
  - diagnostic: `hosted lane lowers console surface through official runtime io boundary`
  - action: lower to runtime call.

## 5. Portable Rules

- Runtime capability-safe lowering required.
- Vector surface unsupported in portable lane -> scalar expansion.
- Pointer/region surface is lowered through capability-safe runtime capsules.
- Console render surface is lowered through capability-safe runtime io boundary.

## 6. Freestanding Rules

- Runtime model guard:
  - diagnostic: `backend target declared but runtime model missing`
  - action: reject.

- Primitive bridge lowering remains native in freestanding lane.
- Pointer/region surface remains native in freestanding lane.
- Console render surface remains native in freestanding lane.

## 7. Sovereign Rules

- Root/runtime boundary kept as explicit backend contract.
- Unsupported sovereign vector surface:
  - diagnostic: `lowering impossible for unsupported sovereign vector surface`
  - action: reject.
- Pointer/region surface remains sovereign-native.
- Console render surface remains sovereign-native under runtime contract.

## 8. Baseical Shape Invariants (B1.11)

Every lane matrix must satisfy:

- non-empty ruleset;
- one and only one rule per issue;
- non-empty official diagnostic string for each rule;
- required issue coverage by lane:
  - Hosted: base + `PrimitiveBridgeHostedConflict`
  - Portable: base + `VectorSurfaceUnsupported`
  - Freestanding: base + `PrimitiveBridgeHostedConflict`
  - Sovereign: base + `VectorSurfaceUnsupported`

Base required issues:

- `AggregatePassing`
- `IntegerWidthNonNative`
- `RelocationModelUnsupported`
- `RuntimeBuiltinBoundary`
- `PointerRegionBoundary`
- `ConsoleRenderSurface`
- `StructuredControlFlowSurface`

## 9. Conformance

Matrix closure is covered by tests in `ncc-ir`:

- hosted lane official runtime/primitive diagnostics
- freestanding lane runtime-missing guard
- portable lane capability-safe pointer/console + vector expansion rules
- sovereign lane native pointer/console + vector reject rules
- duplicate-rule rejection in shape validation
- required-issue coverage rejection in shape validation
