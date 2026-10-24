# nC Toolchain Compatibility Spec v1.0.1

Fecha: 2026-05-02  
Estado: `frozen`  
Serie aplicable: `1.x`

## 1. Objetivo

Fijar reglas de compatibilidad para el modelo de toolchains congelado en `v1.0.1`.

Regla principal:

- No se permiten cambios breaking del modelo de toolchains en `1.x`.
- Cualquier ruptura requiere RFC y salto de `MAJOR`.

## 2. Superficie congelada

Se congela en `1.x`:

- canales `stable`, `beta`, `nightly`;
- formato de id `<channel>-<target>`;
- matriz target-tier del modelo oficial;
- componentes oficiales (`compiler`, `runtime`, `stdlib`, `fmt`, `lint`, `lsp`, `docs`, `sources`);
- perfiles `minimal`, `default`, `complete`.

## 3. Qué se considera breaking

Breaking en `1.x`:

- renombrar o eliminar canales oficiales;
- cambiar formato de id de toolchain;
- eliminar targets canónicos sin migración de major;
- eliminar componentes del set oficial;
- alterar composición de perfiles de instalación de forma incompatible;
- cambiar semántica de `toolchain-model` incompatible con contrato JSON.

## 4. Cambios permitidos en 1.x

Permitido en `1.x` si es compatible:

- añadir targets nuevos sin romper ids existentes;
- mejoras internas de implementación sin cambio de contrato;
- documentación adicional sin contradicción de base;
- expansión aditiva compatible en CLI.

## 5. Gobernanza y RFC

No se acepta cambio potencialmente breaking sin RFC aprobado.

Ruta obligatoria:

1. abrir RFC usando `docs/spec/release/rfc/RFC_TEMPLATE.md`;
2. clasificar impacto semver;
3. adjuntar plan de migración;
4. actualizar gates/contratos;
5. aprobar RFC antes de merge.

## 6. Contrato machine-readable

Contrato vinculante:

- `docs/spec/release/v1_0_1_toolchain_model_contract.json`

Claves que deben permanecer inmutables en `1.x`:

- `state = frozen`
- `series = 1.x`
- `channels`
- `components`
- `install_profiles`
- `required_gates` incluyendo `release_phase2_toolchain_model_gate_tests`

## 7. Criterio de cumplimiento Fase 2

Fase 2 está en cumplimiento si:

1. el gate `release_phase2_toolchain_model_gate_tests` pasa en verde;
2. no existe cambio breaking abierto sin RFC;
3. la CLI y el contrato JSON devuelven el mismo modelo canónico.
