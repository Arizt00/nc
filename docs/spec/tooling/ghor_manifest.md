# ghor Manifest

The official ghor project manifest is `nc.toml`.

Required fields:

- `[package].name`
- `[package].version`

Recognized sections:

- `[package]`
- `[toolchain]`
- `[target.default]`
- `[dependencies]`
- `[machine]`
- `[basal]`
- `[runtime]`
- `[compiler]`
- `[primitive]`
- `[languages]`
- `[language_adapter.*]`
- `[profiles.*]`
- `[workspace]`

Remote dependencies must declare a checksum. Local path dependencies may omit checksum because the local path is explicitly trusted by the workspace owner.

Diagnostics:

- `E_GHOR_MANIFEST_MISSING_NAME_001`
- `E_GHOR_MANIFEST_INVALID_VERSION_001`
- `E_GHOR_MANIFEST_INVALID_TARGET_001`
- `E_GHOR_MANIFEST_INVALID_PROFILE_001`
- `E_GHOR_MANIFEST_INVALID_MODE_001`
- `E_GHOR_LANGUAGE_UNSUPPORTED_001`
- `E_GHOR_LANGUAGE_ADAPTER_MISSING_001`
- `E_GHOR_CHECKSUM_001`
