# Proveedores manuales de imports

Estado: contrato activo para la migracion del runtime y la biblioteca estandar.

## Regla

Las operaciones alojadas en el prelude de backend no exigen escribir un import
en el consumidor. Manual significa que el programa debe invocar cada operacion,
aportar sus buffers, capacidades, indices y politicas; no que deba repetir una
declaracion `use`:

```nC
workspace >> p<byte> <- stackAlloc(compilerUnitStride(), 8);
compilerParse(workspace);
```

Cuando una familia no pertenece al prelude, `use` solo selecciona nombres y
firma. No crea objetos, no reserva memoria, no inserta limpieza y no activa un
interprete. `print` y `println` conservan deliberadamente el import explicito
`use <io:output>;`.

`seed`, `randomInt`, `randomBool`, `randomFloat`, `randomRange`, `sign` y `abs`
pertenecen al prelude de backend: no exigen `use`, pero cada operacion sigue
escrita de forma explicita por el programa. El prelude solo aporta firma y
simbolo canonico; no ejecuta, reserva ni limpia nada por adelantado.

## Dos clases de materializacion

1. Operaciones implementables en el lenguaje se escriben en nC o se
   materializan directamente como LLVM/Machine interno. El consumidor llama a
   simbolos nC canonicos, con o sin `use` visible segun el prelude.
2. Servicios del sistema operativo se expresan mediante ABI, syscalls, tablas de
   importacion y thunks Machine internos. No se genera un wrapper C.

Los proveedores no pueden contener ni generar `.c`, `.cc`, `.cpp` u Objective-C.
Tampoco pueden enlazar libc, UCRT o MSVCRT para sustituir una implementacion nC
ausente.

## Memoria y errores

- El llamador aporta buffers y punteros de salida cuando la operacion los exige.
- El proveedor no introduce `Result`, ownership o crecimiento oculto.
- Un puntero raw conserva null, aliasing, desalineacion, fuera de rango, UAF y
  double free como comportamiento manual/indefinido.
- Las formas que devuelven almacenamiento deben exponer su liberacion.

## Criterio de cierre por import

Un import deja la cola solo cuando tiene catalogo tipado, parser, sema, IR,
LLVM, Machine, objeto, enlace, ejecucion real y manifiesto. La evidencia debe
probar `runtime_nc.source_language=nC`, `c_source_generated=false` y ausencia de
artefactos C. Una familia hosted heredada no queda cerrada por tener solo una
declaracion o un objeto C compilable.

## Compatibilidad externa

La interoperabilidad con objetos third-party puede seguir aceptando artefactos
externos por ABI explicita. Eso no convierte C en implementacion del lenguaje ni
autoriza a usarlo dentro de la biblioteca estandar.
