# nC v1.0.0 — Guía de tooling

## Herramientas oficiales

- `ncc`: compilador/CLI principal.
- `nfmt`: formateador oficial, incluido modo compacto para migrar codigo verboso.
- `nlint`: lint básico oficial.
- `nbuild`: pipeline de build oficial.
- `npkg`: packaging/publicación básica.
- `nide`: salida estructurada para ecosistema IDE (`outline/symbols/diagnostics`) y generador de Run/Check/Format.

## Uso mínimo

```bash
cargo run -p ncc-cli --bin nfmt -- <file.n>
cargo run -p ncc-cli --bin nfmt -- <file.n> --compact
cargo run -p ncc-cli --bin nlint -- <file.n>
cargo run -p ncc-cli --bin nbuild -- <file.n> <out-exe>
cargo run -p ncc-cli --bin npkg -- <file.n>
cargo run -p ncc-cli --bin nide -- outline <file.n>
cargo run -p ncc-cli --bin nide -- symbols <file.n>
cargo run -p ncc-cli --bin nide -- diagnostics <file.n>
cargo run -p ncc-cli --bin nide -- init --ide vscode
cargo run -p ncc-cli --bin nide -- init --ide jetbrains --file <file.n>
```

## Contrato IDE (`nide`)

- `outline`: estructura de contenedores/contextos/secciones en JSON.
- `symbols`: inventario de símbolos por contenedor en JSON.
- `diagnostics`: resultado estructurado por fase (`lexer/parser/sema`) con `error_entries` (`message`, `line`, `column`) y warnings/errors canónicos en JSON.
- `init`: genera `.vscode/tasks.json` y/o `.idea/runConfigurations/*.xml` para lanzar `ncc run <archivo.n>` desde el IDE sin escribir comandos en terminal.
- objetivo: integración limpia con herramientas editor/LSP sin degradar el pipeline tipado.

Contrato detallado:

- `docs/spec/ide/ide_run_configurations.md`

## Contrato `nfmt --compact`

`nfmt <file.n> --compact` convierte codigo `.n` verboso a la superficie compacta
oficial cuando la transformacion es conservadora:

- eleva `public:` desde el cuerpo de funciones al bloque `fn:` cuando el patron
  es inequívoco;
- convierte proyecciones simples `@x >> T : expr;` en asignaciones compactas
  `x = expr;` cuando `x` ya esta declarado;
- conserva `ref main ->> ...;`, imports y secciones baseicas;
- valida de nuevo lexer, parser y sema antes de emitir el resultado.

## Contrato de estabilidad

El comportamiento base de estas herramientas está fijado en:

- `docs/spec/tooling/tooling_operational_contracts_v1_0.md`
