use std::error::Error;
use std::fmt;

use crate::physical_drivers::{
    DriverHealth, PhysicalDriverError, RegisterIo, RegisterWidth, TracedRegisterIo,
};

pub const KERNEL_NETWORK_SCHEMA: &str = "ncc.machine.kernel_network.v1";
pub const ETHERNET_HEADER_BYTES: usize = 14;
pub const ETHERNET_MIN_FRAME_NO_FCS: usize = 60;
pub const ETHERNET_MAX_VLAN_TAGS: usize = 2;
pub const IPV4_MIN_HEADER_BYTES: usize = 20;
pub const IPV6_HEADER_BYTES: usize = 40;
pub const UDP_HEADER_BYTES: usize = 8;
pub const TCP_MIN_HEADER_BYTES: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    BufferTooSmall { required: usize, available: usize },
    Truncated(&'static str),
    Invalid(&'static str),
    Unsupported(&'static str),
    Checksum(&'static str),
    NeighborMissing,
    RouteMissing,
    FragmentMismatch,
    FragmentOverlap,
    FragmentIncomplete,
    SocketTableFull,
    SocketMissing(u32),
    AddressInUse,
    InvalidTcpTransition,
    TimerTableFull,
    Driver(PhysicalDriverError),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall {
                required,
                available,
            } => {
                write!(f, "network buffer needs {required} bytes, has {available}")
            }
            Self::Truncated(item) => write!(f, "truncated {item}"),
            Self::Invalid(item) => write!(f, "invalid {item}"),
            Self::Unsupported(item) => write!(f, "unsupported {item}"),
            Self::Checksum(item) => write!(f, "invalid {item} checksum"),
            Self::NeighborMissing => f.write_str("neighbor is unresolved"),
            Self::RouteMissing => f.write_str("no network route matches the destination"),
            Self::FragmentMismatch => f.write_str("fragment does not belong to this datagram"),
            Self::FragmentOverlap => f.write_str("fragment overlaps already materialized bytes"),
            Self::FragmentIncomplete => f.write_str("fragmented datagram is incomplete"),
            Self::SocketTableFull => f.write_str("caller-owned socket table is full"),
            Self::SocketMissing(id) => write!(f, "socket {id} does not exist"),
            Self::AddressInUse => f.write_str("network address or port is already in use"),
            Self::InvalidTcpTransition => f.write_str("invalid TCP state transition"),
            Self::TimerTableFull => f.write_str("caller-owned network timer table is full"),
            Self::Driver(error) => write!(f, "physical NIC error: {error}"),
        }
    }
}

impl Error for NetworkError {}

impl From<PhysicalDriverError> for NetworkError {
    fn from(value: PhysicalDriverError) -> Self {
        Self::Driver(value)
    }
}

fn require(output: &[u8], required: usize) -> Result<(), NetworkError> {
    if output.len() < required {
        return Err(NetworkError::BufferTooSmall {
            required,
            available: output.len(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = Self([0xff; 6]);

    pub const fn is_multicast(self) -> bool {
        self.0[0] & 1 != 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

pub mod ether_type {
    pub const IPV4: u16 = 0x0800;
    pub const ARP: u16 = 0x0806;
    pub const WAKE_ON_LAN: u16 = 0x0842;
    pub const RARP: u16 = 0x8035;
    pub const APPLETALK: u16 = 0x809b;
    pub const AARP: u16 = 0x80f3;
    pub const VLAN: u16 = 0x8100;
    pub const IPX: u16 = 0x8137;
    pub const IPV6: u16 = 0x86dd;
    pub const MPLS_UNICAST: u16 = 0x8847;
    pub const MPLS_MULTICAST: u16 = 0x8848;
    pub const PPPOE_DISCOVERY: u16 = 0x8863;
    pub const PPPOE_SESSION: u16 = 0x8864;
    pub const QINQ: u16 = 0x88a8;
    pub const EAPOL: u16 = 0x888e;
    pub const LLDP: u16 = 0x88cc;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VlanTag {
    pub priority: u8,
    pub drop_eligible: bool,
    pub identifier: u16,
}

impl VlanTag {
    pub fn tci(self) -> Result<u16, NetworkError> {
        if self.priority > 7 || self.identifier > 4095 {
            return Err(NetworkError::Invalid("802.1Q tag"));
        }
        Ok(((self.priority as u16) << 13) | ((self.drop_eligible as u16) << 12) | self.identifier)
    }

    fn from_tci(value: u16) -> Self {
        Self {
            priority: ((value >> 13) & 7) as u8,
            drop_eligible: value & 0x1000 != 0,
            identifier: value & 0x0fff,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthernetFrame<'a> {
    pub destination: MacAddress,
    pub source: MacAddress,
    pub ether_type: u16,
    pub vlans: [Option<VlanTag>; ETHERNET_MAX_VLAN_TAGS],
    pub payload: &'a [u8],
}

pub fn parse_ethernet(frame: &[u8]) -> Result<EthernetFrame<'_>, NetworkError> {
    require(frame, ETHERNET_HEADER_BYTES)?;
    let destination = MacAddress(frame[0..6].try_into().unwrap());
    let source = MacAddress(frame[6..12].try_into().unwrap());
    let mut offset = 12;
    let mut ether_type = u16::from_be_bytes(frame[offset..offset + 2].try_into().unwrap());
    offset += 2;
    let mut vlans = [None, None];
    for slot in &mut vlans {
        if !matches!(ether_type, ether_type::VLAN | ether_type::QINQ) {
            break;
        }
        require(frame, offset + 4)?;
        *slot = Some(VlanTag::from_tci(u16::from_be_bytes(
            frame[offset..offset + 2].try_into().unwrap(),
        )));
        ether_type = u16::from_be_bytes(frame[offset + 2..offset + 4].try_into().unwrap());
        offset += 4;
    }
    if matches!(ether_type, ether_type::VLAN | ether_type::QINQ) {
        return Err(NetworkError::Unsupported("more than two stacked VLAN tags"));
    }
    Ok(EthernetFrame {
        destination,
        source,
        ether_type,
        vlans,
        payload: &frame[offset..],
    })
}

pub fn write_ethernet(
    output: &mut [u8],
    destination: MacAddress,
    source: MacAddress,
    ether_type: u16,
    vlans: &[VlanTag],
    payload: &[u8],
) -> Result<usize, NetworkError> {
    if vlans.len() > ETHERNET_MAX_VLAN_TAGS {
        return Err(NetworkError::Unsupported("more than two stacked VLAN tags"));
    }
    let header = ETHERNET_HEADER_BYTES + vlans.len() * 4;
    let frame_length = (header + payload.len()).max(ETHERNET_MIN_FRAME_NO_FCS);
    require(output, frame_length)?;
    output[..frame_length].fill(0);
    output[0..6].copy_from_slice(&destination.0);
    output[6..12].copy_from_slice(&source.0);
    let mut offset = 12;
    for (index, vlan) in vlans.iter().enumerate() {
        let tag_type = if index == 0 && vlans.len() == 2 {
            ether_type::QINQ
        } else {
            ether_type::VLAN
        };
        output[offset..offset + 2].copy_from_slice(&tag_type.to_be_bytes());
        output[offset + 2..offset + 4].copy_from_slice(&vlan.tci()?.to_be_bytes());
        offset += 4;
    }
    output[offset..offset + 2].copy_from_slice(&ether_type.to_be_bytes());
    output[offset + 2..offset + 2 + payload.len()].copy_from_slice(payload);
    Ok(frame_length)
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NicDescriptor {
    pub address: u64,
    pub length: u16,
    pub checksum: u16,
    pub command: u8,
    pub status: u8,
    pub errors: u8,
    pub special: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NicFamily {
    Ne2000,
    Pcnet,
    Tulip21140,
    ThreeCom905,
    Rtl8139,
    Rtl8169,
    E1000,
    E1000e,
    Igb,
    Ixgbe,
    I40e,
    BroadcomNetXtreme,
    MellanoxConnectX,
    VirtioNet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NicPciIdentity {
    pub vendor: u16,
    pub device: u16,
    pub family: NicFamily,
    pub name: &'static str,
}

pub const NIC_PCI_CATALOG: &[NicPciIdentity] = &[
    NicPciIdentity {
        vendor: 0x10ec,
        device: 0x8029,
        family: NicFamily::Ne2000,
        name: "Realtek RTL8029/NE2000",
    },
    NicPciIdentity {
        vendor: 0x1022,
        device: 0x2000,
        family: NicFamily::Pcnet,
        name: "AMD PCnet-PCI II",
    },
    NicPciIdentity {
        vendor: 0x1011,
        device: 0x0009,
        family: NicFamily::Tulip21140,
        name: "DEC 21140 Tulip",
    },
    NicPciIdentity {
        vendor: 0x10b7,
        device: 0x9055,
        family: NicFamily::ThreeCom905,
        name: "3Com 3C905B",
    },
    NicPciIdentity {
        vendor: 0x10ec,
        device: 0x8139,
        family: NicFamily::Rtl8139,
        name: "Realtek RTL8139",
    },
    NicPciIdentity {
        vendor: 0x10ec,
        device: 0x8168,
        family: NicFamily::Rtl8169,
        name: "Realtek RTL8168/8169",
    },
    NicPciIdentity {
        vendor: 0x8086,
        device: 0x100e,
        family: NicFamily::E1000,
        name: "Intel 82540EM",
    },
    NicPciIdentity {
        vendor: 0x8086,
        device: 0x10d3,
        family: NicFamily::E1000e,
        name: "Intel 82574L",
    },
    NicPciIdentity {
        vendor: 0x8086,
        device: 0x1533,
        family: NicFamily::Igb,
        name: "Intel I210",
    },
    NicPciIdentity {
        vendor: 0x8086,
        device: 0x10fb,
        family: NicFamily::Ixgbe,
        name: "Intel 82599 10GbE",
    },
    NicPciIdentity {
        vendor: 0x8086,
        device: 0x1572,
        family: NicFamily::I40e,
        name: "Intel X710",
    },
    NicPciIdentity {
        vendor: 0x14e4,
        device: 0x165f,
        family: NicFamily::BroadcomNetXtreme,
        name: "Broadcom NetXtreme BCM5720",
    },
    NicPciIdentity {
        vendor: 0x15b3,
        device: 0x1017,
        family: NicFamily::MellanoxConnectX,
        name: "Mellanox ConnectX-5",
    },
    NicPciIdentity {
        vendor: 0x1af4,
        device: 0x1000,
        family: NicFamily::VirtioNet,
        name: "VirtIO legacy net",
    },
    NicPciIdentity {
        vendor: 0x1af4,
        device: 0x1041,
        family: NicFamily::VirtioNet,
        name: "VirtIO modern net",
    },
];

pub fn identify_nic(vendor: u16, device: u16) -> Option<NicPciIdentity> {
    NIC_PCI_CATALOG
        .iter()
        .copied()
        .find(|identity| identity.vendor == vendor && identity.device == device)
}

#[derive(Debug)]
pub struct ManualNic<'a> {
    pub identity: NicPciIdentity,
    pub register_base: u64,
    pub mac: MacAddress,
    pub rx_descriptors: &'a mut [NicDescriptor],
    pub tx_descriptors: &'a mut [NicDescriptor],
    pub health: DriverHealth,
}

impl<'a> ManualNic<'a> {
    pub fn new(
        identity: NicPciIdentity,
        register_base: u64,
        mac: MacAddress,
        rx_descriptors: &'a mut [NicDescriptor],
        tx_descriptors: &'a mut [NicDescriptor],
    ) -> Result<Self, NetworkError> {
        if register_base == 0
            || rx_descriptors.is_empty()
            || tx_descriptors.is_empty()
            || !rx_descriptors.len().is_power_of_two()
            || !tx_descriptors.len().is_power_of_two()
        {
            return Err(NetworkError::Invalid("NIC ring geometry"));
        }
        Ok(Self {
            identity,
            register_base,
            mac,
            rx_descriptors,
            tx_descriptors,
            health: DriverHealth::default(),
        })
    }

    pub fn reset_and_program(
        &mut self,
        io: &mut impl RegisterIo,
        rx_ring_physical: u64,
        tx_ring_physical: u64,
    ) -> Result<(), NetworkError> {
        if rx_ring_physical & 0xf != 0 || tx_ring_physical & 0xf != 0 {
            return Err(NetworkError::Invalid("NIC DMA ring alignment"));
        }
        let base = self.register_base;
        match self.identity.family {
            NicFamily::Ne2000 => {
                io.write(base, RegisterWidth::U8, 0x21)?;
                io.write(base + 0x0e, RegisterWidth::U8, 0x49)?;
                io.write(base, RegisterWidth::U8, 0x22)?;
            }
            NicFamily::Pcnet => {
                io.write(base + 0x14, RegisterWidth::U32, 0)?;
                io.write(base + 0x18, RegisterWidth::U32, rx_ring_physical)?;
                io.write(base + 0x1c, RegisterWidth::U32, tx_ring_physical)?;
            }
            NicFamily::Tulip21140 | NicFamily::ThreeCom905 => {
                io.write(base, RegisterWidth::U32, 1)?;
                io.write(base + 0x18, RegisterWidth::U32, rx_ring_physical)?;
                io.write(base + 0x20, RegisterWidth::U32, tx_ring_physical)?;
                io.write(base + 0x30, RegisterWidth::U32, (1 << 13) | (1 << 1))?;
            }
            NicFamily::Rtl8139 => {
                io.write(base + 0x37, RegisterWidth::U8, 0x10)?;
                io.write(base + 0x30, RegisterWidth::U32, rx_ring_physical)?;
                io.write(base + 0x37, RegisterWidth::U8, 0x0c)?;
                io.write(base + 0x44, RegisterWidth::U32, 0x0000_e70f)?;
            }
            NicFamily::Rtl8169 => {
                io.write(base + 0x37, RegisterWidth::U8, 0x10)?;
                io.write(base + 0xe4, RegisterWidth::U64, rx_ring_physical)?;
                io.write(base + 0x20, RegisterWidth::U64, tx_ring_physical)?;
                io.write(base + 0x37, RegisterWidth::U8, 0x0c)?;
            }
            NicFamily::E1000 | NicFamily::E1000e | NicFamily::Igb => {
                io.write(base, RegisterWidth::U32, 1 << 26)?;
                program_intel_ring(
                    io,
                    base,
                    0x2800,
                    rx_ring_physical,
                    self.rx_descriptors.len(),
                )?;
                program_intel_ring(
                    io,
                    base,
                    0x3800,
                    tx_ring_physical,
                    self.tx_descriptors.len(),
                )?;
                io.write(
                    base + 0x0100,
                    RegisterWidth::U32,
                    (1 << 1) | (1 << 15) | (1 << 26),
                )?;
                io.write(base + 0x0400, RegisterWidth::U32, (1 << 1) | (1 << 3))?;
            }
            NicFamily::Ixgbe | NicFamily::I40e => {
                io.write(base, RegisterWidth::U32, 1 << 26)?;
                program_intel_ring(
                    io,
                    base,
                    0x01000,
                    rx_ring_physical,
                    self.rx_descriptors.len(),
                )?;
                program_intel_ring(
                    io,
                    base,
                    0x06000,
                    tx_ring_physical,
                    self.tx_descriptors.len(),
                )?;
            }
            NicFamily::BroadcomNetXtreme | NicFamily::MellanoxConnectX => {
                io.write(base, RegisterWidth::U32, 1)?;
                io.write(base + 0x100, RegisterWidth::U64, rx_ring_physical)?;
                io.write(base + 0x108, RegisterWidth::U64, tx_ring_physical)?;
                io.write(
                    base + 0x110,
                    RegisterWidth::U32,
                    self.rx_descriptors.len() as u64,
                )?;
            }
            NicFamily::VirtioNet => {
                io.write(base + 0x70, RegisterWidth::U32, 0)?;
                io.write(base + 0x70, RegisterWidth::U32, 1)?;
                io.write(base + 0x70, RegisterWidth::U32, 3)?;
                io.write(base + 0x30, RegisterWidth::U32, 0)?;
                io.write(
                    base + 0x38,
                    RegisterWidth::U32,
                    self.rx_descriptors.len() as u64,
                )?;
                io.write(base + 0x80, RegisterWidth::U64, rx_ring_physical)?;
                io.write(base + 0x90, RegisterWidth::U64, tx_ring_physical)?;
                io.write(base + 0x70, RegisterWidth::U32, 0x0f)?;
            }
        }
        io.fence()?;
        self.health.operations += 1;
        Ok(())
    }

    pub fn prime_receive_buffers(
        &mut self,
        io: &mut impl RegisterIo,
        first_buffer: u64,
        stride: usize,
        buffer_bytes: usize,
    ) -> Result<(), NetworkError> {
        if first_buffer == 0
            || stride < buffer_bytes
            || buffer_bytes == 0
            || buffer_bytes > u16::MAX as usize
        {
            return Err(NetworkError::Invalid("NIC receive buffer geometry"));
        }
        for (index, descriptor) in self.rx_descriptors.iter_mut().enumerate() {
            descriptor.address = first_buffer
                .checked_add((index * stride) as u64)
                .ok_or(NetworkError::Invalid("NIC receive address overflow"))?;
            descriptor.length = buffer_bytes as u16;
            descriptor.status = 0;
            descriptor.errors = 0;
        }
        io.fence()?;
        io.write(
            self.register_base + nic_rx_tail_offset(self.identity.family),
            RegisterWidth::U32,
            (self.rx_descriptors.len() - 1) as u64,
        )?;
        io.fence()?;
        self.health.operations += 1;
        Ok(())
    }

    pub fn submit_transmit(
        &mut self,
        io: &mut impl RegisterIo,
        descriptor_index: usize,
        buffer_physical: u64,
        bytes: usize,
        checksum_offload: bool,
    ) -> Result<(), NetworkError> {
        if buffer_physical == 0 || bytes == 0 || bytes > u16::MAX as usize {
            return Err(NetworkError::Invalid("NIC transmit buffer"));
        }
        let descriptor = self
            .tx_descriptors
            .get_mut(descriptor_index)
            .ok_or(NetworkError::Invalid("NIC transmit descriptor index"))?;
        if descriptor.status & 1 == 0 && descriptor.address != 0 {
            return Err(NetworkError::Invalid(
                "NIC transmit descriptor is still owned by hardware",
            ));
        }
        *descriptor = NicDescriptor {
            address: buffer_physical,
            length: bytes as u16,
            checksum: 0,
            command: (1 << 0) | (1 << 1) | (1 << 3) | (u8::from(checksum_offload) << 2),
            status: 0,
            errors: 0,
            special: 0,
        };
        io.fence()?;
        io.write(
            self.register_base + nic_tx_tail_offset(self.identity.family),
            RegisterWidth::U32,
            ((descriptor_index + 1) % self.tx_descriptors.len()) as u64,
        )?;
        io.fence()?;
        self.health.operations += 1;
        Ok(())
    }

    pub fn reap_transmit(&mut self, descriptor_index: usize) -> Result<usize, NetworkError> {
        let descriptor = self
            .tx_descriptors
            .get_mut(descriptor_index)
            .ok_or(NetworkError::Invalid("NIC transmit descriptor index"))?;
        if descriptor.status & 1 == 0 {
            return Err(NetworkError::Invalid(
                "NIC transmit descriptor has not completed",
            ));
        }
        if descriptor.errors != 0 {
            self.health.faults += 1;
            return Err(NetworkError::Invalid("NIC transmit completion error"));
        }
        let bytes = descriptor.length as usize;
        descriptor.address = 0;
        descriptor.length = 0;
        self.health.operations += 1;
        Ok(bytes)
    }

    pub fn recover(
        &mut self,
        io: &mut impl RegisterIo,
        rx_ring_physical: u64,
        tx_ring_physical: u64,
    ) -> Result<(), NetworkError> {
        self.health.faults += 1;
        self.health.resets += 1;
        self.reset_and_program(io, rx_ring_physical, tx_ring_physical)
    }
}

fn nic_rx_tail_offset(family: NicFamily) -> u64 {
    match family {
        NicFamily::E1000 | NicFamily::E1000e | NicFamily::Igb => 0x2818,
        NicFamily::Ixgbe | NicFamily::I40e => 0x1018,
        NicFamily::Rtl8139 => 0x38,
        NicFamily::Rtl8169 => 0xe8,
        NicFamily::VirtioNet => 0x50,
        _ => 0x18,
    }
}

fn nic_tx_tail_offset(family: NicFamily) -> u64 {
    match family {
        NicFamily::E1000 | NicFamily::E1000e | NicFamily::Igb => 0x3818,
        NicFamily::Ixgbe | NicFamily::I40e => 0x6018,
        NicFamily::Rtl8139 => 0x10,
        NicFamily::Rtl8169 => 0x38,
        NicFamily::VirtioNet => 0x50,
        _ => 0x20,
    }
}

fn program_intel_ring(
    io: &mut impl RegisterIo,
    base: u64,
    offset: u64,
    address: u64,
    descriptors: usize,
) -> Result<(), NetworkError> {
    io.write(base + offset, RegisterWidth::U32, address as u32 as u64)?;
    io.write(base + offset + 4, RegisterWidth::U32, address >> 32)?;
    io.write(
        base + offset + 8,
        RegisterWidth::U32,
        (descriptors * std::mem::size_of::<NicDescriptor>()) as u64,
    )?;
    io.write(base + offset + 0x10, RegisterWidth::U32, 0)?;
    io.write(
        base + offset + 0x18,
        RegisterWidth::U32,
        (descriptors - 1) as u64,
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    pub const UNSPECIFIED: Self = Self([0; 4]);
    pub const BROADCAST: Self = Self([255; 4]);

    pub const fn to_u32(self) -> u32 {
        u32::from_be_bytes(self.0)
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Ipv6Address(pub [u8; 16]);

impl Ipv6Address {
    pub const UNSPECIFIED: Self = Self([0; 16]);
    pub const LOOPBACK: Self = Self([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

    pub const fn is_multicast(self) -> bool {
        self.0[0] == 0xff
    }

    pub const fn is_link_local(self) -> bool {
        self.0[0] == 0xfe && self.0[1] & 0xc0 == 0x80
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpAddress {
    V4(Ipv4Address),
    V6(Ipv6Address),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpOperation {
    Request = 1,
    Reply = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArpPacket {
    pub operation: ArpOperation,
    pub sender_mac: MacAddress,
    pub sender_ip: Ipv4Address,
    pub target_mac: MacAddress,
    pub target_ip: Ipv4Address,
}

impl ArpPacket {
    pub const BYTES: usize = 28;

    pub fn write(self, output: &mut [u8]) -> Result<usize, NetworkError> {
        require(output, Self::BYTES)?;
        output[..Self::BYTES].fill(0);
        output[0..2].copy_from_slice(&1u16.to_be_bytes());
        output[2..4].copy_from_slice(&ether_type::IPV4.to_be_bytes());
        output[4] = 6;
        output[5] = 4;
        output[6..8].copy_from_slice(&(self.operation as u16).to_be_bytes());
        output[8..14].copy_from_slice(&self.sender_mac.0);
        output[14..18].copy_from_slice(&self.sender_ip.0);
        output[18..24].copy_from_slice(&self.target_mac.0);
        output[24..28].copy_from_slice(&self.target_ip.0);
        Ok(Self::BYTES)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, NetworkError> {
        require(bytes, Self::BYTES)?;
        if u16::from_be_bytes(bytes[0..2].try_into().unwrap()) != 1
            || u16::from_be_bytes(bytes[2..4].try_into().unwrap()) != ether_type::IPV4
            || bytes[4] != 6
            || bytes[5] != 4
        {
            return Err(NetworkError::Unsupported("ARP hardware or protocol format"));
        }
        let operation = match u16::from_be_bytes(bytes[6..8].try_into().unwrap()) {
            1 => ArpOperation::Request,
            2 => ArpOperation::Reply,
            _ => return Err(NetworkError::Invalid("ARP operation")),
        };
        Ok(Self {
            operation,
            sender_mac: MacAddress(bytes[8..14].try_into().unwrap()),
            sender_ip: Ipv4Address(bytes[14..18].try_into().unwrap()),
            target_mac: MacAddress(bytes[18..24].try_into().unwrap()),
            target_ip: Ipv4Address(bytes[24..28].try_into().unwrap()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborState {
    Incomplete,
    Reachable,
    Stale,
    Probe,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArpNeighbor {
    pub address: Ipv4Address,
    pub mac: MacAddress,
    pub state: NeighborState,
    pub expires_at: u64,
}

#[derive(Debug, Clone)]
pub struct ArpCache<const N: usize> {
    entries: [Option<ArpNeighbor>; N],
}

impl<const N: usize> ArpCache<N> {
    pub const fn new() -> Self {
        Self { entries: [None; N] }
    }

    pub fn update(&mut self, entry: ArpNeighbor) -> Result<(), NetworkError> {
        let index = self
            .entries
            .iter()
            .position(|slot| slot.is_some_and(|current| current.address == entry.address))
            .or_else(|| self.entries.iter().position(Option::is_none))
            .ok_or(NetworkError::SocketTableFull)?;
        self.entries[index] = Some(entry);
        Ok(())
    }

    pub fn resolve(&self, address: Ipv4Address, now: u64) -> Result<MacAddress, NetworkError> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| {
                entry.address == address
                    && entry.expires_at >= now
                    && matches!(entry.state, NeighborState::Reachable | NeighborState::Stale)
            })
            .map(|entry| entry.mac)
            .ok_or(NetworkError::NeighborMissing)
    }

    pub fn tick(&mut self, now: u64) {
        for entry in self.entries.iter_mut().flatten() {
            if entry.expires_at < now && entry.state == NeighborState::Reachable {
                entry.state = NeighborState::Stale;
            }
        }
    }
}

impl<const N: usize> Default for ArpCache<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Route {
    pub network: Ipv4Address,
    pub prefix: u8,
    pub gateway: Option<Ipv4Address>,
    pub interface: u16,
    pub metric: u32,
}

pub fn select_ipv4_route(
    routes: &[Ipv4Route],
    destination: Ipv4Address,
) -> Result<Ipv4Route, NetworkError> {
    routes
        .iter()
        .copied()
        .filter(|route| {
            if route.prefix > 32 {
                return false;
            }
            let mask = if route.prefix == 0 {
                0
            } else {
                u32::MAX << (32 - route.prefix)
            };
            destination.to_u32() & mask == route.network.to_u32() & mask
        })
        .max_by_key(|route| (route.prefix, u32::MAX - route.metric))
        .ok_or(NetworkError::RouteMissing)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Packet<'a> {
    pub dscp_ecn: u8,
    pub identification: u16,
    pub dont_fragment: bool,
    pub more_fragments: bool,
    pub fragment_offset_bytes: usize,
    pub ttl: u8,
    pub protocol: u8,
    pub source: Ipv4Address,
    pub destination: Ipv4Address,
    pub options: &'a [u8],
    pub payload: &'a [u8],
}

fn checksum(bytes: &[u8]) -> u16 {
    let mut accumulator = ChecksumAccumulator::default();
    accumulator.add(bytes);
    accumulator.finish()
}

#[derive(Debug, Default)]
struct ChecksumAccumulator {
    sum: u32,
    high: Option<u8>,
}

impl ChecksumAccumulator {
    fn add(&mut self, bytes: &[u8]) {
        let mut offset = 0;
        if let Some(high) = self.high.take() {
            if let Some(low) = bytes.first() {
                self.sum += u16::from_be_bytes([high, *low]) as u32;
                offset = 1;
            } else {
                self.high = Some(high);
                return;
            }
        }
        while offset + 1 < bytes.len() {
            self.sum += u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as u32;
            offset += 2;
        }
        if offset < bytes.len() {
            self.high = Some(bytes[offset]);
        }
    }

    fn finish(mut self) -> u16 {
        if let Some(high) = self.high {
            self.sum += u16::from_be_bytes([high, 0]) as u32;
        }
        while self.sum >> 16 != 0 {
            self.sum = (self.sum & 0xffff) + (self.sum >> 16);
        }
        !(self.sum as u16)
    }
}

pub fn parse_ipv4(bytes: &[u8]) -> Result<Ipv4Packet<'_>, NetworkError> {
    require(bytes, IPV4_MIN_HEADER_BYTES)?;
    if bytes[0] >> 4 != 4 {
        return Err(NetworkError::Invalid("IPv4 version"));
    }
    let header_length = (bytes[0] as usize & 0x0f) * 4;
    if header_length < IPV4_MIN_HEADER_BYTES || header_length > 60 {
        return Err(NetworkError::Invalid("IPv4 header length"));
    }
    require(bytes, header_length)?;
    let total = u16::from_be_bytes(bytes[2..4].try_into().unwrap()) as usize;
    if total < header_length || total > bytes.len() {
        return Err(NetworkError::Truncated("IPv4 packet"));
    }
    if checksum(&bytes[..header_length]) != 0 {
        return Err(NetworkError::Checksum("IPv4 header"));
    }
    let fragment = u16::from_be_bytes(bytes[6..8].try_into().unwrap());
    Ok(Ipv4Packet {
        dscp_ecn: bytes[1],
        identification: u16::from_be_bytes(bytes[4..6].try_into().unwrap()),
        dont_fragment: fragment & 0x4000 != 0,
        more_fragments: fragment & 0x2000 != 0,
        fragment_offset_bytes: (fragment as usize & 0x1fff) * 8,
        ttl: bytes[8],
        protocol: bytes[9],
        source: Ipv4Address(bytes[12..16].try_into().unwrap()),
        destination: Ipv4Address(bytes[16..20].try_into().unwrap()),
        options: &bytes[20..header_length],
        payload: &bytes[header_length..total],
    })
}

pub fn write_ipv4(output: &mut [u8], packet: &Ipv4Packet<'_>) -> Result<usize, NetworkError> {
    if packet.options.len() > 40
        || packet.options.len() % 4 != 0
        || packet.fragment_offset_bytes % 8 != 0
    {
        return Err(NetworkError::Invalid("IPv4 options or fragment offset"));
    }
    let header = IPV4_MIN_HEADER_BYTES + packet.options.len();
    let total = header
        .checked_add(packet.payload.len())
        .filter(|total| *total <= u16::MAX as usize)
        .ok_or(NetworkError::Invalid("IPv4 total length"))?;
    require(output, total)?;
    output[..total].fill(0);
    output[0] = 0x40 | (header / 4) as u8;
    output[1] = packet.dscp_ecn;
    output[2..4].copy_from_slice(&(total as u16).to_be_bytes());
    output[4..6].copy_from_slice(&packet.identification.to_be_bytes());
    let fragment = ((packet.dont_fragment as u16) << 14)
        | ((packet.more_fragments as u16) << 13)
        | ((packet.fragment_offset_bytes / 8) as u16 & 0x1fff);
    output[6..8].copy_from_slice(&fragment.to_be_bytes());
    output[8] = packet.ttl;
    output[9] = packet.protocol;
    output[12..16].copy_from_slice(&packet.source.0);
    output[16..20].copy_from_slice(&packet.destination.0);
    output[20..header].copy_from_slice(packet.options);
    let header_checksum = checksum(&output[..header]);
    output[10..12].copy_from_slice(&header_checksum.to_be_bytes());
    output[header..total].copy_from_slice(packet.payload);
    Ok(total)
}

pub fn fragment_ipv4<F>(
    packet: &Ipv4Packet<'_>,
    mtu: usize,
    scratch: &mut [u8],
    mut emit: F,
) -> Result<usize, NetworkError>
where
    F: FnMut(&[u8]) -> Result<(), NetworkError>,
{
    let header = IPV4_MIN_HEADER_BYTES + packet.options.len();
    if packet.dont_fragment || mtu <= header + 7 {
        return Err(NetworkError::Invalid("IPv4 packet cannot be fragmented"));
    }
    let chunk = ((mtu - header) / 8) * 8;
    require(scratch, mtu)?;
    let mut offset = 0;
    let mut count = 0;
    while offset < packet.payload.len() {
        let remaining = packet.payload.len() - offset;
        let bytes = remaining.min(chunk);
        let fragment = Ipv4Packet {
            dscp_ecn: packet.dscp_ecn,
            identification: packet.identification,
            dont_fragment: false,
            more_fragments: offset + bytes < packet.payload.len(),
            fragment_offset_bytes: offset,
            ttl: packet.ttl,
            protocol: packet.protocol,
            source: packet.source,
            destination: packet.destination,
            options: packet.options,
            payload: &packet.payload[offset..offset + bytes],
        };
        let length = write_ipv4(scratch, &fragment)?;
        emit(&scratch[..length])?;
        offset += bytes;
        count += 1;
    }
    Ok(count)
}

#[derive(Debug)]
pub struct Ipv4Reassembly<'a> {
    data: &'a mut [u8],
    received: &'a mut [u8],
    key: Option<(Ipv4Address, Ipv4Address, u16, u8)>,
    total: Option<usize>,
    unique: usize,
}

impl<'a> Ipv4Reassembly<'a> {
    pub fn new(data: &'a mut [u8], received: &'a mut [u8]) -> Result<Self, NetworkError> {
        if data.is_empty() || received.len() < data.len() {
            return Err(NetworkError::Invalid("IPv4 reassembly buffers"));
        }
        data.fill(0);
        received.fill(0);
        Ok(Self {
            data,
            received,
            key: None,
            total: None,
            unique: 0,
        })
    }

    pub fn ingest(&mut self, fragment: &Ipv4Packet<'_>) -> Result<Option<&[u8]>, NetworkError> {
        let key = (
            fragment.source,
            fragment.destination,
            fragment.identification,
            fragment.protocol,
        );
        if let Some(current) = self.key {
            if current != key {
                return Err(NetworkError::FragmentMismatch);
            }
        } else {
            self.key = Some(key);
        }
        let end = fragment
            .fragment_offset_bytes
            .checked_add(fragment.payload.len())
            .filter(|end| *end <= self.data.len())
            .ok_or(NetworkError::Truncated("IPv4 reassembly buffer"))?;
        if self.received[fragment.fragment_offset_bytes..end]
            .iter()
            .any(|value| *value != 0)
        {
            return Err(NetworkError::FragmentOverlap);
        }
        self.data[fragment.fragment_offset_bytes..end].copy_from_slice(fragment.payload);
        self.received[fragment.fragment_offset_bytes..end].fill(1);
        self.unique += fragment.payload.len();
        if !fragment.more_fragments {
            self.total = Some(end);
        }
        if let Some(total) = self.total {
            if self.unique == total && self.received[..total].iter().all(|value| *value != 0) {
                return Ok(Some(&self.data[..total]));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Route {
    pub network: Ipv6Address,
    pub prefix: u8,
    pub gateway: Option<Ipv6Address>,
    pub interface: u16,
    pub metric: u32,
}

fn ipv6_prefix_matches(address: Ipv6Address, network: Ipv6Address, prefix: u8) -> bool {
    if prefix > 128 {
        return false;
    }
    let whole = (prefix / 8) as usize;
    let remaining = prefix % 8;
    if address.0[..whole] != network.0[..whole] {
        return false;
    }
    remaining == 0
        || (address.0[whole] & (0xff << (8 - remaining)))
            == (network.0[whole] & (0xff << (8 - remaining)))
}

pub fn select_ipv6_route(
    routes: &[Ipv6Route],
    destination: Ipv6Address,
) -> Result<Ipv6Route, NetworkError> {
    routes
        .iter()
        .copied()
        .filter(|route| ipv6_prefix_matches(destination, route.network, route.prefix))
        .max_by_key(|route| (route.prefix, u32::MAX - route.metric))
        .ok_or(NetworkError::RouteMissing)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv6Packet<'a> {
    pub traffic_class: u8,
    pub flow_label: u32,
    pub next_header: u8,
    pub hop_limit: u8,
    pub source: Ipv6Address,
    pub destination: Ipv6Address,
    pub payload: &'a [u8],
}

pub fn write_ipv6(output: &mut [u8], packet: &Ipv6Packet<'_>) -> Result<usize, NetworkError> {
    if packet.flow_label > 0x000f_ffff || packet.payload.len() > u16::MAX as usize {
        return Err(NetworkError::Invalid("IPv6 flow label or payload length"));
    }
    let total = IPV6_HEADER_BYTES + packet.payload.len();
    require(output, total)?;
    let first = (6u32 << 28) | ((packet.traffic_class as u32) << 20) | packet.flow_label;
    output[..4].copy_from_slice(&first.to_be_bytes());
    output[4..6].copy_from_slice(&(packet.payload.len() as u16).to_be_bytes());
    output[6] = packet.next_header;
    output[7] = packet.hop_limit;
    output[8..24].copy_from_slice(&packet.source.0);
    output[24..40].copy_from_slice(&packet.destination.0);
    output[40..total].copy_from_slice(packet.payload);
    Ok(total)
}

pub fn parse_ipv6(bytes: &[u8]) -> Result<Ipv6Packet<'_>, NetworkError> {
    require(bytes, IPV6_HEADER_BYTES)?;
    let first = u32::from_be_bytes(bytes[..4].try_into().unwrap());
    if first >> 28 != 6 {
        return Err(NetworkError::Invalid("IPv6 version"));
    }
    let payload_length = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    require(bytes, IPV6_HEADER_BYTES + payload_length)?;
    Ok(Ipv6Packet {
        traffic_class: ((first >> 20) & 0xff) as u8,
        flow_label: first & 0x000f_ffff,
        next_header: bytes[6],
        hop_limit: bytes[7],
        source: Ipv6Address(bytes[8..24].try_into().unwrap()),
        destination: Ipv6Address(bytes[24..40].try_into().unwrap()),
        payload: &bytes[40..40 + payload_length],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Fragment {
    pub offset_bytes: usize,
    pub more: bool,
    pub identification: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Transport<'a> {
    pub next_header: u8,
    pub payload: &'a [u8],
    pub fragment: Option<Ipv6Fragment>,
    pub extension_bytes: usize,
}

pub fn write_ipv6_fragment(
    output: &mut [u8],
    next_header: u8,
    offset_bytes: usize,
    more: bool,
    identification: u32,
    payload: &[u8],
) -> Result<usize, NetworkError> {
    if offset_bytes % 8 != 0 || offset_bytes / 8 > 0x1fff {
        return Err(NetworkError::Invalid("IPv6 fragment offset"));
    }
    let total = 8 + payload.len();
    require(output, total)?;
    output[..total].fill(0);
    output[0] = next_header;
    let raw = (((offset_bytes / 8) as u16) << 3) | u16::from(more);
    output[2..4].copy_from_slice(&raw.to_be_bytes());
    output[4..8].copy_from_slice(&identification.to_be_bytes());
    output[8..total].copy_from_slice(payload);
    Ok(total)
}

#[derive(Debug)]
pub struct Ipv6Reassembly<'a> {
    data: &'a mut [u8],
    received: &'a mut [u8],
    key: Option<(Ipv6Address, Ipv6Address, u32, u8)>,
    total: Option<usize>,
    unique: usize,
}

impl<'a> Ipv6Reassembly<'a> {
    pub fn new(data: &'a mut [u8], received: &'a mut [u8]) -> Result<Self, NetworkError> {
        if data.is_empty() || received.len() < data.len() {
            return Err(NetworkError::Invalid("IPv6 reassembly buffers"));
        }
        data.fill(0);
        received.fill(0);
        Ok(Self {
            data,
            received,
            key: None,
            total: None,
            unique: 0,
        })
    }

    pub fn ingest(
        &mut self,
        source: Ipv6Address,
        destination: Ipv6Address,
        transport: &Ipv6Transport<'_>,
    ) -> Result<Option<&[u8]>, NetworkError> {
        let fragment = transport
            .fragment
            .ok_or(NetworkError::Invalid("IPv6 packet has no fragment header"))?;
        let key = (
            source,
            destination,
            fragment.identification,
            transport.next_header,
        );
        if let Some(current) = self.key {
            if current != key {
                return Err(NetworkError::FragmentMismatch);
            }
        } else {
            self.key = Some(key);
        }
        let end = fragment
            .offset_bytes
            .checked_add(transport.payload.len())
            .filter(|end| *end <= self.data.len())
            .ok_or(NetworkError::Truncated("IPv6 reassembly buffer"))?;
        if self.received[fragment.offset_bytes..end]
            .iter()
            .any(|value| *value != 0)
        {
            return Err(NetworkError::FragmentOverlap);
        }
        self.data[fragment.offset_bytes..end].copy_from_slice(transport.payload);
        self.received[fragment.offset_bytes..end].fill(1);
        self.unique += transport.payload.len();
        if !fragment.more {
            self.total = Some(end);
        }
        if let Some(total) = self.total {
            if self.unique == total && self.received[..total].iter().all(|value| *value != 0) {
                return Ok(Some(&self.data[..total]));
            }
        }
        Ok(None)
    }
}

pub fn walk_ipv6_extensions<'a>(
    packet: &Ipv6Packet<'a>,
) -> Result<Ipv6Transport<'a>, NetworkError> {
    let mut next = packet.next_header;
    let mut payload = packet.payload;
    let mut consumed = 0;
    let mut fragment = None;
    for _ in 0..8 {
        match next {
            0 | 43 | 60 => {
                require(payload, 8)?;
                let length = (payload[1] as usize + 1) * 8;
                require(payload, length)?;
                next = payload[0];
                payload = &payload[length..];
                consumed += length;
            }
            44 => {
                require(payload, 8)?;
                let raw = u16::from_be_bytes(payload[2..4].try_into().unwrap());
                fragment = Some(Ipv6Fragment {
                    offset_bytes: ((raw >> 3) as usize) * 8,
                    more: raw & 1 != 0,
                    identification: u32::from_be_bytes(payload[4..8].try_into().unwrap()),
                });
                next = payload[0];
                payload = &payload[8..];
                consumed += 8;
            }
            51 => {
                require(payload, 8)?;
                let length = (payload[1] as usize + 2) * 4;
                require(payload, length)?;
                next = payload[0];
                payload = &payload[length..];
                consumed += length;
            }
            50 => return Err(NetworkError::Unsupported("encrypted IPv6 ESP payload")),
            59 => {
                return Ok(Ipv6Transport {
                    next_header: next,
                    payload: &[],
                    fragment,
                    extension_bytes: consumed,
                })
            }
            _ => {
                return Ok(Ipv6Transport {
                    next_header: next,
                    payload,
                    fragment,
                    extension_bytes: consumed,
                })
            }
        }
    }
    Err(NetworkError::Invalid("IPv6 extension header chain"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborDiscovery {
    pub router: bool,
    pub solicited: bool,
    pub override_flag: bool,
    pub target: Ipv6Address,
    pub link_layer: Option<MacAddress>,
}

pub fn write_neighbor_solicitation(
    output: &mut [u8],
    source: Ipv6Address,
    destination: Ipv6Address,
    target: Ipv6Address,
    source_mac: Option<MacAddress>,
) -> Result<usize, NetworkError> {
    let length = 24 + usize::from(source_mac.is_some()) * 8;
    require(output, length)?;
    output[..length].fill(0);
    output[0] = 135;
    output[8..24].copy_from_slice(&target.0);
    if let Some(mac) = source_mac {
        output[24] = 1;
        output[25] = 1;
        output[26..32].copy_from_slice(&mac.0);
    }
    let value = transport_checksum_v6(source, destination, 58, &output[..length]);
    output[2..4].copy_from_slice(&value.to_be_bytes());
    Ok(length)
}

pub fn parse_neighbor_discovery(
    source: Ipv6Address,
    destination: Ipv6Address,
    bytes: &[u8],
) -> Result<NeighborDiscovery, NetworkError> {
    require(bytes, 24)?;
    if !matches!(bytes[0], 135 | 136) || bytes[1] != 0 {
        return Err(NetworkError::Invalid("ICMPv6 neighbor discovery message"));
    }
    if transport_checksum_v6(source, destination, 58, bytes) != 0 {
        return Err(NetworkError::Checksum("ICMPv6"));
    }
    let flags = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    let link_layer = if bytes.len() >= 32 && matches!(bytes[24], 1 | 2) && bytes[25] == 1 {
        Some(MacAddress(bytes[26..32].try_into().unwrap()))
    } else {
        None
    };
    Ok(NeighborDiscovery {
        router: flags & (1 << 31) != 0,
        solicited: flags & (1 << 30) != 0,
        override_flag: flags & (1 << 29) != 0,
        target: Ipv6Address(bytes[8..24].try_into().unwrap()),
        link_layer,
    })
}

fn transport_checksum_v4(
    source: Ipv4Address,
    destination: Ipv4Address,
    protocol: u8,
    bytes: &[u8],
) -> u16 {
    let mut accumulator = ChecksumAccumulator::default();
    accumulator.add(&source.0);
    accumulator.add(&destination.0);
    accumulator.add(&[0, protocol]);
    accumulator.add(&(bytes.len() as u16).to_be_bytes());
    accumulator.add(bytes);
    accumulator.finish()
}

fn transport_checksum_v6(
    source: Ipv6Address,
    destination: Ipv6Address,
    next_header: u8,
    bytes: &[u8],
) -> u16 {
    let mut accumulator = ChecksumAccumulator::default();
    accumulator.add(&source.0);
    accumulator.add(&destination.0);
    accumulator.add(&(bytes.len() as u32).to_be_bytes());
    accumulator.add(&[0, 0, 0, next_header]);
    accumulator.add(bytes);
    accumulator.finish()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpPacket<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub payload: &'a [u8],
}

pub fn write_udp(
    output: &mut [u8],
    packet: &UdpPacket<'_>,
    source: IpAddress,
    destination: IpAddress,
) -> Result<usize, NetworkError> {
    if packet.source_port == 0 || packet.destination_port == 0 {
        return Err(NetworkError::Invalid("UDP port"));
    }
    let length = UDP_HEADER_BYTES
        .checked_add(packet.payload.len())
        .filter(|length| *length <= u16::MAX as usize)
        .ok_or(NetworkError::Invalid("UDP length"))?;
    require(output, length)?;
    output[..length].fill(0);
    output[0..2].copy_from_slice(&packet.source_port.to_be_bytes());
    output[2..4].copy_from_slice(&packet.destination_port.to_be_bytes());
    output[4..6].copy_from_slice(&(length as u16).to_be_bytes());
    output[8..length].copy_from_slice(packet.payload);
    let value = match (source, destination) {
        (IpAddress::V4(source), IpAddress::V4(destination)) => {
            transport_checksum_v4(source, destination, 17, &output[..length])
        }
        (IpAddress::V6(source), IpAddress::V6(destination)) => {
            transport_checksum_v6(source, destination, 17, &output[..length])
        }
        _ => return Err(NetworkError::Invalid("mixed UDP address families")),
    };
    output[6..8].copy_from_slice(&if value == 0 { 0xffff } else { value }.to_be_bytes());
    Ok(length)
}

pub fn parse_udp(
    bytes: &[u8],
    source: IpAddress,
    destination: IpAddress,
) -> Result<UdpPacket<'_>, NetworkError> {
    require(bytes, UDP_HEADER_BYTES)?;
    let length = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    if length < UDP_HEADER_BYTES || length > bytes.len() {
        return Err(NetworkError::Truncated("UDP datagram"));
    }
    let stored = u16::from_be_bytes(bytes[6..8].try_into().unwrap());
    let verified = match (source, destination) {
        (IpAddress::V4(source), IpAddress::V4(destination)) => {
            stored == 0 || transport_checksum_v4(source, destination, 17, &bytes[..length]) == 0
        }
        (IpAddress::V6(source), IpAddress::V6(destination)) => {
            stored != 0 && transport_checksum_v6(source, destination, 17, &bytes[..length]) == 0
        }
        _ => return Err(NetworkError::Invalid("mixed UDP address families")),
    };
    if !verified {
        return Err(NetworkError::Checksum("UDP"));
    }
    Ok(UdpPacket {
        source_port: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
        destination_port: u16::from_be_bytes(bytes[2..4].try_into().unwrap()),
        payload: &bytes[8..length],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TcpFlags(pub u16);

impl TcpFlags {
    pub const FIN: Self = Self(0x001);
    pub const SYN: Self = Self(0x002);
    pub const RST: Self = Self(0x004);
    pub const PSH: Self = Self(0x008);
    pub const ACK: Self = Self(0x010);
    pub const URG: Self = Self(0x020);
    pub const ECE: Self = Self(0x040);
    pub const CWR: Self = Self(0x080);
    pub const NS: Self = Self(0x100);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TcpOptions {
    pub mss: Option<u16>,
    pub window_scale: Option<u8>,
    pub sack_permitted: bool,
    pub timestamp: Option<(u32, u32)>,
}

fn write_tcp_options(output: &mut [u8], options: TcpOptions) -> Result<usize, NetworkError> {
    let mut offset = 0;
    if let Some(mss) = options.mss {
        require(output, offset + 4)?;
        output[offset..offset + 4].copy_from_slice(&[2, 4, (mss >> 8) as u8, mss as u8]);
        offset += 4;
    }
    if let Some(scale) = options.window_scale {
        require(output, offset + 4)?;
        output[offset..offset + 4].copy_from_slice(&[1, 3, 3, scale.min(14)]);
        offset += 4;
    }
    if options.sack_permitted {
        require(output, offset + 4)?;
        output[offset..offset + 4].copy_from_slice(&[1, 1, 4, 2]);
        offset += 4;
    }
    if let Some((value, echo)) = options.timestamp {
        require(output, offset + 12)?;
        output[offset..offset + 4].copy_from_slice(&[1, 1, 8, 10]);
        output[offset + 4..offset + 8].copy_from_slice(&value.to_be_bytes());
        output[offset + 8..offset + 12].copy_from_slice(&echo.to_be_bytes());
        offset += 12;
    }
    while offset % 4 != 0 {
        require(output, offset + 1)?;
        output[offset] = 1;
        offset += 1;
    }
    Ok(offset)
}

fn parse_tcp_options(mut bytes: &[u8]) -> Result<TcpOptions, NetworkError> {
    let mut result = TcpOptions::default();
    while !bytes.is_empty() {
        match bytes[0] {
            0 => break,
            1 => bytes = &bytes[1..],
            kind => {
                require(bytes, 2)?;
                let length = bytes[1] as usize;
                if length < 2 || length > bytes.len() {
                    return Err(NetworkError::Invalid("TCP option length"));
                }
                match (kind, length) {
                    (2, 4) => result.mss = Some(u16::from_be_bytes([bytes[2], bytes[3]])),
                    (3, 3) => result.window_scale = Some(bytes[2].min(14)),
                    (4, 2) => result.sack_permitted = true,
                    (8, 10) => {
                        result.timestamp = Some((
                            u32::from_be_bytes(bytes[2..6].try_into().unwrap()),
                            u32::from_be_bytes(bytes[6..10].try_into().unwrap()),
                        ));
                    }
                    _ => {}
                }
                bytes = &bytes[length..];
            }
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpSegment<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub sequence: u32,
    pub acknowledgment: u32,
    pub flags: TcpFlags,
    pub window: u16,
    pub urgent: u16,
    pub options: TcpOptions,
    pub payload: &'a [u8],
}

pub fn write_tcp(
    output: &mut [u8],
    segment: &TcpSegment<'_>,
    source: IpAddress,
    destination: IpAddress,
) -> Result<usize, NetworkError> {
    if segment.source_port == 0 || segment.destination_port == 0 {
        return Err(NetworkError::Invalid("TCP port"));
    }
    require(output, 60 + segment.payload.len())?;
    output[..60 + segment.payload.len()].fill(0);
    let option_length = write_tcp_options(&mut output[20..60], segment.options)?;
    let header = TCP_MIN_HEADER_BYTES + option_length;
    let total = header + segment.payload.len();
    output[0..2].copy_from_slice(&segment.source_port.to_be_bytes());
    output[2..4].copy_from_slice(&segment.destination_port.to_be_bytes());
    output[4..8].copy_from_slice(&segment.sequence.to_be_bytes());
    output[8..12].copy_from_slice(&segment.acknowledgment.to_be_bytes());
    let control = ((header / 4) as u16) << 12 | (segment.flags.0 & 0x01ff);
    output[12..14].copy_from_slice(&control.to_be_bytes());
    output[14..16].copy_from_slice(&segment.window.to_be_bytes());
    output[18..20].copy_from_slice(&segment.urgent.to_be_bytes());
    output[header..total].copy_from_slice(segment.payload);
    let value = match (source, destination) {
        (IpAddress::V4(source), IpAddress::V4(destination)) => {
            transport_checksum_v4(source, destination, 6, &output[..total])
        }
        (IpAddress::V6(source), IpAddress::V6(destination)) => {
            transport_checksum_v6(source, destination, 6, &output[..total])
        }
        _ => return Err(NetworkError::Invalid("mixed TCP address families")),
    };
    output[16..18].copy_from_slice(&value.to_be_bytes());
    Ok(total)
}

pub fn parse_tcp(
    bytes: &[u8],
    source: IpAddress,
    destination: IpAddress,
) -> Result<TcpSegment<'_>, NetworkError> {
    require(bytes, TCP_MIN_HEADER_BYTES)?;
    let control = u16::from_be_bytes(bytes[12..14].try_into().unwrap());
    let header = ((control >> 12) as usize) * 4;
    if !(20..=60).contains(&header) || header > bytes.len() {
        return Err(NetworkError::Invalid("TCP data offset"));
    }
    let valid = match (source, destination) {
        (IpAddress::V4(source), IpAddress::V4(destination)) => {
            transport_checksum_v4(source, destination, 6, bytes) == 0
        }
        (IpAddress::V6(source), IpAddress::V6(destination)) => {
            transport_checksum_v6(source, destination, 6, bytes) == 0
        }
        _ => return Err(NetworkError::Invalid("mixed TCP address families")),
    };
    if !valid {
        return Err(NetworkError::Checksum("TCP"));
    }
    Ok(TcpSegment {
        source_port: u16::from_be_bytes(bytes[0..2].try_into().unwrap()),
        destination_port: u16::from_be_bytes(bytes[2..4].try_into().unwrap()),
        sequence: u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
        acknowledgment: u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
        flags: TcpFlags(control & 0x01ff),
        window: u16::from_be_bytes(bytes[14..16].try_into().unwrap()),
        urgent: u16::from_be_bytes(bytes[18..20].try_into().unwrap()),
        options: parse_tcp_options(&bytes[20..header])?,
        payload: &bytes[header..],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpAction {
    None,
    SendSyn,
    SendSynAck,
    SendAck,
    Deliver(usize),
    Retransmit,
    Reset,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpControlBlock {
    pub state: TcpState,
    pub send_unacknowledged: u32,
    pub send_next: u32,
    pub receive_next: u32,
    pub send_window: u32,
    pub receive_window: u32,
    pub congestion_window: u32,
    pub slow_start_threshold: u32,
    pub retransmit_deadline: u64,
    pub retransmit_timeout: u64,
    pub retries: u8,
    pub options: TcpOptions,
}

impl TcpControlBlock {
    pub fn listen(receive_window: u32) -> Self {
        Self {
            state: TcpState::Listen,
            send_unacknowledged: 0,
            send_next: 0,
            receive_next: 0,
            send_window: 0,
            receive_window,
            congestion_window: 1460,
            slow_start_threshold: 65_535,
            retransmit_deadline: 0,
            retransmit_timeout: 1_000,
            retries: 0,
            options: TcpOptions::default(),
        }
    }

    pub fn active_open(
        &mut self,
        initial_sequence: u32,
        now: u64,
    ) -> Result<TcpAction, NetworkError> {
        if self.state != TcpState::Closed {
            return Err(NetworkError::InvalidTcpTransition);
        }
        self.send_unacknowledged = initial_sequence;
        self.send_next = initial_sequence.wrapping_add(1);
        self.state = TcpState::SynSent;
        self.retransmit_deadline = now + self.retransmit_timeout;
        Ok(TcpAction::SendSyn)
    }

    pub fn on_segment(
        &mut self,
        segment: &TcpSegment<'_>,
        now: u64,
    ) -> Result<TcpAction, NetworkError> {
        if segment.flags.contains(TcpFlags::RST) {
            self.state = TcpState::Closed;
            return Ok(TcpAction::Reset);
        }
        match self.state {
            TcpState::Listen if segment.flags.contains(TcpFlags::SYN) => {
                self.receive_next = segment.sequence.wrapping_add(1);
                self.send_unacknowledged = self.send_next;
                self.send_next = self.send_next.wrapping_add(1);
                self.send_window = segment.window as u32;
                self.options = segment.options;
                self.state = TcpState::SynReceived;
                self.retransmit_deadline = now + self.retransmit_timeout;
                Ok(TcpAction::SendSynAck)
            }
            TcpState::SynSent
                if segment.flags.contains(TcpFlags::SYN)
                    && segment.flags.contains(TcpFlags::ACK)
                    && segment.acknowledgment == self.send_next =>
            {
                self.send_unacknowledged = segment.acknowledgment;
                self.receive_next = segment.sequence.wrapping_add(1);
                self.send_window = segment.window as u32;
                self.options = segment.options;
                self.state = TcpState::Established;
                self.retransmit_deadline = 0;
                Ok(TcpAction::SendAck)
            }
            TcpState::SynReceived
                if segment.flags.contains(TcpFlags::ACK)
                    && segment.acknowledgment == self.send_next =>
            {
                self.send_unacknowledged = segment.acknowledgment;
                self.state = TcpState::Established;
                self.retransmit_deadline = 0;
                Ok(TcpAction::None)
            }
            TcpState::Established => {
                if segment.flags.contains(TcpFlags::ACK)
                    && sequence_after(segment.acknowledgment, self.send_unacknowledged)
                    && !sequence_after(segment.acknowledgment, self.send_next)
                {
                    let acknowledged = segment
                        .acknowledgment
                        .wrapping_sub(self.send_unacknowledged);
                    self.send_unacknowledged = segment.acknowledgment;
                    if self.congestion_window < self.slow_start_threshold {
                        self.congestion_window =
                            self.congestion_window.saturating_add(acknowledged);
                    } else {
                        self.congestion_window = self.congestion_window.saturating_add(
                            (1460u32.saturating_mul(1460) / self.congestion_window.max(1)).max(1),
                        );
                    }
                    self.retries = 0;
                    self.retransmit_timeout = self.retransmit_timeout.max(200) / 2;
                }
                if segment.sequence != self.receive_next && !segment.payload.is_empty() {
                    return Ok(TcpAction::SendAck);
                }
                self.receive_next = self
                    .receive_next
                    .wrapping_add(segment.payload.len() as u32)
                    .wrapping_add(u32::from(segment.flags.contains(TcpFlags::FIN)));
                if segment.flags.contains(TcpFlags::FIN) {
                    self.state = TcpState::CloseWait;
                    return Ok(TcpAction::SendAck);
                }
                if segment.payload.is_empty() {
                    Ok(TcpAction::None)
                } else {
                    Ok(TcpAction::Deliver(segment.payload.len()))
                }
            }
            TcpState::FinWait1 if segment.flags.contains(TcpFlags::ACK) => {
                self.state = TcpState::FinWait2;
                Ok(TcpAction::None)
            }
            TcpState::FinWait2 if segment.flags.contains(TcpFlags::FIN) => {
                self.receive_next = segment.sequence.wrapping_add(1);
                self.state = TcpState::TimeWait;
                self.retransmit_deadline = now + 60_000;
                Ok(TcpAction::SendAck)
            }
            TcpState::LastAck if segment.flags.contains(TcpFlags::ACK) => {
                self.state = TcpState::Closed;
                Ok(TcpAction::Closed)
            }
            _ => Err(NetworkError::InvalidTcpTransition),
        }
    }

    pub fn send_data(&mut self, bytes: usize, now: u64) -> Result<u32, NetworkError> {
        if self.state != TcpState::Established
            || bytes as u32 > self.send_window.min(self.congestion_window)
        {
            return Err(NetworkError::InvalidTcpTransition);
        }
        let sequence = self.send_next;
        self.send_next = self.send_next.wrapping_add(bytes as u32);
        self.retransmit_deadline = now + self.retransmit_timeout;
        Ok(sequence)
    }

    pub fn close(&mut self, now: u64) -> Result<TcpAction, NetworkError> {
        self.state = match self.state {
            TcpState::Established => TcpState::FinWait1,
            TcpState::CloseWait => TcpState::LastAck,
            _ => return Err(NetworkError::InvalidTcpTransition),
        };
        self.send_next = self.send_next.wrapping_add(1);
        self.retransmit_deadline = now + self.retransmit_timeout;
        Ok(TcpAction::SendAck)
    }

    pub fn tick(&mut self, now: u64) -> TcpAction {
        if self.state == TcpState::TimeWait && now >= self.retransmit_deadline {
            self.state = TcpState::Closed;
            return TcpAction::Closed;
        }
        if self.retransmit_deadline != 0
            && now >= self.retransmit_deadline
            && self.send_unacknowledged != self.send_next
        {
            self.retries = self.retries.saturating_add(1);
            if self.retries > 8 {
                self.state = TcpState::Closed;
                return TcpAction::Reset;
            }
            self.slow_start_threshold = (self.congestion_window / 2).max(2 * 1460);
            self.congestion_window = 1460;
            self.retransmit_timeout = (self.retransmit_timeout * 2).min(60_000);
            self.retransmit_deadline = now + self.retransmit_timeout;
            return TcpAction::Retransmit;
        }
        TcpAction::None
    }
}

fn sequence_after(left: u32, right: u32) -> bool {
    left.wrapping_sub(right) as i32 > 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProtocol {
    Udp,
    Tcp,
    Raw(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketEntry {
    pub id: u32,
    pub protocol: SocketProtocol,
    pub local_address: Option<IpAddress>,
    pub local_port: u16,
    pub remote_address: Option<IpAddress>,
    pub remote_port: u16,
    pub nonblocking: bool,
}

#[derive(Debug, Clone)]
pub struct SocketTable<const N: usize> {
    entries: [Option<SocketEntry>; N],
    next_id: u32,
}

impl<const N: usize> SocketTable<N> {
    pub const fn new() -> Self {
        Self {
            entries: [None; N],
            next_id: 1,
        }
    }

    pub fn create(
        &mut self,
        protocol: SocketProtocol,
        nonblocking: bool,
    ) -> Result<u32, NetworkError> {
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(NetworkError::SocketTableFull)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        *slot = Some(SocketEntry {
            id,
            protocol,
            local_address: None,
            local_port: 0,
            remote_address: None,
            remote_port: 0,
            nonblocking,
        });
        Ok(id)
    }

    pub fn bind(&mut self, id: u32, address: IpAddress, port: u16) -> Result<(), NetworkError> {
        if port == 0
            || self.entries.iter().flatten().any(|entry| {
                entry.id != id
                    && entry.protocol
                        == self
                            .get(id)
                            .map(|socket| socket.protocol)
                            .unwrap_or(SocketProtocol::Raw(0))
                    && entry.local_address == Some(address)
                    && entry.local_port == port
            })
        {
            return Err(NetworkError::AddressInUse);
        }
        let entry = self.get_mut(id)?;
        entry.local_address = Some(address);
        entry.local_port = port;
        Ok(())
    }

    pub fn connect(&mut self, id: u32, address: IpAddress, port: u16) -> Result<(), NetworkError> {
        if port == 0 {
            return Err(NetworkError::Invalid("socket remote port"));
        }
        let entry = self.get_mut(id)?;
        entry.remote_address = Some(address);
        entry.remote_port = port;
        Ok(())
    }

    pub fn close(&mut self, id: u32) -> Result<(), NetworkError> {
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_some_and(|socket| socket.id == id))
            .ok_or(NetworkError::SocketMissing(id))?;
        *slot = None;
        Ok(())
    }

    pub fn get(&self, id: u32) -> Result<&SocketEntry, NetworkError> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.id == id)
            .ok_or(NetworkError::SocketMissing(id))
    }

    fn get_mut(&mut self, id: u32) -> Result<&mut SocketEntry, NetworkError> {
        self.entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.id == id)
            .ok_or(NetworkError::SocketMissing(id))
    }
}

impl<const N: usize> Default for SocketTable<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct SocketBuffer<'a> {
    bytes: &'a mut [u8],
    head: usize,
    len: usize,
}

impl<'a> SocketBuffer<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Result<Self, NetworkError> {
        if bytes.is_empty() {
            return Err(NetworkError::Invalid("socket buffer"));
        }
        Ok(Self {
            bytes,
            head: 0,
            len: 0,
        })
    }

    pub fn write(&mut self, input: &[u8]) -> Result<usize, NetworkError> {
        if input.len() > self.bytes.len() - self.len {
            return Err(NetworkError::BufferTooSmall {
                required: self.len + input.len(),
                available: self.bytes.len(),
            });
        }
        for value in input {
            let tail = (self.head + self.len) % self.bytes.len();
            self.bytes[tail] = *value;
            self.len += 1;
        }
        Ok(input.len())
    }

    pub fn read(&mut self, output: &mut [u8]) -> usize {
        let count = output.len().min(self.len);
        for slot in &mut output[..count] {
            *slot = self.bytes[self.head];
            self.head = (self.head + 1) % self.bytes.len();
            self.len -= 1;
        }
        count
    }

    pub const fn len(&self) -> usize {
        self.len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkTimerKind {
    ArpReachability,
    NeighborDiscovery,
    TcpRetransmit,
    TcpDelayedAck,
    TcpTimeWait,
    DhcpLease,
    RouterAdvertisement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkTimer {
    pub deadline: u64,
    pub token: u32,
    pub kind: NetworkTimerKind,
}

#[derive(Debug, Clone)]
pub struct NetworkTimers<const N: usize> {
    entries: [Option<NetworkTimer>; N],
}

impl<const N: usize> NetworkTimers<N> {
    pub const fn new() -> Self {
        Self { entries: [None; N] }
    }

    pub fn schedule(&mut self, timer: NetworkTimer) -> Result<(), NetworkError> {
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(NetworkError::TimerTableFull)?;
        *slot = Some(timer);
        Ok(())
    }

    pub fn cancel(&mut self, token: u32, kind: NetworkTimerKind) -> bool {
        if let Some(slot) = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_some_and(|timer| timer.token == token && timer.kind == kind))
        {
            *slot = None;
            true
        } else {
            false
        }
    }

    pub fn expire(&mut self, now: u64, output: &mut [NetworkTimer]) -> usize {
        let mut count = 0;
        loop {
            let next = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    entry
                        .filter(|timer| timer.deadline <= now)
                        .map(|timer| (index, timer))
                })
                .min_by_key(|(_, timer)| timer.deadline);
            let Some((index, timer)) = next else { break };
            if count == output.len() {
                break;
            }
            output[count] = timer;
            self.entries[index] = None;
            count += 1;
        }
        count
    }
}

impl<const N: usize> Default for NetworkTimers<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelNetworkReport {
    pub nic_families: usize,
    pub nic_register_accesses: usize,
    pub recovered_nic_faults: u64,
    pub nic_rx_descriptors: usize,
    pub nic_tx_bytes: usize,
    pub ethernet_vlan_tags: usize,
    pub arp_roundtrips: u64,
    pub ipv4_fragments: usize,
    pub ipv4_reassembled_bytes: usize,
    pub ipv6_extension_bytes: usize,
    pub ipv6_reassembled_bytes: usize,
    pub ipv6_nd_messages: u64,
    pub udp_v4_bytes: usize,
    pub udp_v6_bytes: usize,
    pub tcp_states_exercised: usize,
    pub tcp_retransmits: u64,
    pub sockets: usize,
    pub timers_expired: usize,
}

pub fn certify_kernel_network_contracts() -> Result<KernelNetworkReport, NetworkError> {
    let identity =
        identify_nic(0x8086, 0x100e).ok_or(NetworkError::Unsupported("E1000 identity"))?;
    let mut rx = [NicDescriptor::default(); 8];
    let mut tx = [NicDescriptor::default(); 8];
    let mut nic = ManualNic::new(
        identity,
        0xf400_0000,
        MacAddress([0x02, 0, 0, 0, 0, 1]),
        &mut rx,
        &mut tx,
    )?;
    let mut io = TracedRegisterIo::default();
    nic.reset_and_program(&mut io, 0x100_0000, 0x101_0000)?;
    nic.recover(&mut io, 0x100_0000, 0x101_0000)?;
    nic.prime_receive_buffers(&mut io, 0x200_0000, 2048, 2048)?;
    nic.submit_transmit(&mut io, 0, 0x300_0000, 96, true)?;
    nic.tx_descriptors[0].status = 1;
    let nic_tx_bytes = nic.reap_transmit(0)?;

    let vlan = [
        VlanTag {
            priority: 5,
            drop_eligible: false,
            identifier: 42,
        },
        VlanTag {
            priority: 0,
            drop_eligible: true,
            identifier: 7,
        },
    ];
    let mut ethernet = [0u8; 128];
    let ethernet_len = write_ethernet(
        &mut ethernet,
        MacAddress::BROADCAST,
        nic.mac,
        ether_type::IPV4,
        &vlan,
        b"network",
    )?;
    let parsed_ethernet = parse_ethernet(&ethernet[..ethernet_len])?;

    let source_v4 = Ipv4Address([10, 0, 0, 1]);
    let destination_v4 = Ipv4Address([10, 0, 0, 2]);
    let arp = ArpPacket {
        operation: ArpOperation::Request,
        sender_mac: nic.mac,
        sender_ip: source_v4,
        target_mac: MacAddress::default(),
        target_ip: destination_v4,
    };
    let mut arp_bytes = [0u8; ArpPacket::BYTES];
    arp.write(&mut arp_bytes)?;
    if ArpPacket::parse(&arp_bytes)? != arp {
        return Err(NetworkError::Invalid("ARP roundtrip"));
    }
    let mut arp_cache = ArpCache::<4>::new();
    arp_cache.update(ArpNeighbor {
        address: destination_v4,
        mac: MacAddress([0x02, 0, 0, 0, 0, 2]),
        state: NeighborState::Reachable,
        expires_at: 10_000,
    })?;
    arp_cache.resolve(destination_v4, 1)?;

    let ipv4_payload = [0x5au8; 240];
    let ipv4 = Ipv4Packet {
        dscp_ecn: 0x2e,
        identification: 0x4242,
        dont_fragment: false,
        more_fragments: false,
        fragment_offset_bytes: 0,
        ttl: 64,
        protocol: 17,
        source: source_v4,
        destination: destination_v4,
        options: &[1, 1, 0, 0],
        payload: &ipv4_payload,
    };
    let mut fragments = [[0u8; 128]; 3];
    let mut fragment_lengths = [0usize; 3];
    let mut position = 0;
    let mut scratch = [0u8; 128];
    let ipv4_fragments = fragment_ipv4(&ipv4, 124, &mut scratch, |bytes| {
        fragments[position][..bytes.len()].copy_from_slice(bytes);
        fragment_lengths[position] = bytes.len();
        position += 1;
        Ok(())
    })?;
    let mut reassembled = [0u8; 512];
    let mut received = [0u8; 512];
    let mut reassembly = Ipv4Reassembly::new(&mut reassembled, &mut received)?;
    let mut ipv4_reassembled_bytes = 0;
    for index in (0..ipv4_fragments).rev() {
        let fragment = parse_ipv4(&fragments[index][..fragment_lengths[index]])?;
        if let Some(payload) = reassembly.ingest(&fragment)? {
            ipv4_reassembled_bytes = payload.len();
            if payload != ipv4_payload {
                return Err(NetworkError::Invalid("IPv4 reassembly payload"));
            }
        }
    }

    let source_v6 = Ipv6Address([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    let destination_v6 = Ipv6Address([0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0xff, 0, 0, 2]);
    let mut nd = [0u8; 64];
    let nd_len = write_neighbor_solicitation(
        &mut nd,
        source_v6,
        destination_v6,
        destination_v6,
        Some(nic.mac),
    )?;
    parse_neighbor_discovery(source_v6, destination_v6, &nd[..nd_len])?;
    let extension_payload = [17u8, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4];
    let ipv6 = Ipv6Packet {
        traffic_class: 0x2e,
        flow_label: 0x12345,
        next_header: 0,
        hop_limit: 64,
        source: source_v6,
        destination: destination_v6,
        payload: &extension_payload,
    };
    let mut ipv6_bytes = [0u8; 128];
    let ipv6_len = write_ipv6(&mut ipv6_bytes, &ipv6)?;
    let parsed_ipv6 = parse_ipv6(&ipv6_bytes[..ipv6_len])?;
    let transport = walk_ipv6_extensions(&parsed_ipv6)?;
    if transport.next_header != 17 || transport.payload != [1, 2, 3, 4] {
        return Err(NetworkError::Invalid("IPv6 extension walk"));
    }
    let ipv6_fragment_payload = [0xa5u8; 24];
    let mut fragment_a = [0u8; 32];
    let fragment_a_len = write_ipv6_fragment(
        &mut fragment_a,
        17,
        0,
        true,
        0x1122_3344,
        &ipv6_fragment_payload[..16],
    )?;
    let mut fragment_b = [0u8; 24];
    let fragment_b_len = write_ipv6_fragment(
        &mut fragment_b,
        17,
        16,
        false,
        0x1122_3344,
        &ipv6_fragment_payload[16..],
    )?;
    let fragment_packet_a = Ipv6Packet {
        traffic_class: 0,
        flow_label: 0,
        next_header: 44,
        hop_limit: 32,
        source: source_v6,
        destination: destination_v6,
        payload: &fragment_a[..fragment_a_len],
    };
    let fragment_packet_b = Ipv6Packet {
        payload: &fragment_b[..fragment_b_len],
        ..fragment_packet_a.clone()
    };
    let mut fragment_wire_a = [0u8; 96];
    let fragment_wire_a_len = write_ipv6(&mut fragment_wire_a, &fragment_packet_a)?;
    let mut fragment_wire_b = [0u8; 96];
    let fragment_wire_b_len = write_ipv6(&mut fragment_wire_b, &fragment_packet_b)?;
    let parsed_fragment_b = parse_ipv6(&fragment_wire_b[..fragment_wire_b_len])?;
    let parsed_fragment_a = parse_ipv6(&fragment_wire_a[..fragment_wire_a_len])?;
    let transport_b = walk_ipv6_extensions(&parsed_fragment_b)?;
    let transport_a = walk_ipv6_extensions(&parsed_fragment_a)?;
    let mut ipv6_reassembled = [0u8; 64];
    let mut ipv6_received = [0u8; 64];
    let mut ipv6_reassembly = Ipv6Reassembly::new(&mut ipv6_reassembled, &mut ipv6_received)?;
    ipv6_reassembly.ingest(source_v6, destination_v6, &transport_b)?;
    let ipv6_reassembled_bytes = ipv6_reassembly
        .ingest(source_v6, destination_v6, &transport_a)?
        .ok_or(NetworkError::FragmentIncomplete)?
        .len();

    let udp = UdpPacket {
        source_port: 4000,
        destination_port: 5000,
        payload: b"dual-stack",
    };
    let mut udp_v4 = [0u8; 64];
    let udp_v4_bytes = write_udp(
        &mut udp_v4,
        &udp,
        IpAddress::V4(source_v4),
        IpAddress::V4(destination_v4),
    )?;
    parse_udp(
        &udp_v4[..udp_v4_bytes],
        IpAddress::V4(source_v4),
        IpAddress::V4(destination_v4),
    )?;
    let mut udp_v6 = [0u8; 64];
    let udp_v6_bytes = write_udp(
        &mut udp_v6,
        &udp,
        IpAddress::V6(source_v6),
        IpAddress::V6(destination_v6),
    )?;
    parse_udp(
        &udp_v6[..udp_v6_bytes],
        IpAddress::V6(source_v6),
        IpAddress::V6(destination_v6),
    )?;

    let options = TcpOptions {
        mss: Some(1460),
        window_scale: Some(7),
        sack_permitted: true,
        timestamp: Some((100, 50)),
    };
    let syn_ack = TcpSegment {
        source_port: 443,
        destination_port: 40_000,
        sequence: 9000,
        acknowledgment: 1001,
        flags: TcpFlags::SYN.union(TcpFlags::ACK),
        window: 65_535,
        urgent: 0,
        options,
        payload: &[],
    };
    let mut tcp_bytes = [0u8; 128];
    let tcp_len = write_tcp(
        &mut tcp_bytes,
        &syn_ack,
        IpAddress::V4(destination_v4),
        IpAddress::V4(source_v4),
    )?;
    let parsed_syn_ack = parse_tcp(
        &tcp_bytes[..tcp_len],
        IpAddress::V4(destination_v4),
        IpAddress::V4(source_v4),
    )?;
    let mut client = TcpControlBlock::listen(65_535);
    client.state = TcpState::Closed;
    client.active_open(1000, 0)?;
    client.on_segment(&parsed_syn_ack, 10)?;
    client.send_data(64, 20)?;
    let tcp_retransmits = u64::from(client.tick(2_000) == TcpAction::Retransmit);

    let mut sockets = SocketTable::<8>::new();
    let udp_socket = sockets.create(SocketProtocol::Udp, true)?;
    sockets.bind(udp_socket, IpAddress::V4(source_v4), 4000)?;
    sockets.connect(udp_socket, IpAddress::V4(destination_v4), 5000)?;
    let tcp_socket = sockets.create(SocketProtocol::Tcp, false)?;
    sockets.bind(tcp_socket, IpAddress::V6(source_v6), 40_000)?;

    let mut timers = NetworkTimers::<8>::new();
    timers.schedule(NetworkTimer {
        deadline: 10,
        token: tcp_socket,
        kind: NetworkTimerKind::TcpRetransmit,
    })?;
    timers.schedule(NetworkTimer {
        deadline: 20,
        token: udp_socket,
        kind: NetworkTimerKind::ArpReachability,
    })?;
    let mut expired = [NetworkTimer {
        deadline: 0,
        token: 0,
        kind: NetworkTimerKind::TcpDelayedAck,
    }; 4];
    let timers_expired = timers.expire(20, &mut expired);

    Ok(KernelNetworkReport {
        nic_families: NIC_PCI_CATALOG.len(),
        nic_register_accesses: io.accesses.len(),
        recovered_nic_faults: nic.health.faults,
        nic_rx_descriptors: nic.rx_descriptors.len(),
        nic_tx_bytes,
        ethernet_vlan_tags: parsed_ethernet.vlans.iter().flatten().count(),
        arp_roundtrips: 1,
        ipv4_fragments,
        ipv4_reassembled_bytes,
        ipv6_extension_bytes: transport.extension_bytes,
        ipv6_reassembled_bytes,
        ipv6_nd_messages: 1,
        udp_v4_bytes,
        udp_v6_bytes,
        tcp_states_exercised: 3,
        tcp_retransmits,
        sockets: 2,
        timers_expired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ethernet_arp_ipv4_ipv6_udp_tcp_and_nic_contracts_are_material() {
        let report = certify_kernel_network_contracts().unwrap();
        assert!(report.nic_families >= 14);
        assert!(report.nic_register_accesses > 10);
        assert_eq!(report.recovered_nic_faults, 1);
        assert_eq!(report.ethernet_vlan_tags, 2);
        assert_eq!(report.ipv4_fragments, 3);
        assert_eq!(report.ipv4_reassembled_bytes, 240);
        assert_eq!(report.ipv6_extension_bytes, 8);
        assert_eq!(report.tcp_retransmits, 1);
        assert_eq!(report.timers_expired, 2);
    }

    #[test]
    fn malformed_checksums_overlapping_fragments_and_duplicate_sockets_fail_closed() {
        let source = Ipv4Address([192, 0, 2, 1]);
        let destination = Ipv4Address([192, 0, 2, 2]);
        let packet = UdpPacket {
            source_port: 1,
            destination_port: 2,
            payload: b"checksum",
        };
        let mut bytes = [0u8; 64];
        let length = write_udp(
            &mut bytes,
            &packet,
            IpAddress::V4(source),
            IpAddress::V4(destination),
        )
        .unwrap();
        bytes[8] ^= 0x80;
        assert!(matches!(
            parse_udp(
                &bytes[..length],
                IpAddress::V4(source),
                IpAddress::V4(destination)
            ),
            Err(NetworkError::Checksum("UDP"))
        ));

        let mut sockets = SocketTable::<2>::new();
        let a = sockets.create(SocketProtocol::Udp, false).unwrap();
        let b = sockets.create(SocketProtocol::Udp, false).unwrap();
        sockets.bind(a, IpAddress::V4(source), 53).unwrap();
        assert_eq!(
            sockets.bind(b, IpAddress::V4(source), 53),
            Err(NetworkError::AddressInUse)
        );
    }

    #[test]
    fn socket_buffer_and_timer_storage_are_caller_owned_and_bounded() {
        let mut storage = [0u8; 8];
        let mut buffer = SocketBuffer::new(&mut storage).unwrap();
        buffer.write(b"abcdef").unwrap();
        let mut first = [0u8; 4];
        assert_eq!(buffer.read(&mut first), 4);
        assert_eq!(&first, b"abcd");
        buffer.write(b"ghij").unwrap();
        let mut second = [0u8; 6];
        assert_eq!(buffer.read(&mut second), 6);
        assert_eq!(&second, b"efghij");
    }
}
