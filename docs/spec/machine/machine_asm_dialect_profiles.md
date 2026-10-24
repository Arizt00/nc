# Machine ASM Dialect Profiles

Fase 11 closes historical ASM dialect import/export for declared nC profiles only. `.nm` remains official. The closed profiles are `nC-nasm-compat-v1`, `nC-yasm-compat-v1`, `nC-gas-intel-compat-v1`, `nC-gas-att-compat-v1`, `nC-masm-compat-v1`, `nC-fasm-compat-v1`, and `nC-llvm-mc-compat-v1`.

Every profile requires a manifest, import, export, import-back verification, a loss model, positive and negative examples, diagnostics, docs, and reports. Fase 12 linker integration remains out of scope.

Features included in a declared profile are closed at feature depth, not as smoke coverage. In this phase that specifically means:

- MASM `DUP` is fully profiled and closed in `nC-masm-compat-v1`.
- FASM `repeat` is fully profiled and closed in `nC-fasm-compat-v1`.
- FASM `macro` is fully profiled and closed in `nC-fasm-compat-v1`.

No profile may remain `closed` if one of its declared deep features is partial, deferred, or accepted only diagnostically.
