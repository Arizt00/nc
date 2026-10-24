# nC Phase10 Distribution by OS Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Compatibilidad activa

- La familia de bins canónicos (`ncc,nfmt,nlint,nbuild,npkg,nide,nclsp,nwal,ncup,ncup-init`) DEBE mantenerse.
- El layout lógico `nc.toolchain.layout.v1` DEBE mantenerse en todos los empaquetados.
- `winget`, `brew tap` y scripts de instalación NO DEBEN alterar contratos semánticos del compilador.

## Reglas

1. Cambios de layout lógico requieren RFC.
2. Cambios de nombres de binarios requieren RFC.
3. Nuevos empaquetados por OS PUEDEN añadirse sin romper los existentes.
4. Formatos native-runner (MSI/pkg) pueden publicarse como `deferred` fuera de runner nativo, pero NO en release matrix final.
