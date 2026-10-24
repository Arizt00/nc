# Stop World Tour nC

Proyecto escrito íntegramente en nC. `lexicon_es_core.n` es fuente nC generada
con 8.760 raíces verificadas del diccionario Hunspell español de LibreOffice.
El ejecutable no abre diccionarios, no usa red y no necesita archivos de datos.

El paquete emplea front-coding en bloques de 8, índice hash abierto y datos
`immutable`. `embedded_pack.n` es el lector genérico para texto o registros de
bytes; `lexicon_es_core.n` es una especialización para palabras. La fuente
habitual no conoce la sección física elegida por el backend.

La categoría es una regla del juego; el corpus comprueba que la palabra existe.
Los dominios finitos (países, colores y elementos químicos) deben distribuirse
como paquetes exhaustivos, nunca rellenarse con nombres inventados para alcanzar
mil entradas.

Origen del corpus: LibreOffice `dictionaries/es/es_ES.dic`, usado bajo MPL-1.1.

`stopWord.n` es la distribución autocontenida. Su lector temporizado usa la
terminal no bloqueante de nC: muestra el segundero, acepta edición básica y
cierra el turno exactamente al llegar a cero. Incluye copa individual, equipos,
duelo, sprint, supervivencia, maestro de categoría e Infinity. `Esc`, `SALIR` o
`:q` regresan al menú desde un modo.

## Construcción

Desde esta carpeta, después de sincronizar el grafo del proyecto:

```powershell
ncc project sync .
ncc build main.n stop-world-tour.exe --profile release-fast --target x86_64-pc-windows-msvc
.\stop-world-tour.exe

ncc build stopWord.n stopWord.exe --profile release-fast --target x86_64-pc-windows-msvc
.\stopWord.exe
```

```bash
ncc project sync .
ncc build main.n stop-world-tour --profile release-fast --target x86_64-unknown-linux-gnu
./stop-world-tour
ncc build stopWord.n stopWord --profile release-fast --target x86_64-unknown-linux-gnu
./stopWord
ncc build main.n stop-world-tour-macos-arm64 --profile release-fast --target aarch64-apple-darwin
```

El lowering convierte `stc: immutable` en `.rodata` (ELF), `.rdata` (PE) o
`__TEXT,__const` (Mach-O). Cada paquete queda en símbolos independientes, por lo
que un paquete no importado puede eliminarse durante el linking. El componente
explícito `rodata.ncc;` solo se usa al trabajar directamente
con el compilador y no forma parte de este juego.
