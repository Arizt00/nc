# BindIR

Status: closed for Basal Fase 3.

`BasalBindIr` models syntax-level bindings as semantic IR:

- bind id.
- source symbol reference.
- destination symbol reference.
- destination kind.
- source/destination spans.
- resolution status.
- optional machine reference marked pending.

Local `root.site` destinations resolve to `SiteIR` in Phase 3. Machine destinations remain `pending_machine_resolution`; resolving them is Basal Fase 4 lowering, not Phase 3.
