# Validación de la primera publicación

Comprobaciones realizadas el 24 de octubre de 2026. Los resultados de esta página
pertenecen a la preparación pública; la reconstrucción completa V81 se describe
en su dossier y no se ha repetido aquí.

## Kit portable

Se copiaron únicamente el compilador B2 y su proveedor a directorios nuevos.
Los cuatro hashes coinciden con el manifiesto retenido V81. Se ejecutaron
`self-test`, compilación de `hello.n`, su proceso y un programa de aritmética
que comprueba `6 * 7 == 42`. Todos terminaron con salida 0 en Windows x86-64
y Ubuntu 24.04.4 bajo WSL x86-64. La salida de hello fue exactamente
`Hello, nC public beta`.

Tras generar ZIP y TAR.GZ se extrajeron en otros directorios nuevos, se
comprobaron los archivos y se repitió la compilación y ejecución de hello.
El kit no requirió Rust, Cargo ni LLVM para estos recorridos.

| Archivo | SHA-256 |
| --- | --- |
| Windows ncc.exe | `4ff2300d30fbe99a4c541481e0fab9df284d77c32d1df082084440b9718854c9` |
| Windows ncruntime.dll | `83d67b0723e18eb8c9898eae286abf8ffe3bc63bd610d7aea7924de2d70f5dbc` |
| Linux ncc | `e3d6fc744f0768d19a3f44ca93eddcb90cc3f89caee536f68f8f5ce486eae1e7` |
| Linux libncruntime.so.2 | `92ac32740801e0ebdb572cd953bc258fa73843bf950a8cf3f80e7c0ea4681b5e` |

Linux se comprobó con glibc 2.39. Los imports requieren hasta GLIBC_2.34.
El ejecutable generado necesita la ruta de carga al proveedor indicada en
la receta. Windows importa bibliotecas del sistema y el proveedor incluido.

## Fuentes y ejemplos

- `cargo check --locked -p ncc-cli --bin ncc`: correcto con semilla V81 y archivador MSVC configurados; se conservan advertencias de código no utilizado.
- `cargo test --locked -p ncc-parser --lib -- --nocapture`: 206 pruebas aprobadas, 0 fallos.
- `cargo test --locked --workspace -- --nocapture`: se detuvo en `ncc-ast` con 117 pruebas aprobadas y 1 fallo. La prueba `stoud_text_string_is_manual_nc_with_explicit_release` espera 26 símbolos de runtime y encuentra 27 (`crates/ncc-ast/src/import_catalog.rs:7669`). Las suites posteriores no llegaron a ejecutarse. Esta publicación no modifica esa prueba ni la implementación de texto.
- Stage0 retenido: `check` y `emit-ir` de hello y del ejemplo machine de overflow correctos; `machine emit-ir` de `object_elf64.nm` correcto.
- `cargo fmt --all -- --check`: detecta diferencias previas en fuentes del compilador. No se aplicó un reformateo masivo sobre trabajo ajeno a esta publicación.

La suite completa del workspace no está aprobada.
La entrega mantiene su carácter preliminar y no usa una insignia que afirme un
cierre global del compilador.

## Web y documentos

La comprobación estática recorre 20 páginas y 525 referencias locales, además
de los 16 capítulos del índice de búsqueda. Los enlaces locales resuelven.

La revisión en Edge cubre escritorio y anchos de 390, 768 y 1440 píxeles:
copia de código, búsqueda por teclado y contenido, filtros con y sin resultados,
tema persistente, menú móvil, selección de sistema operativo, copia de recetas,
navegación por teclado entre fases, preguntas desplegables y último capítulo
leído. No se detectó desbordamiento horizontal en las rutas comprobadas.

Los PDF tienen 9 páginas cada uno. Se revisaron las 18 páginas renderizadas, con
comprobación adicional del texto y sus límites. Las fuentes editables se
conservan en Markdown.

La exportación pública usa una selección explícita de árboles y archivos y
registra sus hashes en `PUBLICATION.json`. Se excluyen el historial privado,
cachés, informes locales y binarios del árbol fuente; los kits se distribuyen
como activos de Release.
