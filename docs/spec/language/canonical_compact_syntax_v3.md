# Sintaxis compacta canonica v3

Estado: forma oficial hibrida para codigo `.n`.

## Autoridad

El fuente se escribe en secciones cortas y el compilador lo normaliza al AST
tipado. Toda compilacion normal usa semantica manual/raw y baja por la ruta
nativa. No hay ownership, lifetime, bounds, procedencia, limpieza o scheduler
implicitos. Las auditorias son herramientas opcionales y no cambian los bytes.

## Imports y contenedor

```nc
use <io:output, render>;
use <memory:region>;

archivo {
    // secciones
}
```

El nombre simple del archivo sigue siendo su contenedor. El encabezado normal
es `nombre {`. Un rol se escribe solo cuando cambia materialmente el dominio:
`nombre :: kernel {`, `:: machine`, `:: primitive`, `:: runtime`, etc.

## Secciones

```nc
programa {
    const:
        CAP >> usize <- 64;

    type:
        Packet {
            public:
                tag >> int;
            private:
                flags >> byte;
            layout:
                repr(C);
                align(16);
            constructure:
                static(tag >> int, flags >> byte) -> Packet;
        }

    stc:
        executions >> u64 <- 0;

    local:
        scratch >> int <- 0;

    fn:
        internal:
            helper(value >> int) -> int { return value * 2; }

        public:
            run() -> bool { return helper(21) == 42; }

    ref main ->> run;

    fa:
        return true;
}
```

Solo puede existir un `stc:` por contenedor. `const:`, `type:` y `local:`
separan constantes, contratos de tipo y almacenamiento local para evitar
contenedores o funciones ilegibles. `public`, `internal` y `private` expresan
la frontera visible sin decorar cada declaracion.

`public` produce un simbolo global exportable. `internal` conserva enlace
entre unidades del paquete y se materializa como `hidden` en LLVM. `private`
queda limitado al contenedor y se materializa como simbolo LLVM privado y
binding local en el objeto. La visibilidad no crea wrappers ni coste runtime.

`run` es un nombre ordinario: solo es entrada si `ref main ->> run;` lo
selecciona. `to main { ... }` queda reservado para programas basicos de hasta
30 sentencias logicas. `fa:` es la ruta final material y baja a IR,
backend/machine y el formato de salida del target.

## Tipos, texto y punteros

```nc
bytes >> char* <- cast(alloc(64));
values >> int* <- cast(alloc(4 * sizeOf(int)));
next >> int* <- off(values, 1);
wr(next, 9);
joined >> str <- "manual " + "raw";
```

`T*`, `p<T>` y punteros multiples normalizan al mismo contrato tipado. Null,
desalineacion, aliasing, fuera de rango, overflow, UAF y double free compilan;
su resultado es comportamiento indefinido dependiente del target.

`str + str` es una operacion material valida. Solicita la concatenacion nativa
del backend y devuelve su puntero; el programador decide conservarlo,
sobrescribirlo, liberarlo o filtrarlo. `cat(dst, cap, a, b)` sigue disponible
cuando el almacenamiento debe ser aportado por el llamador.

## Forma y tamano

La compactacion separa responsabilidades, no fusiona varias funciones en una
linea. `stc:`, `type:`, `local:`, `fn:` y `run` se dividen cuando dejan de ser
legibles. El encabezado solo declara nombre y, si es necesario, rol material.

`nfmt --compact` conserva comentarios, secciones y visibilidad. Migra
`shutdown :: ...` a `fa:` cuando no es una funcion referenciada, `:=` y `=` a
`<-`, simplifica `@fn(run)` a `run` y mantiene imports `use <...>;`. No acepta
`state { ... }` ni `use::project`: deben escribirse directamente como `stc:` y
`use <project:...>;`.

Los parametros usan la misma proyeccion tipada `nombre >> tipo`. Los contratos
de construccion escriben `static(...)`, `default(...)`, `move(...)`,
`copy(...)` o `region(...)`; las formas historicas `static :: in(...)` y
`static :: fn(...)` solo se aceptan para migracion.

## Compatibilidad

`state` solo nombra una seccion material de primitive y un campo de metadata
basal en `.nb`. En `.n` high-level, tanto el parser como `nfmt` rechazan
`state {}` con `E_STC_CANONICAL_001`. `use::project` tampoco pertenece al
lenguaje y se rechaza con `E_USE_PROJECT_CANONICAL_001`; la unica forma de
proyecto es `use <project:...>;`. `:=` se rechaza y `=` se conserva como entrada
compatible, aunque nunca se emite. Las funciones directas `name :: in(...)`
siguen siendo compatibles, pero codigo nuevo se agrupa en `fn:`.
