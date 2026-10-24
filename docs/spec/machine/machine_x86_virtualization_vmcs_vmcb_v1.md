# Machine x86 virtualization VMCS/VMCB v1

Estado: cerrado para modelado tipado, parser, sema, IR, encoder, decoder, objeto y
roundtrip. La ejecucion de las instrucciones exige ring 0 y soporte VMX o SVM.

## Autoridad tipada

`ncc-machine::x86_virtualization` es la autoridad compartida. Expone:

- `IntelVmxonRegion` e `IntelVmcsRegion` de 4096 bytes, alineados a 4096;
- revision VMCS, shadow bit y abort indicator sin reinterpretaciones de texto;
- `VmcsFieldEncoding` con width, class, index y high/full access validados;
- `VmcsField<T>` para `u16`, `u32`, `u64` y natural width;
- `AmdVmcb` de 4096 bytes con control area `0x000..0x3ff` y state-save desde
  `0x400`;
- `VmcbField<T>` con offset, area y permiso `ReadWrite` o `ReadOnly`;
- `VmcbSegment` de 16 bytes y acceso little-endian;
- `InveptDescriptor` e `InvvpidDescriptor` de 16 bytes, alineados a 16;
- tipos cerrados para las cuatro operaciones INVVPID y las dos INVEPT.

El catalogo de constantes cubre los campos VMCS y VMCB operativos incluidos en
esta revision. No es una barrera para CPU futuras: un campo VMCS nuevo entra por
`VmcsField::<T>::try_from_raw`, y una extension VMCB por `VmcbField::new`, conservando
validacion de anchura, rango y acceso.

## ISA material

| Instruccion | Encoding base | Operandos | Feature | Estado |
|---|---|---|---|---|
| `vmxon` | `F3 0F C7 /6` | `m64` | `vmx` | CF/ZF |
| `vmclear` | `66 0F C7 /6` | `m64` | `vmx` | CF/ZF |
| `vmptrld` | `0F C7 /6` | `m64` | `vmx` | CF/ZF |
| `vmptrst` | `0F C7 /7` | `m64` | `vmx` | sin flags |
| `vmread` | `0F 78 /r` | `r/m64, r64` | `vmx` | CF/ZF |
| `vmwrite` | `0F 79 /r` | `r64, r/m64` | `vmx` | CF/ZF |
| `invept` | `66 0F 38 80 /r` | `r64, m128` | `ept` | CF/ZF |
| `invvpid` | `66 0F 38 81 /r` | `r64, m128` | `vpid` | CF/ZF |
| `vmcall` | `0F 01 C1` | ninguno | `vmx` | VM-exit/fault |
| `vmxoff` | `0F 01 C4` | ninguno | `vmx` | CF/ZF |
| `vmlaunch` | `0F 01 C2` | ninguno | `vmx` | CF/ZF |
| `vmresume` | `0F 01 C3` | ninguno | `vmx` | CF/ZF |
| `vmrun` | `0F 01 D8` | direccion implicita en `RAX` | `svm` | sin flags |
| `vmmcall` | `0F 01 D9` | ninguno | `svm` | VM-exit/fault |
| `vmload` | `0F 01 DA` | direccion implicita en `RAX` | `svm` | sin flags |
| `vmsave` | `0F 01 DB` | direccion implicita en `RAX` | `svm` | sin flags |
| `stgi` | `0F 01 DC` | ninguno | `svm` | GIF=1 |
| `clgi` | `0F 01 DD` | ninguno | `svm` | GIF=0 |
| `invlpga` | `0F 01 DF` | `RAX` direccion, `RCX` ASID | `svm` | invalida TLB |

ModRM/SIB/disp usan el encoder comun x86_64, por lo que las formas de memoria
admiten base, index, scale y displacement. `vmread` y `vmwrite` conservan la
direccion de operandos definida por la ISA; INVEPT/INVVPID exigen descriptor m128.

## Perfil y ejecucion

La fuente `.nm` debe declarar perfil `kernel`, `privileged`, `baremetal`, `firmware`
o equivalente, y las features utilizadas. Perfil `userland` produce
`E_MACHINE_PRIVILEGE_001`; una feature ausente produce
`E_MACHINE_FEATURE_REQUIRED_001`.

`machine run-bytes` es un loader userland y rechaza el stream antes de reservar
memoria ejecutable con `E_MACHINE_JIT_PRIVILEGED_001`. No existe una opcion que
convierta un proceso userland en ring 0. La ruta correcta es una imagen kernel y un
runner bare-metal o VM con nested virtualization.

## Comandos reproducibles

```text
ncc machine virtualization --json
ncc machine check examples/machine/x86_virtualization_vmcs_vmcb.nm
ncc machine emit-bytes examples/machine/x86_virtualization_vmcs_vmcb.nm --out virtualization.bin
ncc machine disasm-bytes virtualization.bin --arch x86_64
ncc machine emit-object examples/machine/x86_virtualization_vmcs_vmcb.nm --target x86_64-unknown-linux-gnu --out virtualization.o
ncc machine verify-object virtualization.o
ncc machine roundtrip examples/machine/x86_virtualization_vmcs_vmcb.nm --target x86_64-unknown-linux-gnu --json
ncc machine virtualization certify-host --out certificate.json
ncc machine virtualization verify-certificate certificate.json --json
```

La evidencia actual cubre 70 bytes, 22 instrucciones del fixture (19 familias de
virtualizacion mas formas register/memory repetidas y `ret`) y roundtrip lossless.
El contrato EPT/NPT y el certificado CPU-backed se documentan en
`machine_x86_guest_memory_ept_npt_v1.md` y
`machine_x86_virtualization_hardware_evidence_v1.md`.

Referencias normativas: Intel 64 and IA-32 Architectures Software Developer's
Manual y AMD64 Architecture Programmer's Manual, Volumes 2 y 3.
