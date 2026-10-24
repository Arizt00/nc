# nC Phase10 File Association + Global Execution Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad de asociación

1. `.n` DEBE mantenerse como extensión canónica high-level.
2. `.nc` PUEDE mantenerse como alias high-level del compilador y LSP, pero NO DEBE ser reclamado por plugins ni temas de iconos.
3. `.nb/.nbb/.nm/.ncp` NO DEBEN perder su rol low-level soberano ni sus tipos de archivo separados.

## Compatibilidad de ejecución global

1. `ncup` DEBE mantener `NCUP_HOME/bin` como superficie oficial de comandos globales.
2. `ncc` NO DEBE requerir `cwd` dentro del repo para ejecutar comandos base.

## Compatibilidad de runtime/stdlib

1. `runtime_stdlib.v1.json` DEBE mantenerse en la toolchain instalada.
2. `ncc stdlib-model` DEBE preservar la prioridad `toolchain_installation` en contexto `ncup` válido.
3. Si no hay contexto `ncup` válido, DEBE existir fallback explícito `embedded_fallback`.

## Compatibilidad de cambios futuros

Cambios breaking en asociaciones, rutas de ejecución global o política de resolución runtime/stdlib REQUIEREN RFC aprobado.

## Gate obligatorio

`release_phase10_file_association_global_exec_gate_tests` DEBE validar este contrato.
