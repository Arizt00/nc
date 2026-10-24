# Manual DMA and IOMMU contract v1

This contract is the raw kernel/system surface for DMA. It performs no hidden
allocation, pinning, cache discovery, IOVA selection, recovery or device reset.
The program owns every address, table, queue, page bit and ordering operation.

## Canonical operations

- `dma_pin_pages` and `dma_unpin_pages` update caller-owned page metadata.
- `dma_sg_write` writes one 32-byte scatter/gather entry.
- `dma_bounce_to_device` and `dma_bounce_from_device` copy explicit byte ranges.
- `dma_cache_clean`, `dma_cache_invalidate` and `dma_cache_flush` walk a range
  using the cache-line size supplied by the caller.
- `iommu_map_entry128` and `iommu_command_write` store caller-supplied 128-bit
  entries without interpreting or masking vendor-private bits.
- `iommu_fault_copy` copies a raw fault record into caller-owned storage.
- `dma_fence` is the explicit ordering boundary before device notification or
  CPU consumption.

The word `pin` means setting the selected bit in page metadata. Making a page
resident, obtaining its physical address and preventing migration remain pager
or driver operations. Hosted page locking and IOMMU mapping are distinct steps.

## Machine materialization

Machine owns typed 16-byte IOMMU commands and 32-byte SG records. Its cache
plans cover:

- x86/i386: `clflush`, `clflushopt`, `clwb`, `sfence`, `mfence`;
- AArch64: `dc ivac`, `dc cvac`, `dc civac`, `dsb osh`, `dsb oshst`;
- ARMv7: `DCIMVAC`, `DCCMVAC`, `DCCIMVAC`, `DSB`;
- RV32/RV64: Zicbom `cbo.inval`, `cbo.clean`, `cbo.flush`, plus `fence`.

The LLVM freestanding route emits no division helper, libc call or hosted
runtime import. Range iteration uses explicit pointer increments so 32-bit
targets remain self-contained.

## Vendor command catalogs

The raw command catalog includes Intel VT-d queued invalidation descriptors,
AMD-Vi command words, Arm SMMUv3 command queue opcodes and RISC-V IOMMU command
opcodes. Hardware-specific validation is optional tooling; raw emission always
preserves the programmer's words.

## Isolation and failure

The reference IOMMU model verifies domain/device attachment, overlap, access
permissions, pin state, IOTLB invalidation and bounded fault queues. It never
runs recovery automatically. The driver must quiesce, mask MSI-X, disable bus
mastering, invalidate, reset and re-enable in its chosen order.

Invalid descriptors, stale IOVA, missing cache maintenance, unpinned pages and
DMA after unmap remain programmer-controlled failure modes. Raw lowering does
not make them deterministic or safe.

## Artifact matrix

`examples/11_machine/kernel_dma_iommu_manual.n` builds through the normal
kernel path to `.nm`, `.obj`, linked image and manifest for:

- `x86_64-unknown-none`;
- `i386-unknown-none-elf`;
- `aarch64-unknown-none`;
- `armv7a-unknown-none-eabi`;
- `riscv32imac-unknown-elf`;
- `riscv64-unknown-elf`.

These artifacts certify parser, sema, IR, LLVM, machine, object and linker
materialization. Physical VT-d, AMD-Vi, SMMUv3 or RISC-V IOMMU execution needs
an attached device and is never inferred from a software model.

## Normative references

- [Linux DMA API HOWTO](https://docs.kernel.org/core-api/dma-api-howto.html)
- [Windows scatter/gather DMA](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/using-scatter-gather-dma)
- [Intel VT-d platform specification](https://edc.intel.com/content/www/id/id/design/products/platforms/details/raptor-lake-s/13th-generation-core-processors-datasheet-volume-1-of-2/004/intel-virtualization-technology-for-directed-i-o/)
- [AMD IOMMU architecture specification](https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/specifications/48882_IOMMU.pdf)
- [Arm SMMU architecture specification](https://documentation-service.arm.com/static/69ae9767e79f9a1d642aab2d)
- [RISC-V IOMMU architecture specification](https://docs.riscv.org/reference/iommu/v20250828/_attachments/riscv-iommu.pdf)
