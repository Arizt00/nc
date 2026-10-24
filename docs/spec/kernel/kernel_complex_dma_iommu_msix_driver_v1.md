# Complex DMA/IOMMU/MSI-X driver contract v1

This contract defines the raw kernel/system surface for complex device drivers.
The `.n` coordinator remains the source of authority; machine provides the
typed registers, IOMMU state, MSI-X state, relocations and ABI.

## Required driver phases

1. Device discovery through PCI config or ECAM.
2. Bus mastering only after the IOMMU domain is mapped.
3. DMA descriptors must use IOVA, byte length and explicit permissions.
4. MSI-X table entries must be programmed and unmasked explicitly.
5. Every device notification must be followed by a DMA fence.
6. Recovery must quiesce the device, mask MSI-X, disable bus master, flush IOTLB,
   reset, re-enable bus master, unmask MSI-X and verify the device is alive.

## Physical certificate boundary

`certify-vfio --exercise` is the physical certification path. It accepts legacy
VFIO groups and VFIO cdev/iommufd. A certificate is physical only when all of
these are true:

- hardware IOMMU family is identified;
- IOMMU group is isolated;
- DMA map and unmap executed;
- the device changed the DMA buffer according to exact checks or SHA-256;
- MSI-X delivered through eventfd;
- interrupt remapping path executed;
- reset and recovery completed;
- teardown completed.

Control-plane evidence is useful, but it is not a physical driver certificate.
QEMU, software vIOMMU and emulated markers never count as physical hardware.

## Compact example

`examples/11_machine/kernel_complex_dma_iommu_msix_driver.n` uses no imports and
no `[@kernel]` marker. It is a manual/raw kernel coordinator with MMIO `at`,
ECAM, IOMMU mapping, MSI-X table writes, DMA fences and recovery.
