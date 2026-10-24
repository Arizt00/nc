# Backend Point 5: JIT/loader directo para bytes autocontenidos

Status: closed for the backend/machine executable-memory route.

## Alcance cerrado

El backend ya separa dos rutas distintas:

- materializar bytes: `.n/.nm -> machine bytes`, sin ejecucion;
- ejecutar bytes: `.nm/.bin -> isolated executable-memory loader`.

La ruta de ejecucion se apoya en `ncc machine run-bytes` y conserva un contrato
explicito:

- entrada `.nm` verificada o `.bin` crudo;
- rechazo obligatorio de externs;
- rechazo obligatorio de relocalizaciones pendientes;
- limite de bytes por `--max-bytes`;
- timeout por `--timeout-ms`;
- ejecucion en proceso hijo;
- retorno ABI `extern-C fn() -> i64`;
- hash SHA-256 del stream ejecutado;
- limpieza del temporal de entrada del hijo;
- errores tipados para bytes vacios, ABI no soportada, host no soportado, timeout,
  crash del hijo, relocalizaciones e imports pendientes.

## Relacion con backend normal

Este punto no sustituye la ruta `.n -> IR optimizado -> machine -> .obj -> .exe`.
La complementa:

- si el codigo necesita imports, relocations, runtime calls o loader OS, debe ir por
  objeto/enlazador;
- si el codigo es autocontenido y relocation-free, puede ejecutarse directamente
  por JIT/loader;
- el manifiesto JSON retiene `input_contract`, `execution_contract` y
  `loader_contract` para CI y herramientas.

## Evidencia

Pruebas cerradas en `crates/ncc-cli/tests/machine_closure.rs`:

- `.nm` autocontenido retorna `42`;
- `.bin` crudo retorna `42`;
- bytes vacios se rechazan;
- `.nm` con extern se rechaza;
- `.nm` con relocation se rechaza;
- bucle infinito se mata por timeout;
- instruccion invalida queda contenida en el proceso hijo.

## Frontera

No se declara ejecucion JIT de unidades con relocations/imports. Eso corresponde al
linker final y al loader del sistema. El cierre aqui es el loader directo de bytes
autocontenidos, con aislamiento y contrato materializado.
