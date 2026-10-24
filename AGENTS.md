# AGENTS.md — Guía operativa para agentes sobre el proyecto nC

## 1. Qué es este repositorio

Este repositorio contiene el compilador experimental de nC y su material de contexto para handoff técnico.

nC es un lenguaje soberano, multi-nivel y semánticamente continuo.
No debe tratarse como un lenguaje convencional con “features” aisladas.
Toda modificación importante debe respetar la continuidad entre:

- `[@]`
- `[@machine]`
- `primitive[@]`
- linking
- memoria/punteros/regiones
- runtime
- capas futuras del sistema modal

---

## 2. Regla central

No cierres una fase por sintaxis.
Una fase solo se considera bien trabajada cuando parser, AST, sema, IR, pruebas y documentación están alineados.

---

## 3. Prioridad actual del proyecto

La prioridad activa inmediata es:

### consolidar `[@machine]` como capa tipada, estable, coherente y preparada para continuidad material.

Eso implica no introducir soluciones “rápidas” que rompan:

- tipado de instrucciones;
- tipado de finales;
- control-flow;
- terminadores;
- relación entre parser, sema e IR;
- diagnósticos ya consolidados.

---

## 4. Estado actual resumido

### Ya muy avanzado
- infraestructura general del compilador;
- root `[@]` base;
- primitive base;
- `[@machine]` en superficie/sema/IR;
- linking en caminos importantes;
- documentación del lenguaje.

### Aún no cerrado de verdad
- lowering/backend real;
- runtime profundo completo;
- memoria/punteros/regiones/aliasing plenamente materializados;
- varias capas modales altas.

---

## 5. Reglas de modificación

### Sí hacer
- cambios coherentes y tipados;
- refactors que reduzcan deuda entre parser/sema/IR;
- fixtures positivos y negativos;
- diagnósticos claros;
- documentación alineada con código;
- comandos reproducibles desde WSL.

### No hacer
- meter sintaxis porque “queda bien”;
- duplicar semántica entre `[@machine]` y `primitive[@]`;
- introducir atajos stringly-typed donde ya existe camino tipado;
- cambiar significado canónico sin dejarlo documentado;
- romper tests existentes para “avanzar más rápido”.

---

## 6. Qué significa “done”

Una tarea no está realmente terminada si solo compila.
Debe cumplir, según el caso:

- parser correcto;
- AST correcto;
- sema correcto;
- IR correcto;
- tests y fixtures;
- CLI funcional si aplica;
- documentación mínima de la decisión si cambia superficie o semántica.

---

## 7. Comandos base de validación

Ejecutar normalmente desde la raíz del repo:

```bash
cargo fmt
cargo test --workspace -- --nocapture
cargo run -- check examples/machine_at/machine_at_nop_wait_halt_ok.n
cargo run -- emit-ir examples/machine_at/machine_at_nop_wait_halt_ok.n
