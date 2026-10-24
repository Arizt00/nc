# Instalar y probar nC

## Kit nativo V81

Descarga el paquete para Windows x86-64 o Linux x86-64 de
[Releases](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81).
Comprueba su digest en `SHA256SUMS.txt` y extráelo en un directorio nuevo.
No hace falta instalar Rust ni LLVM para ejecutar este recorrido.

En Windows, el directorio debe conservar `ncc.exe`, `ncruntime.dll` y `hello.n`:

```powershell
.\ncc.exe self-test
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

En Linux o WSL x86-64 conserva `ncc`, `libncruntime.so.2` y `hello.n`:

```bash
chmod +x ncc
./ncc self-test
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

Salida esperada: `Hello, nC public beta`. Exit code: 0.

Linux se ha probado en Ubuntu 24.04 bajo WSL con glibc 2.39; los imports del kit
requieren símbolos de glibc 2.34 o posterior. Esa condición binaria no certifica
todas las distribuciones. El compilador lleva RUNPATH `$ORIGIN`; el programa
generado del ejemplo necesita la ruta explícita al proveedor.

La CLI del kit es parcial: usa `self-test`, `compiler sema`, `compiler emit-ir`,
`compiler emit-object` y `compiler emit-executable`. `--help`, `check` y `machine`
todavía pueden devolver `E_BOOTSTRAP_STAGE0_REQUIRED_001` (salida 69).

## Construir el Stage0 desde fuentes

Necesitas Git, Rust estable con los componentes de `rust-toolchain.toml`,
un archivador compatible y el kit nativo verificado como semilla. En WSL:

```bash
export NC_STAGE0_POLICY_COMPILER="$HOME/nc-seed/ncc"
export NC_STAGE0_POLICY_ARCHIVER=ar
export CARGO_BUILD_JOBS=1
cargo build --locked -p ncc-cli --bin ncc
```

En Windows usa una terminal de desarrollo MSVC:

```powershell
$env:NC_STAGE0_POLICY_COMPILER = 'C:\nc-seed\ncc.exe'
$env:NC_STAGE0_POLICY_ARCHIVER = 'llvm-lib.exe'
$env:CARGO_BUILD_JOBS = '1'
cargo build --locked -p ncc-cli --bin ncc
```

Sustituye la ruta de ejemplo por la de tu kit. `lib.exe` es otra opción compatible
para el archivador Windows. El puente nativo compila `bootstrap/nc/stage0_policy.n`
durante la construcción; omitir la semilla provoca un error explícito.

Una vez construido, invoca Stage0 directamente. Desde la raíz del repositorio en Linux o WSL:

```bash
./target/debug/ncc check examples/00_hello/hello.n
./target/debug/ncc emit-ir examples/00_hello/hello.n
```

En PowerShell usa `.\target\debug\ncc.exe` con los mismos argumentos. No necesitas volver a ejecutar Cargo para analizar cada programa.

La [guía de uso](public/origen-y-guia.md) desarrolla la sintaxis y las diferencias
entre recorridos. El [estado de publicación](public/estado.md) delimita el soporte.
