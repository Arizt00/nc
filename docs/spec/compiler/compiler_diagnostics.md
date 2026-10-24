# Compiler Diagnostics

## Propiedad
Códigos estables, mensajes reproducibles y salida JSON válida.

## CLI
`ncc compiler diagnostics --json`

## Formato textual punto 11

Cuando un diagnostico de lexer, parser o sema llega al CLI con posicion
`[line:column]` y la invocacion contiene un archivo fuente real, `ncc` imprime
la cabecera baseica, la linea de codigo correspondiente y un marcador `^`
exactamente bajo la columna diagnosticada. La ruta del archivo se resuelve con
el mismo resolvedor universal de la CLI, por lo que no debe perder contexto con
rutas relativas, absolutas, carpetas con espacios, `/mnt/c/...` o rutas Windows
recibidas desde WSL.

Ejemplo:

```text
[2:1] Expected Semicolon
2 | hello {
    ^
```

Este formato no emite sugerencias. Las sugerencias del CLI quedan reservadas
para errores de comandos o subcomandos desconocidos, no para diagnosticos de
codigo fuente.
