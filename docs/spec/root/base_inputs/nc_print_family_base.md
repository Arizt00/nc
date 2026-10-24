# Base activo — Familia `print` de nC

Hasta que exista contradicción formal posterior, este base debe considerarse la base estable, oficial y activa de la familia `print` en nC.

---

## 1. Naturaleza de la familia `print`

La familia `print` no es una simple colección de alias para "mostrar cosas por pantalla". En nC, `print` es una **familia modal de salida, exposición, inspección y contacto de superficie**.

Eso significa que sus miembros no comparten solo un nombre: comparten una misión estructural.

La misión general de la familia es permitir que un programa:

- emita texto humano legible;
- exprese estado operativo;
- marque errores y warnings con canal semántico correcto;
- muestre trazas y depuración con niveles de verdad distintos;
- inspeccione tipos, memoria, sitios y bindings;
- emita salida cruda y determinista;
- y, en sus miembros más extremos, cruce desde el lenguaje hacia la materia, la máquina o lo físico directo.

Por eso, en nC, `print` **no debe entenderse como un único builtin con variantes cosméticas**. Debe entenderse como una **familia estratificada**.

---

## 2. Total oficial de miembros

La familia `print` queda fijada en **22 miembros oficiales**.

### 2.1 Familia humana y operativa
1. `print(...)`
2. `println(...)`
3. `print.err(...)`
4. `print.wm(...)`
5. `print.inf(...)`
6. `print.ok(...)`
7. `print.sta(...)`

### 2.2 Familia diagnóstica
8. `print.trc(...)`
9. `print.dbg(...)`

### 2.3 Familia estructurada
10. `print.inspect(...)`
11. `print.mem(...)`
12. `print.site(...)`
13. `print.type(...)`
14. `print.raw(...)`
15. `print.stable(...)`

### 2.4 Familia soberana y cruda
16. `print.addr(...)`
17. `print.phys(...)`
18. `print.dump(...)`
19. `print.peek(...)`
20. `print.mmio(...)`
21. `print.port(...)`

### 2.5 Miembro terminal físico
22. `print.sensory(...)`

---

## 3. Principio estructural

La familia se divide por **grado de crudeza y verdad expuesta**.

### Nivel A — Humano
Pensado para lectura corriente, mensajes al usuario, consola general y flujo operativo normal.

### Nivel B — Diagnóstico
Pensado para seguir ejecución y comprender valores con más verdad que la salida humana simple.

### Nivel C — Estructurado
Pensado para mostrar información con intención semántica fuerte: tipos, memoria, sitios, determinismo, inspección.

### Nivel D — Soberano y crudo
Pensado para tocar verdad material o casi material: direcciones, volcado de bytes, MMIO, puertos, direcciones físicas.

### Nivel E — Terminal físico
Pensado para cruce final desde `primitive[@]` hacia interacción física directa, fuera ya del confort de la capa máquina legible.

---

## 4. Reglas generales de toda la familia

### Regla 1
El compilador y la semántica mandan. Ningún editor, alias o costumbre de otro lenguaje puede redefinir la familia `print`.

### Regla 2
Los miembros de la familia **no son equivalentes**. Cada uno tiene:

- un propósito;
- un grado de crudeza;
- un contrato de render;
- un canal preferente;
- y una autoridad contextual.

### Regla 3
Los miembros humanos no deben usarse para fingir inspección profunda.

### Regla 4
Los miembros estructurados no deben fingir ser salida humana amable.

### Regla 5
Los miembros soberanos y físicos no deben quedar disponibles por defecto en contenedores altos o perfiles novatos.

### Regla 6
Una parte de la familia escribe a `stdout`, otra a `stderr`, y la operación terminal `print.sensory(...)` ni siquiera debe tratarse como simple salida de consola.

### Regla 7
La crudeza debe crecer de forma monotónica. Es decir: a medida que subimos hacia `print.addr`, `print.dump`, `print.mmio`, `print.port` y `print.sensory`, la familia muestra menos maquillaje y más verdad directa.

### Regla 8
La familia `print` reconoce controles de texto dentro de literales:

- `\n` representa un salto de línea real.
- `\&` imprime `&` de forma literal.
- `\?` imprime `?` de forma literal.
- `\#` imprime `#` de forma literal.
- `##`, cuando es el contenido completo enviado a `print`, `println`, `write` o `writeln`, se expande a una línea separadora estable para menús e informes.

Estos controles viven en la familia de salida. El lexer decodifica los escapes y el runtime normaliza los marcadores de impresión antes de escribir en pantalla.

---

## 5. Canales oficiales

### `stdout`
Canal por defecto para:

- `print`
- `println`
- `print.inf`
- `print.ok`
- `print.sta`
- `print.trc`
- `print.dbg`
- `print.inspect`
- `print.mem`
- `print.site`
- `print.type`
- `print.raw`
- `print.stable`
- `print.addr`
- `print.phys`
- `print.dump`
- `print.peek`

### `stderr`
Canal por defecto para:

- `print.err`
- `print.wm`

### Canal físico / puente soberano
No debe modelarse como simple `stdout` o `stderr` para:

- `print.mmio`
- `print.port`
- `print.sensory`

Estos pueden reflejar resultado a consola, pero su naturaleza no es meramente textual: implican contacto con superficie material o semi-material.

---

## 6. Miembros oficiales, uno por uno

---

### 6.1 `print(...)`

#### Qué hace
Imprime una representación **humana básica** sin salto de línea.

#### Qué no hace
- no inspecciona tipo;
- no inspecciona memoria;
- no garantiza determinismo estructural;
- no debe maquillarse como debug profundo.

#### Uso típico
- prompts;
- composición manual de línea;
- texto previo a un valor.

#### Ejemplo
```nc
print("Edad: ");
print(@edad);
```

#### Restricciones
- permitido ampliamente en modos altos y medios;
- permitido en `[@]` cuando el programa es basal pero aún humano;
- desaconsejado en `[@machine]` y `primitive[@]` salvo demo o debug explícito.

#### Riesgo si se usa mal
El riesgo no es físico; el riesgo es **semántico**: usar `print(...)` para tareas que deberían hacerse con `print.dbg`, `print.inspect` o `print.stable` produce ruido y diagnósticos pobres.

---

### 6.2 `println(...)` / `println()`

#### Qué hace
Imprime una representación humana con salto de línea. La forma vacía emite una línea en blanco.

#### Qué no hace
- no es una operación estructurada;
- no añade metadata;
- no debe reinterpretar el valor como dump o debug.

#### Ejemplo
```nc
println("Hola");
println(@edad);
println();
```

#### Restricciones
Iguales a `print(...)`.

#### Riesgo si se usa mal
Mismo riesgo que `print(...)`: pobreza de contrato, no daño físico.

---

### 6.3 `print.err(...)`

#### Qué hace
Emite un error semántico u operativo por el canal de error.

#### Qué no hace
- no sustituye un diagnóstico formal del compilador;
- no significa excepción automática;
- no repara el fallo.

#### Ejemplo
```nc
print.err("No se pudo abrir el recurso.");
```

#### Restricciones
- permitido ampliamente;
- útil en casi todos los contenedores salvo `primitive[@]`, donde su uso debe ser rarísimo.

#### Riesgo si se usa mal
Saturar `stderr` con mensajes que no son realmente error. Eso corrompe la lectura del sistema.

---

### 6.4 `print.wm(...)`

#### Qué hace
Emite una advertencia semánticamente reconocible por el canal de warning.

#### Qué no hace
- no equivale a error fatal;
- no debe usarse como mensaje informativo normal.

#### Ejemplo
```nc
print.wm("Entrada inválida; se vuelve a pedir.");
```

#### Riesgo si se usa mal
Normalizar warnings para cosas triviales destruye su valor operativo.

---

### 6.5 `print.inf(...)`

#### Qué hace
Emite información operativa humana.

#### Qué no hace
- no es confirmación de éxito fuerte;
- no es traza;
- no es debug.

#### Ejemplo
```nc
print.inf("Proceso de validación iniciado.");
```

#### Riesgo si se usa mal
Convertir `print.inf` en un log monstruoso. Es una salida de información, no un vertedero.

---

### 6.6 `print.ok(...)`

#### Qué hace
Marca éxito, cierre positivo o confirmación satisfactoria.

#### Qué no hace
- no implica que todo el sistema esté sano;
- no sustituye `print.sta`.

#### Ejemplo
```nc
print.ok("Cuenta creada correctamente.");
```

#### Riesgo si se usa mal
Dar mensajes de éxito engañosos donde aún no se ha garantizado el cierre correcto.

---

### 6.7 `print.sta(...)`

#### Qué hace
Emite estado resumido y legible.

#### Qué no hace
- no es dump;
- no es debug;
- no es inspección profunda.

#### Ejemplo
```nc
print.sta("estado nominal");
print.sta(@activo);
```

#### Riesgo si se usa mal
Usarlo como alias de cualquier cosa acaba vaciando su identidad.

---

### 6.8 `print.trc(...)`

#### Qué hace
Emite traza de ejecución.

#### Qué no hace
- no pretende ser salida de usuario final;
- no debe sustituir `print.dbg` si lo importante es el valor concreto.

#### Ejemplo
```nc
print.trc("entrando en pedirAccion");
```

#### Riesgo si se usa mal
Inundar la ejecución con trazas inútiles y volver ilegible el recorrido real.

---

### 6.9 `print.dbg(...)`

#### Qué hace
Emite depuración tipada. Debe mostrar más verdad interna que la salida humana simple.

#### Qué no hace
- no inspecciona binding completo;
- no debe ser dump de memoria.

#### Ejemplo
```nc
print.dbg(@edad);
print.dbg("valor temporal");
```

#### Render esperado
Ejemplos razonables:
- `num(42)`
- `str("hola")`
- `bool(true)`

#### Riesgo si se usa mal
Exponer detalles internos en rutas donde debería imprimirse solo salida humana limpia.

---

### 6.10 `print.inspect(...)`

#### Qué hace
Emite inspección rica del valor o estructura.

#### Qué no hace
- no debe degradarse a simple `dbg`;
- no sustituye `print.site` si lo importante es el sitio.

#### Ejemplo
```nc
print.inspect(@mensaje);
print.inspect(@registro);
```

#### Riesgo si se usa mal
Exceso de verbosidad y mezcla de inspección con experiencia de usuario normal.

---

### 6.11 `print.mem(...)`

#### Qué hace
Expone información de memoria o binding materializado asociado al símbolo o recurso.

#### Qué no hace
- no debe inventar memoria si no existe binding real;
- no debe reinterpretar un valor muerto como válido.

#### Ejemplo
```nc
print.mem(@edad);
print.mem(@buffer);
```

#### Qué debería poder mostrar
- dirección;
- clase;
- región;
- owner;
- tamaño;
- alineación;
- estado del binding.

#### Riesgo si se usa mal
Exposición innecesaria de layout o información sensible.

---

### 6.12 `print.site(...)`

#### Qué hace
Expone el sitio efectivo de un valor. Está ligado conceptualmente a `siteOf(...)`.

#### Qué no hace
- no debe confundirse con `print.mem`;
- no es dump;
- no es dirección física por defecto.

#### Ejemplo
```nc
print.site(@edad);
print.site(@ptrDato);
```

#### Qué debería poder mostrar
- región;
- clase;
- owner;
- dirección lógica;
- estado;
- tamaño;
- alineación.

#### Riesgo si se usa mal
Romper opacidad arquitectónica o filtrar más estructura de la necesaria.

---

### 6.13 `print.type(...)`

#### Qué hace
Imprime el tipo efectivo del valor.

#### Qué no hace
- no imprime memoria;
- no imprime sitio;
- no imprime dump.

#### Ejemplo
```nc
print.type(@edad);
print.type(@mensaje);
```

#### Riesgo si se usa mal
Bajo. El problema suele ser más de ruido que de daño.

---

### 6.14 `print.raw(...)`

#### Qué hace
Emite salida cruda, con el mínimo maquillaje posible.

#### Qué no hace
- no es dump profundo de bytes salvo que el tipo mismo ya sea crudo;
- no debe reescribir el valor como una representación humana adornada.

#### Ejemplo
```nc
print.raw(@byte);
print.raw(@buffer);
```

#### Restricciones
- permitido en modos técnicos y bajos;
- desaconsejado en rutas humanas convencionales.

#### Riesgo si se usa mal
Exponer información demasiado desnuda donde debería haber render humano o redacción.

---

### 6.15 `print.stable(...)`

#### Qué hace
Emite una representación estable, determinista y reproducible.

#### Qué no hace
- no debe depender de orden accidental de hashmaps, layouts temporales o render variable;
- no debe embellecer según el terminal.

#### Ejemplo
```nc
print.stable(@registro);
print.stable(@estadoGlobal);
```

#### Riesgo si se usa mal
El mayor riesgo es romper su promesa de estabilidad; si eso pasa, deja de servir para tests, snapshots y comparaciones.

---

### 6.16 `print.addr(...)`

#### Qué hace
Imprime la dirección efectiva exacta del símbolo o recurso.

#### Qué no hace
- no imprime dirección física por defecto;
- no corrige bindings inválidos;
- no oculta la dirección real.

#### Ejemplo
```nc
print.addr(@edad);
print.addr(@buffer);
print.addr(@ptrNodo);
```

#### Restricciones
- permitido principalmente en `[@]`, `[@machine]`, `[@kernel]`, `primitive[@]`, y contextos técnicos autorizados;
- prohibido por defecto en contenedores altos normales.

#### Riesgo si se usa mal
Exposición de layout real, fuga de información, ruptura de opacidad de memoria.

---

### 6.17 `print.phys(...)`

#### Qué hace
Imprime la dirección física real asociada al recurso, si existe mapeo físico válido.

#### Qué no hace
- no inventa dirección física;
- no funciona sin mapping físico real.

#### Ejemplo
```nc
print.phys(@pageBuffer);
print.phys(@sectorWindow);
```

#### Restricciones
- permitido en `[@machine]`, `[@kernel]`, `primitive[@]`;
- permitido en `[@]` solo si existe contexto soberano claro;
- prohibido en el resto por defecto.

#### Riesgo si se usa mal
Exposición de topología física, fuga de información sensible de hardware o memoria material.

---

### 6.18 `print.dump(...)`

#### Qué hace
Realiza un volcado bruto de bytes o celdas de una región o recurso.

#### Qué no hace
- no redacta secretos;
- no humaniza demasiado el resultado;
- no debe usarse como salida cotidiana.

#### Ejemplo
```nc
print.dump(@buffer, 64);
print.dump(@regionCritica, 128);
```

#### Restricciones
- solo en contenedores y perfiles con autoridad técnica alta;
- especialmente restringido en `[@security]`.

#### Riesgo si se usa mal
Fuga masiva de secretos, estado interno, claves, credenciales, buffers sensibles.

---

### 6.19 `print.peek(...)`

#### Qué hace
Lee una dirección exacta y desreferencia su contenido para imprimirlo.

#### Qué no hace
- no valida bondadosamente direcciones absurdas;
- no garantiza ausencia de side effects en la lectura.

#### Ejemplo
```nc
print.peek(0x1000, byte);
print.peek(0x2000, word);
print.peek(0x3000, dword);
```

#### Restricciones
- permitido en `[@machine]`, `[@kernel]`, `primitive[@]` y `[@]` experto;
- prohibido por defecto en el resto.

#### Riesgo si se usa mal
Leer dirección inválida, tocar recursos indebidos, disparar lecturas con efectos laterales.

---

### 6.20 `print.mmio(...)`

#### Qué hace
Lee una dirección MMIO y emite el valor resultante.

#### Qué no hace
- no debe tratarse como un print inocente;
- no equivale a `peek` genérico.

#### Ejemplo
```nc
print.mmio(0xFEC00020, dword);
print.mmio(@timerBase, qword);
```

#### Restricciones
- permitido en `[@machine]`, `[@kernel]`, `primitive[@]`;
- prohibido por defecto fuera de ellos.

#### Riesgo si se usa mal
Una lectura MMIO puede limpiar flags, reconocer interrupciones, consumir estado del dispositivo o alterar handshake material.

---

### 6.21 `print.port(...)`

#### Qué hace
Lee desde un puerto de I/O y emite el valor.

#### Qué no hace
- no debe modelarse como simple print textual;
- no es equivalente a `mmio`.

#### Ejemplo
```nc
print.port(0x60, byte);
print.port(0x64, byte);
print.port(0x3F8, byte);
```

#### Restricciones
- permitido en `[@machine]`, `primitive[@]`, `[@kernel]` bajo política explícita;
- prohibido en el resto.

#### Riesgo si se usa mal
Consumir datos de controlador, alterar sincronización, dejar periféricos en estado inconsistente.

---

### 6.22 `print.sensory(...)`

#### Qué hace
`print.sensory(...)` es el miembro más delicado, explícito y peligroso de toda la familia.

No debe entenderse como un print de consola. Debe entenderse como una **operación soberana terminal** donde `primitive[@]` cruza un puente especial —tentativamente llamado `sensory`— y habla con lo físico directamente.

#### Qué no hace
- no es alias de `print.raw(...)`;
- no es alias de `print.mmio(...)`;
- no es alias de `print.port(...)`;
- no es dump;
- no es debug;
- no es salida humana.

#### Qué lo distingue
En `print.mmio(...)` y `print.port(...)` aún seguimos pensando con lenguaje de máquina: registro, puerto, lectura de recurso material mediado.

En `print.sensory(...)`, el modelo cambia: el programa deja de operar como máquina legible y entra en una semántica de **contacto físico directo desde `primitive[@]`**.

#### Restricciones absolutas
`print.sensory(...)`:

- solo puede existir en `primitive[@]`;
- requiere un puente soberano `sensory` activo y válido;
- no puede usarse desde `[@machine]`;
- no puede usarse desde `[@kernel]`, `[@system]`, `[@runtime]`, `[@network]`, `[@security]`, `[@tooling]`, `[@compiler]`, `[@inspect]`, `[@ai]`;
- no debe formar parte del perfil novato ni del perfil estándar;
- requiere perfil experto y autoridad soberana explícita;
- requiere lowering y validación propios;
- debe poder fallar si el destino físico sensorial no existe, no está autorizado o no puede materializarse.

#### Ejemplo conceptual
```nc
print.sensory(@waveform);
print.sensory(@materialPulse);
```

#### Riesgo
Aquí ya no hablamos de una mala salida textual. Hablamos de:

- contacto físico directo indebido;
- emisión o lectura sensorial fuera de política;
- corrupción de handshake material;
- side effects irreversibles;
- ruptura de aislamiento entre lenguaje, máquina y materia.

#### Diagnósticos mínimos
- `print.sensory is only allowed in primitive[@]`
- `sensory bridge not declared`
- `sensory capability missing`
- `sensory print forbidden under current authority`
- `sensory lowering not available for current target`
- `print.sensory cannot be lowered through machine layer`

---

## 7. Restricciones por contenedor

### `[@]`
Permitido con naturalidad:
- familia humana;
- diagnóstica;
- estructurada.

Permitido con autoridad técnica:
- `print.addr`
- `print.peek`
- `print.site`
- `print.mem`
- `print.raw`
- `print.stable`

No canónico por defecto:
- `print.phys`
- `print.dump`
- `print.mmio`
- `print.port`
- `print.sensory`

### `[@runtime]`, `[@system]`, `[@network]`, `[@tooling]`, `[@compiler]`, `[@ai]`
Permitidos:
- familia humana;
- diagnóstica;
- buena parte de la estructurada.

Restringidos o prohibidos:
- `print.addr`
- `print.phys`
- `print.dump`
- `print.peek`
- `print.mmio`
- `print.port`
- `print.sensory`

### `[@inspect]`
Permitido:
- casi toda la familia estructurada;
- algunos miembros soberanos bajo perfil técnico.

Prohibido por defecto:
- `print.sensory`

### `[@security]`
Permitido:
- humana, diagnóstica, estructurada controlada.

Muy restringido:
- `print.raw`
- `print.dump`
- `print.addr`
- `print.phys`

La política de redacción debe ser dura.

### `[@kernel]`
Canónicos:
- `print.err`
- `print.wm`
- `print.trc`
- `print.dbg`
- `print.inspect`
- `print.type`
- `print.stable`
- `print.raw`
- `print.addr`
- `print.phys`
- `print.peek`
- `print.mmio`
- `print.port`

No canónico:
- `print.sensory`

### `[@machine]`
Canónicos:
- `print.trc`
- `print.dbg`
- `print.type`
- `print.site`
- `print.mem`
- `print.raw`
- `print.stable`
- `print.addr`
- `print.phys`
- `print.dump`
- `print.peek`
- `print.mmio`
- `print.port`

No permitido:
- `print.sensory`

### `primitive[@]`
Permitidos de manera natural:
- `print.raw`
- `print.stable`
- `print.addr`
- `print.phys`
- `print.dump`
- `print.peek`
- `print.mmio`
- `print.port`
- `print.sensory`

Humanos y operativos:
- desaconsejados o restringidos fuertemente.

---

## 8. Perfiles editoriales

### Perfil novato
Visible y sugerido:
- `print`
- `println`
- `print.err`
- `print.wm`
- `print.inf`
- `print.ok`
- `print.sta`

Oculto o bloqueado:
- resto

### Perfil estándar
Visible:
- familia humana;
- diagnóstica;
- estructurada básica.

Restringido:
- familia soberana y cruda
- `print.sensory`

### Perfil experto
Visible:
- familia completa según contexto y autoridad

---

## 9. Compatibilidad y deprecaciones

### Obsoletos o en retirada
- `print.sp(...)`
- `print.nl(...)`

### Alias transitorio permitido
- `print.state(...)` como alias temporal de `print.sta(...)`

Pero el base corto y oficial es:
- `print.sta(...)`

---

## 10. Conclusión doctrinal

La familia `print` de nC no debe tratarse como decoración sintáctica. Debe tratarse como una **jerarquía de exposición de verdad**.

- La familia humana comunica.
- La diagnóstica sigue la ejecución.
- La estructurada explica.
- La soberana y cruda desnuda la máquina y la materia.
- `print.sensory(...)` cruza el último umbral y habla con lo físico directamente.

Ese umbral final no es un capricho. Es una frontera doctrinal del lenguaje.

Mientras no exista contradicción formal posterior, esta especificación debe considerarse la forma oficial de la familia `print` en nC.
