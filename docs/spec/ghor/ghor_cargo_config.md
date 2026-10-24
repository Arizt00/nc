# ghor Cargo Config Compatibility

Status: closed for Cargo-style registry/network configuration.

References:

- Cargo configuration: https://doc.rust-lang.org/cargo/reference/config.html
- Cargo registries: https://doc.rust-lang.org/cargo/reference/registries.html
- Cargo registry authentication: https://doc.rust-lang.org/cargo/reference/registry-authentication.html
- Cargo source replacement: https://doc.rust-lang.org/cargo/reference/source-replacement.html

`ghor` reads `.cargo/config.toml` and `.cargo/config` as an effective configuration layer for registry-facing commands. This is not a new package semantics layer; it selects registries, auth, and network behavior for the registry protocol already closed in the previous ghor phases.

The nC stage1 module `cargo_config.n` is the material authority for discovery,
merge precedence, named registries, source replacement, offline decisions and
HTTP/Git policy files. Relative CA paths are resolved against the file that
declares them. Tokens are redacted before the effective model is printed.

## Config Discovery

`ghor` merges configuration from:

- `$CARGO_HOME/config.toml`
- `$CARGO_HOME/config`
- `~/.cargo/config.toml`
- `~/.cargo/config`
- `.cargo/config.toml` and `.cargo/config` from path ancestors down to the package/workspace root

Later, more local files override earlier values. The effective config is visible without leaking tokens:

```bash
ghor registry cargo-config --manifest-path ghor.toml --json
```

## Named Registries

Supported Cargo-style forms:

```toml
[registry]
default = "mirror"

[registries]
mirror = { index = "sparse+https://registry.example/index" }

[registries.internal]
index = "git+https://registry.example/index.git"
credential-provider = "cargo:token"
```

Registry commands resolve `--registry <name>` and `registry.default` to the configured index. Supported transports remain:

- filesystem registry
- `sparse+http(s)`
- HTTP(S) registry API/index
- git registry index

## Credential Providers

Token lookup precedence:

1. `--token`
2. `GHOR_REGISTRY_TOKEN`
3. `CARGO_REGISTRIES_<NAME>_TOKEN`
4. `GHOR_REGISTRIES_<NAME>_TOKEN`
5. `[registries.<name>] token`
6. `[registry] token`
7. registry-specific `credential-provider`
8. `[registry] credential-provider`
9. `[registry] global-credential-providers`
10. `~/.ghor/credentials.json`

Supported provider behavior:

- `cargo:token`
- `cargo:token-from-stdout <command> <args>`
- `[credential-alias]` command arrays
- external provider command execution with `CARGO_REGISTRY_INDEX_URL` and `CARGO_REGISTRY_NAME_OPT`

`ghor registry cargo-config` reports only credential availability; it never prints the token.

Direct tokens, `cargo:token` storage lookup and external providers execute in
the native nC stage1. Stage1 resolves an alias to an argv sequence, applies
platform quoting, exports `CARGO_REGISTRY_INDEX_URL` and
`CARGO_REGISTRY_NAME_OPT`, captures stdout through the raw `process:command`
ABI, enforces a 30-second timeout and accepts only exit code zero. The captured
token remains in private process memory and is represented only as
`token_available`; reports never serialize its bytes.

The backend does not insert a shell. A provider invokes `sh`, CMD or
PowerShell only when that executable appears explicitly in its configured argv.

## Network Configuration

Supported network fields:

```toml
[net]
offline = true
retry = 4
git-fetch-with-cli = true

[http]
proxy = "http://proxy.example:8080"
timeout = 30
cainfo = "certs/ca.pem"
proxy-cainfo = "certs/proxy-ca.pem"
check-revoke = false

[git]
proxy = "http://proxy.example:8080"
cainfo = "certs/git-ca.pem"
ssl-verify = true
```

`net.offline` and `--offline` block remote registry access with `E_GHOR_NET_OFFLINE_001`. Filesystem registries continue to work in offline mode.

HTTP registry calls apply:

- proxy
- timeout/connect-timeout
- retry
- CA bundle
- proxy CA bundle
- revoke-check disabling where curl supports it

Git registry index calls use the git executable and apply compatible `git -c` settings for proxy and TLS verification.

## Source Replacement

`ghor` continues to support `[source.<name>]` declarations from manifests and Cargo config:

```toml
[source.crates-io]
replace-with = "mirror"

[source.vendor]
directory = "vendor"
```

Replacement sources can point to configured named registries, local registries, directories, or git sources. The resolver reports the effective source in JSON.

## Tests

Coverage:

- `ghor_cargo_config_named_registry_default_fetch_ok`
- `ghor_cargo_config_credential_provider_alias_ok`
- `ghor_cargo_config_offline_remote_deferred_fail`
- `native_nc_stage1_parses_a_real_manifest` (direct stage1 config, native
  credential-provider execution/capture, redaction, external source replacement
  and native launcher delegation)

These tests verify named default registry fetch, real redacted
credential-provider availability, and remote offline blocking. The same fixture
executes on Windows x86_64 and Ubuntu x86_64; Darwin shares the POSIX ABI but
still requires a macOS runner for execution evidence.
