# nC v1.0.0 — Guía de instalación y uso

## 1. Requisitos

- Rust toolchain estable.
- Entorno Linux/WSL recomendado.
- `clang`/`objcopy` opcionales para formatos adicionales.

## 2. Verificación inicial

```bash
cargo test --workspace -- --nocapture
```

## 3. Uso base del compilador

```bash
cargo run -p ncc-cli --bin ncc -- check <file.n>
cargo run -p ncc-cli --bin ncc -- run <file.n>
```

## 4. Emisión backend/material

```bash
cargo run -p ncc-cli --bin ncc -- emit-backend <file.n>
cargo run -p ncc-cli --bin ncc -- emit-object <file.n>
cargo run -p ncc-cli --bin ncc -- emit-elf <file.n> <out.o>
```

## 5. Build ejecutable

```bash
cargo run -p ncc-cli --bin ncc -- build-exe <file.n> <out> [--cc <profile>] [--linker <bin>] [--entry <symbol>]
```

## 6. Release smoke (F24)

```bash
tools/release/repro_build_v1_0.sh
tools/release/smoke_v1_0.sh
```
