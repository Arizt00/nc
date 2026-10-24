use crate::{
    KERNEL_BOOT_INFO_MAGIC, KERNEL_PLATFORM_MAGIC, MEMORY_KIND_ACPI_NVS, MEMORY_KIND_ACPI_RECLAIM,
    MEMORY_KIND_BAD, MEMORY_KIND_LOADER, MEMORY_KIND_MMIO, MEMORY_KIND_PERSISTENT,
    MEMORY_KIND_RESERVED, MEMORY_KIND_RUNTIME, MEMORY_KIND_USABLE, MEMORY_SOURCE_UEFI,
};
use core::fmt;
use std::fmt::Write as _;

const ELF_HEADER_SIZE: usize = 64;
const ELF_PROGRAM_HEADER_SIZE: usize = 56;
const UEFI_PAGE_SIZE: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UefiLoaderTarget {
    X86_64,
    Aarch64,
}

impl UefiLoaderTarget {
    pub const fn elf_machine(self) -> u16 {
        match self {
            Self::X86_64 => 62,
            Self::Aarch64 => 183,
        }
    }

    pub const fn clang_triple(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64-pc-windows-msvc",
            Self::Aarch64 => "aarch64-pc-windows-msvc",
        }
    }

    pub const fn coff_machine(self) -> &'static str {
        match self {
            Self::X86_64 => "x64",
            Self::Aarch64 => "arm64",
        }
    }

    pub const fn removable_boot_name(self) -> &'static str {
        match self {
            Self::X86_64 => "BOOTX64.EFI",
            Self::Aarch64 => "BOOTAA64.EFI",
        }
    }

    pub const fn fat12_short_name(self) -> &'static [u8; 11] {
        match self {
            Self::X86_64 => b"BOOTX64 EFI",
            Self::Aarch64 => b"BOOTAA64EFI",
        }
    }

    fn efi_calling_convention(self) -> &'static str {
        match self {
            Self::X86_64 => "win64cc ",
            Self::Aarch64 => "",
        }
    }

    fn kernel_calling_convention(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64_sysvcc ",
            Self::Aarch64 => "",
        }
    }

    fn park_instruction(self) -> &'static str {
        match self {
            Self::X86_64 => "hlt",
            Self::Aarch64 => "wfi",
        }
    }

    fn result_reporter_ir(self) -> String {
        let framebuffer_reporter = r#"
define internal void @__ncc_uefi_report_framebuffer(i1 %success) {
entry:
  %boot.flags.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 16
  %boot.flags = load i64, ptr %boot.flags.ptr, align 8
  %fb.flag = and i64 %boot.flags, 8
  %fb.present = icmp ne i64 %fb.flag, 0
  br i1 %fb.present, label %paint, label %finish
paint:
  %fb.ptr.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 40
  %fb.address = load i64, ptr %fb.ptr.ptr, align 8
  %fb.nonzero = icmp ne i64 %fb.address, 0
  br i1 %fb.nonzero, label %store, label %finish
store:
  %fb = inttoptr i64 %fb.address to ptr
  %color = select i1 %success, i32 65280, i32 16711935
  store volatile i32 %color, ptr %fb, align 4
  ret void
finish:
  ret void
}
"#;
        if self != Self::X86_64 {
            return format!(
                "{framebuffer_reporter}\ndefine internal void @__ncc_uefi_report_result(i64 %result) {{\nentry:\n  %success = icmp ne i64 %result, 0\n  call void @__ncc_uefi_report_framebuffer(i1 %success)\n  ret void\n}}\n"
            );
        }

        let success = b"nC kernel x86_64 UEFI kmain\r\n";
        let failure = b"nC kernel x86_64 UEFI kmain bad\r\n";
        format!(
            r#"
@__ncc_uefi_result_success = private constant [{success_len} x i8] c"{success_bytes}", align 1
@__ncc_uefi_result_failure = private constant [{failure_len} x i8] c"{failure_bytes}", align 1
{framebuffer_reporter}
define internal void @__ncc_uefi_report_serial(ptr %message, i64 %length) {{
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %index, %length
  br i1 %done, label %finish, label %body
body:
  %source = getelementptr i8, ptr %message, i64 %index
  %character = load i8, ptr %source, align 1
  call void asm sideeffect "outb $0, $1", "{{al}},{{dx}},~{{memory}},~{{dirflag}},~{{fpsr}},~{{flags}}"(i8 %character, i16 1016)
  %next = add nuw i64 %index, 1
  br label %loop
finish:
  ret void
}}

define internal void @__ncc_uefi_report_result(i64 %result) {{
entry:
  %success = icmp ne i64 %result, 0
  call void @__ncc_uefi_report_framebuffer(i1 %success)
  br i1 %success, label %ok, label %bad
ok:
  call void @__ncc_uefi_report_serial(ptr @__ncc_uefi_result_success, i64 {success_len})
  ret void
bad:
  call void @__ncc_uefi_report_serial(ptr @__ncc_uefi_result_failure, i64 {failure_len})
  ret void
}}
"#,
            success_len = success.len(),
            success_bytes = llvm_bytes(success),
            failure_len = failure.len(),
            failure_bytes = llvm_bytes(failure),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UefiLoaderError {
    InvalidElf(&'static str),
    UnsupportedMachine { expected: u16, actual: u16 },
    ArithmeticOverflow,
}

impl fmt::Display for UefiLoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidElf(reason) => write!(f, "invalid kernel ELF for UEFI: {reason}"),
            Self::UnsupportedMachine { expected, actual } => write!(
                f,
                "kernel ELF machine {actual} does not match UEFI target machine {expected}"
            ),
            Self::ArithmeticOverflow => write!(f, "UEFI loader address arithmetic overflow"),
        }
    }
}

impl std::error::Error for UefiLoaderError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UefiKernelSegment {
    pub file_offset: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub physical_address: u64,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UefiAllocationSpan {
    pub base: u64,
    pub pages: u64,
    pub memory_type: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UefiLoaderPlan {
    pub target: UefiLoaderTarget,
    pub entry: u64,
    pub segments: Vec<UefiKernelSegment>,
    pub allocations: Vec<UefiAllocationSpan>,
    pub kernel_elf: Vec<u8>,
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, UefiLoaderError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(UefiLoaderError::InvalidElf("truncated u16"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, UefiLoaderError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(UefiLoaderError::InvalidElf("truncated u32"))?;
    Ok(u32::from_le_bytes(
        value.try_into().expect("four-byte slice"),
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, UefiLoaderError> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or(UefiLoaderError::InvalidElf("truncated u64"))?;
    Ok(u64::from_le_bytes(
        value.try_into().expect("eight-byte slice"),
    ))
}

fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}

fn align_up(value: u64, align: u64) -> Result<u64, UefiLoaderError> {
    value
        .checked_add(align - 1)
        .map(|value| value & !(align - 1))
        .ok_or(UefiLoaderError::ArithmeticOverflow)
}

impl UefiLoaderPlan {
    pub fn from_elf(kernel_elf: &[u8], target: UefiLoaderTarget) -> Result<Self, UefiLoaderError> {
        if kernel_elf.len() < ELF_HEADER_SIZE
            || kernel_elf.get(0..4) != Some(b"\x7fELF")
            || kernel_elf[4] != 2
            || kernel_elf[5] != 1
            || kernel_elf[6] != 1
        {
            return Err(UefiLoaderError::InvalidElf(
                "expected a little-endian ELF64 executable",
            ));
        }
        let elf_type = read_u16(kernel_elf, 16)?;
        if !matches!(elf_type, 2 | 3) {
            return Err(UefiLoaderError::InvalidElf("expected ET_EXEC or ET_DYN"));
        }
        let machine = read_u16(kernel_elf, 18)?;
        if machine != target.elf_machine() {
            return Err(UefiLoaderError::UnsupportedMachine {
                expected: target.elf_machine(),
                actual: machine,
            });
        }
        let entry = read_u64(kernel_elf, 24)?;
        let phoff = usize::try_from(read_u64(kernel_elf, 32)?)
            .map_err(|_| UefiLoaderError::InvalidElf("program headers exceed host size"))?;
        let phentsize = usize::from(read_u16(kernel_elf, 54)?);
        let phnum = usize::from(read_u16(kernel_elf, 56)?);
        if phentsize < ELF_PROGRAM_HEADER_SIZE || phnum == 0 {
            return Err(UefiLoaderError::InvalidElf("missing ELF64 program headers"));
        }

        let mut segments = Vec::new();
        for index in 0..phnum {
            let header = phoff
                .checked_add(
                    index
                        .checked_mul(phentsize)
                        .ok_or(UefiLoaderError::ArithmeticOverflow)?,
                )
                .ok_or(UefiLoaderError::ArithmeticOverflow)?;
            if read_u32(kernel_elf, header)? != 1 {
                continue;
            }
            let flags = read_u32(kernel_elf, header + 4)?;
            let file_offset = read_u64(kernel_elf, header + 8)?;
            let virtual_address = read_u64(kernel_elf, header + 16)?;
            let physical_raw = read_u64(kernel_elf, header + 24)?;
            let file_size = read_u64(kernel_elf, header + 32)?;
            let memory_size = read_u64(kernel_elf, header + 40)?;
            let physical_address = if physical_raw == 0 {
                virtual_address
            } else {
                physical_raw
            };
            if file_size > memory_size || memory_size == 0 || physical_address == 0 {
                return Err(UefiLoaderError::InvalidElf("invalid PT_LOAD sizes/address"));
            }
            let file_end = file_offset
                .checked_add(file_size)
                .ok_or(UefiLoaderError::ArithmeticOverflow)?;
            if file_end > kernel_elf.len() as u64 {
                return Err(UefiLoaderError::InvalidElf(
                    "PT_LOAD bytes leave the ELF image",
                ));
            }
            physical_address
                .checked_add(memory_size)
                .ok_or(UefiLoaderError::ArithmeticOverflow)?;
            segments.push(UefiKernelSegment {
                file_offset,
                file_size,
                memory_size,
                physical_address,
                flags,
            });
        }
        if segments.is_empty() {
            return Err(UefiLoaderError::InvalidElf("no PT_LOAD segment"));
        }
        if !segments.iter().any(|segment| {
            segment.flags & 1 != 0
                && entry >= segment.physical_address
                && entry < segment.physical_address.saturating_add(segment.memory_size)
        }) {
            return Err(UefiLoaderError::InvalidElf(
                "entry is outside executable PT_LOAD memory",
            ));
        }

        let mut ranges = segments
            .iter()
            .map(|segment| {
                let base = align_down(segment.physical_address, UEFI_PAGE_SIZE);
                let end = align_up(
                    segment
                        .physical_address
                        .checked_add(segment.memory_size)
                        .ok_or(UefiLoaderError::ArithmeticOverflow)?,
                    UEFI_PAGE_SIZE,
                )?;
                Ok((base, end, if segment.flags & 1 != 0 { 1 } else { 2 }))
            })
            .collect::<Result<Vec<_>, UefiLoaderError>>()?;
        ranges.sort_unstable_by_key(|range| range.0);
        let mut merged: Vec<(u64, u64, u32)> = Vec::new();
        for (base, end, memory_type) in ranges {
            if let Some(last) = merged.last_mut() {
                if base < last.1 {
                    last.1 = last.1.max(end);
                    if memory_type == 1 {
                        last.2 = 1;
                    }
                    continue;
                }
                if base == last.1 && memory_type == last.2 {
                    last.1 = end;
                    continue;
                }
            }
            merged.push((base, end, memory_type));
        }
        let allocations = merged
            .into_iter()
            .map(|(base, end, memory_type)| UefiAllocationSpan {
                base,
                pages: (end - base) / UEFI_PAGE_SIZE,
                memory_type,
            })
            .collect();

        Ok(Self {
            target,
            entry,
            segments,
            allocations,
            kernel_elf: kernel_elf.to_vec(),
        })
    }

    pub fn render_llvm_ir(&self) -> Result<String, UefiLoaderError> {
        let mut ir = String::new();
        writeln!(ir, "target triple = \"{}\"", self.target.clang_triple()).unwrap();
        writeln!(ir).unwrap();
        writeln!(
            ir,
            "@__ncc_uefi_kernel_elf = private constant [{} x i8] c\"{}\", align 16",
            self.kernel_elf.len(),
            llvm_bytes(&self.kernel_elf)
        )
        .unwrap();
        let efi_cc = self.target.efi_calling_convention();
        let kernel_cc = self.target.kernel_calling_convention();
        let mut helpers = UEFI_GLOBALS_AND_HELPERS.replace("__EFI_CC__", efi_cc);
        helpers = helpers.replace("__BOOT_MAGIC__", &KERNEL_BOOT_INFO_MAGIC.to_string());
        helpers = helpers.replace("__PLATFORM_MAGIC__", &KERNEL_PLATFORM_MAGIC.to_string());
        for (name, value) in [
            ("__MEMORY_RESERVED__", MEMORY_KIND_RESERVED),
            ("__MEMORY_LOADER__", MEMORY_KIND_LOADER),
            ("__MEMORY_USABLE__", MEMORY_KIND_USABLE),
            ("__MEMORY_ACPI_RECLAIM__", MEMORY_KIND_ACPI_RECLAIM),
            ("__MEMORY_ACPI_NVS__", MEMORY_KIND_ACPI_NVS),
            ("__MEMORY_BAD__", MEMORY_KIND_BAD),
            ("__MEMORY_RUNTIME__", MEMORY_KIND_RUNTIME),
            ("__MEMORY_MMIO__", MEMORY_KIND_MMIO),
            ("__MEMORY_PERSISTENT__", MEMORY_KIND_PERSISTENT),
            ("__MEMORY_SOURCE_UEFI__", MEMORY_SOURCE_UEFI),
        ] {
            helpers = helpers.replace(name, &value.to_string());
        }
        ir.push_str(&helpers);
        ir.push_str(&self.target.result_reporter_ir());

        let mut entry = UEFI_ENTRY_TEMPLATE.replace("__EFI_CC__", efi_cc);
        entry = entry.replace("__KERNEL_CC__", kernel_cc);
        entry = entry.replace("__KERNEL_ENTRY__", &self.entry.to_string());
        entry = entry.replace("__BOOT_MAGIC__", &KERNEL_BOOT_INFO_MAGIC.to_string());
        entry = entry.replace("__KERNEL_ELF_SIZE__", &self.kernel_elf.len().to_string());
        entry = entry.replace("__PARK_INSTRUCTION__", self.target.park_instruction());

        let mut entry_allocas = String::new();
        for index in 0..self.allocations.len() {
            writeln!(
                entry_allocas,
                "  %load.address.{index} = alloca i64, align 8"
            )
            .unwrap();
        }
        entry = entry.replace("__LOAD_ADDRESS_ALLOCAS__", &entry_allocas);

        let mut loads = String::new();
        for (index, span) in self.allocations.iter().enumerate() {
            writeln!(
                loads,
                "  store i64 {}, ptr %load.address.{index}, align 8\n  %load.status.{index} = call {efi_cc}i64 %allocate.pages(i32 2, i32 {}, i64 {}, ptr %load.address.{index})\n  %load.failed.{index} = icmp slt i64 %load.status.{index}, 0\n  br i1 %load.failed.{index}, label %return.load.{index}, label %load.next.{index}\nreturn.load.{index}:\n  ret i64 %load.status.{index}\nload.next.{index}:",
                span.base, span.memory_type, span.pages
            )
            .unwrap();
        }
        for (index, segment) in self.segments.iter().enumerate() {
            writeln!(
                loads,
                "  %segment.source.{index} = getelementptr [{} x i8], ptr @__ncc_uefi_kernel_elf, i64 0, i64 {}\n  %segment.destination.{index} = inttoptr i64 {} to ptr\n  call void @__ncc_uefi_copy(ptr %segment.destination.{index}, ptr %segment.source.{index}, i64 {})",
                self.kernel_elf.len(),
                segment.file_offset,
                segment.physical_address,
                segment.file_size
            )
            .unwrap();
            if segment.memory_size > segment.file_size {
                writeln!(
                    loads,
                    "  %segment.bss.{index} = getelementptr i8, ptr %segment.destination.{index}, i64 {}\n  call void @__ncc_uefi_zero(ptr %segment.bss.{index}, i64 {})",
                    segment.file_size,
                    segment.memory_size - segment.file_size
                )
                .unwrap();
            }
        }
        entry = entry.replace("__LOAD_KERNEL_SEGMENTS__", &loads);
        ir.push_str(&entry);
        Ok(ir)
    }
}

fn llvm_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        write!(encoded, "\\{byte:02X}").expect("writing to String cannot fail");
    }
    encoded
}

const UEFI_GLOBALS_AND_HELPERS: &str = r#"
@__ncc_uefi_boot_info = internal global [88 x i8] zeroinitializer, align 16
@__ncc_uefi_framebuffer = internal global [32 x i8] zeroinitializer, align 16
@__ncc_uefi_platform = internal global [336 x i8] zeroinitializer, align 16
@__ncc_uefi_command_line = internal global [4096 x i8] zeroinitializer, align 16
@__ncc_uefi_image_relocation = internal global ptr @efi_main, align 8
@__ncc_uefi_gop_guid = private constant [16 x i8] c"\DE\A9\42\90\DC\23\38\4A\96\FB\7A\DE\D0\80\51\6A"
@__ncc_uefi_loaded_image_guid = private constant [16 x i8] c"\A1\31\1B\5B\62\95\D2\11\8E\3F\00\A0\C9\69\72\3B"
@__ncc_uefi_acpi20_guid = private constant [16 x i8] c"\71\E8\68\88\F1\E4\D3\11\BC\22\00\80\C7\3C\88\81"
@__ncc_uefi_acpi10_guid = private constant [16 x i8] c"\30\2D\9D\EB\88\2D\D3\11\9A\16\00\90\27\3F\C1\4D"
@__ncc_uefi_smbios_guid = private constant [16 x i8] c"\31\31\4D\EB\2D\1A\D3\11\9A\16\00\90\27\3F\C1\4D"
@__ncc_uefi_smbios3_guid = private constant [16 x i8] c"\44\15\85\F2\9E\58\0D\4A\A7\0B\A2\6B\7C\AF\E5\E8"
@__ncc_uefi_fdt_guid = private constant [16 x i8] c"\D5\21\B6\B1\9C\F1\A5\41\83\0B\D9\15\2C\69\AA\E0"

declare { i64, i1 } @llvm.uadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.umul.with.overflow.i64(i64, i64)

define internal void @__ncc_uefi_copy(ptr %destination, ptr %source, i64 %length) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %index, %length
  br i1 %done, label %finish, label %body
body:
  %source.byte = getelementptr i8, ptr %source, i64 %index
  %destination.byte = getelementptr i8, ptr %destination, i64 %index
  %value = load volatile i8, ptr %source.byte, align 1
  store volatile i8 %value, ptr %destination.byte, align 1
  %next = add nuw i64 %index, 1
  br label %loop
finish:
  ret void
}

define internal void @__ncc_uefi_zero(ptr %destination, i64 %length) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %index, %length
  br i1 %done, label %finish, label %body
body:
  %destination.byte = getelementptr i8, ptr %destination, i64 %index
  store volatile i8 0, ptr %destination.byte, align 1
  %next = add nuw i64 %index, 1
  br label %loop
finish:
  ret void
}

define internal i1 @__ncc_uefi_guid_equal(ptr %left, ptr %right) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %same ]
  %done = icmp eq i64 %index, 16
  br i1 %done, label %yes, label %compare
compare:
  %left.byte = getelementptr i8, ptr %left, i64 %index
  %right.byte = getelementptr i8, ptr %right, i64 %index
  %a = load i8, ptr %left.byte, align 1
  %b = load i8, ptr %right.byte, align 1
  %equal = icmp eq i8 %a, %b
  br i1 %equal, label %same, label %no
same:
  %next = add nuw i64 %index, 1
  br label %loop
yes:
  ret i1 true
no:
  ret i1 false
}

define internal i8 @__ncc_uefi_checksum8(ptr %bytes, i64 %length) {
entry:
  br label %loop
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %body ]
  %sum = phi i8 [ 0, %entry ], [ %sum.next, %body ]
  %done = icmp uge i64 %index, %length
  br i1 %done, label %finish, label %body
body:
  %byte.ptr = getelementptr i8, ptr %bytes, i64 %index
  %byte = load i8, ptr %byte.ptr, align 1
  %sum.next = add i8 %sum, %byte
  %next = add nuw i64 %index, 1
  br label %loop
finish:
  ret i8 %sum
}

define internal i1 @__ncc_uefi_signature(ptr %bytes, i64 %value, i32 %length) {
entry:
  br label %loop
loop:
  %index = phi i32 [ 0, %entry ], [ %next, %same ]
  %done = icmp uge i32 %index, %length
  br i1 %done, label %yes, label %compare
compare:
  %wide = zext i32 %index to i64
  %byte.ptr = getelementptr i8, ptr %bytes, i64 %wide
  %byte = load i8, ptr %byte.ptr, align 1
  %shift = mul i32 %index, 8
  %shift.wide = zext i32 %shift to i64
  %shifted = lshr i64 %value, %shift.wide
  %expected = trunc i64 %shifted to i8
  %equal = icmp eq i8 %byte, %expected
  br i1 %equal, label %same, label %no
same:
  %next = add nuw i32 %index, 1
  br label %loop
yes:
  ret i1 true
no:
  ret i1 false
}

define internal i64 @__ncc_uefi_validate_rsdp(ptr %table) {
entry:
  %present = icmp ne ptr %table, null
  br i1 %present, label %signature, label %fail
signature:
  %sig = call i1 @__ncc_uefi_signature(ptr %table, i64 2329016660419433298, i32 8)
  br i1 %sig, label %base.checksum, label %fail
base.checksum:
  %base.sum = call i8 @__ncc_uefi_checksum8(ptr %table, i64 20)
  %base.ok = icmp eq i8 %base.sum, 0
  br i1 %base.ok, label %revision, label %fail
revision:
  %revision.ptr = getelementptr i8, ptr %table, i64 15
  %revision.value = load i8, ptr %revision.ptr, align 1
  %extended = icmp uge i8 %revision.value, 2
  br i1 %extended, label %length, label %legacy
length:
  %length.ptr = getelementptr i8, ptr %table, i64 20
  %length.value = load i32, ptr %length.ptr, align 1
  %length.min = icmp uge i32 %length.value, 36
  %length.max = icmp ule i32 %length.value, 4096
  %length.ok = and i1 %length.min, %length.max
  br i1 %length.ok, label %extended.checksum, label %fail
extended.checksum:
  %length.wide = zext i32 %length.value to i64
  %extended.sum = call i8 @__ncc_uefi_checksum8(ptr %table, i64 %length.wide)
  %extended.ok = icmp eq i8 %extended.sum, 0
  br i1 %extended.ok, label %extended.success, label %fail
extended.success:
  ret i64 %length.wide
legacy:
  ret i64 20
fail:
  ret i64 0
}

define internal i64 @__ncc_uefi_validate_smbios(ptr %table, i1 %version3) {
entry:
  %present = icmp ne ptr %table, null
  br i1 %present, label %dispatch, label %fail
dispatch:
  br i1 %version3, label %v3.signature, label %v2.signature
v3.signature:
  %v3.sig = call i1 @__ncc_uefi_signature(ptr %table, i64 408882598751, i32 5)
  br i1 %v3.sig, label %v3.length, label %fail
v3.length:
  %v3.length.ptr = getelementptr i8, ptr %table, i64 6
  %v3.length.raw = load i8, ptr %v3.length.ptr, align 1
  %v3.length.value = zext i8 %v3.length.raw to i64
  %v3.min = icmp uge i64 %v3.length.value, 24
  %v3.max = icmp ule i64 %v3.length.value, 255
  %v3.valid = and i1 %v3.min, %v3.max
  br i1 %v3.valid, label %v3.checksum, label %fail
v3.checksum:
  %v3.sum = call i8 @__ncc_uefi_checksum8(ptr %table, i64 %v3.length.value)
  %v3.ok = icmp eq i8 %v3.sum, 0
  br i1 %v3.ok, label %v3.success, label %fail
v3.success:
  ret i64 %v3.length.value
v2.signature:
  %v2.sig = call i1 @__ncc_uefi_signature(ptr %table, i64 1598903135, i32 4)
  br i1 %v2.sig, label %v2.length, label %fail
v2.length:
  %v2.length.ptr = getelementptr i8, ptr %table, i64 5
  %v2.length.raw = load i8, ptr %v2.length.ptr, align 1
  %v2.length.value = zext i8 %v2.length.raw to i64
  %v2.min = icmp uge i64 %v2.length.value, 31
  %v2.max = icmp ule i64 %v2.length.value, 255
  %v2.valid = and i1 %v2.min, %v2.max
  br i1 %v2.valid, label %v2.checksum, label %fail
v2.checksum:
  %v2.sum = call i8 @__ncc_uefi_checksum8(ptr %table, i64 %v2.length.value)
  %v2.ok = icmp eq i8 %v2.sum, 0
  br i1 %v2.ok, label %v2.success, label %fail
v2.success:
  ret i64 %v2.length.value
fail:
  ret i64 0
}

define internal i32 @__ncc_uefi_be32(ptr %value) {
entry:
  %raw = load i32, ptr %value, align 1
  %swapped = call i32 @llvm.bswap.i32(i32 %raw)
  ret i32 %swapped
}

declare i32 @llvm.bswap.i32(i32)

define internal i64 @__ncc_uefi_validate_fdt(ptr %table) {
entry:
  %present = icmp ne ptr %table, null
  br i1 %present, label %magic, label %fail
magic:
  %magic.value = call i32 @__ncc_uefi_be32(ptr %table)
  %magic.ok = icmp eq i32 %magic.value, -804389139
  br i1 %magic.ok, label %length, label %fail
length:
  %length.ptr = getelementptr i8, ptr %table, i64 4
  %length.raw = call i32 @__ncc_uefi_be32(ptr %length.ptr)
  %length.min = icmp uge i32 %length.raw, 40
  %length.max = icmp ule i32 %length.raw, 16777216
  %length.ok = and i1 %length.min, %length.max
  br i1 %length.ok, label %success, label %fail
success:
  %length.wide = zext i32 %length.raw to i64
  ret i64 %length.wide
fail:
  ret i64 0
}

define internal i1 @__ncc_uefi_platform_append(i32 %kind, i32 %flags, ptr %table, i64 %length, i64 %revision) {
entry:
  %present = icmp ne ptr %table, null
  br i1 %present, label %capacity, label %fail
capacity:
  %count.ptr = getelementptr i8, ptr @__ncc_uefi_platform, i64 12
  %count = load i32, ptr %count.ptr, align 4
  %room = icmp ult i32 %count, 10
  br i1 %room, label %write, label %fail
write:
  %count.wide = zext i32 %count to i64
  %entry.body = mul i64 %count.wide, 32
  %entry.offset = add i64 %entry.body, 16
  %entry.ptr = getelementptr i8, ptr @__ncc_uefi_platform, i64 %entry.offset
  store i32 %kind, ptr %entry.ptr, align 4
  %flags.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  store i32 %flags, ptr %flags.ptr, align 4
  %table.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %table.address = ptrtoint ptr %table to i64
  store i64 %table.address, ptr %table.ptr, align 8
  %length.ptr = getelementptr i8, ptr %entry.ptr, i64 16
  store i64 %length, ptr %length.ptr, align 8
  %revision.ptr = getelementptr i8, ptr %entry.ptr, i64 24
  store i64 %revision, ptr %revision.ptr, align 8
  %next = add nuw i32 %count, 1
  store i32 %next, ptr %count.ptr, align 4
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_uefi_platform_upsert_preferred(i32 %kind, i32 %flags, ptr %table, i64 %length, i64 %revision) {
entry:
  %present = icmp ne ptr %table, null
  br i1 %present, label %scan.setup, label %fail
scan.setup:
  %count.ptr = getelementptr i8, ptr @__ncc_uefi_platform, i64 12
  %count = load i32, ptr %count.ptr, align 4
  br label %scan
scan:
  %index = phi i32 [ 0, %scan.setup ], [ %next, %continue ]
  %done = icmp uge i32 %index, %count
  br i1 %done, label %append, label %inspect
inspect:
  %index.wide = zext i32 %index to i64
  %entry.body = mul i64 %index.wide, 32
  %entry.offset = add i64 %entry.body, 16
  %entry.ptr = getelementptr i8, ptr @__ncc_uefi_platform, i64 %entry.offset
  %existing.kind = load i32, ptr %entry.ptr, align 4
  %same = icmp eq i32 %existing.kind, %kind
  br i1 %same, label %prefer, label %continue
prefer:
  %revision.ptr = getelementptr i8, ptr %entry.ptr, i64 24
  %existing.revision = load i64, ptr %revision.ptr, align 8
  %newer = icmp ugt i64 %revision, %existing.revision
  br i1 %newer, label %replace, label %success
replace:
  store i32 %kind, ptr %entry.ptr, align 4
  %flags.ptr = getelementptr i8, ptr %entry.ptr, i64 4
  store i32 %flags, ptr %flags.ptr, align 4
  %table.ptr = getelementptr i8, ptr %entry.ptr, i64 8
  %table.address = ptrtoint ptr %table to i64
  store i64 %table.address, ptr %table.ptr, align 8
  %length.ptr = getelementptr i8, ptr %entry.ptr, i64 16
  store i64 %length, ptr %length.ptr, align 8
  store i64 %revision, ptr %revision.ptr, align 8
  br label %success
continue:
  %next = add nuw i32 %index, 1
  br label %scan
append:
  %appended = call i1 @__ncc_uefi_platform_append(i32 %kind, i32 %flags, ptr %table, i64 %length, i64 %revision)
  ret i1 %appended
success:
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_uefi_prepare_contract(ptr %system.table, i64 %kernel.size) {
entry:
  call void @__ncc_uefi_zero(ptr @__ncc_uefi_boot_info, i64 88)
  call void @__ncc_uefi_zero(ptr @__ncc_uefi_framebuffer, i64 32)
  call void @__ncc_uefi_zero(ptr @__ncc_uefi_platform, i64 336)
  call void @__ncc_uefi_zero(ptr @__ncc_uefi_command_line, i64 4096)
  store i64 __BOOT_MAGIC__, ptr @__ncc_uefi_boot_info, align 16
  %version.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 8
  store i64 1, ptr %version.ptr, align 8
  %platform.ptr.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 56
  %platform.address = ptrtoint ptr @__ncc_uefi_platform to i64
  store i64 %platform.address, ptr %platform.ptr.ptr, align 8
  %payload.ptr.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 72
  %payload.address = ptrtoint ptr @__ncc_uefi_kernel_elf to i64
  store i64 %payload.address, ptr %payload.ptr.ptr, align 8
  %payload.size.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 80
  store i64 %kernel.size, ptr %payload.size.ptr, align 8
  store i64 __PLATFORM_MAGIC__, ptr @__ncc_uefi_platform, align 16
  %platform.version.ptr = getelementptr i8, ptr @__ncc_uefi_platform, i64 8
  store i32 1, ptr %platform.version.ptr, align 4
  %header.size.ptr = getelementptr i8, ptr %system.table, i64 12
  %header.size.raw = load i32, ptr %header.size.ptr, align 4
  %header.size = zext i32 %header.size.raw to i64
  %system.recorded = call i1 @__ncc_uefi_platform_append(i32 3, i32 7, ptr %system.table, i64 %header.size, i64 2)
  %flags.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 16
  store i64 4, ptr %flags.ptr, align 8
  ret i1 %system.recorded
}

define internal i1 @__ncc_uefi_capture_config_tables(ptr %system.table) {
entry:
  %count.ptr = getelementptr i8, ptr %system.table, i64 104
  %count = load i64, ptr %count.ptr, align 8
  %tables.ptr.ptr = getelementptr i8, ptr %system.table, i64 112
  %tables = load ptr, ptr %tables.ptr.ptr, align 8
  %count.ok = icmp ule i64 %count, 4096
  %tables.ok = icmp ne ptr %tables, null
  %valid = and i1 %count.ok, %tables.ok
  br i1 %valid, label %loop, label %finish
loop:
  %index = phi i64 [ 0, %entry ], [ %next, %continue ]
  %done = icmp uge i64 %index, %count
  br i1 %done, label %finish, label %body
body:
  %offset = mul i64 %index, 24
  %entry.ptr = getelementptr i8, ptr %tables, i64 %offset
  %table.ptr.ptr = getelementptr i8, ptr %entry.ptr, i64 16
  %table = load ptr, ptr %table.ptr.ptr, align 8
  %acpi20 = call i1 @__ncc_uefi_guid_equal(ptr %entry.ptr, ptr @__ncc_uefi_acpi20_guid)
  br i1 %acpi20, label %record.acpi20, label %acpi10.test
record.acpi20:
  %acpi20.length = call i64 @__ncc_uefi_validate_rsdp(ptr %table)
  %acpi20.valid = icmp ne i64 %acpi20.length, 0
  br i1 %acpi20.valid, label %append.acpi20, label %continue
append.acpi20:
  %acpi20.ok = call i1 @__ncc_uefi_platform_upsert_preferred(i32 2, i32 5, ptr %table, i64 %acpi20.length, i64 2)
  br label %continue
acpi10.test:
  %acpi10 = call i1 @__ncc_uefi_guid_equal(ptr %entry.ptr, ptr @__ncc_uefi_acpi10_guid)
  br i1 %acpi10, label %record.acpi10, label %smbios3.test
record.acpi10:
  %acpi10.length = call i64 @__ncc_uefi_validate_rsdp(ptr %table)
  %acpi10.valid = icmp ne i64 %acpi10.length, 0
  br i1 %acpi10.valid, label %append.acpi10, label %continue
append.acpi10:
  %acpi10.ok = call i1 @__ncc_uefi_platform_upsert_preferred(i32 2, i32 5, ptr %table, i64 %acpi10.length, i64 1)
  br label %continue
smbios3.test:
  %smbios3 = call i1 @__ncc_uefi_guid_equal(ptr %entry.ptr, ptr @__ncc_uefi_smbios3_guid)
  br i1 %smbios3, label %record.smbios3, label %smbios.test
record.smbios3:
  %smbios3.length = call i64 @__ncc_uefi_validate_smbios(ptr %table, i1 true)
  %smbios3.valid = icmp ne i64 %smbios3.length, 0
  br i1 %smbios3.valid, label %append.smbios3, label %continue
append.smbios3:
  %smbios3.ok = call i1 @__ncc_uefi_platform_append(i32 12, i32 5, ptr %table, i64 %smbios3.length, i64 3)
  br label %continue
smbios.test:
  %smbios = call i1 @__ncc_uefi_guid_equal(ptr %entry.ptr, ptr @__ncc_uefi_smbios_guid)
  br i1 %smbios, label %record.smbios, label %fdt.test
record.smbios:
  %smbios.length = call i64 @__ncc_uefi_validate_smbios(ptr %table, i1 false)
  %smbios.valid = icmp ne i64 %smbios.length, 0
  br i1 %smbios.valid, label %append.smbios, label %continue
append.smbios:
  %smbios.ok = call i1 @__ncc_uefi_platform_append(i32 11, i32 5, ptr %table, i64 %smbios.length, i64 2)
  br label %continue
fdt.test:
  %fdt = call i1 @__ncc_uefi_guid_equal(ptr %entry.ptr, ptr @__ncc_uefi_fdt_guid)
  br i1 %fdt, label %record.fdt, label %continue
record.fdt:
  %fdt.length = call i64 @__ncc_uefi_validate_fdt(ptr %table)
  %fdt.valid = icmp ne i64 %fdt.length, 0
  br i1 %fdt.valid, label %append.fdt, label %continue
append.fdt:
  %fdt.ok = call i1 @__ncc_uefi_platform_append(i32 6, i32 5, ptr %table, i64 %fdt.length, i64 17)
  br label %continue
continue:
  %next = add nuw i64 %index, 1
  br label %loop
finish:
  ret i1 true
}

define internal i1 @__ncc_uefi_capture_gop(ptr %boot.services) {
entry:
  %locate.slot = getelementptr i8, ptr %boot.services, i64 320
  %locate = load ptr, ptr %locate.slot, align 8
  %gop.slot = alloca ptr, align 8
  store ptr null, ptr %gop.slot, align 8
  %status = call __EFI_CC__i64 %locate(ptr @__ncc_uefi_gop_guid, ptr null, ptr %gop.slot)
  %ok = icmp eq i64 %status, 0
  br i1 %ok, label %gop, label %fail
gop:
  %gop.value = load ptr, ptr %gop.slot, align 8
  %gop.present = icmp ne ptr %gop.value, null
  br i1 %gop.present, label %mode, label %fail
mode:
  %mode.slot = getelementptr i8, ptr %gop.value, i64 24
  %mode.value = load ptr, ptr %mode.slot, align 8
  %mode.present = icmp ne ptr %mode.value, null
  br i1 %mode.present, label %info, label %fail
info:
  %info.slot = getelementptr i8, ptr %mode.value, i64 8
  %info.value = load ptr, ptr %info.slot, align 8
  %info.present = icmp ne ptr %info.value, null
  br i1 %info.present, label %read, label %fail
read:
  %width.ptr = getelementptr i8, ptr %info.value, i64 4
  %height.ptr = getelementptr i8, ptr %info.value, i64 8
  %format.ptr = getelementptr i8, ptr %info.value, i64 12
  %scan.ptr = getelementptr i8, ptr %info.value, i64 32
  %base.ptr = getelementptr i8, ptr %mode.value, i64 24
  %size.ptr = getelementptr i8, ptr %mode.value, i64 32
  %width = load i32, ptr %width.ptr, align 4
  %height = load i32, ptr %height.ptr, align 4
  %format = load i32, ptr %format.ptr, align 4
  %scan = load i32, ptr %scan.ptr, align 4
  %base = load i64, ptr %base.ptr, align 8
  %size = load i64, ptr %size.ptr, align 8
  %width.ok = icmp ne i32 %width, 0
  %height.ok = icmp ne i32 %height, 0
  %scan.ok = icmp uge i32 %scan, %width
  %format.ok = icmp ule i32 %format, 2
  %base.ok = icmp ne i64 %base, 0
  %size.ok = icmp ne i64 %size, 0
  %scan.wide = zext i32 %scan to i64
  %height.wide = zext i32 %height to i64
  %pitch.wide = mul nuw i64 %scan.wide, 4
  %required.result = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %pitch.wide, i64 %height.wide)
  %required = extractvalue { i64, i1 } %required.result, 0
  %required.overflow = extractvalue { i64, i1 } %required.result, 1
  %required.fits = icmp ule i64 %required, %size
  %required.safe = xor i1 %required.overflow, true
  %buffer.a = and i1 %required.safe, %required.fits
  %a = and i1 %width.ok, %height.ok
  %b = and i1 %scan.ok, %base.ok
  %c = and i1 %a, %b
  %d = and i1 %c, %size.ok
  %e = and i1 %d, %format.ok
  %valid = and i1 %e, %buffer.a
  br i1 %valid, label %store, label %fail
store:
  store i64 %base, ptr @__ncc_uefi_framebuffer, align 16
  %dst.width = getelementptr i8, ptr @__ncc_uefi_framebuffer, i64 8
  %dst.height = getelementptr i8, ptr @__ncc_uefi_framebuffer, i64 12
  %dst.pitch = getelementptr i8, ptr @__ncc_uefi_framebuffer, i64 16
  %dst.format = getelementptr i8, ptr @__ncc_uefi_framebuffer, i64 20
  %dst.flags = getelementptr i8, ptr @__ncc_uefi_framebuffer, i64 24
  %pitch = mul i32 %scan, 4
  store i32 %width, ptr %dst.width, align 4
  store i32 %height, ptr %dst.height, align 4
  store i32 %pitch, ptr %dst.pitch, align 4
  store i32 %format, ptr %dst.format, align 4
  store i64 3, ptr %dst.flags, align 8
  %boot.fb.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 40
  %fb.address = ptrtoint ptr @__ncc_uefi_framebuffer to i64
  store i64 %fb.address, ptr %boot.fb.ptr, align 8
  %boot.flags.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 16
  %boot.flags = load i64, ptr %boot.flags.ptr, align 8
  %with.fb = or i64 %boot.flags, 8
  store i64 %with.fb, ptr %boot.flags.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i1 @__ncc_uefi_capture_command_line(i64 %image, ptr %boot.services) {
entry:
  %handle.slot = getelementptr i8, ptr %boot.services, i64 152
  %handle.protocol = load ptr, ptr %handle.slot, align 8
  %loaded.slot = alloca ptr, align 8
  store ptr null, ptr %loaded.slot, align 8
  %status = call __EFI_CC__i64 %handle.protocol(i64 %image, ptr @__ncc_uefi_loaded_image_guid, ptr %loaded.slot)
  %ok = icmp eq i64 %status, 0
  br i1 %ok, label %loaded, label %fail
loaded:
  %loaded.value = load ptr, ptr %loaded.slot, align 8
  %present = icmp ne ptr %loaded.value, null
  br i1 %present, label %options, label %fail
options:
  %size.ptr = getelementptr i8, ptr %loaded.value, i64 48
  %source.ptr.ptr = getelementptr i8, ptr %loaded.value, i64 56
  %size.raw = load i32, ptr %size.ptr, align 4
  %source = load ptr, ptr %source.ptr.ptr, align 8
  %source.present = icmp ne ptr %source, null
  %size.present = icmp uge i32 %size.raw, 2
  %valid = and i1 %source.present, %size.present
  br i1 %valid, label %loop, label %fail
loop:
  %index = phi i32 [ 0, %options ], [ %next, %write ]
  %byte.offset = mul i32 %index, 2
  %inside = icmp ult i32 %byte.offset, %size.raw
  %room = icmp ult i32 %index, 4095
  %continue = and i1 %inside, %room
  br i1 %continue, label %read, label %terminate
read:
  %wide.offset = zext i32 %byte.offset to i64
  %source.char.ptr = getelementptr i8, ptr %source, i64 %wide.offset
  %character = load i16, ptr %source.char.ptr, align 2
  %end = icmp eq i16 %character, 0
  br i1 %end, label %terminate, label %write
write:
  %ascii = icmp ule i16 %character, 127
  %truncated = trunc i16 %character to i8
  %encoded = select i1 %ascii, i8 %truncated, i8 63
  %wide.index = zext i32 %index to i64
  %destination = getelementptr i8, ptr @__ncc_uefi_command_line, i64 %wide.index
  store i8 %encoded, ptr %destination, align 1
  %next = add nuw i32 %index, 1
  br label %loop
terminate:
  %final.index = phi i32 [ %index, %loop ], [ %index, %read ]
  %final.wide = zext i32 %final.index to i64
  %terminator = getelementptr i8, ptr @__ncc_uefi_command_line, i64 %final.wide
  store i8 0, ptr %terminator, align 1
  %boot.command.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 64
  %command.address = ptrtoint ptr @__ncc_uefi_command_line to i64
  store i64 %command.address, ptr %boot.command.ptr, align 8
  ret i1 true
fail:
  ret i1 false
}

define internal i64 @__ncc_uefi_normalize_memory(ptr %raw, i64 %raw.size, i64 %descriptor.size, ptr %output, i64 %capacity) {
entry:
  %descriptor.valid = icmp uge i64 %descriptor.size, 40
  %output.valid = icmp ne ptr %output, null
  %raw.valid = icmp ne ptr %raw, null
  %pointers.valid = and i1 %output.valid, %raw.valid
  %valid = and i1 %descriptor.valid, %pointers.valid
  br i1 %valid, label %shape, label %fail
shape:
  %remainder = urem i64 %raw.size, %descriptor.size
  %exact = icmp eq i64 %remainder, 0
  br i1 %exact, label %loop, label %fail
loop:
  %offset = phi i64 [ 0, %shape ], [ %next.offset, %advance ]
  %count = phi i64 [ 0, %shape ], [ %next.count, %advance ]
  %required.result = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %offset, i64 40)
  %required = extractvalue { i64, i1 } %required.result, 0
  %required.overflow = extractvalue { i64, i1 } %required.result, 1
  %required.no.overflow = xor i1 %required.overflow, true
  %required.inside = icmp ule i64 %required, %raw.size
  %inside = and i1 %required.no.overflow, %required.inside
  br i1 %inside, label %capacity.check, label %finish
capacity.check:
  %room = icmp ult i64 %count, %capacity
  br i1 %room, label %read, label %fail
read:
  %descriptor = getelementptr i8, ptr %raw, i64 %offset
  %type = load i32, ptr %descriptor, align 1
  %physical.ptr = getelementptr i8, ptr %descriptor, i64 8
  %pages.ptr = getelementptr i8, ptr %descriptor, i64 24
  %attributes.ptr = getelementptr i8, ptr %descriptor, i64 32
  %physical = load i64, ptr %physical.ptr, align 1
  %pages = load i64, ptr %pages.ptr, align 1
  %attributes = load i64, ptr %attributes.ptr, align 1
  %length.result = call { i64, i1 } @llvm.umul.with.overflow.i64(i64 %pages, i64 4096)
  %length = extractvalue { i64, i1 } %length.result, 0
  %length.overflow = extractvalue { i64, i1 } %length.result, 1
  br i1 %length.overflow, label %fail, label %classify
classify:
  %loader.low = icmp uge i32 %type, 1
  %loader.high = icmp ule i32 %type, 4
  %loader = and i1 %loader.low, %loader.high
  %runtime5 = icmp eq i32 %type, 5
  %runtime6 = icmp eq i32 %type, 6
  %runtime = or i1 %runtime5, %runtime6
  %usable = icmp eq i32 %type, 7
  %bad = icmp eq i32 %type, 8
  %acpi = icmp eq i32 %type, 9
  %nvs = icmp eq i32 %type, 10
  %mmio11 = icmp eq i32 %type, 11
  %mmio12 = icmp eq i32 %type, 12
  %mmio = or i1 %mmio11, %mmio12
  %persistent = icmp eq i32 %type, 14
  %kind.loader = select i1 %loader, i32 __MEMORY_LOADER__, i32 __MEMORY_RESERVED__
  %kind.runtime = select i1 %runtime, i32 __MEMORY_RUNTIME__, i32 %kind.loader
  %kind.usable = select i1 %usable, i32 __MEMORY_USABLE__, i32 %kind.runtime
  %kind.bad = select i1 %bad, i32 __MEMORY_BAD__, i32 %kind.usable
  %kind.acpi = select i1 %acpi, i32 __MEMORY_ACPI_RECLAIM__, i32 %kind.bad
  %kind.nvs = select i1 %nvs, i32 __MEMORY_ACPI_NVS__, i32 %kind.acpi
  %kind.mmio = select i1 %mmio, i32 __MEMORY_MMIO__, i32 %kind.nvs
  %kind = select i1 %persistent, i32 __MEMORY_PERSISTENT__, i32 %kind.mmio
  %output.offset = mul i64 %count, 32
  %region = getelementptr i8, ptr %output, i64 %output.offset
  store i64 %physical, ptr %region, align 8
  %length.dst = getelementptr i8, ptr %region, i64 8
  %kind.dst = getelementptr i8, ptr %region, i64 16
  %flags.dst = getelementptr i8, ptr %region, i64 20
  %source.dst = getelementptr i8, ptr %region, i64 24
  %reserved.dst = getelementptr i8, ptr %region, i64 28
  %flags = trunc i64 %attributes to i32
  store i64 %length, ptr %length.dst, align 8
  store i32 %kind, ptr %kind.dst, align 4
  store i32 %flags, ptr %flags.dst, align 4
  store i32 __MEMORY_SOURCE_UEFI__, ptr %source.dst, align 4
  store i32 0, ptr %reserved.dst, align 4
  %next.count = add nuw i64 %count, 1
  %next.offset.result = call { i64, i1 } @llvm.uadd.with.overflow.i64(i64 %offset, i64 %descriptor.size)
  %next.offset = extractvalue { i64, i1 } %next.offset.result, 0
  %next.offset.overflow = extractvalue { i64, i1 } %next.offset.result, 1
  br i1 %next.offset.overflow, label %fail, label %advance
advance:
  br label %loop
finish:
  ret i64 %count
fail:
  ret i64 0
}
"#;

const UEFI_ENTRY_TEMPLATE: &str = r#"
define dso_local __EFI_CC__i64 @efi_main(i64 %image, ptr %system.table) {
entry:
__LOAD_ADDRESS_ALLOCAS__
  %map.capacity = alloca i64, align 8
  %map.size = alloca i64, align 8
  %map.key = alloca i64, align 8
  %descriptor.size = alloca i64, align 8
  %descriptor.version = alloca i32, align 4
  %map.pointer = alloca ptr, align 8
  %normalized.pointer = alloca ptr, align 8
  %normalized.capacity = alloca i64, align 8
  %image.relocation = load volatile ptr, ptr @__ncc_uefi_image_relocation, align 8
  %image.relocation.valid = icmp ne ptr %image.relocation, null
  %system.pointer.present = icmp ne ptr %system.table, null
  %system.present = and i1 %system.pointer.present, %image.relocation.valid
  br i1 %system.present, label %system.valid, label %return.invalid
system.valid:
  %system.signature = load i64, ptr %system.table, align 8
  %signature.valid = icmp eq i64 %system.signature, 6076298535811760713
  br i1 %signature.valid, label %boot.services, label %return.invalid
boot.services:
  %boot.services.slot = getelementptr i8, ptr %system.table, i64 96
  %boot.services.value = load ptr, ptr %boot.services.slot, align 8
  %boot.services.present = icmp ne ptr %boot.services.value, null
  br i1 %boot.services.present, label %services.load, label %return.invalid
services.load:
  %allocate.pages.slot = getelementptr i8, ptr %boot.services.value, i64 40
  %get.memory.map.slot = getelementptr i8, ptr %boot.services.value, i64 56
  %allocate.pool.slot = getelementptr i8, ptr %boot.services.value, i64 64
  %free.pool.slot = getelementptr i8, ptr %boot.services.value, i64 72
  %exit.boot.services.slot = getelementptr i8, ptr %boot.services.value, i64 232
  %allocate.pages = load ptr, ptr %allocate.pages.slot, align 8
  %get.memory.map = load ptr, ptr %get.memory.map.slot, align 8
  %allocate.pool = load ptr, ptr %allocate.pool.slot, align 8
  %free.pool = load ptr, ptr %free.pool.slot, align 8
  %exit.boot.services = load ptr, ptr %exit.boot.services.slot, align 8
  %contract.ok = call i1 @__ncc_uefi_prepare_contract(ptr %system.table, i64 __KERNEL_ELF_SIZE__)
  br i1 %contract.ok, label %load.kernel, label %return.invalid
load.kernel:
__LOAD_KERNEL_SEGMENTS__
  %config.ok = call i1 @__ncc_uefi_capture_config_tables(ptr %system.table)
  %gop.ok = call i1 @__ncc_uefi_capture_gop(ptr %boot.services.value)
  %command.ok = call i1 @__ncc_uefi_capture_command_line(i64 %image, ptr %boot.services.value)
  store i64 0, ptr %map.capacity, align 8
  store ptr null, ptr %map.pointer, align 8
  store ptr null, ptr %normalized.pointer, align 8
  store i64 0, ptr %normalized.capacity, align 8
  br label %map.loop
map.loop:
  %capacity = load i64, ptr %map.capacity, align 8
  store i64 %capacity, ptr %map.size, align 8
  %map.value = load ptr, ptr %map.pointer, align 8
  %map.status = call __EFI_CC__i64 %get.memory.map(ptr %map.size, ptr %map.value, ptr %map.key, ptr %descriptor.size, ptr %descriptor.version)
  %map.small = icmp eq i64 %map.status, -9223372036854775803
  br i1 %map.small, label %map.grow, label %map.status.check
map.grow:
  %required = load i64, ptr %map.size, align 8
  %descriptor.margin.raw = load i64, ptr %descriptor.size, align 8
  %descriptor.margin.valid = icmp uge i64 %descriptor.margin.raw, 40
  %descriptor.margin.base = select i1 %descriptor.margin.valid, i64 %descriptor.margin.raw, i64 48
  %descriptor.margin = mul i64 %descriptor.margin.base, 32
  %with.margin = add i64 %required, %descriptor.margin
  %new.capacity = add i64 %with.margin, 4096
  %old.map = load ptr, ptr %map.pointer, align 8
  %old.present = icmp ne ptr %old.map, null
  br i1 %old.present, label %map.free, label %map.allocate
map.free:
  %map.free.status = call __EFI_CC__i64 %free.pool(ptr %old.map)
  br label %map.allocate
map.allocate:
  %map.allocate.status = call __EFI_CC__i64 %allocate.pool(i32 2, i64 %new.capacity, ptr %map.pointer)
  %map.allocate.ok = icmp eq i64 %map.allocate.status, 0
  br i1 %map.allocate.ok, label %map.capacity.store, label %return.map.allocate
map.capacity.store:
  store i64 %new.capacity, ptr %map.capacity, align 8
  br label %map.loop
map.status.check:
  %map.ok = icmp eq i64 %map.status, 0
  br i1 %map.ok, label %descriptor.check, label %return.map
descriptor.check:
  %final.map.size = load i64, ptr %map.size, align 8
  %final.descriptor.size = load i64, ptr %descriptor.size, align 8
  %descriptor.large.enough = icmp uge i64 %final.descriptor.size, 40
  %descriptor.nonzero = icmp ne i64 %final.descriptor.size, 0
  %descriptor.valid = and i1 %descriptor.large.enough, %descriptor.nonzero
  br i1 %descriptor.valid, label %normalized.check, label %return.invalid
normalized.check:
  %descriptor.count = udiv i64 %final.map.size, %final.descriptor.size
  %normalized.cap = load i64, ptr %normalized.capacity, align 8
  %normalized.room = icmp uge i64 %normalized.cap, %descriptor.count
  br i1 %normalized.room, label %exit.boot, label %normalized.grow
normalized.grow:
  %normalized.old = load ptr, ptr %normalized.pointer, align 8
  %normalized.old.present = icmp ne ptr %normalized.old, null
  br i1 %normalized.old.present, label %normalized.free, label %normalized.allocate
normalized.free:
  %normalized.free.status = call __EFI_CC__i64 %free.pool(ptr %normalized.old)
  br label %normalized.allocate
normalized.allocate:
  %normalized.new.capacity = add i64 %descriptor.count, 32
  %normalized.bytes = mul i64 %normalized.new.capacity, 32
  %normalized.allocate.status = call __EFI_CC__i64 %allocate.pool(i32 2, i64 %normalized.bytes, ptr %normalized.pointer)
  %normalized.allocate.ok = icmp eq i64 %normalized.allocate.status, 0
  br i1 %normalized.allocate.ok, label %normalized.capacity.store, label %return.normalized.allocate
normalized.capacity.store:
  store i64 %normalized.new.capacity, ptr %normalized.capacity, align 8
  br label %map.loop
exit.boot:
  %key = load i64, ptr %map.key, align 8
  %exit.status = call __EFI_CC__i64 %exit.boot.services(i64 %image, i64 %key)
  %exit.ok = icmp eq i64 %exit.status, 0
  br i1 %exit.ok, label %handoff, label %exit.retry.test
exit.retry.test:
  %exit.retry = icmp eq i64 %exit.status, -9223372036854775806
  br i1 %exit.retry, label %map.loop, label %return.exit
handoff:
  %handoff.raw = load ptr, ptr %map.pointer, align 8
  %handoff.raw.size = load i64, ptr %map.size, align 8
  %handoff.descriptor.size = load i64, ptr %descriptor.size, align 8
  %handoff.normalized = load ptr, ptr %normalized.pointer, align 8
  %handoff.capacity = load i64, ptr %normalized.capacity, align 8
  %handoff.count = call i64 @__ncc_uefi_normalize_memory(ptr %handoff.raw, i64 %handoff.raw.size, i64 %handoff.descriptor.size, ptr %handoff.normalized, i64 %handoff.capacity)
  %handoff.valid = icmp ne i64 %handoff.count, 0
  br i1 %handoff.valid, label %boot.info, label %park
boot.info:
  %memory.ptr.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 24
  %memory.count.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 32
  %memory.address = ptrtoint ptr %handoff.normalized to i64
  store i64 %memory.address, ptr %memory.ptr.ptr, align 8
  store i64 %handoff.count, ptr %memory.count.ptr, align 8
  %boot.flags.ptr = getelementptr i8, ptr @__ncc_uefi_boot_info, i64 16
  %boot.flags = load i64, ptr %boot.flags.ptr, align 8
  %with.memory = or i64 %boot.flags, 1
  store i64 %with.memory, ptr %boot.flags.ptr, align 8
  %kernel.entry = inttoptr i64 __KERNEL_ENTRY__ to ptr
  %kernel.result = call __KERNEL_CC__i64 %kernel.entry(ptr @__ncc_uefi_boot_info, i64 __BOOT_MAGIC__)
  call void @__ncc_uefi_report_result(i64 %kernel.result)
  br label %park
park:
  call void asm sideeffect "__PARK_INSTRUCTION__", "~{memory}"()
  br label %park
return.invalid:
  ret i64 -9223372036854775806
return.load.terminal:
  ret i64 -9223372036854775799
return.map.allocate:
  ret i64 %map.allocate.status
return.normalized.allocate:
  ret i64 %normalized.allocate.status
return.map:
  ret i64 %map.status
return.exit:
  ret i64 %exit.status
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn elf64(machine: u16, entry: u64) -> Vec<u8> {
        let mut bytes = vec![0_u8; 0x1100];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&entry.to_le_bytes());
        bytes[32..40].copy_from_slice(&64_u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
        bytes[54..56].copy_from_slice(&56_u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1_u16.to_le_bytes());
        bytes[64..68].copy_from_slice(&1_u32.to_le_bytes());
        bytes[68..72].copy_from_slice(&5_u32.to_le_bytes());
        bytes[72..80].copy_from_slice(&0x1000_u64.to_le_bytes());
        bytes[80..88].copy_from_slice(&entry.to_le_bytes());
        bytes[88..96].copy_from_slice(&entry.to_le_bytes());
        bytes[96..104].copy_from_slice(&0x100_u64.to_le_bytes());
        bytes[104..112].copy_from_slice(&0x180_u64.to_le_bytes());
        bytes[112..120].copy_from_slice(&0x1000_u64.to_le_bytes());
        bytes[0x1000] = 0xc3;
        bytes
    }

    #[test]
    fn plans_x64_and_aarch64_physical_loads() {
        for (target, machine, entry) in [
            (UefiLoaderTarget::X86_64, 62, 0x40_0000),
            (UefiLoaderTarget::Aarch64, 183, 0x4000_0000),
        ] {
            let plan = UefiLoaderPlan::from_elf(&elf64(machine, entry), target).unwrap();
            assert_eq!(plan.entry, entry);
            assert_eq!(plan.allocations.len(), 1);
            assert_eq!(plan.allocations[0].base, entry);
            assert_eq!(plan.allocations[0].pages, 1);
            assert_eq!(plan.allocations[0].memory_type, 1);
            let ir = plan.render_llvm_ir().unwrap();
            assert!(ir.contains("@efi_main"));
            assert!(ir.contains("%exit.boot.services"));
            assert!(ir.contains("@__ncc_uefi_normalize_memory"));
            assert!(ir.contains("@__ncc_uefi_capture_gop"));
            assert!(ir.contains("@__ncc_uefi_capture_config_tables"));
            assert!(ir.contains("@__ncc_uefi_report_result"));
            assert!(ir.contains("%kernel.result = call"));
            assert!(!ir.contains("__EFI_CC__"));
            assert!(!ir.contains("__LOAD_KERNEL_SEGMENTS__"));
        }
    }

    #[test]
    fn rejects_a_kernel_for_the_wrong_firmware_machine() {
        let error =
            UefiLoaderPlan::from_elf(&elf64(62, 0x40_0000), UefiLoaderTarget::Aarch64).unwrap_err();
        assert!(matches!(
            error,
            UefiLoaderError::UnsupportedMachine {
                expected: 183,
                actual: 62
            }
        ));
    }

    #[test]
    fn llvm_accepts_both_native_uefi_loaders_when_clang_is_available() {
        if Command::new("clang").arg("--version").output().is_err() {
            return;
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ncc-machine-uefi-loader-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let lld = ["lld-link", "lld-link-20", "lld-link-19", "lld-link-18"]
            .into_iter()
            .find(|candidate| Command::new(candidate).arg("--version").output().is_ok());
        for (target, machine, entry) in [
            (UefiLoaderTarget::X86_64, 62, 0x40_0000),
            (UefiLoaderTarget::Aarch64, 183, 0x4000_0000),
        ] {
            let plan = UefiLoaderPlan::from_elf(&elf64(machine, entry), target).unwrap();
            let ll = root.join(format!("{}.ll", target.coff_machine()));
            let object = root.join(format!("{}.obj", target.coff_machine()));
            fs::write(&ll, plan.render_llvm_ir().unwrap()).unwrap();
            let output = Command::new("clang")
                .args([
                    "-target",
                    target.clang_triple(),
                    "-ffreestanding",
                    "-fno-stack-protector",
                    "-O2",
                    "-c",
                    "-x",
                    "ir",
                ])
                .arg(&ll)
                .arg("-o")
                .arg(&object)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "clang rejected {} UEFI loader:\n{}",
                target.coff_machine(),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(fs::metadata(&object).unwrap().len() > 0);
            if let Some(lld) = lld {
                let efi = root.join(target.removable_boot_name());
                let machine = format!("/machine:{}", target.coff_machine());
                let output = Command::new(lld)
                    .args([
                        "/subsystem:efi_application",
                        "/entry:efi_main",
                        "/nodefaultlib",
                        "/dynamicbase",
                        "/nxcompat",
                        &machine,
                    ])
                    .arg(format!("/out:{}", efi.display()))
                    .arg(&object)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "lld-link rejected {} UEFI loader:\n{}",
                    target.coff_machine(),
                    String::from_utf8_lossy(&output.stderr)
                );
                let efi_bytes = fs::read(&efi).unwrap();
                assert!(
                    efi_bytes.windows(6).any(|window| window == b".reloc"),
                    "{} UEFI image must retain base relocations",
                    target.coff_machine()
                );
                let parsed = crate::GuestLoadPlan::parse(&efi_bytes).unwrap();
                assert_eq!(
                    parsed.cpu_mode,
                    match target {
                        UefiLoaderTarget::X86_64 => crate::GuestCpuMode::Long64,
                        UefiLoaderTarget::Aarch64 => crate::GuestCpuMode::Aarch64,
                    }
                );
                let disk =
                    crate::build_uefi_fat12_disk_named(&efi_bytes, target.fat12_short_name())
                        .unwrap();
                assert!(disk
                    .windows(target.fat12_short_name().len())
                    .any(|window| window == target.fat12_short_name()));
            }
        }
        let _ = fs::remove_dir_all(root);
    }
}
