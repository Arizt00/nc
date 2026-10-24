# nC Imports Operational Compatibility Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`

## Política de compatibilidad

Hasta contradicción formal posterior:

1. NO se permite romper nombres de módulos/imports canónicos sin RFC.
2. NO se permite romper firmas públicas (aridad/tipo retorno) sin RFC.
3. NO se permite romper alias runtime (`nC.runtime.heap/net/trace/alias/time`) sin RFC.
4. NO se permite romper wildcard canónico `use nC.runtime.*;` para familias activas sin RFC.

## Cambios permitidos sin breaking

1. Añadir nuevas familias o funciones compatibles.
2. Añadir nuevos diagnósticos complementarios sin alterar los diagnósticos canónicos existentes.
3. Mejorar observabilidad/reportes sin modificar contrato funcional.

## Cambios que REQUIEREN RFC (breaking)

1. Renombrar módulo o función de import canónica.
2. Eliminar alias activo documentado.
3. Cambiar aridad mínima/variádica de función canónica.
4. Cambiar tipo de retorno canónico.
5. Alterar reglas de autoridad modal para imports activos sin migración formal.
