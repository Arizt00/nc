# Entrada histórica — operador `+` y concatenación textual

> Registro de diseño preservado para trazabilidad. No define sintaxis vigente ni
> debe copiarse como estilo actual. La autoridad de escritura es
> `docs/spec/language/canonical_compact_syntax_v3.md`.

**Estado:** base de diseño propuesto  
**Área:** operadores, texto, runtime, impresión, semántica, lowering  
**Tema:** uso de `+` como suma y como concatenación, con bajada explícita sin amortiguadores  

---

## 1. Idea central

El operador `+` puede tener dos lecturas en nC:

1. **Suma aritmética**, cuando ambos operandos son numéricos.
2. **Concatenación textual**, cuando participa texto de forma explícita.

Sin embargo, nC no debe convertir valores a texto de forma invisible en zonas críticas. La comodidad puede existir en capas altas, pero el núcleo semántico debe ser explícito: texto, renderizado, concatenación, buffer y salida.

La regla filosófica queda así:

```txt
Comodidad arriba, verdad semántica debajo.
```

Ejemplo cómodo:

```nC
println("Edad: " + @edadActual);
```

Bajada semántica real:

```txt
literal textual -> renderizado del entero -> concatenación -> buffer/salida
```

Forma explícita recomendada:

```nC
println("Edad: " + renderInt(@edadActual));
```

Forma sin amortiguadores:

```nC
@buffer >> memRegion<byte> : fixedBuffer<byte>(64);
@usados >> u64 : writeLiteral(@buffer, 0, "Edad: ");
@usados >> u64 : writeIntAscii(@buffer, @usados, @edadActual);
writeBytes(stdout(), @buffer, @usados);
```

---

## 2. Problema que resuelve

Sin concatenación textual, el código de impresión se vuelve torpe:

```nC
println("Edad: ");
println(@edadActual);
```

Con concatenación textual, el código se vuelve natural:

```nC
println("Edad: " + renderInt(@edadActual));
```

Pero si nC permite demasiada magia, se rompe su filosofía de control:

```nC
println("Edad: " + @edadActual); // cómodo, pero debe tener reglas claras
```

El objetivo es permitir expresividad sin convertir nC en un lenguaje donde el compilador haga conversiones oscuras.

---

## 3. Regla oficial del operador `+`

### 3.1 Suma aritmética

Si ambos operandos son numéricos, `+` significa suma.

```nC
@x >> int : 10 + 20;
@y >> ft64 : 3.5 + 4.2;
```

Resultado:

```txt
int + int     -> int
u64 + u64     -> u64
ft64 + ft64   -> ft64
```

### 3.2 Concatenación textual estricta

Si ambos operandos son texto, `+` significa concatenación.

```nC
@nombreCompleto >> str : @nombre + " " + @apellido;
@mensaje >> str : "Hola " + "mundo";
```

Resultado:

```txt
str + str     -> str
text + text   -> text
str + text    -> text
text + str    -> text
```

### 3.3 Concatenación con render explícito

Si un valor no textual debe participar en una concatenación, debe pasar por una función de renderizado en modo estricto.

```nC
@mensaje >> str : "Edad: " + renderInt(@edadActual);
println("Activo: " + renderBool(@activo));
println("Letra: " + renderChar(@letra));
```

Esto evita conversiones implícitas peligrosas.

---

## 4. Forma cómoda frente a forma explícita

nC puede ofrecer una forma cómoda en capas altas:

```nC
println("Edad: " + @edadActual);
```

Pero esa forma debe entenderse como azúcar sintáctico controlado. El compilador debe bajarla internamente a algo equivalente a:

```nC
println("Edad: " + renderInt(@edadActual));
```

Y luego a:

```nC
println(concat("Edad: ", renderInt(@edadActual)));
```

La forma cómoda no debe existir en capas críticas como `primitive[@]`, `[@]` o `[@machine]`, salvo casos de diagnóstico extremadamente restringidos.

---

## 5. Tres niveles oficiales

## Nivel 1 — Cómodo

Uso pensado para código normal, ejemplos pedagógicos, aplicaciones de consola, tooling y capas altas.

```nC
println("Edad: " + @edadActual);
println("Resultado: " + @puntos + " puntos");
```

Este nivel permite renderizado textual automático controlado, siempre que el tipo sea imprimible y no sensible.

Equivalente semántico:

```nC
println(concat("Edad: ", renderInt(@edadActual)));
```

Permitido principalmente en:

```txt
código normal
[@runtime]
[@tooling]
[@inspect]
[@ai] restringido
[@system] controlado
```

---

## Nivel 2 — Explícito

Uso recomendado para nC serio. No hay render implícito.

```nC
@mensaje >> str : "Edad: " + renderInt(@edadActual);
println(@mensaje);
```

También:

```nC
@mensaje >> str : concat("Edad: ", renderInt(@edadActual));
println(@mensaje);
```

Este nivel obliga a que el programador declare cómo se convierte cada valor a texto.

Permitido en:

```txt
código normal
[@runtime]
[@tooling]
[@inspect]
[@system]
[@security] con restricciones
[@kernel] solo debug/trace restringido
```

---

## Nivel 3 — Sin amortiguadores

Uso para runtime bajo, sistema, kernel-debug, machine bridge o zonas donde se quiere ver la operación completa.

```nC
@buffer >> memRegion<byte> : fixedBuffer<byte>(64)
    !> memRegion<byte> : dev(BufferCreateError @runtime);

@usados >> u64 : writeLiteral(@buffer, 0, "Edad: ")
    !> u64 : dev(BufferWriteError @runtime);

@usados >> u64 : writeIntAscii(@buffer, @usados, @edadActual)
    !> u64 : dev(IntRenderError @runtime);

writeBytes(stdout(), @buffer, @usados)
    !> bool : dev(OutputWriteError @runtime);
```

Este nivel hace explícitos:

```txt
buffer
capacidad
posición
renderizado ASCII/UTF-8
cantidad de bytes usados
escritura en salida
posibles errores
```

---

## 6. Familias necesarias

Para soportar esta arquitectura hacen falta varias familias/librerías.

### 6.1 Entrada y salida

```nC
use stoud.io { output };
```

Habilita:

```nC
print()
println()
write()
flush()
```

### 6.2 Render textual

```nC
use nC.text.render;
```

Habilita:

```nC
renderInt()
renderUInt()
renderFloat()
renderBool()
renderChar()
renderStr()
renderText()
renderHex()
renderBin()
```

### 6.3 Concatenación textual

```nC
use nC.text.concat;
```

Habilita:

```nC
concat()
concatMany()
```

### 6.4 Builder textual

```nC
use nC.text.builder;
```

Habilita:

```nC
builderCreate()
builderAppend()
builderFinish()
builderClear()
```

### 6.5 Buffer explícito

```nC
use nC.runtime.buffer;
use nC.runtime.stream;
```

Habilita:

```nC
fixedBuffer<T>()
writeLiteral()
writeIntAscii()
writeBytes()
stdout()
```

---

## 7. Sintaxis recomendada por capa

### Código normal

```nC
use stoud.io { output };
use nC.text.render;

programa {
    stc:
        edad >> int;

    ejecutar :: in() -> bool {
        edad = 25;
        println("Edad: " + renderInt(edad));
        return true;
    }

    ref main ->> ejecutar;

    fa:
        return true;
}
```

### `[@runtime]`

```nC
use stoud.io { output };
use nC.text.render;
use nC.text.concat;

edadRuntime[@runtime] {
    stc:
        edad >> int;
        mensaje >> str;

    ejecutar :: in() -> bool {
        edad = 25;
        mensaje = concat("Edad: ", renderInt(edad));
        println(mensaje);
        return true;
    }

    ref main ->> ejecutar;

    fa:
        return true;
}
```

### `[@system]`

En `[@system]` se puede usar concatenación, pero conviene dejar explícitos los cruces importantes:

```nC
use stoud.io { output };
use nC.text.render;
use nC.text.concat;

systemLogger[@system] {
    logCodigo :: in(codigo >> int) -> bool {
        println(concat("system-code=", renderInt(codigo)));
        return true;
    }
}
```

### `[@security]`

En `[@security]`, concatenar valores sensibles está prohibido salvo redacción explícita.

Incorrecto:

```nC
println("Token: " + renderText(@token));
```

Correcto:

```nC
println("Token: " + redact(@token));
```

### `[@machine]`

En `[@machine]`, `+` debe conservarse principalmente como suma, desplazamiento u operación cercana a layout.

No recomendado:

```nC
println("Registro: " + renderHex(@rax));
```

Permitido solo bajo puente de diagnóstico:

```nC
use nC.runtime.trace with bridge(machine, debug);
use nC.text.render with bridge(machine, debug);

machineProbe[@machine] {
    fn:
        debugReg(valor: u64) -> bool {
            traceMachine("reg=" + renderHex(@valor));
            return true;
        }
}
```

### `[@]`

Prohibido. `[@]` no debe depender de runtime/texto cómodo.

```nC
rootCore[@] {
    iniciar :: in() -> bool {
        println("Root"); // ERROR
        return true;
    }
}
```

### `primitive[@]`

Totalmente prohibido. No hay strings cómodos, no hay `println`, no hay render automático.

```nC
primitive[@]:
    emit 0x55AA;
    // println("boot") no existe aquí
;
```

---

## 8. Tabla de permisos por modo

| Modo / capa | `+` suma | `+` concat cómodo | `+` concat explícito | builder/buffer explícito | Motivo |
|---|---:|---:|---:|---:|---|
| Código normal | Sí | Sí | Sí | Opcional | Ergonomía |
| `[@runtime]` | Sí | Sí | Sí | Sí | El runtime puede construir texto |
| `[@tooling]` | Sí | Sí | Sí | Sí | Diagnósticos, mensajes, reportes |
| `[@inspect]` | Sí | Sí | Sí | Sí | Trazas, timelines, informes |
| `[@ai]` | Sí | Sí restringido | Sí | Opcional | Texto generado, pero con política |
| `[@system]` | Sí | Limitado | Sí | Sí | Sistema debe ser explícito |
| `[@security]` | Sí | Muy limitado | Sí con redacción | Sí | Evitar filtración de secretos |
| `[@kernel]` | Sí | No normal | Debug/trace restringido | Sí restringido | No depender de texto cómodo |
| `[@machine]` | Sí | No normal | Bridge debug | Sí muy restringido | `+` suele ser suma/layout/offset |
| `[@]` | Sí si aplica a valores propios | No | No | No | Raíz sin runtime |
| `primitive[@]` | Operación física si se define | No | No | No | Sin runtime, sin texto alto |

---

## 9. Tipos válidos e inválidos

### 9.1 Válidos

```txt
int + int         -> int
u64 + u64         -> u64
ft64 + ft64       -> ft64
str + str         -> str
text + text       -> text
str + text        -> text
text + str        -> text
str + renderInt(x) -> str
text + renderBool(x) -> text
```

### 9.2 Inválidos en modo estricto

```txt
str + int
str + bool
str + ptr<T>
str + addr
str + memRegion<T>
str + secret<T>
ptr<T> + str
addr + str
memRegion<T> + str
```

### 9.3 Casos que requieren render explícito

```nC
"Edad: " + renderInt(@edad)
"Activo: " + renderBool(@activo)
"Byte: " + renderHex(@byte)
"Letra: " + renderChar(@letra)
```

---

## 10. Relación con `print` y `println`

`print` y `println` deben aceptar texto final, no valores arbitrarios sin contrato.

Forma cómoda:

```nC
println("Edad: " + @edad);
```

Forma estricta:

```nC
println("Edad: " + renderInt(@edad));
```

Forma canónica interna:

```nC
println(concat("Edad: ", renderInt(@edad)));
```

Forma baja:

```nC
writeBytes(stdout(), @buffer, @usados);
```

Regla:

```txt
print/println no deben ser vertederos mágicos de cualquier tipo.
Deben recibir texto o valores renderizados bajo contrato.
```

---

## 11. Bajada a AST

Código fuente:

```nC
@mensaje >> str : "Edad: " + renderInt(@edadActual);
```

AST conceptual:

```txt
Projection
 ├─ target: AtRef(edadActual)
 ├─ declaredType: str
 └─ value:
     BinaryOp(+)
      ├─ StrLiteral("Edad: ")
      └─ Call(renderInt)
          └─ AtRef(edadActual)
```

Después de sema, el `BinaryOp(+)` textual puede normalizarse a:

```txt
Concat
 ├─ StrLiteral("Edad: ")
 └─ Call(renderInt)
     └─ AtRef(edadActual)
```

---

## 12. Bajada a nIR

Código:

```nC
@mensaje >> str : "Edad: " + renderInt(@edadActual);
```

nIR conceptual:

```txt
%0 = LOAD @edadActual
%1 = CALL nC.text.render.renderInt(%0)
%2 = CALL nC.text.concat.concat("Edad: ", %1)
STORE @mensaje, %2
```

Código:

```nC
println("Edad: " + renderInt(@edadActual));
```

nIR conceptual:

```txt
%0 = LOAD @edadActual
%1 = CALL renderInt(%0)
%2 = CALL concat("Edad: ", %1)
CALL println(%2)
```

Forma sin amortiguadores:

```txt
%buffer = CALL fixedBuffer<byte>(64)
%pos0 = CALL writeLiteral(%buffer, 0, "Edad: ")
%pos1 = CALL writeIntAscii(%buffer, %pos0, @edadActual)
CALL writeBytes(stdout, %buffer, %pos1)
```

---

## 13. Diagnósticos canónicos

### 13.1 Concatenación implícita prohibida

```nC
@mensaje >> str : "Edad: " + @edad;
```

En modo estricto:

```txt
error[E_TEXT_PLUS_001]: implicit text rendering is not allowed in strict mode
  --> src/main.nc:12:29
   |
12 | @mensaje >> str : "Edad: " + @edad;
   |                             ^^^^^ value `@edad` has type int
   |
help: use explicit rendering
   |
12 | @mensaje >> str : "Edad: " + renderInt(@edad);
```

### 13.2 Texto usado como número

```nC
@x >> int : "10" + 5;
```

Diagnóstico:

```txt
error[E_PLUS_002]: operator '+' cannot produce int from str and int
  --> src/main.nc:8:12
   |
8  | @x >> int : "10" + 5;
   |            ^^^^^^^^
   |
help: parse the text explicitly if numeric conversion is intended
   |
8  | @x >> int : parseInt("10") + 5;
```

### 13.3 Render de valor sensible

```nC
println("Token: " + renderText(@token));
```

Diagnóstico:

```txt
error[E_SEC_RENDER_001]: cannot render sensitive value into text
  --> src/security.nc:44:27
   |
44 | println("Token: " + renderText(@token));
   |                           ^^^^^^^^^^^^^ sensitive value
   |
help: use an explicit redacted representation
   |
44 | println("Token: " + redact(@token));
```

### 13.4 Concatenación prohibida en `primitive[@]`

```nC
primitive[@]:
    emit "boot=" + renderHex(0x7C00);
;
```

Diagnóstico:

```txt
error[E_PRIMITIVE_TEXT_001]: text concatenation is not available in primitive[@]
  --> boot.ncp:3:17
   |
3  | emit "boot=" + renderHex(0x7C00);
   |                 ^^^^^^^^^^^^^^^^^
   |
reason: primitive[@] has no runtime text layer
```

---

## 14. Seguridad

El renderizado textual puede filtrar información. Por eso, en `[@security]` y zonas sensibles, nC debe distinguir valores imprimibles de valores secretos.

Tipos o marcas sensibles:

```txt
secret<T>
credential
privateKey
sessionToken
capability
sensitiveRegion<T>
```

Incorrecto:

```nC
println("Clave: " + renderText(@privateKey));
println("Token: " + @token);
```

Correcto:

```nC
println("Clave: " + redact(@privateKey));
println("Token: " + redact(@token));
```

Regla:

```txt
Un valor sensible nunca se convierte a texto sin una función explícita de redacción, hashing seguro o representación autorizada.
```

---

## 15. Influencia de perfiles

### `profile normal`

Permite forma cómoda:

```nC
println("Edad: " + @edad);
```

### `profile strict`

Exige render explícito:

```nC
println("Edad: " + renderInt(@edad));
```

### `profile secure`

Exige render explícito y bloquea secretos:

```nC
println("Token: " + redact(@token));
```

### `profile machine`

Prohíbe concatenación cómoda. Solo bridge debug si se declara.

### `profile primitive`

Sin concatenación textual.

---

## 16. Comparación conceptual con otros lenguajes

### Java

Java permite:

```java
System.out.println("Edad: " + edad);
```

Ventaja: cómodo.  
Problema: oculta bastante conversión.

nC puede tomar la ergonomía, pero no la opacidad.

### C

C fuerza a pensar en formato y buffer:

```c
printf("Edad: %d\n", edad);
```

Ventaja: más explícito.  
Problema: riesgo de formato, buffer y seguridad.

nC debe aprender de C, pero con tipos y diagnósticos mejores.

### C++

C++ permite streams:

```cpp
std::cout << "Edad: " << edad << std::endl;
```

Ventaja: composición clara.  
Problema: semántica pesada y a veces compleja.

nC puede tomar la idea de composición, pero con lowering visible.

### Rust

Rust usa formato explícito:

```rust
println!("Edad: {}", edad);
```

Ventaja: seguro y controlado.  
Problema: macro/sistema propio.

nC debería acercarse más a Rust en seguridad, pero conservar una sintaxis limpia.

### ASM

ASM no concatena texto. Escribe bytes.

```asm
mov rax, 1
mov rdi, 1
mov rsi, buffer
mov rdx, len
syscall
```

Lección central:

```txt
La máquina no concatena strings.
La máquina mueve bytes.
```

nC debe recordar esto siempre.

---

## 17. Ejemplo completo recomendado

```nC
use nC.runtime.input;
use nC.runtime.output;
use nC.text.render;
use nC.text.concat;

edadConTexto[@runtime] {
    stc:
        edadActual: int;
        mensaje: str;

    fn:
        ejecutar() -> bool {
            print("Por favor, ingresa tu edad: ");

            @edadActual >> int : readInt()
                !> int : dev(InputParseError @runtime);

            @mensaje >> str : concat(
                "Has introducido la edad: ",
                renderInt(@edadActual)
            );

            println(@mensaje);
            println("Edad confirmada: " + renderInt(@edadActual));

            return true;
        }

    ref main ->> ejecutar;

    fa:
        return true;
}
```

---

## 18. Ejemplo sin amortiguadores

```nC
use nC.runtime.output;
use nC.runtime.buffer;
use nC.runtime.stream;
use nC.text.raw;

edadCruda[@runtime] {
    stc:
        edad: int;
        buffer: memRegion<byte>;
        usados: u64;

    fn:
        ejecutar() -> bool {
            @edad >> int : 25;

            @buffer >> memRegion<byte> : fixedBuffer<byte>(64)
                !> memRegion<byte> : dev(BufferCreateError @runtime);

            @usados >> u64 : writeLiteral(@buffer, 0, "Edad: ")
                !> u64 : dev(BufferWriteError @runtime);

            @usados >> u64 : writeIntAscii(@buffer, @usados, @edad)
                !> u64 : dev(IntRenderError @runtime);

            writeBytes(stdout(), @buffer, @usados)
                !> bool : dev(OutputWriteError @runtime);

            return true;
        }

    ref main ->> ejecutar;

    fa:
        return true;
}
```

---

## 19. Reglas finales

```txt
1. `+` conserva suma aritmética cuando ambos operandos son numéricos.
2. `+` puede representar concatenación cuando ambos operandos son texto.
3. En modo cómodo, `str + valor` puede bajar a `str + render(valor)` si el valor es imprimible y no sensible.
4. En modo estricto, todo valor no textual debe usar render explícito.
5. En `[@security]`, los valores sensibles no pueden renderizarse sin redacción explícita.
6. En `[@machine]`, la concatenación queda prohibida salvo bridge debug muy restringido.
7. En `[@]` y `primitive[@]`, no existe concatenación textual mediante runtime.
8. La bajada real debe pasar por concat, builder, buffer o writeBytes según nivel.
9. `print` y `println` no deben esconder conversiones peligrosas.
10. nC puede ser cómodo, pero nunca debe mentir sobre los bytes que mueve.
```

---

## 20. Cierre canónico

El operador `+` puede hacer que nC sea agradable de escribir, pero su implementación no debe convertirse en una caja negra. En código alto, `println("Edad: " + @edad)` es una comodidad válida. En código serio, `renderInt(@edad)` declara la intención. En zonas bajas, el texto se convierte en bytes, los bytes van a un buffer, y el buffer se escribe en una salida concreta.

La superficie puede ser limpia. El fondo debe ser honesto.

```txt
nC permite belleza arriba,
pero exige metal debajo.
```
# Historical base input

This file preserves an original language-design input. Its snippets are not the
current authoring surface. Use `docs/spec/language/canonical_compact_syntax_v3.md`
for executable code.
