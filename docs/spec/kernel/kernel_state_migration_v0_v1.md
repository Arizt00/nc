# Kernel State Migration v0 to v1

Estado: `closed`

## Esquemas admitidos

El perfil kernel acepta tres estados durante la migracion:

1. v1 completo: `schedulerOpen:bool`, `panicState:bool`,
   `quarantineCount:int`, `lastTask:str`, `lastEvent:str`.
2. v0 completo: `openState:bool`, `scheduled:bool`, `panicked:bool`,
   `quarantined:bool`.
3. Hibrido completo, para una transicion gradual.

Una declaracion v0 parcial se rechaza con `E_KERNEL_STATE_MIGRATION_001`; sema no
adivina los campos ausentes.

## Mapeo

| v0 | v1 / significado |
| --- | --- |
| `openState` | `schedulerOpen` |
| `panicked` | `panicState` |
| `scheduled` | latch compatible de `schedulerNext` |
| `quarantined` | espejo booleano de `quarantineCount > 0` |

El runtime sincroniza ambos nombres al abrir/cerrar el scheduler, seleccionar una
tarea, entrar en panic o poner una tarea en cuarentena. El reconocimiento de v0
solo se activa en contexto kernel, evitando que campos homonimos de `[@system]`
activen por accidente el contrato.

## Forma canonica nueva

El codigo nuevo debe usar v1 y `.n` normal con `--profile kernel`. `[@kernel]` y v0
permanecen como compatibilidad migrable, no como estilo recomendado.

## Evidencia

- `kernel_legacy_state_v0_migrates_explicitly_to_v1`.
- `kernel_partial_legacy_state_is_rejected_with_migration_diagnostic`.
- Modulo `kernel_f18_sema_tests`: 8 aprobadas, 0 fallos.
- Fixtures runtime v2 heredados: aprobados por la suite global.
