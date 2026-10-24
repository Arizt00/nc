# Kernel compact raw surface

Fecha: 2026-07-18

## Regla

Kernel se escribe en `.n` normal. La forma oficial corta admite funciones directas y
una entrada declarativa:

```n
driverKernel {
    run :: in() -> bool {
        port >> addr <- 0x71000;
        value >> int <- mmio_read32(port);
        return mmio_write32(port, value);
    }

    ref main ->> run;

    fa:
        return true;
}
```

No requiere `[@kernel]` ni imports hosted. Funciones, tipos, declaraciones y
entrada usan la misma forma compacta canonica que el resto de `.n`; la sintaxis
anterior permanece como entrada compatible y baja al mismo AST.

## Prelude raw kernel

Sin imports se resuelven estas familias inequivocas:

- `atomic32_*` y `atomic64_*`;
- `atomic_fence_acquire`, `atomic_fence_release`, `atomic_fence_seq_cst`;
- `spin_try_lock`, `spin_lock`, `spin_unlock`;
- `mmio_read8/16/32/64` y `mmio_write8/16/32/64` escalares;
- `mmio_read128/256/512(mmio, destino)` y
  `mmio_write128/256/512(mmio, origen)` como transferencias raw de buffer;
- `cpu_relax`, barreras, control de interrupciones y contador de ciclos;
- `memcpy`, `memset`, `memmove` y `memcmp` en perfil freestanding.

Los aliases ambiguos del contrato operacional siguen requiriendo contexto suficiente;
el compilador no adivina si `open`, `next` o `panic` nombran una operacion kernel o una
funcion del programa.

## Raw y estricto

La ruta nativa predeterminada materializa las operaciones peligrosas solicitadas sin
comprobar vida, limites o generaciones de punteros. El programador puede provocar
overflow, acceso fuera de rango, UAF o double free y asumir sus consecuencias reales.

`--strict-source` activa de forma explicita el analisis preventivo. Incluso en raw se
mantienen lexer, parser, tipos estructurales, ABI, relocaciones y formato de objeto:
son requisitos de codificacion, no protecciones de memoria.

## Tooling

`nfmt --compact` elimina envoltorios verbosos y convierte
`ref main ->> run;` o `main: run;` en `ref main ->> run;`. Los perfiles de paquete son identificadores
abiertos; `manual`, `raw`, `kernel`, `release_fast` y perfiles privados se conservan
como metadatos aceptados.
