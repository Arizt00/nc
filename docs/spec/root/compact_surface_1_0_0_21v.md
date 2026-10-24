# Superficie compacta 1.0.0.21v

Fecha: 2026-07-21

## Regla oficial

La sintaxis compacta es la forma normal de escritura. La forma histórica puede
seguir entrando por compatibilidad, pero parser y `nfmt` la normalizan hacia un
único AST tipado. El backend nunca decide semántica a partir del spelling corto.

```n
compactDemo {
    copy :: in(source >> p<int>, destination >> p<int>) -> void {
        wr(destination, rd(source));
        return adr(destination);
    }

    ref main ->> copy;
}
```

En una función `void`, la expresión devuelta se evalúa por sus efectos y su valor
se descarta. No se crea un retorno material oculto.

## Punteros

`p<T>` es la escritura oficial de `ptr<T>`. Las formas largas permanecen como
entrada compatible y se normalizan así:

| Oficial | Compatible | Contrato interno |
|---|---|---|
| `p<T>` | `ptr<T>` | `ptr<T>` |
| `pRead<T>(p)` o `Read<T>(p)` | `ptrRead<T>(p)` | `ptrRead<T>` |
| `pWrite<T>(p, v)` o `Write<T>(p, v)` | `ptrWrite<T>(p, v)` | `ptrWrite<T>` |
| `pOffset<T>(p, n)` | `ptrOffset<T>(p, n)` | `ptrOffset<T>` |
| `pDiff<T>(a, b)` | `ptrDiff<T>(a, b)` | `ptrDiff<T>` |
| `pFromAddr<T>(a)` | `ptrFromAddr<T>(a)` | `ptrFromAddr<T>` |
| `pCast<T>(p)` | `ptrCast<T>(p)` | `ptrCast<T>` |
| `pAddr<T>(p)` | `ptrAddr<T>(p)` | `ptrAddr<T>` |
| `pRegion<T>(p, n)` | `regionFromPtr<T>(p, n)` | `regionFromPtr<T>` |

Estas operaciones son raw. No añaden bounds checks, tracking de vida, aliasing,
alineación, inicialización o liberación automática.

La comparación con `null` es válida para cualquier `p<T>` raw. Sema conserva
el tipo puntero y LLVM emite `icmp eq/ne ptr` al ancho completo del target.

## Prelude backend

Código hosted y root puede llamar sin `use` a la superficie prelude que el
compilador resuelve a símbolos canónicos:

- cadenas y caracteres: toda la familia `nC.runtime.string`;
- terminal: toda la familia `nC.runtime.terminal`;
- tiempo: toda la familia `nC.runtime.time`;
- JSON: toda la familia `nC.runtime.json`;
- individuales: `alloc`, `calloc`, `realloc`, `free`, punteros, layout,
  atomics, `print`, `println`, entrada, `sleepMs`, `monotonicMs`, `exists` y
  `listDir`.

No se inyecta texto `use`. Sema resuelve nombre y aridad; IR conserva el símbolo
runtime o intrinsic canónico y el linker materializa su implementación.

Los nombres ABI internos `ncc_rt_*` nunca son imports del sistema operativo.
`run` y `build` detectan esas referencias, enlazan el objeto runtime nativo y
solo dejan en PE/ELF/Mach-O los imports externos pertenecientes al host.

Kernel y system no reciben runtime hosted por esta regla. Solo ven intrinsics y
operaciones freestanding admitidas por su perfil.

## Salida

La superficie fuente contiene únicamente:

```n
print(value);
println(value);
println();
```

`print.*` está retirado y produce `E_PRINT_SURFACE_001`. `nfmt --compact`
convierte spellings históricos sin alterar cadenas ni comentarios.
