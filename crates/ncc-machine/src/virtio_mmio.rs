use crate::{PciVirtioError, VirtioBlockDevice, VirtioFeatures, VirtioGuestMemory};

pub const VIRTIO_MMIO_MAGIC: u32 = 0x7472_6976;
pub const VIRTIO_MMIO_VERSION: u32 = 2;
pub const VIRTIO_MMIO_VENDOR_NCC: u32 = 0x4e43_4320;
pub const VIRTIO_MMIO_BLOCK_DEVICE_ID: u32 = 2;
pub const VIRTIO_MMIO_BYTES: u64 = 0x1000;

const REG_MAGIC: u64 = 0x000;
const REG_VERSION: u64 = 0x004;
const REG_DEVICE_ID: u64 = 0x008;
const REG_VENDOR_ID: u64 = 0x00c;
const REG_DEVICE_FEATURES: u64 = 0x010;
const REG_DEVICE_FEATURES_SEL: u64 = 0x014;
const REG_DRIVER_FEATURES: u64 = 0x020;
const REG_DRIVER_FEATURES_SEL: u64 = 0x024;
const REG_QUEUE_SEL: u64 = 0x030;
const REG_QUEUE_NUM_MAX: u64 = 0x034;
const REG_QUEUE_NUM: u64 = 0x038;
const REG_QUEUE_READY: u64 = 0x044;
const REG_QUEUE_NOTIFY: u64 = 0x050;
const REG_INTERRUPT_STATUS: u64 = 0x060;
const REG_INTERRUPT_ACK: u64 = 0x064;
const REG_STATUS: u64 = 0x070;
const REG_QUEUE_DESC_LOW: u64 = 0x080;
const REG_QUEUE_DESC_HIGH: u64 = 0x084;
const REG_QUEUE_DRIVER_LOW: u64 = 0x090;
const REG_QUEUE_DRIVER_HIGH: u64 = 0x094;
const REG_QUEUE_DEVICE_LOW: u64 = 0x0a0;
const REG_QUEUE_DEVICE_HIGH: u64 = 0x0a4;
const REG_CONFIG_GENERATION: u64 = 0x0fc;
const REG_CONFIG: u64 = 0x100;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct QueueAddresses {
    descriptor: u64,
    driver: u64,
    device: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VirtioMmioAction {
    pub queue: Option<u16>,
    pub completed_requests: usize,
    pub raise_interrupt: bool,
    pub reset: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioMmioBlockDevice {
    pub block: VirtioBlockDevice,
    queue_addresses: QueueAddresses,
}

impl VirtioMmioBlockDevice {
    pub fn new(mut block: VirtioBlockDevice) -> Result<Self, PciVirtioError> {
        // VirtIO block: SIZE_MAX, SEG_MAX, BLK_SIZE and RO when applicable.
        for feature in [1, 2, 6] {
            block.transport.offered_features.insert(feature)?;
        }
        if block.backend.read_only {
            block.transport.offered_features.insert(5)?;
        }
        Ok(Self {
            block,
            queue_addresses: QueueAddresses::default(),
        })
    }

    pub fn read(&mut self, offset: u64, output: &mut [u8]) -> Result<(), PciVirtioError> {
        if output.is_empty() || output.len() > 8 || offset >= VIRTIO_MMIO_BYTES {
            return Err(PciVirtioError::InvalidConfigAccess);
        }
        if offset >= REG_CONFIG {
            return self.read_device_config(offset - REG_CONFIG, output);
        }
        if output.len() != 4 || offset & 3 != 0 {
            return Err(PciVirtioError::InvalidConfigAccess);
        }
        let value = match offset {
            REG_MAGIC => VIRTIO_MMIO_MAGIC,
            REG_VERSION => VIRTIO_MMIO_VERSION,
            REG_DEVICE_ID => VIRTIO_MMIO_BLOCK_DEVICE_ID,
            REG_VENDOR_ID => VIRTIO_MMIO_VENDOR_NCC,
            REG_DEVICE_FEATURES => self
                .block
                .transport
                .offered_features
                .page(self.block.transport.device_feature_select),
            REG_DEVICE_FEATURES_SEL => self.block.transport.device_feature_select,
            REG_DRIVER_FEATURES => self
                .block
                .transport
                .driver_features
                .page(self.block.transport.driver_feature_select),
            REG_DRIVER_FEATURES_SEL => self.block.transport.driver_feature_select,
            REG_QUEUE_SEL => self.block.transport.queue_select as u32,
            REG_QUEUE_NUM_MAX => self
                .selected_queue()
                .map_or(0, |queue| queue.maximum_size as u32),
            REG_QUEUE_NUM => self.selected_queue().map_or(0, |queue| queue.size as u32),
            REG_QUEUE_READY => self.selected_queue().is_some_and(|queue| queue.ready) as u32,
            REG_INTERRUPT_STATUS => self.block.transport.isr_status as u32,
            REG_STATUS => self.block.transport.device_status as u32,
            REG_QUEUE_DESC_LOW => self.queue_addresses.descriptor as u32,
            REG_QUEUE_DESC_HIGH => (self.queue_addresses.descriptor >> 32) as u32,
            REG_QUEUE_DRIVER_LOW => self.queue_addresses.driver as u32,
            REG_QUEUE_DRIVER_HIGH => (self.queue_addresses.driver >> 32) as u32,
            REG_QUEUE_DEVICE_LOW => self.queue_addresses.device as u32,
            REG_QUEUE_DEVICE_HIGH => (self.queue_addresses.device >> 32) as u32,
            REG_CONFIG_GENERATION => self.block.transport.config_generation as u32,
            _ => 0,
        };
        output.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn write<M: VirtioGuestMemory + ?Sized>(
        &mut self,
        offset: u64,
        input: &[u8],
        memory: &mut M,
    ) -> Result<VirtioMmioAction, PciVirtioError> {
        if input.len() != 4 || offset & 3 != 0 || offset >= REG_CONFIG {
            return Err(PciVirtioError::InvalidConfigAccess);
        }
        let value = u32::from_le_bytes(input.try_into().expect("four-byte MMIO write"));
        let mut action = VirtioMmioAction::default();
        match offset {
            REG_DEVICE_FEATURES_SEL => {
                if value >= 4 {
                    return Err(PciVirtioError::FeatureNegotiationFailed);
                }
                self.block.transport.device_feature_select = value;
            }
            REG_DRIVER_FEATURES_SEL => {
                if value >= 4 {
                    return Err(PciVirtioError::FeatureNegotiationFailed);
                }
                self.block.transport.driver_feature_select = value;
            }
            REG_DRIVER_FEATURES => self
                .block
                .transport
                .driver_features
                .set_page(self.block.transport.driver_feature_select, value)?,
            REG_QUEUE_SEL => {
                self.block.transport.queue_select =
                    u16::try_from(value).map_err(|_| PciVirtioError::InvalidQueue)?;
            }
            REG_QUEUE_NUM => {
                let queue = self.selected_queue_mut()?;
                if value == 0 || value > queue.maximum_size as u32 || !value.is_power_of_two() {
                    return Err(PciVirtioError::InvalidQueue);
                }
                queue.size = value as u16;
            }
            REG_QUEUE_READY => match value {
                0 => self.selected_queue_mut()?.reset(),
                1 => {
                    let addresses = self.queue_addresses;
                    let size = self.selected_queue_mut()?.size;
                    self.selected_queue_mut()?.configure(
                        size,
                        addresses.descriptor,
                        addresses.driver,
                        addresses.device,
                    )?;
                }
                _ => return Err(PciVirtioError::InvalidQueue),
            },
            REG_QUEUE_NOTIFY => {
                let queue = u16::try_from(value).map_err(|_| PciVirtioError::InvalidQueue)?;
                self.block.transport.notify(queue)?;
                let completed_requests = self.block.process_queue(memory)?;
                action.queue = Some(queue);
                action.completed_requests = completed_requests;
                action.raise_interrupt = completed_requests != 0;
            }
            REG_INTERRUPT_ACK => {
                self.block.transport.isr_status &= !(value as u8);
            }
            REG_STATUS => {
                self.block.transport.set_status(value as u8)?;
                if value == 0 {
                    self.queue_addresses = QueueAddresses::default();
                    action.reset = true;
                }
            }
            REG_QUEUE_DESC_LOW => set_low(&mut self.queue_addresses.descriptor, value),
            REG_QUEUE_DESC_HIGH => set_high(&mut self.queue_addresses.descriptor, value),
            REG_QUEUE_DRIVER_LOW => set_low(&mut self.queue_addresses.driver, value),
            REG_QUEUE_DRIVER_HIGH => set_high(&mut self.queue_addresses.driver, value),
            REG_QUEUE_DEVICE_LOW => set_low(&mut self.queue_addresses.device, value),
            REG_QUEUE_DEVICE_HIGH => set_high(&mut self.queue_addresses.device, value),
            // Read-only, reserved and device-specific configuration writes are ignored by spec.
            REG_MAGIC
            | REG_VERSION
            | REG_DEVICE_ID
            | REG_VENDOR_ID
            | REG_DEVICE_FEATURES
            | REG_QUEUE_NUM_MAX
            | REG_INTERRUPT_STATUS
            | REG_CONFIG_GENERATION => {}
            _ => {}
        }
        Ok(action)
    }

    pub fn interrupt_pending(&self) -> bool {
        self.block.transport.isr_status != 0
    }

    pub fn negotiated_features(&self) -> VirtioFeatures {
        self.block.transport.driver_features
    }

    fn selected_queue(&self) -> Option<&crate::VirtQueue> {
        self.block
            .transport
            .queues
            .get(self.block.transport.queue_select as usize)
    }

    fn selected_queue_mut(&mut self) -> Result<&mut crate::VirtQueue, PciVirtioError> {
        self.block
            .transport
            .queues
            .get_mut(self.block.transport.queue_select as usize)
            .ok_or(PciVirtioError::InvalidQueue)
    }

    fn read_device_config(
        &self,
        config_offset: u64,
        output: &mut [u8],
    ) -> Result<(), PciVirtioError> {
        let mut config = [0u8; 64];
        config[0..8].copy_from_slice(&self.block.backend.sectors().to_le_bytes());
        config[8..12].copy_from_slice(&(u32::MAX - 4095).to_le_bytes());
        config[12..16].copy_from_slice(&254_u32.to_le_bytes());
        config[20..24].copy_from_slice(&self.block.backend.sector_size.to_le_bytes());
        config[32] = self.block.backend.dirty as u8;
        let start =
            usize::try_from(config_offset).map_err(|_| PciVirtioError::InvalidConfigAccess)?;
        let end = start
            .checked_add(output.len())
            .filter(|end| *end <= config.len())
            .ok_or(PciVirtioError::InvalidConfigAccess)?;
        output.copy_from_slice(&config[start..end]);
        Ok(())
    }
}

fn set_low(address: &mut u64, value: u32) {
    *address = (*address & 0xffff_ffff_0000_0000) | value as u64;
}

fn set_high(address: &mut u64, value: u32) {
    *address = (*address & 0x0000_0000_ffff_ffff) | ((value as u64) << 32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        VirtualBlockDevice, VIRTIO_F_RING_EVENT_IDX, VIRTIO_F_VERSION_1, VIRTIO_STATUS_ACKNOWLEDGE,
        VIRTIO_STATUS_DRIVER, VIRTIO_STATUS_DRIVER_OK, VIRTIO_STATUS_FAILED,
        VIRTIO_STATUS_FEATURES_OK, VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE,
    };

    const DESC: usize = 0x1000;
    const AVAIL: usize = 0x2000;
    const USED: usize = 0x3000;
    const HEADER: usize = 0x4000;
    const DATA: usize = 0x5000;
    const STATUS: usize = 0x6000;

    fn write32(
        device: &mut VirtioMmioBlockDevice,
        memory: &mut [u8],
        register: u64,
        value: u32,
    ) -> VirtioMmioAction {
        device
            .write(register, &value.to_le_bytes(), memory)
            .unwrap()
    }

    fn descriptor(
        memory: &mut [u8],
        index: usize,
        address: u64,
        length: u32,
        flags: u16,
        next: u16,
    ) {
        let offset = DESC + index * 16;
        memory[offset..offset + 8].copy_from_slice(&address.to_le_bytes());
        memory[offset + 8..offset + 12].copy_from_slice(&length.to_le_bytes());
        memory[offset + 12..offset + 14].copy_from_slice(&flags.to_le_bytes());
        memory[offset + 14..offset + 16].copy_from_slice(&next.to_le_bytes());
    }

    fn ready_device() -> (VirtioMmioBlockDevice, Vec<u8>) {
        let mut disk = vec![0u8; 4096];
        disk[512..1024].fill(0xa5);
        let backend = VirtualBlockDevice::from_bytes(disk, 512, false).unwrap();
        let block = VirtioBlockDevice::new(backend, b"ncc-mmio-block").unwrap();
        let mut device = VirtioMmioBlockDevice::new(block).unwrap();
        let mut memory = vec![0u8; 0x8000];
        write32(&mut device, &mut memory, REG_DRIVER_FEATURES_SEL, 0);
        write32(
            &mut device,
            &mut memory,
            REG_DRIVER_FEATURES,
            VIRTIO_F_RING_EVENT_IDX as u32,
        );
        write32(&mut device, &mut memory, REG_DRIVER_FEATURES_SEL, 1);
        write32(
            &mut device,
            &mut memory,
            REG_DRIVER_FEATURES,
            (VIRTIO_F_VERSION_1 >> 32) as u32,
        );
        write32(&mut device, &mut memory, REG_QUEUE_SEL, 0);
        write32(&mut device, &mut memory, REG_QUEUE_NUM, 8);
        write32(&mut device, &mut memory, REG_QUEUE_DESC_LOW, DESC as u32);
        write32(&mut device, &mut memory, REG_QUEUE_DRIVER_LOW, AVAIL as u32);
        write32(&mut device, &mut memory, REG_QUEUE_DEVICE_LOW, USED as u32);
        write32(&mut device, &mut memory, REG_QUEUE_READY, 1);
        let status = VIRTIO_STATUS_ACKNOWLEDGE
            | VIRTIO_STATUS_DRIVER
            | VIRTIO_STATUS_FEATURES_OK
            | VIRTIO_STATUS_DRIVER_OK;
        write32(&mut device, &mut memory, REG_STATUS, status as u32);
        (device, memory)
    }

    #[test]
    fn block_mmio_negotiates_queue_reads_disk_and_acknowledges_interrupt() {
        let (mut device, mut memory) = ready_device();
        memory[HEADER..HEADER + 4].copy_from_slice(&0_u32.to_le_bytes());
        memory[HEADER + 8..HEADER + 16].copy_from_slice(&1_u64.to_le_bytes());
        descriptor(&mut memory, 0, HEADER as u64, 16, VIRTQ_DESC_F_NEXT, 1);
        descriptor(
            &mut memory,
            1,
            DATA as u64,
            512,
            VIRTQ_DESC_F_NEXT | VIRTQ_DESC_F_WRITE,
            2,
        );
        descriptor(&mut memory, 2, STATUS as u64, 1, VIRTQ_DESC_F_WRITE, 0);
        memory[AVAIL + 2..AVAIL + 4].copy_from_slice(&1_u16.to_le_bytes());
        memory[AVAIL + 4..AVAIL + 6].copy_from_slice(&0_u16.to_le_bytes());

        let action = write32(&mut device, &mut memory, REG_QUEUE_NOTIFY, 0);
        assert_eq!(action.completed_requests, 1);
        assert!(action.raise_interrupt);
        assert_eq!(&memory[DATA..DATA + 512], &[0xa5; 512]);
        assert_eq!(memory[STATUS], 0);
        assert!(device.interrupt_pending());
        write32(&mut device, &mut memory, REG_INTERRUPT_ACK, 1);
        assert!(!device.interrupt_pending());
    }

    #[test]
    fn block_mmio_event_index_requests_every_followup_notification() {
        let (mut device, mut memory) = ready_device();
        memory[HEADER..HEADER + 4].copy_from_slice(&0_u32.to_le_bytes());
        memory[HEADER + 8..HEADER + 16].copy_from_slice(&1_u64.to_le_bytes());
        descriptor(&mut memory, 0, HEADER as u64, 16, VIRTQ_DESC_F_NEXT, 1);
        descriptor(
            &mut memory,
            1,
            DATA as u64,
            512,
            VIRTQ_DESC_F_NEXT | VIRTQ_DESC_F_WRITE,
            2,
        );
        descriptor(&mut memory, 2, STATUS as u64, 1, VIRTQ_DESC_F_WRITE, 0);
        memory[AVAIL + 2..AVAIL + 4].copy_from_slice(&1_u16.to_le_bytes());
        memory[AVAIL + 4..AVAIL + 6].copy_from_slice(&0_u16.to_le_bytes());

        assert_eq!(
            write32(&mut device, &mut memory, REG_QUEUE_NOTIFY, 0).completed_requests,
            1
        );
        assert_eq!(
            u16::from_le_bytes(memory[USED + 68..USED + 70].try_into().unwrap()),
            1
        );
        write32(&mut device, &mut memory, REG_INTERRUPT_ACK, 1);

        memory[HEADER + 8..HEADER + 16].copy_from_slice(&2_u64.to_le_bytes());
        memory[AVAIL + 2..AVAIL + 4].copy_from_slice(&2_u16.to_le_bytes());
        memory[AVAIL + 6..AVAIL + 8].copy_from_slice(&0_u16.to_le_bytes());
        memory[AVAIL + 20..AVAIL + 22].copy_from_slice(&1_u16.to_le_bytes());
        let action = write32(&mut device, &mut memory, REG_QUEUE_NOTIFY, 0);
        assert_eq!(action.completed_requests, 1);
        assert!(action.raise_interrupt);
        assert_eq!(device.block.transport.queues[0].used_index, 2);
        assert_eq!(
            u16::from_le_bytes(memory[USED + 68..USED + 70].try_into().unwrap()),
            2
        );
    }

    #[test]
    fn block_mmio_reports_identity_capacity_and_resets_all_transport_state() {
        let (mut device, mut memory) = ready_device();
        let mut word = [0u8; 4];
        device.read(REG_MAGIC, &mut word).unwrap();
        assert_eq!(u32::from_le_bytes(word), VIRTIO_MMIO_MAGIC);
        device.read(REG_CONFIG, &mut word).unwrap();
        assert_eq!(u32::from_le_bytes(word), 8);
        let action = write32(&mut device, &mut memory, REG_STATUS, 0);
        assert!(action.reset);
        assert_eq!(device.block.transport.device_status, 0);
        assert!(!device.block.transport.queues[0].ready);
        assert_eq!(device.negotiated_features(), VirtioFeatures::default());
    }

    #[test]
    fn block_mmio_rejects_unaligned_access_and_unsupported_features() {
        let (mut device, mut memory) = ready_device();
        assert_eq!(
            device.read(REG_MAGIC + 1, &mut [0u8; 4]),
            Err(PciVirtioError::InvalidConfigAccess)
        );
        write32(&mut device, &mut memory, REG_STATUS, 0);
        write32(
            &mut device,
            &mut memory,
            REG_STATUS,
            (VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER) as u32,
        );
        write32(&mut device, &mut memory, REG_DRIVER_FEATURES_SEL, 0);
        write32(&mut device, &mut memory, REG_DRIVER_FEATURES, 1 << 31);
        assert_eq!(
            device.write(
                REG_STATUS,
                &(VIRTIO_STATUS_ACKNOWLEDGE as u32
                    | VIRTIO_STATUS_DRIVER as u32
                    | VIRTIO_STATUS_FEATURES_OK as u32)
                    .to_le_bytes(),
                &mut memory,
            ),
            Err(PciVirtioError::FeatureNegotiationFailed)
        );
        assert_eq!(
            device.block.transport.device_status & VIRTIO_STATUS_FAILED,
            0
        );
    }
}
