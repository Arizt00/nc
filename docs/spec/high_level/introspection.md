# High-Level Introspection

Introspection is the safe half of Fase 12. It lets code and CLI reports observe metadata that the compiler already knows from earlier closed phases.

The introspection model includes:

- Type metadata from Fase 2-7.
- Module, package, export, and public API metadata from Fase 10.
- Closure signature metadata from Fase 11.
- Effect metadata from Fase 9.

Introspection may be used by reports and diagnostics. It cannot execute generated code or bypass visibility.
