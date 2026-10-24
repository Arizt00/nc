# ghor Machine Integration

`ghor` is machine-first.

It consults and routes to `ncc machine` for:

- `machine doctor`;
- `machine targets`;
- `machine abi`;
- `machine verify-object`;
- `machine disasm`;
- `machine roundtrip`;
- `machine parity`;
- machine conformance/evidence reports.

`ghor doctor` requires repository `machine doctor` to pass and `machine asm-complete native-doctor --json` to report `asm_complete_total=true`.

If an installed `ncc` reports old basal or machine gates while the repository toolchain reports closure, `ghor` reports `E_GHOR_TOOLCHAIN_STALE_001`.
