# ABI kernel raw para hipervisor e IOMMU v1

## Regla

Un archivo kernel sigue siendo un `.n` normal. No necesita `[@kernel]` ni imports
hosted. El perfil se selecciona al construir:

```text
ncc build kernel.n --profile kernel --target aarch64-unknown-none
```

Los intrínsecos reciben direcciones, índices y valores crudos. No reservan heap,
no crean tablas, no fijan memoria, no descubren dispositivos y no programan un
scheduler. Esas decisiones pertenecen al kernel.

## Superficie AArch64

| Intrínseco | Firma compacta | Efecto |
|---|---|---|
| `el2_read_hcr` | `() -> int` | Lee `HCR_EL2`. |
| `el2_write_hcr` | `(int) -> bool` | Escribe `HCR_EL2` y ejecuta `ISB`. |
| `el2_write_vtcr` | `(int) -> bool` | Programa `VTCR_EL2`. |
| `el2_write_vttbr` | `(int) -> bool` | Programa raíz/VMID stage-2. |
| `el2_tlbi_all` | `() -> bool` | DSB, TLBI stage-1+2, DSB, ISB. |
| `el2_hvc` | `(int) -> int` | Ejecuta el hypercall material. |
| `el2_exit_snapshot` | `(addr) -> bool` | Guarda ESR/FAR/HPFAR/ELR/SPSR. |

## Superficie RISC-V H

| Intrínseco | Firma compacta | Efecto |
|---|---|---|
| `riscv_h_read_hstatus` | `() -> int` | Lee `hstatus`. |
| `riscv_h_write_hstatus` | `(int) -> bool` | Escribe `hstatus`. |
| `riscv_h_write_hgatp` | `(int) -> bool` | Programa G-stage. |
| `riscv_h_hfence_gvma` | `(int,int) -> bool` | Invalida GPA/VMID. |
| `riscv_h_hfence_vvma` | `(int,int) -> bool` | Invalida GVA/ASID. |
| `riscv_h_hlv` | `(addr) -> int` | Carga desde memoria virtual guest. |
| `riscv_h_hsv` | `(addr,int) -> bool` | Escribe memoria virtual guest. |
| `riscv_h_exit_snapshot` | `(addr) -> bool` | Guarda trap HS/VS completo. |

## Superficie IOMMU/DMA

| Intrínseco | Firma compacta | Efecto |
|---|---|---|
| `iommu_map_entry` | `(addr,int,int) -> bool` | Escribe una entrada de tabla. |
| `iommu_unmap_entry` | `(addr,int) -> bool` | Invalida la entrada indicada. |
| `iommu_invalidate` | `(addr,int) -> bool` | Publica un comando de invalidación. |
| `dma_descriptor_write` | `(addr,addr,addr,int,int) -> bool` | Materializa descriptor IOVA/host/longitud/permisos. |
| `dma_fence` | `() -> bool` | Emite barrera target-specific. |

Los punteros no se validan y pueden referir MMIO, RAM física, memoria no mapeada
o una dirección colgante. El backend conserva `volatile`, orden y anchura; no
inserta bounds checks, lifetime tracking ni asignaciones ocultas.

## Cadena de materialización

```text
.n compacto
  -> parser/AST
  -> firmas sema kernel
  -> LLVM IR freestanding con intrínsecos materiales
  -> objeto target-specific
  -> linker interno
  -> ELF/imagen kernel sin imports
```

La prueba `ncc_build_kernel_portable_el2_h_iommu_intrinsics_into_real_objects`
exige objetos no vacíos, símbolos resueltos y ausencia de imports finales para
AArch64, RV64 y RV32.

## Límite hosted

WHPX, KVM, HVF y VFIO viven en el host CLI. Un kernel puede implementar la misma
ABI directamente desde EL2/HS/ring0. Ningún proveedor hosted se enlaza de forma
oculta en un artefacto freestanding.
