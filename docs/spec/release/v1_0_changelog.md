# nC v1.0.0 — Changelog oficial

Fecha: 2026-04-30

## Added

- cierre operativo de modos `[@security]`, `[@network]`, `[@system]`, `[@tooling]`, `[@compiler]`, `[@kernel]`, `[@ai]`;
- consolidación backend material defendible (`emit-elf`/`emit-coff`/`emit-macho`/`build-exe`);
- gates de release F21/F22/F23/F24 en `ncc-cli`;
- tooling oficial (`nfmt`, `nlint`, `nbuild`, `npkg`, `nide`) integrado en flujo estándar;
- cierre transversal v1.0.1 para `[@]`, `[@machine]`, `primitive[@]` e IDE.

## Changed

- matriz oficial de aceptación actualizada con F24 en estado `SI`;
- roadmap/master sync y acta binaria alineados al cierre total v1.0.0;
- contrato machine-readable de compatibilidad actualizado con release completamente cerrada.

## Fixed

- desambiguación parser de `>>>` entre cierre genérico anidado y operador shift;
- `nide diagnostics` con fase y posición canónica (`line`/`column`) en errores estructurados.

