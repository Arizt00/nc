# Machine CPU Profiles

Supported profile reports:

- `baseline`
- `x86_64-v1`
- `x86_64-v2`
- `x86_64-v3`
- `x86_64-v4`
- `skylake`
- `icelake`
- `zen3`
- `zen4`
- `aarch64-generic`
- `apple-m1`
- `apple-m2`
- `neoverse-n1`
- `neoverse-v1`
- `riscv64-generic`
- `sifive-u74`
- `wasm32-generic`
- `generic`

Command:

`ncc machine cpu-profile <profile> --json`

The normal ncc backend also accepts `--cpu <profile>` and `--cpu native`
through build/run/emit/optimize commands. x86_64-v2/v3/v4, Intel, AMD Zen and
AArch64 profiles gate real SIMD row selection and scheduling in release-fast.
RISC-V profiles are present in the CPU catalog for object/link planning; the
high-level `.n` backend still rejects them until a RISC-V `BackendTargetIR`
lane is exposed.
