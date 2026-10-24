use ncc_parser::Program;
use std::collections::HashMap;

mod atomic;
mod console;
mod json;
mod memory_map;
mod value;

pub use atomic::{
    atomic_fence, compiler_fence, volatile_read, volatile_write, AtomicI64Cell, AtomicRegistry,
    AtomicRegistryError, AtomicWaitResult,
};
pub use console::{
    console_builtin_from_print_kind, execute_console_builtin, execute_console_builtin_path,
    render_console_text_controls, resolve_console_builtin, ConsoleBuiltin, ConsoleBuiltinSpec,
    ConsoleChannel,
};
pub use console::{emit_print, ConsolePrintKind};
pub use json::{
    JsonError, JsonErrorCode, JsonHandle, JsonKind, JsonLimits, JsonResult, JsonRuntime,
    JSON_INVALID_HANDLE,
};
pub use memory_map::{HostMemoryMap, MapAdvice, MapFlags, MapProtection, MapSync};
pub use value::{
    render_binary_i64, render_binary_i64_portable, render_binary_i64_with_provider,
    render_binary_provider, render_value, render_value_with_mode, RenderBinaryProvider, RenderMode,
    RuntimeClosureHandle, RuntimePointer, RuntimeValue,
};

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
}

#[derive(Debug, Clone)]
pub struct RuntimeResult {
    pub ok: bool,
    pub message: String,
}

pub fn run_program(_program: &Program) -> Result<RuntimeResult, String> {
    Ok(RuntimeResult {
        ok: true,
        message: "runtime program contract accepted".into(),
    })
}

pub fn run_function(
    _program: &Program,
    _container_name: &str,
    _function_name: &str,
    _args: &[String],
) -> Result<Value, String> {
    Err("E_RUNTIME_EXEC_001: run_function requires ncc-cli execution bridge".into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapDeathReason {
    Freed,
    Reallocated,
    Unmapped,
    Remapped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapEventKind {
    Alloc,
    Map,
    Zero,
    Protect,
    Sync,
    Advise,
    Lock,
    Unlock,
    Free,
    Unmap,
    ReallocOut,
    ReallocIn,
    RemapOut,
    RemapIn,
}

impl HeapEventKind {
    fn is_root_create(&self) -> bool {
        matches!(self, Self::Alloc | Self::Map)
    }

    fn is_transition_out(&self) -> bool {
        matches!(self, Self::ReallocOut | Self::RemapOut)
    }

    fn is_transition_in(&self) -> bool {
        matches!(self, Self::ReallocIn | Self::RemapIn)
    }

    fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Free | Self::Unmap | Self::ReallocOut | Self::RemapOut
        )
    }

    fn transition_pair_matches(previous: &Self, current: &Self) -> bool {
        matches!(
            (previous, current),
            (Self::ReallocOut, Self::ReallocIn) | (Self::RemapOut, Self::RemapIn)
        )
    }

    fn mutation_name(&self) -> &'static str {
        match self {
            Self::Alloc => "alloc",
            Self::Map => "map",
            Self::Zero => "zero",
            Self::Protect => "protect",
            Self::Sync => "sync",
            Self::Advise => "advise",
            Self::Lock => "lock",
            Self::Unlock => "unlock",
            Self::Free => "free",
            Self::Unmap => "unmap",
            Self::ReallocOut => "realloc-out",
            Self::ReallocIn => "realloc-in",
            Self::RemapOut => "remap-out",
            Self::RemapIn => "remap-in",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeapLineage {
    root_addr: String,
    parent_addr: Option<String>,
    generation: usize,
    path: Vec<String>,
}

impl HeapLineage {
    pub fn root(addr: impl Into<String>) -> Result<Self, String> {
        let addr = addr.into();
        if addr.is_empty() {
            return Err("heap lineage root address cannot be empty".into());
        }
        Ok(Self {
            root_addr: addr.clone(),
            parent_addr: None,
            generation: 0,
            path: vec![addr],
        })
    }

    pub fn child(&self, addr: impl Into<String>) -> Result<Self, String> {
        let addr = addr.into();
        if addr.is_empty() {
            return Err("heap lineage child address cannot be empty".into());
        }
        let parent = self
            .path
            .last()
            .cloned()
            .ok_or_else(|| "heap lineage has no leaf".to_string())?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| "heap lineage generation overflow".to_string())?;
        let mut path = self.path.clone();
        path.push(addr);
        Ok(Self {
            root_addr: self.root_addr.clone(),
            parent_addr: Some(parent),
            generation,
            path,
        })
    }

    pub fn from_parts(
        addr: impl Into<String>,
        root_addr: impl Into<String>,
        parent_addr: Option<String>,
        generation: usize,
        lineage_path: impl AsRef<str>,
    ) -> Result<Self, String> {
        let addr = addr.into();
        let root_addr = root_addr.into();
        let path = lineage_path
            .as_ref()
            .split("->")
            .map(str::to_string)
            .collect::<Vec<_>>();
        let lineage = Self {
            root_addr,
            parent_addr,
            generation,
            path,
        };
        lineage.validate_leaf(&addr)?;
        Ok(lineage)
    }

    pub fn validate_leaf(&self, addr: &str) -> Result<(), String> {
        if addr.is_empty() || self.root_addr.is_empty() {
            return Err("heap lineage address cannot be empty".into());
        }
        if self.path.is_empty() || self.path.iter().any(String::is_empty) {
            return Err("heap lineage path contains an empty segment".into());
        }
        if self.path.first() != Some(&self.root_addr) {
            return Err(format!(
                "heap lineage root mismatch: expected {}, got {}",
                self.root_addr,
                self.path.first().map(String::as_str).unwrap_or("-")
            ));
        }
        if self.path.last().map(String::as_str) != Some(addr) {
            return Err(format!(
                "heap lineage leaf mismatch: expected {}, got {}",
                addr,
                self.path.last().map(String::as_str).unwrap_or("-")
            ));
        }
        let expected_depth = self
            .generation
            .checked_add(1)
            .ok_or_else(|| "heap lineage depth overflow".to_string())?;
        if self.path.len() != expected_depth {
            return Err(format!(
                "heap lineage depth mismatch: expected {}, got {}",
                expected_depth,
                self.path.len()
            ));
        }
        match &self.parent_addr {
            Some(parent) => {
                if self.generation == 0 {
                    return Err("heap lineage with parent cannot have generation 0".into());
                }
                let actual_parent = self.path.get(self.path.len().saturating_sub(2));
                if actual_parent != Some(parent) {
                    return Err(format!(
                        "heap lineage parent mismatch: expected {}, got {}",
                        parent,
                        actual_parent.map(String::as_str).unwrap_or("-")
                    ));
                }
            }
            None => {
                if self.generation != 0 || self.root_addr != addr || self.path.len() != 1 {
                    return Err("root heap lineage must have one generation-0 segment".into());
                }
            }
        }
        Ok(())
    }

    pub fn root_addr(&self) -> &str {
        &self.root_addr
    }

    pub fn parent_addr(&self) -> Option<&str> {
        self.parent_addr.as_deref()
    }

    pub fn generation(&self) -> usize {
        self.generation
    }

    pub fn leaf_addr(&self) -> &str {
        self.path.last().map(String::as_str).unwrap_or("")
    }

    pub fn path(&self) -> &[String] {
        &self.path
    }

    pub fn path_string(&self) -> String {
        self.path.join("->")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeapEventRecord {
    pub event_id: usize,
    pub kind: HeapEventKind,
    pub actor_symbol: Option<String>,
    pub addr: String,
    pub root_addr: String,
    pub parent_addr: Option<String>,
    pub generation: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeapBlockSnapshot {
    pub addr: String,
    pub live: bool,
    pub death_reason: Option<HeapDeathReason>,
    pub parent_addr: Option<String>,
    pub root_addr: String,
    pub lineage_path: String,
    pub generation: usize,
    pub event_id: usize,
    pub last_mutation: String,
    pub trace_total_len: usize,
    pub trace_dropped_count: usize,
    pub event_trace: Vec<String>,
}

pub fn audit_heap_state(
    events: &[HeapEventRecord],
    blocks: &[HeapBlockSnapshot],
) -> Result<(), String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum AddrLife {
        Live,
        Dead,
    }

    #[derive(Clone)]
    struct AddrInfo {
        root_addr: String,
        parent_addr: Option<String>,
        generation: usize,
        life: AddrLife,
        last_event_id: usize,
        last_event_kind: HeapEventKind,
    }

    let mut addr_info: HashMap<String, AddrInfo> = HashMap::new();
    let mut prev_event: Option<&HeapEventRecord> = None;

    for ev in events {
        if let Some(prev) = prev_event {
            if ev.event_id < prev.event_id {
                return Err(format!(
                    "event_id regresses: prev={} curr={}",
                    prev.event_id, ev.event_id
                ));
            }

            if ev.event_id == prev.event_id {
                if !prev.kind.is_transition_out() {
                    return Err(format!(
                        "duplicate event_id {} requires previous transition-out kind",
                        ev.event_id
                    ));
                }

                if !ev.kind.is_transition_in()
                    || !HeapEventKind::transition_pair_matches(&prev.kind, &ev.kind)
                {
                    return Err(format!(
                        "duplicate event_id {} requires matching transition-in kind",
                        ev.event_id
                    ));
                }

                let Some(parent_addr) = ev.parent_addr.as_ref() else {
                    return Err(format!(
                        "realloc-in with duplicate event_id {} requires parent addr",
                        ev.event_id
                    ));
                };

                if parent_addr != &prev.addr {
                    return Err(format!(
                        "realloc-in parent mismatch at event_id {}: expected {}, got {}",
                        ev.event_id, prev.addr, parent_addr
                    ));
                }

                if ev.root_addr != prev.root_addr {
                    return Err(format!(
                        "realloc-in root mismatch at event_id {}: expected {}, got {}",
                        ev.event_id, prev.root_addr, ev.root_addr
                    ));
                }

                if ev.generation != prev.generation.saturating_add(1) {
                    return Err(format!(
                        "realloc-in generation mismatch at event_id {}: expected {}, got {}",
                        ev.event_id,
                        prev.generation.saturating_add(1),
                        ev.generation
                    ));
                }
            }
        }

        if ev.addr.is_empty() {
            return Err(format!("empty address in event_id {}", ev.event_id));
        }
        if ev.root_addr.is_empty() {
            return Err(format!("empty root address in event_id {}", ev.event_id));
        }

        if let Some(parent) = &ev.parent_addr {
            if parent.is_empty() {
                return Err(format!("empty parent address in event_id {}", ev.event_id));
            }
            if ev.generation == 0 {
                return Err(format!(
                    "non-root event has generation 0 in event_id {}",
                    ev.event_id
                ));
            }
            let Some(parent_info) = addr_info.get(parent) else {
                return Err(format!(
                    "parent address {} not found before event_id {}",
                    parent, ev.event_id
                ));
            };
            if parent_info.root_addr != ev.root_addr {
                return Err(format!(
                    "parent/root mismatch in event_id {}: parent root {}, event root {}",
                    ev.event_id, parent_info.root_addr, ev.root_addr
                ));
            }
        } else if ev.kind.is_transition_in() {
            return Err(format!(
                "transition-in missing parent address in event_id {}",
                ev.event_id
            ));
        }

        if ev.kind.is_root_create() {
            if ev.parent_addr.is_some() {
                return Err(format!(
                    "root creation must not have parent address in event_id {}",
                    ev.event_id
                ));
            }
            if ev.generation != 0 {
                return Err(format!(
                    "root creation must have generation 0 in event_id {}",
                    ev.event_id
                ));
            }
            if ev.root_addr != ev.addr {
                return Err(format!(
                    "root creation mismatch in event_id {}: expected root {}",
                    ev.event_id, ev.addr
                ));
            }
            if let Some(info) = addr_info.get(&ev.addr) {
                if matches!(info.life, AddrLife::Live) {
                    return Err(format!(
                        "address {} already live before Alloc event_id {}",
                        ev.addr, ev.event_id
                    ));
                }
            }
            addr_info.insert(
                ev.addr.clone(),
                AddrInfo {
                    root_addr: ev.root_addr.clone(),
                    parent_addr: None,
                    generation: ev.generation,
                    life: AddrLife::Live,
                    last_event_id: ev.event_id,
                    last_event_kind: ev.kind.clone(),
                },
            );
        } else if ev.kind.is_transition_in() {
            if ev.parent_addr.is_none() {
                return Err(format!(
                    "transition-in missing parent address in event_id {}",
                    ev.event_id
                ));
            }
            if let Some(info) = addr_info.get(&ev.addr) {
                if matches!(info.life, AddrLife::Live) {
                    return Err(format!(
                        "address {} already live before ReallocIn event_id {}",
                        ev.addr, ev.event_id
                    ));
                }
            }
            addr_info.insert(
                ev.addr.clone(),
                AddrInfo {
                    root_addr: ev.root_addr.clone(),
                    parent_addr: ev.parent_addr.clone(),
                    generation: ev.generation,
                    life: AddrLife::Live,
                    last_event_id: ev.event_id,
                    last_event_kind: ev.kind.clone(),
                },
            );
        } else {
            let Some(info) = addr_info.get_mut(&ev.addr) else {
                return Err(format!(
                    "address {} missing before {:?} event_id {}",
                    ev.addr, ev.kind, ev.event_id
                ));
            };

            if !matches!(info.life, AddrLife::Live) {
                return Err(format!(
                    "address {} is not live before {:?} event_id {}",
                    ev.addr, ev.kind, ev.event_id
                ));
            }

            if info.root_addr != ev.root_addr {
                return Err(format!(
                    "root mismatch for {} in event_id {}: expected {}, got {}",
                    ev.addr, ev.event_id, info.root_addr, ev.root_addr
                ));
            }

            if info.generation != ev.generation {
                return Err(format!(
                    "generation mismatch for {} in event_id {}: expected {}, got {}",
                    ev.addr, ev.event_id, info.generation, ev.generation
                ));
            }

            if info.parent_addr != ev.parent_addr {
                return Err(format!(
                    "parent mismatch for {} in event_id {}",
                    ev.addr, ev.event_id
                ));
            }

            if ev.kind.is_terminal() {
                info.life = AddrLife::Dead;
            }
            info.last_event_id = ev.event_id;
            info.last_event_kind = ev.kind.clone();
        }

        prev_event = Some(ev);
    }

    let mut blocks_by_addr: HashMap<&str, &HeapBlockSnapshot> = HashMap::new();
    for block in blocks {
        if block.addr.is_empty() {
            return Err("heap block has empty address".into());
        }
        if block.root_addr.is_empty() {
            return Err(format!("heap block {} has empty root", block.addr));
        }
        if block.lineage_path.is_empty() {
            return Err(format!("heap block {} has empty lineage_path", block.addr));
        }
        if let Some(parent) = &block.parent_addr {
            if parent.is_empty() {
                return Err(format!("heap block {} has empty parent", block.addr));
            }
            if block.generation == 0 {
                return Err(format!(
                    "heap block {} has parent but generation 0",
                    block.addr
                ));
            }
        }
        HeapLineage::from_parts(
            block.addr.clone(),
            block.root_addr.clone(),
            block.parent_addr.clone(),
            block.generation,
            &block.lineage_path,
        )
        .map_err(|err| format!("heap block {} {err}", block.addr))?;

        if block.trace_total_len < block.event_trace.len() {
            return Err(format!(
                "heap block {} trace_total_len underflows event_trace len",
                block.addr
            ));
        }
        if block.trace_total_len != block.trace_dropped_count + block.event_trace.len() {
            return Err(format!(
                "heap block {} trace counters mismatch: total={}, dropped={}, kept={}",
                block.addr,
                block.trace_total_len,
                block.trace_dropped_count,
                block.event_trace.len()
            ));
        }
        if let Some(last_trace) = block.event_trace.last() {
            let parts: Vec<&str> = last_trace.split('#').collect();
            if parts.len() != 3 {
                return Err(format!(
                    "heap block {} has malformed trace entry '{}'",
                    block.addr, last_trace
                ));
            }
            let trace_event_id = parts[2].parse::<usize>().map_err(|_| {
                format!(
                    "heap block {} has non-numeric trace event id in '{}'",
                    block.addr, last_trace
                )
            })?;
            if trace_event_id != block.event_id {
                return Err(format!(
                    "heap block {} trace/event mismatch: trace_id={}, block_event_id={}",
                    block.addr, trace_event_id, block.event_id
                ));
            }
            if parts[0] != block.last_mutation {
                return Err(format!(
                    "heap block {} trace op mismatch: trace_op={}, last_mutation={}",
                    block.addr, parts[0], block.last_mutation
                ));
            }
        }

        if blocks_by_addr.insert(block.addr.as_str(), block).is_some() {
            return Err(format!("duplicate heap block snapshot for {}", block.addr));
        }
    }

    for (addr, info) in &addr_info {
        let Some(block) = blocks_by_addr.get(addr.as_str()) else {
            return Err(format!("missing heap block snapshot for {}", addr));
        };

        if block.root_addr != info.root_addr {
            return Err(format!(
                "block root mismatch for {}: expected {}, got {}",
                addr, info.root_addr, block.root_addr
            ));
        }
        if block.generation != info.generation {
            return Err(format!(
                "block generation mismatch for {}: expected {}, got {}",
                addr, info.generation, block.generation
            ));
        }
        if block.parent_addr != info.parent_addr {
            return Err(format!("block parent mismatch for {}", addr));
        }
        if block.event_id != info.last_event_id {
            return Err(format!(
                "block event_id mismatch for {}: expected {}, got {}",
                addr, info.last_event_id, block.event_id
            ));
        }
        let expected_mutation = info.last_event_kind.mutation_name();
        if block.last_mutation != expected_mutation {
            return Err(format!(
                "block last_mutation mismatch for {}: expected {}, got {}",
                addr, expected_mutation, block.last_mutation
            ));
        }

        let expected_live = matches!(info.life, AddrLife::Live);
        if block.live != expected_live {
            return Err(format!(
                "block live mismatch for {}: expected {}, got {}",
                addr, expected_live, block.live
            ));
        }

        if block.live {
            if block.death_reason.is_some() {
                return Err(format!("live block {} must not have death reason", addr));
            }
        } else {
            let expected_reason = match info.last_event_kind {
                HeapEventKind::Free => Some(HeapDeathReason::Freed),
                HeapEventKind::ReallocOut => Some(HeapDeathReason::Reallocated),
                HeapEventKind::Unmap => Some(HeapDeathReason::Unmapped),
                HeapEventKind::RemapOut => Some(HeapDeathReason::Remapped),
                _ => None,
            };
            if block.death_reason != expected_reason {
                return Err(format!(
                    "block death reason mismatch for {}: expected {:?}, got {:?}",
                    addr, expected_reason, block.death_reason
                ));
            }
        }
    }

    for addr in blocks_by_addr.keys() {
        if !addr_info.contains_key(*addr) {
            return Err(format!("heap block {} has no backing events", addr));
        }
    }

    Ok(())
}

#[cfg(test)]
mod runtime_heap_audit_tests {
    use super::*;

    fn ev(
        event_id: usize,
        kind: HeapEventKind,
        addr: &str,
        root_addr: &str,
        parent_addr: Option<&str>,
        generation: usize,
    ) -> HeapEventRecord {
        HeapEventRecord {
            event_id,
            kind,
            actor_symbol: None,
            addr: addr.into(),
            root_addr: root_addr.into(),
            parent_addr: parent_addr.map(str::to_string),
            generation,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn block(
        addr: &str,
        root_addr: &str,
        parent_addr: Option<&str>,
        generation: usize,
        event_id: usize,
        live: bool,
        death_reason: Option<HeapDeathReason>,
        last_mutation: &str,
    ) -> HeapBlockSnapshot {
        HeapBlockSnapshot {
            addr: addr.into(),
            live,
            death_reason,
            parent_addr: parent_addr.map(str::to_string),
            root_addr: root_addr.into(),
            lineage_path: if let Some(parent) = parent_addr {
                format!("{}->{}", parent, addr)
            } else {
                addr.into()
            },
            generation,
            event_id,
            last_mutation: last_mutation.into(),
            trace_total_len: 1,
            trace_dropped_count: 0,
            event_trace: vec![format!("{}#-#{}", last_mutation, event_id)],
        }
    }

    #[test]
    fn audit_heap_state_accepts_valid_realloc_timeline_and_blocks() {
        let events = vec![
            ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0),
            ev(2, HeapEventKind::ReallocOut, "0x1010", "0x1010", None, 0),
            ev(
                2,
                HeapEventKind::ReallocIn,
                "0x1020",
                "0x1010",
                Some("0x1010"),
                1,
            ),
            ev(
                3,
                HeapEventKind::Zero,
                "0x1020",
                "0x1010",
                Some("0x1010"),
                1,
            ),
        ];

        let blocks = vec![
            block(
                "0x1010",
                "0x1010",
                None,
                0,
                2,
                false,
                Some(HeapDeathReason::Reallocated),
                "realloc-out",
            ),
            block("0x1020", "0x1010", Some("0x1010"), 1, 3, true, None, "zero"),
        ];

        assert!(audit_heap_state(&events, &blocks).is_ok());
    }

    #[test]
    fn audit_heap_state_rejects_duplicate_non_realloc_event_id() {
        let events = vec![
            ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0),
            ev(1, HeapEventKind::Zero, "0x1010", "0x1010", None, 0),
        ];
        let blocks = vec![block("0x1010", "0x1010", None, 0, 1, true, None, "alloc")];

        let result = audit_heap_state(&events, &blocks);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("duplicate event_id"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn audit_heap_state_rejects_block_snapshot_live_mismatch() {
        let events = vec![
            ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0),
            ev(2, HeapEventKind::Free, "0x1010", "0x1010", None, 0),
        ];
        let blocks = vec![block("0x1010", "0x1010", None, 0, 2, true, None, "free")];

        let result = audit_heap_state(&events, &blocks);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("block live mismatch"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn audit_heap_state_rejects_realloc_parent_mismatch() {
        let events = vec![
            ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0),
            ev(2, HeapEventKind::ReallocOut, "0x1010", "0x1010", None, 0),
            ev(
                2,
                HeapEventKind::ReallocIn,
                "0x1020",
                "0x1010",
                Some("0x9999"),
                1,
            ),
        ];
        let blocks = vec![
            block(
                "0x1010",
                "0x1010",
                None,
                0,
                2,
                false,
                Some(HeapDeathReason::Reallocated),
                "realloc-out",
            ),
            block(
                "0x1020",
                "0x1010",
                Some("0x9999"),
                1,
                2,
                true,
                None,
                "realloc-in",
            ),
        ];

        let result = audit_heap_state(&events, &blocks);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("realloc-in parent mismatch"),
            "unexpected error: {}",
            msg
        );
    }

    #[test]
    fn audit_heap_state_rejects_lineage_depth_mismatch() {
        let events = vec![ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0)];
        let mut b = block("0x1010", "0x1010", None, 0, 1, true, None, "alloc");
        b.lineage_path = "0x1010->0x9999".into();
        let result = audit_heap_state(&events, &[b]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("lineage"),
            "unexpected error for lineage mismatch: {}",
            msg
        );
    }

    #[test]
    fn audit_heap_state_rejects_last_mutation_mismatch() {
        let events = vec![ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0)];
        let b = block("0x1010", "0x1010", None, 0, 1, true, None, "zero");
        let result = audit_heap_state(&events, &[b]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("last_mutation mismatch"),
            "unexpected error for last_mutation mismatch: {}",
            msg
        );
    }

    #[test]
    fn audit_heap_state_rejects_trace_event_id_mismatch() {
        let events = vec![ev(1, HeapEventKind::Alloc, "0x1010", "0x1010", None, 0)];
        let mut b = block("0x1010", "0x1010", None, 0, 1, true, None, "alloc");
        b.event_trace = vec!["alloc#-#2".into()];
        let result = audit_heap_state(&events, &[b]);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("trace/event mismatch"),
            "unexpected error for trace/event mismatch: {}",
            msg
        );
    }

    #[test]
    fn heap_lineage_builds_long_chains_and_stable_siblings() {
        let root = HeapLineage::root("0x1000").expect("root lineage");
        let a = root.child("0x1100").expect("first child");
        let b = a.child("0x1300").expect("second child");
        let c = b.child("0x1700").expect("third child");
        let sibling = a.child("0x1900").expect("sibling child");

        assert_eq!(c.root_addr(), "0x1000");
        assert_eq!(c.parent_addr(), Some("0x1300"));
        assert_eq!(c.generation(), 3);
        assert_eq!(c.path_string(), "0x1000->0x1100->0x1300->0x1700");
        assert_eq!(sibling.root_addr(), "0x1000");
        assert_eq!(sibling.parent_addr(), Some("0x1100"));
        assert_eq!(sibling.generation(), 2);
        assert_eq!(sibling.path_string(), "0x1000->0x1100->0x1900");
    }

    #[test]
    fn audit_heap_state_accepts_four_generation_realloc_chain() {
        let addresses = ["0x1000", "0x1100", "0x1300", "0x1700", "0x1f00"];
        let mut events = vec![ev(
            1,
            HeapEventKind::Alloc,
            addresses[0],
            addresses[0],
            None,
            0,
        )];
        for generation in 0..4 {
            let event_id = generation + 2;
            let parent = if generation == 0 {
                None
            } else {
                Some(addresses[generation - 1])
            };
            events.push(ev(
                event_id,
                HeapEventKind::ReallocOut,
                addresses[generation],
                addresses[0],
                parent,
                generation,
            ));
            events.push(ev(
                event_id,
                HeapEventKind::ReallocIn,
                addresses[generation + 1],
                addresses[0],
                Some(addresses[generation]),
                generation + 1,
            ));
        }

        let mut blocks = Vec::new();
        for generation in 0..=4 {
            let lineage_path = addresses[..=generation].join("->");
            blocks.push(HeapBlockSnapshot {
                addr: addresses[generation].into(),
                live: generation == 4,
                death_reason: if generation == 4 {
                    None
                } else {
                    Some(HeapDeathReason::Reallocated)
                },
                parent_addr: generation.checked_sub(1).map(|idx| addresses[idx].into()),
                root_addr: addresses[0].into(),
                lineage_path,
                generation,
                event_id: if generation == 4 { 5 } else { generation + 2 },
                last_mutation: if generation == 4 {
                    "realloc-in".into()
                } else {
                    "realloc-out".into()
                },
                trace_total_len: 1,
                trace_dropped_count: 0,
                event_trace: vec![format!(
                    "{}#-#{}",
                    if generation == 4 {
                        "realloc-in"
                    } else {
                        "realloc-out"
                    },
                    if generation == 4 { 5 } else { generation + 2 }
                )],
            });
        }

        assert!(audit_heap_state(&events, &blocks).is_ok());
    }
}
