# Certificacion SMP en placas fisicas

Los JSON de esta carpeta son plantillas generadas por `ncc machine board-template`.
Los campos `EDIT_ME` son intencionados: impiden que una plantilla se presente como
evidencia de una placa inexistente.

## Preparacion

1. Construir el ejemplo para el target de la placa.
2. Copiar la plantilla a un manifest de laboratorio no compartido.
3. Rellenar vendor, modelo, revision, serial de placa y serial de probe.
4. Fijar `artifact.path` al medio generado y conservar `sha256: auto`.
5. Sustituir los argv de probe/flash/reset/capture/framebuffer por herramientas reales.
6. Declarar resolucion, stride y formato de pixel de la salida fisica.
7. Ajustar marcadores UART a los que emite el kernel de certificacion.

Primero se valida sin tocar hardware:

```powershell
ncc machine certify-board --manifest board.json --bundle .ncc/board-validation --validate-only --json
ncc machine verify-board-certificate .ncc/board-validation --json
```

Despues, con la placa conectada:

```powershell
ncc machine certify-board --manifest board.json --bundle .ncc/board-certification --json
ncc machine verify-board-certificate .ncc/board-certification --json
```

La herramienta usa argv estructurado, timeout, hashes, captura UART, framebuffer
binario de tamano exacto y marcadores ordenados. Rechaza transportes o comandos que
contengan emuladores. Solo la segunda ruta puede producir estado `certified`.
