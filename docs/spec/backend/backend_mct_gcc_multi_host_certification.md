# MCT GCC Multi-Host Certification

Status: certification command and CI fan-in contract integrated for the
declared GCC bridge subset.

This gate certifies the current MCT/GCC C99 bridge surface as an executable
corpus, not as full GCC parity. The local command runs the declared fixture
corpus on the current host and emits a host report. The aggregate mode consumes
retained host reports from CI and emits a multi-host certificate only when two
or more distinct host reports are present.

## Declared Corpus

- `examples/backend/mct_gcc_scalar_bridge.n`.
- `examples/backend/mct_gcc_vector_struct_access.n`.
- `examples/backend/mct_gcc_pointer_region_access.n`.
- `examples/backend/mct_gcc_aggregate_abi.n`.

The corpus covers scalar/control-flow/print lowering, `vector<Struct>` dynamic
descriptors, pointer/region intrinsics and aggregate ABI params/returns through
the host C ABI.

## Schemas

- `nc.mct.gcc.host_certification.v1`: one host report produced by running the
  declared corpus.
- `nc.mct.gcc.multi_host_certification.v1`: fan-in report over retained host
  reports.

Both schemas report toolchain availability honestly. Missing `gcc`/`cc` is
`missing_tools`, not a pass.

## CLI

```sh
ncc mct gcc-certify --out-dir target/tmp/mct-gcc-certify --out target/tmp/mct-gcc-certification.json --json --cc nC.sysv64 --allow-missing-tools
ncc mct gcc-certify --input target/tmp/mct-gcc-certification.json --out target/tmp/mct-gcc-multi-host-certification.json --json
```

## Truth Boundary

- `declared_gcc_bridge_adapter = true`.
- `full_gcc_parity = false`.
- single-host reports do not claim multi-host certification.
- multi-host certification requires retained reports from at least two distinct
  hosts.
- exhaustive per-target aggregate ABI classification independent of the host C
  compiler remains outside this gate.
