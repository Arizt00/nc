# nC Syntax

La superficie publica `.n` es compacta, seccionada y manual/raw. El
compilador analiza, tipa, optimiza sin cambiar semantica y materializa bytes;
no introduce seguridad, ownership, limpieza o planificacion ocultos.

## Imports

```nC
use <io:input, output, render>;
use <memory:region, heap>;
```

`<...>` selecciona la biblioteca, `io` el modulo, `:` sus elementos y `,` los
separa. Los imports expandidos solo son representacion compatible.

## Contenedor y secciones

```nC
counter {
    const:
        LIMIT >> int <- 100;

    type:
        Sample {
            public:
                value >> int;
            layout:
                repr(C);
        }

    stc:
        calls >> u64 <- 0;

    local:
        scratch >> int <- 0;

    fn:
        private:
            clamp(value >> int) -> int {
                if (value > LIMIT) { return LIMIT; }
                return value;
            }

        public:
            run() -> bool {
                calls += 1;
                return clamp(7) == 7;
            }

    ref main ->> run;

    fa:
        return true;
}
```

El contenedor normal solo lleva nombre. `:: kernel`, `:: machine`,
`:: primitive`, `:: runtime` y otros roles aparecen cuando cambian el contrato
material. Solo se permite un `stc:`. `run` no es especial: `ref main` lo
convierte en entrada.

La visibilidad forma parte del simbolo material:

- `public`: API exportada; LLVM usa visibilidad normal y el objeto un simbolo global.
- `internal`: visible entre unidades del paquete; LLVM usa `hidden` y el objeto conserva un simbolo global enlazable.
- `private`: visible solo en su contenedor; LLVM usa `private` y el objeto un simbolo local.

Las mismas reglas se aplican a funciones, datos y campos cuando su contrato
admite visibilidad. Ninguna de ellas agrega dispatch o wrappers ocultos.

Los parametros se declaran con `nombre >> tipo`. En `constructure:` la forma
oficial es `static(...) -> Tipo;`; `nfmt` migra `static :: in(...)` y
`static :: fn(...)` sin cambiar el contrato tipado.

Para una entrada basica de hasta 30 sentencias logicas se puede usar:

```nC
hello {
    to main { println("Hola"); }
    fa:
        return true;
}
```

## Tipos y memoria

`>> T <- value` declara un lugar tipado y escribe sus bits iniciales. La misma
flecha escribe lugares existentes. `=` se acepta solo como compatibilidad de
fuentes historicas y `nfmt` lo convierte a `<-`; `:=` ha sido retirado. Una
declaracion sin inicializador no promete bytes inicializados.

```nC
raw >> byte* <- cast(alloc(64));
numbers >> int* <- cast(alloc(8 * sizeOf(int)));
wr(numbers, 7);
wr(off(numbers, 1), 11);
value >> int <- rd(numbers);
free(numbers);
```

`T*`, `p<T>` y `ptr<T>` llegan al mismo AST tipado. Se permiten null,
desalineacion, aliasing, overflow, fuera de rango, UAF y double free. Compilar
no garantiza un resultado estable para comportamiento indefinido.

## Strings

```nC
message >> str <- "resultado: " + (ok ? "true" : "false");
println(message);
```

`str + str` baja a la operacion nativa de concatenacion. El puntero resultante
queda bajo control manual del programador. `cat(dst, cap, left, right)` permite
aportar el buffer cuando se necesita controlar direccion y capacidad.

## ABI y layout

Los contratos `repr(C)`, `packed`, `align`, unions, bitfields, arrays flexibles,
TLS, per-CPU, varargs, retornos agregados y punteros de funcion se conservan en
AST, IR, machine, objeto y enlace. Las declaraciones `abi` y `bind` siguen
disponibles para seleccionar registros, stack, preservados y retorno.

## Control de flujo

```nC
if (ready) { consume(); }
else if (retry) { recover(); }
else { stop(); }

while (active) { step(); }
guard valid else return false;
```

`&&`, `||`, `!`, comparaciones, aritmetica, escrituras compuestas y el operador
condicional conservan orden de evaluacion de fuente.

## Formato

`nfmt --compact` preserva comentarios y las secciones `const/type/stc/local/fn/fa`.
No elimina visibilidad ni convierte funciones agrupadas en declaraciones
directas. `state {}` migra a `stc:` y el final generado `shutdown` migra a
`fa:`.
