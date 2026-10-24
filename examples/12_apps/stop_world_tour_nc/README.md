# Stop Word nC: distribución de un solo archivo

[`stopWord.n`](./stopWord.n) es el juego completo y su única fuente de entrada.
No es un `project`, no importa `lexicon_es_core.n` y no requiere archivos de
datos junto al programa ni junto al ejecutable.

La declaración `rodata.ncc;` solicita al compilador el componente léxico nC
integrado. El componente solo se materializa si el fuente usa `rodataNcc`; su
corpus de 8.760 raíces españolas, el índice hash y el lector front-coded quedan
incorporados al binario durante el lowering. El archivo del juego permanece
pequeño y el ejecutable continúa funcionando sin red.

La categoría es una regla del juego; el corpus comprueba que la palabra existe.
Los dominios finitos (países, colores y elementos químicos) deben distribuirse
como paquetes exhaustivos, nunca rellenarse con nombres inventados para alcanzar
mil entradas.

Origen del corpus: LibreOffice `dictionaries/es/es_ES.dic`, usado bajo MPL-1.1.

Su lector temporizado usa la
terminal no bloqueante de nC: muestra el segundero, acepta edición básica y
cierra el turno exactamente al llegar a cero. Incluye copa individual, equipos,
duelo, sprint, supervivencia, maestro de categoría e Infinity. `Esc`, `SALIR` o
`:q` regresan al menú desde un modo.

## Construcción

Desde esta carpeta, sin inicializar ni sincronizar ningún proyecto:

```powershell
ncc build stopWord.n stopWord.exe --profile release-fast --target x86_64-pc-windows-msvc
.\stopWord.exe
```

```bash
ncc build stopWord.n stopWord --profile release-fast --target x86_64-unknown-linux-gnu
./stopWord
ncc build stopWord.n stopWord-macos-arm64 --profile release-fast --target aarch64-apple-darwin
```

El lowering convierte los datos `immutable` aportados por `rodata.ncc` en
`.rodata` (ELF), `.rdata` (PE) o `__TEXT,__const` (Mach-O). El corpus no aparece
como miles de líneas dentro de `stopWord.n`: pertenece al componente del
compilador y solo alcanza el ejecutable que lo solicita.
