# ABI de closures ejecutables

## Representación

Una closure material usa un descriptor de cuatro palabras de target:

| Offset | Campo |
| --- | --- |
| `0 * word` | code pointer a `.__invoke` |
| `1 * word` | environment pointer |
| `2 * word` | aridad visible |
| `3 * word` | número de captures |

El compilador calcula una envolvente de almacenamiento uniforme para todas las
closures del programa. El environment comienza en el primer offset compatible
con la alineación máxima de sus captures. `move/value` almacena el valor;
`ref/mut` almacena un puntero al slot original.

## Lowering del cuerpo

Cada expresión closure produce una `FunctionIR` independiente con
`__closure_env: ptr<byte>` como primer parámetro. Los parámetros del usuario siguen
después y pasan por la ABI normal del target. El cuerpo usa offsets de environment
tipados, conserva effects y baja el retorno declarado.

## Construcción e invocación

La construcción reserva descriptor más environment, instala la relocation al code
pointer y escribe env/arity/capture-count. La llamada:

1. carga el descriptor;
2. carga code pointer y environment pointer;
3. prepara `env` como argumento oculto;
4. prepara argumentos visibles en registros/stack;
5. emite `CallIndirect` y materializa el retorno ABI.

Una closure válida no usa `DevTrap`, `.__invoke_trap` ni `ud2`. `ud2` permanece solo
como opcode explícito de trap/fallback de paths que lo declaran.

## Retorno caller-owned

Una función cuyo retorno sea `fn(...) -> T` recibe como primer parámetro ABI
`__ncc_closure_return_storage: ptr<byte>`. El llamador reserva la envolvente en
su propio frame y pasa su dirección antes de los argumentos visibles.

- un literal devuelto directamente se construye dentro de esa envolvente;
- una llamada encadenada reenvía la misma dirección;
- un valor closure ya materializado se copia a la envolvente y reajusta su
  `environment pointer` al environment copiado;
- la función devuelve la dirección suministrada por el llamador;
- no se introduce `malloc`, runtime de ownership ni limpieza implícita.

La misma regla se materializa en las firmas LLVM y en los planes ABI machine.
El tamaño y la alineación se calculan después del layout real de captures,
incluidos captures indirectos y agregados.

## Vida y escape

Captures owned pueden escapar con el descriptor. Captures borrowed obedecen el
análisis de escape y no pueden sobrevivir al storage capturado. Captures mutables
comparten el slot por puntero, por lo que varias invocaciones observan la mutación.

## Evidencia

`closureExecutable.n` verifica closure simple, argumento closure, captures value y
mut, escape owned, retorno closure y seis argumentos. Se ejecuta como proceso ELF64
x86_64 y ELF32 i386. Una prueba estructural adicional verifica parámetro oculto,
layout de 32/64 bits, llamada machine, ausencia de storage local escapado y retorno
del puntero caller-owned.
