# Machine AVX-512 Feature Gates

AVX512F foundation forms require the `avx512f` target feature. Later subfeatures require their own explicit target features and remain blocked until their approved phase.

The primary Phase 4A feature diagnostic is `E_MACHINE_AVX512_FEATURE_REQUIRED_001`. Deferred subfeature use reports `E_MACHINE_AVX512_SUBFEATURE_001`.
