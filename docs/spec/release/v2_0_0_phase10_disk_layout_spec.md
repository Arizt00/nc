# nC Phase10 Disk Layout Spec v2.0.0

Estado: `frozen`  
Fase: `phase10_p4_disk_layout`

## Objetivo

Cerrar estructura canónica de `ncup` en disco para serie `2.x`, con override por proyecto y paridad de comportamiento multi-OS.

## Base activo

### Home raíz

- Unix: `~/.ncup`
- Windows: `%LOCALAPPDATA%\\ncup`
- Override explícito permitido: `NCUP_HOME` y `ncup --home <path>`.

### Estructura canónica obligatoria

Toda instalación DEBE contener:

- `toolchains/`
- `downloads/`
- `tmp/`
- `manifests/`
- `keys/`
- `logs/`
- `bin/`

`bin/` DEBE contener shims/proxies activos del toolchain default.

### Scripts de entorno

`ncup init` DEBE materializar:

- `env.sh`
- `env.bat`

Ambos DEBEN exportar `NCUP_HOME` y agregar `<home>/bin` al `PATH`.

### Override por proyecto

El archivo `nc-toolchain.toml` DEBE permitir override efectivo de toolchain por árbol de proyecto.

Forma canónica:

```toml
[toolchain]
id = "stable-x86_64-unknown-linux-gnu"
```

Forma alternativa permitida:

```toml
[toolchain]
channel = "stable"
target = "x86_64-unknown-linux-gnu"
```

`ncup toolchain which` DEBE resolver este override automáticamente (o vía `--cwd <path>`).

## Gate oficial

- `release_phase10_disk_layout_gate_tests`

## Criterio de cierre

F10-P4 queda cerrado cuando:

1. contrato y spec están en `frozen`;
2. layout canónico se materializa en `ncup init`;
3. `nc-toolchain.toml` resuelve override de toolchain;
4. `ncup doctor` valida layout canónico;
5. gate oficial está en verde.
