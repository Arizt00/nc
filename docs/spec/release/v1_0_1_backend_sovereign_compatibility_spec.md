# nC Backend Sovereign Compatibility Spec v1.0.1

Estado: `frozen`  
Serie: `1.x`

## Objetivo

Fijar reglas de compatibilidad para backend soberano y contratos ABI/lowering congelados en Fase 4.

## Regla principal

No se permiten cambios breaking en `1.x` sobre:

- mapping calling convention -> ABI;
- data layout canónico de targets congelados;
- object formats oficiales por perfil;
- matriz tiered de targets activos;
- reglas de lowering y legalización congeladas para carriles activos.

## Qué es breaking

Se considera breaking (prohibido sin RFC):

- cambiar triple o tier de perfiles canónicos sin RFC;
- cambiar object format de un perfil canónico;
- cambiar calling convention canónica de `nC.sysv64/nC.win64/nC.aapcs64/nC.portable/nC.sovereign`;
- degradar diagnóstico canónico de incompatibilidad ABI/artefacto;
- romper reproducibilidad de emisiones bajo mismo input y perfil.

## Qué es aditivo permitido

Se permite (sin romper `1.x`):

- agregar targets nuevos fuera de matriz congelada;
- agregar lanes experimentales no canónicos;
- agregar metadata de backend-model sin romper campos existentes.

## Gobernanza

- `no_breaking_without_rfc = true`
- RFC obligatorio para cualquier cambio en contrato backend congelado.

Referencia:

- `docs/spec/release/rfc/RFC_TEMPLATE.md`

## Contrato machine-readable

- `docs/spec/release/v1_0_1_backend_sovereign_contract.json`

## Gate obligatorio

- `release_phase4_backend_sovereign_gate_tests`
