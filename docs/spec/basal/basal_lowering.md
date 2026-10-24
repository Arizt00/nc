# Basal Lowering

Basal Phase 4 closes the lowering planning layer from verified BasalIR/RootIR to the closed `[@machine]` backend.

The lowering layer is a plan, not a binary writer. It builds:

- `BasalLoweringPlan`
- `BasalMachineLoweringPlan`
- `BasalRootLowering`
- `BasalSiteLowering`
- `BasalBindLowering`
- `BasalMachineSymbolRef`
- `BasalMachineTargetRef`
- `BasalMachineSectionRef`
- `BasalMachineRelocationRef`
- `BasalLoweringVerifierReport`

Inputs:

- Phase 1 `.nb` AST and spans.
- Phase 2 root/site/bind tables.
- Phase 3 BasalIR/RootIR verifier output.

Outputs:

- A structured lowering plan.
- Machine target/profile references.
- Local site resolution.
- Machine symbol resolution.
- Pending evidence boundary.
- Baseical lowering diagnostics.

Phase 4 does not emit `.nbb`, object files, executable artifacts, or final machine evidence.

Fase 5.5 adds executable surface lowering records for const, routine, raw, verify/probe, report, and memory access plans. They are still plans, and final machine evidence remains outside this phase.
