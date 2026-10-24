# Registro comun de diagnosticos v1

## Autoridad

`diagnostics/registry.json` es la unica autoridad para codigo, fase, resumen y ayuda. Su esquema es `nc.diagnostic.registry.v1`. La implementacion material nC reside en `runtime/nc/diagnostic_registry.n`.

`ncc-diagnostics/build.rs` genera el catalogo Rust de stage0. `ncc-ast/build.rs` genera desde el mismo archivo la parte de la especificacion del lenguaje. Ningun consumidor conserva un segundo catalogo.

## Productores

Lexer y parser convierten su diagnostico heredado mediante `Diagnostic::canonical`. Sema expone `analyze_canonical`. MCT y machine construyen diagnosticos registrados. NIDE consume esas fronteras y publica esquema, version y numero de entradas en su informe.

Los mensajes dinamicos de stage0 pueden conservar el detalle encontrado. La identidad, fase por defecto y ayuda proceden del registro. Una fase explicita incluida en el diagnostico puede precisar una subfase.

## Gobierno

`diagnostics/rust-stage0-allowlist.json` contiene limites decrecientes para los sitios heredados. La prueba `registry_governance.rs` falla si aparece un codigo desconocido, si crecen los codigos literales o si crecen los mensajes diagnosticos definidos directamente en Rust.

Para introducir un diagnostico se modifica primero el registro y despues se usa la constante generada `ncc_diagnostics::codes::*`. El script de bootstrap solo reconstruye el registro con `-Force`; no forma parte del flujo normal de desarrollo.

## Transporte

La identidad comun se conserva en terminal, JSONL, NDIAG y el registro machine de ancho fijo. La presentacion puede variar; codigo, fase, severidad, span, expected, found y help mantienen el mismo contrato.
