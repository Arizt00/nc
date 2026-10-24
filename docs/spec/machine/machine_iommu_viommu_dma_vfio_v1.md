# IOMMU, vIOMMU, DMA y VFIO v1

## Modelo portable

`ncc-machine::iommu` define dominios, requester IDs PCI, permisos, mapeos,
traducciones, invalidaciones y colas de fallo sin asignación oculta. Un dominio:

- adjunta y separa dispositivos;
- rechaza solapamientos y desalineaciones;
- traduce IOVA a VA/PA con longitud restante;
- comprueba lectura, escritura y ejecución por separado;
- mantiene generación, command queue, invalidation queue y fault queue.

Hay layouts tipados para Intel VT-d, AMD-Vi, Arm SMMUv3 y RISC-V IOMMU. La
implementación `VirtualIommu` ejecuta el mismo ciclo attach/map/invalidate/fence,
DMA, fault, unmap/detach sobre memoria material.

## VFIO Linux

Se soportan las dos APIs vigentes:

### Container/group

1. valida IOMMU group y que todos sus miembros sean viables;
2. exige que el BDF esté ligado previamente a `vfio-pci`;
3. abre container y group, comprueba API y Type1/Type1v2;
4. obtiene el fd del dispositivo e inventaría regiones/IRQ/reset;
5. fija IOMMU, mapea y desmapea IOVA;
6. separa group y cierra todos los fd incluso ante error.

### VFIO cdev + iommufd

1. abre `/dev/vfio/devices/vfioN` y `/dev/iommu`;
2. liga el dispositivo a iommufd;
3. crea IOAS y adjunta el hw page table;
4. ejecuta `IOMMU_IOAS_MAP/UNMAP`;
5. separa, destruye IOAS y cierra descriptores por RAII.

## Dos clases de certificado

`vfio_control_plane` demuestra acceso al dispositivo, IOMMU real y map/unmap.
No afirma que el dispositivo haya producido DMA.

`vfio_physical_passthrough` exige además:

- manifiesto `ncc.machine.vfio_dma_exercise.v1` validado;
- identidad BDF/vendor/device cuando se declara;
- control exclusivo reconocido explícitamente;
- BAR accesible por `mmap` volátil o `pread/pwrite` VFIO;
- programación MMIO con IOVA/longitud simbólicas;
- polling y deadline acotados;
- buffer distinto del patrón inicial;
- hash SHA-256 exacto o comprobaciones exactas de contenido;
- identidad PCI BDF/vendor/device obligatoria;
- todos los miembros del grupo ligados a `vfio-pci`;
- familia física detectada desde sysfs: VT-d, AMD-Vi, SMMUv3 o RISC-V IOMMU;
- vector MSI-X ligado a `eventfd` antes de iniciar el dispositivo;
- interrupción originada por el dispositivo, con contador y deadline observados;
- ruta de remapeo de interrupción VFIO sin loopback de software;
- reset VFIO antes y después;
- unmap, detach y teardown completos.

El esquema está en `tools/release/vfio-dma-exercise.schema.json`. Cada protocolo
de dispositivo aporta sus offsets y valores; el certificador genérico no inventa
una secuencia DMA universal.

## Ejemplo de ejercicio

Los offsets siguientes son ilustrativos. Solo deben sustituirse por el contrato
documentado del dispositivo asignado al runner.

```json
{
  "schema": "ncc.machine.vfio_dma_exercise.v1",
  "acknowledge_exclusive_device_control": true,
  "device": {
    "bdf": "0000:03:00.0",
    "vendor": "0x1234",
    "device": "0x5678"
  },
  "mmio": {
    "region_index": 0,
    "operations": [
      {"op": "write", "offset": "0x20", "width": 4, "value": "dma_iova_low"},
      {"op": "write", "offset": "0x24", "width": 4, "value": "dma_iova_high"},
      {"op": "write", "offset": "0x28", "width": 4, "value": "dma_bytes"},
      {"op": "write", "offset": "0x2c", "width": 4, "value": 1},
      {"op": "poll", "offset": "0x30", "width": 4, "mask": 1, "equals": 1}
    ]
  },
  "dma": {
    "bytes": 16384,
    "seed_byte": 90,
    "completion": {
      "expect_changed": true,
      "checks": [{"offset": 0, "width": 4, "equals": "0x12345678"}]
    }
  },
  "interrupt": {
    "mode": "msix",
    "vector": 0,
    "minimum_count": 1,
    "timeout_ms": 5000,
    "require_vfio_interrupt_remapping": true
  },
  "reset": {"before": true, "after": true}
}
```

## Comandos

```text
ncc machine virtualization probe-iommu --json
ncc machine virtualization certify-viommu --out viommu.json
ncc machine virtualization certify-vfio --group 12 --device 0000:03:00.0 --out control.json
ncc machine virtualization certify-vfio --group 12 --device 0000:03:00.0 --exercise exercise.json --out physical.json
ncc machine virtualization verify-iommu-certificate physical.json
```

El certificado se sella con JSON canónico, dominio separado y SHA-256. El
verificador rechaza elevar un certificado de control plane a físico aunque se
recalcule correctamente su hash.

## Autoridades externas

- [Linux VFIO](https://docs.kernel.org/driver-api/vfio.html)
- [KVM VFIO device](https://docs.kernel.org/virt/kvm/devices/vfio.html)
- [RISC-V IOMMU](https://docs.riscv.org/reference/iommu/index.html)
