# High Level Fase 4: Interfaces, Traits, Contracts

> Historical phase boundary. This file records what Phase 4 closed at the time.
> Current generic, trait and dispatch semantics are defined by
> `zero_cost_generics_traits.md` and the generated exact-head specification.

Estado: historical; superseded for current capability status.

Fase 4 cierra la superficie declarativa y verificable para interfaces, traits, impl blocks, generic bounds, contracts y dispatch estatico de high-level. La fase se apoya en Fase 3: los genericos ya existen, y Fase 4 agrega restricciones nominales verificadas en compilacion.

En su cierre original no incluia dispatch dinamico, trait objects, specialization, associated types, GAT, HKT real, reflection, effects system ni async/concurrency avanzado. Esta frase no limita el estado actual del compilador.

## Forma baseica seccionada

La forma baseica de Fase 4 queda normalizada por secciones:

- `interface:` agrupa interfaces.
- `trait:` agrupa traits.
- `impl:` agrupa implementaciones.
- `contract:` agrupa contratos.

Las formas flotantes heredadas siguen parseandose por compatibilidad de ejemplos antiguos, pero los ejemplos positivos nuevos usan secciones explicitas. La normalizacion no cambia la semantica de Fase 4: solo fija la superficie de lectura y evita mezclar declaraciones top-level heterogeneas.

Ejemplo baseico:

```n
phase4Example {
    interface:
        Show<T> {
            fn show(value: T) -> str;
        }

    impl:
        Show<int> for int {
            fn show(value: int) -> str {
                public:
                    return "int";
            }
        }

    render<T: Show<T>>(value: T) -> str {
        return show(value);
    }
}
```

El bound `T: Show<T>` se valida contra impls reales. Si se llama `render("x")` sin `impl Show<str> for str`, el compilador emite `E_HIGH_BOUND_UNSATISFIED_001`.

## Cierre historico de Fase 4

- Interfaces: closed.
- Traits: closed.
- Impl blocks: closed.
- Contracts: closed para `require` estatico.
- Generic bounds: closed.
- Static dispatch: closed.
- Dynamic dispatch: fuera del alcance original de Fase 4. La semantica actual nunca lo introduce de forma implicita y lo prohibe como trabajo oculto en kernel.
- HKT: fuera del alcance original de Fase 4.
