# Backends portables de hipervisor v1

## Alcance

La autoridad portable vive en `ncc-machine::hypervisor_backend`. Define una ABI sin
dependencias del sistema operativo para:

- descubrir capacidades;
- crear y destruir una partición;
- registrar, mapear y desmapear memoria invitada;
- crear, ejecutar, cancelar, pausar, reanudar y reiniciar vCPU;
- leer y escribir registros;
- inyectar interrupciones;
- obtener bitmap de páginas sucias;
- normalizar exits de memoria, I/O, MMIO, CPUID, MSR, hypercall, excepción,
  depuración, halt, shutdown, EL2 y RISC-V H.

La versión de la ABI es `HYPERVISOR_ABI_VERSION = 1`. La granularidad portable
mínima es 4096 bytes. Los estados de VM y vCPU son explícitos; una transición no
válida se rechaza antes de llamar al proveedor.

## Proveedores

### WHPX / Hyper-V

La implementación Windows llama directamente a Windows Hypervisor Platform:

1. comprueba `WHvCapabilityCodeHypervisorPresent`;
2. crea y configura la partición;
3. mapea GPA con permisos materiales;
4. crea el virtual processor y fija su estado inicial;
5. ejecuta `WHvRunVirtualProcessor` con deadline y cancelación;
6. consume exits de I/O y halt;
7. valida la escritura del invitado en RAM;
8. desmonta vCPU, GPA y partición mediante RAII.

No ejecuta VMX/SVM desde userland. Hyper-V conserva el estado privilegiado.

### KVM

La implementación Linux abre `/dev/kvm`, comprueba la versión de API y crea VM,
slots de memoria y vCPU. Hay rutas nativas para x86_64, AArch64 y RISC-V64.
Cada una instala el estado arquitectónico correspondiente, ejecuta `KVM_RUN`,
normaliza exits y aplica un watchdog capaz de interrumpir una entrada bloqueada.

La certificación exige `guest_code_executed`, `final_halt`, deadline y teardown.
QEMU TCG no satisface este contrato.

### Hypervisor.framework

En macOS, `ncc` delega la ejecución privilegiada en `ncc-hvf-smoke`, enlazado con
Hypervisor.framework. El helper crea VM/vCPU, mapea memoria, fija registros,
ejecuta el invitado y devuelve evidencia JSON. Debe firmarse con el entitlement
`com.apple.security.hypervisor`; el workflow conserva también los entitlements
extraídos del binario firmado.

## Certificados

```text
ncc machine virtualization probe-backends --json
ncc machine virtualization certify-backend --provider whpx|kvm|hvf --out cert.json
ncc machine virtualization verify-backend-certificate cert.json
```

El certificado contiene host, proveedor, tiempos, exits, verdad material, revisión
Git y SHA-256 canónico. En GitHub Actions, `github_sha`, `git_head`, árbol limpio y
`exact_head_match` deben coincidir. Una sonda de disponibilidad nunca se acepta
como certificado de ejecución.

## Matriz física

El workflow `machine-portable-hypervisor-iommu-physical.yml` separa:

- Windows x86_64 con etiqueta `ncc-whpx`;
- Linux x86_64 con `ncc-kvm`;
- Linux AArch64/EL2 con `ncc-kvm-el2`;
- Linux RISC-V64/H con `ncc-kvm-h`;
- macOS ARM64/EL2 con `ncc-hvf-el2`;
- macOS Intel con `ncc-hvf`.

Los jobs físicos son `self-hosted` y opt-in. Un runner ausente produce una celda
sin ejecutar, nunca evidencia simulada.

## Autoridades externas

- [Windows Hypervisor Platform](https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/hypervisor-platform)
- [Linux KVM API](https://www.kernel.org/doc/html/latest/virt/kvm/api.html)
- [Apple Hypervisor framework](https://developer.apple.com/documentation/hypervisor)
