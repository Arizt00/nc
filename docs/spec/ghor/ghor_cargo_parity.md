# ghor Cargo-Like Parity

`ghor` mirrors the practical command families developers expect from Cargo:

- `new` / `init`
- `check`
- `build`
- `run`
- `test`
- `bench`
- `clean`
- `fmt`
- `lint`
- `doc`
- `package`
- `publish`
- `install`
- `uninstall`
- `update`
- `tree`
- `metadata`
- `manifest`
- `search`
- `info`
- `login`
- `owner`
- `yank`
- `vendor`
- `workspace`

This parity is orchestration parity plus a closed ghor registry/network protocol. ghor does not clone Cargo's full crates.io service model, but the package registry surface is real end to end for declared ghor packages:

- filesystem registries use `index.json` and `packages/<name>/<version>/`;
- HTTP(S) registries use `GET` for index/package fetch and `PUT` for publish/yank index updates;
- `publish` writes archive bytes, a package manifest, SHA-256 checksum data, and an index entry;
- `fetch` reads the index, downloads package bytes, verifies the registry checksum, verifies the manifest-declared checksum, and writes a local cache artifact;
- `search`, `info`, and `yank` operate on the same index contract;
- `login` stores optional bearer credentials for remote registries without printing tokens.
- `.cargo/config.toml` controls named registries, `registry.default`, credential providers, offline mode, retry, proxy, timeout, certs, and git registry fetch config.
- `update`, `vendor`, `tree`, `metadata`, `install`, `owner`, `login`, and `logout` are closed as Cargo-compatible ghor command surfaces; see [ghor_cargo_commands.md](ghor_cargo_commands.md).

External compiler execution must still be provided by real configured tools. ghor reports absent optional tools honestly instead of recording unobserved evidence.
