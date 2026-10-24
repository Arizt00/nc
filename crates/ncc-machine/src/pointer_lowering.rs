use crate::{machine_profile, MachineProfile};
use ncc_ast::{
    pointer_type_contract_from_surface, PointerAddressSpace, PointerPointeeKind,
    PointerTypeContract,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachinePointerCarrier {
    NativeRegister,
    NarrowRegister,
    RegisterPair,
    RegisterQuad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineAddressDomain {
    VirtualCpu,
    VolatileMmio,
    Physical,
    DmaIova,
    GpuDevice,
    Firmware,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachinePointerInstructionSet {
    pub add: &'static [&'static str],
    pub subtract: &'static [&'static str],
    pub compare_unsigned: &'static [&'static str],
    pub load: &'static [&'static str],
    pub store: &'static [&'static str],
    pub widen: &'static [&'static str],
    pub truncate: &'static [&'static str],
    pub indirect_call: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachinePointerLoweringPlan {
    pub target: &'static str,
    pub architecture: &'static str,
    pub native_width_bits: u16,
    pub pointer_width_bits: u16,
    pub llvm_address_space: u32,
    pub address_space: PointerAddressSpace,
    pub domain: MachineAddressDomain,
    pub pointee_kind: PointerPointeeKind,
    pub carrier: MachinePointerCarrier,
    pub volatile_access: bool,
    pub hidden_checks: bool,
    pub instructions: MachinePointerInstructionSet,
}

pub fn machine_pointer_lowering_plan(
    target: &str,
    pointer_surface: &str,
) -> Result<MachinePointerLoweringPlan, String> {
    let profile = machine_profile(target)
        .ok_or_else(|| format!("E_MACHINE_PTR_TARGET_001: unsupported target '{target}'"))?;
    let pointer = pointer_type_contract_from_surface(pointer_surface)
        .map_err(|error| format!("E_MACHINE_PTR_TYPE_001: {error}"))?
        .ok_or_else(|| {
            format!("E_MACHINE_PTR_TYPE_001: '{pointer_surface}' is not a pointer type")
        })?;
    build_pointer_plan(profile, pointer)
}

fn build_pointer_plan(
    profile: MachineProfile,
    pointer: PointerTypeContract,
) -> Result<MachinePointerLoweringPlan, String> {
    let native_width_bits = u16::from(profile.pointer_width);
    let pointer_width_bits = pointer.resolved_width_bits(native_width_bits);
    let carrier = match (pointer_width_bits, native_width_bits) {
        (width, native) if width == native => MachinePointerCarrier::NativeRegister,
        (width, native) if width < native => MachinePointerCarrier::NarrowRegister,
        (128, 64) => MachinePointerCarrier::RegisterPair,
        (64, 32) => MachinePointerCarrier::RegisterPair,
        (128, 32) => MachinePointerCarrier::RegisterQuad,
        (width, native) => {
            return Err(format!(
                "E_MACHINE_PTR_WIDTH_001: no raw pointer carrier for {width}-bit pointers on {}-bit target '{}'",
                native, profile.target
            ))
        }
    };
    let domain = match pointer.address_space {
        PointerAddressSpace::Cpu => MachineAddressDomain::VirtualCpu,
        PointerAddressSpace::Mmio => MachineAddressDomain::VolatileMmio,
        PointerAddressSpace::Physical => MachineAddressDomain::Physical,
        PointerAddressSpace::Dma => MachineAddressDomain::DmaIova,
        PointerAddressSpace::Gpu => MachineAddressDomain::GpuDevice,
        PointerAddressSpace::Firmware => MachineAddressDomain::Firmware,
    };
    Ok(MachinePointerLoweringPlan {
        target: profile.target,
        architecture: profile.architecture,
        native_width_bits,
        pointer_width_bits,
        llvm_address_space: pointer.llvm_address_space(native_width_bits),
        address_space: pointer.address_space,
        domain,
        pointee_kind: pointer.pointee_kind,
        carrier,
        volatile_access: pointer.address_space == PointerAddressSpace::Mmio,
        hidden_checks: false,
        instructions: pointer_instructions(profile.architecture, carrier)?,
    })
}

fn pointer_instructions(
    architecture: &str,
    carrier: MachinePointerCarrier,
) -> Result<MachinePointerInstructionSet, String> {
    let set = match (architecture, carrier) {
        ("x86_64", MachinePointerCarrier::NativeRegister) => instructions(
            &["lea", "add"], &["sub"], &["cmp", "setb", "seta"], &["mov"],
            &["mov"], &["movzx"], &["mov"], &["call r/m64"],
        ),
        ("x86_64", MachinePointerCarrier::NarrowRegister) => instructions(
            &["lea", "add"], &["sub"], &["cmp", "setb", "seta"], &["movzx", "mov"],
            &["mov"], &["movzx"], &["mov r16/r32"], &["call r/m64"],
        ),
        ("x86_64", MachinePointerCarrier::RegisterPair) => instructions(
            &["add", "adc"], &["sub", "sbb"], &["cmp", "sbb", "setb", "seta"],
            &["mov", "mov"], &["mov", "mov"], &["xor", "mov"], &["mov"],
            &["call r/m64"],
        ),
        ("i386", MachinePointerCarrier::NativeRegister) => instructions(
            &["lea", "add"], &["sub"], &["cmp", "setb", "seta"], &["mov"],
            &["mov"], &["movzx"], &["mov"], &["call r/m32"],
        ),
        ("i386", MachinePointerCarrier::NarrowRegister) => instructions(
            &["lea", "add"], &["sub"], &["cmp", "setb", "seta"], &["movzx", "mov"],
            &["mov"], &["movzx"], &["mov r16"], &["call r/m32"],
        ),
        ("i386", MachinePointerCarrier::RegisterPair) => instructions(
            &["add", "adc"], &["sub", "sbb"], &["cmp", "sbb", "setb", "seta"],
            &["mov", "mov"], &["mov", "mov"], &["xor", "mov"], &["mov"],
            &["call r/m32"],
        ),
        ("i386", MachinePointerCarrier::RegisterQuad) => instructions(
            &["add", "adc", "adc", "adc"], &["sub", "sbb", "sbb", "sbb"],
            &["cmp", "sbb", "sbb", "sbb", "setb", "seta"],
            &["mov", "mov", "mov", "mov"], &["mov", "mov", "mov", "mov"],
            &["xor", "mov"], &["mov"], &["call r/m32"],
        ),
        ("aarch64", MachinePointerCarrier::NativeRegister) => instructions(
            &["add", "adr", "adrp"], &["sub"], &["cmp", "cset lo", "cset hi"],
            &["ldr"], &["str"], &["uxtw"], &["mov wN,wM"], &["blr"],
        ),
        ("aarch64", MachinePointerCarrier::NarrowRegister) => instructions(
            &["add"], &["sub"], &["cmp", "cset lo", "cset hi"], &["ldrh", "ldr"],
            &["strh", "str"], &["uxth", "uxtw"], &["mov wN,wM"], &["blr"],
        ),
        ("aarch64", MachinePointerCarrier::RegisterPair) => instructions(
            &["adds", "adc"], &["subs", "sbc"], &["cmp", "ccmp", "cset lo", "cset hi"],
            &["ldp"], &["stp"], &["mov", "mov"], &["mov"], &["blr"],
        ),
        ("armv7a-thumb2", MachinePointerCarrier::NativeRegister) => instructions(
            &["add", "adr"], &["sub"], &["cmp", "it", "movlo", "movhi"], &["ldr"],
            &["str"], &["uxth"], &["mov"], &["blx"],
        ),
        ("armv7a-thumb2", MachinePointerCarrier::NarrowRegister) => instructions(
            &["add"], &["sub"], &["cmp", "it", "movlo", "movhi"], &["ldrh"],
            &["strh"], &["uxth"], &["mov"], &["blx"],
        ),
        ("armv7a-thumb2", MachinePointerCarrier::RegisterPair) => instructions(
            &["adds", "adc"], &["subs", "sbc"], &["cmp", "sbc", "it"],
            &["ldrd"], &["strd"], &["mov", "mov"], &["mov"], &["blx"],
        ),
        ("armv7a-thumb2", MachinePointerCarrier::RegisterQuad) => instructions(
            &["adds", "adc", "adc", "adc"], &["subs", "sbc", "sbc", "sbc"],
            &["cmp", "sbc", "sbc", "sbc", "it"], &["ldmia"], &["stmia"],
            &["mov", "mov"], &["mov"], &["blx"],
        ),
        ("riscv64", MachinePointerCarrier::NativeRegister) => instructions(
            &["add", "addi", "auipc"], &["sub"], &["sltu", "xor", "seqz"], &["ld"],
            &["sd"], &["zext.w"], &["slli", "srli"], &["jalr"],
        ),
        ("riscv64", MachinePointerCarrier::NarrowRegister) => instructions(
            &["add", "addi"], &["sub"], &["sltu", "xor", "seqz"], &["lhu", "lwu"],
            &["sh", "sw"], &["zext.h", "zext.w"], &["slli", "srli"], &["jalr"],
        ),
        ("riscv64", MachinePointerCarrier::RegisterPair) => instructions(
            &["add", "sltu", "add"], &["sub", "sltu", "sub"], &["sltu", "xor", "seqz"],
            &["ld", "ld"], &["sd", "sd"], &["li", "mv"], &["mv"], &["jalr"],
        ),
        ("riscv32imac", MachinePointerCarrier::NativeRegister) => instructions(
            &["add", "addi", "auipc"], &["sub"], &["sltu", "xor", "seqz"], &["lw"],
            &["sw"], &["zext.h"], &["slli", "srli"], &["jalr"],
        ),
        ("riscv32imac", MachinePointerCarrier::NarrowRegister) => instructions(
            &["add", "addi"], &["sub"], &["sltu", "xor", "seqz"], &["lhu"],
            &["sh"], &["zext.h"], &["slli", "srli"], &["jalr"],
        ),
        ("riscv32imac", MachinePointerCarrier::RegisterPair) => instructions(
            &["add", "sltu", "add"], &["sub", "sltu", "sub"], &["sltu", "xor", "seqz"],
            &["lw", "lw"], &["sw", "sw"], &["li", "mv"], &["mv"], &["jalr"],
        ),
        ("riscv32imac", MachinePointerCarrier::RegisterQuad) => instructions(
            &["add", "sltu", "add", "sltu", "add", "sltu", "add"],
            &["sub", "sltu", "sub", "sltu", "sub", "sltu", "sub"],
            &["sltu", "xor", "or", "seqz"], &["lw", "lw", "lw", "lw"],
            &["sw", "sw", "sw", "sw"], &["li", "mv"], &["mv"], &["jalr"],
        ),
        _ => {
            return Err(format!(
                "E_MACHINE_PTR_ARCH_001: pointer carrier {carrier:?} is not implemented for architecture '{architecture}'"
            ))
        }
    };
    Ok(set)
}

#[allow(clippy::too_many_arguments)]
fn instructions(
    add: &'static [&'static str],
    subtract: &'static [&'static str],
    compare_unsigned: &'static [&'static str],
    load: &'static [&'static str],
    store: &'static [&'static str],
    widen: &'static [&'static str],
    truncate: &'static [&'static str],
    indirect_call: &'static [&'static str],
) -> MachinePointerInstructionSet {
    MachinePointerInstructionSet {
        add,
        subtract,
        compare_unsigned,
        load,
        store,
        widen,
        truncate,
        indirect_call,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGETS: [&str; 6] = [
        "x86_64-unknown-linux-gnu",
        "i386-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "armv7-unknown-none-eabi",
        "riscv32imac-unknown-elf",
        "riscv64-unknown-elf",
    ];

    #[test]
    fn all_required_targets_lower_every_pointer_width_without_hidden_checks() {
        for target in TARGETS {
            for width in [16, 32, 64, 128] {
                let plan = machine_pointer_lowering_plan(target, &format!("p<Packet,dma,{width}>"))
                    .unwrap_or_else(|error| panic!("{target}/{width}: {error}"));
                assert_eq!(plan.pointer_width_bits, width);
                assert_eq!(plan.address_space, PointerAddressSpace::Dma);
                assert_eq!(plan.domain, MachineAddressDomain::DmaIova);
                assert_eq!(plan.pointee_kind, PointerPointeeKind::Aggregate);
                assert!(!plan.hidden_checks);
                assert!(!plan.instructions.add.is_empty());
                assert!(!plan.instructions.subtract.is_empty());
                assert!(!plan.instructions.compare_unsigned.is_empty());
                assert!(!plan.instructions.load.is_empty());
                assert!(!plan.instructions.store.is_empty());
                assert!(!plan.instructions.indirect_call.is_empty());
            }
        }
    }

    #[test]
    fn address_domains_and_function_flexible_and_member_pointers_are_preserved() {
        for (surface, domain) in [
            ("p<u32,mmio>", MachineAddressDomain::VolatileMmio),
            ("p<byte,physical>", MachineAddressDomain::Physical),
            ("p<byte,gpu>", MachineAddressDomain::GpuDevice),
            ("p<byte,firmware>", MachineAddressDomain::Firmware),
        ] {
            let plan = machine_pointer_lowering_plan("x86_64-unknown-linux-gnu", surface).unwrap();
            assert_eq!(plan.domain, domain);
            assert_eq!(
                plan.volatile_access,
                domain == MachineAddressDomain::VolatileMmio
            );
        }
        assert_eq!(
            machine_pointer_lowering_plan("aarch64-unknown-linux-gnu", "fn<int,bool>")
                .unwrap()
                .pointee_kind,
            PointerPointeeKind::Function
        );
        assert_eq!(
            machine_pointer_lowering_plan("i386-unknown-linux-gnu", "p<byte[]>")
                .unwrap()
                .pointee_kind,
            PointerPointeeKind::FlexibleArray
        );
        assert_eq!(
            machine_pointer_lowering_plan("riscv64-unknown-elf", "member<Packet,u32>")
                .unwrap()
                .pointee_kind,
            PointerPointeeKind::Member
        );
    }
}
