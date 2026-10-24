# IOMMU y passthrough fisico v2

## Alcance

El certificador cubre VFIO legacy container/group y VFIO cdev con iommufd/IOAS.
La familia fisica se obtiene de sysfs, identidad CPU y nodos IOMMU; no se acepta
una etiqueta declarada por el ejercicio.

Familias admitidas:

- Intel VT-d;
- AMD-Vi;
- Arm SMMUv3;
- RISC-V IOMMU.

## Requisitos de un certificado completo

El dispositivo debe estar bajo control exclusivo y su manifiesto debe fijar
BDF, vendor y device. Antes de tocar MMIO se comprueba:

1. grupo IOMMU existente y viable;
2. todos sus miembros ligados a `vfio-pci`;
3. dispositivo con reset VFIO;
4. mapa IOVA de memoria alineada y fijada;
5. region BAR y anchuras MMIO validas;
6. indice MSI-X con soporte `eventfd`.

La ejecucion obliga al dispositivo a modificar el buffer DMA, valida hash o
campos exactos, recibe al menos una interrupcion MSI-X fisica y desasigna el
vector. Despues ejecuta unmap, detach, destruccion IOAS y reset final. RAII
repite la limpieza si cualquier operacion falla.

No se usa `VFIO_IRQ_SET_DATA_NONE` como loopback: esa prueba seria una
interrupcion generada por el kernel y no evidencia del dispositivo.

## IOVA, aislamiento e interrupciones

`iova_isolation_executed` prueba el ciclo MAP/UNMAP en un dominio exclusivo.
`iommu_group_isolation_verified` prueba la frontera de dispositivos del grupo.
`vfio_interrupt_remapping_path_executed` significa que una MSI-X del dispositivo
llego al `eventfd` instalado por VFIO. El informe declara por separado que ncc
no lee tablas privadas de remapeo del firmware o del kernel.

## Matriz fisica

| Lane | Runner | Arquitectura exigida |
|---|---|---|
| VT-d | `ncc-vfio-vtd` | `intel_vtd` |
| AMD-Vi | `ncc-vfio-amdvi` | `amd_vi` |
| SMMUv3 | `ncc-vfio-smmuv3` | `arm_smmuv3` |
| RISC-V IOMMU | `ncc-vfio-riscv` | `riscv_iommu` |

Cada runner aporta un protocolo de dispositivo real mediante
`NCC_VFIO_DMA_EXERCISE`; no existe una secuencia MMIO universal segura para
todos los dispositivos PCI.

## Comandos

```text
ncc machine virtualization probe-iommu --json
ncc machine virtualization certify-vfio --group 12 --device 0000:03:00.0 --exercise device.json --out physical.json
ncc machine virtualization verify-iommu-certificate physical.json --json
```

Sin grupo asignable o sin dispositivo exclusivo el comando debe fallar. Ese
resultado es `unavailable`, no un certificado parcial.
