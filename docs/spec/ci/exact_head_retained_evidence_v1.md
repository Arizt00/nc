# Evidencia CI exact-head retenida v1

## Regla de autoridad

Una ejecucion solo puede llamarse `exact-head` si, antes de compilar:

1. `git rev-parse HEAD` coincide byte a byte con el SHA solicitado;
2. `git status --porcelain=v1 --untracked-files=all` esta vacio;
3. se retienen el tree, padres, submodulos, host y toolchain;
4. la captura queda ligada al certificado final mediante SHA-256.

El verificador comun es `ncc-certification-v43 exact-head-evidence`.
El [build nativo V43](../../NATIVE_TOOLS_V43_COMMANDS.md) retiene Stage0 y no
requiere Python. Los adaptadores de build disponibles estan validados para
Windows x86_64 y Linux x86_64; macOS y ARM requieren su propio adaptador y
validacion antes de anunciar esa cobertura.

## Ciclo reproducible

```text
ncc-certification-v43 exact-head-evidence capture \
  --out reports/lane/exact-head.json \
  --expected-sha <commit> --lane linux-x86_64 --runner-class hosted

ncc-certification-v43 exact-head-evidence seal \
  --root reports/lane --expected-sha <commit>

ncc-certification-v43 exact-head-evidence verify \
  --root reports/lane --expected-sha <commit> \
  --require-role image --require-role disassembly \
  --require-role log --require-role certificate
```

`capture` se niega a escribir si el arbol ya esta sucio. `seal` indexa cada
archivo regular, asigna su rol, conserva longitud y SHA-256, emite
`evidence-certificate.json` y un `SHA256SUMS` portable. Antes de sellar vuelve
a comprobar HEAD, tree y limpieza; una fuente modificada durante el build
invalida la ejecucion. `verify` rechaza:

- SHA o tree de origen distintos;
- paths absolutos, traversal, barras no portables o enlaces simbolicos;
- archivos ausentes, extra, duplicados o modificados;
- roles falsos o familias de evidencia requeridas ausentes;
- indices SHA-256 incompletos;
- HMAC ausente o invalido cuando se exige.

La variable secreta opcional es `NCC_CI_EVIDENCE_HMAC`; su identificador se
publica mediante `NCC_CI_EVIDENCE_HMAC_KEY_ID`, pero la clave nunca se copia al
bundle.

## Matriz GitHub Actions

`.github/workflows/kernel-machine-exact-head-stress.yml` define cinco lanes
hosted:

- Windows 2025 x86_64;
- Ubuntu 24.04 x86_64;
- Ubuntu 24.04 ARM64;
- macOS 15 ARM64;
- macOS 15 Intel x86_64.

Cada lane retiene durante 180 dias:

- imagenes kernel/firmware y objetos;
- hashes y manifiestos;
- desensamblados generados por machine;
- logs de build, tests y ejecucion host;
- informe y certificado de resistencia;
- certificado funcional kernel;
- captura y certificado exact-head.

Un job final descarga y vuelve a verificar los cinco bundles. No confia en el
resultado del job productor. La matriz declarada no demuestra que el builder
nativo actual soporte los cinco hosts; los lanes macOS/ARM siguen necesitando
adaptacion y evidencia real.

## Runners fisicos

Los lanes WHPX, KVM, HVF y placa son opt-in. Exigen estas identidades en el
servicio del runner:

```text
NCC_PHYSICAL_RUNNER_ID
NCC_PHYSICAL_RUNNER_MODEL
NCC_PHYSICAL_RUNNER_SERIAL
```

WHPX/KVM/HVF deben ejecutar su backend real. Las placas deben tener un
manifest por arquitectura, flashear, resetear, capturar serial y terminar con
`status=certified` y `physical_board=true`. Un resultado
`validated_not_executed` hace fallar el lane fisico; nunca se promociona a
certificado.

Los lanes de hipervisor repiten 256 ciclos de certificacion del proveedor.
Cada ciclo se verifica y retiene; el resumen registra tiempo total y latencia
media. Esto separa la tormenta real de VM entry/exit de la carga determinista
del motor de referencia.

## Frontera de verdad

La existencia del workflow demuestra que el contrato es ejecutable, no que un
run remoto ya ocurrio. Solo los artefactos descargados de un run sobre un
commit limpio autorizan una afirmacion multi-host o fisica.
