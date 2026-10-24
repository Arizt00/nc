# MCT GCC Aggregate ABI Bridge

Status: integrated for the declared C99 bridge surface.

This phase closes aggregate parameters and returns in the GCC bridge for the
declared nC surface that already lowers to `ProgramIR`.

## Closed Surface

- Struct parameters by value.
- Struct returns by value.
- Dynamic descriptor parameters by value.
- Dynamic descriptor returns by value.
- Calls that pass or receive aggregate values.
- Field access after aggregate return.
- Descriptor access after descriptor return.

The generated C delegates the physical calling convention to the host C
compiler. This is the correct GCC bridge boundary: ncc owns typed lowering and
layout metadata, while `gcc`/`cc` owns the platform C ABI for the emitted C99.

## Boundary

This is not a claim of independent per-target ABI classification for every
possible aggregate shape. That remains a separate certification lane for
multi-host evidence and deeper provider conformance.

## Evidence

```sh
cargo test -p ncc-ir mct_gcc_bridge --all-targets -- --nocapture
cargo test -p ncc-cli mct_cli_gcc_smoke_aggregate_abi_when_gcc_is_available --lib -- --nocapture
ncc mct gcc-smoke examples/backend/mct_gcc_aggregate_abi.n --out-dir target/tmp/mct-gcc-aggregate-abi --json --cc nC.sysv64 --allow-missing-tools
```
