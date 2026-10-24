# nC Phase10 IDE Marketplace Runbook v2.0.0

Estado: `frozen`

## Objetivo

Publicación controlada de artefactos IDE:

- JetBrains Marketplace
- VSCode Marketplace
- Distribución oficial Ghidra (official-site/GitHub Releases)

## Flujo canónico

## 1) Generar bundles IDE

```bash
bash tools/release/build_phase10_ide_distribution_v2_0_0.sh
```

## 2) Plan seco obligatorio

```bash
bash tools/release/publish_phase10_ide_marketplaces_v2_0_0.sh \
  --mode dry-run \
  --allow-partial
```

## 3) Render de template seguro

```bash
bash tools/release/render_phase10_ide_marketplace_env_v2_0_0.sh
```

## 4) Completar secretos y ejecutar

```bash
source docs/status/F10_P20_ide_marketplace_env_template.sh
```

## Reglas de bloqueo

1. `status=error` BLOQUEA publicación.
2. `status=partial` solo se acepta si el canal objetivo no requiere todos los marketplaces.
3. Prohibido publicar sin evidencias en `docs/status/F10_P20_ide_marketplace_publish_report.json`.

## Seguridad

1. Tokens NO DEBEN guardarse en repositorio.
2. Publicación DEBE ejecutarse en runners limpios.
3. Artefactos DEBEN provenir de build/release trazable.
