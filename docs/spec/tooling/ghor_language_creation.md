# ghor Language Creation

`ghor lang new <name> --host <host>` creates a language-forge scaffold under ghor orchestration.

Supported hosts:

- `nC`
- `c`
- `cpp`
- `rust`
- `java`
- `python`
- `javascript`
- `typescript`

The generated layout includes:

- `nc.toml`
- lexer surface
- parser surface
- AST surface
- diagnostics surface
- semantic analysis surface
- IR surface
- lowering surface
- backend surface
- runtime surface
- tests
- examples
- docs
- conformance scaffold report

For `--host nC`, the scaffold records machine origin and can integrate with `ncc`, `[@compiler]`, `[@runtime]`, `[@machine]` and later basal support when the language project declares it.

For foreign hosts, `ghor` creates adapter build metadata and host-language files, but it does not convert that host into a native ghor dependency. Host tools are used only by explicit adapter configuration.

Baseical smoke:

```sh
ghor lang new lumen --host nC --out target/tmp/lumen_nc
ghor lang new lumen_c --host c --out target/tmp/lumen_c
ghor lang new lumen_py --host python --out target/tmp/lumen_py
```
