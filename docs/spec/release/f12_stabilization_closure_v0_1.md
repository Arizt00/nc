# nC v0.1 — F12 estabilización integral (línea activa)

Fecha de corte: 2026-04-30

Este documento declara el cierre operativo de F12 en la línea activa del repositorio.

## Objetivo de F12

Convertir los cierres técnicos previos (F1..F11) en una entrega defendible para terceros, sin degradar base ni tipado interno.

## Criterios de cierre y cumplimiento

1. Repositorio entregable
- `README.md` de entrada con quickstart real y rutas de documentación canónica.
- superficies estables publicadas por fase.

2. Handoff limpio
- estado binario sincronizado en:
  - `_archive/deprecated/context_legacy/status/ACTA_CIERRE_FASES_2026-04-29.md`
- convergencia de estado operativo en:
  - `_archive/deprecated/context_legacy/roadmap/ROADMAP_MASTER.md`
  - `_archive/deprecated/context_legacy/status/OPEN_ITEMS.md`

3. Base sólida para evolución posterior
- límites de alcance v0.1 explícitos (sin deuda oculta).
- evolución futura tratada como versionada, no como parche sobre cierres activos.

## Superficie oficial estable (v0.1 activa)

- F1 `[@]`
  - referencia: `docs/spec/root/root_technical_closure_v0_1.md`
- F2 `[@machine]`
  - referencias:
    - `docs/spec/machine/machine_surface_official_v0_1.md`
    - `docs/spec/machine/machine_technical_closure_v0_1.md`
- F3 `primitive[@]`
  - referencias:
    - `docs/spec/primitive/primitive_surface_official_v0_1.md`
    - `docs/spec/primitive/primitive_technical_closure_v0_1.md`
- F4 linking soberano
  - referencias:
    - `docs/spec/root/linking_block_e_closure_definition_v0_1.md`
    - `docs/spec/root/linking_resolution_precedence_v0_1.md`
    - `docs/spec/root/linking_context_authority_matrix_v0_1.md`
    - `docs/spec/root/linking_diagnostics_official_matrix_v0_1.md`
- F5 memoria/punteros/regiones/alias
  - referencias:
    - `docs/spec/root/memory_pointer_flow_static_guards_v0_1.md`
    - `docs/spec/root/pointer_compatibility_official_matrix_v0_1.md`
    - `docs/spec/root/memory_heap_manager_official_matrix_v0_1.md`
- F6 runtime profundo
  - referencia: `docs/spec/runtime/runtime_event_audit_f6_tranche_v0_1.md`
- F7 inspect
  - referencias:
    - `docs/spec/inspect/inspect_mode_contracts_v0_1.md`
    - `docs/spec/inspect/inspect_runtime_query_paths_v0_1.md`
- F8 security
  - referencia: estado consolidado en `_archive/deprecated/context_legacy/status/ACTA_CIERRE_FASES_2026-04-29.md`
- F9 network
  - referencia: estado consolidado en `_archive/deprecated/context_legacy/status/ACTA_CIERRE_FASES_2026-04-29.md`
- F10 capas altas
  - referencia: `docs/spec/high/f10_high_layer_mode_contracts_v0_1.md`
- F11 ABI/lowering/backend
  - referencia: `docs/spec/backend/abi_lowering_contracts_v0_1.md`

## Diagnósticos canónicos (índice operativo)

- Root/linking/imports:
  - `docs/spec/root/linking_diagnostics_official_matrix_v0_1.md`
  - `docs/spec/root/import_use_diagnostics_family_matrix_v0_1.md`
- Machine:
  - `docs/spec/machine/machine_diagnostics_official_v0_1.md`
- Primitive:
  - `docs/spec/primitive/primitive_diagnostics_official_v0_1.md`

## Suite canónica de ejemplos (entrada terceros)

- Root: `examples/root_at/`
- Machine: `examples/machine/legacy_at/`
- Primitive: `examples/primitive_at/`
- F10 capas altas: `examples/f10_at/`
- Inspect: `examples/inspect_at/`
- Security: `examples/security_at/`
- Network: `examples/network_at/`
- Import/Use catalog: `examples/import_use_families/`
- Linking soberano: `examples/linking_l*.n`, `examples/l1*.n`, `examples/l2*.n`
- Memoria/punteros/runtime: `examples/pointer_phase_*.n`, `examples/l2k_s7*.n`, `examples/l2k_s8*.n`

## Comandos mínimos de verificación

```bash
cargo test --workspace -- --nocapture
```

```bash
cargo run -p ncc-cli -- check examples/root_at/root_at_mature_suite_ok.n
cargo run -p ncc-cli -- check examples/machine/legacy_at/machine_at_flow_full_ok.n
cargo run -p ncc-cli -- check examples/primitive_at/primitive_at_boot_serial_material_ok.n
```

## Fuera de alcance v0.1 activa (evolución versionada)

- ampliación multi-target profunda fuera de perfiles ABI activos;
- formatos nativos extra fuera del pipeline actual;
- linker/loader in-repo de nueva generación;
- expansión de superficies avanzadas de capas altas (`[@ai]`, políticas avanzadas).

Estos puntos no son deuda del cierre activo F12: son frente versionado posterior.

## Nota de transición a v1.0.0

La ruta mayor v1.0.0 se rige por:

- `_archive/deprecated/context_legacy/roadmap/ROADMAP_MASTER.md` (sección 25)
- `docs/spec/release/v1_0_roadmap_obligatorio.md`
- `docs/spec/release/v1_0_acceptance_matrix.md`
