pub(crate) fn primitive_protocol_diag_code(protocol: &str) -> &'static str {
    match protocol {
        "UART" => "E_PRIMITIVE_UART_001",
        "PCIE" => "E_PRIMITIVE_PCIE_001",
        "USB" => "E_PRIMITIVE_USB_001",
        "SATA" => "E_PRIMITIVE_SATA_001",
        "NVME" => "E_PRIMITIVE_NVME_001",
        "I2C" => "E_PRIMITIVE_I2C_001",
        "SPI" => "E_PRIMITIVE_SPI_001",
        "ETHERNET" => "E_PRIMITIVE_ETHERNET_001",
        "HDMI" | "DISPLAYPORT" | "DP" => "E_PRIMITIVE_DISPLAY_001",
        _ => "E_PRIMITIVE_PROTOCOL_001",
    }
}

pub(crate) fn primitive_normalize_protocol_word(word: &str) -> Option<&'static str> {
    match word.to_ascii_lowercase().as_str() {
        "uart" => Some("UART"),
        "pcie" => Some("PCIE"),
        "usb" => Some("USB"),
        "sata" => Some("SATA"),
        "nvme" => Some("NVME"),
        "i2c" => Some("I2C"),
        "spi" => Some("SPI"),
        "ethernet" => Some("ETHERNET"),
        "hdmi" => Some("HDMI"),
        "displayport" | "dp" => Some("DISPLAYPORT"),
        _ => None,
    }
}

pub(crate) fn primitive_protocol_marker_byte(protocol: &str) -> u8 {
    match protocol {
        "UART" => 0x31,
        "PCIE" => 0x32,
        "USB" => 0x33,
        "SATA" => 0x34,
        "NVME" => 0x35,
        "I2C" => 0x36,
        "SPI" => 0x37,
        "ETHERNET" => 0x38,
        "HDMI" => 0x39,
        "DISPLAYPORT" => 0x3A,
        _ => 0x3F,
    }
}

pub(crate) fn primitive_protocol_lowering_mode(protocol: &str) -> &'static str {
    match protocol {
        "UART" => "protocol_uart_port_io_tx_material",
        "PCIE" => "protocol_pcie_config_cycle_material",
        "USB" => "protocol_usb_host_mmio_material",
        "SATA" => "protocol_sata_ahci_mmio_material",
        "NVME" => "protocol_nvme_queue_mmio_material",
        "I2C" => "protocol_i2c_controller_io_material",
        "SPI" => "protocol_spi_controller_io_material",
        "ETHERNET" => "protocol_ethernet_link_mmio_material",
        "HDMI" => "protocol_hdmi_link_mmio_material",
        "DISPLAYPORT" => "protocol_displayport_link_mmio_material",
        _ => "protocol_contract_probe",
    }
}

pub(crate) fn primitive_protocol_transport(protocol: &str) -> &'static str {
    match protocol {
        "UART" | "PCIE" | "I2C" | "SPI" => "port_io",
        "USB" | "SATA" | "NVME" | "ETHERNET" | "HDMI" | "DISPLAYPORT" => "mmio",
        _ => "unknown",
    }
}

pub(crate) fn primitive_protocol_lowering_prefix_bytes(protocol: &str) -> &'static [u8] {
    match protocol {
        "UART" => &[0xBA, 0xFD, 0x03, 0xEC, 0xA8, 0x20],
        "PCIE" => &[0xBA, 0xF8, 0x0C, 0xB0, 0x80, 0xEE, 0xBA, 0xFC, 0x0C, 0xEC],
        "USB" => &[0xBB, 0x00, 0x10, 0x8A, 0x07, 0xA8, 0x08],
        "SATA" => &[0xBB, 0x00, 0x12, 0x8A, 0x07, 0xA8, 0x04],
        "NVME" => &[0xBB, 0x00, 0x14, 0x8A, 0x07, 0x0F, 0xAE, 0xF0],
        "I2C" => &[0xBA, 0x00, 0x05, 0xEC, 0xA8, 0x02],
        "SPI" => &[0xBA, 0x00, 0x06, 0xEC, 0xA8, 0x01],
        "ETHERNET" => &[0xBB, 0x00, 0x16, 0x8A, 0x07, 0x0F, 0xAE, 0xF0],
        "HDMI" => &[0xBB, 0x00, 0x18, 0x8A, 0x07, 0xA8, 0x40],
        "DISPLAYPORT" => &[0xBB, 0x00, 0x1A, 0x8A, 0x07, 0xA8, 0x40],
        _ => &[0x90],
    }
}

pub(crate) fn primitive_protocol_lowering_suffix_bytes(protocol: &str) -> &'static [u8] {
    match protocol {
        "UART" => &[0xBA, 0xF8, 0x03, 0x0C, 0x01, 0xEE],
        "PCIE" => &[0x0F, 0xAE, 0xF0, 0x3C, 0x32],
        "USB" => &[0xB0, 0x04, 0x88, 0x07, 0x0F, 0xAE, 0xF0],
        "SATA" => &[0xB0, 0x34, 0x88, 0x07, 0x0F, 0xAE, 0xF0],
        "NVME" => &[0xA8, 0x20, 0xB0, 0x35, 0x88, 0x07],
        "I2C" => &[0x24, 0x0F, 0xEE],
        "SPI" => &[0x24, 0x1F, 0xEE],
        "ETHERNET" => &[0xA8, 0x10, 0xB0, 0x38, 0x88, 0x07],
        "HDMI" => &[0xB0, 0x39, 0x88, 0x07],
        "DISPLAYPORT" => &[0xB0, 0x3A, 0x88, 0x07],
        _ => &[0x3C, 0x3F],
    }
}

pub(crate) fn primitive_protocol_lowering_semantics(protocol: &str) -> &'static [&'static str] {
    match protocol {
        "UART" => &[
            "protocol_uart_contract",
            "uart_line_status_poll",
            "serial_link_materialized",
        ],
        "PCIE" => &[
            "protocol_pcie_contract",
            "pcie_config_cycle_materialized",
            "pcie_link_capability_checked",
        ],
        "USB" => &[
            "protocol_usb_contract",
            "usb_host_controller_mmio_materialized",
            "usb_controller_capability_checked",
        ],
        "SATA" => &[
            "protocol_sata_contract",
            "sata_ahci_register_window_materialized",
            "sata_controller_capability_checked",
        ],
        "NVME" => &[
            "protocol_nvme_contract",
            "nvme_queue_doorbell_materialized",
            "nvme_controller_capability_checked",
        ],
        "I2C" => &[
            "protocol_i2c_contract",
            "i2c_controller_io_materialized",
            "i2c_bus_capability_checked",
        ],
        "SPI" => &[
            "protocol_spi_contract",
            "spi_controller_io_materialized",
            "spi_bus_capability_checked",
        ],
        "ETHERNET" => &[
            "protocol_ethernet_contract",
            "ethernet_link_register_materialized",
            "ethernet_phy_capability_checked",
        ],
        "HDMI" => &[
            "protocol_hdmi_contract",
            "hdmi_link_training_materialized",
            "display_link_capability_checked",
        ],
        "DISPLAYPORT" => &[
            "protocol_displayport_contract",
            "displayport_link_training_materialized",
            "display_link_capability_checked",
        ],
        _ => &["protocol_contract", "target_protocol_capability_checked"],
    }
}
