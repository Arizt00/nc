# High Level Visibility

Visibility levels:

- `public`: exportable outside the module.
- `internal`: visible only within the package.
- `private`: visible only inside the defining module.
- `strictly`: preserves existing strict access semantics.

Public APIs must not expose private types, `any`, unresolved bounds, deferred HKT or async effects hidden from the signature.

Diagnostics: `E_HIGH_VIS_PRIVATE_ACCESS_001`, `E_HIGH_VIS_INTERNAL_ACCESS_001`, `E_HIGH_API_PRIVATE_TYPE_LEAK_001`, `E_HIGH_API_ANY_LEAK_001`, `E_HIGH_API_UNRESOLVED_BOUND_001`, `E_HIGH_API_EFFECTS_MISSING_001`.
