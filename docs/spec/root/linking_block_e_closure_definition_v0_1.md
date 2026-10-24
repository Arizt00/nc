# nC v0.1 — Block E (linking soberano) — definition of done

## Estado
Documento de cierre formal de Block E en la línea activa.

## Propósito
Definir criterios estrictos de cierre para linking soberano y evitar regresión semántica entre parser/AST/sema/IR/docs.

## Alcance de Block E en esta línea
- precedencia canónica de resolución de `@fn(...)`.
- autoridad de contexto en referencias no cualificadas y cualificadas.
- estabilidad de imports directos y wildcard.
- consistencia entre referencia semántica y llamada cualificada de expresión.
- diagnóstico canónico con trazabilidad test/fixture.

## Criterios de cierre (DoD)

### 1. Cierre de base
- orden de resolución documentado y vigente:
  - `docs/spec/root/linking_resolution_precedence_v0_1.md`
- matriz canónica de diagnósticos publicada:
  - `docs/spec/root/linking_diagnostics_official_matrix_v0_1.md`
- matriz modal de autoridad publicada:
  - `docs/spec/root/linking_context_authority_matrix_v0_1.md`

### 2. Cierre de superficie (parser/AST)
- parser conserva forma tipada de `FnRef` y `Expr::QualifiedCall`.
- no se introduce fallback textual para linking en rutas ya tipadas.

### 3. Cierre semántico
- resolución no cualificada con precedencia:
  - local > same-file/import visibles.
- ambigüedad no cualificada reporta listas canónicas de candidatos.
- candidaturas same-file bloqueadas (`private`/contexto) no eclipsan import único válido.
- `container.fn` de 2 segmentos prioriza namespace same-file.
- ausencia de contenedor same-file en `container.fn` produce `container '<name>' not found`.
- ruta import cualificada exige `use` visible y export válido.
- ruta import (cualificada o no cualificada) exige autoridad modal contra contexto del módulo importado.
- resolución import cualificada estable frente al orden de `use`.
- superposición `directo + wildcard` del mismo módulo no crea ambigüedad falsa.
- reglas root directas y frontera machine↔root se preservan.

### 4. Cierre IR
- linking no degrada representaciones tipadas existentes en IR.
- no se rompe continuidad de lowering root/machine/primitive ya cerrada.

### 5. Cierre de pruebas
- suite `linking_sema_tests` cubre:
  - no cualificada: éxito, ausencia, ambigüedad, contexto, visibilidad.
  - same-file cualificada: éxito, missing, private, contexto.
  - import cualificada: éxito, missing function, missing visible import.
  - root directa: contexto inválido, símbolo inexistente, símbolo físico no linked.
  - wildcard: exposición única, superposición estable, ambigüedad multi-módulo ordenada.
  - llamada cualificada de expresión con matriz equivalente.

### 6. Cierre documental
- `STATUS` actualizado con cierre de Block E.
- `HANDOFF` actualizado con reglas no rompibles y artefactos de referencia.
- `ROADMAP_CODEX_MASTER` actualizado de “iniciado” a “cerrado en esta línea”.

## Invariantes no negociables
- no mezclar camino tipado con fallback stringly en linking donde ya hay estructura.
- no degradar diagnóstico canónico a mensajes vagos.
- no permitir que orden de `use` altere resultado semántico.
- no romper frontera root/machine ya estabilizada.

## Checklist operativo (debe pasar en verde)
1. `cargo test -p ncc-sema --lib -- --nocapture`
2. `cargo test --workspace -- --nocapture`

## Fuera de alcance (siguiente fase)
- expansión del grafo modal profundo con runtime/system beyond Block E.
- reglas futuras de linking para capas superiores no activas en esta línea.
