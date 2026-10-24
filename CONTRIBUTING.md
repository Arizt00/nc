# Contribuir a nC

Empieza por un comportamiento que otra persona pueda reproducir. Puede ser un error del compilador, un diagnóstico poco claro, una explicación incorrecta o un ejemplo que no coincide con las instrucciones.

## Preparar una reproducción

Incluye sistema y arquitectura, hash o versión del compilador, proveedor utilizado, comando completo, fuente mínima, salida obtenida y resultado esperado. Distingue el kit nativo de la CLI Stage0. El [estado actual](docs/public/estado.md) describe sus diferencias.

Antes de proponer una sintaxis, explica qué decisión permite expresar y cómo se conservará en parser, AST, análisis semántico, IR y materialización. Las fases se cierran con un comportamiento coherente, no solo porque un texto ya se analice.

## Cambios de código

Trabaja en una rama y mantén el cambio centrado en un problema. Conserva los diagnósticos existentes salvo que el cambio justifique su revisión. Incluye fixtures positivos y negativos donde el contrato lo necesite. Para ABI, memoria o backend, documenta el perfil y el resultado material que has comprobado.

Para comprobar fixtures, invoca el compilador ya construido. Con Stage0 disponible, desde la raíz en Linux o WSL:

```bash
./target/debug/ncc check examples/00_hello/hello.n
./target/debug/ncc emit-ir examples/11_machine/checked_add_overflow_machine.n
```

En Windows usa `.\target\debug\ncc.exe`. Si trabajas con el kit nativo, emplea `compiler sema` y `compiler emit-ir` para las rutas admitidas; no presupongas equivalencia con la CLI de Stage0.

Los cambios en la implementación Rust de Stage0 requieren además sus controles de formato y pruebas. Prepara primero la semilla como indica [instalación](docs/install.md):

```bash
cargo fmt --all -- --check
cargo test --locked --workspace -- --nocapture
```

Algunas suites requieren herramientas o entornos adicionales. Indica qué ejecutaste y qué no pudiste comprobar; conserva el fallo exacto. No cambies una expectativa para ocultar una regresión.

## Documentación y web

Las guías editables están en `docs/public/`. La web se genera desde esas fuentes:

```bash
node tools/publishing/build-site.mjs
node tools/publishing/check-public.mjs
```

El generador web usa solo Node. Los PDF se generan aparte con `build-manuals.mjs` y Playwright. Esas dependencias pertenecen a la edición de documentación y no al runtime del lenguaje. Comprueba los enlaces, la búsqueda, el tamaño móvil y los ejemplos después de editar.

## Enviar una propuesta

Explica el problema concreto, el comportamiento anterior y posterior, las pruebas y los límites pendientes. Una propuesta pequeña con evidencia se revisa mejor que una reescritura sin un caso que la motive.

Discute las decisiones técnicas con respeto. No incluyas credenciales, archivos personales, cachés o informes con rutas privadas en issues y pull requests.
