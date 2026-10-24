# nC — Especificación de salida y consola para imprimir cualquier cosa como un lenguaje maduro

## Propósito

Este documento fija **todo lo que nC necesita** para que su sistema de impresión y salida por consola se comporte como el de un lenguaje serio, estable, pulido y veterano. No se trata solo de que `print(@x)` “saque algo”. Se trata de que nC pueda **imprimir cualquier valor, cualquier estructura y cualquier estado** de forma correcta, consistente, portable, legible, auditable y extensible.

Un lenguaje con 30 años de mercado no improvisa su salida. La salida forma parte de su identidad operativa. Cuando imprime, imprime bien.

---

## 1. Tesis central

nC ya ha demostrado que:

- evalúa expresiones;
- ejecuta `to main`, `ref main` y `skk main` en buena parte de la superficie;
- ya tiene familia `print` funcionando;
- ya puede sacar `inspect`, `mem`, `state` y ejemplos reales;
- parser, sema e IR están muy fuertes;
- parte de la ejecución visible ya existe.

Pero eso todavía **no equivale** a tener un subsistema de consola completamente cerrado.

La diferencia entre “ya imprime algo” y “imprime perfecto como lenguaje acabado” está en estas capas:

1. arquitectura;
2. semántica;
3. formato;
4. render de tipos;
5. canales de salida;
6. testing;
7. portabilidad;
8. estabilidad futura.

---

## 2. Estado real actual de nC

A nivel práctico, el proyecto ya ha demostrado:

- `cargo check --workspace` sano;
- parser fuerte;
- sema fuerte;
- IR fuerte;
- runtime base presente;
- `check` de ejemplos modales correcto;
- `run` de ejemplos reales correcto;
- familia `print` y derivados visible;
- salida real para ejemplos como:
  - `sumaResta.n`
  - `sumaSimple.n`
  - `calculadoraBasica.n`
  - `demoPrintFamily.n`
  - `bibliotecaCentral.n`
  - fixtures de `security_at`, `system_at`, `compiler_at`.

### Hallazgo estructural clave

La lógica de ejecución visible y de consola está todavía demasiado concentrada en:

```text
crates/ncc-cli/src/lib.rs
```

y no en un runtime de consola limpio dentro de:

```text
crates/ncc-runtime
```

Esto significa que nC **ya habla**, pero todavía habla con la garganta del CLI, no con una voz soberana y formalmente encapsulada del runtime.

---

## 3. Objetivo de cierre

El objetivo final no es solo soportar:

```nc
print(@x);
println(@y);
```

El objetivo real es que nC pueda imprimir correctamente:

- escalares;
- cadenas;
- booleanos;
- direcciones;
- punteros;
- null;
- vectores;
- records;
- regiones de memoria;
- estructuras modales;
- estados del runtime;
- errores;
- avisos;
- trazas;
- diagnósticos;
- dumps parciales;
- salidas formateadas;
- resultados de ejecución complejos;
- introspección avanzada;
- logs de subsistemas.

Y todo ello con una política coherente.

---

## 4. Requisitos arquitectónicos obligatorios

## 4.1 Separación real entre CLI y runtime

La consola no debe vivir de forma principal en `ncc-cli`.

### Regla

- `ncc-parser` parsea.
- `ncc-sema` valida.
- `ncc-ir` representa.
- `ncc-runtime` ejecuta y resuelve builtins.
- `ncc-cli` solo orquesta, invoca y presenta.

### Lo que debe moverse al runtime

Deben salir de `ncc-cli` y formalizarse en `ncc-runtime`:

- render canónico de valores;
- dispatcher de builtins de consola;
- política de `stdout` / `stderr`;
- helpers de salto, espacio, prefijos, color futuro;
- serialización textual oficial de tipos runtime;
- salida estructurada para `inspect`, `mem`, `state`.

### Resultado deseado

Crear algo como:

```text
crates/ncc-runtime/src/console.rs
crates/ncc-runtime/src/render.rs
crates/ncc-runtime/src/builtins/console.rs
```

El nombre exacto puede variar, pero la responsabilidad no.

---

## 4.2 Dispatcher builtin oficial

nC necesita un dispatcher central de builtins de consola.

### Debe existir una capa formal equivalente a:

```text
execute_console_builtin(name, args, exec_state)
```

o una variante semánticamente equivalente.

### Debe resolver

- `print`
- `println`
- `print.err`
- `print.wm`
- `print.trc`
- `print.dbg`
- `print.sta`
- `print.inspect` si se mantiene como builtin de superficie
- futuros `print.hex`, `print.bin`, `print.fmt`, etc. si se desean

### Regla

Nada de tener media familia resuelta por caminos especiales dispersos y media por ramas perdidas del CLI. El sistema tiene que estar unificado.

---

## 4.3 Contrato semántico inmutable de la familia `print`

La familia de impresión debe quedar congelada doctrinalmente.

### Propuesta canónica mínima

#### `print(x)`
Imprime sin salto de línea.

#### `println(x)`
Imprime con salto de línea.

#### `print.err(x)`
Emite salida de error controlado. Debe ir a `stderr` o a un canal equivalente claramente etiquetado.

#### `print.wm(x)`
Warning operativo. Debe diferenciarse semánticamente de error fatal.

#### `print.trc(x)`
Traza de ejecución. Útil para diagnóstico, profiling y seguimiento fino.

#### `print.dbg(x)`
Depuración. Debe estar orientado a desarrollador y poder endurecerse o silenciarse por perfil.

#### `print.sta(x)`
Estado resumido o nominal. Ideal para estados booleanos, lifecycle y checkpoints.

### Extensiones recomendables

#### `print.nl()`
Imprime salto de línea explícito.

#### `print.sp()`
Imprime espacio explícito.

#### `print.tab()`
Imprime tabulación explícita, si se desea.

#### `println()`
Versión sin argumentos para salto simple.

Estas cuatro funciones vuelven la consola mucho más usable sin romper sobriedad.

---

## 5. Render canónico de valores

La impresión perfecta depende de una función central de render de runtime.

Debe existir y quedar cerrada una función conceptual equivalente a:

```text
render_value(RuntimeValue) -> String
```

pero formalizada como pieza oficial del runtime, no como utilidad informal.

## 5.1 Qué debe soportar

### Bool
- `true`
- `false`

### Enteros
- decimal por defecto
- sin basura visual
- respetando signo

### Flotantes
- representación estable
- sin ruido innecesario
- política explícita para truncado, precisión y notación científica

### Str / Text
- salida legible
- UTF-8 bien tratado
- política clara sobre comillas: salida humana por defecto sin comillas, debug con comillas si hace falta

### Null
- `null`

### Addr / ptr
- representación estable tipo `0x1000`
- distinguir `null` de una dirección real
- no tratar punteros como enteros corrientes

### Vector
- formato como:
```text
[1, 2, 3]
```

### Record
- formato como:
```text
{clave: valor, otra: valor}
```

### MemRegion
- render corto por defecto:
```text
<memRegion#3>
```
- render expandido bajo modo debug/inspect si se desea

### Runtime state
Representación resumida y consistente.

### Tipos no imprimibles directamente
Debe existir una política clara:
- o se imprime placeholder;
- o se rechaza;
- o se exige conversión explícita.

Nunca debe quedar en ambigüedad silenciosa.

---

## 6. Modos de impresión

Un lenguaje maduro no usa un único modo textual para todo.

nC debería tener al menos estos modos conceptuales:

### 6.1 Human mode
Pensado para usuario o desarrollador común.

### 6.2 Debug mode
Con más detalle estructural.

### 6.3 Inspect mode
Orientado a introspección del runtime.

### 6.4 Stable mode
Pensado para snapshots reproducibles y tests.

Esto no obliga a exponer cuatro APIs distintas ya mismo, pero sí a diseñar el render pensando en ello desde el principio.

---

## 7. Separación de canales

## 7.1 stdout
Para salida normal del programa:
- `print`
- `println`
- `print.sta`

## 7.2 stderr
Para:
- `print.err`
- parte de `print.wm`, según política elegida

## 7.3 canales lógicos especiales
Si el runtime quiere conservar trazas o inspecciones por separado, puede tener canales internos además de la emisión visible.

### Regla

La semántica de canal debe existir aunque visualmente en una primera fase termine todo en terminal.

---

## 8. Formateo y legibilidad

El problema visible actual en ejemplos como `calculadoraBasica.n` o `bibliotecaCentral.n` no es que el runtime esté roto. El problema es que **la salida es demasiado cruda**.

### Ejemplo actual

```text
241210830
```

Eso probablemente son varios `print(...)` consecutivos sin separación.

### Solución estructural

Debe fijarse una de estas estrategias:

#### Opción A
Responsabilidad total del programador:
- `print` no separa nada;
- `println` sí;
- el usuario mete espacios y saltos a mano.

#### Opción B
Añadir builtins auxiliares:
- `print.sp()`
- `print.nl()`

#### Opción C
Permitir múltiples argumentos:
```nc
print(@a, " ", @b, " ", @c);
```

### Recomendación para nC

La opción más robusta y tradicional es:

- mantener `print(x)` crudo;
- mantener `println(x)` con salto;
- añadir `print.sp()` y `print.nl()`.

Con eso ya se puede producir salida perfecta sin volver barro la gramática.

---

## 9. Familia de impresión avanzada que debería existir o prepararse

No hace falta implementarla toda mañana, pero sí dejar el camino doctrinal limpio.

### Núcleo imprescindible
- `print`
- `println`
- `print.err`
- `print.wm`
- `print.trc`
- `print.dbg`
- `print.sta`
- `print.sp`
- `print.nl`

### Familia avanzadas
- `print.hex(x)`
- `print.bin(x)`
- `print.oct(x)`
- `print.ptr(x)`
- `print.addr(x)`
- `print.fmt(pattern, args...)`
- `print.json(x)`  salida estructurada
- `print.table(...)` solo si encaja con la filosofía futura
- `print.color(...)` como capacidad  de terminal, nunca como semántica central

---

## 10. Requisitos semánticos de impresión por tipo

## 10.1 Tipos básicos
Todos deben ser imprimibles sin fricción:
- `int`
- `ft`
- `bool`
- `char`
- `str`
- `text`

## 10.2 Tipos bajos
Imprimibles con render específico:
- `addr`
- `ptr<T>`
- `skk*`
- `skk**`
- `region`

## 10.3 Colecciones
- vector/list
- watchlist
- queue
- record
- map 

## 10.4 Tipos modales
Debe definirse si:
- se imprimen en resumen,
- se inspeccionan con builtins específicos,
- o ambos.

---

## 11. Requisitos de ejecución

## 11.1 La salida debe ocurrir en runtime, no en parse/sema/IR
Parece obvio, pero hay que congelarlo:

- parser no imprime;
- sema no imprime salvo diagnóstico;
- IR no imprime salvo debugging interno;
- runtime imprime.

## 11.2 Los builtins de consola deben ser visibles a `to main`, `ref main` y `skk main`
Toda la familia main debe poder usarlos sin trucos extraños.

## 11.3 La salida debe funcionar en todos los contenedores autorizados
Siempre que el modo lo permita doctrinalmente, por ejemplo:
- `[@]`
- `primitive[@]`
- `[@machine]` si el perfil lo tolera
- `[@system]`
- `[@security]`
- `[@tooling]`
- `[@compiler]`
- `[@inspect]`
- `[@ai]`

En perfiles extremos o físicos, la política puede restringirse, pero no quedar ambigua.

---

## 12. Diagnóstico y errores

El subsistema de impresión debe tener errores canónicos.

### Ejemplos
- builtin print expects 1 argument
- println expects value printable in current mode
- print.hex requires integer-like value
- print.ptr requires pointer-like value
- console backend unavailable in this execution profile
- print not allowed in this physical phase
- invalid console builtin path

No debe fallar en silencio ni con mensajes improvisados.

---

## 13. Testing obligatorio

Un lenguaje veterano no cierra consola sin tests fuertes.

## 13.1 Tests unitarios
Para:
- `render_value`
- canales
- separación
- newline
- formatos por tipo

## 13.2 Tests end-to-end
Deben comprobar salida real de:
- suma simple
- calculadora
- print family
- biblioteca
- examples de `ref main`
- examples de `skk main`
- ejemplos modales con salida

## 13.3 Golden tests
Comparar stdout/stderr esperado con salida real exacta.

## 13.4 Tests de estabilidad
Comprobar que el render de estructuras complejas no cambia accidentalmente entre versiones.

---

## 14. Portabilidad real

La impresión madura también exige esto:

- UTF-8 estable;
- Windows, Linux, WSL y macOS previsibles;
- política clara de saltos de línea;
- sin dependencia frágil de terminal concreta;
- no mezclar color ANSI obligatorio en toda salida.

---

## 15. Rendimiento y buffering

Para parecer un lenguaje serio, la salida debe respetar rendimiento.

### Debe definirse:
- flush inmediato o no;
- comportamiento de `println`;
- buffering por línea o por stream;
- interacción con trazas intensivas.

### Regla
El sistema debe poder ser correcto por defecto y configurable por perfil, no improvisado.

---

## 16. Integración con `inspect`, `mem` y `state`

nC ya saca cosas como:

- `[inspect] ...`
- `[mem] ...`
- `[state] ...`

Eso es oro. No debe perderse.

### Lo correcto
Formalizar estas salidas como parte del runtime de introspección.

### Deben tener:
- prefijo estable;
- render estable;
- orden estable;
- opción futura de salida machine-readable si se necesita.

---

## 17. Qué se necesita exactamente para que nC imprima “cualquier cosa” perfecto

Aquí va la lista dura y completa.

### Bloque A — Arquitectura
1. mover la consola principal de `ncc-cli` a `ncc-runtime`;
2. crear módulo runtime específico de consola;
3. separar claramente orquestación CLI y ejecución real.

### Bloque B — Builtins
4. crear dispatcher builtin central de impresión;
5. fijar semántica oficial de toda la familia `print`;
6. unificar todas las variantes bajo un único contrato.

### Bloque C — Render
7. formalizar `render_value` como renderer canónico;
8. soportar todos los tipos base y runtime;
9. diferenciar render humano, debug e inspect si hace falta.

### Bloque D — Formato
10. definir separación, newline y espacios;
11. añadir `print.sp()` y `print.nl()` o equivalente;
12. evitar salida pegada no intencional.

### Bloque E — Canales
13. separar stdout y stderr;
14. definir política de warnings y debug;
15. conservar trazas e inspecciones con formato estable.

### Bloque F — Semántica
16. definir qué tipos son imprimibles por defecto;
17. definir qué tipos requieren formato específico;
18. definir errores canónicos de impresión.

### Bloque G — Tests
19. tests unitarios de render;
20. tests end-to-end de stdout/stderr;
21. golden tests;
22. tests modales por contenedor;
23. tests de regresión.

### Bloque H — Pulido de lenguaje acabado
24. política de UTF-8;
25. estabilidad interplataforma;
26. buffering razonable;
27. documentación oficial de la familia console;
28. ejemplos canónicos oficiales bien formateados;
29. contratos versionados para no romper salida futura.

---

## 18. Orden ideal de implementación

## Fase 1 — extracción arquitectónica
- mover render y builtins de consola a runtime.

## Fase 2 — semántica oficial
- congelar familia `print` y contratos.

## Fase 3 — render completo
- soportar todos los tipos principales.

## Fase 4 — formato útil
- `print.sp`, `print.nl`, política newline.

## Fase 5 — tests
- golden tests + end-to-end.

## Fase 6 — pulido veterano
- docs, estabilidad, compatibilidad, ejemplos oficiales.

---

## 19. Definición de “cerrado” para esta área

La consola de nC solo se considera cerrada cuando:

- la ejecución visible no depende estructuralmente del CLI;
- existe dispatcher builtin formal;
- la familia `print` está congelada doctrinalmente;
- `render_value` es estable y canónico;
- la salida de todos los tipos principales es coherente;
- stdout/stderr están definidos;
- los ejemplos complejos se ven legibles;
- existen tests de regresión de salida;
- `inspect`, `mem` y `state` forman parte de un subsistema formal, no un accidente feliz;
- la documentación permite mantener la feature durante años sin reinterpretaciones.

---

## 20. Veredicto final

nC **ya tiene el fuego**.  
Lo que le falta para imprimir como un lenguaje acabado no es magia, sino cierre.

Le falta:

- sacar la voz del CLI;
- darle un runtime de consola propio;
- fijar su gramática de salida;
- cerrar el render de valores;
- añadir formato humano serio;
- endurecerlo con tests.

Cuando eso esté hecho, nC no solo imprimirá números y cadenas.

Imprimirá **cualquier cosa con dignidad de lenguaje maduro**.

Y eso es justo lo que debe hacer un lenguaje soberano.
