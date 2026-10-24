use super::*;

pub(crate) fn primitive_protocol_clause_symbol_impl(
    atoms: &[ncc_ast::PrimitiveDirectiveAtom],
    idx: usize,
) -> Option<(String, usize)> {
    if let Some((path, next_idx)) = primitive_extract_dotted_path(atoms, idx) {
        return Some((path.join("."), next_idx));
    }
    primitive_directive_word(atoms, idx).map(|word| (word.to_string(), idx + 1))
}

pub(crate) fn primitive_parse_protocol_contract_clause_impl(
    atoms: &[ncc_ast::PrimitiveDirectiveAtom],
    entry_name: &str,
) -> Result<Option<PrimitiveProtocolClauseContract>, Box<dyn std::error::Error>> {
    if primitive_directive_word(atoms, 0) != Some("protocol") {
        return Ok(None);
    }
    let Some(protocol_word) = primitive_directive_word(atoms, 1) else {
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' has malformed protocol clause",
            entry_name
        )
        .into());
    };
    let Some(protocol_name) = primitive_normalize_protocol_word(protocol_word) else {
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' uses unsupported protocol '{}'",
            entry_name, protocol_word
        )
        .into());
    };
    let mut out = PrimitiveProtocolClauseContract {
        protocol: protocol_name.to_string(),
        ..PrimitiveProtocolClauseContract::default()
    };
    let mut idx = 2usize;
    while idx < atoms.len() {
        let Some(key) = primitive_directive_word(atoms, idx) else {
            return Err(format!(
                "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' has malformed qualifier syntax",
                entry_name, protocol_name
            )
            .into());
        };
        idx += 1;
        match key {
            "controller" => {
                if out.controller.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier 'controller'",
                        entry_name, protocol_name
                    )
                    .into());
                }
                let Some((symbol, next_idx)) = primitive_protocol_clause_symbol_impl(atoms, idx)
                else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects symbol after 'controller'",
                        entry_name, protocol_name
                    )
                    .into());
                };
                out.controller = Some(symbol);
                idx = next_idx;
            }
            "peripheral" => {
                if out.peripheral.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier 'peripheral'",
                        entry_name, protocol_name
                    )
                    .into());
                }
                let Some((symbol, next_idx)) = primitive_protocol_clause_symbol_impl(atoms, idx)
                else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects symbol after 'peripheral'",
                        entry_name, protocol_name
                    )
                    .into());
                };
                out.peripheral = Some(symbol);
                idx = next_idx;
            }
            "phy" => {
                if out.phy.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier 'phy'",
                        entry_name, protocol_name
                    )
                    .into());
                }
                let Some((symbol, next_idx)) = primitive_protocol_clause_symbol_impl(atoms, idx)
                else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects symbol after 'phy'",
                        entry_name, protocol_name
                    )
                    .into());
                };
                out.phy = Some(symbol);
                idx = next_idx;
            }
            "lane" => {
                if out.lane.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier 'lane'",
                        entry_name, protocol_name
                    )
                    .into());
                }
                let Some((symbol, next_idx)) = primitive_protocol_clause_symbol_impl(atoms, idx)
                else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects symbol after 'lane'",
                        entry_name, protocol_name
                    )
                    .into());
                };
                out.lane = Some(symbol);
                idx = next_idx;
            }
            "channel" => {
                if out.channel.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier 'channel'",
                        entry_name, protocol_name
                    )
                    .into());
                }
                let Some((symbol, next_idx)) = primitive_protocol_clause_symbol_impl(atoms, idx)
                else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects symbol after 'channel'",
                        entry_name, protocol_name
                    )
                    .into());
                };
                out.channel = Some(symbol);
                idx = next_idx;
            }
            "window" => {
                if out.window.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier 'window'",
                        entry_name, protocol_name
                    )
                    .into());
                }
                let Some((symbol, next_idx)) = primitive_protocol_clause_symbol_impl(atoms, idx)
                else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects symbol after 'window'",
                        entry_name, protocol_name
                    )
                    .into());
                };
                out.window = Some(symbol);
                idx = next_idx;
            }
            "port" | "dataPort" => {
                if out.config_addr_port_path.is_some() || out.config_data_port_path.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' cannot mix '{}' with configAddrPort/configDataPort qualifiers",
                        entry_name, protocol_name, key
                    )
                    .into());
                }
                if out.port_path.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                }
                let Some((path, next_idx)) = primitive_extract_dotted_path(atoms, idx) else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects dotted port path after '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                };
                out.port_path = Some(path);
                idx = next_idx;
            }
            "configAddrPort" | "configDataPort" => {
                if out.port_path.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' cannot mix '{}' with port/dataPort qualifier",
                        entry_name, protocol_name, key
                    )
                    .into());
                }
                let slot = if key == "configAddrPort" {
                    &mut out.config_addr_port_path
                } else {
                    &mut out.config_data_port_path
                };
                if slot.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                }
                let Some((path, next_idx)) = primitive_extract_dotted_path(atoms, idx) else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects dotted port path after '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                };
                *slot = Some(path);
                idx = next_idx;
            }
            "statusOffset" | "commandOffset" => {
                let slot = if key == "statusOffset" {
                    &mut out.status_offset
                } else {
                    &mut out.command_offset
                };
                if slot.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                }
                let Some(raw) = primitive_directive_number(atoms, idx) else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects numeric literal after '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                };
                let Some(parsed) = primitive_parse_u16_literal(raw) else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' has invalid '{}' literal '{}'",
                        entry_name, protocol_name, key, raw
                    )
                    .into());
                };
                *slot = Some(parsed);
                idx += 1;
            }
            "readyMask" | "writeMask" | "configCommand" | "compareValue" => {
                let slot = match key {
                    "readyMask" => &mut out.ready_mask,
                    "writeMask" => &mut out.write_mask,
                    "configCommand" => &mut out.config_command,
                    "compareValue" => &mut out.compare_value,
                    _ => unreachable!(),
                };
                if slot.is_some() {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' duplicates qualifier '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                }
                let Some(raw) = primitive_directive_number(atoms, idx) else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' expects numeric literal after '{}'",
                        entry_name, protocol_name, key
                    )
                    .into());
                };
                let Some(parsed) = primitive_parse_u8_literal(raw) else {
                    return Err(format!(
                        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' has invalid '{}' literal '{}'",
                        entry_name, protocol_name, key, raw
                    )
                    .into());
                };
                *slot = Some(parsed);
                idx += 1;
            }
            other => {
                return Err(format!(
                    "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' has unknown qualifier '{}'",
                    entry_name, protocol_name, other
                )
                .into());
            }
        }
    }
    Ok(Some(out))
}

pub(crate) fn primitive_parse_port_ref_impl(
    atoms: &[ncc_ast::PrimitiveDirectiveAtom],
    start: usize,
    port_addr_by_path: &HashMap<Vec<String>, u16>,
) -> Result<Option<(u16, Vec<String>, usize)>, Box<dyn std::error::Error>> {
    if let Some((path, next_idx)) = primitive_extract_dotted_path(atoms, start) {
        let port = *port_addr_by_path.get(&path).ok_or_else(|| {
            format!(
                "E_PRIMITIVE_PORT_UNSUPPORTED_001: unresolved port address for '{}'",
                path.join(".")
            )
        })?;
        return Ok(Some((port, path, next_idx)));
    }

    if let Some(raw_port) = primitive_directive_number(atoms, start) {
        let port = primitive_parse_u16_literal(raw_port).ok_or_else(|| {
            format!(
                "E_PRIMITIVE_PORT_UNSUPPORTED_001: invalid port literal '{}'",
                raw_port
            )
        })?;
        return Ok(Some((port, Vec::new(), start + 1)));
    }

    Ok(None)
}

pub(crate) fn primitive_parse_address_ref_impl(
    atoms: &[ncc_ast::PrimitiveDirectiveAtom],
    start: usize,
) -> Result<Option<(u32, usize)>, Box<dyn std::error::Error>> {
    if let Some(raw_addr) = primitive_directive_number(atoms, start) {
        let addr = primitive_parse_u32_literal(raw_addr).ok_or_else(|| {
            format!(
                "E_PRIMITIVE_MMIO_UNSUPPORTED_001: invalid MMIO address literal '{}'",
                raw_addr
            )
        })?;
        return Ok(Some((addr, start + 1)));
    }

    let helper = primitive_directive_word(atoms, start);
    if matches!(helper, Some("ioaddr" | "mmioaddr" | "physaddr" | "addr"))
        && matches!(
            atoms.get(start + 1),
            Some(ncc_ast::PrimitiveDirectiveAtom::Punct(
                ncc_ast::PrimitiveDirectivePunct::LParen
            ))
        )
        && matches!(
            atoms.get(start + 3),
            Some(ncc_ast::PrimitiveDirectiveAtom::Punct(
                ncc_ast::PrimitiveDirectivePunct::RParen
            ))
        )
    {
        let Some(raw_addr) = primitive_directive_number(atoms, start + 2) else {
            return Err(
                "E_PRIMITIVE_MMIO_UNSUPPORTED_001: MMIO helper requires numeric literal address"
                    .into(),
            );
        };
        let addr = primitive_parse_u32_literal(raw_addr).ok_or_else(|| {
            format!(
                "E_PRIMITIVE_MMIO_UNSUPPORTED_001: invalid MMIO helper address literal '{}'",
                raw_addr
            )
        })?;
        return Ok(Some((addr, start + 4)));
    }

    Ok(None)
}

pub(crate) fn primitive_wait_status_port_for_path_impl(
    path: &[String],
    port_addr_by_path: &HashMap<Vec<String>, u16>,
    fallback_port: u16,
) -> Option<u16> {
    if path.is_empty() {
        return None;
    }
    let mut status_path = path.to_vec();
    status_path.pop();
    status_path.push("status".to_string());
    Some(
        port_addr_by_path
            .get(&status_path)
            .copied()
            .unwrap_or_else(|| fallback_port.saturating_add(5)),
    )
}

pub(crate) fn primitive_resolve_protocol_symbol_impl(
    entry_name: &str,
    protocol: &str,
    role: &str,
    explicit: Option<&str>,
    declared: &std::collections::HashSet<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(symbol) = explicit {
        if declared.contains(symbol) {
            return Ok(symbol.to_string());
        }
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' references unknown {} '{}'",
            entry_name, protocol, role, symbol
        )
        .into());
    }
    if declared.len() == 1 {
        return Ok(declared.iter().next().cloned().unwrap_or_default());
    }
    Err(format!(
        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' requires explicit {} qualifier",
        entry_name, protocol, role
    )
    .into())
}

pub(crate) fn primitive_resolve_protocol_optional_symbol_impl(
    entry_name: &str,
    protocol: &str,
    role: &str,
    explicit: Option<&str>,
    declared: &std::collections::HashSet<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(symbol) = explicit {
        if declared.contains(symbol) {
            return Ok(Some(symbol.to_string()));
        }
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' references unknown {} '{}'",
            entry_name, protocol, role, symbol
        )
        .into());
    }
    if declared.len() == 1 {
        return Ok(declared.iter().next().cloned());
    }
    Ok(None)
}

pub(crate) fn primitive_resolve_protocol_window_impl(
    entry_name: &str,
    protocol: &str,
    explicit: Option<&str>,
    windows: &std::collections::HashSet<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    primitive_resolve_protocol_symbol_impl(entry_name, protocol, "window", explicit, windows)
}

pub(crate) fn primitive_resolve_protocol_window_addr_impl(
    entry_name: &str,
    protocol: &str,
    window: &str,
    window_addr_by_symbol: &HashMap<String, PrimitiveWindowAddrContract>,
) -> Result<u32, Box<dyn std::error::Error>> {
    let Some(contract) = window_addr_by_symbol.get(window).copied() else {
        return Err(format!(
            "E_PRIMITIVE_MMIO_ENV_001: operation '{}' protocol '{}' requires MMIO window '{}' with explicit addr mmioaddr(...) contract",
            entry_name, protocol, window
        )
        .into());
    };
    if contract.kind != PrimitiveWindowAddrKind::MmioHelper {
        return Err(format!(
            "E_PRIMITIVE_MMIO_ENV_001: operation '{}' protocol '{}' requires MMIO window '{}' declared as addr mmioaddr(...), but current window contract is non-MMIO",
            entry_name, protocol, window
        )
        .into());
    }
    Ok(contract.addr)
}

pub(crate) fn primitive_resolve_protocol_port_path_impl(
    entry_name: &str,
    protocol: &str,
    explicit_path: Option<&[String]>,
    port_addr_by_path: &HashMap<Vec<String>, u16>,
) -> Result<(Vec<String>, u16), Box<dyn std::error::Error>> {
    if let Some(path) = explicit_path {
        let key = path.to_vec();
        let Some(port) = port_addr_by_path.get(&key).copied() else {
            return Err(format!(
                "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' references unknown port path '{}'",
                entry_name,
                protocol,
                key.join(".")
            )
            .into());
        };
        return Ok((key, port));
    }
    if port_addr_by_path.len() == 1 {
        if let Some((path, port)) = port_addr_by_path.iter().next() {
            return Ok((path.clone(), *port));
        }
    }
    Err(format!(
        "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' requires explicit port/dataPort qualifier",
        entry_name, protocol
    )
    .into())
}

pub(crate) fn primitive_protocol_clause_to_op_impl(
    entry_name: &str,
    clause: &PrimitiveProtocolClauseContract,
    controller_symbols: &std::collections::HashSet<String>,
    peripheral_symbols: &std::collections::HashSet<String>,
    phy_symbols: &std::collections::HashSet<String>,
    lane_symbols: &std::collections::HashSet<String>,
    channel_symbols: &std::collections::HashSet<String>,
    window_symbols: &std::collections::HashSet<String>,
    window_addr_by_symbol: &HashMap<String, PrimitiveWindowAddrContract>,
    port_addr_by_path: &HashMap<Vec<String>, u16>,
) -> Result<PrimitiveMaterialOp, Box<dyn std::error::Error>> {
    let protocol = clause.protocol.as_str();
    if protocol != "PCIE"
        && (clause.config_addr_port_path.is_some() || clause.config_data_port_path.is_some())
    {
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' only allows configAddrPort/configDataPort qualifiers for protocol PCIE",
            entry_name, protocol
        )
        .into());
    }
    let has_window_offsets = clause.status_offset.is_some() || clause.command_offset.is_some();
    if has_window_offsets
        && !matches!(
            protocol,
            "USB" | "SATA" | "NVME" | "ETHERNET" | "HDMI" | "DISPLAYPORT"
        )
    {
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' only allows statusOffset/commandOffset qualifiers for MMIO protocols",
            entry_name, protocol
        )
        .into());
    }
    if (clause.config_command.is_some() || clause.compare_value.is_some()) && protocol != "PCIE" {
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' only allows configCommand/compareValue qualifiers for protocol PCIE",
            entry_name, protocol
        )
        .into());
    }
    if clause.write_mask.is_some() && !matches!(protocol, "UART" | "I2C" | "SPI") {
        return Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' only allows writeMask qualifier for UART/I2C/SPI protocols",
            entry_name, protocol
        )
        .into());
    }
    match protocol {
        "UART" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let peripheral = primitive_resolve_protocol_optional_symbol_impl(
                entry_name,
                protocol,
                "peripheral",
                clause.peripheral.as_deref(),
                peripheral_symbols,
            )?;
            let (data_path, data_port) = primitive_resolve_protocol_port_path_impl(
                entry_name,
                protocol,
                clause.port_path.as_deref(),
                port_addr_by_path,
            )?;
            let status_port =
                primitive_wait_status_port_for_path_impl(&data_path, port_addr_by_path, data_port)
                    .unwrap_or(data_port.saturating_add(5));
            let ready_mask = clause.ready_mask.unwrap_or(0x20);
            let write_mask = clause.write_mask.unwrap_or(0x01);
            Ok(PrimitiveMaterialOp::ProtocolUart {
                controller,
                peripheral,
                data_port,
                status_port,
                ready_mask,
                write_mask,
            })
        }
        "PCIE" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let lane = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "lane",
                clause.lane.as_deref(),
                lane_symbols,
            )?;
            if clause.config_addr_port_path.is_none() && clause.config_data_port_path.is_none() {
                return Err(format!(
                    "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' requires explicit configAddrPort and configDataPort qualifiers",
                    entry_name, protocol
                )
                .into());
            }
            if clause.config_addr_port_path.is_some() ^ clause.config_data_port_path.is_some() {
                return Err(format!(
                    "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' requires both configAddrPort and configDataPort qualifiers",
                    entry_name, protocol
                )
                .into());
            }
            let (Some(addr_path), Some(data_path)) = (
                clause.config_addr_port_path.as_deref(),
                clause.config_data_port_path.as_deref(),
            ) else {
                return Err(format!(
                    "E_PRIMITIVE_PROTOCOL_001: operation '{}' protocol '{}' requires explicit configAddrPort/configDataPort contract",
                    entry_name, protocol
                )
                .into());
            };
            let (_, config_addr_port) = primitive_resolve_protocol_port_path_impl(
                entry_name,
                protocol,
                Some(addr_path),
                port_addr_by_path,
            )?;
            let (_, config_data_port) = primitive_resolve_protocol_port_path_impl(
                entry_name,
                protocol,
                Some(data_path),
                port_addr_by_path,
            )?;
            let config_command = clause.config_command.unwrap_or(0x80);
            let compare_value = clause.compare_value.unwrap_or(0x32);
            Ok(PrimitiveMaterialOp::ProtocolPcie {
                controller,
                lane,
                config_addr_port,
                config_data_port,
                config_command,
                compare_value,
            })
        }
        "USB" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let peripheral = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "peripheral",
                clause.peripheral.as_deref(),
                peripheral_symbols,
            )?;
            let window = primitive_resolve_protocol_window_impl(
                entry_name,
                protocol,
                clause.window.as_deref(),
                window_symbols,
            )?;
            let window_addr = primitive_resolve_protocol_window_addr_impl(
                entry_name,
                protocol,
                &window,
                window_addr_by_symbol,
            )?;
            let status_offset = clause.status_offset.unwrap_or(0x04);
            let command_offset = clause.command_offset.unwrap_or(0x00);
            Ok(PrimitiveMaterialOp::ProtocolUsb {
                controller,
                peripheral,
                window,
                window_addr,
                status_offset,
                command_offset,
            })
        }
        "SATA" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let peripheral = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "peripheral",
                clause.peripheral.as_deref(),
                peripheral_symbols,
            )?;
            let channel = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "channel",
                clause.channel.as_deref(),
                channel_symbols,
            )?;
            let window = primitive_resolve_protocol_window_impl(
                entry_name,
                protocol,
                clause.window.as_deref(),
                window_symbols,
            )?;
            let window_addr = primitive_resolve_protocol_window_addr_impl(
                entry_name,
                protocol,
                &window,
                window_addr_by_symbol,
            )?;
            let status_offset = clause.status_offset.unwrap_or(0x10);
            let command_offset = clause.command_offset.unwrap_or(0x18);
            Ok(PrimitiveMaterialOp::ProtocolSata {
                controller,
                peripheral,
                channel,
                window,
                window_addr,
                status_offset,
                command_offset,
            })
        }
        "NVME" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let peripheral = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "peripheral",
                clause.peripheral.as_deref(),
                peripheral_symbols,
            )?;
            let channel = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "channel",
                clause.channel.as_deref(),
                channel_symbols,
            )?;
            let window = primitive_resolve_protocol_window_impl(
                entry_name,
                protocol,
                clause.window.as_deref(),
                window_symbols,
            )?;
            let window_addr = primitive_resolve_protocol_window_addr_impl(
                entry_name,
                protocol,
                &window,
                window_addr_by_symbol,
            )?;
            let status_offset = clause.status_offset.unwrap_or(0x1C);
            let command_offset = clause.command_offset.unwrap_or(0x20);
            Ok(PrimitiveMaterialOp::ProtocolNvme {
                controller,
                peripheral,
                channel,
                window,
                window_addr,
                status_offset,
                command_offset,
            })
        }
        "I2C" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let channel = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "channel",
                clause.channel.as_deref(),
                channel_symbols,
            )?;
            let (_, control_port) = primitive_resolve_protocol_port_path_impl(
                entry_name,
                protocol,
                clause.port_path.as_deref(),
                port_addr_by_path,
            )?;
            let ready_mask = clause.ready_mask.unwrap_or(0x02);
            let write_mask = clause.write_mask.unwrap_or(0x0F);
            Ok(PrimitiveMaterialOp::ProtocolI2c {
                controller,
                channel,
                control_port,
                ready_mask,
                write_mask,
            })
        }
        "SPI" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let channel = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "channel",
                clause.channel.as_deref(),
                channel_symbols,
            )?;
            let (_, control_port) = primitive_resolve_protocol_port_path_impl(
                entry_name,
                protocol,
                clause.port_path.as_deref(),
                port_addr_by_path,
            )?;
            let ready_mask = clause.ready_mask.unwrap_or(0x01);
            let write_mask = clause.write_mask.unwrap_or(0x1F);
            Ok(PrimitiveMaterialOp::ProtocolSpi {
                controller,
                channel,
                control_port,
                ready_mask,
                write_mask,
            })
        }
        "ETHERNET" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let peripheral = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "peripheral",
                clause.peripheral.as_deref(),
                peripheral_symbols,
            )?;
            let phy = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "phy",
                clause.phy.as_deref(),
                phy_symbols,
            )?;
            let window = primitive_resolve_protocol_window_impl(
                entry_name,
                protocol,
                clause.window.as_deref(),
                window_symbols,
            )?;
            let window_addr = primitive_resolve_protocol_window_addr_impl(
                entry_name,
                protocol,
                &window,
                window_addr_by_symbol,
            )?;
            let status_offset = clause.status_offset.unwrap_or(0x08);
            let command_offset = clause.command_offset.unwrap_or(0x14);
            let ready_mask = clause.ready_mask.unwrap_or(0x10);
            Ok(PrimitiveMaterialOp::ProtocolEthernet {
                controller,
                peripheral,
                phy,
                window,
                window_addr,
                status_offset,
                command_offset,
                ready_mask,
            })
        }
        "HDMI" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let phy = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "phy",
                clause.phy.as_deref(),
                phy_symbols,
            )?;
            let window = primitive_resolve_protocol_window_impl(
                entry_name,
                protocol,
                clause.window.as_deref(),
                window_symbols,
            )?;
            let window_addr = primitive_resolve_protocol_window_addr_impl(
                entry_name,
                protocol,
                &window,
                window_addr_by_symbol,
            )?;
            let status_offset = clause.status_offset.unwrap_or(0x0C);
            let command_offset = clause.command_offset.unwrap_or(0x10);
            let ready_mask = clause.ready_mask.unwrap_or(0x40);
            Ok(PrimitiveMaterialOp::ProtocolHdmi {
                controller,
                phy,
                window,
                window_addr,
                status_offset,
                command_offset,
                ready_mask,
            })
        }
        "DISPLAYPORT" => {
            let controller = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "controller",
                clause.controller.as_deref(),
                controller_symbols,
            )?;
            let phy = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "phy",
                clause.phy.as_deref(),
                phy_symbols,
            )?;
            let lane = primitive_resolve_protocol_symbol_impl(
                entry_name,
                protocol,
                "lane",
                clause.lane.as_deref(),
                lane_symbols,
            )?;
            let window = primitive_resolve_protocol_window_impl(
                entry_name,
                protocol,
                clause.window.as_deref(),
                window_symbols,
            )?;
            let window_addr = primitive_resolve_protocol_window_addr_impl(
                entry_name,
                protocol,
                &window,
                window_addr_by_symbol,
            )?;
            let status_offset = clause.status_offset.unwrap_or(0x14);
            let command_offset = clause.command_offset.unwrap_or(0x18);
            let ready_mask = clause.ready_mask.unwrap_or(0x40);
            Ok(PrimitiveMaterialOp::ProtocolDisplayPort {
                controller,
                phy,
                lane,
                window,
                window_addr,
                status_offset,
                command_offset,
                ready_mask,
            })
        }
        _ => Err(format!(
            "E_PRIMITIVE_PROTOCOL_001: operation '{}' uses unsupported protocol '{}'",
            entry_name, protocol
        )
        .into()),
    }
}
