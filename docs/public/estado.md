# Estado de nC

Actualizado el 24 de octubre de 2026. Referencia técnica V81.

nC está en bootstrap nativo. La distribución inicial permite probar un compilador nativo parcial en Windows y Linux/WSL x86-64. El Stage0 retenido sigue siendo necesario para otras rutas de desarrollo.

## Evidencia retenida

El dossier V81 documenta reconstrucciones B2/C2 con hashes idénticos por anfitrión, 74 unidades por reconstrucción, 147 programas de corpus y 38 contratos runtime por generación y sistema. La nueva materialización de alineaciones cubre potencias de dos entre 32 bytes y 1 MiB en el perfil indicado. COFF limita la alineación de sección a 8192 bytes.

Consulta [dossier](../NCC_MODULES_V81_DOSSIER.md), [reproducción](../NCC_MODULES_V81_REPRO.md) y [ABI](../NCC_MODULES_V81_ABI_LAYOUT.md). La evidencia corresponde a generaciones identificadas; no es una certificación automática de cualquier commit posterior.

## Límites de la entrega

- El kit nativo ofrece `self-test`, `compiler sema`, `compiler emit-ir`, `compiler emit-object` y `compiler emit-executable` en los recorridos probados.
- `--help`, `compiler --help`, `check` y `machine` no tienen la misma disponibilidad que en Stage0; pueden devolver `E_BOOTSTRAP_STAGE0_REQUIRED_001` y salida 69.
- Linux requiere su proveedor compartido. Los programas generados del tutorial necesitan `LD_LIBRARY_PATH` al directorio que lo contiene.
- La semántica de memoria es manual/raw; compilar no demuestra seguridad de punteros ni añade limpieza automática.
- Stage3 global permanece abierto. La entrega no reemplaza la toolchain instalada ni certifica ejecución física de otras ISAs.
- No se publican comparaciones de rendimiento con otros lenguajes: los tiempos del dossier son observaciones de construcción.

Los [smoke tests de la publicación](validacion.md) se registran por separado. Toda ampliación de soporte necesita su caso, entorno, artefactos y resultado reproducible.
