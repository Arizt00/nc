# Basal Evidence Bundle

The final Basal evidence bundle is generated under
`reports/basal/phase7/` and can be exported to
`target/release-evidence/basal/` for release packaging.

The bundle records or references:

- machine doctor output
- machine asm-complete native-doctor output
- basal doctor and basal doctor JSON output
- cargo fmt, check, test, and clippy output
- final `bit_weight_pure_root_sealed` smoke outputs
- examples sweep and negative sweep outputs
- search gate output
- conformance matrix
- final CLI matrix
- worktree classification
- release notes and next-step recommendation

Evidence entries include hashes when files are present. Missing captures must be
reported as missing environment/runtime evidence; they must not be invented.
