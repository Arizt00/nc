# Machine x86 virtualization hardware evidence v1

Estado: certificado CPU-backed en Intel VMX anidado y preparado para runners Intel
y AMD. La certificacion bare-metal permanece separada y nunca se infiere de KVM.

## Comandos

```text
ncc machine virtualization probe-host --json
ncc machine virtualization certify-host --out certificate.json
ncc machine virtualization verify-certificate certificate.json --json
```

`certify-host` solo esta disponible en un binario Linux x86_64 con `/dev/kvm`.
Abre KVM directamente, crea VM/vCPU, registra 16 KiB de RAM con dirty logging,
ejecuta el invitado y rechaza cualquier salida o marcador no esperado.

Cada `KVM_RUN` queda protegido por un deadline monotono global de 15 000 ms. Un
watchdog POSIX interrumpe el ioctl bloqueado con una senal no reiniciable, restaura
handler y mascara al terminar y emite `E_MACHINE_KVM_TIMEOUT_001`. El certificado
retiene deadline, duracion observada y activacion de esta proteccion; el verificador
los exige y rechaza una duracion superior al limite.

## Contenido certificado

- CPUID vendor, brand, family, model, stepping, VMX/SVM e hipervisor;
- kernel, nesting y existencia de `/dev/kvm`;
- SHA-256 del codigo invitado y sus handlers;
- transcript ordenado de cada VM-exit;
- evento externo inyectado;
- dirty pages y valor material escrito en RAM;
- lecturas/escrituras de GPA no mapeada;
- causa final del bucle tipado;
- deadline fisico y capacidad de interrumpir un `KVM_RUN` bloqueado;
- limites de verdad del entorno.

El payload se serializa de forma canonica, se separa por dominio y se atestigua
con SHA-256. El verificador elimina `attestation`, recalcula el payload y exige que
todas las evidencias materiales sean `true`. Cualquier mutacion invalida el hash.

## Evidencia retenida 2026-07-20

Host:

```text
CPU: Intel(R) Core(TM) Ultra 9 275HX
CPUID vendor: GenuineIntel
VMX: true
hypervisor: Microsoft Hv
WSL kernel: 6.6.87.2-microsoft-standard-WSL2
nested Intel: Y
KVM API: 12
```

Resultado:

```text
status: cpu_backed_hardware_passed
typed loop: true
stop: Halted
VM-exits: 10
events injected: 1 (external vector 0x20)
markers: SEXIMO
dirty pages: 3
RAM value: 4660 (0x1234)
MMIO GPA: 0x8000, write 0x5a, read 0xa5
deadline: 15000 ms, elapsed: 2 ms
payload SHA-256: a984b0c2efe10b91382d6e74200c99a2cf258ce61087f34169af62d757654565
file SHA-256: c65108e5e658eba8b414be532febd48ac2928f4f783b0645b87ba2dda14c7cb3
```

Archivo: `reports/machine/virtualization/kvm-intel-wsl-certificate.json`.

## Limites de verdad

Esta evidencia prueba ejecucion sobre CPU con aceleracion KVM, virtualizacion
anidada, memoria invitada, VM-exits e inyeccion. KVM controla el VMCS/VMCB del
host. Los bytes y layouts VMCS/VMCB de ncc se certifican por separado mediante el
roundtrip retenido.

No se declara:

- placa bare-metal;
- ejecucion SVM fisica en una CPU AMD, porque este host es Intel;
- que QEMU-TCG sea hardware;
- ejecucion de instrucciones privilegiadas desde userland.

Para AMD se ejecuta el mismo comando en un runner `AuthenticAMD` con SVM/NPT y
`/dev/kvm`. Para bare metal se usa `machine certify-board` con identidad, serial,
flash/reset/captura y certificado de placa.
