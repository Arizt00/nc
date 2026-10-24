# High Level Imports / Use

Imports are explicit and checked against declared local modules.

```n
use:
    math.geometry.area;
    math.geometry.area as circleArea;
    math.geometry.*;
```

Compact project imports address sibling or child containers by project name:

```n
use stoud.project { biblioteca, usuarios };
```

Resolution uses the project graph in this order: `bridge.json` main entry,
`bridge.json` link order, `.ncc/code/*.json` metadata, then source files in
lexicographic path order. Duplicate container names are diagnosed because the
first graph winner owns the import. Cycles are retained in the project graph
for inspection; compiler module cycles still fail where a semantic module graph
requires an acyclic dependency relation.

Rules:

- Exact imports resolve to visible symbols.
- Project imports resolve to known child or sibling containers.
- Alias imports cannot collide with local symbols or duplicate imported names.
- Wildcard imports report imported symbols and are denied in secure profiles.
- Private symbols cannot be imported.
- Internal symbols can only be imported inside the same package.
- Ambiguous imports fail.
- Unknown project imports fail with an unresolved import diagnostic.

Diagnostics: `E_HIGH_USE_UNRESOLVED_001`, `E_HIGH_USE_AMBIGUOUS_001`, `E_HIGH_USE_PRIVATE_001`, `E_HIGH_USE_INTERNAL_001`, `E_HIGH_USE_ALIAS_DUP_001`, `E_HIGH_USE_WILDCARD_DENIED_001`, `E_HIGH_USE_CYCLE_001`.

## Hosted runtime prelude v1

El compilador ofrece un preludio implícito y deliberadamente pequeño para
código high-level alojado (contenedor sin contexto) y `[@runtime]`:

- string/char, terminal and time families;
- mobile, network and system-host services;
- the direct filesystem prelude operations declared by the backend catalog.

`print` and `println` are deliberately excluded: source must select them with
`use stoud.io { output };`.

No se inserta ningún `UseDecl` sintético: `Program.uses` conserva únicamente
los imports escritos por el usuario. La resolución se realiza desde el
catálogo tipado y el IR registra el destino canónico
`nC.runtime.<familia>.<función>` como enlace `Import`.

El orden es: símbolo/closure local, función del contenedor o del mismo archivo,
import explícito y, finalmente, preludio alojado. Una coincidencia de nombre y
aridad ambigua se diagnostica; no se elige una familia silenciosamente.

El preludio no está disponible en `[@]`, `[@machine]` ni `primitive[@]`, y no
autoriza cruzar sus límites. Los imports explícitos continúan aceptándose por
compatibilidad. Este contrato no implica ownership automático de handles JSON,
scheduler async ni concurrencia material.

Diagnósticos propios: `E_RUNTIME_PRELUDE_ARITY_001`,
`E_RUNTIME_PRELUDE_AMBIGUOUS_001`, `E_RUNTIME_PRELUDE_CATALOG_001`.
