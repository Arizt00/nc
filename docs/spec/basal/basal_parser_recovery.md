# Basal Parser Recovery

The Phase 1 parser recovers after malformed sections by synchronizing at section labels and root braces.

Recovery sync surfaces:

- `;`
- `}`
- `root:`
- `target:`
- `site:`
- `bind:`
- `seal:`
- `evidence:`
- `meta:`

When recovery reports more than one syntax error, the parser also emits `E_BASAL_RECOVERY_001`. Recovery is for diagnostics only; it does not make invalid input pass.
