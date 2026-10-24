# API Stability

Phase 10 closes basic API stability metadata.

Supported markers:

- `stable`
- `experimental`
- `deprecated`

The active implementation reports stability metadata and rejects unsafe public surfaces. Full semver resolution, package registry policy and external publishing are deferred.

If a future baseline manifest is supplied, compatibility checks can report breaking changes with `E_HIGH_API_BREAKING_CHANGE_001`; Phase 10 does not implement a registry or version solver.
