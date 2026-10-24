# nC v2.0.0 External Publication Runbook

Estado: `frozen`

## Propósito

Runbook operativo para publicar artefactos nC en canales externos sin romper contratos de Fase 10.

## Precondiciones

1. `tools/release/qa_phase10_v2_0_0.sh` en verde.
2. `docs/status/F10_P19_final_closure_report.json` en `ok`.
3. Artefactos firmados disponibles en `var/releases/v2_0_0/phase10_p3_dist`.
4. Credenciales de publicación activas por canal.

## Flujo canónico

## 0) Preparar artefactos multi-OS (obligatorio para `all`)

```bash
bash tools/release/build_phase10_distribution_os_v2_0_0.sh \
  --os all \
  --allow-deferred
```

## 1) Plan seco por runner (obligatorio)

```bash
bash tools/release/run_phase10_external_publish_matrix_v2_0_0.sh \
  --runner linux \
  --channel stable \
  --mode dry-run \
  --allow-partial
```

## 2) Generar template runner-specific con comandos reales

```bash
bash tools/release/render_phase10_external_publish_env_v2_0_0.sh \
  --channel stable \
  --runner linux \
  --out docs/status/F10_external_publish_env_template.sh
```

Opciones de runner:

- `--runner linux`   -> `deb,rpm`
- `--runner windows` -> `winget`
- `--runner macos`   -> `brew`
- `--runner all`     -> todos los adapters

## 3) Ajustar variables de entorno y ejecutar

```bash
source docs/status/F10_external_publish_env_template.sh
```

El template ya incluye comandos reales de publicación. Solo se ajustan rutas/credenciales (`NC_DEB_REPO_BASE`, `NC_RPM_REPO_DIR`, `NC_BREW_TAP_REPO`, etc.).

## 4) Evidencia y bloqueo

1. Guardar `docs/status/F10_external_publish_report.json` como evidencia.
2. Si `status=error`, la promoción de canal DEBE bloquearse.
3. Si `status=partial`, solo PUEDE continuarse cuando los canales faltantes NO formen parte del SLA objetivo.

## 5) Matriz canónica por sistema operativo

1. Runner Linux DEBE publicar `deb,rpm`.
2. Runner Windows DEBE publicar `winget`.
3. Runner macOS DEBE publicar `brew`.
4. Para smoke integral en un solo host (sin publicar externamente), PUEDE usarse `--runner all` con adapters locales.

Ejemplos:

```bash
# Linux
bash tools/release/run_phase10_external_publish_matrix_v2_0_0.sh \
  --runner linux --channel stable --mode execute

# Windows
bash tools/release/run_phase10_external_publish_matrix_v2_0_0.sh \
  --runner windows --channel stable --mode execute

# macOS
bash tools/release/run_phase10_external_publish_matrix_v2_0_0.sh \
  --runner macos --channel stable --mode execute
```

## Regla de seguridad

1. Prohibido publicar bins/manifests sin bundle de integridad.
2. Prohibido inyectar tokens en scripts versionados.
3. Toda publicación DEBE ser trazable por reporte machine-readable.
