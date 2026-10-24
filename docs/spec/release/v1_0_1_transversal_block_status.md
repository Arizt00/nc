# nC v1.0.1 — Bloque transversal previo a fase final

Fecha de corte: 2026-04-30

## Objetivo

Ejecutar la profundización obligatoria previa a la fase final sin reabrir cierres estables de `v1.0.0`.

## Alcance obligatorio (línea v1.0.1)

1. tooling más sofisticado;
2. expansión de ISA de `[@machine]`;
3. ecosistema IDE profundo;
4. profundización radical de `primitive[@]`;
5. profundización de `[@]`.

## Estado actual del bloque (2026-04-30)

Bloque transversal v1.0.1: **cerrado** en la línea activa.

### A. Profundización `[@]` (cerrado)

- parser: símbolo cualificado sin llamada ahora se acepta como `Expr::ContextRef` tipado.
- se elimina rechazo parser de rutas cualificadas no invocadas.
- parser/sema: desambiguación canónica de contexto polimorfo `>>>`:
  - cierre genérico anidado profundo (`list<list<list<int>>>`) sin degradar a shift textual;
  - `>>` en expresión preservado como operador de shift.
- evidencia:
  - test parser `root_at_qualified_symbol_without_call_parses_as_context_ref`.
  - test parser `root_at_nested_generic_triple_gt_closure_and_shift_right_are_disambiguated`.
  - test sema `root_nested_generic_triple_gt_closure_and_shift_right_passes_in_sema`.

### B. Tooling + IDE (cerrado tranche-2)

- se incorpora `nide` como herramienta oficial:
  - bin dedicado (`--bin nide`);
  - alias ejecutable (`nide -> nide`);
  - comandos:
    - `nide outline <file.n> [--out <path>]`
    - `nide symbols <file.n> [--out <path>]`
    - `nide diagnostics <file.n> [--out <path>]`
- salida JSON tipada, sin fallback textual en el core semántico.
- `nide diagnostics` ahora expone fase y estructura posicional canónica:
  - `phase` (`lexer` | `parser` | `sema`);
  - `error_entries[]` con `message`, `line`, `column`.
- evidencia:
  - tests CLI:
    - `nide_outline_report_includes_context_and_sections`
    - `nide_diagnostics_report_surfaces_sema_failure`
    - `nide_diagnostics_report_surfaces_parser_failure_with_line_and_column`
    - `executable_alias_maps_to_tooling_command` (incluye `nide`).

### C. ISA `[@machine]` (tranche-1 cerrado)

- expansión tipada incorporada:
  - `lea <dst>, mem[...]`
  - `bsf <dst>, <src>`
  - `bsr <dst>, <src>`
  - `bswap <reg>`
- continuidad cerrada parser → AST → sema → IR → backend/object:
  - AST: `MachineInst::{Lea,Bsf,Bsr,Bswap}`.
  - parser: parseo canónico de las cuatro formas.
  - sema: validación de registros/memoria con diagnósticos canónicos dedicados.
  - IR: lowering tipado a `MachineInstIR::{Lea,Bsf,Bsr,Bswap}`.
  - backend: emisión nativa x86_64 para las cuatro instrucciones.
- evidencia:
  - parser: `machine_at_v1_0_1_isa_ok_parses`
  - sema: `machine_v1_0_1_isa_ok_passes_in_sema`
  - IR/backend:
    - `lower_machine_extended_isa_into_machine_ir`
    - `emit_machine_object_encodes_extended_isa_native_when_registers_are_typed`
  - fixtures:
    - `machine_at_isa_v1_0_1_ok.n`
    - `machine_at_lea_unknown_base_fail.n`
    - `machine_at_bsf_unknown_src_fail.n`
    - `machine_at_bswap_unknown_fail.n`

### D. Profundización `primitive[@]` (tranche-2 cerrado)

- endurecimiento semántico material aplicado en secciones nombradas:
  - `window ... requires signature <name>` ahora se valida contra tabla de `signature` declaradas;
  - `entry ... requires <name>` y `entry ... requires signature <name>` comparten resolución tipada canónica en sema.
  - contratos de cardinalidad/correlación por sección:
    - `operation`: máximo una cláusula `capability`;
    - `window`: máximo una cláusula `requires signature ...`;
    - `signal`: máximo una cláusula `source`, máximo una `field`, y `field` requiere `source`.
- continuidad cerrada parser → AST → sema → IR/backend:
  - sema: diagnóstico canónico nuevo para referencia `window -> signature` no resuelta;
  - backend: `window_contracts` tipados con `requires_signature_refs` en unidad primitive/meta.
- evidencia:
  - sema:
    - `primitive_window_requires_unknown_signature_fails_in_sema`
    - `primitive_entry_unknown_signature_requires_keyword_form_fails_in_sema`
    - `primitive_operation_duplicate_capability_clause_fails_in_sema`
    - `primitive_signal_field_without_source_clause_fails_in_sema`
    - `primitive_window_duplicate_requires_signature_clause_fails_in_sema`
  - backend:
    - `emit_backend_materializes_typed_primitive_contracts_for_boot_serial_fixture` (incluye `window_contracts`).
  - fixtures:
    - `primitive_at_window_requires_unknown_signature_fail.n`
    - `primitive_at_entry_requires_signature_unknown_fail.n`

## Evidencia ejecutada en este corte

Comandos:

```bash
cargo test -p ncc-parser root_f0_tests:: -- --nocapture
cargo test -p ncc-cli nide_ -- --nocapture
cargo test -p ncc-cli executable_alias_maps_to_tooling_command -- --nocapture
cargo test -p ncc-parser root_f0_tests::root_at_nested_generic_triple_gt_closure_and_shift_right_are_disambiguated -- --nocapture
cargo test -p ncc-sema root_sema_tests::root_nested_generic_triple_gt_closure_and_shift_right_passes_in_sema -- --nocapture
cargo test -p ncc-sema primitive_operation_duplicate_capability_clause_fails_in_sema -- --nocapture
cargo test -p ncc-sema primitive_signal_field_without_source_clause_fails_in_sema -- --nocapture
cargo test -p ncc-sema primitive_window_duplicate_requires_signature_clause_fails_in_sema -- --nocapture
cargo test --workspace -- --nocapture
```

Resultado: `OK` en todos los comandos.

Gate automático adicional de cierre transversal:

```bash
cargo test -p ncc-cli release_v1_0_1_transversal_gate_tests -- --nocapture
```

## Estado de cierre

1. `[@]` tranche-2: cerrado.
2. tooling + IDE tranche-2 (`nide` + `diagnostics` posicional): cerrado.
3. `[@machine]` tranche-1 ISA (`lea/bsf/bsr/bswap` tipado E2E): cerrado.
4. `primitive[@]` tranche-2 (cardinalidad/clases `operation/window/signal` + `window->signature`): cerrado.
5. gate automático `release_v1_0_1_transversal_gate_tests` activo en `ncc-cli`: cerrado.
