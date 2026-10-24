# v2.0.0 Phase10 — Plugin Marketplaces Spec

## Publication contract

Each publishable plugin package must include:

- product plugin version `2.0.0.2`
- professional IDE parity tier `c_cpp_rust_java_grade`
- official SPDX: `MIT OR Apache-2.0`
- repository metadata
- mandatory license files (`LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`)
- smoke/build evidence
- theme assets and base palette link

## Prohibited publication states

- missing `repository`
- ambiguous license string (`MIT/Apache`, `Dual`, etc.)
- missing license files
- command surface claiming non-existent CLI capabilities

## Base note

Plugin packaging does not alter semantic authority.
`ncc` remains the sole semantic source.
