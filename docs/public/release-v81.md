# nC V81 Primer kit nativo público

Esta entrega permite probar nC con un compilador nativo experimental en Windows
y Linux x86-64. Incluye el proveedor de runtime, un primer programa, un ejemplo
de aritmética, licencias y hashes de integridad. La primera prueba no necesita
Rust ni LLVM.

## Descargar y ejecutar

Elige `nc-bootstrap-v81-windows-x86_64.zip` o
`nc-bootstrap-v81-linux-x86_64.tar.gz`. Compara el hash del archivo descargado
con `SHA256SUMS.txt` antes de extraerlo.

Windows PowerShell:

```powershell
.\ncc.exe self-test
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

Linux o WSL:

```bash
chmod +x ncc
./ncc self-test
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

Resultado esperado: `Hello, nC public beta`, exit 0. Linux se probó en Ubuntu
24.04 bajo WSL; sus imports requieren glibc 2.34 o posterior. Conserva el runtime
incluido junto al compilador.

## Documentación

- [Web y biblioteca](https://nc-language-arizt00.rarepanda1024.chatgpt.site).
- Guía de origen y uso en PDF: 9 páginas.
- Arquitectura y bootstrap en PDF: 9 páginas.
- [Instalación desde fuentes](https://github.com/Arizt00/nc/blob/main/docs/install.md).
- [Validación de esta publicación](https://github.com/Arizt00/nc/blob/main/docs/public/validacion.md).

## Alcance

El candidato V81 ofrece una CLI parcial de bootstrap. `self-test`,
`compiler sema`, `compiler emit-ir`, `compiler emit-object` y
`compiler emit-executable` cubren los recorridos probados. `--help`, `check`
y `machine` pueden devolver `E_BOOTSTRAP_STAGE0_REQUIRED_001` (salida 69).

La evidencia V81 documenta generaciones B/C idénticas por anfitrión. Stage0
permanece y Stage3 global sigue abierto. Esta entrega no sustituye
automáticamente una instalación existente ni certifica otras ISAs. La memoria
mantiene semántica manual/raw.

Se comprobaron los kits en directorios aislados y después de extraer sus archivos.
La comprobación de compilación con semilla configurada y las 206 pruebas del
parser pasaron. La suite global se detuvo en `ncc-ast`: 117 pruebas aprobadas y
un fallo en `stoud_text_string_is_manual_nc_with_explicit_release`, que espera
26 símbolos y encuentra 27. También quedan diferencias de formato previas.
El [registro de validación](https://github.com/Arizt00/nc/blob/main/docs/public/validacion.md)
conserva el alcance y el resultado exacto.

Si encuentras un problema, abre un issue con el compilador utilizado, sistema,
comando, fuente mínima y resultado esperado. Esa reproducción permite convertir
una primera prueba en una mejora concreta.
