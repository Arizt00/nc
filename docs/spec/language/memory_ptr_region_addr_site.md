# Memory ptr/region/addr/site

## Modelo
- `ptr<T>` con origen,
- `memRegion<T>` con bounds/owner,
- `addr` materializado,
- `site` para trazabilidad.

## Reglas
No use-after-free, no stale-site silencioso, no ptr sin origen.
