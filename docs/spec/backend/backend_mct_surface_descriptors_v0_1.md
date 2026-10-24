# MCT Surface Descriptors v0.1

Este documento fija el contrato inicial para materializar construcciones de
superficie `.n` dentro de MCT sin tratarlas como informacion interpretada por
runtime.

## Objetivo

`stc`, `local`, `type`, `fn`, `move fn`, `const fn`, `probe`, `public`,
`private`, `internal`, `strictly` y `void` deben llegar al compilador como
material estructurado. El parser sigue produciendo AST, pero el lowering a IR
emite ademas descriptores MCT con bytes deterministas.

Esto permite que backend, diagnosticos, bridges y herramientas puedan auditar
esas piezas sin reconstruirlas desde texto fuente.

## Formato NCSD

Cada descriptor usa el esquema `nc.mct.surface_descriptor.v1` y una carga de
bytes con este layout:

```text
magic      = "NCSD"
version    = u8(1)
construct  = len_u16 + utf8
container  = len_u16 + utf8
symbol     = len_u16 + utf8
nc_type    = len_u16 + utf8
visibility = len_u16 + utf8
flags_len  = u16
flags      = repeated(len_u16 + utf8)
payload    = len_u16 + utf8
```

El fingerprint se calcula sobre esos bytes con FNV-1a 64-bit y se expone como
cadena hexadecimal de 16 caracteres. No es una garantia criptografica; es una
huella estable para trazabilidad y tests.

## Constructos cubiertos

Los descriptores iniciales cubren:

- Secciones: `section.stc`, `section.local`, `section.fn`,
  `section.move_fn`, `section.const_fn`, `section.probe`.
- Simbolos: `stc.symbol`, `local.var`, `local.const`, `local.metadata`.
- Tipos: `type.alias`, `type.record`, `type.field`, `type.contract`,
  `type.void`.
- Funciones: `function.fn`, `function.move_fn`, `function.const_fn`.
- Autoridad: `visibility.public`, `visibility.private`,
  `visibility.internal`, `visibility.strictly`.
- Modificadores: `modifier.strictly`.
- Contenedor: `container.surface`.

## Salida CLI

`mct translate --json` y `mct emit-llvm --json` exponen:

- `surface_descriptor_count`
- `surface_descriptor_bytes`
- `surface_descriptors[]`

Cada elemento incluye `construct`, `container`, `symbol`, `nc_type`,
`visibility`, `flags`, `payload`, `bytes_len`, `bytes_hex` y `fingerprint`.

## Estado

Implementado en `ncc-ir` como `MctSurfaceDescriptorIR` y conectado a la
cobertura de constructs LLVM/MCT. La salida LLVM puede seguir optimizando o
eliminando codigo, pero la superficie del lenguaje ya queda materializada antes
del handoff de proveedor.
