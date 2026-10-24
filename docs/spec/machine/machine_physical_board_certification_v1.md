# Machine physical-board certification v1

Estado normativo: activo.

## Frontera

La certificacion fisica es una ruta distinta de QEMU. Una ejecucion QEMU, TCG,
Renode, gem5 u otro emulador nunca puede producir `status=certified`.

Comandos:

```text
ncc machine board-template --target aarch64-unknown-none --out board.json
ncc machine certify-board --manifest board.json --bundle target/board-certification
ncc machine verify-board-certificate target/board-certification
```

La opcion `--out` de `certify-board` se conserva como alias de `--bundle`.

## Manifiesto

El manifiesto usa:

```json
{
  "schema": "ncc.machine.physical_board_manifest.v1",
  "manifest_version": 1
}
```

Son obligatorios:

- identidad, fabricante, modelo, revision y serial unicos de la placa;
- arquitectura y target freestanding compatibles;
- puerto fisico y serial del probe;
- artefacto no vacio con SHA-256 esperado o `auto`;
- `commands.probe`, `commands.flash` y `commands.capture`;
- marcadores de probe y marcadores de boot ordenados;
- limites de artefacto, log y timeout;
- `transport.physical=true`;
- `policy.reject_emulated_transport=true`.

Los manifiestos nuevos deben activar ademas
`policy.require_framebuffer_capture=true`, declarar `framebuffer.width`,
`height`, `stride` y `pixel_format`, y proporcionar `commands.framebuffer`.
Los formatos admitidos son RGB/BGR 565, RGB/BGR 888 y las variantes de 32
bits XRGB/ARGB/BGRA/RGBA 8888.

Cada comando contiene un unico `argv`:

```json
{
  "argv": ["openocd", "-f", "board.cfg", "-c", "program {artifact} verify reset exit"],
  "timeout_ms": 180000,
  "success_exit_codes": [0]
}
```

No se aceptan `program`, `args`, `shell` ni `command_line`. NCC usa
`std::process::Command` directamente y nunca inserta una shell implicita. `cwd`,
`env`, `inherit_env`, `allow_timeout` y los exit codes son declarativos. Los
valores de `env` no se guardan en el certificado.

El entorno heredado esta desactivado por defecto y solo se activa con
`inherit_env=true`. Los programas `.bat` y `.cmd` se rechazan porque requieren
una shell indirecta. `allow_timeout=true` solo es valido para `capture`; probe,
flash y reset deben finalizar con un codigo admitido.

El ejecutable de cada etapa se resuelve a una ruta regular y se hashea antes y
despues. Una herramienta no identificable o modificada durante la sesion puede
dejar logs de diagnostico, pero invalida cualquier certificacion.

## Estados

- `certified`: probe identificado, flash, reset opcional, captura, marcadores,
  limpieza requerida y rechazo de emulador han pasado.
- `validated_not_executed`: `--validate-only` o placa/probe no disponible. Este
  estado siempre lleva `physical_board=false`. La clase es `validation_only`
  cuando se solicito validacion y `physical_hardware_unavailable` cuando el
  probe no pudo demostrar la presencia de la placa.
- `rejected_emulator`: se encontro evidencia de emulacion.
- `certification_failed`: habia placa identificada, pero fallo una operacion,
  un limite, el orden de boot o la identidad.

Un probe ausente termina correctamente con un bundle
`validated_not_executed`; nunca se promociona a certificado.
Los bundles `rejected_emulator` y `certification_failed` siguen siendo
verificables como evidencia de un resultado fallido, siempre con
`physical_board=false`.

## Limites

El manifiesto fija `max_artifact_bytes`, `max_log_bytes` por stream y
`max_timeout_ms`. Hay techos internos adicionales. NCC drena stdout y stderr
con lectores separados, conserva como maximo el limite declarado y termina el
proceso al rebasarlo. Un truncado invalida la certificacion.

## Bundle

El directorio es nuevo o esta vacio y contiene:

```text
bundle.json
certificate.json
inputs/board-manifest.json
inputs/artifact.bin
metadata/session.json
logs/<stage>.stdout.log
logs/<stage>.stderr.log
evidence/framebuffer.raw
```

Todas las rutas son relativas y ordenadas. `bundle.json` registra rol, bytes y
SHA-256 de cada archivo. El certificado registra por separado el conjunto de
evidencias y su hash canonico. No se admiten symlinks ni archivos no indexados.

El verificador recalcula:

- conjunto exacto de archivos;
- tamanos y SHA-256;
- `bundle_id`;
- esquema, campos, orden y `file_count` del indice;
- payload canonico del certificado;
- manifiesto y artefacto;
- correspondencia manifiesto/certificado;
- correspondencia exacta de estado y ejecucion entre `session.json` y el
  certificado;
- forma cerrada, rol, orden y unicidad de cada entrada de evidencia;
- marcadores directamente desde el log de captura;
- semantica de `certified` y `validated_not_executed`;
- ausencia de evidencia de emuladores.

Cualquier cambio posterior en un input, log, metadata o certificado falla.
Cuando se exige framebuffer, el archivo debe ser regular, no symlink, y medir
exactamente `stride * height`. Resolucion, formato, bytes y SHA-256 quedan en
el certificado. Sin esa evidencia el estado fisico no puede ser `certified`.

## HMAC opcional

```text
ncc machine certify-board ... --signing-key-env NCC_BOARD_HMAC_KEY
ncc machine verify-board-certificate ... \
  --signing-key-env NCC_BOARD_HMAC_KEY --require-attestation
```

La clave debe tener al menos 32 bytes. Solo se lee desde el entorno, se elimina
del entorno de los procesos hijos y se sobrescribe en la copia temporal. El
bundle guarda el identificador de la variable y el tag HMAC-SHA-256, nunca la
clave ni su fingerprint.

El HMAC autentica la evidencia retenida y su clasificacion. No sustituye una
raiz de confianza del silicio, secure boot, TPM, DICE ni una atestacion firmada
por la propia placa.
