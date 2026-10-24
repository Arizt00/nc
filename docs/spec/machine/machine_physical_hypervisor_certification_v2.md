# Certificacion fisica de hipervisor v2

## Autoridad

Un certificado fisico solo es valido cuando el invitado se ejecuta mediante la
API del hipervisor del host y no mediante TCG ni un interprete. El certificado
registra arquitectura, fabricante, extension, revision fuente, duraciones,
salidas y limite de verdad.

Hay tres estados distintos:

- `implemented`: parser, machine, kernel, objetos y verificadores existen;
- `cpu_backed_nested`: ejecucion real en CPU bajo otro hipervisor;
- `bare_metal_certified`: el host no anuncia hipervisor y la evidencia procede
  de la revision exacta.

`cpu_backed_nested` nunca se eleva automaticamente a `bare_metal_certified`.

## Contrato material

`certify-host` ejecuta directamente `/dev/kvm`, sin proceso QEMU. En x86 exige:

1. entrada y salida material del invitado;
2. excepcion `#UD` y continuacion por `IRET`;
3. interrupcion externa vector `0x20`;
4. lectura y escritura de GPA no mapeada por la traduccion de segundo nivel;
5. escritura de RAM y dirty bitmap;
6. vCPU bloqueada en un bucle infinito, cancelada por deadline real;
7. reentrada de la misma vCPU despues de `EINTR` y parada en `HLT`;
8. dos vCPU, LAPIC/PIT, MSI e IOAPIC con destino AP verificable.

En AArch64 y RISC-V el binario nativo del host ejecuta el guest mediante KVM y
retiene MMIO, dirty pages, estado final y deadline. Los runners fisicos deben
identificarse respectivamente como EL2 y H.

## Matriz obligatoria

| Lane | Etiqueta del runner | Evidencia |
|---|---|---|
| Intel VMX | `ncc-kvm-vmx` | CPUID Intel+VMX y certificado KVM rico |
| AMD SVM | `ncc-kvm-svm` | CPUID AMD+SVM y certificado KVM rico |
| AArch64 EL2 | `ncc-kvm-el2` | KVM ARM64 sobre host fisico |
| RISC-V H | `ncc-kvm-h` | KVM RISC-V64 sobre host fisico |

El workflow exige `--require-bare-metal`, `exact_head_match=true`, hashes de
todos los artefactos y retencion independiente.

## Comandos fuera de QEMU

```text
ncc machine virtualization probe-host --json
ncc machine virtualization certify-host --out hypervisor.json
ncc machine virtualization verify-certificate hypervisor.json --require-bare-metal --json
```

Para diagnostico anidado se omite `--require-bare-metal`; el certificado sigue
indicando `bare_metal_board=false` y no sirve para cerrar una lane fisica.

## Limite privilegiado

VMCS, VMCB, EL2 y H pertenecen al kernel o hipervisor anfitrion. Un proceso de
usuario no ejecuta directamente `VMXON`, `VMRUN`, `ERET` o `HRET`; KVM, WHPX y
HVF son los propietarios materiales de ese estado. Los opcodes y layouts nC se
certifican por separado en machine y en imagen kernel.
