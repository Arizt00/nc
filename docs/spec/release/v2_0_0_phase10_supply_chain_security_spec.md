# nC Phase10 Supply Chain Security Spec v2.0.0

Estado: `frozen`  
Serie: `2.x`  
Fase: `phase10_p7_supply_chain_security`

## Objetivo

Cerrar seguridad de cadena de suministro de extremo a extremo para `ncup` y artefactos de distribución, con verificación obligatoria antes de instalar y soporte enterprise con mirror + trust key interna.

## Base activo

F10-P7 DEBE cumplir, como mínimo:

1. Todos los artefactos DEBEN tener checksum `SHA-256`.
2. Todos los manifests y bundles DEBEN tener firma criptográfica verificable.
3. `ncup-init` DEBE incluir clave raíz embebida para bootstrap de confianza.
4. `ncup` DEBE verificar integridad/firma antes de aceptar instalación saludable.
5. Debe existir modo enterprise con mirror explícito y trust key interna.

## Modelo de confianza

### Clave raíz embebida

`ncup-init` y `ncup` usan material raíz embebido:

- `root_key_id`: `ncup-root-v2-2026`
- `hash_algorithm`: `sha256`
- `signature_algorithm`: `hmac-sha256`

Durante bootstrap, `ncup init` DEBE materializar:

- `keys/root-trust-key.v1.json`

`ncup doctor` y `ncup verify` DEBEN fallar si el root trust no coincide con material embebido.

### Modo enterprise

`ncup init` DEBE aceptar:

- `--enterprise-mirror <url>`
- `--enterprise-trust-key <key>`

Debe materializar:

- `keys/enterprise-trust.key`
- `keys/enterprise-trust-key.v1.json`
- metadata `supply_chain.enterprise` en `manifests/ncup-config.v1.json`

`NCUP_ENTERPRISE_TRUST_KEY` PUEDE sobrescribir key material por entorno.

## Integridad por toolchain instalada

Cada instalación de toolchain DEBE generar y verificar:

- `integrity/SHA256SUMS`
- `integrity/manifest.v1.json`
- `integrity/manifest.v1.sig`
- `integrity/attestation.v1.json`

`ncup install`/`ncup toolchain install` DEBEN:

1. verificar política root trust;
2. resolver key efectiva (root o enterprise);
3. generar bundle de integridad;
4. verificar checksum/firma;
5. fallar si hay mismatch.

## Verificación obligatoria

Comandos mínimos:

- `ncup doctor --home <path>`
- `ncup verify --home <path> [--toolchain <id>]`

`ncup verify` DEBE detectar tampering de bins o manifests y devolver diagnóstico canónico (`checksum mismatch` o `signature mismatch`).

## Integridad en distribución

Los bundles distribuidos por F10 deben incluir:

- `integrity/SHA256SUMS`
- `integrity/SHA256SUMS.sig`
- `integrity/SBOM.ncc.v1.json`
- `integrity/attestation.v1.json`

`tools/release/verify_phase10_clean_install_v2_0_0.sh` DEBE verificar integridad de distribución antes de extraer e instalar.

## Diagnósticos mínimos

- `ncup supply-chain: missing root trust file ...`
- `ncup supply-chain: root trust material mismatch ...`
- `ncup supply-chain: missing integrity artifact ...`
- `ncup supply-chain: checksum mismatch for ...`
- `ncup supply-chain: integrity signature mismatch`
- `ncup supply-chain: enterprise trust key is missing ...`

## Gate oficial

- `release_phase10_supply_chain_security_gate_tests`

## Pipeline oficial

- `ncc-release-v43 release-supply-chain` ([build V43](../../NATIVE_TOOLS_V43_COMMANDS.md))
- `tools/release/qa_phase10_p7_v2_0_0.sh`
- `tools/release/qa_phase10_v2_0_0.sh`

## Criterio de cierre

F10-P7 solo se considera cerrado cuando:

1. spec/compat/contract están en `frozen`;
2. `ncup verify` pasa en estado sano;
3. `ncup verify` falla al detectar tampering;
4. enterprise mode queda validado con mirror + trust key;
5. clean install valida integridad de distribución antes de bootstrap;
6. gate y QA global quedan en verde.
