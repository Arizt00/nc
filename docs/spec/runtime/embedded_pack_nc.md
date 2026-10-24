# Paquetes incrustados genéricos en nC

## Contrato

`runtime/nc/embedded_pack.n` implementa en nC un lector de paquetes ordenados
con claves UTF-8 y valores binarios arbitrarios. El formato de cada registro es:

```text
u16 prefijo_compartido
u16 bytes_sufijo
u32 bytes_valor
byte[bytes_sufijo]
byte[bytes_valor]
```

Los registros se agrupan en bloques. El primer registro de cada bloque usa
prefijo cero. Una consulta reconstruye como máximo el bloque que contiene la
clave solicitada; no descomprime el paquete completo.

Cada especialización publica tres arrays `stc: immutable`:

- offsets `u32` de los registros;
- tabla hash abierta `u32`, con `0xFFFFFFFF` como hueco;
- payload front-coded de bytes.

El lector ofrece `packFind`, `packContains`, `packKeyAt`, `packValueSize` y
`packByteAt`. Por ello sirve para palabras, metadatos, MIME, tablas de protocolo,
firmas, traducciones, catálogos, reglas o cualquier valor binario.

## Lowering y linking

No se fija un nombre de sección específico en la fuente. La declaración
`immutable` expresa la intención del programa y el lowering selecciona la
representación nativa correcta:

| Destino | Sección final |
|---|---|
| ELF/Linux | `.rodata` |
| PE/Windows | `.rdata` |
| Mach-O/macOS | `__TEXT,__const` |

Esta decisión evita el límite de 16 caracteres de los nombres de sección
Mach-O y garantiza memoria no escribible. Cada paquete vive en su propia unidad
nC y solo se importa desde sus consumidores; las unidades no alcanzables se
pueden descartar antes o durante el linking.

`rodata.ncc;` es una declaración de componente del compilador, no un módulo de
`project`. En una fuente única habilita tanto la colocación física explícita
como la API integrada `rodataNcc`. La API se incorpora de forma perezosa: si el
programa no la referencia, no se inyectan el lector ni el corpus. Por tanto, una
aplicación autocontenida puede consultar `rodataNcc.lexiconContains(...)` sin
transportar `lexicon_es_core.n` ni otro archivo exterior.

## Integridad funcional

El juego de ejemplo ejecuta al arrancar una prueba positiva, Unicode, negativa
y de reconstrucción de bloque. El corpus no se abre desde disco ni usa red en
ejecución. Sus 8.760 raíces españolas proceden de `es_ES.dic` de LibreOffice.
