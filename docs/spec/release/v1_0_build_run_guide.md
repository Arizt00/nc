# nC v1.0.0 — Guía de compilación y ejecución

## 1. Verificación de semántica

```bash
cargo run -p ncc-cli --bin ncc -- check examples/l2k_s8c_runtime_inspect_runtime_paths_ok.n
```

## 2. Ejecución runtime

```bash
cargo run -p ncc-cli --bin ncc -- run examples/l2k_s8c_runtime_inspect_runtime_paths_ok.n
```

## 3. Emisión de artefactos

```bash
cargo run -p ncc-cli --bin ncc -- emit-elf examples/machine/legacy_at/machine_at_strict_ok.n /tmp/machine.o
cargo run -p ncc-cli --bin ncc -- emit-coff examples/machine/legacy_at/machine_at_strict_ok.n /tmp/machine.obj
cargo run -p ncc-cli --bin ncc -- emit-macho examples/machine/legacy_at/machine_at_strict_ok.n /tmp/machine_macho.o
```

## 4. Build ejecutable

```bash
cargo run -p ncc-cli --bin ncc -- build-exe examples/linking_l9_full_ok.n /tmp/linking.out --cc nC.sysv64
```

## 5. Perfil backend

Perfiles aceptados:

- `nC.sysv64`
- `nC.win64`
- `nC.aapcs64`

Las combinaciones inválidas de artefacto/perfil fallan con diagnóstico canónico.

## 6. Scripts oficiales de release (F24)

```bash
tools/release/repro_build_v1_0.sh
tools/release/smoke_v1_0.sh
```
