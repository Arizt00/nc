# Prelude DOM manual de backend

Estado: canonico desde 2026-08-08.

## Regla

Las familias `compilerManual`, `aiManual`, `highManual`, `inspectManual` y
`toolingManual` se resuelven sin `use`. No ejecutan trabajo oculto: el programa
reserva la memoria y llama cada operacion de forma explicita.

## Layouts

| Familia | Descriptor | Datos auxiliares |
|---|---:|---|
| compiler | 128 bytes | token 24, node 32, IR 32, diagnostic 40, manifest 48 |
| ai | 64 bytes | id, kind, input, output, weight, bias, flags, state |
| high | 64 bytes | kind, key, value, aux, flags, source, bytes, state |
| inspect | 64 bytes | event, site, value, flags, tick, detail, sequence, state |
| tooling | 64 bytes | kind, path, data, bytes, flags, status, hash, state |

No hay comprobacion de capacidad, ownership, copia implicita ni liberacion. Las
direcciones se consumen con semantica raw.

## Trabajo material

- compiler escanea bytes, materializa tokens/nodos/IR y escribe report/manifest;
- ai calcula puntuaciones y selecciona el registro con puntuacion maxima;
- high evalua operaciones enteras raw, copia y hashea bytes;
- inspect registra, busca y calcula checksums de eventos;
- tooling cuenta lineas, busca bytes, copia y hashea buffers.

Todos los cuerpos forman parte del LLVM y del objeto nC. No se genera ni compila
C para estas familias.
