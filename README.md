<p align="center"><img src="sites/nc-language/dist/mark.svg" width="82" alt="nC"></p>

# nC

**Un lenguaje con continuidad tipada entre el programa, la memoria y la máquina.**

nC reúne funciones, contratos basales, instrucciones de máquina y primitivas materiales dentro de un mismo lenguaje. Su compilador, **ncc**, está en bootstrap nativo. El proyecto trabaja para conservar el significado de cada operación desde la fuente hasta el ejecutable.

[![Estado: bootstrap V81](https://img.shields.io/badge/estado-bootstrap_V81-244c3b)](docs/public/estado.md)
[![Licencia: MIT o Apache 2.0](https://img.shields.io/badge/licencia-MIT_o_Apache_2.0-e4edb1)](LICENSE)

[**Web y biblioteca**](https://nc-language-arizt00.rarepanda1024.chatgpt.site) · [**Descargar el kit nativo**](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81) · [**Guía de uso**](docs/public/origen-y-guia.md) · [**Arquitectura**](docs/public/arquitectura-y-bootstrap.md) · [**Contribuir**](CONTRIBUTING.md)

## Por qué existe

El layout de un dato, la dirección de una reserva y el protocolo de una llamada influyen en lo que un programa hace. nC busca expresar esas decisiones sin perder sus contratos al cambiar de nivel. La raíz `[@]`, `[@machine]`, `primitive[@]`, el runtime y el enlace forman parte de esa continuidad.

La memoria es manual/raw. El tipado no añade ownership, liberación automática ni comprobaciones implícitas de punteros. El [origen y propósito del lenguaje](docs/public/origen-y-guia.md#por-qué-nace-nc) explica esa elección.

## Prueba un programa

Descarga el paquete de tu plataforma en [Releases](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81), verifica su SHA-256 y extráelo. El kit incluye el compilador, su runtime y `hello.n`. La primera prueba no necesita Rust ni LLVM.

```nc
use <io:output>;

helloCompacto {
    to main {
        println("Hello, nC public beta");
    }

    fa:
        return true;
}
```

En Windows x86-64, desde PowerShell en el directorio extraído:

```powershell
.\ncc.exe self-test
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

En Linux x86-64 o WSL:

```bash
chmod +x ncc
./ncc self-test
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

Salida esperada: `Hello, nC public beta`. Código de salida: `0`.

El paquete Linux se probó en Ubuntu 24.04 bajo WSL y requiere símbolos de glibc 2.34 o posterior. Conserva el proveedor junto al compilador. Los comandos del kit son una parte de la CLI: `--help`, `check` y `machine` todavía requieren Stage0. Consulta [instalación](docs/install.md) antes de usar otros recorridos.

## Qué está demostrado

| Área | Alcance de esta edición |
| --- | --- |
| Primer uso | Kit aislado, self-test, hello y aritmética ejecutados en Windows y Linux/WSL x86-64 |
| Reconstrucción V81 | B2/C2 idénticos por anfitrión; 74 unidades por reconstrucción según el dossier |
| Evidencia V81 | 147 programas y 38 contratos de runtime por generación y sistema documentados |
| Memoria y ABI | Alineaciones nativas de 32 bytes a 1 MiB en el perfil descrito; límites por formato |
| Pendiente | Stage3 global, CLI nativa completa y cierre de ejecución de otras ISAs |

Los resultados V81 pertenecen a su [dossier técnico](docs/NCC_MODULES_V81_DOSSIER.md). La publicación registra sus [pruebas de entrega](docs/public/validacion.md) por separado. No se presenta como una versión estable para producción.

## Documentación

- [Origen y guía de uso](docs/public/origen-y-guia.md) · [PDF](docs/public/origen-y-guia.pdf).
- [Arquitectura y bootstrap](docs/public/arquitectura-y-bootstrap.md) · [PDF](docs/public/arquitectura-y-bootstrap.pdf).
- [Sintaxis](docs/syntax.md), [runtime](docs/runtime.md), [máquina](docs/machine.md), [primitivas](docs/primitive.md) y [FFI](docs/ffi.md).
- [Reproducción V81](docs/NCC_MODULES_V81_REPRO.md), [ABI y layout](docs/NCC_MODULES_V81_ABI_LAYOUT.md) y [límites](docs/public/estado.md).
- [Próximos pasos](ROADMAP.md) y [guía de contribución](CONTRIBUTING.md).

## Trabajar desde las fuentes

La construcción del Stage0 necesita una semilla nativa verificada, su runtime y un archivador. `cargo build` por sí solo no prepara esa dependencia. El kit V81 proporciona una semilla identificada; la [guía de instalación](docs/install.md) explica las variables `NC_STAGE0_POLICY_COMPILER` y `NC_STAGE0_POLICY_ARCHIVER`.

```bash
cargo build --locked -p ncc-cli --bin ncc
```

| Directorio | Contenido |
| --- | --- |
| `tools/ncc/nc/src` | Compilador escrito en nC |
| `crates` | Implementación y herramientas Stage0 retenidas |
| `runtime/nc` | Runtime y proveedores |
| `examples` | Ejemplos por tema |
| `bootstrap` | Contratos y recetas de construcción |
| `docs` | Guías y referencias técnicas |
| `sites/nc-language/dist` | Web estática, sin dependencias de servidor |

## Participar

Un ejemplo mínimo que falla es una contribución útil. Incluye el hash del compilador, tu sistema, el comando y el resultado esperado. Las [plantillas de issues](https://github.com/Arizt00/nc/issues/new/choose) ayudan a preparar una reproducción.

Las propuestas de lenguaje deben explicar cómo se conserva su significado en parser, AST, semántica e IR. Para una primera contribución, empieza por una explicación confusa, un ejemplo comprobable o un diagnóstico concreto.

## Licencia

El código se distribuye bajo [MIT](LICENSE-MIT) o [Apache 2.0](LICENSE-APACHE), a elección del usuario. Los avisos de terceros conservan sus propias condiciones.
