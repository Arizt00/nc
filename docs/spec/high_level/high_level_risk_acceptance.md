# High Level Risk Acceptance

Status: Phase 0 audit baseline. This document defines the risk vocabulary for future phases only.

## Principle

Risk acceptance is not silence. If a program declares a dangerous capability, the compiler can allow it, but the compiler must preserve enough information to make the decision reviewable.

Allowed in future expert phases:

- unsafe casts with explicit capability
- unchecked indexing with explicit capability
- raw memory or low-level bridges with explicit capability
- partial matching with explicit capability
- races or non-rollback behavior with explicit capability

Not allowed:

- implicit dangerous behavior
- fake reports
- fake lowering
- fake success
- impossible claims accepted as OK
- unresolved symbols accepted without a declared pending state

## Evidence Shape

Future phases should record:

- capability name
- source span
- profile
- affected symbol/type/site
- accepted hazard
- lowering status
- evidence or report boundary

## Current Phase 0 Finding

The existing high-level CLI can emit diagnostics while returning exit code 0 for some negative examples. That behavior is captured as audit debt and should be addressed before a final expert release seal.
