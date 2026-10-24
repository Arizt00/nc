# Compact `.n` Surface v0.1

> Historical compatibility record. The current authoring authority is
> `docs/spec/language/canonical_compact_syntax_v3.md`; snippets below document
> input accepted by older releases and must not be copied as current style.

Esta superficie compacta reduce verbosidad en codigo normal `.n` sin cambiar
la forma baseica del lenguaje. La regla principal es que el parser acepta el
azucar, sema valida contra simbolos ya tipados y el IR baja siempre con tipos
efectivos, no con marcadores internos.

## Alcance

El cambio aplica a programas root/runtime de alto nivel. No redefine `[@machine]`,
`primitive[@]`, linking material, ni la familia baseica de imports.

Se mantiene:

- `ref main ->> run;` como entrada baseica.
- La identidad de proyeccion `>>`.
- `stc:`, `local:`, `type:` y `fn:` como secciones del lenguaje.
- La forma baseica antigua con `@x >> int : expr;`.

## Formas compactas aceptadas

Declaracion con inicializador directo:

```n
stc:
    first >> int = 5;
    ok >> bool = false;
```

La forma anterior sigue siendo valida:

```n
stc:
    first >> int : 5;
```

Bloque de visibilidad para funciones:

```n
fn:
    public:
        run() -> bool {
            return true;
        }
```

Asignacion compacta sobre simbolos ya declarados:

```n
ok = first == 10;
```

Esta forma infiere el tipo desde `stc`, `local` o parametros visibles. En IR se
baja como una proyeccion tipada real, por ejemplo `bool`, `int` o `ft64`.

Intercambio compacto:

```n
swap(first, second);
```

`swap` requiere dos simbolos simples ya declarados y compatibles en tipo.

Azucar de punteros C-like:

```n
*p = 25;
valor = *p;
```

El parser lo normaliza a llamadas de lectura/escritura del runtime de punteros.

## No objetivos

- No se elimina `@`; sigue disponible para proyeccion baseica y referencias
  donde la semantica lo necesite.
- No se cambia la nomenclatura `use::familia::<items>;`.
- No se activa inferencia global de tipos; solo se infiere el tipo de una
  asignacion compacta cuando el simbolo ya fue declarado.
- No se permite inferir proyecciones `mem[...]`; esas rutas deben usar la forma
  baseica explicita.

## Ejemplo

```n
use stoud.io { output, render };

swapNumerosCompacto[@runtime] {
    stc:
        first  >> int  <- 5;
        second >> int  <- 10;
        ok     >> bool <- false;

    run :: in() -> bool {
        swap(first, second);
        ok = first == 10 && second == 5;
        println("Swap correcto = " + renderBool(ok));
        return ok;
    }

    ref main ->> run;

    fa:
        return true;
}
```
