# Compiler CLI

## Superficie
- `ncc compiler doctor`
- `ncc compiler pipeline|phases|tokens|ast|sema|imports|types|type-contracts|bounds|operators|conditions|nir|lower|backend <file>`
- `ncc compiler targets`
- `ncc compiler diagnostics --json`
- `ncc compiler report <file> --md-out <path>`
- `ncc compiler self-check`
- `ncc compiler ffi-smoke --json [--strict]`
- `ncc compiler ffi-link-manifest <file> --json`
- `ncc compiler machine-interop <file.n> --target <triple> --json [--strict]`
- `ncc compiler build-native <file.n> --profile release-fast --out-dir .ncc --json`

## Native build estilo C

La entrada normal es `ncc build <file.n|file.nm>`. Esa ruta retiene los
artefactos que se esperan en una cadena de compilacion normal:

- `.ncc/lowering/<unidad>.nm`: puente Machine legible y validable.
- `.ncc/lowering/<unidad>.backend-object.nbo`: objeto backend estructurado con
  secciones, simbolos, relocaciones y procedencia Basal/Machine.
- `.ncc/code/<unidad>.launcher.n`: lanzador nC del ejecutable retenido.
- `.ncc/obj/<unidad>.obj`: objeto nativo emitido por el backend.
- `.ncc/bin/<unidad>.exe`: ejecutable enlazado.
- `.ncc/bin/<unidad>.ps1` y `.ncc/bin/<unidad>.bat`: wrappers de shell
  reconocidos por `ncc run` y `ncc check`.
- `.ncc/run/<unidad>.native-build.json`: manifest de artefactos, hashes,
  target, entry y etapas.

Para `--cc nC.win64`, la ruta normal genera PE32+ directamente cuando no se
pasa `--linker`: conserva el `.nm`, emite el `.obj` high-level, emite un
`.obj` puente de entrada desde Machine y enlaza el `.exe` con Phase 12 interno.
El PE final lleva `.reloc`, entradas `IMAGE_REL_BASED_DIR64` para absolutos y
flags ASLR `DYNAMIC_BASE`/`HIGH_ENTROPY_VA`.
Si se pasa `--linker <bin>`, se respeta el enlazador externo indicado.

Para un `.nm` escrito a mano, `ncc build archivo.nm --out-dir .ncc --json`
valida Machine, infiere el target desde `target:`, emite `.obj`, enlaza `.exe`
y escribe manifest sin pasar por el parser high-level.

`ncc run archivo.n --real` y `ncc run archivo.n --native-fast` ejecutan por la
misma superficie material cuando el perfil baja a bytes reales: además de
lanzar el proceso nativo, retienen `.ncc/lowering/<unidad>.nm`,
`.ncc/lowering/<unidad>.backend-object.nbo`,
`.ncc/code/<unidad>.launcher.n`,
`.ncc/obj/<unidad>.obj`, `.ncc/obj/<unidad>.entry.obj` cuando hace falta puente
hosted, `.ncc/bin/<unidad>.exe` y
`.ncc/bin/<unidad>.ps1`, `.ncc/bin/<unidad>.bat` y
`.ncc/run/<unidad>.native-run.json`.

El puente hosted se genera como Machine `.nm` y como objeto nativo; la ruta
normal no crea `launcher.c`. `launcher.n` es el lanzador nC visible para el
usuario; el puente que baja a bytes sigue siendo el `.nm` validado por Machine.
Si una familia high-level con runtime pesado aun no puede bajar por el objeto
directo, `run --real` cae automaticamente al carril LLVM/MCT, retiene
`.ll/.obj/.exe`, escribe `route:
mct-llvm-build-native-fallback` y ejecuta el `.exe` real del sistema.

Despues de enlazar, `ncc` libera el `.exe` como artefacto de usuario: no deja
handles retenidos por el proceso de compilacion y, en Windows, retira el flag
read-only si aparece. El ejecutable se puede lanzar fuera del IDE desde
`.ncc/bin`.

`ncc compiler build-native <file.n>` sigue disponible como carril explicito
LLVM/MCT; esa ruta adicional retiene `.ncc/lowering/<unidad>.ll` y materializa
objeto/ejecutable con `clang`.

La ruta normal por defecto es `release-fast` y no cae al interprete: si no puede
materializar objeto/ejecutable real con el linker interno o con la toolchain
configurada, informa el fallo. En el carril LLVM/MCT, `release-fast` selecciona
`O3` salvo que se indique `--opt-level` o `-O3`.

El `.nm` retenido es un puente Machine de entrada hacia el `main` generado por
la ruta nativa. No declara que ya exista una traduccion instruccion por
instruccion de todo `.n` arbitrario a Machine.

## Bounds
- `ncc compiler bounds <file> --json`
- Returns bounds/index-oriented diagnostics filtered from compiler type diagnostics.
- Supports `--strict` gate for release profiles.

## FFI smoke
Valida enlace real C/C++/Rust (sin ASM en esta fase) con símbolos y ejecución de harness cuando toolchain externa está disponible.

El payload incluye `host` y `abi_matrix` para trazar compatibilidad de enlace por perfil:
- `nC.sysv64`
- `nC.win64`
- `nC.aapcs64`

Además, para ruta directa de usuario final:
- `ncc run <file.n> --native-ffi [--cc <profile>] [--linker <bin>] [--entry <symbol>]`

`run --native-ffi` realiza autolink end-to-end (sin wrapper manual) cuando el programa declara `foreign` symbols.

`ffi-link-manifest` emite el inventario de enlace/packaging para `foreign:`
symbols: headers, libraries, artifact paths, checksums, `-L`, `-l` y runtime
paths. No enlaza ni ejecuta; prepara la verdad consumible por tooling y release.

`machine-interop` emite `nc.compiler.machine_interop.v1`: nombres logicos y
simbolos nativos de funciones `.n`, ABI de target, layout de structs, punteros,
strings, imports `foreign:` y simbolos runtime seleccionados desde
`nc.runtime.symbol_abi.v1`.

See also: `docs/spec/compiler/compiler_ffi_real_link_run.md`.
