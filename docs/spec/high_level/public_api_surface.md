# Public API Surface

The public API surface is the deterministic set of exported declarations for a module/package.

It records:

- Packages and modules.
- Exported functions, types, enums, traits, interfaces and contracts.
- Generic parameters and bounds.
- Associated types where present.
- Async function effects.
- Result/Option public error types.
- Visibility and stability markers.
- Signature hash/report metadata.

Public API checks reject:

- `any` exposure.
- Private type leaks.
- Unresolved public bounds.
- Async APIs without effect exposure.
- Result APIs with private error types.
