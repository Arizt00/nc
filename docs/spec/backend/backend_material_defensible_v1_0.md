# nC v1.0.0 — F20 Backend material defendible (cierre operativo)

Fecha de cierre: 2026-04-30

## 1. Objetivo de F20 en línea activa

Cerrar la capa material del compilador con garantías estrictas de coherencia tipada:

- AST -> sema -> IR -> lowering ABI tipado -> object tipado -> artefacto material.
- Sin degradar zonas tipadas a fallback textual no controlado.
- Con salida reproducible y verificable en CLI.

## 2. Contrato material cerrado

### 2.1 Perfiles ABI/calling convention activos

- `nC.sysv64` (`BackendCallingConventionIR::NcSysV64`)
- `nC.win64` (`BackendCallingConventionIR::NcWin64`)
- `nC.aapcs64` (`BackendCallingConventionIR::NcAapcs64`)

### 2.2 Artefactos CLI activos

- `emit-backend`
- `emit-object`
- `emit-elf`
- `emit-coff`
- `emit-macho`
- `build-exe`
- `nbuild` (front oficial de build ejecutable)

### 2.3 Matriz de compatibilidad perfil/artefacto (estricta)

- `emit-elf`: `nC.sysv64` / `nC.aapcs64`
- `build-exe`: `nC.sysv64` / `nC.aapcs64`
- `emit-coff`: permitido en perfiles activos
- `emit-macho`: `nC.sysv64` / `nC.aapcs64`
- `nC.win64` queda bloqueado para `emit-elf`, `build-exe`, `emit-macho` con diagnóstico canónico.

## 3. Reglas duras de materialización

### 3.1 Salida material obligatoria

Toda operación de emisión/ensamblado que reporte éxito debe dejar archivo material no vacío:

- `emit-elf`
- `emit-coff` (vía `objcopy` o fallback `clang`)
- `emit-macho` (vía `objcopy` o fallback `clang`)
- `build-exe` (linker externo)

Si el proceso externo devuelve éxito pero no materializa salida, se considera error canónico.

### 3.2 Reproducibilidad mínima exigida

Para entrada y perfil idénticos, `emit-elf` debe producir bytes idénticos (determinismo de emisión).

## 4. Diagnósticos canónicos F20 añadidos

- `backend artifact '<cmd>' is incompatible with calling convention nC.win64; use --cc nC.sysv64 or --cc nC.aapcs64 (or emit-coff for Win64 object output)`
- `backend artifact 'emit-macho' is incompatible with calling convention nC.win64; use --cc nC.sysv64 or --cc nC.aapcs64`
- `<artifact> reported success but did not produce output '<path>'`
- `<artifact> reported success but output '<path>' is empty`
- `<artifact> reported success but output '<path>' is not a regular file`

## 5. Evidencia de pruebas (código)

`crates/ncc-cli/src/main.rs`:

- `backend_emit_cli_tests::backend_artifact_profile_rejects_win64_for_elf_exe_and_macho`
- `backend_emit_cli_tests::emit_elf_is_deterministic_for_same_input_and_profile`
- `backend_emit_cli_tests::build_executable_with_fake_linker_materializes_output`
- `backend_emit_cli_tests::build_executable_with_fake_linker_without_output_fails`
- `backend_emit_cli_tests::emit_macho_with_clang_fallback_writes_macho_object`
- `backend_emit_cli_tests::emit_coff_with_clang_fallback_writes_coff_object`

`crates/ncc-ir/src/lib.rs` mantiene cobertura ABI/object/ELF64 y relocs tipados de F11/F20.

## 6. Verificación operacional mínima

```bash
cargo test -p ncc-cli backend_emit_cli_tests:: -- --nocapture
cargo test --workspace -- --nocapture
```

## 7. Cierre binario F20

F20 queda cerrado en la línea activa v1.0.0 porque:

1. hay coherencia tipada material end-to-end;
2. hay validación estricta de compatibilidad artefacto/ABI;
3. hay verificación dura de salida material;
4. existe evidencia reproducible de tests.
