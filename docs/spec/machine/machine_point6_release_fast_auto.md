# Machine Point 6: release-fast desde el build normal

Status: closed as a machine-consumer contract.

## Papel de machine

Machine no recibe una ISA nueva en este punto. La ampliacion es de integracion:
el backend normal entrega a machine artefactos ya optimizados por la ruta
release-fast automatica.

El build conserva:

- `.nm` bridge verificado por `machine check`;
- `.obj` de programa;
- `.entry.obj` de entrada machine;
- `.exe` enlazado por el linker interno o por la ruta configurada;
- metricas de bytes y tiempo en `release_fast_auto`.

## Contrato visible para machine

`release_fast_auto` declara:

- `normal_build_surface: true`;
- `special_command_required: false`;
- `optimization.effective: O3` cuando no hay `-O` explicito;
- `artifact_metrics.program_object_bytes`;
- `artifact_metrics.machine_entry_object_bytes`;
- `artifact_metrics.executable_bytes`.

## Frontera ISA

No se anadio codificacion nueva. Si un futuro caso release-fast selecciona una
instruccion que machine no pueda codificar, ese caso vuelve a machine y se cierra
con fixture de encoding, object emission y link smoke.
