# nC - B1 Lane Acceptance Gate

Status: Active Base (B1.7)
Version: v2.0.0-B1

## 1. Scope

This document freezes backend lane/context acceptance at pre-lowering stage.

Implemented in `ncc-ir`:

- `validate_program_for_backend_lane(program, target_spec)`

Integrated in `ncc-cli` before lowering in backend emit flows.

## 2. Acceptance Policy by Lane

### Hosted

- accepts all language contexts.

### Portable

- rejects `[@machine]` and `primitive[@]`;
- accepts regular/hosted high-level contexts.

### Freestanding

- accepts: `regular`, `[@]`, `[@runtime]`, `[@machine]`, `primitive[@]`.

### Sovereign

- accepts only: `[@]`, `[@machine]`, `primitive[@]`;
- requires at least one sovereign container surface.

## 3. Baseical Diagnostics

Rejection diagnostic:

- `backend lane '<lane>' rejects container '<name>' in context '<ctx>'; allowed contexts: ...`

Sovereign minimum-surface diagnostic:

- `backend lane 'sovereign' requires at least one sovereign container in contexts [@], [@machine], primitive[@]`

## 4. Conformance Locks

Covered by tests:

- `abi_ir_tests::backend_lane_acceptance_gate_rejects_non_sovereign_context_for_sovereign_lane`
- `abi_ir_tests::backend_lane_acceptance_gate_accepts_root_machine_primitive_for_sovereign_lane`
- `backend_emit_cli_tests::emit_backend_rejects_non_sovereign_container_under_sovereign_lane_gate`
