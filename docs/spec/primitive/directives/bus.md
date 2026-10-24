# primitive directive: bus

## Syntax
Use the official primitive[@] section form for `bus` when the parser accepts it.

## Semantics
`bus` participates in the primitive material contract only when represented in AST, validated by sema, and lowered or explicitly marked deferred.

## Allowed Modes
- primitive[@]

## Forbidden Modes
- [@runtime]
- [@compiler]
- [@machine] as user-facing shortcut for primitive closure

## Lowering
Status: `deferred`.

## Required Capability
- primitive material capability for the operation family.

## Required Policy
- no host console substitution
- no silent fallback
- no physical claim without evidence

## Safety
The directive must not perform host or device writes outside the allowlist and target contract.

## Evidence
base is documented, but complete parser/lowering/physical evidence is not claimed here.

## Examples
See `examples/primitive/hardware_identity_uart.ncp` and `examples/primitive/industrial/`.

## Diagnostics
Diagnostics must be official and specific to unsupported capability, environment, permission, or unsafe address.

## Current Status
`deferred`
