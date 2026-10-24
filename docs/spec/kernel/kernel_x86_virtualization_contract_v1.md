# Kernel x86 virtualization contract v1

Estado: cerrado para lowering `.n` normal a wrappers x86_64 reales, sin imports,
heap, scheduler ni inicializacion oculta.

## Superficie compacta

| Llamada | ABI material | Operacion |
|---|---|---|
| `vmx_on(slotAddress)` | `addr -> bool` | `VMXON m64` |
| `vmx_off()` | `() -> bool` | `VMXOFF` |
| `vmcs_clear(slotAddress)` | `addr -> bool` | `VMCLEAR m64` |
| `vmcs_load(slotAddress)` | `addr -> bool` | `VMPTRLD m64` |
| `vmcs_store(destination)` | `addr -> bool` | `VMPTRST m64` |
| `vmcs_read(field, destination)` | `(int, addr) -> bool` | `VMREAD m64, r64` |
| `vmcs_write(field, value)` | `(int, int) -> bool` | `VMWRITE r64, r64` |
| `vm_invalidate_ept(kind, descriptor)` | `(int, addr) -> bool` | `INVEPT r64, m128` |
| `vm_invalidate_vpid(kind, descriptor)` | `(int, addr) -> bool` | `INVVPID r64, m128` |
| `vm_call()` | `() -> bool` | `VMCALL` |
| `vm_launch()` | `() -> bool` | `VMLAUNCH` |
| `vm_resume()` | `() -> bool` | `VMRESUME` |
| `svm_run(vmcbPhysical)` | `addr -> bool` | `RAX` + `VMRUN` |
| `svm_call()` | `() -> bool` | `VMMCALL` |
| `svm_load(vmcbPhysical)` | `addr -> bool` | `RAX` + `VMLOAD` |
| `svm_save(vmcbPhysical)` | `addr -> bool` | `RAX` + `VMSAVE` |
| `svm_clgi()` / `svm_stgi()` | `() -> bool` | `CLGI` / `STGI` |
| `svm_invlpga(address, asid)` | `(addr, int) -> bool` | `RAX`, `RCX`, `INVLPGA` |

Aliases sin underscore y nombres cualificados `kernel.virtualization.intel.*` o
`kernel.virtualization.amd.*` resuelven al mismo simbolo ABI.

## Semantica cruda

- Las direcciones son responsabilidad del programador.
- `slotAddress` apunta a una celda m64 que contiene la direccion fisica VMXON/VMCS.
- `destination` en `vmcs_read` y `vmcs_store` debe ser memoria escribible.
- El descriptor INVEPT/INVVPID debe tener 16 bytes y alineacion adecuada.
- `vmcbPhysical` se carga en `RAX` exactamente como exige SVM.
- No hay allocation, zeroing, pinning, traduccion virtual-fisica ni locks ocultos.
- No hay checks de ownership, lifetime, aliasing o rango.

Los wrappers VMX capturan `RFLAGS` inmediatamente despues de la instruccion. El
retorno es `true` solo cuando `(RFLAGS & (CF | ZF)) == 0`; por tanto distingue
VMsuccess de VMfailInvalid y VMfailValid. `VMPTRST`, `VMRUN`, `VMLOAD` y `VMSAVE`
devuelven `true` si la instruccion retorna; sus fallos arquitectonicos son faults o
VMEXIT, no un estado CF/ZF.

`vm_call()` y `svm_call()` tienen cuerpos LLVM materiales y exportables. El objeto
retenido desensambla exactamente `0f 01 c1` (`VMCALL`) y `0f 01 d9` (`VMMCALL`),
seguidos del retorno booleano. El fixture invoca ambos para impedir que optimizacion
o enlace oculten una declaracion sin cuerpo; no se presenta ese fixture mixto como
una carga arrancable sobre un unico proveedor.

## Precondiciones externas

Antes de ejecutar Intel VMX, el kernel debe comprobar CPUID, IA32_FEATURE_CONTROL,
CR4.VMXE, fixed bits de CR0/CR4, IA32_VMX_BASIC, revision, size, memory type y las
capabilities de controles/EPT/VPID. Antes de AMD SVM debe comprobar CPUID SVM,
VM_CR, EFER.SVME, ASID y features del leaf `0x8000000A`.

Estas comprobaciones dependen de MSR/control registers y solo pueden certificarse
en ring 0. `machine virtualization --json` informa lo visible por CPUID en el host,
pero no declara que firmware/MSR permitan entrar en VMX/SVM.

En targets no x86_64 los simbolos conservan firma para diagnostico de lowering y
terminan en `llvm.trap`; una aplicacion no puede confundirlos con virtualizacion ARM
o RISC-V.

## Build

```text
ncc build examples/11_machine/kernel_x86_virtualization_vmcs_vmcb.n \
  --profile kernel --target x86_64-unknown-none --out-dir target/kernel-vmx
```

La salida incluye `.nm`, objeto machine, LLVM del cuerpo, objeto nativo, ELF64,
mapa, manifest e imagen boot.

Memoria fisica invitada, EPT/NPT, snapshots, reinyeccion y el bucle completo se
definen en `machine_x86_guest_memory_ept_npt_v1.md` y
`kernel_x86_vmexit_exception_interrupt_v1.md`.
