pub use ncc_ast::*;

pub type Function = FunctionDecl;
pub type Statement = Stmt;
type IndexedContextRef = (ContextRef, Vec<Expr>, Option<Vec<String>>);

#[derive(Debug, Clone, Copy)]
pub enum WhileFlavor {
    Normal,
}

#[cfg(test)]
mod material_v2_parser_tests {
    use super::*;

    #[test]
    fn parses_places_bits_operations_and_ref_main() {
        let source = r#"
use <time:time>;

sample {
    const:
        GLOBAL_MAGIC >> u32 <- 0x4E43;

    type:
        Header {
            public:
                kind >> byte;
                value >> int;
            layout:
                repr(C);
                align(8);
        }

    stc:
        magic >> u32 <- GLOBAL_MAGIC;
        counter >> int <- 0;

    fn:
        public:
            run(memory >> addr) -> bool {
                value: int = 0;
                bytes: usize = 4;
                copyValue: int = 0;

                store int [memory] <- 7;
                value <- load int [memory];
                copy bytes bytes [memory + 8] <- [memory];
                fill bytes bytes [memory + 16] <- 0;
                copyValue <- value;
                counter <- value;
                return value == 7;
            }

    ref main ->> run;

    fa:
        return true;
}
"#;
        let program = parse_material_v2(ncc_lexer::lex(source).expect("lex material v2"))
            .expect("parse material v2");
        assert_eq!(program.containers.len(), 1);
        let sections = &program.containers[0].sections;
        assert!(sections.iter().any(|s| matches!(s, Section::ConstEval(_))));
        assert!(sections
            .iter()
            .any(|s| matches!(s, Section::TypeContracts(_))));
        assert_eq!(
            sections
                .iter()
                .filter(|s| matches!(s, Section::Stc(_)))
                .count(),
            1
        );
        assert!(sections.iter().any(|s| matches!(s, Section::Fn(_))));
        let function = sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => functions.first(),
                _ => None,
            })
            .expect("material function");
        assert!(function.public_body.iter().any(|statement| matches!(
            statement,
            Stmt::Projection {
                target: AssignTarget::Context(ContextRef { path, .. }),
                ..
            } if path == &["counter".to_string()]
        )));
        assert!(sections.iter().any(|s| matches!(
            s,
            Section::MainEntry {
                kind: MainEntryKind::RefFn(FnRef { path }),
                ..
            } if path == &["run".to_string()]
        )));
    }

    #[test]
    fn public_material_parser_accepts_the_hybrid_sectioned_container() {
        let source = r#"
hybrid {
    stc:
        value >> int <- 0;
    fn:
        public:
            run() -> bool { return true; }
    ref main ->> run;
    fa:
        return true;
}
"#;
        parse_material_v2(ncc_lexer::lex(source).expect("lex hybrid"))
            .expect("the hybrid sectioned surface is canonical");
    }

    #[test]
    fn stc_accepts_raw_as_a_contextual_declaration_name() {
        let source = r#"
rawBuffer {
    stc:
        raw >> p<byte>;
    fa:
        return true;
}
"#;
        let program = parse_material_v2(ncc_lexer::lex(source).expect("lex raw stc name"))
            .expect("raw followed by >> is a declaration, not a raw section");
        assert!(matches!(
            &program.containers[0].sections[0],
            Section::Stc(declarations)
                if matches!(&declarations[0], StcDecl::Typed { name, .. } if name == "raw")
        ));
    }

    #[test]
    fn canonical_root_role_is_the_basal_material_origin() {
        let source = "basalOrigin :: root { fa: return true; }";
        let program = parse_material_v2(ncc_lexer::lex(source).expect("lex canonical root"))
            .expect("canonical root role must parse");
        let container = program.containers.first().expect("root container");
        assert_eq!(container.name, "basalOrigin");
        assert!(matches!(container.context, Some(Context::Root)));
    }

    #[test]
    fn rejects_removed_colon_equal_initializer() {
        let source = "removed { stc: value >> int := 1; }";
        let error = parse_material_v2(ncc_lexer::lex(source).expect("lex removed initializer"))
            .expect_err(":= must not remain a compiler spelling");
        assert!(format!("{error}").contains("E_ASSIGN_INIT_REMOVED_001"));
    }

    #[test]
    fn public_material_parser_rejects_state_shutdown_and_duplicate_stc() {
        let state = "bad { state { value >> int; } }";
        let error = parse_material_v2(ncc_lexer::lex(state).expect("lex state"))
            .expect_err("state block must be spelled stc");
        assert!(format!("{error}").contains("E_STC_CANONICAL_001"));

        let shutdown = "bad { shutdown :: in() -> bool { return true; } }";
        let error = parse_material_v2(ncc_lexer::lex(shutdown).expect("lex shutdown"))
            .expect_err("shutdown definition must be spelled fa");
        assert!(format!("{error}").contains("E_FA_CANONICAL_001"));

        let duplicate = "bad { stc: first >> int; stc: second >> int; }";
        let error = parse_material_v2(ncc_lexer::lex(duplicate).expect("lex duplicate stc"))
            .expect_err("a container owns one stc section");
        assert!(format!("{error}").contains("E_STC_DUPLICATE_001"));
    }

    #[test]
    fn primitive_keeps_state_as_its_material_section() {
        let source = "material :: primitive { state: bootState; }";
        let program = parse_material_v2(ncc_lexer::lex(source).expect("lex primitive state"))
            .expect("primitive state section must parse");
        assert!(matches!(
            &program.containers[0].sections[0],
            Section::State(items) if items == &["bootState".to_string()]
        ));
    }

    #[test]
    fn project_imports_require_the_canonical_angle_surface() {
        let canonical = "use <project:worker>; app { fa: return true; }";
        parse_material_v2(ncc_lexer::lex(canonical).expect("lex canonical project import"))
            .expect("canonical project import must parse");

        let legacy = "use::project::<worker>; app { fa: return true; }";
        let error = parse_material_v2(ncc_lexer::lex(legacy).expect("lex legacy project import"))
            .expect_err("legacy project facade must be rejected outside nfmt");
        assert!(format!("{error}").contains("E_USE_PROJECT_CANONICAL_001"));
    }

    #[test]
    fn parses_official_short_surface_without_changing_backend_semantics() {
        let source = r#"
use <memory:region>;

shortProgram {
    stc:
        value >> int <- 0;

    to main {
        value <- 7;
        println(value);
    }

    fa:
        return true;
}
"#;
        let program = parse_material_v2(ncc_lexer::lex(source).expect("lex short surface"))
            .expect("official short surface must parse");
        let sections = &program.containers[0].sections;
        assert!(sections
            .iter()
            .any(|section| matches!(section, Section::Stc(_))));
        assert!(sections
            .iter()
            .any(|section| matches!(section, Section::Final(_))));
        assert!(sections.iter().any(|section| matches!(
            section,
            Section::MainEntry {
                kind: MainEntryKind::ToMain,
                ..
            }
        )));
    }

    #[test]
    fn legacy_colon_import_is_reported_as_an_import_error() {
        let source = "use::stoud::io::<output>;\nlegacy { fa: return true; }";
        let error = parse_material_v2(ncc_lexer::lex(source).expect("lex legacy import"))
            .expect_err("legacy import must not be mistaken for a container role");
        assert!(format!("{error}").contains("E_MATERIAL_V2_USE_001"));
    }
}

#[cfg(test)]
mod compact_uninitialized_declaration_tests {
    use super::*;

    #[test]
    fn stc_and_local_accept_compact_uninitialized_storage() {
        let source = r#"
compactStorage {
    stc:
        global >> int;

    local:
        scratch >> ptr<byte>;

    fn:
        run() -> bool {
            public:
                return true;
    }
}
"#;
        let program = parse(ncc_lexer::lex(source).expect("fixture must lex"))
            .expect("compact uninitialized declarations must parse");
        let sections = &program.containers[0].sections;
        assert!(sections.iter().any(|section| matches!(
            section,
            Section::Stc(declarations)
                if matches!(&declarations[0], StcDecl::Typed { init: None, .. })
        )));
        assert!(sections.iter().any(|section| matches!(
            section,
            Section::Local(LocalSection { decls })
                if matches!(&decls[0], LocalDecl::Var { init: None, .. })
        )));
    }
}

#[cfg(test)]
mod compact_static_storage_tests {
    use super::*;

    #[test]
    fn immutable_stc_selects_read_only_storage_without_exposing_rodata() {
        let source = r#"
immutableStorage {
    stc:
        immutable words >> array<byte, 4> <- [78, 67, 67, 0];
}
"#;
        let program = parse(ncc_lexer::lex(source).expect("fixture must lex"))
            .expect("ordinary immutable storage must parse without a lowering import");
        let Section::Stc(decls) = &program.containers[0].sections[0] else {
            panic!("expected stc section");
        };
        let StcDecl::Typed { ty, init, .. } = &decls[0];
        let (storage, value_type) = split_storage_contract(ty).expect("immutable storage metadata");
        assert_eq!(storage.section, StorageSection::ReadOnlyData);
        assert_eq!(
            value_type,
            &TypeName::Generic {
                base: "array".to_string(),
                args: vec![
                    TypeName::Simple("byte".to_string()),
                    TypeName::Simple("4".to_string()),
                ],
            }
        );
        assert!(init.is_some());
    }

    #[test]
    fn explicit_rodata_requires_the_named_lowering_component() {
        let ordinary = r#"
ordinarySource {
    stc:
        rodata bytes >> array<byte, 1> <- [1];
}
"#;
        let error = parse(ncc_lexer::lex(ordinary).expect("fixture must lex"))
            .expect_err("physical rodata must not be an ordinary source qualifier");
        assert!(
            format!("{error}").contains("E_LOWERING_COMPONENT_REQUIRED_001"),
            "{error}"
        );

        let direct = r#"
rodata.ncc;
directLowering {
    stc:
        rodata bytes >> array<byte, 1> <- [1];
}
"#;
        parse(ncc_lexer::lex(direct).expect("fixture must lex"))
            .expect("the named lowering component must enable direct rodata work");
    }

    #[test]
    fn rodata_component_is_lazy_and_injects_its_nc_api_on_demand() {
        let direct_only = r#"
rodata.ncc;
directOnly {
    stc:
        rodata bytes >> array<byte, 1> <- [1];
}
"#;
        let direct_program =
            parse_material_v2(ncc_lexer::lex(direct_only).expect("fixture must lex"))
                .expect("direct rodata source must parse");
        assert_eq!(direct_program.containers.len(), 1);
        assert_eq!(direct_program.containers[0].name, "directOnly");

        let with_api = r#"
rodata.ncc;
singleFileGame {
    contains(word >> str) -> bool {
        return rodataNcc.lexiconContains(word);
    }
}
"#;
        let program = parse_material_v2(ncc_lexer::lex(with_api).expect("fixture must lex"))
            .expect("rodata.ncc must provide its bundled nC API");
        assert!(program
            .containers
            .iter()
            .any(|container| container.name == "singleFileGame"));
        assert!(program
            .containers
            .iter()
            .any(|container| container.name == "nccEmbeddedPack"));
        assert!(program
            .containers
            .iter()
            .any(|container| container.name == "rodataNcc"));
    }

    #[test]
    fn storage_qualifiers_preserve_the_declared_value_type() {
        let source = r#"
rodata.ncc;
staticMemory {
    stc:
        tls align(64) section(".tdata.fast") errno >> int = 0;
        percpu bss ticks >> u64;
        weak data hook >> p<void> = null;
        comdat("shared.config") rodata config >> int = 7;

    local:
        static align(32) cache >> array<byte, 32>;
}
"#;
        let program = parse(ncc_lexer::lex(source).expect("fixture must lex"))
            .expect("qualified storage must parse");
        let Section::Stc(decls) = &program.containers[0].sections[0] else {
            panic!("expected stc section");
        };
        let StcDecl::Typed { ty, .. } = &decls[0];
        let (storage, value_type) = split_storage_contract(ty).expect("tls storage metadata");
        assert_eq!(storage.duration, StorageDuration::ThreadLocal);
        assert_eq!(storage.align_bytes, Some(64));
        assert_eq!(storage.custom_section.as_deref(), Some(".tdata.fast"));
        assert_eq!(value_type, &TypeName::Simple("int".to_string()));

        let StcDecl::Typed { ty, .. } = &decls[3];
        let (storage, _) = split_storage_contract(ty).expect("COMDAT storage metadata");
        assert_eq!(storage.linkage, StorageLinkage::LinkOnceOdr);
        assert_eq!(storage.comdat.as_deref(), Some("shared.config"));
    }

    #[test]
    fn function_static_storage_is_preserved_as_a_typed_projection() {
        let source = r#"
functionStaticMemory {
    fn:
        run() -> int {
            static align(32) calls >> int = 0;
            tls align(64) errno >> int = 0;
            calls += 1;
            return calls;
        }
}
"#;
        let program = parse(ncc_lexer::lex(source).expect("fixture must lex"))
            .expect("function-static storage must parse");
        let Section::Fn(functions) = &program.containers[0].sections[0] else {
            panic!("expected fn section");
        };
        let Stmt::Projection { ty, .. } = &functions[0].public_body[0] else {
            panic!("expected static projection");
        };
        let (storage, value_type) = split_storage_contract(ty).expect("static metadata");
        assert_eq!(storage.duration, StorageDuration::Static);
        assert_eq!(storage.align_bytes, Some(32));
        assert_eq!(value_type, &TypeName::Simple("int".to_string()));

        let Stmt::Projection { ty, .. } = &functions[0].public_body[1] else {
            panic!("expected TLS projection");
        };
        let (storage, _) = split_storage_contract(ty).expect("TLS metadata");
        assert_eq!(storage.duration, StorageDuration::ThreadLocal);
        assert_eq!(storage.align_bytes, Some(64));
    }

    #[test]
    fn data_identifier_index_write_is_not_parsed_as_a_storage_qualifier() {
        let source = r#"
compactDataIndex {
    stc:
        data >> array<int, 4>;

    fn:
        run() -> bool {
            data[0] = 41;
            return data[0] == 41;
        }
}
"#;
        parse(ncc_lexer::lex(source).expect("fixture must lex"))
            .expect("data[index] must remain an indexed assignment");
    }
}

#[cfg(test)]
mod compact_kernel_pointer_surface_tests {
    use super::*;

    fn parse_source(source: &str) -> Result<Program, Diagnostic> {
        parse(ncc_lexer::lex(source).expect("fixture must lex"))
    }

    #[test]
    fn tagless_kernel_at_mmio_and_register_aliases_normalize_to_canonical_ast() {
        let source = r#"
kernelApicSmpGuestBoot {
    configureXapic() -> bool {
        lapic  at 0xFEE00000;
        ioapic at 0xFEC00000;
        hpet   at 0xFED00000;
        source at 0x70000;
        destination at 0x71000;

        guard load32(lapic + 0x30) >= 0 else return false;
        *32(lapic + 0xF0, 0x1FF);
        route = load32(ioapic + 0x10);
        64(hpet + 0xF0, route);
        *128(hpet + 0x100, source);
        256(hpet + 0x120, source);
        store512(hpet + 0x160, source);
        guard load128(hpet + 0x100, destination) else return false;
        guard load256(hpet + 0x120, destination) else return false;
        guard load512(hpet + 0x160, destination) else return false;
        sendIpi(lapic, 1, 0x4042);
        return true;
    }

    configureX2apic() -> bool {
        guard msrRead(0x803) >= 0 else return false;
        msrWrite(0x80F, 0x1FF);
        tscDeadline(tscRead() + 1_000_000);
        return true;
    }

    main: configureXapic;
}
"#;
        let program = parse_source(source).expect("compact kernel source must parse");
        assert!(matches!(
            program.containers[0].context,
            Some(Context::Kernel)
        ));
        let ast = format!("{program:#?}");
        for canonical in [
            "mmio_read32",
            "mmio_read128",
            "mmio_read256",
            "mmio_read512",
            "mmio_write32",
            "mmio_write64",
            "mmio_write128",
            "mmio_write256",
            "mmio_write512",
            "apic_send_ipi",
            "x2apic_read",
            "x2apic_write",
            "tsc_read",
            "tsc_deadline_write",
        ] {
            assert!(
                ast.contains(canonical),
                "missing canonical call {canonical}: {ast}"
            );
        }
        assert!(ast.contains("\"addr\""), "{ast}");
    }

    #[test]
    fn tagless_kernel_name_with_main_infers_kernel_without_mmio_markers() {
        let source = r#"
kernelQemuBootSmoke {
    run() -> bool {
        return true;
    }

    main: run;
}
"#;
        let program = parse_source(source).expect("tagless kernel container must parse");
        assert!(matches!(
            program.containers[0].context,
            Some(Context::Kernel)
        ));
    }

    #[test]
    fn compact_kernel_groups_parameter_types_and_normalizes_debug_aliases() {
        let source = r#"
kernelDebugCompact {
    capture(source, registers, memory: addr; slots, bytes: int) -> bool {
        copied = dbgRegs(source, registers, slots);
        dumped = dbgDump(source, memory, bytes);
        return copied == slots && dumped == bytes;
    }

    idle() -> bool { return true; }
    main: idle;
}
"#;
        let program = parse_source(source).expect("compact debug surface must parse");
        let Section::Fn(functions) = &program.containers[0].sections[0] else {
            panic!("capture must be a function section");
        };
        assert_eq!(
            functions[0]
                .params
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect::<Vec<_>>(),
            ["source", "registers", "memory", "slots", "bytes"]
        );
        let ast = format!("{program:#?}");
        assert!(ast.contains("debug_register_snapshot"));
        assert!(ast.contains("debug_memory_dump"));
        assert!(!ast.contains("dbgRegs"));
        assert!(!ast.contains("dbgDump"));
    }

    #[test]
    fn compact_kernel_markers_inside_comments_or_strings_do_not_infer_kernel() {
        let source = r#"
ordinaryProgram {
    run() -> bool {
        // lapic at 0xFEE00000;
        text >> str = "*32(lapic, 1)";
        return true;
    }

    main: run;
}
"#;
        let program = parse_source(source).expect("ordinary source must parse");
        assert!(!matches!(
            program.containers[0].context,
            Some(Context::Kernel)
        ));
    }

    #[test]
    fn compact_pointer_type_and_calls_keep_one_internal_contract() {
        let source = r#"
compactPointers {
    usePointer(address: p<int>) -> int {
        pWrite<int>(address, 7);
        wr<int>(address, 8);
        addressValue <- adr(address);
        aliasPointer: p<int> = from<int>(addressValue);
        next <- off<int>(aliasPointer, 1);
        distance <- dist(next, aliasPointer);
        bytePointer <- asptr<byte>(aliasPointer);
        if (nil(bytePointer)) return distance;
        return rd<int>(asptr<int>(bytePointer));
    }
}
"#;
        let program = parse_source(source).expect("compact pointer source must parse");
        let ast = format!("{program:#?}");
        assert!(ast.contains("base: \"ptr\""), "{ast}");
        assert!(ast.matches("ptrWrite<int>").count() >= 2, "{ast}");
        assert!(ast.contains("ptrRead<int>"), "{ast}");
        assert!(ast.contains("ptrAddr"), "{ast}");
        assert!(ast.contains("ptrFromAddr<int>"), "{ast}");
        assert!(ast.contains("ptrCast<byte>"), "{ast}");
        assert!(ast.contains("ptrCast<int>"), "{ast}");
        assert!(ast.contains("ptrOffset<int>"), "{ast}");
        assert!(ast.contains("ptrDiff"), "{ast}");
        assert!(ast.contains("ptrIsNull"), "{ast}");
        assert!(!ast.contains("pWrite<int>"), "{ast}");
        assert!(!ast.contains("pFromAddr<int>"), "{ast}");
    }

    #[test]
    fn compact_raw_memory_calls_normalize_before_sema() {
        let source = r#"
rawCalls {
    run(dst: p<byte>, left: str, right: str) -> int {
        written <- cat(dst, 64, left, right);
        mapped <- map(null, 4096, 3, 0, -1, 0);
        protect(mapped, 4096, 5);
        exec(mapped, 4096);
        cp(adr(dst), adr(dst), 0);
        mv(adr(dst), adr(dst), 0);
        fill(adr(dst), 0, 0);
        cmp(adr(dst), adr(dst), 0);
        unmap(mapped, 4096);
        return written;
    }
}
"#;
        let program = parse_source(source).expect("compact raw calls must parse");
        let ast = format!("{program:#?}");
        for canonical in [
            "strConcatInto",
            "vmMap",
            "vmProtect",
            "vmExecutable",
            "vmUnmap",
            "memcpy",
            "memmove",
            "memset",
            "memcmp",
        ] {
            assert!(ast.contains(canonical), "missing {canonical}: {ast}");
        }
    }

    #[test]
    fn raw_api_v1_normalizes_mmio_and_dma_before_kernel_sema() {
        let source = r#"
kernelRawCatalog {
    run(base, out: addr) -> bool {
        value <- mrd32(base);
        mwr32(base, value);
        mrd128(base, out);
        dClean(base, 64, 0);
        dFence();
        return true;
    }
    main: run;
}
"#;
        let program = parse_source(source).expect("raw MMIO/DMA aliases must parse");
        let ast = format!("{program:#?}");
        for canonical in [
            "mmio_read32",
            "mmio_write32",
            "mmio_read128",
            "dma_cache_clean",
            "dma_fence",
        ] {
            assert!(ast.contains(canonical), "missing {canonical}: {ast}");
        }
    }

    #[test]
    fn legacy_print_members_are_rejected_at_the_source_boundary() {
        let source = r#"
onlyPrint {
    run() -> bool {
        print.err("legacy");
        return true;
    }
}
"#;
        let error = parse_source(source).expect_err("print.* must not remain source syntax");
        assert!(
            format!("{error}").contains("E_PRINT_SURFACE_001"),
            "{error:?}"
        );
    }
}

use ncc_diagnostics::Diagnostic;
use ncc_lexer::{Token, TokenKind};

const RODATA_NCC_SOURCE: &str = include_str!("components/rodata.ncc");

pub fn parse(tokens: Vec<Token>) -> Result<Program, Diagnostic> {
    Parser::new(tokens).parse_program()
}

/// Parses the only public source surface: nC Material V2.
///
/// `parse` remains available to the source migrator so an old file can be
/// read and rewritten, but compiler entry points must call this function.
pub fn parse_material_v2(tokens: Vec<Token>) -> Result<Program, Diagnostic> {
    validate_material_v2_surface(&tokens)?;
    Parser::new(tokens).parse_program()
}

pub fn validate_material_v2_surface(tokens: &[Token]) -> Result<(), Diagnostic> {
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < tokens.len() {
        let token = &tokens[index];
        if depth == 0
            && matches!(&token.kind, TokenKind::Identifier(name) if name == "package" || name == "module")
        {
            let mut manifest_depth = 0usize;
            while index < tokens.len() {
                match tokens[index].kind {
                    TokenKind::LBrace => manifest_depth += 1,
                    TokenKind::RBrace => {
                        manifest_depth = manifest_depth.saturating_sub(1);
                        if manifest_depth == 0 {
                            index += 1;
                            break;
                        }
                    }
                    TokenKind::Semicolon if manifest_depth == 0 => {
                        index += 1;
                        break;
                    }
                    TokenKind::Eof => break,
                    _ => {}
                }
                index += 1;
            }
            continue;
        }
        if depth == 0
            && matches!(
                (
                    tokens.get(index).map(|token| &token.kind),
                    tokens.get(index + 1).map(|token| &token.kind),
                    tokens.get(index + 2).map(|token| &token.kind),
                    tokens.get(index + 3).map(|token| &token.kind),
                ),
                (
                    Some(TokenKind::Identifier(component)),
                    Some(TokenKind::Dot),
                    Some(TokenKind::Identifier(extension)),
                    Some(TokenKind::Semicolon),
                ) if component.eq_ignore_ascii_case("rodata")
                    && extension.eq_ignore_ascii_case("ncc")
            )
        {
            index += 4;
            continue;
        }
        if depth == 0
            && matches!(&token.kind, TokenKind::Identifier(_))
            && matches!(
                tokens.get(index + 1).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
            && matches!(
                tokens.get(index + 2).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
        {
            let role = match tokens.get(index + 3).map(|token| &token.kind) {
                Some(TokenKind::Identifier(role)) => role.to_ascii_lowercase(),
                _ => String::new(),
            };
            if matches!(role.as_str(), "const") {
                while index < tokens.len()
                    && !matches!(tokens[index].kind, TokenKind::Semicolon | TokenKind::Eof)
                {
                    index += 1;
                }
                index += 1;
                continue;
            }
            if matches!(role.as_str(), "type" | "struct" | "union" | "enum") {
                let mut cursor = index + 4;
                while cursor < tokens.len()
                    && !matches!(tokens[cursor].kind, TokenKind::LBrace | TokenKind::Eof)
                {
                    cursor += 1;
                }
                if !matches!(
                    tokens.get(cursor).map(|token| &token.kind),
                    Some(TokenKind::LBrace)
                ) {
                    return Err(Diagnostic::stage0_registered(
                        ncc_diagnostics::codes::E_MATERIAL_V2_TYPE_001,
                        "a material type definition requires layout or body",
                        token.line,
                        token.column,
                    ));
                }
                depth = 1;
                index = cursor + 1;
                continue;
            }
            if !matches!(
                role.as_str(),
                "program"
                    | "runtime"
                    | "network"
                    | "system"
                    | "kernel"
                    | "tooling"
                    | "compiler"
                    | "security"
                    | "inspect"
                    | "ai"
                    | "machine"
                    | "root"
                    | "primitive"
                    | "basal"
            ) {
                return Err(Diagnostic::stage0_registered(
                    ncc_diagnostics::codes::E_MATERIAL_V2_CONTAINER_001,
                    "a top-level definition requires a Material V2 role",
                    token.line,
                    token.column,
                ));
            }
            if !matches!(
                tokens.get(index + 4).map(|token| &token.kind),
                Some(TokenKind::LBrace)
            ) {
                return Err(Diagnostic::stage0_registered(
                    ncc_diagnostics::codes::E_MATERIAL_V2_CONTAINER_002,
                    "the container role must be followed by `{`",
                    token.line,
                    token.column,
                ));
            }
            depth = 1;
            index += 5;
            continue;
        }
        match &token.kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => depth = depth.saturating_sub(1),
            TokenKind::KeywordUse if depth == 0 => {
                let legacy_project_facade = matches!(
                    (
                        tokens.get(index + 1).map(|token| &token.kind),
                        tokens.get(index + 2).map(|token| &token.kind),
                        tokens.get(index + 3).map(|token| &token.kind),
                    ),
                    (
                        Some(TokenKind::Colon),
                        Some(TokenKind::Colon),
                        Some(TokenKind::Identifier(domain)),
                    ) if domain.eq_ignore_ascii_case("project")
                );
                if legacy_project_facade {
                    return Err(Diagnostic::new(
                    "E_USE_PROJECT_CANONICAL_001: project imports use `use <project:module>;`; `use::project` is not part of the language",
                        token.line,
                        token.column,
                    ));
                }
                let mut cursor = index + 1;
                let compact_selection = matches!(
                    tokens.get(cursor).map(|token| &token.kind),
                    Some(TokenKind::Less)
                );
                let mut has_item_block = false;
                while cursor < tokens.len()
                    && !matches!(tokens[cursor].kind, TokenKind::Semicolon | TokenKind::Eof)
                {
                    if matches!(tokens[cursor].kind, TokenKind::LBrace) {
                        has_item_block = true;
                    }
                    if !compact_selection
                        && matches!(tokens[cursor].kind, TokenKind::Colon | TokenKind::At)
                    {
                        return Err(Diagnostic::stage0_registered(
                            ncc_diagnostics::codes::E_MATERIAL_V2_USE_001,
                            "imports use `use domain.module { item, ... };`",
                            tokens[cursor].line,
                            tokens[cursor].column,
                        ));
                    }
                    cursor += 1;
                }
                if !compact_selection && !has_item_block {
                    return Err(Diagnostic::stage0_registered(
                        ncc_diagnostics::codes::E_MATERIAL_V2_USE_001,
                        "imports require an explicit item block, for example `use stoud.io { output };`",
                        token.line,
                        token.column,
                    ));
                }
                index = cursor;
            }
            TokenKind::Identifier(_) if depth == 0 => {
                if matches!(
                    tokens.get(index + 1).map(|token| &token.kind),
                    Some(TokenKind::LBrace)
                ) {
                    // The canonical hosted header is deliberately short: `name {`.
                    // An explicit `:: role` remains available only when the role is
                    // material (kernel, machine, runtime, and the other domains).
                    depth = 1;
                    index += 2;
                    continue;
                }
                return Err(Diagnostic::new(
                    "E_MATERIAL_CONTAINER_001: expected a short container header `name {` or an intentional `name :: role {` header",
                    token.line,
                    token.column,
                ));
            }
            TokenKind::KeywordState
                if depth == 1
                    && matches!(
                        tokens.get(index + 1).map(|token| &token.kind),
                        Some(TokenKind::LBrace)
                    ) =>
            {
                return Err(Diagnostic::new(
                    "E_STC_CANONICAL_001: high-level container state uses one `stc:` section; `state` is reserved for primitive material and basal `.nb` metadata",
                    token.line,
                    token.column,
                ));
            }
            TokenKind::Identifier(name)
                if depth == 1
                    && name.eq_ignore_ascii_case("shutdown")
                    && matches!(
                        tokens.get(index + 1).map(|token| &token.kind),
                        Some(TokenKind::Colon)
                    )
                    && matches!(
                        tokens.get(index + 2).map(|token| &token.kind),
                        Some(TokenKind::Colon)
                    ) =>
            {
                return Err(Diagnostic::new(
                    "E_FA_CANONICAL_001: material finalization uses `fa:`; `shutdown :: ...` is not canonical source",
                    token.line,
                    token.column,
                ));
            }
            _ => {}
        }
        index += 1;
    }
    Ok(())
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    active_context: Option<Context>,
    explicit_rodata_lowering: bool,
}

enum VariantPatternItem {
    Binding(String),
    Pattern(Pattern),
}

impl Parser {
    fn inferred_projection_type() -> TypeName {
        TypeName::Simple("__nC_infer_projection_type".to_string())
    }

    fn type_surface(ty: &TypeName) -> String {
        match ty {
            TypeName::Simple(name) => name.clone(),
            TypeName::Vector(inner) => format!("vector<{}>", Self::type_surface(inner)),
            TypeName::Array(inner) => format!("array<{}>", Self::type_surface(inner)),
            TypeName::Union(items) => items
                .iter()
                .map(Self::type_surface)
                .collect::<Vec<_>>()
                .join("|"),
            TypeName::Generic { base, args } => format!(
                "{}<{}>",
                base,
                args.iter()
                    .map(Self::type_surface)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }

    fn expect_material_access_type(&mut self) -> Result<TypeName, Diagnostic> {
        let mut name = self.expect_type_ident()?;
        if name.eq_ignore_ascii_case("p") {
            name = "ptr".to_string();
        }
        while self.match_kind(&TokenKind::Dot) {
            name.push('.');
            name.push_str(&self.expect_type_ident()?);
        }
        if self.match_lt() {
            let mut args = Vec::new();
            loop {
                args.push(self.expect_type_generic_arg()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_gt()?;
            Ok(TypeName::Generic { base: name, args })
        } else {
            Ok(TypeName::Simple(name))
        }
    }

    fn expect_projection_init_separator(&mut self) -> Result<(), Diagnostic> {
        if self.match_kind(&TokenKind::LessDash) {
            return Ok(());
        }
        if self.match_kind(&TokenKind::Colon) {
            if self.match_kind(&TokenKind::Eq) {
                return self.error_here(
                    "E_ASSIGN_INIT_REMOVED_001: ':=' was removed; use '<-' for initialization and writes",
                );
            }
            return Ok(());
        }
        if self.match_kind(&TokenKind::Eq) {
            return Ok(());
        }
        self.error_here("Expected '<-' before projection initializer")
    }

    fn check_visibility_block_start(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::KeywordPublic
                | TokenKind::KeywordPrivate
                | TokenKind::KeywordInternal
                | TokenKind::KeywordStrictly
        ) && matches!(
            self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
            Some(TokenKind::Colon)
        )
    }

    fn parse_function_group(&mut self) -> Result<Vec<FunctionDecl>, Diagnostic> {
        let mut funcs = Vec::new();

        while self.peek_starts_function() {
            if self.check_visibility_block_start() {
                let visibility = self
                    .parse_visibility_kw_opt()
                    .expect("visibility block start checked");
                self.expect_kind(TokenKind::Colon)?;
                while self.peek_starts_function() && !self.check_visibility_block_start() {
                    let mut func = self.parse_function()?;
                    if func.visibility.is_none() {
                        func.visibility = Some(visibility.clone());
                    }
                    funcs.push(func);
                }
                continue;
            }

            funcs.push(self.parse_function()?);
        }

        Ok(funcs)
    }

    fn match_lt(&mut self) -> bool {
        self.match_kind(&TokenKind::Less)
    }

    fn expect_gt(&mut self) -> Result<(), Diagnostic> {
        if self.match_kind(&TokenKind::Greater) {
            return Ok(());
        }

        if self.check(&TokenKind::Shift3) {
            let tok = self.current().clone();
            self.tokens[self.pos].kind = TokenKind::Greater;
            self.tokens.insert(
                self.pos + 1,
                Token {
                    kind: TokenKind::Shift2,
                    line: tok.line,
                    column: tok.column + 1,
                },
            );
            self.advance();
            return Ok(());
        }

        if self.check(&TokenKind::Shift2) {
            let tok = self.current().clone();
            self.tokens[self.pos].kind = TokenKind::Greater;
            self.tokens.insert(
                self.pos + 1,
                Token {
                    kind: TokenKind::Greater,
                    line: tok.line,
                    column: tok.column + 1,
                },
            );
            self.advance();
            return Ok(());
        }

        self.error_here("Expected Greater")
    }

    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            active_context: None,
            explicit_rodata_lowering: false,
        }
    }

    fn note_lowering_components(&mut self, uses: &[UseDecl]) {
        self.explicit_rodata_lowering |= uses.iter().any(|decl| {
            matches!(
                decl.path.as_slice(),
                [rodata, ncc]
                    if rodata.eq_ignore_ascii_case("rodata")
                        && ncc.eq_ignore_ascii_case("ncc")
            )
        });
    }

    fn peek_lowering_component_decl(&self) -> bool {
        matches!(
            (
                self.tokens.get(self.pos).map(|token| &token.kind),
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                self.tokens.get(self.pos + 2).map(|token| &token.kind),
                self.tokens.get(self.pos + 3).map(|token| &token.kind),
            ),
            (
                Some(TokenKind::Identifier(component)),
                Some(TokenKind::Dot),
                Some(TokenKind::Identifier(extension)),
                Some(TokenKind::Semicolon),
            ) if component.eq_ignore_ascii_case("rodata")
                && extension.eq_ignore_ascii_case("ncc")
        )
    }

    fn parse_lowering_component_decl(&mut self) -> Result<UseDecl, Diagnostic> {
        let component = self.expect_ident()?;
        self.expect_kind(TokenKind::Dot)?;
        let extension = self.expect_ident()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(UseDecl {
            path: vec![component, extension],
            wildcard: false,
            alias: None,
        })
    }

    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut modules = Vec::new();
        let mut packages = Vec::new();
        let mut uses = Vec::new();
        let mut containers = Vec::new();
        let mut global_consts = Vec::new();
        let mut global_type_contracts = Vec::new();

        while !self.at_eof() {
            if self.peek_lowering_component_decl() {
                let decl = self.parse_lowering_component_decl()?;
                self.note_lowering_components(std::slice::from_ref(&decl));
                uses.push(decl);
                continue;
            }

            if self.check_ident_name("module") && !self.check_next_kind(&TokenKind::LBrace) {
                modules.push(self.parse_module_decl_after_keyword()?);
                continue;
            }

            if self.check_ident_name("package") && !self.check_next_kind(&TokenKind::LBrace) {
                packages.push(self.parse_package_decl_after_keyword()?);
                continue;
            }

            if self.match_kind(&TokenKind::KeywordUse) || self.match_kind(&TokenKind::KeywordImport)
            {
                let parsed = self.parse_use_decls_after_keyword()?;
                self.note_lowering_components(&parsed);
                uses.extend(parsed);
                continue;
            }

            if self.match_ident_name("const") {
                global_consts.push(self.parse_global_const_decl_after_keyword()?);
                continue;
            }

            if self.peek_v2_global_const_decl() {
                global_consts.push(self.parse_v2_global_const_decl()?);
                continue;
            }

            if self.peek_starts_canonical_type_decl() {
                global_type_contracts.push(self.parse_canonical_type_contract_decl()?);
                continue;
            }

            if self.match_kind(&TokenKind::KeywordStructure) || self.match_ident_name("struct") {
                let contract = self.parse_type_contract_decl()?;
                if contract.kind != TypeContractKind::Struct {
                    return self.error_here(
                        "E_PARSE_GLOBAL_STRUCT_001: global struct syntax must declare a struct contract",
                    );
                }
                global_type_contracts.push(contract);
                continue;
            }

            containers.push(self.parse_container()?);
        }

        let references_rodata_api = self.tokens.iter().any(|token| {
            matches!(
                &token.kind,
                TokenKind::Identifier(name) if name.eq_ignore_ascii_case("rodataNcc")
            )
        });
        if self.explicit_rodata_lowering && references_rodata_api {
            let component_tokens =
                ncc_lexer::lex(RODATA_NCC_SOURCE).map_err(|error| Diagnostic {
                    message: format!(
                        "E_RODATA_COMPONENT_LEX_001: bundled rodata.ncc failed to lex: {error}"
                    ),
                    line: 0,
                    column: 0,
                })?;
            let component = Parser::new(component_tokens)
                .parse_program()
                .map_err(|error| Diagnostic {
                    message: format!(
                        "E_RODATA_COMPONENT_PARSE_001: bundled rodata.ncc failed to parse: {error}"
                    ),
                    line: 0,
                    column: 0,
                })?;

            for bundled in component.containers {
                if containers
                    .iter()
                    .any(|container| container.name.eq_ignore_ascii_case(&bundled.name))
                {
                    return Err(Diagnostic {
                        message: format!(
                            "E_RODATA_COMPONENT_COLLISION_001: source container '{}' conflicts with bundled rodata.ncc",
                            bundled.name
                        ),
                        line: 0,
                        column: 0,
                    });
                }
                containers.push(bundled);
            }
        }

        let imports_mobile_telephony = uses.iter().any(|use_decl| {
            use_decl
                .path
                .iter()
                .map(|segment| segment.to_ascii_lowercase())
                .collect::<Vec<_>>()
                == ["nc", "runtime", "telephony"]
        });
        let references_mobile_bloqueo = self.tokens.iter().any(|token| {
            matches!(
                &token.kind,
                TokenKind::Identifier(name) if name.eq_ignore_ascii_case("Bloqueo")
            )
        });
        let declares_bloqueo = global_type_contracts
            .iter()
            .any(|contract| contract.name.eq_ignore_ascii_case("Bloqueo"))
            || containers.iter().any(|container| {
                container.sections.iter().any(|section| {
                    matches!(
                        section,
                        Section::TypeContracts(contracts)
                            if contracts.iter().any(|contract| contract.name.eq_ignore_ascii_case("Bloqueo"))
                    )
                })
            });
        if (imports_mobile_telephony || references_mobile_bloqueo) && !declares_bloqueo {
            global_type_contracts.push(TypeContractDecl {
                name: "Bloqueo".into(),
                kind: TypeContractKind::Struct,
                generic_params: Vec::new(),
                variants: Vec::new(),
                clauses: vec![TypeContractClause::Fields {
                    visibility: Visibility::Public,
                    fields: ["numero", "patron", "origen"]
                        .into_iter()
                        .map(|name| TypeContractField {
                            name: name.into(),
                            ty: TypeName::Simple("str".into()),
                        })
                        .collect(),
                }],
            });
        }

        if !global_consts.is_empty() {
            for container in &mut containers {
                container
                    .sections
                    .insert(0, Section::ConstEval(global_consts.clone()));
            }
        }

        if !global_type_contracts.is_empty() {
            for container in &mut containers {
                container
                    .sections
                    .insert(0, Section::TypeContracts(global_type_contracts.clone()));
            }
        }

        Ok(Program {
            modules,
            packages,
            uses,
            containers,
        })
    }

    fn parse_global_const_decl_after_keyword(&mut self) -> Result<ConstEvalDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let (ty, value) = if self.match_kind(&TokenKind::Colon) {
            let ty = self.expect_type_name()?;
            if !(self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)) {
                return self
                    .error_here("E_MATERIAL_V2_CONST_001: constant declaration expects '<-'");
            }
            (ty, self.parse_expr()?)
        } else if self.match_kind(&TokenKind::Shift2) {
            let ty = self.expect_type_name()?;
            self.expect_projection_init_separator()?;
            (ty, self.parse_expr()?)
        } else {
            if !(self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)) {
                return self.error_here(
                    "E_MATERIAL_V2_CONST_001: inferred constant declaration expects '<-'",
                );
            }
            let value = self.parse_expr()?;
            let ty = match &value {
                Expr::String(_) => TypeName::Simple("str".into()),
                Expr::Char(_) => TypeName::Simple("char".into()),
                Expr::Bool(_) => TypeName::Simple("bool".into()),
                Expr::Number(number)
                    if number.contains('.') || number.contains('e') || number.contains('E') =>
                {
                    TypeName::Simple("ft64".into())
                }
                Expr::Number(_) => TypeName::Simple("int".into()),
                Expr::Unary {
                    op: UnaryOp::Plus | UnaryOp::Minus,
                    expr,
                } if matches!(&**expr, Expr::Number(_)) => TypeName::Simple("int".into()),
                _ => {
                    return self.error_here(
                        "E_PARSE_GLOBAL_CONST_TYPE_001: inferred global const requires a scalar literal; add an explicit type",
                    )
                }
            };
            (ty, value)
        };
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(ConstEvalDecl {
            visibility: None,
            name,
            ty,
            value,
        })
    }

    fn peek_v2_global_const_decl(&self) -> bool {
        matches!(self.current().kind, TokenKind::Identifier(_))
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
            && matches!(
                self.tokens.get(self.pos + 3).map(|token| &token.kind),
                Some(TokenKind::Identifier(kind)) if kind.eq_ignore_ascii_case("const")
            )
    }

    fn parse_v2_global_const_decl(&mut self) -> Result<ConstEvalDecl, Diagnostic> {
        let name = self.expect_ident()?;
        self.expect_double_colon()?;
        if !self.match_ident_name("const") {
            return self.error_here("E_MATERIAL_V2_CONST_001: expected `const` after `::`");
        }
        let explicit_type = if self.check(&TokenKind::LessDash) {
            None
        } else {
            Some(self.expect_type_name()?)
        };
        self.expect_kind(TokenKind::LessDash)?;
        let value = self.parse_expr()?;
        self.expect_kind(TokenKind::Semicolon)?;
        let ty = explicit_type.unwrap_or_else(|| match &value {
            Expr::String(_) => TypeName::Simple("str".into()),
            Expr::Char(_) => TypeName::Simple("char".into()),
            Expr::Bool(_) => TypeName::Simple("bool".into()),
            Expr::Number(number)
                if number.contains('.') || number.contains('e') || number.contains('E') =>
            {
                TypeName::Simple("ft64".into())
            }
            Expr::Number(_) => TypeName::Simple("int".into()),
            _ => Self::inferred_projection_type(),
        });
        Ok(ConstEvalDecl {
            visibility: None,
            name,
            ty,
            value,
        })
    }

    fn parse_use_decls_after_keyword(&mut self) -> Result<Vec<UseDecl>, Diagnostic> {
        if self.check(&TokenKind::Less) {
            return self.parse_canonical_use_decls_after_keyword();
        }
        if self.check_double_colon() {
            return self.parse_domain_use_decls_after_keyword();
        }

        if self.v2_use_has_item_block() {
            return self.parse_v2_use_decls_after_keyword();
        }

        Ok(vec![self.parse_legacy_use_decl_after_keyword()?])
    }

    fn v2_use_has_item_block(&self) -> bool {
        let mut cursor = self.pos;
        while matches!(
            self.tokens.get(cursor).map(|token| &token.kind),
            Some(TokenKind::Identifier(_))
        ) {
            cursor += 1;
            if matches!(
                self.tokens.get(cursor).map(|token| &token.kind),
                Some(TokenKind::Dot)
            ) {
                cursor += 1;
                continue;
            }
            break;
        }
        matches!(
            self.tokens.get(cursor).map(|token| &token.kind),
            Some(TokenKind::LBrace)
        )
    }

    fn parse_v2_use_decls_after_keyword(&mut self) -> Result<Vec<UseDecl>, Diagnostic> {
        let mut module_path = vec![self.expect_use_path_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            module_path.push(self.expect_use_path_segment()?);
        }
        self.expect_kind(TokenKind::LBrace)?;
        let mut items = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.match_kind(&TokenKind::Star) {
                items.push("*".to_string());
            } else {
                items.push(self.expect_use_path_segment()?);
            }
            if self.match_kind(&TokenKind::Comma) {
                if self.check(&TokenKind::RBrace) {
                    break;
                }
                continue;
            }
            break;
        }
        self.expect_kind(TokenKind::RBrace)?;
        self.expect_kind(TokenKind::Semicolon)?;

        if module_path.len() == 1 && module_path[0].eq_ignore_ascii_case("project") {
            return Ok(items
                .into_iter()
                .map(|item| UseDecl {
                    path: vec!["project".into(), item],
                    wildcard: false,
                    alias: None,
                })
                .collect());
        }

        if module_path.len() > 2 {
            return Ok(items
                .into_iter()
                .map(|item| {
                    let wildcard = item == "*";
                    UseDecl {
                        path: if wildcard {
                            module_path.clone()
                        } else {
                            module_path
                                .iter()
                                .cloned()
                                .chain(std::iter::once(item))
                                .collect()
                        },
                        wildcard,
                        alias: None,
                    }
                })
                .collect());
        }

        let (domain, raw_family) = match module_path.as_slice() {
            [family] => ("stoud", family.as_str()),
            [domain, family] => (domain.as_str(), family.as_str()),
            _ => {
                return self.error_here(
                    "E_USE_V2_MODULE_001: use path must be 'family { items }' or 'domain.family { items }'",
                )
            }
        };
        let normalized_family = raw_family.to_ascii_lowercase();
        let family = match normalized_family.as_str() {
            "network" if domain.eq_ignore_ascii_case("stoud") => "net",
            "filesystem" if domain.eq_ignore_ascii_case("stoud") => "fs",
            other => other,
        };
        let expanded = match ncc_ast::expand_use_domain_import(domain, family, &items) {
            Ok(expanded) => expanded,
            Err(diag) => {
                let explicit_standard_domain = module_path.len() == 2
                    && matches!(
                        module_path[0].to_ascii_lowercase().as_str(),
                        "core" | "alloc" | "stoud" | "auth" | "dom"
                    );
                if explicit_standard_domain {
                    return Err(Diagnostic::new(
                        format!("{}: {}", diag.code, diag.message),
                        self.current().line,
                        self.current().column,
                    ));
                }
                return Ok(items
                    .into_iter()
                    .map(|item| {
                        let wildcard = item == "*";
                        UseDecl {
                            path: if wildcard {
                                module_path.clone()
                            } else {
                                module_path
                                    .iter()
                                    .cloned()
                                    .chain(std::iter::once(item))
                                    .collect()
                            },
                            wildcard,
                            alias: None,
                        }
                    })
                    .collect());
            }
        };
        Ok(expanded
            .into_iter()
            .flat_map(|entry| {
                entry.expands_to.into_iter().map(|path| UseDecl {
                    path: path.split('.').map(str::to_string).collect(),
                    wildcard: false,
                    alias: None,
                })
            })
            .collect())
    }

    fn parse_canonical_use_decls_after_keyword(&mut self) -> Result<Vec<UseDecl>, Diagnostic> {
        self.expect_kind(TokenKind::Less)?;
        let mut decls = Vec::new();

        loop {
            let mut module_path = vec![self.expect_use_path_segment()?];
            while self.match_kind(&TokenKind::Dot) {
                module_path.push(self.expect_use_path_segment()?);
            }
            self.expect_kind(TokenKind::Colon)?;
            let mut items = Vec::new();
            loop {
                if self.match_kind(&TokenKind::Star) {
                    items.push("*".to_string());
                } else {
                    items.push(self.expect_use_path_segment()?);
                }
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }

            if module_path.len() == 1 && module_path[0].eq_ignore_ascii_case("project") {
                decls.extend(items.into_iter().map(|item| UseDecl {
                    path: vec!["project".into(), item],
                    wildcard: false,
                    alias: None,
                }));
            } else {
                let (domain, raw_family) = match module_path.as_slice() {
                    [family] => ("stoud", family.as_str()),
                    [domain, family] => (domain.as_str(), family.as_str()),
                    _ => return self.error_here(
                        "canonical use module must be '<family:items>' or '<domain.family:items>'",
                    ),
                };
                let family = match raw_family.to_ascii_lowercase().as_str() {
                    "network" if domain.eq_ignore_ascii_case("stoud") => "net".to_string(),
                    "filesystem" if domain.eq_ignore_ascii_case("stoud") => "fs".to_string(),
                    other => other.to_string(),
                };
                match ncc_ast::expand_use_domain_import(domain, &family, &items) {
                    Ok(expanded) => decls.extend(expanded.into_iter().flat_map(|entry| {
                        entry.expands_to.into_iter().map(|path| UseDecl {
                            path: path.split('.').map(|part| part.to_string()).collect(),
                            wildcard: false,
                            alias: None,
                        })
                    })),
                    Err(diag)
                        if matches!(
                            domain.to_ascii_lowercase().as_str(),
                            "core" | "alloc" | "stoud" | "auth" | "dom"
                        ) =>
                    {
                        return Err(Diagnostic::new(
                            format!("{}: {}", diag.code, diag.message),
                            self.current().line,
                            self.current().column,
                        ));
                    }
                    Err(_) => decls.extend(items.into_iter().map(|item| {
                        let wildcard = item == "*";
                        UseDecl {
                            path: if wildcard {
                                module_path.clone()
                            } else {
                                module_path
                                    .iter()
                                    .cloned()
                                    .chain(std::iter::once(item))
                                    .collect()
                            },
                            wildcard,
                            alias: None,
                        }
                    })),
                }
            }

            if self.match_kind(&TokenKind::Pipe) {
                continue;
            }
            break;
        }

        self.expect_gt()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(decls)
    }

    fn parse_legacy_use_decl_after_keyword(&mut self) -> Result<UseDecl, Diagnostic> {
        let mut path = vec![self.expect_use_path_segment()?];
        let mut wildcard = false;

        while self.match_kind(&TokenKind::Dot) {
            if self.match_kind(&TokenKind::Star) {
                wildcard = true;
                break;
            }
            path.push(self.expect_use_path_segment()?);
        }

        let alias = if self.match_ident_name("as") {
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect_kind(TokenKind::Semicolon)?;
        Ok(UseDecl {
            path,
            wildcard,
            alias,
        })
    }

    fn parse_domain_use_decls_after_keyword(&mut self) -> Result<Vec<UseDecl>, Diagnostic> {
        self.expect_double_colon()?;
        let domain = self.expect_use_path_segment()?;
        self.expect_double_colon()?;

        if domain.eq_ignore_ascii_case("project") {
            return self.error_here(
                "E_USE_PROJECT_CANONICAL_001: project imports use `use <project:module>;`; `use::project` is not part of the language",
            );
        }

        let family = self.expect_use_path_segment()?;
        self.expect_double_colon()?;

        let mut items = Vec::new();
        if self.match_kind(&TokenKind::Less) {
            if self.match_kind(&TokenKind::Star) {
                return self.error_here(
                    "E_USE_WILDCARD_DENIED_001: wildcard imports are denied for use::core/alloc/stoud/auth/dom",
                );
            }
            loop {
                items.push(self.expect_use_path_segment()?);
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
            self.expect_gt()?;
        }
        self.expect_kind(TokenKind::Semicolon)?;

        let expanded =
            ncc_ast::expand_use_domain_import(&domain, &family, &items).map_err(|diag| {
                Diagnostic::new(
                    format!("{}: {}", diag.code, diag.message),
                    self.current().line,
                    self.current().column,
                )
            })?;

        Ok(expanded
            .into_iter()
            .flat_map(|entry| {
                entry.expands_to.into_iter().map(|path| UseDecl {
                    path: path.split('.').map(|part| part.to_string()).collect(),
                    wildcard: false,
                    alias: None,
                })
            })
            .collect())
    }

    fn parse_module_decl_after_keyword(&mut self) -> Result<ModuleDecl, Diagnostic> {
        self.expect_ident()?;
        let path = self.parse_high_level_path()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(ModuleDecl {
            path,
            package: None,
        })
    }

    fn parse_package_decl_after_keyword(&mut self) -> Result<PackageDecl, Diagnostic> {
        self.expect_ident()?;
        let name = self.expect_ident()?;
        let mut version = None;
        let mut profile = None;
        let mut modules = Vec::new();

        if self.match_kind(&TokenKind::LBrace) {
            while !self.check(&TokenKind::RBrace) {
                if self.match_ident_name("version") {
                    self.expect_kind(TokenKind::Colon)?;
                    version = Some(self.match_string().ok_or_else(|| {
                        Diagnostic::new(
                            "E_HIGH_PACKAGE_PARSE_001: expected package version string",
                            self.current().line,
                            self.current().column,
                        )
                    })?);
                    self.expect_kind(TokenKind::Semicolon)?;
                    continue;
                }
                if self.match_ident_name("profile") {
                    self.expect_kind(TokenKind::Colon)?;
                    let requested = self.expect_use_path_segment()?;
                    profile = Some(requested);
                    self.expect_kind(TokenKind::Semicolon)?;
                    continue;
                }
                if self.match_ident_name("module") {
                    modules.push(self.parse_high_level_path()?);
                    self.expect_kind(TokenKind::Semicolon)?;
                    continue;
                }
                return self.error_here("E_HIGH_PACKAGE_PARSE_001: expected package field");
            }
            self.expect_kind(TokenKind::RBrace)?;
        } else {
            self.expect_kind(TokenKind::Semicolon)?;
        }

        Ok(PackageDecl {
            name,
            version,
            profile,
            modules,
        })
    }

    fn parse_high_level_path(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut path = vec![self.expect_use_path_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            path.push(self.expect_use_path_segment()?);
        }
        Ok(path)
    }

    fn parse_container(&mut self) -> Result<Container, Diagnostic> {
        // Compatibility header form:
        //   [@mode] containerName { ... }
        // Baseical form remains: containerName[@mode] { ... }
        let prefixed_context = if self.check(&TokenKind::LBracket)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::At)
            ) {
            Some(self.parse_context_header_bracket()?)
        } else if self.check_ident_name("primitive")
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::LBracket)
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::At)
            )
            && matches!(
                self.tokens.get(self.pos + 3).map(|tok| &tok.kind),
                Some(TokenKind::RBracket)
            )
            && matches!(
                self.tokens.get(self.pos + 4).map(|tok| &tok.kind),
                Some(TokenKind::Identifier(_))
            )
            && matches!(
                self.tokens.get(self.pos + 5).map(|tok| &tok.kind),
                Some(TokenKind::LBrace)
            )
        {
            // Compatibility prefix:
            //   primitive[@] containerName { ... }
            self.expect_ident()?;
            self.expect_kind(TokenKind::LBracket)?;
            self.expect_kind(TokenKind::At)?;
            self.expect_kind(TokenKind::RBracket)?;
            Some(Context::Primitive)
        } else {
            None
        };

        let name = self.expect_ident()?;

        // Material V2 makes the container role an explicit definition:
        // `name :: program|kernel|machine|root|primitive { ... }`.
        let v2_context = if self.check_double_colon() {
            self.expect_double_colon()?;
            let role = self.expect_ident()?;
            match role.to_ascii_lowercase().as_str() {
                "program" => None,
                "runtime" => Some(Context::Runtime),
                "network" => Some(Context::Network),
                "system" => Some(Context::System),
                "kernel" => Some(Context::Kernel),
                "tooling" => Some(Context::Tooling),
                "compiler" => Some(Context::Compiler),
                "security" => Some(Context::Security),
                "inspect" => Some(Context::Inspect),
                "ai" => Some(Context::Ai),
                "machine" => Some(Context::Machine),
                "root" => Some(Context::Root),
                "primitive" | "basal" => Some(Context::Primitive),
                other => {
                    return self.error_here(format!(
                        "E_V2_CONTAINER_ROLE_001: unknown definition role '{other}'"
                    ))
                }
            }
        } else {
            prefixed_context.clone()
        };

        let visibility = if self.check(&TokenKind::LBracket) {
            let save = self.pos;
            self.advance(); // [
            let vis = match &self.current().kind {
                TokenKind::Identifier(s)
                    if matches!(
                        s.as_str(),
                        "Pub"
                            | "public"
                            | "Priv"
                            | "private"
                            | "Inter"
                            | "internal"
                            | "Strict"
                            | "strictly"
                    ) =>
                {
                    let v = match s.as_str() {
                        "Pub" | "public" => Visibility::Public,
                        "Priv" | "private" => Visibility::Private,
                        "Inter" | "internal" => Visibility::Internal,
                        "Strict" | "strictly" => Visibility::Strictly,
                        _ => unreachable!(),
                    };
                    self.advance();
                    self.expect_kind(TokenKind::RBracket)?;
                    Some(v)
                }
                _ => {
                    self.pos = save;
                    None
                }
            };
            vis
        } else {
            None
        };

        // Base compatibility form:
        //   name[primitive[@]] { ... }
        // This keeps `[ @primitive ]` as official internal context while accepting
        // the nested header variant from extended material examples.
        let nested_primitive_header = if self.check(&TokenKind::LBracket)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Identifier(s)) if s.eq_ignore_ascii_case("primitive")
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::LBracket)
            )
            && matches!(
                self.tokens.get(self.pos + 3).map(|tok| &tok.kind),
                Some(TokenKind::At)
            )
            && matches!(
                self.tokens.get(self.pos + 4).map(|tok| &tok.kind),
                Some(TokenKind::RBracket)
            )
            && matches!(
                self.tokens.get(self.pos + 5).map(|tok| &tok.kind),
                Some(TokenKind::RBracket)
            ) {
            self.expect_kind(TokenKind::LBracket)?;
            self.expect_ident()?;
            self.expect_kind(TokenKind::LBracket)?;
            self.expect_kind(TokenKind::At)?;
            self.expect_kind(TokenKind::RBracket)?;
            self.expect_kind(TokenKind::RBracket)?;
            true
        } else {
            false
        };

        let mut context = if nested_primitive_header {
            Some(Context::Primitive)
        } else if self.check(&TokenKind::LBracket)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::At)
            )
        {
            Some(self.parse_context_header_bracket()?)
        } else {
            v2_context
        };

        // Compatibilidad controlada para la notación canónica extendida:
        // `primitive[@]:` / `primitive[@] target <arch>:`
        // En esta forma, `[@]` no expresa root sino el encabezado corto de primitive.
        if name == "primitive"
            && matches!(context, Some(Context::Root))
            && (self.check_ident_name("target") || self.check(&TokenKind::Colon))
        {
            context = Some(Context::Primitive);
        }

        let mut primitive_header_had_colon = false;
        let primitive_target =
            if matches!(context, Some(Context::Primitive)) && self.match_ident_name("target") {
                let arch = self.expect_ident()?;
                if self.match_kind(&TokenKind::Colon) {
                    // Optional ':' keeps room for extended primitive header forms while
                    // preserving the active brace-based container surface.
                    primitive_header_had_colon = true;
                }
                Some(Section::PrimitiveTarget(arch))
            } else {
                None
            };

        let primitive_colon_without_target =
            matches!(context, Some(Context::Primitive)) && self.match_kind(&TokenKind::Colon);

        let use_brace_block = if self.match_kind(&TokenKind::LBrace) {
            true
        } else if matches!(context, Some(Context::Primitive))
            && (primitive_header_had_colon || primitive_colon_without_target)
        {
            false
        } else {
            return self.error_here("Expected LBrace");
        };

        if context.is_none() && use_brace_block && self.infer_implicit_kernel_context(&name) {
            context = Some(Context::Kernel);
        }

        let mut sections = Vec::new();
        if let Some(target) = primitive_target {
            sections.push(target);
        }
        let previous_context = self
            .active_context
            .replace(context.clone().unwrap_or(Context::Root));
        let mut stc_seen = false;
        if use_brace_block {
            while !self.check(&TokenKind::RBrace) {
                let line = self.current().line;
                let column = self.current().column;
                let section = self.parse_section(&context)?;
                if matches!(section, Section::Stc(_)) {
                    if stc_seen {
                        return Err(Diagnostic::new(
                            "E_STC_DUPLICATE_001: a container may declare exactly one `stc:` section",
                            line,
                            column,
                        ));
                    }
                    stc_seen = true;
                }
                sections.push(section);
            }
            self.expect_kind(TokenKind::RBrace)?;
        } else {
            while !self.at_eof() {
                let line = self.current().line;
                let column = self.current().column;
                let section = self.parse_section(&context)?;
                if matches!(section, Section::Stc(_)) {
                    if stc_seen {
                        return Err(Diagnostic::new(
                            "E_STC_DUPLICATE_001: a container may declare exactly one `stc:` section",
                            line,
                            column,
                        ));
                    }
                    stc_seen = true;
                }
                sections.push(section);
            }
        }
        self.active_context = previous_context;

        Ok(Container {
            visibility,
            name,
            context,
            sections,
        })
    }

    fn parse_visibility_kw_opt(&mut self) -> Option<Visibility> {
        if self.match_kind(&TokenKind::KeywordPublic) {
            return Some(Visibility::Public);
        }
        if self.match_kind(&TokenKind::KeywordPrivate) {
            return Some(Visibility::Private);
        }
        if self.match_kind(&TokenKind::KeywordInternal) {
            return Some(Visibility::Internal);
        }
        if self.match_kind(&TokenKind::KeywordStrictly) {
            return Some(Visibility::Strictly);
        }
        None
    }

    fn parse_primitive_symbol_list_section(
        &mut self,
        keyword: TokenKind,
        ctor: fn(Vec<String>) -> Section,
    ) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        self.expect_kind(keyword)?;
        self.expect_kind(TokenKind::Colon)?;
        let mut items = Vec::new();
        while !self.at_eof()
            && !self.check(&TokenKind::RBrace)
            && !self.is_primitive_top_level_section_start_at_column_or_left(header_column)
        {
            let symbol = self.parse_primitive_word_segment()?;
            items.push(symbol);
            // Compatibility surface: allow rich declarative tails in list-like
            // primitive sections and keep only the leading symbol identity.
            while !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
                self.advance();
            }
            if self.check(&TokenKind::RBrace) {
                return self.error_here("Expected Semicolon");
            }
            self.expect_kind(TokenKind::Semicolon)?;
        }
        Ok(ctor(items))
    }

    fn parse_primitive_named_clause_section_with_keyword(
        &mut self,
        keyword: TokenKind,
    ) -> Result<(Vec<String>, Vec<PrimitiveDirectiveItem>), Diagnostic> {
        let header_column = self.current().column;
        self.expect_kind(keyword)?;
        let name = self.parse_primitive_dotted_path()?;
        let clauses = if self.match_kind(&TokenKind::Colon) {
            self.parse_primitive_operation_clauses(header_column)?
        } else {
            let inline_clauses = self.parse_primitive_operation_clauses(header_column)?;
            if inline_clauses.is_empty() {
                return self.error_here("Expected Colon");
            }
            inline_clauses
        };
        Ok((name, clauses))
    }

    fn parse_primitive_anchor_block_section(&mut self) -> Result<Section, Diagnostic> {
        let (name, clauses) =
            self.parse_primitive_named_clause_section_with_keyword(TokenKind::KeywordAnchor)?;
        Ok(Section::AnchorBlock(PrimitiveAnchorBlockSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_window_block_section(&mut self) -> Result<Section, Diagnostic> {
        let (name, clauses) =
            self.parse_primitive_named_clause_section_with_keyword(TokenKind::KeywordWindow)?;
        Ok(Section::WindowBlock(PrimitiveWindowBlockSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_signal_block_section(&mut self) -> Result<Section, Diagnostic> {
        let (name, clauses) =
            self.parse_primitive_named_clause_section_with_keyword(TokenKind::KeywordSignal)?;
        Ok(Section::SignalBlock(PrimitiveSignalBlockSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_bus_block_section(&mut self) -> Result<Section, Diagnostic> {
        let (name, clauses) =
            self.parse_primitive_named_clause_section_with_keyword(TokenKind::KeywordBus)?;
        Ok(Section::BusBlock(PrimitiveBusBlockSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_bridge_path_segment(&mut self) -> Result<String, Diagnostic> {
        if let TokenKind::Identifier(s) = &self.current().kind {
            let out = s.clone();
            self.advance();
            return Ok(out);
        }

        if self.match_kind(&TokenKind::KeywordBind) {
            return Ok("bind".to_string());
        }

        if self.match_kind(&TokenKind::KeywordBinary) {
            return Ok("binary".to_string());
        }

        self.error_here("expected bridge path segment")
    }

    fn parse_primitive_bridge_path(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut path = vec![self.parse_primitive_bridge_path_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            path.push(self.parse_primitive_bridge_path_segment()?);
        }
        Ok(path)
    }

    fn parse_primitive_bridge_target(&mut self) -> Result<PrimitiveBridgeTarget, Diagnostic> {
        let path = self.parse_primitive_bridge_path()?;

        if path.len() >= 2 && path[0] == "root" {
            return Ok(PrimitiveBridgeTarget::RootPath(path));
        }

        if path.len() == 2 && path[0] == "machine" {
            return Ok(PrimitiveBridgeTarget::MachinePath(path));
        }

        self.error_here(format!(
            "bridge section in [@primitive] only allows 'root.<...>' or 'machine.<...>', got '{}'",
            path.join(".")
        ))
    }

    fn parse_primitive_bridge_section(&mut self) -> Result<Section, Diagnostic> {
        self.expect_kind(TokenKind::KeywordBridge)?;
        self.expect_kind(TokenKind::Colon)?;

        let mut items = Vec::new();
        while !self.at_eof()
            && !self.is_primitive_section_boundary()
            && !self.check(&TokenKind::RBrace)
        {
            let target = self.parse_primitive_bridge_target()?;
            self.expect_kind(TokenKind::Semicolon)?;
            items.push(target);
        }

        Ok(Section::Bridge(items))
    }

    fn parse_primitive_word_segment(&mut self) -> Result<String, Diagnostic> {
        if let TokenKind::Identifier(s) = &self.current().kind {
            let out = s.clone();
            self.advance();
            return Ok(out);
        }

        if let Some(word) = Self::primitive_directive_word_from_keyword(&self.current().kind) {
            self.advance();
            return Ok(word.to_string());
        }

        self.error_here("expected primitive symbol segment")
    }

    fn parse_primitive_dotted_path(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut path = vec![self.parse_primitive_word_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            path.push(self.parse_primitive_word_segment()?);
        }
        Ok(path)
    }

    fn parse_primitive_capability_section(&mut self) -> Result<Section, Diagnostic> {
        if !self.match_ident_name("capability") {
            return self.error_here("Expected capability section");
        }

        let name = self.expect_ident()?;
        self.expect_kind(TokenKind::Colon)?;

        let mut rules = Vec::new();
        while !self.at_eof()
            && !self.is_primitive_section_boundary()
            && !self.check(&TokenKind::RBrace)
        {
            let action = if self.match_ident_name("allow") {
                PrimitiveCapabilityAction::Allow
            } else if self.match_ident_name("deny") {
                PrimitiveCapabilityAction::Deny
            } else {
                return self.error_here(
                    "capability rule must start with 'allow' or 'deny' in [@primitive] context",
                );
            };

            let path = self.parse_primitive_dotted_path()?;
            self.expect_kind(TokenKind::Semicolon)?;

            rules.push(PrimitiveCapabilityRule { action, path });
        }

        Ok(Section::Capability(PrimitiveCapabilitySection {
            name,
            rules,
        }))
    }

    fn parse_primitive_bitfield_scalar(&mut self) -> Result<String, Diagnostic> {
        if let Some(number) = self.match_number() {
            return Ok(number);
        }
        self.parse_primitive_word_segment()
    }

    fn parse_primitive_bitfield_section(&mut self) -> Result<Section, Diagnostic> {
        if !self.match_ident_name("bitfield") {
            return self.error_here("Expected bitfield section");
        }

        let name = self.expect_ident()?;
        let base = self.expect_ident()?;
        self.expect_kind(TokenKind::Colon)?;

        let mut entries = Vec::new();
        while !self.at_eof()
            && !self.is_primitive_section_boundary()
            && !self.check(&TokenKind::RBrace)
        {
            if self.match_ident_name("bit") {
                let index = self.parse_primitive_bitfield_scalar()?;
                let entry_name = self.expect_ident()?;
                self.expect_kind(TokenKind::Semicolon)?;
                entries.push(PrimitiveBitfieldEntry::Bit {
                    index,
                    name: entry_name,
                });
                continue;
            }

            if self.match_ident_name("bits") {
                let start_or_pair = self.parse_primitive_bitfield_scalar()?;
                let (start, end) = if let Some((left, right)) = start_or_pair.split_once("..") {
                    (left.to_string(), right.to_string())
                } else {
                    self.expect_kind(TokenKind::Dot)?;
                    let end = if self.match_kind(&TokenKind::Dot) {
                        self.parse_primitive_bitfield_scalar()?
                    } else if let TokenKind::Number(n) = &self.current().kind {
                        if let Some(stripped) = n.strip_prefix('.') {
                            let out = stripped.to_string();
                            self.advance();
                            out
                        } else {
                            return self.error_here("Expected Dot");
                        }
                    } else {
                        return self.error_here("Expected Dot");
                    };
                    (start_or_pair, end)
                };
                let entry_name = self.expect_ident()?;
                self.expect_kind(TokenKind::Semicolon)?;
                entries.push(PrimitiveBitfieldEntry::Bits {
                    start,
                    end,
                    name: entry_name,
                });
                continue;
            }

            return self.error_here(
                "bitfield entry must start with 'bit' or 'bits' in [@primitive] context",
            );
        }

        Ok(Section::Bitfield(PrimitiveBitfieldSection {
            name,
            base,
            entries,
        }))
    }

    fn parse_primitive_type_alias_section(&mut self) -> Result<Section, Diagnostic> {
        if !self.match_ident_name("type") {
            return self.error_here("Expected type alias section");
        }

        let name = self.expect_ident()?;
        self.expect_kind(TokenKind::Eq)?;

        let start = self.pos;
        while !self.at_eof()
            && !self.check(&TokenKind::Semicolon)
            && !self.check(&TokenKind::RBrace)
        {
            self.advance();
        }
        if self.at_eof() {
            return self.error_here("Expected Semicolon");
        }
        if self.check(&TokenKind::RBrace) {
            return self.error_here("Expected Semicolon");
        }
        let end = self.pos;
        self.expect_kind(TokenKind::Semicolon)?;

        let atoms = self.tokens[start..end]
            .iter()
            .map(Self::primitive_directive_atom_from_token)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Section::PrimitiveTypeAlias(PrimitiveTypeAliasSection {
            name,
            value: PrimitiveDirectiveItem { atoms },
        }))
    }

    fn parse_primitive_operation_section(&mut self) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        if !self.match_ident_name("operation") {
            return self.error_here("Expected operation section");
        }

        let name = self.parse_primitive_dotted_path()?;
        self.expect_kind(TokenKind::LParen)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let param_name = self.expect_ident()?;
                self.expect_kind(TokenKind::Colon)?;
                let param_ty = self.expect_type_name()?;
                params.push(PrimitiveOperationParam {
                    name: param_name,
                    ty: param_ty,
                });

                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }

        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::Colon)?;

        let clauses = self.parse_primitive_operation_clauses(header_column)?;
        Ok(Section::Operation(PrimitiveOperationSection {
            name,
            params,
            clauses,
        }))
    }

    fn parse_primitive_operation_compat_section(&mut self) -> Result<Section, Diagnostic> {
        if !self.match_ident_name("operation") {
            return self.error_here("Expected operation section");
        }
        self.expect_kind(TokenKind::Colon)?;

        let mut clauses = Vec::new();
        while !self.at_eof()
            && !self.is_primitive_section_boundary()
            && !self.check(&TokenKind::RBrace)
        {
            self.expect_ident()?;
            self.expect_kind(TokenKind::LBrace)?;

            while !self.check(&TokenKind::RBrace) {
                let start = self.pos;
                while !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
                    self.advance();
                }

                if self.check(&TokenKind::RBrace) {
                    return self.error_here("Expected Semicolon");
                }

                let end = self.pos;
                self.expect_kind(TokenKind::Semicolon)?;

                let atoms = self.tokens[start..end]
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, tok)| {
                        if idx == 1 && matches!(tok.kind, TokenKind::Colon) {
                            None
                        } else {
                            Some(tok)
                        }
                    })
                    .map(Self::primitive_directive_atom_from_token)
                    .collect::<Result<Vec<_>, _>>()?;
                let mut atoms = atoms;
                if let Some(PrimitiveDirectiveAtom::Word(head)) = atoms.first_mut() {
                    if matches!(
                        head.as_str(),
                        "source" | "requires" | "emit" | "reads" | "await" | "stable"
                    ) {
                        *head = format!("compat_{}", head);
                    }
                }
                if matches!(
                    atoms.first(),
                    Some(PrimitiveDirectiveAtom::Word(head)) if head == "stable"
                ) {
                    if let Some(PrimitiveDirectiveAtom::Word(flag)) = atoms.get(1) {
                        if flag == "true" || flag == "false" {
                            let cycles = if flag == "true" { "1" } else { "0" };
                            atoms = vec![
                                PrimitiveDirectiveAtom::Word("stable".into()),
                                PrimitiveDirectiveAtom::Word("after".into()),
                                PrimitiveDirectiveAtom::Number(cycles.into()),
                                PrimitiveDirectiveAtom::Word("cycles".into()),
                            ];
                        }
                    }
                }
                if matches!(
                    atoms.first(),
                    Some(PrimitiveDirectiveAtom::Word(head)) if head == "await"
                ) {
                    if let Some(timeout_idx) = atoms.iter().position(|atom| {
                        matches!(atom, PrimitiveDirectiveAtom::Word(word) if word == "timeout")
                    }) {
                        let number_idx = atoms
                            .iter()
                            .enumerate()
                            .skip(timeout_idx + 1)
                            .find_map(|(idx, atom)| match atom {
                                PrimitiveDirectiveAtom::Number(_) => Some(idx),
                                _ => None,
                            });
                        if let Some(number_idx) = number_idx {
                            let has_cycle_word_after = atoms.iter().skip(number_idx + 1).any(|atom| {
                                matches!(atom, PrimitiveDirectiveAtom::Word(word) if word == "cycle" || word == "cycles")
                            });
                            if !has_cycle_word_after {
                                atoms.insert(
                                    number_idx + 1,
                                    PrimitiveDirectiveAtom::Word("cycles".into()),
                                );
                            }
                        }
                    }
                }
                clauses.push(PrimitiveDirectiveItem { atoms });
            }

            self.expect_kind(TokenKind::RBrace)?;
        }

        if clauses.is_empty() {
            return self.error_here("operation section requires at least one clause block");
        }

        Ok(Section::Operation(PrimitiveOperationSection {
            name: vec!["compat".into(), "operation".into()],
            params: Vec::new(),
            clauses,
        }))
    }

    fn parse_primitive_port_section(&mut self) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        if !self.match_ident_name("port") {
            return self.error_here("Expected port section");
        }

        let name = self.parse_primitive_dotted_path()?;
        self.expect_kind(TokenKind::Colon)?;
        let clauses = self.parse_primitive_operation_clauses(header_column)?;
        Ok(Section::Port(PrimitivePortSection { name, clauses }))
    }

    fn parse_primitive_signature_section(&mut self) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        if !self.match_ident_name("signature") {
            return self.error_here("Expected signature section");
        }

        let name = self.parse_primitive_dotted_path()?;
        self.expect_kind(TokenKind::Colon)?;
        let clauses = self.parse_primitive_operation_clauses(header_column)?;
        Ok(Section::Signature(PrimitiveSignatureSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_invariant_section(&mut self) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        if !self.match_ident_name("invariant") {
            return self.error_here("Expected invariant section");
        }

        let name = self.parse_primitive_dotted_path()?;
        self.expect_kind(TokenKind::Colon)?;
        let clauses = self.parse_primitive_operation_clauses(header_column)?;
        Ok(Section::Invariant(PrimitiveInvariantSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_constraint_section(&mut self) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        if !self.match_ident_name("constraint") {
            return self.error_here("Expected constraint section");
        }

        let name = self.parse_primitive_dotted_path()?;
        self.expect_kind(TokenKind::Colon)?;
        let clauses = self.parse_primitive_operation_clauses(header_column)?;
        Ok(Section::Constraint(PrimitiveConstraintSection {
            name,
            clauses,
        }))
    }

    fn parse_primitive_entry_section(&mut self) -> Result<Section, Diagnostic> {
        let header_column = self.current().column;
        if !self.match_ident_name("entry") {
            return self.error_here("Expected entry section");
        }

        let name = self.parse_primitive_dotted_path()?;
        self.expect_kind(TokenKind::Colon)?;
        let clauses = self.parse_primitive_operation_clauses(header_column)?;
        Ok(Section::Entry(PrimitiveEntrySection { name, clauses }))
    }

    fn primitive_directive_kind_from_ident(name: &str) -> Option<PrimitiveDirectiveKind> {
        match name {
            "id" => Some(PrimitiveDirectiveKind::Id),
            "module" => Some(PrimitiveDirectiveKind::Module),
            "type" => Some(PrimitiveDirectiveKind::Type),
            "version" => Some(PrimitiveDirectiveKind::Version),
            "isa" => Some(PrimitiveDirectiveKind::Isa),
            "opcode" => Some(PrimitiveDirectiveKind::Opcode),
            "encoding" => Some(PrimitiveDirectiveKind::Encoding),
            "relocation" | "reloc" => Some(PrimitiveDirectiveKind::Relocation),
            "abi" => Some(PrimitiveDirectiveKind::Abi),
            "clock" => Some(PrimitiveDirectiveKind::Clock),
            "edge" => Some(PrimitiveDirectiveKind::Edge),
            "cycle" => Some(PrimitiveDirectiveKind::Cycle),
            "domain" => Some(PrimitiveDirectiveKind::Domain),
            "latency" => Some(PrimitiveDirectiveKind::Latency),
            "timeout" => Some(PrimitiveDirectiveKind::Timeout),
            "voltage" => Some(PrimitiveDirectiveKind::Voltage),
            "current" => Some(PrimitiveDirectiveKind::Current),
            "noise" => Some(PrimitiveDirectiveKind::Noise),
            "metastability" => Some(PrimitiveDirectiveKind::Metastability),
            "power" => Some(PrimitiveDirectiveKind::Power),
            "powergate" | "powerGate" => Some(PrimitiveDirectiveKind::PowerGate),
            "temperature" => Some(PrimitiveDirectiveKind::Temperature),
            "thermalstate" | "thermalState" => Some(PrimitiveDirectiveKind::ThermalState),
            "throttlestate" | "throttleState" => Some(PrimitiveDirectiveKind::ThrottleState),
            "watchdog" => Some(PrimitiveDirectiveKind::Watchdog),
            "pin" => Some(PrimitiveDirectiveKind::Pin),
            "port" => Some(PrimitiveDirectiveKind::Port),
            "wire" => Some(PrimitiveDirectiveKind::Wire),
            "width" => Some(PrimitiveDirectiveKind::Width),
            "interruptline" | "interruptLine" => Some(PrimitiveDirectiveKind::InterruptLine),
            "addr" => Some(PrimitiveDirectiveKind::Addr),
            "register" => Some(PrimitiveDirectiveKind::Register),
            "map" => Some(PrimitiveDirectiveKind::Map),
            "read" => Some(PrimitiveDirectiveKind::Read),
            "write" => Some(PrimitiveDirectiveKind::Write),
            "volatile" => Some(PrimitiveDirectiveKind::Volatile),
            "dmachannel" | "dmaChannel" => Some(PrimitiveDirectiveKind::DmaChannel),
            "cachestate" | "cacheState" => Some(PrimitiveDirectiveKind::CacheState),
            "tlbstate" | "tlbState" => Some(PrimitiveDirectiveKind::TlbState),
            "firmwareregion" | "firmwareRegion" => Some(PrimitiveDirectiveKind::FirmwareRegion),
            "bootromregion" | "bootRomRegion" => Some(PrimitiveDirectiveKind::BootRomRegion),
            "biosregion" | "biosRegion" => Some(PrimitiveDirectiveKind::BiosRegion),
            "uefihandoff" | "uefiHandoff" => Some(PrimitiveDirectiveKind::UefiHandoff),
            "resetvector" | "resetVector" => Some(PrimitiveDirectiveKind::ResetVector),
            "bootwindow" | "bootWindow" => Some(PrimitiveDirectiveKind::BootWindow),
            "bootanchor" | "bootAnchor" => Some(PrimitiveDirectiveKind::BootAnchor),
            "firmwaretable" | "firmwareTable" => Some(PrimitiveDirectiveKind::FirmwareTable),
            "devicetree" | "deviceTree" => Some(PrimitiveDirectiveKind::DeviceTree),
            "acpitable" | "acpiTable" => Some(PrimitiveDirectiveKind::AcpiTable),
            "microcodestate" | "microcodeState" => Some(PrimitiveDirectiveKind::MicrocodeState),
            "route" => Some(PrimitiveDirectiveKind::Route),
            "valid" => Some(PrimitiveDirectiveKind::Valid),
            "ready" => Some(PrimitiveDirectiveKind::Ready),
            "ack" => Some(PrimitiveDirectiveKind::Ack),
            "trigger" => Some(PrimitiveDirectiveKind::Trigger),
            "guard" => Some(PrimitiveDirectiveKind::Guard),
            "irq" => Some(PrimitiveDirectiveKind::Irq),
            "fault" => Some(PrimitiveDirectiveKind::Fault),
            "error" => Some(PrimitiveDirectiveKind::Error),
            "retry" => Some(PrimitiveDirectiveKind::Retry),
            "fallback" => Some(PrimitiveDirectiveKind::Fallback),
            "atomic" => Some(PrimitiveDirectiveKind::Atomic),
            "barrier" => Some(PrimitiveDirectiveKind::Barrier),
            "commit" => Some(PrimitiveDirectiveKind::Commit),
            "reset" => Some(PrimitiveDirectiveKind::Reset),
            "trace" => Some(PrimitiveDirectiveKind::Trace),
            "ptr" => Some(PrimitiveDirectiveKind::Ptr),
            "load" => Some(PrimitiveDirectiveKind::Load),
            "store" => Some(PrimitiveDirectiveKind::Store),
            "mmio" => Some(PrimitiveDirectiveKind::Mmio),
            "trap" => Some(PrimitiveDirectiveKind::Trap),
            "fence" => Some(PrimitiveDirectiveKind::Fence),
            "comb" => Some(PrimitiveDirectiveKind::Comb),
            "seq" => Some(PrimitiveDirectiveKind::Seq),
            "assign" => Some(PrimitiveDirectiveKind::Assign),
            "always" => Some(PrimitiveDirectiveKind::Always),
            "transition" => Some(PrimitiveDirectiveKind::Transition),
            "drive" => Some(PrimitiveDirectiveKind::Drive),
            "release" => Some(PrimitiveDirectiveKind::Release),
            "mux" => Some(PrimitiveDirectiveKind::Mux),
            "decode" => Some(PrimitiveDirectiveKind::Decode),
            "encode" => Some(PrimitiveDirectiveKind::Encode),
            "pipeline" => Some(PrimitiveDirectiveKind::Pipeline),
            "pipelinestate" | "pipelineState" => Some(PrimitiveDirectiveKind::PipelineState),
            "branchpredictorstate" | "branchPredictorState" => {
                Some(PrimitiveDirectiveKind::BranchPredictorState)
            }
            "reorderstate" | "reorderState" => Some(PrimitiveDirectiveKind::ReorderState),
            "outoforder" | "outOfOrder" => Some(PrimitiveDirectiveKind::OutOfOrder),
            "hazard" => Some(PrimitiveDirectiveKind::Hazard),
            "controller" => Some(PrimitiveDirectiveKind::Controller),
            "peripheral" => Some(PrimitiveDirectiveKind::Peripheral),
            "phy" => Some(PrimitiveDirectiveKind::Phy),
            "lane" => Some(PrimitiveDirectiveKind::Lane),
            "channel" => Some(PrimitiveDirectiveKind::Channel),
            "protocol" => Some(PrimitiveDirectiveKind::Protocol),
            "physicalfault" | "physicalFault" | "physicalfailure" | "physicalFailure" => {
                Some(PrimitiveDirectiveKind::PhysicalFault)
            }
            "busfault" | "busFault" => Some(PrimitiveDirectiveKind::BusFault),
            "signalloss" | "signalLoss" => Some(PrimitiveDirectiveKind::SignalLoss),
            "timingviolation" | "timingViolation" => Some(PrimitiveDirectiveKind::TimingViolation),
            "thermalviolation" | "thermalViolation" => {
                Some(PrimitiveDirectiveKind::ThermalViolation)
            }
            "metastabilityrisk" | "metastabilityRisk" => {
                Some(PrimitiveDirectiveKind::MetastabilityRisk)
            }
            "dmaviolation" | "dmaViolation" => Some(PrimitiveDirectiveKind::DmaViolation),
            "cachecoherenceviolation" | "cacheCoherenceViolation" => {
                Some(PrimitiveDirectiveKind::CacheCoherenceViolation)
            }
            "mmiosideeffectviolation" | "mmioSideEffectViolation" => {
                Some(PrimitiveDirectiveKind::MmioSideEffectViolation)
            }
            _ => None,
        }
    }

    fn is_primitive_identifier_directive_section_start(&self) -> bool {
        if !matches!(
            self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
            Some(TokenKind::Colon)
        ) {
            return false;
        }

        match &self.current().kind {
            TokenKind::Identifier(name) => {
                Self::primitive_directive_kind_from_ident(name).is_some()
            }
            _ => false,
        }
    }

    fn is_primitive_type_alias_section_start(&self) -> bool {
        matches!(
            (
                self.tokens.get(self.pos).map(|tok| &tok.kind),
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
            ),
            (
                Some(TokenKind::Identifier(name)),
                Some(TokenKind::Identifier(_)),
                Some(TokenKind::Eq)
            ) if name == "type"
        )
    }

    fn is_primitive_named_section_start(&self) -> bool {
        matches!(
            &self.current().kind,
            TokenKind::Identifier(name)
                if matches!(
                    name.as_str(),
                    "capability"
                        | "bitfield"
                        | "operation"
                        | "port"
                        | "signature"
                        | "invariant"
                        | "constraint"
                        | "entry"
                )
        )
    }

    fn is_security_identifier_section_start(&self) -> bool {
        if !matches!(
            self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
            Some(TokenKind::Colon)
        ) {
            return false;
        }

        matches!(
            &self.current().kind,
            TokenKind::Identifier(name)
                if matches!(
                    name.as_str(),
                    "asset"
                        | "identity"
                        | "secret"
                        | "finding"
                        | "incident"
                        | "policy"
                        | "evidence"
                )
        )
    }

    fn is_foreign_identifier_section_start(&self) -> bool {
        matches!(
            (
                self.tokens.get(self.pos).map(|tok| &tok.kind),
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
            ),
            (Some(TokenKind::Identifier(name)), Some(TokenKind::Colon))
                if name.eq_ignore_ascii_case("foreign")
        )
    }

    fn is_primitive_known_section_start(&self) -> bool {
        self.is_section_start()
            || self.is_primitive_named_section_start()
            || self.is_primitive_type_alias_section_start()
    }

    fn is_identifier_colon_section_start(&self) -> bool {
        matches!(self.current().kind, TokenKind::Identifier(_))
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            )
    }

    fn is_type_contract_section_start(&self) -> bool {
        matches!(
            (
                self.tokens.get(self.pos).map(|tok| &tok.kind),
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
            ),
            (Some(TokenKind::Identifier(name)), Some(TokenKind::Colon)) if name == "type"
        )
    }

    fn is_primitive_top_level_section_start_at_column_or_left(&self, col: usize) -> bool {
        self.current().column <= col
            && (self.is_primitive_known_section_start() || self.is_identifier_colon_section_start())
    }

    fn is_primitive_section_boundary(&self) -> bool {
        self.is_primitive_known_section_start() || self.is_identifier_colon_section_start()
    }

    fn parse_primitive_operation_clauses(
        &mut self,
        header_column: usize,
    ) -> Result<Vec<PrimitiveDirectiveItem>, Diagnostic> {
        let mut clauses = Vec::new();

        while !self.at_eof()
            && !self.check(&TokenKind::RBrace)
            && !self.is_primitive_top_level_section_start_at_column_or_left(header_column)
        {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }

            let start = self.pos;
            while !self.check(&TokenKind::Semicolon)
                && !self.at_eof()
                && !self.check(&TokenKind::RBrace)
                && !self.is_primitive_top_level_section_start_at_column_or_left(header_column)
            {
                self.advance();
            }

            let end = self.pos;
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
            }

            if start == end {
                continue;
            }

            let atoms = self.tokens[start..end]
                .iter()
                .map(Self::primitive_directive_atom_from_token)
                .collect::<Result<Vec<_>, _>>()?;
            clauses.push(PrimitiveDirectiveItem { atoms });
        }

        Ok(clauses)
    }

    fn parse_primitive_directive_section(
        &mut self,
        kind: PrimitiveDirectiveKind,
    ) -> Result<Section, Diagnostic> {
        match &self.current().kind {
            TokenKind::Identifier(_) | TokenKind::KeywordReg => self.advance(),
            _ => return self.error_here("Expected primitive directive section"),
        };

        self.expect_kind(TokenKind::Colon)?;
        let items = self.parse_primitive_directive_items_section()?;
        Ok(Section::PrimitiveDirective(PrimitiveDirectiveSection {
            kind,
            items,
        }))
    }

    fn primitive_directive_word_from_keyword(kind: &TokenKind) -> Option<&'static str> {
        match kind {
            TokenKind::KeywordUse => Some("use"),
            TokenKind::KeywordImport => Some("import"),
            TokenKind::KeywordStc => Some("stc"),
            TokenKind::KeywordLocal => Some("local"),
            TokenKind::KeywordFn => Some("fn"),
            TokenKind::KeywordMove => Some("move"),
            TokenKind::KeywordProbe => Some("probe"),
            TokenKind::KeywordBind => Some("bind"),
            TokenKind::KeywordBinary => Some("binary"),
            TokenKind::KeywordRaw => Some("raw"),
            TokenKind::KeywordStructure => Some("structure"),
            TokenKind::KeywordStr => Some("str"),
            TokenKind::KeywordFinal => Some("final"),
            TokenKind::KeywordFa => Some("fa"),
            TokenKind::KeywordPublic => Some("public"),
            TokenKind::KeywordPrivate => Some("private"),
            TokenKind::KeywordInternal => Some("internal"),
            TokenKind::KeywordStrictly => Some("strictly"),
            TokenKind::KeywordReturn => Some("return"),
            TokenKind::KeywordDev => Some("dev"),
            TokenKind::KeywordIf => Some("if"),
            TokenKind::KeywordElse => Some("else"),
            TokenKind::KeywordWhile => Some("while"),
            TokenKind::KeywordRef => Some("ref"),
            TokenKind::KeywordMain => Some("main"),
            TokenKind::KeywordSkkMain => Some("skkMain"),
            TokenKind::KeywordTrue => Some("true"),
            TokenKind::KeywordFalse => Some("false"),
            TokenKind::KeywordLy => Some("ly"),
            TokenKind::KeywordCnt => Some("cnt"),
            TokenKind::KeywordCut => Some("cut"),
            TokenKind::KeywordAnchor => Some("anchor"),
            TokenKind::KeywordSlot => Some("slot"),
            TokenKind::KeywordPattern => Some("pattern"),
            TokenKind::KeywordMask => Some("mask"),
            TokenKind::KeywordWindow => Some("window"),
            TokenKind::KeywordSignal => Some("signal"),
            TokenKind::KeywordBus => Some("bus"),
            TokenKind::KeywordState => Some("state"),
            TokenKind::KeywordTiming => Some("timing"),
            TokenKind::KeywordEmit => Some("emit"),
            TokenKind::KeywordSample => Some("sample"),
            TokenKind::KeywordLatch => Some("latch"),
            TokenKind::KeywordAssert => Some("assert"),
            TokenKind::KeywordRupture => Some("rupture"),
            TokenKind::KeywordBridge => Some("bridge"),
            TokenKind::KeywordSeal => Some("seal"),
            TokenKind::KeywordReg => Some("reg"),
            TokenKind::KeywordFlag => Some("flag"),
            TokenKind::KeywordLabel => Some("label"),
            TokenKind::KeywordInst => Some("inst"),
            _ => None,
        }
    }

    fn primitive_directive_atom_from_token(
        token: &Token,
    ) -> Result<PrimitiveDirectiveAtom, Diagnostic> {
        match &token.kind {
            TokenKind::Identifier(s) => Ok(PrimitiveDirectiveAtom::Word(s.clone())),
            TokenKind::Number(s) => Ok(PrimitiveDirectiveAtom::Number(s.clone())),
            TokenKind::String(s) => Ok(PrimitiveDirectiveAtom::StringLiteral(s.clone())),
            TokenKind::Comma => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Comma,
            )),
            TokenKind::Dot => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Dot)),
            TokenKind::At => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::At)),
            TokenKind::Star | TokenKind::StarEq => {
                Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Star))
            }
            TokenKind::Colon => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Colon,
            )),
            TokenKind::Arrow => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Arrow,
            )),
            TokenKind::Arrow2 => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Arrow2,
            )),
            TokenKind::Shift2 | TokenKind::Shift3 => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Shift2,
            )),
            TokenKind::ShiftLeft => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::ShiftLeft,
            )),
            TokenKind::BangShift => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::BangShift,
            )),
            TokenKind::Bang => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Bang)),
            TokenKind::LessDash => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Arrow,
            )),
            TokenKind::Pipe => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Pipe)),
            TokenKind::Amp => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Amp)),
            TokenKind::Caret => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Caret,
            )),
            TokenKind::Tilde => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Tilde,
            )),
            TokenKind::Plus | TokenKind::PlusPlus | TokenKind::PlusEq => {
                Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Plus))
            }
            TokenKind::Minus | TokenKind::MinusMinus | TokenKind::MinusEq => Ok(
                PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Minus),
            ),
            TokenKind::Slash => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Slash,
            )),
            TokenKind::Percent => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Percent,
            )),
            TokenKind::AndAnd => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::AndAnd,
            )),
            TokenKind::OrOr => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::OrOr)),
            TokenKind::Less => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Less)),
            TokenKind::Greater => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::Greater,
            )),
            TokenKind::GreaterEq => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::GreaterEq,
            )),
            TokenKind::LessEq => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::LessEq,
            )),
            TokenKind::EqEq => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::EqEq)),
            TokenKind::Eq => Ok(PrimitiveDirectiveAtom::Punct(PrimitiveDirectivePunct::Eq)),
            TokenKind::NotEq => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::NotEq,
            )),
            TokenKind::LParen => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::LParen,
            )),
            TokenKind::RParen => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::RParen,
            )),
            TokenKind::LBracket => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::LBracket,
            )),
            TokenKind::RBracket => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::RBracket,
            )),
            TokenKind::LBrace => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::LBrace,
            )),
            TokenKind::RBrace => Ok(PrimitiveDirectiveAtom::Punct(
                PrimitiveDirectivePunct::RBrace,
            )),
            keyword => {
                if let Some(word) = Self::primitive_directive_word_from_keyword(keyword) {
                    Ok(PrimitiveDirectiveAtom::Word(word.into()))
                } else {
                    Err(Diagnostic {
                        message: format!(
                            "unexpected token in primitive directive section: {:?}",
                            keyword
                        ),
                        line: token.line,
                        column: token.column,
                    })
                }
            }
        }
    }

    fn parse_primitive_directive_items_section(
        &mut self,
    ) -> Result<Vec<PrimitiveDirectiveItem>, Diagnostic> {
        let mut items = Vec::new();

        while !self.at_eof()
            && !self.is_primitive_section_boundary()
            && !self.check(&TokenKind::RBrace)
        {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }

            let start = self.pos;
            while !self.at_eof()
                && !self.check(&TokenKind::Semicolon)
                && !self.check(&TokenKind::RBrace)
            {
                self.advance();
            }
            if self.at_eof() {
                return self.error_here("Expected Semicolon");
            }
            if self.check(&TokenKind::RBrace) {
                return self.error_here("Expected Semicolon");
            }

            let end = self.pos;
            self.expect_kind(TokenKind::Semicolon)?;

            if start == end {
                continue;
            }

            let atoms = self.tokens[start..end]
                .iter()
                .map(Self::primitive_directive_atom_from_token)
                .collect::<Result<Vec<_>, _>>()?;

            items.push(PrimitiveDirectiveItem { atoms });
        }

        Ok(items)
    }

    fn match_storage_qualifier(&mut self, name: &str) -> bool {
        if !self.check_word_ci(name)
            || self.check_next_kind(&TokenKind::Shift2)
            || self.check_next_kind(&TokenKind::Colon)
            || !matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Identifier(_))
            )
        {
            return false;
        }
        self.advance();
        true
    }

    fn parse_storage_contract_prefix(&mut self) -> Result<StorageContract, Diagnostic> {
        let mut contract = StorageContract::default();
        loop {
            if self.match_storage_qualifier("static") {
                contract.duration = StorageDuration::Static;
                continue;
            }
            if self.match_storage_qualifier("tls") || self.match_storage_qualifier("threadlocal") {
                contract.duration = StorageDuration::ThreadLocal;
                continue;
            }
            if self.match_storage_qualifier("percpu") {
                contract.duration = StorageDuration::PerCpu;
                continue;
            }
            if self.match_storage_qualifier("weak") {
                contract.linkage = StorageLinkage::Weak;
                continue;
            }
            if self.match_storage_qualifier("common") {
                contract.linkage = StorageLinkage::Common;
                continue;
            }
            if self.match_storage_qualifier("global") || self.match_storage_qualifier("extern") {
                contract.linkage = StorageLinkage::External;
                continue;
            }
            if self.match_storage_qualifier("data") {
                contract.section = StorageSection::Data;
                continue;
            }
            if self.match_storage_qualifier("bss") {
                contract.section = StorageSection::Bss;
                continue;
            }
            if self.match_storage_qualifier("rodata") || self.match_storage_qualifier("readonly") {
                if !self.explicit_rodata_lowering {
                    return self.error_here(
                        "E_LOWERING_COMPONENT_REQUIRED_001: explicit rodata placement is a lowering operation; use immutable storage for ordinary nC, or declare rodata.ncc; for direct compiler work",
                    );
                }
                contract.section = StorageSection::ReadOnlyData;
                continue;
            }
            if self.check_word_ci("align") && self.check_next_kind(&TokenKind::LParen) {
                self.advance();
                self.expect_kind(TokenKind::LParen)?;
                let value = self.match_number().ok_or_else(|| Diagnostic {
                    message: "E_STORAGE_ALIGN_001: align expects an integer byte alignment"
                        .to_string(),
                    line: self.current().line,
                    column: self.current().column,
                })?;
                let align = value
                    .replace('_', "")
                    .parse::<usize>()
                    .map_err(|_| Diagnostic {
                        message: "E_STORAGE_ALIGN_001: invalid byte alignment".to_string(),
                        line: self.current().line,
                        column: self.current().column,
                    })?;
                self.expect_kind(TokenKind::RParen)?;
                contract.align_bytes = Some(align);
                continue;
            }
            if self.check_word_ci("section") && self.check_next_kind(&TokenKind::LParen) {
                self.advance();
                self.expect_kind(TokenKind::LParen)?;
                let name = self.match_string().ok_or_else(|| Diagnostic {
                    message: "E_STORAGE_SECTION_001: section expects a quoted native section name"
                        .to_string(),
                    line: self.current().line,
                    column: self.current().column,
                })?;
                self.expect_kind(TokenKind::RParen)?;
                contract.custom_section = Some(name);
                continue;
            }
            if self.check_word_ci("comdat") && self.check_next_kind(&TokenKind::LParen) {
                self.advance();
                self.expect_kind(TokenKind::LParen)?;
                let group = self
                    .match_string()
                    .or_else(|| {
                        self.peek_is_ident()
                            .then(|| self.expect_ident().ok())
                            .flatten()
                    })
                    .ok_or_else(|| Diagnostic {
                        message: "E_STORAGE_COMDAT_001: comdat expects a group name".to_string(),
                        line: self.current().line,
                        column: self.current().column,
                    })?;
                self.expect_kind(TokenKind::RParen)?;
                contract.linkage = StorageLinkage::LinkOnceOdr;
                contract.comdat = Some(group);
                continue;
            }
            break;
        }
        Ok(contract)
    }

    fn parse_local_section(
        &mut self,
        current_context: &Option<Context>,
    ) -> Result<Section, Diagnostic> {
        let section_column = self.current().column;
        self.expect_kind(TokenKind::KeywordLocal)?;
        if matches!(
            current_context,
            Some(Context::Machine | Context::Primitive | Context::Kernel)
        ) {
            return self.error_here(
                "E_LOCAL_MODE_FORBIDDEN_001: local section is not allowed in machine, primitive or kernel contexts",
            );
        }
        self.expect_kind(TokenKind::Colon)?;

        let mut decls = Vec::new();
        while self.peek_starts_local_decl() {
            if self.current().column <= section_column && self.peek_starts_function() {
                break;
            }
            if self.current().column <= section_column
                && self.is_local_boundary_start()
                && !self.current_starts_local_decl_keyword()
            {
                break;
            }

            let visibility = self.parse_visibility_kw_opt();
            let storage = self.parse_storage_contract_prefix()?;
            if self.match_ident_name("const") {
                if !storage.is_default() {
                    return self.error_here(
                        "E_STORAGE_CONST_001: storage qualifiers apply to variables, not local compile-time constants",
                    );
                }
                let name = self.expect_ident()?;
                let (ty, value) = if self.match_kind(&TokenKind::Colon) {
                    let ty = self.expect_type_name()?;
                    if !(self.match_kind(&TokenKind::LessDash)
                        || self.match_kind(&TokenKind::Eq)
                        || self.match_kind(&TokenKind::Colon))
                    {
                        return self.error_here(
                            "E_LOCAL_CONST_EVAL_001: local const requires '=' or ':' initializer",
                        );
                    }
                    let value = self.parse_expr()?;
                    (ty, value)
                } else if self.match_kind(&TokenKind::Shift2) {
                    let ty = self.expect_type_name()?;
                    self.expect_projection_init_separator()?;
                    let value = self.parse_expr()?;
                    (ty, value)
                } else {
                    return self.error_here(
                        "E_LOCAL_CONST_EVAL_001: local const expects ':' or '>>' type syntax",
                    );
                };
                self.expect_kind(TokenKind::Semicolon)?;
                decls.push(LocalDecl::Const {
                    visibility,
                    name,
                    ty,
                    value,
                });
                continue;
            }

            if self.match_ident_name("metadata") {
                let name = self.expect_ident()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                decls.push(LocalDecl::Metadata {
                    visibility,
                    name,
                    value,
                });
                continue;
            }

            if self.match_ident_name("type") {
                let name = self.expect_ident()?;
                if self.match_kind(&TokenKind::Colon) {
                    let ty = self.expect_type_name()?;
                    self.expect_kind(TokenKind::Semicolon)?;
                    decls.push(LocalDecl::TypeAlias {
                        visibility,
                        name,
                        ty,
                    });
                    continue;
                }
                if self.match_kind(&TokenKind::LBrace) {
                    let fields = self.parse_local_type_record_fields()?;
                    decls.push(LocalDecl::TypeRecord {
                        visibility,
                        name,
                        fields,
                    });
                    continue;
                }
                return self.error_here(
                    "E_LOCAL_TYPE_UNKNOWN_001: local type expects ':' alias or record body",
                );
            }

            let name = self.expect_ident()?;
            let (ty, init) = if self.match_kind(&TokenKind::Colon) {
                let ty = self.expect_type_name()?;
                let init = if self.match_kind(&TokenKind::Eq) || self.match_kind(&TokenKind::Colon)
                {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                (ty, init)
            } else if self.match_kind(&TokenKind::Shift2) {
                let ty = self.expect_type_name()?;
                let init = if self.check(&TokenKind::Semicolon) {
                    None
                } else {
                    self.expect_projection_init_separator()?;
                    Some(self.parse_expr()?)
                };
                (ty, init)
            } else {
                return self.error_here(
                    "E_LOCAL_DECL_001: local declaration expects ':' or '>>' type syntax",
                );
            };
            self.expect_kind(TokenKind::Semicolon)?;
            let ty = attach_storage_contract(ty, storage);
            decls.push(LocalDecl::Var {
                visibility,
                name,
                ty,
                init,
            });
        }

        Ok(Section::Local(LocalSection { decls }))
    }

    fn parse_local_type_record_fields(&mut self) -> Result<Vec<LocalTypeField>, Diagnostic> {
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let visibility = self.parse_visibility_kw_opt();
            let name = self.expect_ident()?;
            if !(self.match_kind(&TokenKind::Colon) || self.match_kind(&TokenKind::Shift2)) {
                return self.error_here(
                    "E_LOCAL_TYPE_FIELD_001: local type field expects ':' or '>>' type syntax",
                );
            }
            let ty = self.expect_type_name()?;
            self.expect_kind(TokenKind::Semicolon)?;
            fields.push(LocalTypeField {
                visibility,
                name,
                ty,
            });
        }
        self.expect_kind(TokenKind::RBrace)?;
        Ok(fields)
    }

    fn parse_v2_storage_region(&mut self, region: &str) -> Result<Section, Diagnostic> {
        self.expect_kind(TokenKind::LBrace)?;
        let mut declarations = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let mut contract = self.parse_storage_contract_prefix()?;
            let is_const = self.match_ident_name("const");
            let name = self.expect_ident()?;
            if !(self.match_kind(&TokenKind::Shift2) || self.match_kind(&TokenKind::Colon)) {
                return self.error_here("E_MATERIAL_V2_STATE_001: state place expects '>>'");
            }
            let value_type = self.expect_type_name()?;
            let init = if self.match_kind(&TokenKind::LessDash)
                || self.match_kind(&TokenKind::Eq)
                || self.match_kind(&TokenKind::Colon)
            {
                Some(self.parse_expr()?)
            } else {
                None
            };
            if is_const && init.is_none() {
                return self.error_here(
                    "E_MATERIAL_V2_STATE_CONST_001: material const expects '<-' initializer",
                );
            }
            self.expect_kind(TokenKind::Semicolon)?;

            if is_const {
                contract.section = StorageSection::ReadOnlyData;
            } else if contract.section == StorageSection::Automatic {
                contract.section = if init.is_some() {
                    StorageSection::Data
                } else {
                    StorageSection::Bss
                };
            }
            if contract.custom_section.is_none() {
                contract.custom_section = Some(format!(".nc.{region}"));
            }
            if contract.linkage == StorageLinkage::Default {
                contract.linkage = StorageLinkage::Internal;
            }
            declarations.push(StcDecl::Typed {
                visibility: None,
                strictly: false,
                name,
                ty: attach_explicit_storage_contract(value_type, contract),
                init,
            });
        }
        self.expect_kind(TokenKind::RBrace)?;
        Ok(Section::Stc(declarations))
    }

    fn parse_v2_stack_region(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect_kind(TokenKind::LBrace)?;
        let mut declarations = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let is_const = self.match_ident_name("const");
            let name = self.expect_ident()?;
            if !(self.match_kind(&TokenKind::Shift2) || self.match_kind(&TokenKind::Colon)) {
                return self.error_here("E_MATERIAL_V2_STACK_001: stack place expects '>>'");
            }
            let ty = self.expect_type_name()?;
            let value = if self.match_kind(&TokenKind::LessDash)
                || self.match_kind(&TokenKind::Eq)
                || self.match_kind(&TokenKind::Colon)
            {
                self.parse_expr()?
            } else if is_const {
                return self.error_here(
                    "E_MATERIAL_V2_STACK_CONST_001: stack const expects '<-' initializer",
                );
            } else {
                Expr::Call {
                    callee: "__nC_uninit".to_string(),
                    args: Vec::new(),
                }
            };
            self.expect_kind(TokenKind::Semicolon)?;
            declarations.push(Stmt::Projection {
                target: AssignTarget::Context(ContextRef::plain(vec![name])),
                ty,
                value,
            });
        }
        self.expect_kind(TokenKind::RBrace)?;
        Ok(declarations)
    }

    fn peek_v2_manual_abi_decl(&self) -> bool {
        matches!(
            (
                self.tokens.get(self.pos).map(|token| &token.kind),
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                self.tokens.get(self.pos + 2).map(|token| &token.kind),
                self.tokens.get(self.pos + 3).map(|token| &token.kind),
            ),
            (
                Some(TokenKind::Identifier(_)),
                Some(TokenKind::Colon),
                Some(TokenKind::Colon),
                Some(TokenKind::Identifier(role))
            ) if role.eq_ignore_ascii_case("abi")
        )
    }

    fn parse_v2_abi_register_list(&mut self) -> Result<Vec<String>, Diagnostic> {
        self.expect_kind(TokenKind::LBracket)?;
        let mut registers = Vec::new();
        while !self.check(&TokenKind::RBracket) {
            registers.push(self.expect_ident()?);
            if self.match_kind(&TokenKind::Comma) {
                continue;
            }
            break;
        }
        self.expect_kind(TokenKind::RBracket)?;
        Ok(registers)
    }

    fn parse_v2_abi_usize(&mut self, field: &str) -> Result<usize, Diagnostic> {
        let Some(raw) = self.match_number() else {
            return self.error_here(format!("E_V2_ABI_NUMBER_001: '{field}' expects an integer"));
        };
        raw.replace('_', "").parse::<usize>().map_err(|_| {
            Diagnostic::new(
                format!("E_V2_ABI_NUMBER_001: invalid integer '{raw}' for '{field}'"),
                self.current().line,
                self.current().column,
            )
        })
    }

    fn v2_abi_required<T>(&self, value: Option<T>, field: &str) -> Result<T, Diagnostic> {
        value.ok_or_else(|| {
            Diagnostic::new(
                format!("E_V2_ABI_REQUIRED_001: manual ABI requires '{field}'"),
                self.current().line,
                self.current().column,
            )
        })
    }

    fn parse_v2_manual_abi_decl(&mut self) -> Result<Section, Diagnostic> {
        let name = self.expect_ident()?;
        self.expect_double_colon()?;
        if !self.match_ident_name("abi") {
            return self.error_here("E_V2_ABI_HEADER_001: expected ':: abi'");
        }
        self.expect_kind(TokenKind::LBrace)?;

        let mut target = None;
        let mut pointer_bits = None;
        let mut stack_align_bytes = None;
        let mut shadow_space_bytes = None;
        let mut red_zone_bytes = None;
        let mut integer_argument_registers = None;
        let mut float_argument_registers = None;
        let mut integer_return_registers = None;
        let mut float_return_registers = None;
        let mut callee_saved_registers = None;
        let mut clobbered_registers = None;
        let mut sret_register = None;

        while !self.check(&TokenKind::RBrace) {
            let mut field = self.expect_ident()?;
            while self.match_kind(&TokenKind::Dot) {
                field.push('.');
                field.push_str(&self.expect_ident()?);
            }
            self.expect_kind(TokenKind::LessDash)?;
            match field.as_str() {
                "target" => {
                    target = self.match_string();
                    if target.is_none() {
                        return self
                            .error_here("E_V2_ABI_TARGET_001: target expects a triple string");
                    }
                }
                "pointer.bits" => pointer_bits = Some(self.parse_v2_abi_usize(&field)? as u16),
                "stack.align" => stack_align_bytes = Some(self.parse_v2_abi_usize(&field)?),
                "stack.shadow" => shadow_space_bytes = Some(self.parse_v2_abi_usize(&field)?),
                "stack.redzone" => red_zone_bytes = Some(self.parse_v2_abi_usize(&field)?),
                "args.int" => integer_argument_registers = Some(self.parse_v2_abi_register_list()?),
                "args.float" => float_argument_registers = Some(self.parse_v2_abi_register_list()?),
                "returns.int" => {
                    integer_return_registers = Some(self.parse_v2_abi_register_list()?)
                }
                "returns.float" => {
                    float_return_registers = Some(self.parse_v2_abi_register_list()?)
                }
                "preserve" => callee_saved_registers = Some(self.parse_v2_abi_register_list()?),
                "clobber" => clobbered_registers = Some(self.parse_v2_abi_register_list()?),
                "sret" => {
                    let register = self.expect_ident()?;
                    sret_register = if register.eq_ignore_ascii_case("none") {
                        Some(None)
                    } else {
                        Some(Some(register))
                    };
                }
                _ => {
                    return self.error_here(format!(
                        "E_V2_ABI_FIELD_001: unknown manual ABI field '{field}'"
                    ))
                }
            }
            self.expect_kind(TokenKind::Semicolon)?;
        }
        self.expect_kind(TokenKind::RBrace)?;

        let declaration = ManualAbiDecl {
            name,
            target: self.v2_abi_required(target, "target")?,
            pointer_bits: self.v2_abi_required(pointer_bits, "pointer.bits")?,
            stack_align_bytes: self.v2_abi_required(stack_align_bytes, "stack.align")?,
            shadow_space_bytes: self.v2_abi_required(shadow_space_bytes, "stack.shadow")?,
            red_zone_bytes: self.v2_abi_required(red_zone_bytes, "stack.redzone")?,
            integer_argument_registers: self
                .v2_abi_required(integer_argument_registers, "args.int")?,
            float_argument_registers: self
                .v2_abi_required(float_argument_registers, "args.float")?,
            integer_return_registers: self
                .v2_abi_required(integer_return_registers, "returns.int")?,
            float_return_registers: self
                .v2_abi_required(float_return_registers, "returns.float")?,
            callee_saved_registers: self.v2_abi_required(callee_saved_registers, "preserve")?,
            clobbered_registers: self.v2_abi_required(clobbered_registers, "clobber")?,
            sret_register: self.v2_abi_required(sret_register, "sret")?,
        };
        Ok(Section::ManualAbi(vec![ManualAbiItem::Contract(
            declaration,
        )]))
    }

    fn parse_v2_manual_abi_binding(&mut self) -> Result<Section, Diagnostic> {
        self.expect_kind(TokenKind::KeywordBind)?;
        let symbol = self.parse_v2_abi_path()?;
        self.expect_kind(TokenKind::LessDash)?;
        let function = self.parse_v2_abi_path()?;
        if !self.match_ident_name("using") {
            return self.error_here("E_V2_ABI_BINDING_001: binding requires 'using ABI'");
        }
        let abi = self.expect_ident()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Section::ManualAbi(vec![ManualAbiItem::Binding(
            ManualAbiBinding {
                symbol,
                function,
                abi,
            },
        )]))
    }

    fn parse_v2_abi_path(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut path = vec![self.expect_v2_abi_path_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            path.push(self.expect_v2_abi_path_segment()?);
        }
        Ok(path)
    }

    fn expect_v2_abi_path_segment(&mut self) -> Result<String, Diagnostic> {
        if self.match_kind(&TokenKind::KeywordMain) {
            return Ok("main".to_string());
        }
        self.expect_ref_segment()
    }

    fn parse_section(&mut self, current_context: &Option<Context>) -> Result<Section, Diagnostic> {
        if self.peek_v2_manual_abi_decl() {
            return self.parse_v2_manual_abi_decl();
        }

        if self.check(&TokenKind::KeywordBind)
            && !self.peek_starts_function()
            && !matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Colon)
            )
        {
            return self.parse_v2_manual_abi_binding();
        }

        if !matches!(current_context, Some(Context::Primitive))
            && self.check_ident_name("static")
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::LBrace)
            )
        {
            self.expect_ident()?;
            return self.parse_v2_storage_region("static");
        }

        if self.check(&TokenKind::KeywordLocal) {
            return self.parse_local_section(current_context);
        }

        if matches!(current_context, Some(Context::Primitive)) {
            if self.check(&TokenKind::KeywordAnchor) {
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                ) {
                    return self.parse_primitive_symbol_list_section(
                        TokenKind::KeywordAnchor,
                        Section::Anchor,
                    );
                }
                return self.parse_primitive_anchor_block_section();
            }
            if self.check(&TokenKind::KeywordSlot) {
                return self
                    .parse_primitive_symbol_list_section(TokenKind::KeywordSlot, Section::Slot);
            }
            if self.check(&TokenKind::KeywordPattern) {
                return self.parse_primitive_symbol_list_section(
                    TokenKind::KeywordPattern,
                    Section::Pattern,
                );
            }
            if self.check(&TokenKind::KeywordMask) {
                return self
                    .parse_primitive_symbol_list_section(TokenKind::KeywordMask, Section::Mask);
            }
            if self.check(&TokenKind::KeywordWindow) {
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                ) {
                    return self.parse_primitive_symbol_list_section(
                        TokenKind::KeywordWindow,
                        Section::Window,
                    );
                }
                return self.parse_primitive_window_block_section();
            }
            if self.check(&TokenKind::KeywordSignal) {
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                ) {
                    return self.parse_primitive_symbol_list_section(
                        TokenKind::KeywordSignal,
                        Section::Signal,
                    );
                }
                return self.parse_primitive_signal_block_section();
            }
            if self.check(&TokenKind::KeywordBus) {
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                ) {
                    return self
                        .parse_primitive_symbol_list_section(TokenKind::KeywordBus, Section::Bus);
                }
                return self.parse_primitive_bus_block_section();
            }
            if self.check(&TokenKind::KeywordState) {
                return self
                    .parse_primitive_symbol_list_section(TokenKind::KeywordState, Section::State);
            }
            if self.check(&TokenKind::KeywordTiming) {
                return self.parse_primitive_symbol_list_section(
                    TokenKind::KeywordTiming,
                    Section::Timing,
                );
            }
            if self.check(&TokenKind::KeywordEmit) {
                return self
                    .parse_primitive_symbol_list_section(TokenKind::KeywordEmit, Section::Emit);
            }
            if self.check(&TokenKind::KeywordSample) {
                return self.parse_primitive_symbol_list_section(
                    TokenKind::KeywordSample,
                    Section::Sample,
                );
            }
            if self.check(&TokenKind::KeywordLatch) {
                return self
                    .parse_primitive_symbol_list_section(TokenKind::KeywordLatch, Section::Latch);
            }
            if self.check(&TokenKind::KeywordAssert) {
                return self.parse_primitive_symbol_list_section(
                    TokenKind::KeywordAssert,
                    Section::Assert,
                );
            }
            if self.check(&TokenKind::KeywordRupture) {
                return self.parse_primitive_symbol_list_section(
                    TokenKind::KeywordRupture,
                    Section::Rupture,
                );
            }
            if self.check(&TokenKind::KeywordBridge) {
                return self.parse_primitive_bridge_section();
            }
            if self.check(&TokenKind::KeywordSeal) {
                return self
                    .parse_primitive_symbol_list_section(TokenKind::KeywordSeal, Section::Seal);
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "capability") {
                return self.parse_primitive_capability_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "bitfield") {
                return self.parse_primitive_bitfield_section();
            }
            if self.is_primitive_type_alias_section_start() {
                return self.parse_primitive_type_alias_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "operation")
                && matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                )
            {
                return self.parse_primitive_operation_compat_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "operation") {
                return self.parse_primitive_operation_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "port")
                && !matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                )
            {
                return self.parse_primitive_port_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "signature") {
                return self.parse_primitive_signature_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "invariant") {
                return self.parse_primitive_invariant_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "constraint") {
                return self.parse_primitive_constraint_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "entry") {
                return self.parse_primitive_entry_section();
            }
            if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "target")
                && matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                )
            {
                self.expect_ident()?;
                self.expect_kind(TokenKind::Colon)?;
                let mut arch: Option<String> = None;

                while !self.is_section_start() && !self.check(&TokenKind::RBrace) {
                    if self.match_ident_name("arch") {
                        self.expect_kind(TokenKind::Colon)?;
                        arch = Some(self.expect_ident()?);
                        self.expect_kind(TokenKind::Semicolon)?;
                        continue;
                    }

                    while !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
                        self.advance();
                    }

                    if self.check(&TokenKind::RBrace) {
                        return self.error_here("Expected Semicolon");
                    }
                    self.expect_kind(TokenKind::Semicolon)?;
                }

                let Some(arch) = arch else {
                    return self.error_here("primitive target section requires arch");
                };
                return Ok(Section::PrimitiveTarget(arch));
            }
            if self.check(&TokenKind::KeywordReg)
                && matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                )
            {
                return self.parse_primitive_directive_section(PrimitiveDirectiveKind::Register);
            }
            if let TokenKind::Identifier(name) = &self.current().kind {
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                ) {
                    if let Some(kind) = Self::primitive_directive_kind_from_ident(name) {
                        return self.parse_primitive_directive_section(kind);
                    }
                }
            }
            if self.is_identifier_colon_section_start() {
                return self.error_here("section not allowed in [@primitive] context");
            }

            if self.check(&TokenKind::KeywordFn)
                || self.check(&TokenKind::KeywordStc)
                || self.check(&TokenKind::KeywordLocal)
                || self.check(&TokenKind::KeywordBinary)
                || self.check(&TokenKind::KeywordBind)
                || self.check(&TokenKind::KeywordRaw)
                || self.check(&TokenKind::KeywordProbe)
                || self.check(&TokenKind::KeywordMove)
                || self.check(&TokenKind::KeywordStructure)
                || self.check(&TokenKind::KeywordStr)
                || self.check(&TokenKind::KeywordFinal)
                || self.check(&TokenKind::KeywordFa)
            {
                return self.error_here("section not allowed in [@primitive] context");
            }
        }

        // A canonical function name may intentionally coincide with a primitive
        // directive word (for example `hazard :: fn`). The `:: fn` shape is
        // unambiguous and must win before primitive-section diagnostics.
        if !matches!(current_context, Some(Context::Primitive)) && self.peek_starts_function() {
            return Ok(Section::Fn(self.parse_function_group()?));
        }

        if self.match_kind(&TokenKind::KeywordAnchor)
            || self.match_kind(&TokenKind::KeywordSlot)
            || self.match_kind(&TokenKind::KeywordPattern)
            || self.match_kind(&TokenKind::KeywordMask)
            || self.match_kind(&TokenKind::KeywordWindow)
            || self.match_kind(&TokenKind::KeywordSignal)
            || self.match_kind(&TokenKind::KeywordBus)
            || self.match_kind(&TokenKind::KeywordState)
            || self.match_kind(&TokenKind::KeywordTiming)
            || self.match_kind(&TokenKind::KeywordEmit)
            || self.match_kind(&TokenKind::KeywordSample)
            || self.match_kind(&TokenKind::KeywordLatch)
            || self.match_kind(&TokenKind::KeywordAssert)
            || self.match_kind(&TokenKind::KeywordRupture)
            || self.match_kind(&TokenKind::KeywordBridge)
            || self.match_kind(&TokenKind::KeywordSeal)
        {
            return self.error_here("section not allowed outside [@primitive] context");
        }
        if self.is_primitive_named_section_start() {
            return self.error_here("section not allowed outside [@primitive] context");
        }
        if self.is_primitive_type_alias_section_start() {
            return self.error_here("section not allowed outside [@primitive] context");
        }
        if self.is_primitive_identifier_directive_section_start()
            && !self.is_type_contract_section_start()
            && !matches!(&self.current().kind, TokenKind::Identifier(name)
                if matches!(name.as_str(), "module" | "use" | "export"))
        {
            return self.error_here("section not allowed outside [@primitive] context");
        }
        if self.is_security_identifier_section_start()
            && !matches!(current_context, Some(Context::Security))
            && !(self.check_word_ci("risk")
                || self.check_word_ci("policy")
                || self.check_word_ci("authority"))
        {
            return self.error_here("section not allowed outside [@security] context");
        }

        if self.check_ident_name("module")
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            )
        {
            let decl = self.parse_module_section()?;
            return Ok(Section::ModuleDecl(decl));
        }

        if (self.check_ident_name("use") || self.check(&TokenKind::KeywordUse))
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            )
        {
            if matches!(
                current_context,
                Some(Context::Machine | Context::Primitive | Context::Kernel)
            ) {
                return self.error_here(use_forbidden_in_mode_diagnostic(current_context));
            }
            let decls = if matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            ) {
                self.advance();
                self.parse_use_decls_after_keyword()?
            } else {
                self.parse_use_section()?
            };
            self.note_lowering_components(&decls);
            return Ok(Section::Use(decls));
        }

        if self.check_ident_name("export")
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            )
        {
            let decls = self.parse_export_section()?;
            return Ok(Section::Exports(decls));
        }

        if self.check_ident_name("interface") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self.error_here(
                    "E_HIGH_INTERFACE_PARSE_001: interface section not allowed in this context",
                );
            }
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            ) {
                let decls = self.parse_interface_section()?;
                return Ok(Section::Interfaces(decls));
            }
            let decl = self.parse_interface_decl()?;
            return Ok(Section::Interfaces(vec![decl]));
        }

        if self.check_ident_name("trait") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self.error_here(
                    "E_HIGH_TRAIT_PARSE_001: trait section not allowed in this context",
                );
            }
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            ) {
                let decls = self.parse_trait_section()?;
                return Ok(Section::Traits(decls));
            }
            let decl = self.parse_trait_decl()?;
            return Ok(Section::Traits(vec![decl]));
        }

        if self.check_ident_name("impl") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self
                    .error_here("E_HIGH_IMPL_PARSE_001: impl section not allowed in this context");
            }
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            ) {
                let decls = self.parse_impl_section()?;
                return Ok(Section::ImplBlocks(decls));
            }
            let decl = self.parse_impl_decl()?;
            return Ok(Section::ImplBlocks(vec![decl]));
        }

        if self.check_ident_name("contract") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self.error_here(
                    "E_HIGH_CONTRACT_PARSE_001: contract section not allowed in this context",
                );
            }
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            ) {
                let decls = self.parse_contract_section()?;
                return Ok(Section::Contracts(decls));
            }
            let decl = self.parse_contract_decl()?;
            return Ok(Section::Contracts(vec![decl]));
        }

        if self.check_ident_name("enum") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self
                    .error_here("E_HIGH_ENUM_PARSE_001: enum section not allowed in this context");
            }
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Colon)
            ) {
                let decls = self.parse_enum_section()?;
                return Ok(Section::Enums(decls));
            }
            let decl = self.parse_enum_decl()?;
            return Ok(Section::Enums(vec![decl]));
        }

        if self.check_ident_name("const") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self.error_here(
                    "E_HIGH_CONST_EVAL_FORBIDDEN_EFFECT_001: const eval section not allowed in this context",
                );
            }
            self.expect_ident()?;
            if self.match_kind(&TokenKind::KeywordFn) {
                self.expect_kind(TokenKind::Colon)?;
                let funcs = self.parse_function_group()?;
                if funcs.is_empty() {
                    return self.error_here("const fn section requires at least one function");
                }
                return Ok(Section::ConstFn(funcs));
            }
            self.expect_kind(TokenKind::Colon)?;
            let decls = self.parse_const_eval_decls()?;
            if decls.is_empty() {
                return self.error_here("const section requires at least one declaration");
            }
            return Ok(Section::ConstEval(decls));
        }

        if self.check_ident_name("comptime") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self.error_here(
                    "E_HIGH_COMPTIME_UNDECLARED_EFFECT_001: comptime section not allowed in this context",
                );
            }
            self.expect_ident()?;
            if self.match_ident_name("unsafe") {
                let no_limit = self.match_ident_name("noLimit")
                    || self.match_ident_name("nolimit")
                    || self.match_ident_name("no_limit");
                self.expect_kind(TokenKind::Colon)?;
                let blocks = self.parse_comptime_unsafe_blocks(no_limit)?;
                if blocks.is_empty() {
                    return self.error_here("comptime unsafe section requires at least one block");
                }
                return Ok(Section::ComptimeUnsafe(blocks));
            }
            self.expect_kind(TokenKind::Colon)?;
            let decls = self.parse_const_eval_decls()?;
            if decls.is_empty() {
                return self.error_here("comptime section requires at least one declaration");
            }
            return Ok(Section::Comptime(decls));
        }

        if self.check_ident_name("macro") {
            if matches!(current_context, Some(Context::Machine | Context::Primitive)) {
                return self.error_here(
                    "E_HIGH_MACRO_PARSE_001: macro section not allowed in this context",
                );
            }
            self.expect_ident()?;
            self.expect_kind(TokenKind::Colon)?;
            let items = self.parse_macro_section()?;
            if items.is_empty() {
                return self.error_here("E_HIGH_MACRO_PARSE_001: macro section requires items");
            }
            return Ok(Section::Macros(items));
        }

        if (self.check_word_ci("risk")
            || self.check_word_ci("policy")
            || self.check_word_ci("authority"))
            && !(matches!(current_context, Some(Context::Security)) && self.check_word_ci("policy"))
        {
            if matches!(
                current_context,
                Some(
                    Context::Machine
                        | Context::Primitive
                        | Context::System
                        | Context::Kernel
                        | Context::Root
                )
            ) {
                return self.error_here(
                    "E_HIGH_RISK_POLICY_001: risk authority section is high-level only",
                );
            }
            self.advance();
            self.expect_kind(TokenKind::Colon)?;
            let items = self.parse_high_risk_section()?;
            if items.is_empty() {
                return self.error_here("E_HIGH_RISK_POLICY_001: risk section requires items");
            }
            return Ok(Section::Risk(items));
        }

        if self.peek_starts_canonical_type_decl() {
            return Ok(Section::TypeContracts(vec![
                self.parse_canonical_type_contract_decl()?
            ]));
        }

        // Official compact container form. `fn:` remains fully supported, but a
        // function declaration is already self-delimiting and needs no wrapper.
        if self.peek_starts_function() {
            let funcs = self.parse_function_group()?;
            return Ok(Section::Fn(funcs));
        }

        if self.check(&TokenKind::KeywordStc) {
            let section_column = self.current().column;
            self.expect_kind(TokenKind::KeywordStc)?;
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let mut decls = Vec::new();
            while self.peek_starts_stc_decl() {
                if self.current().column <= section_column
                    && self.peek_starts_function()
                    && !self.check_visibility_block_start()
                {
                    break;
                }
                if self.current().column <= section_column
                    && self.is_stc_boundary_start()
                    && !matches!(
                        self.current().kind,
                        TokenKind::KeywordPublic
                            | TokenKind::KeywordPrivate
                            | TokenKind::KeywordInternal
                            | TokenKind::KeywordStrictly
                            | TokenKind::KeywordState
                            | TokenKind::KeywordLabel
                    )
                    && !(matches!(self.current().kind, TokenKind::KeywordRaw)
                        && ncc_stage0_bridge::raw_declaration(match self.tokens
                            .get(self.pos + 1).map(|token| &token.kind) {
                                Some(TokenKind::Shift2) => 21,
                                Some(TokenKind::Colon) => 13,
                                _ => 0,
                            }))
                {
                    break;
                }

                if matches!(current_context, Some(Context::Security))
                    && self.is_security_identifier_section_start()
                {
                    break;
                }

                let visibility = self.parse_visibility_kw_opt();
                let strictly = if matches!(visibility, Some(Visibility::Strictly)) {
                    true
                } else {
                    self.match_kind(&TokenKind::KeywordStrictly)
                };
                if strictly {
                    let _ = self.match_kind(&TokenKind::Colon);
                }
                let immutable = self.match_ident_name("immutable");
                let mut storage = self.parse_storage_contract_prefix()?;
                if immutable {
                    if storage.section != StorageSection::Automatic {
                        return self.error_here(
                            "E_IMMUTABLE_STORAGE_SECTION_001: immutable storage chooses its physical read-only section during lowering; remove data, bss, rodata or readonly",
                        );
                    }
                    storage.section = StorageSection::ReadOnlyData;
                }
                let name = self.expect_ident()?;
                let (ty, init) = if self.match_kind(&TokenKind::Colon) {
                    let ty = self.expect_type_name()?;
                    (ty, None)
                } else if self.match_kind(&TokenKind::Shift2) {
                    let ty = self.expect_type_name()?;
                    let init = if self.check(&TokenKind::Semicolon) {
                        None
                    } else {
                        self.expect_projection_init_separator()?;
                        Some(self.parse_expr()?)
                    };
                    (ty, init)
                } else {
                    return self.error_here("Expected ':' or '>>' in stc declaration");
                };
                if immutable && init.is_none() {
                    return self.error_here(
                        "E_IMMUTABLE_STORAGE_INIT_001: immutable storage requires an initializer",
                    );
                }
                self.expect_kind(TokenKind::Semicolon)?;
                let ty = attach_storage_contract(ty, storage);
                decls.push(StcDecl::Typed {
                    visibility,
                    strictly,
                    name,
                    ty,
                    init,
                });
            }
            return Ok(Section::Stc(decls));
        }

        if self.match_kind(&TokenKind::KeywordFn) {
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let funcs = self.parse_function_group()?;
            return Ok(Section::Fn(funcs));
        }

        if self.match_kind(&TokenKind::KeywordMove) {
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Inspect)) {
                return self.error_here("section not allowed in [@inspect] context");
            }
            if matches!(current_context, Some(Context::Security)) {
                return self.error_here("section not allowed in [@security] context");
            }
            if matches!(current_context, Some(Context::Network)) {
                return self.error_here("section not allowed in [@network] context");
            }
            if let Some(label) = f10_context_label(current_context) {
                return self.error_here(format!("section not allowed in [@{}] context", label));
            }
            if matches!(current_context, Some(Context::Root)) {
                return self.error_here("move fn section is not allowed in [@] root context");
            }
            if self.match_kind(&TokenKind::KeywordFn) {
                self.expect_kind(TokenKind::Colon)?;
                let funcs = self.parse_function_group()?;
                return Ok(Section::MoveFn(funcs));
            } else {
                return self.error_here("Expected 'fn' after 'move'");
            }
        }

        if self.match_kind(&TokenKind::KeywordProbe) {
            let section_column = self.tokens[self.pos - 1].column;
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            // Compatibility: allow declarative probe functions and treat them as
            // regular container functions (execution remains explicit via calls).
            if self.peek_starts_function() && self.peek_function_header_has_arrow() {
                let funcs = self.parse_function_group()?;
                return Ok(Section::Fn(funcs));
            }
            let mut stmts = Vec::new();
            while !self.is_section_start_at_column_or_left(section_column)
                && !self.check(&TokenKind::RBrace)
            {
                stmts.push(self.parse_stmt()?);
            }
            return Ok(Section::Probe(stmts));
        }

        if self.match_kind(&TokenKind::KeywordBinary) {
            let section_column = self.tokens[self.pos - 1].column;
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Inspect)) {
                return self.error_here("section not allowed in [@inspect] context");
            }
            if matches!(current_context, Some(Context::Security)) {
                return self.error_here("section not allowed in [@security] context");
            }
            if matches!(current_context, Some(Context::Network)) {
                return self.error_here("section not allowed in [@network] context");
            }
            if let Some(label) = f10_context_label(current_context) {
                return self.error_here(format!("section not allowed in [@{}] context", label));
            }
            self.expect_kind(TokenKind::Colon)?;
            let mut stmts = Vec::new();
            while !self.is_section_start_at_column_or_left(section_column)
                && !self.check(&TokenKind::RBrace)
            {
                stmts.push(self.parse_stmt()?);
            }
            return Ok(Section::Binary(stmts));
        }

        if self.match_kind(&TokenKind::KeywordBind) {
            let section_column = self.tokens[self.pos - 1].column;
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Inspect)) {
                return self.error_here("section not allowed in [@inspect] context");
            }
            if matches!(current_context, Some(Context::Security)) {
                return self.error_here("section not allowed in [@security] context");
            }
            if matches!(current_context, Some(Context::Network)) {
                return self.error_here("section not allowed in [@network] context");
            }
            if let Some(label) = f10_context_label(current_context) {
                return self.error_here(format!("section not allowed in [@{}] context", label));
            }
            self.expect_kind(TokenKind::Colon)?;
            let mut stmts = Vec::new();
            while !self.is_section_start_at_column_or_left(section_column)
                && !self.check(&TokenKind::RBrace)
            {
                stmts.push(self.parse_stmt()?);
            }
            return Ok(Section::Bind(stmts));
        }

        if self.match_kind(&TokenKind::KeywordRaw) {
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Inspect)) {
                return self.error_here("section not allowed in [@inspect] context");
            }
            if matches!(current_context, Some(Context::Security)) {
                return self.error_here("section not allowed in [@security] context");
            }
            if matches!(current_context, Some(Context::Network)) {
                return self.error_here("section not allowed in [@network] context");
            }
            if let Some(label) = f10_context_label(current_context) {
                return self.error_here(format!("section not allowed in [@{}] context", label));
            }
            let stmts = if self.match_kind(&TokenKind::Colon) {
                let section_column = self.tokens[self.pos - 2].column;
                let mut section_stmts = Vec::new();
                while !self.is_section_start_at_column_or_left(section_column)
                    && !self.check(&TokenKind::RBrace)
                {
                    section_stmts.push(self.parse_stmt()?);
                }
                section_stmts
            } else {
                self.parse_block_stmts()?
            };
            return Ok(Section::Raw(stmts));
        }

        if self.match_ident_name("type") && matches!(self.current().kind, TokenKind::Colon) {
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Inspect)) {
                return self.error_here("section not allowed in [@inspect] context");
            }
            if matches!(current_context, Some(Context::Security)) {
                return self.error_here("section not allowed in [@security] context");
            }
            if matches!(current_context, Some(Context::Network)) {
                return self.error_here("section not allowed in [@network] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let mut contracts = Vec::new();
            while self.peek_starts_type_contract_decl() {
                contracts.push(self.parse_type_contract_decl()?);
            }
            if contracts.is_empty() {
                return self.error_here("type section requires at least one type contract");
            }
            return Ok(Section::TypeContracts(contracts));
        }

        if self.match_ident_name("foreign") {
            if matches!(current_context, Some(Context::Primitive)) {
                return self.error_here("section not allowed in [@primitive] context");
            }
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Kernel)) {
                return self.error_here("section not allowed in [@kernel] context");
            }
            return self.parse_foreign_section();
        }

        if self.match_kind(&TokenKind::KeywordStr) || self.match_kind(&TokenKind::KeywordStructure)
        {
            let section_column = self.tokens[self.pos - 1].column;
            if matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed in [@machine] context");
            }
            if matches!(current_context, Some(Context::Inspect)) {
                return self.error_here("section not allowed in [@inspect] context");
            }
            if matches!(current_context, Some(Context::Security)) {
                return self.error_here("section not allowed in [@security] context");
            }
            if matches!(current_context, Some(Context::Network)) {
                return self.error_here("section not allowed in [@network] context");
            }
            if let Some(label) = f10_context_label(current_context) {
                return self.error_here(format!("section not allowed in [@{}] context", label));
            }
            self.expect_kind(TokenKind::Colon)?;
            let mut stmts = Vec::new();
            while !self.is_section_start_at_column_or_left(section_column)
                && !self.check(&TokenKind::RBrace)
            {
                stmts.push(self.parse_stmt()?);
            }
            return Ok(Section::Structure(stmts));
        }

        if self.check(&TokenKind::KeywordFa) && matches!(current_context, Some(Context::Machine)) {
            return self.error_here(
                "fa section is forbidden in [@machine] context; use ret/hlt/jmp with verify/seal",
            );
        }

        if self.match_kind(&TokenKind::KeywordFa) || self.match_kind(&TokenKind::KeywordFinal) {
            let section_column = self.tokens[self.pos - 1].column;
            if matches!(current_context, Some(Context::Machine)) {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_final_section()?;
                return Ok(Section::MachineFinal(items));
            }

            self.expect_kind(TokenKind::Colon)?;
            let mut stmts = Vec::new();
            while !self.is_section_start_at_column_or_left(section_column)
                && !self.check(&TokenKind::RBrace)
            {
                stmts.push(self.parse_stmt()?);
            }
            return Ok(Section::Final(stmts));
        }

        if self.match_kind(&TokenKind::KeywordReg) {
            if !matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed outside [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let items = self.parse_machine_atoms_section()?;
            return Ok(Section::Reg(items));
        }

        if self.match_kind(&TokenKind::KeywordFlag) {
            if !matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed outside [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let items = self.parse_machine_atoms_section()?;
            return Ok(Section::Flag(items));
        }

        if self.match_kind(&TokenKind::KeywordLabel) {
            if !matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed outside [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let items = self.parse_machine_atoms_section()?;
            return Ok(Section::Label(items));
        }

        if self.match_kind(&TokenKind::KeywordInst) {
            if !matches!(current_context, Some(Context::Machine)) {
                return self.error_here("section not allowed outside [@machine] context");
            }
            self.expect_kind(TokenKind::Colon)?;
            let items = self.parse_machine_inst_section()?;
            return Ok(Section::Inst(items));
        }

        // Compact entry alias for `ref main ->> @fn(path);`.
        if self.match_kind(&TokenKind::KeywordMain) {
            self.expect_kind(TokenKind::Colon)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            if self.check(&TokenKind::LBrace) {
                return self.error_here("main: cannot declare an inline body");
            }
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Section::MainEntry {
                kind: MainEntryKind::RefFn(fn_ref),
                body: Vec::new(),
            });
        }

        if self.match_ident_name("to") {
            self.expect_kind(TokenKind::KeywordMain)?;
            let kind = if self.check(&TokenKind::At) {
                MainEntryKind::ToMainProfile(self.parse_main_profile()?)
            } else {
                MainEntryKind::ToMain
            };
            let body = self.parse_block_stmts()?;
            return Ok(Section::MainEntry { kind, body });
        }

        if self.match_kind(&TokenKind::KeywordRef) {
            self.expect_kind(TokenKind::KeywordMain)?;
            self.expect_kind(TokenKind::Arrow2)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            if self.check(&TokenKind::LBrace) {
                return self.error_here("ref main cannot declare an inline body");
            }
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Section::MainEntry {
                kind: MainEntryKind::RefFn(fn_ref),
                body: Vec::new(),
            });
        }

        if self.match_kind(&TokenKind::KeywordSkkMain) {
            self.expect_kind(TokenKind::Arrow2)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            if self.check(&TokenKind::LBrace) {
                return self.error_here("skk main cannot declare an inline body");
            }
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Section::MainEntry {
                kind: MainEntryKind::SkkFn(fn_ref),
                body: Vec::new(),
            });
        }

        // Compatibility alias:
        //   skk main ->> @fn(...)
        if self.match_ident_name("skk") {
            self.expect_kind(TokenKind::KeywordMain)?;
            self.expect_kind(TokenKind::Arrow2)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            if self.check(&TokenKind::LBrace) {
                return self.error_here("skk main cannot declare an inline body");
            }
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Section::MainEntry {
                kind: MainEntryKind::SkkFn(fn_ref),
                body: Vec::new(),
            });
        }

        if matches!(current_context, Some(Context::Security)) {
            if self.match_ident_name("asset") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecurityAsset(items));
            }

            if self.match_ident_name("identity") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecurityIdentity(items));
            }

            if self.match_ident_name("secret") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecuritySecret(items));
            }

            if self.match_ident_name("finding") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecurityFinding(items));
            }

            if self.match_ident_name("incident") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecurityIncident(items));
            }

            if self.match_ident_name("policy") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecurityPolicy(items));
            }

            if self.match_ident_name("evidence") {
                self.expect_kind(TokenKind::Colon)?;
                let items = self.parse_machine_atoms_section()?;
                return Ok(Section::SecurityEvidence(items));
            }

            if self.is_identifier_colon_section_start() {
                return self.error_here("section not allowed in [@security] context");
            }
        }

        self.error_here("Expected section")
    }

    fn is_machine_inst_section_end(&self) -> bool {
        if self.check(&TokenKind::RBrace) {
            return true;
        }

        if !self.is_section_start() {
            return false;
        }

        match self.current().kind {
            TokenKind::Identifier(_) => false,
            TokenKind::KeywordBind | TokenKind::KeywordBinary => {
                matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                )
            }
            _ => true,
        }
    }

    fn is_machine_inline_label_start(&self) -> bool {
        matches!(
            (
                &self.current().kind,
                self.tokens.get(self.pos + 1).map(|t| &t.kind)
            ),
            (TokenKind::Identifier(_), Some(TokenKind::Colon))
        )
    }

    fn machine_token_text(token: &ncc_lexer::Token) -> Result<String, Diagnostic> {
        let text = match &token.kind {
            TokenKind::Identifier(s) => s.clone(),
            TokenKind::Number(s) => s.clone(),
            TokenKind::String(s) => format!("\"{}\"", s),
            TokenKind::Comma => ",".into(),
            TokenKind::Dot => ".".into(),
            TokenKind::At => "@".into(),
            TokenKind::Star => "*".into(),
            TokenKind::StarEq => "*=".into(),
            TokenKind::Colon => ":".into(),
            TokenKind::Arrow => "->".into(),
            TokenKind::Arrow2 => "->>".into(),
            TokenKind::Shift2 => ">>".into(),
            TokenKind::Shift3 => ">>>".into(),
            TokenKind::ShiftLeft => "<<".into(),
            TokenKind::BangShift => "!>".into(),
            TokenKind::Bang => "!".into(),
            TokenKind::Pipe => "|".into(),
            TokenKind::Amp => "&".into(),
            TokenKind::Caret => "^".into(),
            TokenKind::Tilde => "~".into(),
            TokenKind::Plus => "+".into(),
            TokenKind::PlusPlus => "++".into(),
            TokenKind::PlusEq => "+=".into(),
            TokenKind::Minus => "-".into(),
            TokenKind::MinusMinus => "--".into(),
            TokenKind::MinusEq => "-=".into(),
            TokenKind::Slash => "/".into(),
            TokenKind::Percent => "%".into(),
            TokenKind::AndAnd => "&&".into(),
            TokenKind::OrOr => "||".into(),
            TokenKind::Less => "<".into(),
            TokenKind::Greater => ">".into(),
            TokenKind::GreaterEq => ">=".into(),
            TokenKind::LessEq => "<=".into(),
            TokenKind::EqEq => "==".into(),
            TokenKind::NotEq => "!=".into(),
            TokenKind::LParen => "(".into(),
            TokenKind::RParen => ")".into(),
            TokenKind::LBracket => "[".into(),
            TokenKind::RBracket => "]".into(),
            TokenKind::LBrace => "{".into(),
            TokenKind::RBrace => "}".into(),
            TokenKind::KeywordReg => "reg".into(),
            TokenKind::KeywordFlag => "flag".into(),
            TokenKind::KeywordLabel => "label".into(),
            TokenKind::KeywordInst => "inst".into(),
            TokenKind::KeywordFinal => "final".into(),
            TokenKind::KeywordBind => "bind".into(),
            TokenKind::KeywordBinary => "binary".into(),
            other => {
                return Err(Diagnostic {
                    message: format!("unexpected token in machine inst section: {:?}", other),
                    line: token.line,
                    column: token.column,
                });
            }
        };

        Ok(text)
    }

    fn parse_machine_inst_section(&mut self) -> Result<Vec<MachineInst>, Diagnostic> {
        let mut items = Vec::new();

        while !self.is_machine_inst_section_end() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }

            if self.is_machine_inline_label_start() {
                let name = match &self.current().kind {
                    TokenKind::Identifier(s) => s.clone(),
                    _ => unreachable!(),
                };

                self.advance();
                self.expect_kind(TokenKind::Colon)?;

                items.push(MachineInst::Label { name });

                if self.check(&TokenKind::Semicolon) {
                    self.expect_kind(TokenKind::Semicolon)?;
                }

                continue;
            }

            let first_line = self.current().line;
            let first_column = self.current().column;
            let start = self.pos;

            while !self.is_machine_inst_section_end()
                && !self.check(&TokenKind::Semicolon)
                && !self.is_machine_inline_label_start()
            {
                self.advance();
            }

            let end = self.pos;
            if start != end {
                let tokens = self.tokens[start..end].to_vec();
                items.push(self.parse_machine_inst_from_tokens(
                    &tokens,
                    first_line,
                    first_column,
                )?);
            }

            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
            }
        }

        Ok(items)
    }

    fn normalize_machine_parts(&self, parts: Vec<String>) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0usize;

        while i < parts.len() {
            let cur = parts[i].as_str();

            if cur == "," || cur == "->" {
                i += 1;
                continue;
            }

            if i + 2 < parts.len() && parts[i + 1] == "." {
                let mut merged = parts[i].clone();
                let mut j = i;
                while j + 2 < parts.len() && parts[j + 1] == "." {
                    merged.push('.');
                    merged.push_str(&parts[j + 2]);
                    j += 2;
                }
                out.push(merged);
                i = j + 1;
                continue;
            }

            out.push(parts[i].clone());
            i += 1;
        }

        out
    }

    fn render_machine_tokens(&self, tokens: &[Token]) -> Result<String, Diagnostic> {
        let parts = tokens
            .iter()
            .map(Self::machine_token_text)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.normalize_machine_parts(parts).join(" "))
    }

    fn parse_machine_atom(&self, tokens: &[Token], start: usize) -> Option<(String, usize)> {
        let token = tokens.get(start)?;
        let value = match &token.kind {
            TokenKind::Identifier(s) => s.clone(),
            TokenKind::Number(s) => s.clone(),
            TokenKind::String(s) => format!("\"{}\"", s),
            _ => return None,
        };
        Some((value, start + 1))
    }

    fn parse_machine_path_segment(
        &self,
        tokens: &[Token],
        start: usize,
    ) -> Option<(String, usize)> {
        let token = tokens.get(start)?;
        let value = match &token.kind {
            TokenKind::Identifier(s) => s.clone(),
            TokenKind::KeywordBind => "bind".to_string(),
            TokenKind::KeywordBinary => "binary".to_string(),
            _ => return None,
        };
        Some((value, start + 1))
    }

    fn parse_machine_path(&self, tokens: &[Token], start: usize) -> Option<(Vec<String>, usize)> {
        let (head, mut idx) = self.parse_machine_path_segment(tokens, start)?;
        let mut out = vec![head];

        while matches!(tokens.get(idx).map(|t| &t.kind), Some(TokenKind::Dot)) {
            let (segment, next) = self.parse_machine_path_segment(tokens, idx + 1)?;
            out.push(segment);
            idx = next;
        }

        Some((out, idx))
    }

    fn parse_machine_call_target(
        &self,
        tokens: &[Token],
        start: usize,
    ) -> Option<(MachineCallTarget, usize)> {
        let (path, idx) = self.parse_machine_path(tokens, start)?;

        if path.len() == 1 {
            return Some((MachineCallTarget::Label(path[0].clone()), idx));
        }

        if path.len() >= 2 && path[0] == "root" {
            return Some((MachineCallTarget::RootPath(path), idx));
        }

        None
    }

    fn parse_machine_opcode(&self, tokens: &[Token], start: usize) -> Option<(String, usize)> {
        let head = match &tokens.get(start)?.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => return None,
        };

        let mut opcode = head;
        let mut idx = start + 1;
        while idx + 1 < tokens.len() {
            if !matches!(tokens[idx].kind, TokenKind::Dot) {
                break;
            }
            let segment = match &tokens[idx + 1].kind {
                TokenKind::Identifier(s) => s,
                _ => break,
            };
            opcode.push('.');
            opcode.push_str(segment);
            idx += 2;
        }

        Some((opcode, idx))
    }

    fn parse_machine_optional_comma(&self, tokens: &[Token], start: usize) -> usize {
        if matches!(tokens.get(start).map(|t| &t.kind), Some(TokenKind::Comma)) {
            start + 1
        } else {
            start
        }
    }

    fn parse_machine_optional_arrow(&self, tokens: &[Token], start: usize) -> usize {
        if matches!(tokens.get(start).map(|t| &t.kind), Some(TokenKind::Arrow)) {
            start + 1
        } else {
            start
        }
    }

    fn parse_machine_binary_operands(
        &self,
        tokens: &[Token],
        start: usize,
    ) -> Option<(String, String, usize)> {
        let (left, idx) = self.parse_machine_atom(tokens, start)?;
        let idx = self.parse_machine_optional_comma(tokens, idx);
        let (right, idx) = self.parse_machine_atom(tokens, idx)?;
        Some((left, right, idx))
    }

    fn parse_machine_mem_ref(
        &self,
        tokens: &[Token],
        start: usize,
    ) -> Option<(MachineMemRef, usize)> {
        if !matches!(
            tokens.get(start).map(|t| &t.kind),
            Some(TokenKind::Identifier(s)) if s == "mem"
        ) {
            return None;
        }
        if !matches!(
            tokens.get(start + 1).map(|t| &t.kind),
            Some(TokenKind::LBracket)
        ) {
            return None;
        }
        let (base, mut idx) = self.parse_machine_atom(tokens, start + 2)?;
        let mut index: Option<String> = None;
        let mut scale: u8 = 1;
        let mut disp: i64 = 0;
        let mut extended = false;

        while let Some(op) = tokens.get(idx).map(|t| &t.kind) {
            let sign = match op {
                TokenKind::Plus => 1_i64,
                TokenKind::Minus => -1_i64,
                _ => break,
            };
            idx += 1;

            let (term, next) = self.parse_machine_atom(tokens, idx)?;
            idx = next;

            let mut term_is_index = false;
            let mut term_scale = 1u8;
            if matches!(tokens.get(idx).map(|t| &t.kind), Some(TokenKind::Star)) {
                idx += 1;
                let (scale_raw, next) = self.parse_machine_atom(tokens, idx)?;
                idx = next;
                term_scale = Self::parse_machine_scale_literal(&scale_raw)?;
                term_is_index = true;
            }

            if term_is_index {
                if sign < 0 || index.is_some() {
                    return None;
                }
                index = Some(term);
                scale = term_scale;
                extended = true;
                continue;
            }

            if let Some(value) = Self::parse_machine_int_literal(&term) {
                let signed = if sign > 0 { value } else { -value };
                disp = disp.checked_add(signed)?;
                extended = true;
                continue;
            }

            if sign < 0 || index.is_some() {
                return None;
            }
            index = Some(term);
            scale = 1;
            extended = true;
        }

        if !matches!(tokens.get(idx).map(|t| &t.kind), Some(TokenKind::RBracket)) {
            return None;
        }

        if !extended {
            return Some((MachineMemRef::MemReg { base }, idx + 1));
        }

        let disp = isize::try_from(disp).ok()?;
        Some((
            MachineMemRef::MemAddr {
                base,
                index,
                scale,
                disp,
            },
            idx + 1,
        ))
    }

    fn parse_machine_reg_or_mem(
        &self,
        tokens: &[Token],
        start: usize,
    ) -> Option<(MachineRegOrMem, usize)> {
        if let Some((mem, next)) = self.parse_machine_mem_ref(tokens, start) {
            return Some((MachineRegOrMem::Mem(mem), next));
        }
        let (reg, next) = self.parse_machine_atom(tokens, start)?;
        Some((MachineRegOrMem::Reg(reg), next))
    }

    fn parse_machine_scale_literal(raw: &str) -> Option<u8> {
        let scale = Self::parse_machine_int_literal(raw)?;
        match scale {
            1 | 2 | 4 | 8 => Some(scale as u8),
            _ => None,
        }
    }

    fn parse_machine_int_literal(raw: &str) -> Option<i64> {
        let compact: String = raw.chars().filter(|ch| *ch != '_').collect();

        if let Some(bits) = compact
            .strip_prefix("0b")
            .or_else(|| compact.strip_prefix("0B"))
        {
            if bits.is_empty() || !bits.chars().all(|ch| ch == '0' || ch == '1') {
                return None;
            }
            return i64::try_from(u64::from_str_radix(bits, 2).ok()?).ok();
        }

        if let Some(hex) = compact
            .strip_prefix("0x")
            .or_else(|| compact.strip_prefix("0X"))
        {
            if hex.is_empty() || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return None;
            }
            return i64::try_from(u64::from_str_radix(hex, 16).ok()?).ok();
        }

        if compact.chars().all(|ch| ch.is_ascii_digit()) {
            return compact.parse::<i64>().ok();
        }

        None
    }

    fn parse_machine_inst_from_tokens(
        &self,
        tokens: &[Token],
        line: usize,
        column: usize,
    ) -> Result<MachineInst, Diagnostic> {
        let invalid_inst = || -> Result<MachineInst, Diagnostic> {
            let rendered = self.render_machine_tokens(tokens)?;
            Err(Diagnostic {
                message: format!(
                    "invalid machine instruction in [@machine] inst section: '{}'",
                    rendered
                ),
                line,
                column,
            })
        };

        if tokens.is_empty() {
            return Err(Diagnostic {
                message: "empty machine instruction in [@machine] inst section".into(),
                line,
                column,
            });
        }

        let Some((opcode, mut idx)) = self.parse_machine_opcode(tokens, 0) else {
            return invalid_inst();
        };

        let inst = match opcode.as_str() {
            "ret" => {
                if idx != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Ret
            }
            "nop" => {
                if idx != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Nop
            }
            "wait" => {
                if idx != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Wait
            }
            "halt" => {
                if idx != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Halt
            }
            "vmcall" | "vmxoff" | "vmlaunch" | "vmresume" | "vmrun" | "vmmcall" | "vmload"
            | "vmsave" | "stgi" | "clgi" | "invlpga" => {
                if idx != tokens.len() {
                    return invalid_inst();
                }
                match opcode.as_str() {
                    "vmcall" => MachineInst::VmCall,
                    "vmxoff" => MachineInst::VmxOff,
                    "vmlaunch" => MachineInst::VmLaunch,
                    "vmresume" => MachineInst::VmResume,
                    "vmrun" => MachineInst::VmRun,
                    "vmmcall" => MachineInst::VmmCall,
                    "vmload" => MachineInst::VmLoad,
                    "vmsave" => MachineInst::VmSave,
                    "stgi" => MachineInst::StGi,
                    "clgi" => MachineInst::ClGi,
                    "invlpga" => MachineInst::InvLpga,
                    _ => unreachable!(),
                }
            }
            "vmxon" | "vmclear" | "vmptrld" | "vmptrst" => {
                let Some((region, next)) = self.parse_machine_mem_ref(tokens, idx) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                match opcode.as_str() {
                    "vmxon" => MachineInst::VmxOn { region },
                    "vmclear" => MachineInst::VmClear { region },
                    "vmptrld" => MachineInst::VmPtrLoad { region },
                    "vmptrst" => MachineInst::VmPtrStore {
                        destination: region,
                    },
                    _ => unreachable!(),
                }
            }
            "vmread" => {
                let Some((destination, next)) = self.parse_machine_reg_or_mem(tokens, idx) else {
                    return invalid_inst();
                };
                let next = self.parse_machine_optional_comma(tokens, next);
                let Some((field, next)) = self.parse_machine_atom(tokens, next) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::VmRead { destination, field }
            }
            "vmwrite" => {
                let Some((value, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_inst();
                };
                let next = self.parse_machine_optional_comma(tokens, next);
                let Some((field, next)) = self.parse_machine_reg_or_mem(tokens, next) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::VmWrite { value, field }
            }
            "invept" | "invvpid" => {
                let Some((kind, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_inst();
                };
                let next = self.parse_machine_optional_comma(tokens, next);
                let Some((descriptor, next)) = self.parse_machine_mem_ref(tokens, next) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                if opcode == "invept" {
                    MachineInst::InvEpt { kind, descriptor }
                } else {
                    MachineInst::InvVpid { kind, descriptor }
                }
            }
            "load" => {
                let Some((dst, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_inst();
                };
                let next = self.parse_machine_optional_comma(tokens, next);
                let Some((src, next)) = self.parse_machine_mem_ref(tokens, next) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Load { dst, src }
            }
            "store" => {
                let Some((dst, next)) = self.parse_machine_mem_ref(tokens, idx) else {
                    return invalid_inst();
                };
                let next = self.parse_machine_optional_comma(tokens, next);
                let Some((src, next)) = self.parse_machine_atom(tokens, next) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Store { dst, src }
            }
            "lea" => {
                let Some((dst, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_inst();
                };
                let next = self.parse_machine_optional_comma(tokens, next);
                let Some((src, next)) = self.parse_machine_mem_ref(tokens, next) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Lea { dst, src }
            }
            "call" => {
                idx = self.parse_machine_optional_arrow(tokens, idx);
                let Some((target, next)) = self.parse_machine_call_target(tokens, idx) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                MachineInst::Call { target }
            }
            "jmp" | "jmp.eq" | "jmp.ne" | "jmp.gt" | "jmp.lt" | "jmp.ge" | "jmp.le" | "jmp.ov"
            | "je" | "jne" | "jg" | "jl" | "jge" | "jle" | "jo" => {
                idx = self.parse_machine_optional_arrow(tokens, idx);
                let Some((target, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                match opcode.as_str() {
                    "jmp" => MachineInst::Jmp { target },
                    "jmp.eq" => MachineInst::JmpEq { target },
                    "jmp.ne" => MachineInst::JmpNe { target },
                    "jmp.gt" => MachineInst::JmpGt { target },
                    "jmp.lt" => MachineInst::JmpLt { target },
                    "jmp.ge" => MachineInst::JmpGe { target },
                    "jmp.le" => MachineInst::JmpLe { target },
                    "jmp.ov" => MachineInst::JmpOverflow { target },
                    "je" => MachineInst::JmpEq { target },
                    "jne" => MachineInst::JmpNe { target },
                    "jg" => MachineInst::JmpGt { target },
                    "jl" => MachineInst::JmpLt { target },
                    "jge" => MachineInst::JmpGe { target },
                    "jle" => MachineInst::JmpLe { target },
                    "jo" => MachineInst::JmpOverflow { target },
                    _ => unreachable!(),
                }
            }
            "push" | "pop" | "inc" | "dec" | "not.bin" | "bswap" | "neg.bin" => {
                let Some((reg, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                match opcode.as_str() {
                    "push" => MachineInst::Push { reg },
                    "pop" => MachineInst::Pop { reg },
                    "inc" => MachineInst::Inc { reg },
                    "dec" => MachineInst::Dec { reg },
                    "not.bin" => MachineInst::NotBin { reg },
                    "bswap" => MachineInst::Bswap { reg },
                    "neg.bin" => MachineInst::NegBin { reg },
                    _ => unreachable!(),
                }
            }
            "mov" | "cmp" | "test" | "add" | "sub" | "mul" | "adc" | "sbb" | "and" | "or"
            | "xor" | "shl" | "shr" | "sar" | "rol" | "ror" | "xchg" | "bsf" | "bsr" => {
                let Some((left, right, next)) = self.parse_machine_binary_operands(tokens, idx)
                else {
                    return invalid_inst();
                };
                if next != tokens.len() {
                    return invalid_inst();
                }
                match opcode.as_str() {
                    "mov" => MachineInst::Mov {
                        dst: left,
                        src: right,
                    },
                    "cmp" => MachineInst::Cmp { left, right },
                    "test" => MachineInst::Test { left, right },
                    "bsf" => MachineInst::Bsf {
                        dst: left,
                        src: right,
                    },
                    "bsr" => MachineInst::Bsr {
                        dst: left,
                        src: right,
                    },
                    "add" => MachineInst::Add {
                        dst: left,
                        src: right,
                    },
                    "sub" => MachineInst::Sub {
                        dst: left,
                        src: right,
                    },
                    "mul" => MachineInst::Mul {
                        dst: left,
                        src: right,
                    },
                    "adc" => MachineInst::Adc {
                        dst: left,
                        src: right,
                    },
                    "sbb" => MachineInst::Sbb {
                        dst: left,
                        src: right,
                    },
                    "and" => MachineInst::And {
                        dst: left,
                        src: right,
                    },
                    "or" => MachineInst::Or {
                        dst: left,
                        src: right,
                    },
                    "xor" => MachineInst::Xor {
                        dst: left,
                        src: right,
                    },
                    "shl" => MachineInst::Shl {
                        dst: left,
                        src: right,
                    },
                    "shr" => MachineInst::Shr {
                        dst: left,
                        src: right,
                    },
                    "sar" => MachineInst::Sar {
                        dst: left,
                        src: right,
                    },
                    "rol" => MachineInst::Rol {
                        dst: left,
                        src: right,
                    },
                    "ror" => MachineInst::Ror {
                        dst: left,
                        src: right,
                    },
                    "xchg" => MachineInst::Xchg {
                        dst: left,
                        src: right,
                    },
                    _ => unreachable!(),
                }
            }
            _ => return invalid_inst(),
        };

        Ok(inst)
    }

    fn parse_machine_final_section(&mut self) -> Result<Vec<MachineFinalOp>, Diagnostic> {
        let mut items = Vec::new();

        while !self.is_machine_inst_section_end() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }

            let first_line = self.current().line;
            let first_column = self.current().column;
            let start = self.pos;

            while !self.is_machine_inst_section_end() && !self.check(&TokenKind::Semicolon) {
                self.advance();
            }

            let end = self.pos;
            if start != end {
                let tokens = self.tokens[start..end].to_vec();
                items.push(self.parse_machine_final_from_tokens(
                    &tokens,
                    first_line,
                    first_column,
                )?);
            }

            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
            }
        }

        Ok(items)
    }

    fn parse_machine_final_from_tokens(
        &self,
        tokens: &[Token],
        line: usize,
        column: usize,
    ) -> Result<MachineFinalOp, Diagnostic> {
        let invalid_final = || -> Result<MachineFinalOp, Diagnostic> {
            let rendered = self.render_machine_tokens(tokens)?;
            Err(Diagnostic {
                message: format!(
                    "final section in [@machine] container only allows 'ret', 'halt' or 'jmp <label>', got '{}'",
                    rendered
                ),
                line,
                column,
            })
        };

        if tokens.is_empty() {
            return Err(Diagnostic {
                message: "empty operation in [@machine] final section".into(),
                line,
                column,
            });
        }

        let Some((opcode, mut idx)) = self.parse_machine_opcode(tokens, 0) else {
            return invalid_final();
        };

        let op = match opcode.as_str() {
            "ret" => {
                if idx != tokens.len() {
                    return invalid_final();
                }
                MachineFinalOp::Ret
            }
            "halt" => {
                if idx != tokens.len() {
                    return invalid_final();
                }
                MachineFinalOp::Halt
            }
            "jmp" => {
                idx = self.parse_machine_optional_arrow(tokens, idx);
                let Some((target, next)) = self.parse_machine_atom(tokens, idx) else {
                    return invalid_final();
                };
                if next != tokens.len() {
                    return invalid_final();
                }
                MachineFinalOp::Jmp { target }
            }
            _ => return invalid_final(),
        };

        Ok(op)
    }

    fn peek_starts_type_contract_decl(&self) -> bool {
        let Some(TokenKind::Identifier(_)) = self.tokens.get(self.pos).map(|tok| &tok.kind) else {
            return false;
        };

        let mut idx = self.pos + 1;

        if matches!(
            self.tokens.get(idx).map(|tok| &tok.kind),
            Some(TokenKind::Less)
        ) {
            idx += 1;
            loop {
                let Some(TokenKind::Identifier(_)) = self.tokens.get(idx).map(|tok| &tok.kind)
                else {
                    return false;
                };
                idx += 1;
                if matches!(
                    self.tokens.get(idx).map(|tok| &tok.kind),
                    Some(TokenKind::Comma)
                ) {
                    idx += 1;
                    continue;
                }
                if matches!(
                    self.tokens.get(idx).map(|tok| &tok.kind),
                    Some(TokenKind::Greater)
                ) {
                    idx += 1;
                    break;
                }
                return false;
            }
        }

        if let Some(TokenKind::Identifier(kind_name)) = self.tokens.get(idx).map(|tok| &tok.kind) {
            if kind_name.eq_ignore_ascii_case("enum") || kind_name.eq_ignore_ascii_case("union") {
                idx += 1;
            } else if kind_name.eq_ignore_ascii_case("tagged") {
                idx += 1;
                let Some(TokenKind::Identifier(union_kw)) =
                    self.tokens.get(idx).map(|tok| &tok.kind)
                else {
                    return false;
                };
                if !union_kw.eq_ignore_ascii_case("union") {
                    return false;
                }
                idx += 1;
            }
        }

        matches!(
            self.tokens.get(idx).map(|tok| &tok.kind),
            Some(TokenKind::LBrace)
        )
    }

    fn is_type_contract_clause_start(&self) -> bool {
        if matches!(
            self.current().kind,
            TokenKind::KeywordPublic
                | TokenKind::KeywordPrivate
                | TokenKind::KeywordInternal
                | TokenKind::KeywordStrictly
                | TokenKind::KeywordMove
        ) && matches!(
            self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
            Some(TokenKind::Colon)
        ) {
            return true;
        }

        matches!(
            (
                self.tokens.get(self.pos).map(|tok| &tok.kind),
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind)
            ),
            (Some(TokenKind::Identifier(name)), Some(TokenKind::Colon))
                if matches!(
                    name.as_str(),
                    "layout"
                        | "invariant"
                        | "bounds"
                        | "constructure"
                        | "destructure"
                        | "copy"
                        | "clone"
                        | "move"
                        | "ffi"
                        | "ownership"
                        | "nullability"
                        | "methods"
                        | "associated"
                        | "validate"
                        | "drop"
                        | "default"
                        | "compare"
                        | "hash"
                        | "debug"
                        | "stable"
                )
        )
    }

    fn parse_canonical_type_contract_clause(
        &mut self,
    ) -> Result<Option<TypeContractClause>, Diagnostic> {
        let name = match &self.current().kind {
            TokenKind::Identifier(name)
                if matches!(
                    self.tokens.get(self.pos + 1).map(|token| &token.kind),
                    Some(TokenKind::Colon)
                ) && matches!(
                    name.as_str(),
                    "layout"
                        | "invariant"
                        | "bounds"
                        | "constructure"
                        | "destructure"
                        | "copy"
                        | "clone"
                        | "move"
                        | "ffi"
                        | "ownership"
                        | "nullability"
                        | "methods"
                        | "associated"
                        | "validate"
                        | "drop"
                        | "default"
                        | "compare"
                        | "hash"
                        | "debug"
                        | "stable"
                ) =>
            {
                name.clone()
            }
            TokenKind::KeywordMove
                if matches!(
                    self.tokens.get(self.pos + 1).map(|token| &token.kind),
                    Some(TokenKind::Colon)
                ) =>
            {
                "move".to_string()
            }
            _ => return Ok(None),
        };
        self.advance();
        self.expect_kind(TokenKind::Colon)?;
        let items = self.parse_type_contract_clause_items()?;
        let clause = match name.as_str() {
            "layout" => TypeContractClause::Layout(items),
            "invariant" => TypeContractClause::Invariant(items),
            "bounds" => TypeContractClause::Bounds(items),
            "constructure" => TypeContractClause::Constructure(items),
            "destructure" => TypeContractClause::Destructure(items),
            "copy" => TypeContractClause::Copy(items),
            "clone" => TypeContractClause::Clone(items),
            "move" => TypeContractClause::Move(items),
            "ffi" => TypeContractClause::Ffi(items),
            "ownership" => TypeContractClause::Ownership(items),
            "nullability" => TypeContractClause::Nullability(items),
            "methods" => TypeContractClause::Methods(items),
            "associated" => TypeContractClause::Associated(items),
            "validate" => TypeContractClause::Validate(items),
            "drop" => TypeContractClause::Drop(items),
            "default" => TypeContractClause::Default(items),
            "compare" => TypeContractClause::Compare(items),
            "hash" => TypeContractClause::Hash(items),
            "debug" => TypeContractClause::Debug(items),
            "stable" => TypeContractClause::Stable(items),
            _ => unreachable!("canonical type clause name was validated"),
        };
        Ok(Some(clause))
    }

    fn parse_type_contract_decl(&mut self) -> Result<TypeContractDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let mut generic_params = Vec::new();
        if self.match_kind(&TokenKind::Less) {
            loop {
                generic_params.push(self.expect_ident()?);
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                self.expect_kind(TokenKind::Greater)?;
                break;
            }
        }

        let mut kind = TypeContractKind::Struct;
        if self.match_ident_name("enum") {
            kind = TypeContractKind::Enum;
        } else if self.match_ident_name("union") {
            kind = TypeContractKind::Union;
        } else if self.match_ident_name("tagged") {
            if !self.match_ident_name("union") {
                return self.error_here(
                    "E_TYPE_CONTRACT_SECTION_001: phase=parser.type reason=invalid type contract section status=rejected",
                );
            }
            kind = TypeContractKind::TaggedUnion;
        }

        self.expect_kind(TokenKind::LBrace)?;

        let mut variants = Vec::new();
        let mut clauses = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if matches!(kind, TypeContractKind::Enum | TypeContractKind::TaggedUnion)
                && !self.is_type_contract_clause_start()
            {
                variants.push(self.parse_type_contract_variant_decl()?);
                continue;
            }

            // Compact structs and unions use public fields by default. Explicit
            // visibility clauses remain available and may follow this group.
            if matches!(kind, TypeContractKind::Struct | TypeContractKind::Union)
                && !self.is_type_contract_clause_start()
                && matches!(self.current().kind, TokenKind::Identifier(_))
            {
                let fields = self.parse_type_contract_fields()?;
                clauses.push(TypeContractClause::Fields {
                    visibility: Visibility::Public,
                    fields,
                });
                continue;
            }

            if self.match_kind(&TokenKind::KeywordPublic) {
                self.expect_kind(TokenKind::Colon)?;
                let fields = self.parse_type_contract_fields()?;
                clauses.push(TypeContractClause::Fields {
                    visibility: Visibility::Public,
                    fields,
                });
                continue;
            }
            if self.match_kind(&TokenKind::KeywordPrivate) {
                self.expect_kind(TokenKind::Colon)?;
                let fields = self.parse_type_contract_fields()?;
                clauses.push(TypeContractClause::Fields {
                    visibility: Visibility::Private,
                    fields,
                });
                continue;
            }
            if self.match_kind(&TokenKind::KeywordInternal) {
                self.expect_kind(TokenKind::Colon)?;
                let fields = self.parse_type_contract_fields()?;
                clauses.push(TypeContractClause::Fields {
                    visibility: Visibility::Internal,
                    fields,
                });
                continue;
            }
            if self.match_kind(&TokenKind::KeywordStrictly) {
                self.expect_kind(TokenKind::Colon)?;
                let fields = self.parse_type_contract_fields()?;
                clauses.push(TypeContractClause::Fields {
                    visibility: Visibility::Strictly,
                    fields,
                });
                continue;
            }
            if self.match_ident_name("layout") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Layout(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("invariant") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Invariant(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("bounds") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Bounds(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("constructure") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Constructure(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("destructure") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Destructure(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("copy") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Copy(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("clone") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Clone(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_kind(&TokenKind::KeywordMove) || self.match_ident_name("move") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Move(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("ffi") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Ffi(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("ownership") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Ownership(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("nullability") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Nullability(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("methods") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Methods(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("associated") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Associated(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("validate") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Validate(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("drop") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Drop(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("default") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Default(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("compare") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Compare(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("hash") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Hash(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("debug") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Debug(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }
            if self.match_ident_name("stable") {
                self.expect_kind(TokenKind::Colon)?;
                clauses.push(TypeContractClause::Stable(
                    self.parse_type_contract_clause_items()?,
                ));
                continue;
            }

            return self.error_here(
                "E_TYPE_CONTRACT_SECTION_001: phase=parser.type reason=invalid type contract section status=rejected",
            );
        }

        self.expect_kind(TokenKind::RBrace)?;
        Ok(TypeContractDecl {
            name,
            kind,
            generic_params,
            variants,
            clauses,
        })
    }

    fn peek_starts_canonical_type_decl(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Identifier(_)) {
            return false;
        }
        let mut cursor = self.pos + 1;
        if matches!(
            self.tokens.get(cursor).map(|token| &token.kind),
            Some(TokenKind::Less)
        ) {
            let mut depth = 0i32;
            while let Some(token) = self.tokens.get(cursor) {
                match token.kind {
                    TokenKind::Less => depth += 1,
                    TokenKind::Greater => depth -= 1,
                    TokenKind::Shift2 => depth -= 2,
                    TokenKind::Shift3 => depth -= 3,
                    _ => {}
                }
                cursor += 1;
                if depth <= 0 {
                    break;
                }
            }
        }
        matches!(
            (
                self.tokens.get(cursor).map(|token| &token.kind),
                self.tokens.get(cursor + 1).map(|token| &token.kind),
                self.tokens.get(cursor + 2).map(|token| &token.kind),
            ),
            (
                Some(TokenKind::Colon),
                Some(TokenKind::Colon),
                Some(TokenKind::Identifier(kind))
            ) if matches!(kind.to_ascii_lowercase().as_str(), "type" | "struct" | "union" | "enum")
        )
    }

    fn parse_canonical_type_contract_decl(&mut self) -> Result<TypeContractDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let (generic_params, _) = self.parse_optional_type_param_decls()?;
        self.expect_double_colon()?;
        let kind_word = self.expect_ident()?.to_ascii_lowercase();
        let kind =
            match kind_word.as_str() {
                "type" | "struct" => TypeContractKind::Struct,
                "union" => TypeContractKind::Union,
                "enum" => TypeContractKind::Enum,
                _ => return self.error_here(
                    "E_TYPE_CONTRACT_SECTION_001: expected type, struct, union or enum after '::'",
                ),
            };

        let mut layout = Vec::new();
        if self.match_kind(&TokenKind::LBracket) {
            while !self.check(&TokenKind::RBracket) {
                let key = self.expect_use_path_segment()?;
                let normalized = match key.to_ascii_lowercase().as_str() {
                    "repr" => {
                        let value = self.expect_use_path_segment()?;
                        format!("repr({value})")
                    }
                    "align" => {
                        let value = match self.advance().kind {
                            TokenKind::Number(value) | TokenKind::Identifier(value) => value,
                            _ => {
                                return self.error_here("E_TYPE_LAYOUT_001: align expects a value")
                            }
                        };
                        format!("align({value})")
                    }
                    "endian" => {
                        let value = self.expect_use_path_segment()?;
                        format!("endian({value})")
                    }
                    "packed" => "packed".to_string(),
                    "c" => "repr(C)".to_string(),
                    "little" | "le" => "endian(little)".to_string(),
                    "big" | "be" => "endian(big)".to_string(),
                    other => other.to_string(),
                };
                layout.push(normalized);
                if !self.match_kind(&TokenKind::Pipe) {
                    break;
                }
            }
            self.expect_kind(TokenKind::RBracket)?;
        }

        self.expect_kind(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        let mut clauses = Vec::new();
        if matches!(kind, TypeContractKind::Enum | TypeContractKind::TaggedUnion) {
            while !self.check(&TokenKind::RBrace) {
                variants.push(self.parse_type_contract_variant_decl()?);
            }
        } else {
            while !self.check(&TokenKind::RBrace) {
                let visibility = if self.match_kind(&TokenKind::KeywordPublic) {
                    Some(Visibility::Public)
                } else if self.match_kind(&TokenKind::KeywordPrivate) {
                    Some(Visibility::Private)
                } else if self.match_kind(&TokenKind::KeywordInternal) {
                    Some(Visibility::Internal)
                } else if self.match_kind(&TokenKind::KeywordStrictly) {
                    Some(Visibility::Strictly)
                } else {
                    None
                };
                if let Some(visibility) = visibility {
                    self.expect_kind(TokenKind::Colon)?;
                    clauses.push(TypeContractClause::Fields {
                        visibility,
                        fields: self.parse_type_contract_fields()?,
                    });
                    continue;
                }
                if let Some(clause) = self.parse_canonical_type_contract_clause()? {
                    clauses.push(clause);
                    continue;
                }
                if matches!(self.current().kind, TokenKind::Identifier(_)) {
                    clauses.push(TypeContractClause::Fields {
                        visibility: Visibility::Public,
                        fields: self.parse_type_contract_fields()?,
                    });
                    continue;
                }
                return self.error_here(
                    "E_TYPE_CONTRACT_SECTION_001: invalid canonical type contract body",
                );
            }
        }
        self.expect_kind(TokenKind::RBrace)?;
        if !layout.is_empty() {
            clauses.push(TypeContractClause::Layout(layout));
        }

        Ok(TypeContractDecl {
            name,
            kind,
            generic_params,
            variants,
            clauses,
        })
    }

    fn parse_type_contract_variant_decl(&mut self) -> Result<TypeContractVariant, Diagnostic> {
        let name = self.expect_ident()?;
        let mut payload = Vec::new();
        if self.match_kind(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) {
                let field_name = self.expect_ident()?;
                self.expect_kind(TokenKind::Colon)?;
                let field_ty = self.expect_type_name()?;
                payload.push(TypeContractField {
                    name: field_name,
                    ty: field_ty,
                });
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(TokenKind::RParen)?;
        }
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(TypeContractVariant { name, payload })
    }

    fn parse_type_contract_fields(&mut self) -> Result<Vec<TypeContractField>, Diagnostic> {
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_type_contract_clause_start() {
            let field_name = self.expect_ident()?;
            if !(self.match_kind(&TokenKind::Colon) || self.match_kind(&TokenKind::Shift2)) {
                return self
                    .error_here("E_TYPE_FIELD_001: type field expects ':' or '>>' type syntax");
            }
            let field_ty = self.expect_type_name()?;
            self.expect_kind(TokenKind::Semicolon)?;
            fields.push(TypeContractField {
                name: field_name,
                ty: field_ty,
            });
        }
        if fields.is_empty() {
            return self.error_here("E_TYPE_FIELD_001: invalid or unknown type field");
        }
        Ok(fields)
    }

    fn parse_type_contract_clause_items(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_type_contract_clause_start() {
            let start = self.pos;
            let mut brace_depth = 0usize;
            let mut paren_depth = 0usize;
            let mut bracket_depth = 0usize;
            loop {
                match &self.current().kind {
                    TokenKind::LBrace => {
                        brace_depth = brace_depth.saturating_add(1);
                        self.advance();
                    }
                    TokenKind::RBrace => {
                        if brace_depth > 0 {
                            brace_depth -= 1;
                            self.advance();
                        } else if paren_depth == 0 && bracket_depth == 0 {
                            break;
                        } else {
                            self.advance();
                        }
                    }
                    TokenKind::LParen => {
                        paren_depth = paren_depth.saturating_add(1);
                        self.advance();
                    }
                    TokenKind::RParen => {
                        paren_depth = paren_depth.saturating_sub(1);
                        self.advance();
                    }
                    TokenKind::LBracket => {
                        bracket_depth = bracket_depth.saturating_add(1);
                        self.advance();
                    }
                    TokenKind::RBracket => {
                        bracket_depth = bracket_depth.saturating_sub(1);
                        self.advance();
                    }
                    TokenKind::Semicolon
                        if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 =>
                    {
                        break;
                    }
                    TokenKind::Eof => break,
                    _ => {
                        self.advance();
                    }
                }
            }
            if self.check(&TokenKind::RBrace) {
                return self.error_here("Expected Semicolon");
            }
            let end = self.pos;
            self.expect_kind(TokenKind::Semicolon)?;
            if end > start {
                let rendered = self.tokens[start..end]
                    .iter()
                    .map(|tok| match &tok.kind {
                        TokenKind::Identifier(s) => s.clone(),
                        TokenKind::Number(s) => s.clone(),
                        TokenKind::String(s) => format!("\"{}\"", s),
                        TokenKind::Comma => ",".to_string(),
                        TokenKind::Dot => ".".to_string(),
                        TokenKind::At => "@".to_string(),
                        TokenKind::Star => "*".to_string(),
                        TokenKind::StarEq => "*=".to_string(),
                        TokenKind::Colon => ":".to_string(),
                        TokenKind::Arrow => "->".to_string(),
                        TokenKind::Arrow2 => "->>".to_string(),
                        TokenKind::Shift2 => ">>".to_string(),
                        TokenKind::Shift3 => ">>>".to_string(),
                        TokenKind::ShiftLeft => "<<".to_string(),
                        TokenKind::BangShift => "!>".to_string(),
                        TokenKind::Bang => "!".to_string(),
                        TokenKind::Pipe => "|".to_string(),
                        TokenKind::Amp => "&".to_string(),
                        TokenKind::Caret => "^".to_string(),
                        TokenKind::Tilde => "~".to_string(),
                        TokenKind::Plus => "+".to_string(),
                        TokenKind::PlusPlus => "++".to_string(),
                        TokenKind::PlusEq => "+=".to_string(),
                        TokenKind::Minus => "-".to_string(),
                        TokenKind::MinusMinus => "--".to_string(),
                        TokenKind::MinusEq => "-=".to_string(),
                        TokenKind::Slash => "/".to_string(),
                        TokenKind::Percent => "%".to_string(),
                        TokenKind::AndAnd => "&&".to_string(),
                        TokenKind::OrOr => "||".to_string(),
                        TokenKind::Less => "<".to_string(),
                        TokenKind::Greater => ">".to_string(),
                        TokenKind::GreaterEq => ">=".to_string(),
                        TokenKind::LessEq => "<=".to_string(),
                        TokenKind::EqEq => "==".to_string(),
                        TokenKind::NotEq => "!=".to_string(),
                        TokenKind::Eq => "=".to_string(),
                        TokenKind::LParen => "(".to_string(),
                        TokenKind::RParen => ")".to_string(),
                        TokenKind::LBracket => "[".to_string(),
                        TokenKind::RBracket => "]".to_string(),
                        TokenKind::LBrace => "{".to_string(),
                        TokenKind::RBrace => "}".to_string(),
                        TokenKind::KeywordLocal => "local".to_string(),
                        TokenKind::KeywordMove => "move".to_string(),
                        TokenKind::KeywordBinary => "binary".to_string(),
                        TokenKind::KeywordRef => "ref".to_string(),
                        TokenKind::KeywordPublic => "public".to_string(),
                        TokenKind::KeywordPrivate => "private".to_string(),
                        TokenKind::KeywordInternal => "internal".to_string(),
                        TokenKind::KeywordStrictly => "strictly".to_string(),
                        TokenKind::KeywordTrue => "true".to_string(),
                        TokenKind::KeywordFalse => "false".to_string(),
                        _ => format!("{:?}", tok.kind),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                items.push(rendered.trim().to_string());
            }
        }
        if items.is_empty() {
            return self.error_here("type contract clause cannot be empty");
        }
        Ok(items)
    }

    fn token_fragment_for_contract(kind: &TokenKind) -> Option<String> {
        Some(match kind {
            TokenKind::Identifier(s) => s.clone(),
            TokenKind::Number(s) => s.clone(),
            TokenKind::String(s) => format!("\"{}\"", s),
            TokenKind::Comma => ",".to_string(),
            TokenKind::Dot => ".".to_string(),
            TokenKind::At => "@".to_string(),
            TokenKind::Star => "*".to_string(),
            TokenKind::StarEq => "*=".to_string(),
            TokenKind::Colon => ":".to_string(),
            TokenKind::Arrow => "->".to_string(),
            TokenKind::Arrow2 => "->>".to_string(),
            TokenKind::Shift2 => ">>".to_string(),
            TokenKind::Shift3 => ">>>".to_string(),
            TokenKind::ShiftLeft => "<<".to_string(),
            TokenKind::BangShift => "!>".to_string(),
            TokenKind::Bang => "!".to_string(),
            TokenKind::Pipe => "|".to_string(),
            TokenKind::Amp => "&".to_string(),
            TokenKind::Caret => "^".to_string(),
            TokenKind::Tilde => "~".to_string(),
            TokenKind::Plus => "+".to_string(),
            TokenKind::PlusPlus => "++".to_string(),
            TokenKind::PlusEq => "+=".to_string(),
            TokenKind::Minus => "-".to_string(),
            TokenKind::MinusMinus => "--".to_string(),
            TokenKind::MinusEq => "-=".to_string(),
            TokenKind::Slash => "/".to_string(),
            TokenKind::Percent => "%".to_string(),
            TokenKind::AndAnd => "&&".to_string(),
            TokenKind::OrOr => "||".to_string(),
            TokenKind::Less => "<".to_string(),
            TokenKind::Greater => ">".to_string(),
            TokenKind::GreaterEq => ">=".to_string(),
            TokenKind::LessEq => "<=".to_string(),
            TokenKind::EqEq => "==".to_string(),
            TokenKind::NotEq => "!=".to_string(),
            TokenKind::Eq => "=".to_string(),
            TokenKind::LParen => "(".to_string(),
            TokenKind::RParen => ")".to_string(),
            TokenKind::LBracket => "[".to_string(),
            TokenKind::RBracket => "]".to_string(),
            TokenKind::LBrace => "{".to_string(),
            TokenKind::RBrace => "}".to_string(),
            TokenKind::KeywordLocal => "local".to_string(),
            TokenKind::KeywordMove => "move".to_string(),
            TokenKind::KeywordBinary => "binary".to_string(),
            TokenKind::KeywordRef => "ref".to_string(),
            TokenKind::KeywordPublic => "public".to_string(),
            TokenKind::KeywordPrivate => "private".to_string(),
            TokenKind::KeywordInternal => "internal".to_string(),
            TokenKind::KeywordStrictly => "strictly".to_string(),
            TokenKind::KeywordTrue => "true".to_string(),
            TokenKind::KeywordFalse => "false".to_string(),
            TokenKind::KeywordFn => "fn".to_string(),
            TokenKind::KeywordUse => "use".to_string(),
            TokenKind::KeywordImport => "import".to_string(),
            _ => return None,
        })
    }

    fn render_contract_tokens(&self, start: usize, end: usize) -> Result<String, Diagnostic> {
        let mut rendered = Vec::new();
        for tok in &self.tokens[start..end] {
            let Some(fragment) = Self::token_fragment_for_contract(&tok.kind) else {
                return Err(Diagnostic {
                    message: "E_PARSE_FFI_CONTRACT_FIELD_001: invalid foreign contract field value"
                        .to_string(),
                    line: tok.line,
                    column: tok.column,
                });
            };
            rendered.push(fragment);
        }
        Ok(rendered.join(" ").trim().to_string())
    }

    fn parse_foreign_language(&mut self) -> Result<ForeignLanguage, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        let name = self.expect_ident()?.to_ascii_lowercase();
        match name.as_str() {
            "c" => Ok(ForeignLanguage::C),
            "cpp" | "cxx" => Ok(ForeignLanguage::Cpp),
            "rust" => Ok(ForeignLanguage::Rust),
            _ => Err(Diagnostic {
                message: format!(
                    "E_PARSE_FFI_SYMBOL_001: invalid foreign symbol language '{}'",
                    name
                ),
                line,
                column,
            }),
        }
    }

    fn parse_foreign_contract_value(&mut self) -> Result<String, Diagnostic> {
        let start = self.pos;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        while !self.at_eof() {
            if self.check(&TokenKind::Semicolon)
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
            {
                break;
            }

            match self.current().kind {
                TokenKind::LParen => paren_depth = paren_depth.saturating_add(1),
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth = bracket_depth.saturating_add(1),
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth = brace_depth.saturating_add(1),
                TokenKind::RBrace => {
                    if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 {
                        break;
                    }
                    brace_depth = brace_depth.saturating_sub(1);
                }
                _ => {}
            }
            self.advance();
        }

        let end = self.pos;
        let rendered = self.render_contract_tokens(start, end)?;
        if rendered.is_empty() {
            return self.error_here(
                "E_PARSE_FFI_CONTRACT_FIELD_001: invalid foreign contract field value",
            );
        }
        Ok(rendered)
    }

    fn parse_foreign_symbol_decl(&mut self) -> Result<ForeignSymbolDecl, Diagnostic> {
        let language = self.parse_foreign_language()?;
        if !self.match_ident_name("symbol") {
            return self
                .error_here("E_PARSE_FFI_SYMBOL_001: expected 'symbol' in foreign declaration");
        }

        let symbol = match self.advance().kind {
            TokenKind::String(value) => value,
            TokenKind::Identifier(value) => value,
            _ => {
                return self.error_here(
                    "E_PARSE_FFI_SYMBOL_001: foreign symbol expects string or identifier name",
                )
            }
        };

        self.expect_kind(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            let key = self.expect_ident()?;
            self.expect_kind(TokenKind::Colon)?;
            let value = self.parse_foreign_contract_value()?;
            self.expect_kind(TokenKind::Semicolon)?;
            fields.push(ForeignContractField { key, value });
        }
        self.expect_kind(TokenKind::RBrace)?;
        if self.check(&TokenKind::Semicolon) {
            self.expect_kind(TokenKind::Semicolon)?;
        }

        if fields.is_empty() {
            return self.error_here(
                "E_PARSE_FFI_CONTRACT_FIELD_001: foreign symbol requires contract fields",
            );
        }

        Ok(ForeignSymbolDecl {
            language,
            symbol,
            fields,
        })
    }

    fn parse_foreign_section(&mut self) -> Result<Section, Diagnostic> {
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            decls.push(self.parse_foreign_symbol_decl()?);
        }
        if decls.is_empty() {
            return self.error_here("E_PARSE_FOREIGN_BLOCK_001: empty foreign block");
        }
        Ok(Section::Foreign(decls))
    }

    fn parse_const_eval_decls(&mut self) -> Result<Vec<ConstEvalDecl>, Diagnostic> {
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            let visibility = self.parse_visibility_kw_opt();
            let name = self.expect_ident()?;
            let (ty, value) = if self.match_kind(&TokenKind::Colon) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Eq)?;
                let value = self.parse_expr()?;
                (ty, value)
            } else if self.match_kind(&TokenKind::Shift2) {
                let ty = self.expect_type_name()?;
                self.expect_projection_init_separator()?;
                let value = self.parse_expr()?;
                (ty, value)
            } else {
                return self.error_here("Expected ':' or '>>' in const/comptime declaration");
            };
            self.expect_kind(TokenKind::Semicolon)?;
            decls.push(ConstEvalDecl {
                visibility,
                name,
                ty,
                value,
            });
        }
        Ok(decls)
    }

    fn parse_comptime_allow_segment(&mut self) -> Result<String, Diagnostic> {
        match self.advance().kind {
            TokenKind::Identifier(s) => Ok(s),
            TokenKind::KeywordPrivate => Ok("private".into()),
            TokenKind::KeywordPublic => Ok("public".into()),
            TokenKind::KeywordRaw => Ok("raw".into()),
            TokenKind::Star => Ok("*".into()),
            _ => self.error_here("Expected comptime allow capability segment"),
        }
    }

    fn parse_comptime_allow(&mut self) -> Result<ComptimeAllow, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_ident_name("allow") {
            return self.error_here("Expected allow declaration in comptime unsafe block");
        }
        let mut parts = vec![self.parse_comptime_allow_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            if self.check(&TokenKind::Semicolon) {
                parts.push("*".into());
                break;
            }
            parts.push(self.parse_comptime_allow_segment()?);
        }
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(ComptimeAllow {
            capability: parts.join("."),
            line,
            column,
        })
    }

    fn parse_comptime_unsafe_blocks(
        &mut self,
        no_limit: bool,
    ) -> Result<Vec<ComptimeUnsafeBlock>, Diagnostic> {
        let mut blocks = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            let line = self.current().line;
            let column = self.current().column;
            let name = self.expect_ident()?;
            self.expect_kind(TokenKind::LBrace)?;
            let mut allows = Vec::new();
            while self.check_ident_name("allow") {
                allows.push(self.parse_comptime_allow()?);
            }
            let mut body = Vec::new();
            while !self.check(&TokenKind::RBrace) {
                body.push(self.parse_stmt()?);
            }
            self.expect_kind(TokenKind::RBrace)?;
            blocks.push(ComptimeUnsafeBlock {
                name,
                no_limit,
                allows,
                body,
                line,
                column,
            });
        }
        Ok(blocks)
    }

    fn parse_macro_section(&mut self) -> Result<Vec<MacroItem>, Diagnostic> {
        let mut items = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.check_word_ci("expand") {
                items.push(MacroItem::Invocation(self.parse_macro_invocation()?));
                continue;
            }
            items.push(MacroItem::Decl(self.parse_macro_decl()?));
        }
        Ok(items)
    }

    fn parse_high_risk_section(&mut self) -> Result<Vec<HighRiskItem>, Diagnostic> {
        let mut items = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) {
            if self.is_section_start()
                && !(self.check_word_ci("allow")
                    || self.check_word_ci("accept")
                    || self.check_word_ci("bridge")
                    || self.check_word_ci("report"))
            {
                break;
            }
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.check_word_ci("allow") {
                items.push(HighRiskItem::Allow(self.parse_high_risk_allow()?));
                continue;
            }
            if self.check_word_ci("accept") {
                items.push(HighRiskItem::Accept(self.parse_high_risk_accept()?));
                continue;
            }
            if self.check_word_ci("bridge") {
                items.push(HighRiskItem::Bridge(self.parse_high_risk_bridge()?));
                continue;
            }
            if self.check_word_ci("report") {
                items.push(HighRiskItem::Report(self.parse_high_risk_report_decl()?));
                continue;
            }
            return self.error_here("E_HIGH_RISK_POLICY_001: unknown risk policy clause");
        }
        Ok(items)
    }

    fn parse_high_risk_allow(&mut self) -> Result<HighRiskAllowance, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("allow") {
            return self.error_here("E_HIGH_RISK_CAPABILITY_REQUIRED_001: expected allow");
        }
        let capability = self.parse_high_risk_capability()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(HighRiskAllowance {
            capability,
            line,
            column,
        })
    }

    fn parse_high_risk_accept(&mut self) -> Result<HighRiskAccept, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("accept") {
            return self.error_here("E_HIGH_RISK_POLICY_001: expected accept");
        }
        let raw_kind = self.expect_ident()?;
        let kind = self.parse_high_risk_kind(raw_kind)?;
        let detail = match &self.current().kind {
            TokenKind::String(value) | TokenKind::Identifier(value) => {
                let out = value.clone();
                self.advance();
                Some(out)
            }
            _ => None,
        };
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(HighRiskAccept {
            kind,
            detail,
            line,
            column,
        })
    }

    fn parse_high_risk_bridge(&mut self) -> Result<HighRiskBridge, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("bridge") {
            return self.error_here("E_HIGH_RISK_BRIDGE_AUTHORITY_001: expected bridge");
        }
        let target = self.expect_ident()?;
        let mut reason = None;
        let mut authority = None;
        while !self.check(&TokenKind::Semicolon) {
            if self.match_word_ci("reason") {
                reason = self.match_string().or_else(|| {
                    if let TokenKind::Identifier(value) = self.current().kind.clone() {
                        self.advance();
                        Some(value)
                    } else {
                        None
                    }
                });
                if reason.is_none() {
                    return self.error_here("E_HIGH_RISK_BRIDGE_001: bridge reason expected");
                }
                continue;
            }
            if self.match_word_ci("authority") {
                authority = Some(self.parse_high_risk_capability()?);
                continue;
            }
            return self.error_here("E_HIGH_RISK_BRIDGE_001: unknown bridge clause");
        }
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(HighRiskBridge {
            target,
            reason,
            authority,
            line,
            column,
        })
    }

    fn parse_high_risk_report_decl(&mut self) -> Result<HighRiskReportDecl, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("report") {
            return self.error_here("E_HIGH_RISK_POLICY_001: expected report");
        }
        let name = self.expect_ident()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(HighRiskReportDecl { name, line, column })
    }

    fn parse_high_risk_capability(&mut self) -> Result<String, Diagnostic> {
        let mut parts = vec![self.parse_comptime_allow_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            if self.check(&TokenKind::Semicolon) {
                parts.push("*".into());
                break;
            }
            parts.push(self.parse_comptime_allow_segment()?);
        }
        Ok(parts.join("."))
    }

    fn parse_high_risk_kind(&mut self, raw: String) -> Result<HighRiskKind, Diagnostic> {
        let normalized = raw
            .chars()
            .filter(|ch| *ch != '_' && *ch != '-')
            .collect::<String>()
            .to_ascii_lowercase();
        let kind = match normalized.as_str() {
            "race" | "datarace" | "allowrace" => HighRiskKind::DataRace,
            "aliasmutation" => HighRiskKind::AliasMutation,
            "nobackpressure" => HighRiskKind::NoBackpressure,
            "streamnobackpressure" => HighRiskKind::StreamNoBackpressure,
            "channelnobackpressure" | "unboundedchannel" => HighRiskKind::ChannelNoBackpressure,
            "norollback" => HighRiskKind::NoRollback,
            "transactionpartialcommit" | "partialcommit" => HighRiskKind::TransactionPartialCommit,
            "partialmatch" => HighRiskKind::PartialMatch,
            "uncheckedunwrap" => HighRiskKind::UncheckedUnwrap,
            "uncheckedcast" => HighRiskKind::UncheckedCast,
            "effecterase" | "effecterasure" => HighRiskKind::EffectErasure,
            "privateleak" => HighRiskKind::PrivateLeak,
            "anyleak" => HighRiskKind::AnyLeak,
            "anyconstruction" => HighRiskKind::AnyConstruction,
            "constructionleak" => HighRiskKind::ConstructionLeak,
            "rawptr" => HighRiskKind::RawPtr,
            "rawaddr" => HighRiskKind::RawAddr,
            "rawsite" => HighRiskKind::RawSite,
            "unsafebridge" | "lowlevelbridge" => HighRiskKind::LowLevelBridge,
            "comptimeunsafe" => HighRiskKind::ComptimeUnsafe,
            "unsafemacro" => HighRiskKind::UnsafeMacro,
            "nolimit" => HighRiskKind::NoLimit,
            "unboundedloop" => HighRiskKind::UnboundedLoop,
            "unboundedrecursion" => HighRiskKind::UnboundedRecursion,
            "lowlevelpatch" => HighRiskKind::LowLevelPatch,
            "fragiledesign" => HighRiskKind::FragileDesign,
            "silentpointer" | "silentptr" | "trustmepointer" => HighRiskKind::SilentPointer,
            "heapregionview" | "regionview" | "heapregion" => HighRiskKind::HeapRegionView,
            "impossiblelowering" => HighRiskKind::ImpossibleLowering,
            "unresolvedsymbol" => HighRiskKind::UnresolvedSymbol,
            "invalidtype" => HighRiskKind::InvalidType,
            "fakesafety" => HighRiskKind::FakeSafety,
            "silentfallback" => HighRiskKind::SilentFallback,
            _ => {
                return self.error_here(format!(
                    "E_HIGH_RISK_POLICY_001: unknown risk kind '{}'",
                    raw
                ))
            }
        };
        Ok(kind)
    }

    fn parse_macro_decl(&mut self) -> Result<MacroDecl, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        let kind = if self.match_word_ci("declarative") {
            MacroKind::Declarative
        } else if self.match_word_ci("template") {
            MacroKind::Template
        } else if self.match_word_ci("attribute") || self.match_word_ci("attr") {
            MacroKind::Attribute
        } else if self.match_word_ci("derive") {
            MacroKind::Derive
        } else if self.match_word_ci("unsafe") {
            MacroKind::Unsafe
        } else {
            return self.error_here("E_HIGH_MACRO_PARSE_001: expected macro kind");
        };

        let no_limit = if matches!(kind, MacroKind::Unsafe) {
            self.match_word_ci("noLimit")
                || self.match_word_ci("nolimit")
                || self.match_word_ci("no_limit")
        } else {
            false
        };

        let name = self.expect_ident()?;
        let derive_target = if matches!(kind, MacroKind::Derive) && self.match_word_ci("for") {
            Some(self.expect_ident()?)
        } else {
            None
        };
        let params = if matches!(kind, MacroKind::Derive) {
            Vec::new()
        } else {
            self.parse_macro_params()?
        };
        self.expect_kind(TokenKind::LBrace)?;

        let mut decl = MacroDecl {
            kind,
            name,
            params,
            derive_target,
            no_limit,
            allows: Vec::new(),
            generated_items: Vec::new(),
            const_eval_bridges: Vec::new(),
            effects: Vec::new(),
            hygiene: MacroHygienePolicy {
                explicit_captures: Vec::new(),
                exports: Vec::new(),
                hygiene_escapes: Vec::new(),
                implicit_capture: None,
                private_metadata_access: false,
                any_leak: false,
            },
            recursion_depth: 0,
            expansion_steps: 1,
            generated_nodes: 0,
            nested_expansions: 0,
            source_map_required: true,
            invalid_generated_ast: false,
            generated_sema_fail: false,
            private_leak: false,
            any_leak: false,
            line,
            column,
        };

        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.check_word_ci("allow") {
                decl.allows.push(self.parse_macro_allow()?);
                continue;
            }
            if self.check_word_ci("generate") {
                decl.generated_items
                    .push(self.parse_macro_generated_item()?);
                continue;
            }
            if self.match_word_ci("effect") {
                decl.effects.push(self.parse_macro_effect_after_keyword()?);
                continue;
            }
            if self.match_word_ci("constEval") || self.match_word_ci("consteval") {
                let expr = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                decl.const_eval_bridges.push(expr);
                continue;
            }
            if self.match_word_ci("recursion") {
                decl.recursion_depth = self.expect_usize_literal("macro recursion depth")?;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("steps") {
                decl.expansion_steps = self.expect_usize_literal("macro expansion steps")?;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("generatedNodes") || self.match_word_ci("generated_nodes") {
                decl.generated_nodes = self.expect_usize_literal("macro generated nodes")?;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("nested") {
                decl.nested_expansions = self.expect_usize_literal("macro nested expansions")?;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("capture") {
                let name = self.expect_ident()?;
                decl.hygiene.explicit_captures.push(name);
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("implicitCapture") || self.match_word_ci("implicit_capture") {
                let name = self.expect_ident()?;
                decl.hygiene.implicit_capture = Some(name);
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("export") {
                let name = self.expect_ident()?;
                decl.hygiene.exports.push(name);
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("hygiene") {
                if self.match_word_ci("escape") {
                    let name = self.expect_ident()?;
                    decl.hygiene.hygiene_escapes.push(name);
                    self.expect_kind(TokenKind::Semicolon)?;
                    continue;
                }
                return self.error_here("E_HIGH_MACRO_HYGIENE_001: expected hygiene escape");
            }
            if self.match_word_ci("sourceMap") || self.match_word_ci("source_map") {
                if self.match_word_ci("missing") {
                    decl.source_map_required = false;
                    self.expect_kind(TokenKind::Semicolon)?;
                    continue;
                }
                return self.error_here("E_HIGH_MACRO_SOURCE_MAP_001: expected sourceMap missing");
            }
            if self.match_word_ci("invalidAst") || self.match_word_ci("invalid_ast") {
                decl.invalid_generated_ast = true;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("generatedSemaFail") || self.match_word_ci("generated_sema_fail")
            {
                decl.generated_sema_fail = true;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("privateLeak") || self.match_word_ci("private_leak") {
                decl.private_leak = true;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("anyLeak") || self.match_word_ci("any_leak") {
                decl.any_leak = true;
                decl.hygiene.any_leak = true;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_word_ci("privateMetadata") || self.match_word_ci("private_metadata") {
                decl.hygiene.private_metadata_access = true;
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            return self.error_here("E_HIGH_MACRO_PARSE_001: unknown macro clause");
        }
        self.expect_kind(TokenKind::RBrace)?;
        Ok(decl)
    }

    fn parse_macro_params(&mut self) -> Result<Vec<MacroParam>, Diagnostic> {
        let mut params = Vec::new();
        self.expect_kind(TokenKind::LParen)?;
        if !self.check(&TokenKind::RParen) {
            loop {
                let name = self.expect_ident()?;
                self.expect_kind(TokenKind::Colon)?;
                let kind_name = self.expect_ident()?;
                let kind = match kind_name.to_ascii_lowercase().as_str() {
                    "ident" | "identifier" => MacroParamKind::Ident,
                    "type" | "ty" => MacroParamKind::Type,
                    "expr" | "expression" => MacroParamKind::Expr,
                    "str" | "string" => MacroParamKind::Str,
                    "item" => MacroParamKind::Item,
                    _ => {
                        return self
                            .error_here("E_HIGH_MACRO_PATTERN_001: unknown macro parameter kind")
                    }
                };
                params.push(MacroParam { name, kind });
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }
        self.expect_kind(TokenKind::RParen)?;
        Ok(params)
    }

    fn parse_macro_invocation(&mut self) -> Result<MacroInvocation, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("expand") {
            return self.error_here("E_HIGH_MACRO_EXPANSION_001: expected expand invocation");
        }
        let name = self.expect_ident()?;
        self.expect_kind(TokenKind::LParen)?;
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_macro_arg()?);
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(MacroInvocation {
            name,
            args,
            line,
            column,
        })
    }

    fn parse_macro_arg(&mut self) -> Result<MacroArg, Diagnostic> {
        match &self.current().kind {
            TokenKind::String(value) => {
                let out = value.clone();
                self.advance();
                Ok(MacroArg::Str(out))
            }
            TokenKind::Identifier(_)
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Comma | TokenKind::RParen)
                ) =>
            {
                Ok(MacroArg::Ident(self.expect_ident()?))
            }
            TokenKind::KeywordStr
                if matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Comma | TokenKind::RParen)
                ) =>
            {
                Ok(MacroArg::Type(self.expect_type_name()?))
            }
            _ => Ok(MacroArg::Expr(self.parse_expr()?)),
        }
    }

    fn parse_macro_allow(&mut self) -> Result<MacroAllow, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("allow") {
            return self.error_here("E_HIGH_MACRO_UNDECLARED_EFFECT_001: expected allow");
        }
        let capability = self.parse_macro_capability()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(MacroAllow {
            capability,
            line,
            column,
        })
    }

    fn parse_macro_effect_after_keyword(&mut self) -> Result<MacroEffect, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        let capability = self.parse_macro_capability()?;
        let detail = if self.match_kind(&TokenKind::LParen) {
            let detail = match &self.current().kind {
                TokenKind::String(value) | TokenKind::Identifier(value) => {
                    let out = value.clone();
                    self.advance();
                    out
                }
                _ => {
                    return self
                        .error_here("E_HIGH_MACRO_COMPTIME_EFFECT_001: invalid effect detail")
                }
            };
            self.expect_kind(TokenKind::RParen)?;
            Some(detail)
        } else {
            None
        };
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(MacroEffect {
            capability,
            detail,
            line,
            column,
        })
    }

    fn parse_macro_capability(&mut self) -> Result<String, Diagnostic> {
        let mut parts = vec![self.parse_comptime_allow_segment()?];
        while self.match_kind(&TokenKind::Dot) {
            if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::LParen) {
                parts.push("*".into());
                break;
            }
            parts.push(self.parse_comptime_allow_segment()?);
        }
        let mut capability = parts.join(".");
        if let Some(stripped) = capability.strip_prefix("comptime.") {
            capability = stripped.to_string();
        }
        Ok(capability)
    }

    fn parse_macro_generated_item(&mut self) -> Result<MacroGeneratedItem, Diagnostic> {
        let line = self.current().line;
        let column = self.current().column;
        if !self.match_word_ci("generate") {
            return self.error_here("E_HIGH_MACRO_EXPANSION_001: expected generate");
        }
        let kind = if self.match_kind(&TokenKind::KeywordFn) || self.match_word_ci("function") {
            MacroGeneratedKind::Function
        } else if self.match_word_ci("type") {
            MacroGeneratedKind::Type
        } else if self.match_word_ci("const") {
            MacroGeneratedKind::Const
        } else if self.match_word_ci("impl") {
            MacroGeneratedKind::Impl
        } else if self.match_word_ci("annotation") || self.match_word_ci("attr") {
            MacroGeneratedKind::Annotation
        } else if self.match_word_ci("report") {
            MacroGeneratedKind::Report
        } else {
            return self.error_here("E_HIGH_MACRO_EXPANSION_001: unknown generated item kind");
        };
        let name_template = self.expect_ident()?;
        let mut ty = None;
        let mut value = None;
        if self.match_kind(&TokenKind::Colon) || self.match_kind(&TokenKind::Arrow) {
            ty = Some(self.expect_type_name()?);
        }
        if self.match_kind(&TokenKind::Eq) {
            value = Some(self.parse_expr()?);
        }
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(MacroGeneratedItem {
            kind,
            name_template,
            ty,
            value,
            export: false,
            explicit_capture: false,
            hygiene_escape: false,
            line,
            column,
        })
    }

    fn expect_usize_literal(&mut self, label: &str) -> Result<usize, Diagnostic> {
        match self.advance().kind {
            TokenKind::Number(raw) => raw.parse::<usize>().map_err(|_| Diagnostic {
                message: format!("E_HIGH_MACRO_EXPANSION_LIMIT_001: invalid {label}"),
                line: self.current().line,
                column: self.current().column,
            }),
            _ => self.error_here(format!(
                "E_HIGH_MACRO_EXPANSION_LIMIT_001: expected numeric {label}"
            )),
        }
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, Diagnostic> {
        let visibility = self.parse_visibility_kw_opt();
        let is_async = self.match_word_ci("async");
        if is_async {
            let _ = self.match_kind(&TokenKind::KeywordFn);
        }
        let name = self.expect_ident()?;
        let (mut generic_params, mut generic_bounds) = self.parse_optional_type_param_decls()?;
        let mut v2_input_definition = false;
        if self.check_double_colon() {
            self.expect_double_colon()?;
            if self.match_ident_name("in") {
                v2_input_definition = true;
            } else {
                self.expect_kind(TokenKind::KeywordFn)?;
            }
            if generic_params.is_empty() {
                let (params, bounds) = self.parse_optional_type_param_decls()?;
                generic_params = params;
                generic_bounds.extend(bounds);
            }
        }
        let _ = v2_input_definition;
        self.expect_kind(TokenKind::LParen)?;
        let params = self.parse_compact_function_params()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::Arrow)?;
        let return_type = self.expect_type_name()?;
        generic_bounds.extend(self.parse_optional_where_bounds()?);
        let effects = self.parse_optional_effects()?;

        self.expect_kind(TokenKind::LBrace)?;
        if self.match_kind(&TokenKind::KeywordPublic) {
            self.expect_kind(TokenKind::Colon)?;
        }

        let mut public_body = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.check_ident_name("stack")
                && matches!(
                    self.tokens.get(self.pos + 1).map(|token| &token.kind),
                    Some(TokenKind::LBrace)
                )
            {
                self.expect_ident()?;
                public_body.extend(self.parse_v2_stack_region()?);
                continue;
            }

            if self.match_kind(&TokenKind::KeywordPublic) {
                self.expect_kind(TokenKind::Colon)?;
                continue;
            }

            if self.match_kind(&TokenKind::KeywordRaw) {
                let raw_stmts = self.parse_block_stmts()?;
                public_body.extend(raw_stmts);
                continue;
            }

            public_body.push(self.parse_stmt()?);
        }

        self.expect_kind(TokenKind::RBrace)?;

        Ok(FunctionDecl {
            visibility,
            is_async,
            effects,
            name,
            generic_params,
            generic_bounds,
            params,
            return_type,
            public_body,
        })
    }

    fn parse_optional_effects(&mut self) -> Result<Vec<String>, Diagnostic> {
        if !self.match_word_ci("effects") {
            return Ok(Vec::new());
        }

        self.expect_kind(TokenKind::LParen)?;
        let mut effects = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let mut parts = vec![self.expect_ident()?];
                while self.match_kind(&TokenKind::Dot) {
                    parts.push(self.expect_ident()?);
                }
                effects.push(parts.join("."));
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }
        self.expect_kind(TokenKind::RParen)?;
        Ok(effects)
    }

    fn parse_optional_type_param_decls(
        &mut self,
    ) -> Result<(Vec<String>, Vec<GenericBound>), Diagnostic> {
        if !self.match_kind(&TokenKind::Less) {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut params = Vec::new();
        let mut bounds = Vec::new();
        loop {
            let is_const = self.match_ident_name("const");
            let param = self.expect_ident()?;
            params.push(param.clone());
            if is_const {
                self.expect_kind(TokenKind::Colon)?;
                let value_type = self.expect_type_name()?;
                bounds.push(GenericBound {
                    param,
                    bound: TypeName::Generic {
                        base: "const".to_string(),
                        args: vec![value_type],
                    },
                });
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                self.expect_gt()?;
                break;
            }
            if self.match_lt() {
                return self.error_here(
                    "E_HIGH_GENERIC_HKT_DEFERRED_001: higher-kinded generic parameters are deferred",
                );
            }
            if self.match_kind(&TokenKind::Colon) {
                loop {
                    let bound = self.expect_type_name()?;
                    bounds.push(GenericBound {
                        param: param.clone(),
                        bound,
                    });
                    if !self.match_kind(&TokenKind::Plus) {
                        break;
                    }
                }
            }
            if self.match_kind(&TokenKind::Comma) {
                continue;
            }
            self.expect_gt()?;
            break;
        }
        Ok((params, bounds))
    }

    fn parse_optional_where_bounds(&mut self) -> Result<Vec<GenericBound>, Diagnostic> {
        if !self.match_ident_name("where") {
            return Ok(Vec::new());
        }
        let mut bounds = Vec::new();
        loop {
            let param = self.expect_ident()?;
            self.expect_kind(TokenKind::Colon)?;
            loop {
                let bound = self.expect_type_name()?;
                bounds.push(GenericBound {
                    param: param.clone(),
                    bound,
                });
                if !self.match_kind(&TokenKind::Plus) {
                    break;
                }
            }
            if self.match_kind(&TokenKind::Comma) {
                continue;
            }
            break;
        }
        Ok(bounds)
    }

    fn parse_method_signature_decl(&mut self) -> Result<MethodSignatureDecl, Diagnostic> {
        if self.match_kind(&TokenKind::KeywordFn) {
            // official method requirement prefix
        }
        let name = self.expect_ident()?;
        self.expect_kind(TokenKind::LParen)?;
        let params = self.parse_compact_function_params()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::Arrow)?;
        let return_type = self.expect_type_name()?;
        Ok(MethodSignatureDecl {
            name,
            params,
            return_type,
        })
    }

    fn parse_compact_function_params(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) {
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if self.match_kind(&TokenKind::Colon) || self.match_kind(&TokenKind::Shift2) {
                    break;
                }
                self.expect_kind(TokenKind::Comma)?;
            }

            let ty = self.expect_type_name()?;
            params.extend(names.into_iter().map(|name| Param {
                name,
                ty: ty.clone(),
            }));

            if self.match_kind(&TokenKind::Comma) || self.match_kind(&TokenKind::Semicolon) {
                if self.check(&TokenKind::RParen) {
                    return self.error_here("function parameter list cannot end with a separator");
                }
                continue;
            }
            break;
        }
        Ok(params)
    }

    fn parse_module_section(&mut self) -> Result<ModuleDecl, Diagnostic> {
        self.match_ident_name("module");
        self.expect_kind(TokenKind::Colon)?;
        let mut path: Option<Vec<String>> = None;
        let mut package: Option<String> = None;

        while !self.is_section_start() && !self.check(&TokenKind::RBrace) {
            if self.match_ident_name("path") {
                self.expect_kind(TokenKind::Colon)?;
                path = Some(self.parse_high_level_path()?);
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            if self.match_ident_name("package") {
                self.expect_kind(TokenKind::Colon)?;
                package = Some(self.expect_ident()?);
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            return self.error_here("E_HIGH_MODULE_PARSE_001: expected module field");
        }

        let Some(path) = path else {
            return self.error_here("E_HIGH_MODULE_PARSE_001: module section requires path");
        };
        Ok(ModuleDecl { path, package })
    }

    fn parse_use_section(&mut self) -> Result<Vec<UseDecl>, Diagnostic> {
        if !self.match_ident_name("use") {
            self.expect_kind(TokenKind::KeywordUse)?;
        }
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.is_section_start() && !self.check(&TokenKind::RBrace) {
            let mut path = vec![self.expect_use_path_segment()?];
            let mut wildcard = false;
            while self.match_kind(&TokenKind::Dot) {
                if self.match_kind(&TokenKind::Star) {
                    wildcard = true;
                    break;
                }
                path.push(self.expect_use_path_segment()?);
            }
            let alias = if self.match_ident_name("as") {
                Some(self.expect_ident()?)
            } else {
                None
            };
            self.expect_kind(TokenKind::Semicolon)?;
            decls.push(UseDecl {
                path,
                wildcard,
                alias,
            });
        }
        Ok(decls)
    }

    fn parse_export_section(&mut self) -> Result<Vec<ExportDecl>, Diagnostic> {
        self.match_ident_name("export");
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.is_section_start()
                && matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Colon)
                )
            {
                break;
            }
            let kind_word = self.expect_use_path_segment()?;
            let kind = match kind_word.as_str() {
                "fn" => ExportKind::Function,
                "type" => ExportKind::Type,
                "enum" => ExportKind::Enum,
                "trait" => ExportKind::Trait,
                "interface" => ExportKind::Interface,
                "contract" => ExportKind::Contract,
                "module" => ExportKind::Module,
                "use" => ExportKind::Use,
                _ => return self.error_here("E_HIGH_EXPORT_PARSE_001: invalid export kind"),
            };
            let path = self.parse_high_level_path()?;
            let alias = if self.match_ident_name("as") {
                Some(self.expect_ident()?)
            } else {
                None
            };
            self.expect_kind(TokenKind::Semicolon)?;
            decls.push(ExportDecl { kind, path, alias });
        }
        Ok(decls)
    }

    fn parse_interface_section(&mut self) -> Result<Vec<InterfaceDecl>, Diagnostic> {
        self.match_ident_name("interface");
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            decls.push(self.parse_interface_decl_body()?);
        }
        if decls.is_empty() {
            return self.error_here("E_HIGH_INTERFACE_SECTION_001: interface section is empty");
        }
        Ok(decls)
    }

    fn parse_enum_section(&mut self) -> Result<Vec<EnumDecl>, Diagnostic> {
        self.match_ident_name("enum");
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            decls.push(self.parse_enum_decl_body()?);
        }
        if decls.is_empty() {
            return self.error_here("E_HIGH_ENUM_PARSE_001: enum section is empty");
        }
        Ok(decls)
    }

    fn parse_enum_decl(&mut self) -> Result<EnumDecl, Diagnostic> {
        self.match_ident_name("enum");
        self.parse_enum_decl_body()
    }

    fn parse_enum_decl_body(&mut self) -> Result<EnumDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let (generic_params, bounds) = self.parse_optional_type_param_decls()?;
        if !bounds.is_empty() {
            return self
                .error_here("E_HIGH_ENUM_PARSE_001: enum type parameters cannot declare bounds");
        }
        self.expect_kind(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            variants.push(self.parse_enum_variant()?);
        }
        self.expect_kind(TokenKind::RBrace)?;
        if variants.is_empty() {
            return self.error_here("E_HIGH_ENUM_PARSE_001: enum requires variants");
        }
        Ok(EnumDecl {
            name,
            generic_params,
            variants,
        })
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant, Diagnostic> {
        let name = self.expect_ident()?;
        let mut payload = Vec::new();
        if self.match_kind(&TokenKind::LParen) {
            if !self.check(&TokenKind::RParen) {
                loop {
                    let field_name = self.expect_ident()?;
                    self.expect_kind(TokenKind::Colon)?;
                    let ty = self.expect_type_name()?;
                    payload.push(EnumPayloadField {
                        name: field_name,
                        ty,
                    });
                    if self.match_kind(&TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
            }
            self.expect_kind(TokenKind::RParen)?;
        }
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(EnumVariant { name, payload })
    }

    fn parse_interface_decl(&mut self) -> Result<InterfaceDecl, Diagnostic> {
        self.match_ident_name("interface");
        self.parse_interface_decl_body()
    }

    fn parse_interface_decl_body(&mut self) -> Result<InterfaceDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let (generic_params, bounds) = self.parse_optional_type_param_decls()?;
        if !bounds.is_empty() {
            return self.error_here("E_HIGH_BOUND_PARSE_001: interface type parameters cannot declare bounds in Phase 4");
        }
        self.expect_kind(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let method = self.parse_method_signature_decl()?;
            self.expect_kind(TokenKind::Semicolon)?;
            methods.push(method);
        }
        self.expect_kind(TokenKind::RBrace)?;
        if methods.is_empty() {
            return self
                .error_here("E_HIGH_INTERFACE_PARSE_001: interface requires at least one method");
        }
        Ok(InterfaceDecl {
            name,
            generic_params,
            methods,
        })
    }

    fn parse_trait_section(&mut self) -> Result<Vec<TraitDecl>, Diagnostic> {
        self.match_ident_name("trait");
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            decls.push(self.parse_trait_decl_body()?);
        }
        if decls.is_empty() {
            return self.error_here("E_HIGH_TRAIT_SECTION_001: trait section is empty");
        }
        Ok(decls)
    }

    fn parse_trait_decl(&mut self) -> Result<TraitDecl, Diagnostic> {
        self.match_ident_name("trait");
        self.parse_trait_decl_body()
    }

    fn parse_trait_decl_body(&mut self) -> Result<TraitDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let (generic_params, bounds) = self.parse_optional_type_param_decls()?;
        if !bounds.is_empty() {
            return self.error_here(
                "E_HIGH_BOUND_PARSE_001: trait type parameters cannot declare bounds in Phase 4",
            );
        }
        self.expect_kind(TokenKind::LBrace)?;
        let mut associated_types = Vec::new();
        let mut required_methods = Vec::new();
        let mut default_methods = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.match_ident_name("type") {
                let name = self.expect_ident()?;
                self.expect_kind(TokenKind::Semicolon)?;
                associated_types.push(AssociatedTypeDecl { name });
                continue;
            }
            if self.check(&TokenKind::KeywordFn) {
                let save = self.pos;
                self.expect_kind(TokenKind::KeywordFn)?;
                if matches!(self.current().kind, TokenKind::Identifier(_))
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                        Some(TokenKind::Less)
                    )
                {
                    self.pos = save + 1;
                    let default = self.parse_function()?;
                    default_methods.push(default);
                    continue;
                }
                let signature = self.parse_method_signature_decl()?;
                if self.match_kind(&TokenKind::Semicolon) {
                    required_methods.push(signature);
                } else {
                    self.pos = save + 1;
                    let default = self.parse_function()?;
                    default_methods.push(default);
                }
                continue;
            }
            let method = self.parse_method_signature_decl()?;
            self.expect_kind(TokenKind::Semicolon)?;
            required_methods.push(method);
        }
        self.expect_kind(TokenKind::RBrace)?;
        if required_methods.is_empty() && default_methods.is_empty() {
            return self.error_here("E_HIGH_TRAIT_PARSE_001: trait requires at least one method");
        }
        Ok(TraitDecl {
            name,
            generic_params,
            associated_types,
            required_methods,
            default_methods,
        })
    }

    fn parse_impl_section(&mut self) -> Result<Vec<ImplDecl>, Diagnostic> {
        self.match_ident_name("impl");
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            decls.push(self.parse_impl_decl_body()?);
        }
        if decls.is_empty() {
            return self.error_here("E_HIGH_IMPL_SECTION_001: impl section is empty");
        }
        Ok(decls)
    }

    fn parse_impl_decl(&mut self) -> Result<ImplDecl, Diagnostic> {
        self.match_ident_name("impl");
        self.parse_impl_decl_body()
    }

    fn parse_impl_decl_body(&mut self) -> Result<ImplDecl, Diagnostic> {
        let trait_ref = self.expect_type_name()?;
        if !self.match_ident_name("for") {
            return self.error_here("E_HIGH_IMPL_PARSE_001: impl requires 'for'");
        }
        let for_type = self.expect_type_name()?;
        if self.match_ident_name("specialize") {
            return self.error_here(
                "E_HIGH_SPECIALIZATION_DEFERRED_001: specialization is deferred past Phase 5",
            );
        }
        self.expect_kind(TokenKind::LBrace)?;
        let mut associated_type_bindings = Vec::new();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.match_ident_name("type") {
                let name = self.expect_ident()?;
                self.expect_kind(TokenKind::Eq)?;
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Semicolon)?;
                associated_type_bindings.push(AssociatedTypeBinding { name, ty });
                continue;
            }
            self.match_kind(&TokenKind::KeywordFn);
            methods.push(self.parse_function()?);
        }
        self.expect_kind(TokenKind::RBrace)?;
        if methods.is_empty() {
            return self.error_here("E_HIGH_IMPL_PARSE_001: impl requires methods");
        }
        Ok(ImplDecl {
            trait_ref,
            for_type,
            associated_type_bindings,
            methods,
        })
    }

    fn parse_contract_section(&mut self) -> Result<Vec<ContractDecl>, Diagnostic> {
        self.match_ident_name("contract");
        self.expect_kind(TokenKind::Colon)?;
        let mut decls = Vec::new();
        while !self.at_eof() && !self.check(&TokenKind::RBrace) && !self.is_section_start() {
            if self.check(&TokenKind::Semicolon) {
                self.expect_kind(TokenKind::Semicolon)?;
                continue;
            }
            decls.push(self.parse_contract_decl_body()?);
        }
        if decls.is_empty() {
            return self.error_here("E_HIGH_CONTRACT_SECTION_001: contract section is empty");
        }
        Ok(decls)
    }

    fn parse_contract_decl(&mut self) -> Result<ContractDecl, Diagnostic> {
        self.match_ident_name("contract");
        self.parse_contract_decl_body()
    }

    fn parse_contract_decl_body(&mut self) -> Result<ContractDecl, Diagnostic> {
        let name = self.expect_ident()?;
        let (generic_params, bounds) = self.parse_optional_type_param_decls()?;
        self.expect_kind(TokenKind::LBrace)?;
        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        let mut invariants = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.match_ident_name("require") {
                requires.push(self.parse_expr()?);
                self.expect_kind(TokenKind::Semicolon)?;
            } else if self.match_ident_name("ensure") {
                ensures.push(self.parse_expr()?);
                self.expect_kind(TokenKind::Semicolon)?;
            } else if self.match_ident_name("invariant") {
                invariants.push(self.parse_expr()?);
                self.expect_kind(TokenKind::Semicolon)?;
            } else {
                return self
                    .error_here("E_HIGH_CONTRACT_PARSE_001: expected require/ensure/invariant");
            }
        }
        self.expect_kind(TokenKind::RBrace)?;
        if requires.is_empty() && ensures.is_empty() && invariants.is_empty() {
            return self.error_here("E_HIGH_CONTRACT_PARSE_001: contract requires conditions");
        }
        Ok(ContractDecl {
            name,
            generic_params,
            bounds,
            requires,
            ensures,
            invariants,
        })
    }

    fn parse_machine_atoms_section(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut items = Vec::new();

        while !self.check(&TokenKind::RBrace)
            && (!self.is_section_start()
                || (self.check(&TokenKind::KeywordFlag)
                    && !matches!(
                        self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                        Some(TokenKind::Colon)
                    )))
        {
            let start = self.pos;
            while !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
                self.advance();
            }
            if self.check(&TokenKind::RBrace) {
                return self.error_here("Expected Semicolon");
            }

            let end = self.pos;
            self.expect_kind(TokenKind::Semicolon)?;

            let line_tokens = &self.tokens[start..end];

            if let Some(Token {
                kind: TokenKind::Identifier(name),
                ..
            }) = line_tokens.first()
            {
                if matches!(line_tokens.get(1).map(|t| &t.kind), Some(TokenKind::Colon)) {
                    items.push(name.clone());
                    continue;
                }
            }

            if matches!(
                line_tokens.first().map(|t| &t.kind),
                Some(TokenKind::KeywordFlag)
            ) {
                if let Some(Token {
                    kind: TokenKind::Identifier(name),
                    ..
                }) = line_tokens.get(1)
                {
                    items.push(name.clone());
                    continue;
                }
            }

            let text = line_tokens
                .iter()
                .map(|t| match &t.kind {
                    TokenKind::Identifier(s) => s.clone(),
                    TokenKind::Number(s) => s.clone(),
                    TokenKind::String(s) => format!("\"{}\"", s),
                    TokenKind::Comma => ",".into(),
                    TokenKind::Dot => ".".into(),
                    TokenKind::At => "@".into(),
                    TokenKind::Star => "*".into(),
                    TokenKind::Colon => ":".into(),
                    TokenKind::Arrow => "->".into(),
                    TokenKind::Arrow2 => "->>".into(),
                    TokenKind::Shift2 => ">>".into(),
                    TokenKind::Shift3 => ">>>".into(),
                    TokenKind::ShiftLeft => "<<".into(),
                    TokenKind::BangShift => "!>".into(),
                    TokenKind::Bang => "!".into(),
                    TokenKind::Pipe => "|".into(),
                    TokenKind::Amp => "&".into(),
                    TokenKind::Caret => "^".into(),
                    TokenKind::Tilde => "~".into(),
                    TokenKind::Plus => "+".into(),
                    TokenKind::Minus => "-".into(),
                    TokenKind::Slash => "/".into(),
                    TokenKind::Percent => "%".into(),
                    TokenKind::AndAnd => "&&".into(),
                    TokenKind::OrOr => "||".into(),
                    TokenKind::Less => "<".into(),
                    TokenKind::Greater => ">".into(),
                    TokenKind::GreaterEq => ">=".into(),
                    TokenKind::LessEq => "<=".into(),
                    TokenKind::EqEq => "==".into(),
                    TokenKind::NotEq => "!=".into(),
                    TokenKind::LParen => "(".into(),
                    TokenKind::RParen => ")".into(),
                    TokenKind::LBracket => "[".into(),
                    TokenKind::RBracket => "]".into(),
                    TokenKind::LBrace => "{".into(),
                    TokenKind::RBrace => "}".into(),
                    TokenKind::KeywordReg => "reg".into(),
                    TokenKind::KeywordFlag => "flag".into(),
                    TokenKind::KeywordLabel => "label".into(),
                    TokenKind::KeywordInst => "inst".into(),
                    TokenKind::KeywordFinal => "final".into(),
                    _ => format!("{:?}", t.kind),
                })
                .collect::<Vec<_>>()
                .join(" ");

            items.push(text.trim().to_string());
        }

        Ok(items)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        // Compatibility directives inside binary section surface:
        //   profile stable;
        //   target x86_64;
        // These are normalized into inert string expression statements so the
        // official AST and runtime pipeline stay stable.
        if self.check_ident_name("profile")
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Identifier(_))
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::Semicolon)
            )
        {
            self.expect_ident()?;
            let value = self.expect_ident()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::String(format!("profile {}", value))));
        }

        if self.check_ident_name("target")
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Identifier(_))
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::Semicolon)
            )
        {
            self.expect_ident()?;
            let value = self.expect_ident()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::String(format!("target {}", value))));
        }

        // Compatibility dotted symbolic statement (no call):
        //   root.policy.bind;
        //   root.memory.resolve;
        // Normalized into inert string expression statements.
        if self.peek_is_ident() && self.check_next_kind(&TokenKind::Dot) {
            let save = self.pos;
            let mut path = Vec::new();
            path.push(self.expect_ident()?);

            let parse_path_segment = |parser: &mut Self| -> Option<String> {
                match &parser.current().kind {
                    TokenKind::Identifier(s) => {
                        let out = s.clone();
                        parser.advance();
                        Some(out)
                    }
                    TokenKind::KeywordBind => {
                        parser.advance();
                        Some("bind".to_string())
                    }
                    TokenKind::KeywordBinary => {
                        parser.advance();
                        Some("binary".to_string())
                    }
                    _ => None,
                }
            };

            let mut valid = true;
            while self.match_kind(&TokenKind::Dot) {
                if let Some(segment) = parse_path_segment(self) {
                    path.push(segment);
                } else {
                    valid = false;
                    break;
                }
            }

            if valid && self.match_kind(&TokenKind::Semicolon) {
                return Ok(Stmt::Expr(Expr::String(path.join("."))));
            }

            self.pos = save;
        }

        // Compatibility probe invocation:
        //   @probe(checkFn);
        //   @probe(checkFn(@x));
        // Normalized into direct call execution statements.
        if self.check(&TokenKind::At)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::KeywordProbe)
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::LParen)
            )
        {
            self.expect_kind(TokenKind::At)?;
            self.expect_kind(TokenKind::KeywordProbe)?;
            self.expect_kind(TokenKind::LParen)?;
            let target = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semicolon)?;

            let normalized = match target {
                Expr::Call { .. } | Expr::QualifiedCall { .. } | Expr::MethodCall { .. } => target,
                Expr::Identifier(name) => Expr::Call {
                    callee: name,
                    args: Vec::new(),
                },
                Expr::ContextRef(ctx) if ctx.path.len() == 1 => Expr::Call {
                    callee: ctx.path[0].clone(),
                    args: Vec::new(),
                },
                Expr::ContextRef(ctx) => Expr::QualifiedCall {
                    path: ctx.path,
                    args: Vec::new(),
                },
                _ => return self.error_here("probe target must be function reference or call"),
            };

            return Ok(Stmt::Expr(normalized));
        }

        if self.match_ident_name("var") {
            let name = self.expect_ident()?;
            let ty = if self.match_kind(&TokenKind::Colon) {
                self.expect_type_name()?
            } else if self.match_kind(&TokenKind::Shift2) {
                self.expect_type_name()?
            } else {
                Self::inferred_projection_type()
            };
            if !(self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)) {
                return self.error_here("E_MATERIAL_V2_VAR_001: inferred declaration expects '<-'");
            }
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Projection {
                target: AssignTarget::Context(ContextRef::plain(vec![name])),
                ty,
                value,
            });
        }

        if self.match_ident_name("print") {
            if self.match_kind(&TokenKind::Dot) {
                return self.error_here(
                    "E_PRINT_SURFACE_001: print.* was removed; use print(...) or println(...)",
                );
            }

            self.expect_kind(TokenKind::LParen)?;
            if self.check(&TokenKind::RParen) {
                return self.error_here("print expects 1 argument");
            }
            let value = self.parse_expr()?;
            if self.check(&TokenKind::Comma) {
                return self.error_here("print expects exactly 1 argument");
            }
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Print {
                kind: PrintKind::Std,
                value,
            });
        }

        if self.match_ident_name("println") {
            self.expect_kind(TokenKind::LParen)?;
            let value = if self.check(&TokenKind::RParen) {
                Expr::String(String::new())
            } else {
                self.parse_expr()?
            };
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Print {
                kind: PrintKind::Ln,
                value,
            });
        }

        if self.match_word_ci("destructure") {
            return self.parse_destructure_stmt_after_keyword();
        }

        if self.match_word_ci("constructure") {
            return self.parse_constructure_stmt_after_keyword();
        }

        if self.match_word_ci("yield") {
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::Call {
                callee: "yield".into(),
                args: vec![value],
            }));
        }

        if self.match_kind(&TokenKind::KeywordReturn) {
            let expr = if self.check(&TokenKind::Semicolon) {
                // `Stmt::Return` is the canonical control-flow terminator in
                // every backend. Void returns carry a discarded unit marker
                // instead of growing a parallel AST/IR instruction family.
                Expr::Bool(true)
            } else {
                self.parse_expr()?
            };
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Return(expr));
        }

        if self.match_kind(&TokenKind::KeywordDev) {
            let errors = self.parse_dev_error_list()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Dev(errors));
        }

        if self.peek_is_ident() && self.check_next_kind(&TokenKind::Colon) {
            let save = self.pos;
            let label = self.expect_ident()?;
            self.expect_kind(TokenKind::Colon)?;
            if self.check(&TokenKind::KeywordWhile)
                || self.check_word_ci("for")
                || self.check_word_ci("loop")
                || self.check_word_ci("repeat")
                || self.check_word_ci("do")
                || self.check_word_ci("switch")
                || self.check_word_ci("match")
                || self.check_word_ci("guard")
                || self.check_word_ci("select")
                || self.check_word_ci("same")
                || self.check_word_ci("each")
            {
                let stmt = self.parse_stmt()?;
                return Ok(Stmt::Labeled {
                    label,
                    stmt: Box::new(stmt),
                });
            }
            self.pos = save;
        }

        if self.match_kind(&TokenKind::KeywordIf) {
            return self.parse_if_stmt_after_keyword();
        }

        if self.match_word_ci("notif") {
            return self.parse_notif_stmt_after_word();
        }

        if self.match_word_ci("when") {
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let then_block = self.parse_block_stmts()?;
            return Ok(Stmt::If {
                condition,
                then_block,
                else_block: Vec::new(),
            });
        }

        if self.match_word_ci("unless") {
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let body = self.parse_block_stmts()?;
            return Ok(Stmt::Unless { condition, body });
        }

        if self.match_kind(&TokenKind::KeywordWhile) {
            if self.match_word_ci("let") {
                let pattern = self.parse_pattern()?;
                if !(self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)) {
                    return self.error_here("E_MATERIAL_V2_WHILE_LET_001: while let expects '<-'");
                }
                let value = self.parse_expr()?;
                let body = self.parse_block_stmts()?;
                return Ok(Stmt::Loop {
                    body: vec![Stmt::PatternMatch {
                        value,
                        arms: vec![
                            MatchArm {
                                pattern,
                                guard: None,
                                body,
                            },
                            MatchArm {
                                pattern: Pattern::Wildcard,
                                guard: None,
                                body: vec![Stmt::Break { label: None }],
                            },
                        ],
                    }],
                });
            }
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            let body = self.parse_block_stmts()?;
            return Ok(Stmt::While { condition, body });
        }

        if self.match_word_ci("do") {
            let body = self.parse_block_stmts()?;
            if !self.match_word_ci("while") {
                return self.error_here("Expected while after do block");
            }
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::DoWhile { body, condition });
        }

        if self.match_word_ci("async") {
            if !self.match_word_ci("for") {
                return self.error_here("Expected 'for' after async in async for-loop");
            }
            return Ok(Stmt::Labeled {
                label: "__ncc_loop_flavor_async".to_string(),
                stmt: Box::new(self.parse_for_stmt()?),
            });
        }

        if self.match_word_ci("parallel") {
            if !self.match_word_ci("for") {
                return self.error_here("Expected 'for' after parallel in parallel for-loop");
            }
            return Ok(Stmt::Labeled {
                label: "__ncc_loop_flavor_parallel".to_string(),
                stmt: Box::new(self.parse_for_stmt()?),
            });
        }

        if self.match_word_ci("comptime") {
            if !self.match_word_ci("for") {
                return self.error_here("Expected 'for' after comptime in comptime for-loop");
            }
            return Ok(Stmt::Labeled {
                label: "__ncc_loop_flavor_comptime".to_string(),
                stmt: Box::new(self.parse_for_stmt()?),
            });
        }

        if self.match_word_ci("for") {
            return self.parse_for_stmt();
        }

        if self.match_word_ci("loop") {
            let body = self.parse_block_stmts()?;
            return Ok(Stmt::Loop { body });
        }

        if self.match_word_ci("repeat") {
            let body = self.parse_block_stmts()?;
            if !self.match_word_ci("until") {
                return self.error_here("Expected until after repeat block");
            }
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::RepeatUntil { body, condition });
        }

        let starts_same_loop = self.check_word_ci("same")
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Identifier(word))
                    if word.eq_ignore_ascii_case("for") || word.eq_ignore_ascii_case("in")
            );
        if starts_same_loop {
            self.advance();
            if self.match_word_ci("for") {
                self.expect_kind(TokenKind::LParen)?;
                let count = self.parse_expr()?;
                self.expect_kind(TokenKind::RParen)?;
                let body = self.parse_block_stmts()?;
                return Ok(Stmt::RepeatCount { count, body });
            }
            if self.match_word_ci("in") {
                let item = self.expect_ident()?;
                self.expect_kind(TokenKind::Colon)?;
                let iterable = if self.peek_is_ident() && self.check_next_kind(&TokenKind::LBrace) {
                    Expr::Identifier(self.expect_ident()?)
                } else {
                    self.parse_expr()?
                };
                let body = self.parse_block_stmts()?;
                return Ok(Stmt::SameIn {
                    item,
                    iterable,
                    body,
                });
            }
            return self.error_here("Expected 'for' or 'in' after same");
        }

        if self.match_word_ci("each") {
            let item = self.expect_ident()?;
            if !self.match_word_ci("in") {
                return self.error_here("Expected 'in' in each-loop");
            }
            let iterable = self.parse_expr()?;
            if !self.match_word_ci("do") {
                return self.error_here("Expected 'do' in each-loop");
            }
            let body = self.parse_block_stmts()?;
            if !self.match_word_ci("while") {
                return self.error_here("Expected while after each do-block");
            }
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::EachDoWhile {
                item,
                iterable,
                body,
                condition,
            });
        }

        if self.match_word_ci("switch") {
            return self.parse_switch_stmt();
        }

        if self.match_word_ci("match") {
            return self.parse_match_stmt();
        }

        if self.match_word_ci("guard") {
            let delimited = if self.check(&TokenKind::LParen) {
                let tags = self.tokens[self.pos..].iter().map(|token| match token.kind {
                    TokenKind::LParen => 7,
                    TokenKind::RParen => 8,
                    TokenKind::KeywordElse => 71,
                    _ => 0,
                }).collect::<Vec<i32>>();
                ncc_stage0_bridge::guard_condition(&tags)
            } else { false };
            let condition = if delimited {
                self.expect_kind(TokenKind::LParen)?;
                let condition = self.parse_expr()?;
                self.expect_kind(TokenKind::RParen)?;
                condition
            } else {
                self.parse_expr()?
            };
            if !self.match_kind(&TokenKind::KeywordElse) {
                return self.error_here(
                    "E_CONDITION_GUARD_ELSE_MISSING_001: guard requires else/failure path",
                );
            }
            let else_block = self.parse_block_or_single_stmt()?;
            return Ok(Stmt::GuardElse {
                condition,
                else_block,
            });
        }

        if self.match_word_ci("defer") || self.match_kind(&TokenKind::KeywordFinal) {
            let body = self.parse_block_or_single_stmt()?;
            return Ok(Stmt::Defer { body });
        }

        if self.check_word_ci("select") && self.check_next_kind(&TokenKind::LBrace) {
            self.advance();
            return self.parse_select_stmt();
        }

        if self.match_word_ci("break") {
            let label = if self.check(&TokenKind::Semicolon) {
                None
            } else {
                Some(self.expect_ident()?)
            };
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Break { label });
        }

        if self.match_word_ci("continue") {
            let label = if self.check(&TokenKind::Semicolon) {
                None
            } else {
                Some(self.expect_ident()?)
            };
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Continue { label });
        }

        // Kernel/system MMIO base declaration. The compact source form keeps
        // the address binding explicit while the AST retains the canonical
        // `addr` projection used by sema and every backend.
        if self.in_kernel_or_system_context()
            && self.peek_is_ident()
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Identifier(word)) if word.eq_ignore_ascii_case("at")
            )
        {
            let name = self.expect_ident()?;
            self.expect_ident()?;
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Projection {
                target: AssignTarget::Context(ContextRef::plain(vec![name])),
                ty: TypeName::Simple("addr".to_string()),
                value,
            });
        }

        if self.in_kernel_or_system_context() {
            if let Some(width) = self.compact_mmio_write_width(true) {
                self.expect_kind(TokenKind::Star)?;
                return self.parse_compact_mmio_write(width);
            }
            if let Some(width) = self.compact_mmio_write_width(false) {
                return self.parse_compact_mmio_write(width);
            }
        }

        if self.check_ident_name("store") {
            self.expect_ident()?;
            let ty = self.expect_material_access_type()?;
            self.expect_kind(TokenKind::LBracket)?;
            let address = self.parse_expr()?;
            self.expect_kind(TokenKind::RBracket)?;
            self.expect_kind(TokenKind::LessDash)?;
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::Call {
                callee: format!("ptrWrite<{}>", Self::type_surface(&ty)),
                args: vec![address, value],
            }));
        }

        if self.check_ident_name("copy")
            && !matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(
                    TokenKind::Shift2
                        | TokenKind::LessDash
                        | TokenKind::Eq
                        | TokenKind::PlusEq
                        | TokenKind::MinusEq
                        | TokenKind::Dot
                )
            )
        {
            self.expect_ident()?;
            let count = self.parse_expr()?;
            if !self.match_ident_name("bytes") {
                return self.error_here("E_V2_COPY_BYTES_001: copy requires the 'bytes' unit");
            }
            self.expect_kind(TokenKind::LBracket)?;
            let destination = self.parse_expr()?;
            self.expect_kind(TokenKind::RBracket)?;
            self.expect_kind(TokenKind::LessDash)?;
            self.expect_kind(TokenKind::LBracket)?;
            let source = self.parse_expr()?;
            self.expect_kind(TokenKind::RBracket)?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::Call {
                callee: "memcpy".to_string(),
                args: vec![destination, source, count],
            }));
        }

        if self.check_ident_name("fill")
            && !matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::LBracket | TokenKind::LParen)
            )
        {
            self.expect_ident()?;
            let count = self.parse_expr()?;
            if !self.match_ident_name("bytes") {
                return self.error_here("E_V2_FILL_BYTES_001: fill requires the 'bytes' unit");
            }
            self.expect_kind(TokenKind::LBracket)?;
            let destination = self.parse_expr()?;
            self.expect_kind(TokenKind::RBracket)?;
            self.expect_kind(TokenKind::LessDash)?;
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::Call {
                callee: "memset".to_string(),
                args: vec![destination, value, count],
            }));
        }

        if self.match_kind(&TokenKind::Star) {
            let receiver = self.parse_expr()?;
            if !(self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)) {
                return self.error_here("E_MATERIAL_V2_STORE_001: expected '<-' after raw address");
            }
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Expr(Expr::MethodCall {
                receiver: Box::new(receiver),
                method: "write".to_string(),
                args: vec![value],
            }));
        }

        if self.match_kind(&TokenKind::KeywordRef) {
            self.expect_kind(TokenKind::KeywordMain)?;
            self.expect_kind(TokenKind::Arrow2)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::MainRefFn {
                kind: MainKind::Ref,
                fn_ref,
            });
        }

        if self.match_kind(&TokenKind::KeywordSkkMain) {
            self.expect_kind(TokenKind::Arrow2)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::MainRefFn {
                kind: MainKind::Skk,
                fn_ref,
            });
        }

        if self.match_ident_name("skk") {
            self.expect_kind(TokenKind::KeywordMain)?;
            self.expect_kind(TokenKind::Arrow2)?;
            let fn_ref = self.parse_main_fn_ref_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::MainRefFn {
                kind: MainKind::Skk,
                fn_ref,
            });
        }

        if self.check(&TokenKind::At) {
            let save = self.pos;

            if let Ok(fn_ref) = self.parse_fn_ref_expr() {
                if self.match_kind(&TokenKind::KeywordLy) {
                    self.expect_kind(TokenKind::Semicolon)?;
                    return Ok(Stmt::FunctionRelation {
                        fn_ref,
                        kind: FunctionRelationKind::Legacy,
                    });
                }
                if self.match_kind(&TokenKind::KeywordCnt) {
                    self.expect_kind(TokenKind::Semicolon)?;
                    return Ok(Stmt::FunctionRelation {
                        fn_ref,
                        kind: FunctionRelationKind::Connect,
                    });
                }
                if self.match_kind(&TokenKind::KeywordCut) {
                    self.expect_kind(TokenKind::Semicolon)?;
                    return Ok(Stmt::FunctionRelation {
                        fn_ref,
                        kind: FunctionRelationKind::Concurrent,
                    });
                }
            }

            self.pos = save;

            if let Some(stmt) = self.try_parse_method_array_field_projection_stmt()? {
                return Ok(stmt);
            }

            self.pos = save;

            let (target, indices, indexed_field) = self.parse_context_ref_with_indices()?;
            if self.match_kind(&TokenKind::LessDash) {
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if let Some(fields) = indexed_field {
                        args.push(Expr::ContextRef(ContextRef::plain(fields)));
                        "arrayFieldWrite"
                    } else {
                        "arrayWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty: Self::inferred_projection_type(),
                    value,
                });
            }

            if let Some(op) = self.match_compound_assignment_op() {
                if !indices.is_empty() {
                    return self.error_here(
                        "compound assignment expects a mutable symbol or field target; use explicit arrayWrite for indexed writes",
                    );
                }
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Self::compound_update_projection(target, op, value));
            }

            if self.match_kind(&TokenKind::MinusMinus) {
                if !indices.is_empty() {
                    return self.error_here(
                        "decrement expects a mutable symbol or field target; use explicit arrayWrite for indexed writes",
                    );
                }
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Self::compound_update_projection(
                    target,
                    BinaryOp::Minus,
                    Expr::Number("1".to_string()),
                ));
            }

            if self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq) {
                if !indices.is_empty() {
                    return self.error_here(
                        "compact assignment expects a simple symbol target; use official projection for indexed writes",
                    );
                }
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty: Self::inferred_projection_type(),
                    value,
                });
            }

            if self.match_kind(&TokenKind::Shift2) {
                if indices.is_empty()
                    && self.check_ident_name("site")
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                        Some(TokenKind::Dot)
                    )
                {
                    self.match_ident_name("site");
                    self.expect_kind(TokenKind::Dot)?;
                    let field = self.expect_ident()?;
                    self.expect_kind(TokenKind::Dot)?;
                    if !self.match_ident_name("print") {
                        return self.error_here("site projection expects '.print' suffix");
                    }
                    self.expect_kind(TokenKind::Semicolon)?;
                    let callee = match field.as_str() {
                        "addr" => "sitePrintAddr",
                        "region" => "sitePrintRegion",
                        "class" => "sitePrintClass",
                        "type" => "sitePrintType",
                        "state" => "sitePrintState",
                        "owner" => "sitePrintOwner",
                        "size" => "sitePrintSize",
                        "align" => "sitePrintAlign",
                        _ => {
                            return self.error_here(
                                "invalid site projection field (expected: addr | region | class | type | state | owner | size | align)",
                            );
                        }
                    };
                    return Ok(Stmt::Print {
                        kind: PrintKind::Std,
                        value: Expr::Call {
                            callee: callee.to_string(),
                            args: vec![Expr::ContextRef(target)],
                        },
                    });
                }
                let ty = self.expect_type_name()?;
                if indices.is_empty() && self.match_kind(&TokenKind::Semicolon) {
                    return Ok(Stmt::Projection {
                        target: AssignTarget::Context(target),
                        ty,
                        value: Expr::Call {
                            callee: "__nC_uninit".to_string(),
                            args: Vec::new(),
                        },
                    });
                }
                self.expect_projection_init_separator()?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if let Some(fields) = indexed_field {
                        args.push(Expr::ContextRef(ContextRef::plain(fields)));
                        "arrayFieldWrite"
                    } else {
                        "arrayWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }

            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if let Some(fields) = indexed_field {
                        args.push(Expr::ContextRef(ContextRef::plain(fields)));
                        "arrayFieldWrite"
                    } else {
                        "arrayWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::StrictCheck {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }

            self.pos = save;
        }

        if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "mem")
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::LBracket)
            )
        {
            let save = self.pos;
            self.expect_ident()?;
            self.expect_kind(TokenKind::LBracket)?;
            let addr = self.parse_expr()?;
            self.expect_kind(TokenKind::RBracket)?;

            if self.match_kind(&TokenKind::Shift2) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Mem {
                        addr: Box::new(addr),
                    },
                    ty,
                    value,
                });
            }

            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Stmt::StrictCheck {
                    target: AssignTarget::Mem {
                        addr: Box::new(addr),
                    },
                    ty,
                    value,
                });
            }

            self.pos = save;
        }

        let storage_start = self.pos;
        let storage = self.parse_storage_contract_prefix()?;
        if self.pos != storage_start {
            if !self.peek_is_ident() {
                return self.error_here(
                    "E_STORAGE_FUNCTION_LOCAL_001: storage qualifiers require a local variable name",
                );
            }
            let target = self.parse_plain_context_ref()?;
            if target.path.len() != 1 {
                return self.error_here(
                    "E_STORAGE_FUNCTION_LOCAL_001: function-static storage requires a simple local name",
                );
            }
            self.expect_kind(TokenKind::Shift2)?;
            let value_type = self.expect_type_name()?;
            self.expect_projection_init_separator()?;
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Stmt::Projection {
                target: AssignTarget::Context(target),
                ty: attach_explicit_storage_contract(value_type, storage),
                value,
            });
        }
        self.pos = storage_start;

        if self.peek_is_ident() {
            let save = self.pos;
            match self.parse_expr() {
                Ok(Expr::Index { target, index }) if self.match_kind(&TokenKind::Eq) => {
                    let value = self.parse_expr()?;
                    self.expect_kind(TokenKind::Semicolon)?;
                    let receiver = match *target {
                        Expr::Identifier(name) => {
                            Box::new(Expr::ContextRef(ContextRef::plain(vec![name])))
                        }
                        other => Box::new(other),
                    };
                    let mut args = match *index {
                        Expr::ArrayLiteral(items) => items,
                        other => vec![other],
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver,
                        method: "arrayWrite".to_string(),
                        args,
                    }));
                }
                _ => {
                    self.pos = save;
                }
            }
        }

        if self.peek_is_ident() {
            let save = self.pos;
            let target = self.parse_plain_context_ref()?;
            let mut indices = Vec::new();
            while self.match_kind(&TokenKind::LBracket) {
                if self.check(&TokenKind::RBracket) {
                    return self.error_here("invalid index expression");
                }
                loop {
                    indices.push(self.parse_expr()?);
                    if !self.match_kind(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_kind(TokenKind::RBracket)?;
            }
            let mut indexed_fields = Vec::new();
            while !indices.is_empty() && self.match_kind(&TokenKind::Dot) {
                indexed_fields.push(self.expect_ident()?);
            }

            if indices.is_empty()
                && self.check(&TokenKind::Colon)
                && matches!(
                    self.tokens.get(self.pos + 1).map(|token| &token.kind),
                    Some(TokenKind::Eq)
                )
            {
                return self.error_here(
                    "E_ASSIGN_INIT_REMOVED_001: ':=' was removed; use '<-' for initialization and writes",
                );
            }

            if indices.is_empty() && self.check(&TokenKind::Colon) && !self.check_double_colon() {
                self.expect_kind(TokenKind::Colon)?;
                let ty = if self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)
                {
                    Self::inferred_projection_type()
                } else {
                    let ty = self.expect_type_name()?;
                    if !(self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq)) {
                        return self.error_here(
                            "E_MATERIAL_V2_LOCAL_001: local place initializer expects '<-'",
                        );
                    }
                    ty
                };
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }

            if self.match_kind(&TokenKind::LessDash) {
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if indexed_fields.is_empty() {
                        "arrayWrite"
                    } else {
                        args.push(Expr::ContextRef(ContextRef::plain(indexed_fields)));
                        "arrayFieldWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty: Self::inferred_projection_type(),
                    value,
                });
            }

            if self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq) {
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if indexed_fields.is_empty() {
                        "arrayWrite"
                    } else {
                        args.push(Expr::ContextRef(ContextRef::plain(indexed_fields)));
                        "arrayFieldWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty: Self::inferred_projection_type(),
                    value,
                });
            }

            if indices.is_empty() {
                if let Some(op) = self.match_compound_assignment_op() {
                    let value = self.parse_expr()?;
                    self.expect_kind(TokenKind::Semicolon)?;
                    return Ok(Self::compound_update_projection(target, op, value));
                }
            }

            if indices.is_empty() && self.match_kind(&TokenKind::MinusMinus) {
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Self::compound_update_projection(
                    target,
                    BinaryOp::Minus,
                    Expr::Number("1".to_string()),
                ));
            }

            if self.match_kind(&TokenKind::Shift2) {
                let ty = self.expect_type_name()?;
                if self.match_kind(&TokenKind::Semicolon) {
                    return Ok(Stmt::Projection {
                        target: AssignTarget::Context(target),
                        ty,
                        value: Expr::Call {
                            callee: "__nC_uninit".to_string(),
                            args: Vec::new(),
                        },
                    });
                }
                self.expect_projection_init_separator()?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }

            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                self.expect_kind(TokenKind::Semicolon)?;
                return Ok(Stmt::StrictCheck {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }

            self.pos = save;
        }

        let expr = self.parse_expr()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_plain_context_ref(&mut self) -> Result<ContextRef, Diagnostic> {
        if !Self::is_context_segment_start(&self.current().kind) {
            return self.error_here("Expected identifier");
        }

        let mut path = vec![self.expect_context_segment()?];
        while self.check(&TokenKind::Dot)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(kind) if Self::is_context_segment_start(kind)
            )
            && !matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::LParen)
            )
        {
            self.advance();
            path.push(self.expect_context_segment()?);
        }

        Ok(ContextRef::plain(path))
    }

    fn match_compound_assignment_op(&mut self) -> Option<BinaryOp> {
        if self.match_kind(&TokenKind::PlusEq) {
            Some(BinaryOp::Plus)
        } else if self.match_kind(&TokenKind::MinusEq) {
            Some(BinaryOp::Minus)
        } else if self.match_kind(&TokenKind::StarEq) {
            Some(BinaryOp::Star)
        } else {
            None
        }
    }

    fn compound_update_projection(target: ContextRef, op: BinaryOp, value: Expr) -> Stmt {
        let left = Expr::ContextRef(target.clone());
        Stmt::Projection {
            target: AssignTarget::Context(target),
            ty: Self::inferred_projection_type(),
            value: Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(value),
            },
        }
    }

    fn parse_nominal_kind_segment(&mut self) -> Result<String, Diagnostic> {
        match &self.current().kind {
            TokenKind::Identifier(name) => {
                let out = name.to_ascii_lowercase();
                self.advance();
                Ok(out)
            }
            TokenKind::KeywordMove => {
                self.advance();
                Ok("move".to_string())
            }
            TokenKind::KeywordBinary => {
                self.advance();
                Ok("binary".to_string())
            }
            TokenKind::KeywordRef => {
                self.advance();
                Ok("ref".to_string())
            }
            _ => self.error_here("expected destructure/constructure kind segment"),
        }
    }

    fn parse_expr_list_in_parens(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        self.expect_kind(TokenKind::LParen)?;
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect_kind(TokenKind::RParen)?;
        Ok(args)
    }

    fn parse_destructure_stmt_after_keyword(&mut self) -> Result<Stmt, Diagnostic> {
        let kind = if self.check(&TokenKind::LParen) {
            "default".to_string()
        } else {
            let _ = self.match_kind(&TokenKind::Dot);
            self.parse_nominal_kind_segment()?
        };

        let kind = match kind.as_str() {
            "default" => DestructureKind::Default,
            "static" => DestructureKind::Static,
            "move" => DestructureKind::Move,
            "ref" => DestructureKind::Ref,
            "region" => DestructureKind::Region,
            "binary" => DestructureKind::Binary,
            "result" => DestructureKind::Result,
            "ownership" => DestructureKind::Ownership,
            _ => {
                return self.error_here(
                    "unknown destructure kind (expected: static | move | ref | region | binary | result | ownership)",
                )
            }
        };

        let args = self.parse_expr_list_in_parens()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Stmt::Destructure { kind, args })
    }

    fn parse_constructure_stmt_after_keyword(&mut self) -> Result<Stmt, Diagnostic> {
        let kind = if self.check(&TokenKind::LParen) {
            "default".to_string()
        } else {
            let _ = self.match_kind(&TokenKind::Dot);
            self.parse_nominal_kind_segment()?
        };

        let kind = match kind.as_str() {
            "default" => ConstructureKind::Default,
            "static" => ConstructureKind::Static,
            "move" => ConstructureKind::Move,
            "copy" => ConstructureKind::Copy,
            "region" => ConstructureKind::Region,
            "binary" => ConstructureKind::Binary,
            "unbinary" => ConstructureKind::Unbinary,
            _ => {
                return self.error_here(
                    "unknown constructure kind (expected: static | move | copy | region | binary | unbinary)",
                )
            }
        };

        let args = self.parse_expr_list_in_parens()?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Stmt::Constructure { kind, args })
    }

    fn parse_if_stmt_after_keyword(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect_kind(TokenKind::LParen)?;
        let condition = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        let then_block = self.parse_block_or_single_stmt()?;
        let else_block = if self.match_kind(&TokenKind::KeywordElse) {
            if self.match_kind(&TokenKind::KeywordIf) {
                vec![self.parse_if_stmt_after_keyword()?]
            } else {
                self.parse_block_or_single_stmt()?
            }
        } else if self.match_word_ci("elseif") {
            vec![self.parse_if_stmt_after_keyword()?]
        } else {
            Vec::new()
        };
        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_notif_stmt_after_word(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect_kind(TokenKind::LParen)?;
        let condition = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        let then_block = self.parse_block_or_single_stmt()?;
        let else_block = if self.match_kind(&TokenKind::KeywordElse) {
            self.parse_block_or_single_stmt()?
        } else {
            Vec::new()
        };
        Ok(Stmt::If {
            condition: Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(condition),
            },
            then_block,
            else_block,
        })
    }

    fn parse_inline_stmt_no_semi(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&TokenKind::At) {
            let save = self.pos;
            let (target, indices, indexed_field) = self.parse_context_ref_with_indices()?;
            if self.match_kind(&TokenKind::LessDash) {
                if !indices.is_empty() {
                    return self.error_here(
                        "compact pointer write expects a pointer symbol; use ptrOffset/regionPtrAt for indexed writes",
                    );
                }
                let value = self.parse_expr()?;
                return Ok(Stmt::Expr(Expr::MethodCall {
                    receiver: Box::new(Expr::ContextRef(target)),
                    method: "write".to_string(),
                    args: vec![value],
                }));
            }

            if let Some(op) = self.match_compound_assignment_op() {
                if !indices.is_empty() {
                    return self.error_here(
                        "compound assignment expects a mutable symbol or field target",
                    );
                }
                let value = self.parse_expr()?;
                return Ok(Self::compound_update_projection(target, op, value));
            }

            if self.match_kind(&TokenKind::Shift2) {
                let ty = self.expect_type_name()?;
                self.expect_projection_init_separator()?;
                let value = self.parse_expr()?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if let Some(fields) = indexed_field {
                        args.push(Expr::ContextRef(ContextRef::plain(fields)));
                        "arrayFieldWrite"
                    } else {
                        "arrayWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }
            if self.match_kind(&TokenKind::Eq) {
                if !indices.is_empty() {
                    return self.error_here(
                        "compact assignment expects a simple symbol target; use official projection for indexed writes",
                    );
                }
                let value = self.parse_expr()?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(target),
                    ty: Self::inferred_projection_type(),
                    value,
                });
            }
            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                if !indices.is_empty() {
                    let mut args = indices;
                    let method = if let Some(fields) = indexed_field {
                        args.push(Expr::ContextRef(ContextRef::plain(fields)));
                        "arrayFieldWrite"
                    } else {
                        "arrayWrite"
                    };
                    args.push(value);
                    return Ok(Stmt::Expr(Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(target)),
                        method: method.to_string(),
                        args,
                    }));
                }
                return Ok(Stmt::StrictCheck {
                    target: AssignTarget::Context(target),
                    ty,
                    value,
                });
            }
            self.pos = save;
        }

        if self.peek_is_ident() {
            let save = self.pos;
            let name = self.expect_ident()?;
            if let Some(op) = self.match_compound_assignment_op() {
                let value = self.parse_expr()?;
                return Ok(Self::compound_update_projection(
                    ContextRef::plain(vec![name]),
                    op,
                    value,
                ));
            }
            if self.match_kind(&TokenKind::LessDash) || self.match_kind(&TokenKind::Eq) {
                let value = self.parse_expr()?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(ContextRef::plain(vec![name])),
                    ty: Self::inferred_projection_type(),
                    value,
                });
            }
            if self.match_kind(&TokenKind::Shift2) {
                let ty = self.expect_type_name()?;
                self.expect_projection_init_separator()?;
                let value = self.parse_expr()?;
                return Ok(Stmt::Projection {
                    target: AssignTarget::Context(ContextRef::plain(vec![name])),
                    ty,
                    value,
                });
            }
            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                return Ok(Stmt::StrictCheck {
                    target: AssignTarget::Context(ContextRef::plain(vec![name])),
                    ty,
                    value,
                });
            }
            self.pos = save;
        }

        Ok(Stmt::Expr(self.parse_expr()?))
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.match_kind(&TokenKind::LParen) {
            let init = if self.check(&TokenKind::Semicolon) {
                None
            } else {
                Some(Box::new(self.parse_inline_stmt_no_semi()?))
            };
            self.expect_kind(TokenKind::Semicolon)?;

            let condition = if self.check(&TokenKind::Semicolon) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect_kind(TokenKind::Semicolon)?;

            let step = if self.check(&TokenKind::RParen) {
                None
            } else {
                Some(Box::new(self.parse_inline_stmt_no_semi()?))
            };
            self.expect_kind(TokenKind::RParen)?;

            let body = self.parse_block_stmts()?;
            return Ok(Stmt::ForClassic {
                init,
                condition,
                step,
                body,
            });
        }

        let item = self.expect_ident()?;
        if !self.match_word_ci("in") {
            return self.error_here("Expected 'in' in for-loop");
        }

        let start_or_iter = if self.peek_is_ident() && self.check_next_kind(&TokenKind::LBrace) {
            Expr::Identifier(self.expect_ident()?)
        } else {
            self.parse_expr()?
        };
        if let Expr::Call { callee, args } = &start_or_iter {
            if callee.eq_ignore_ascii_case("range") && args.len() == 1 {
                let end = args[0].clone();
                let body = self.parse_block_stmts()?;
                return Ok(Stmt::ForRange {
                    item,
                    start: Expr::Number("0".into()),
                    end,
                    inclusive: false,
                    step: None,
                    body,
                });
            }
        }
        if self.match_dotdot() {
            let inclusive = self.match_kind(&TokenKind::Eq);
            let end = if self.peek_is_ident() && self.check_next_kind(&TokenKind::LBrace) {
                Expr::Identifier(self.expect_ident()?)
            } else {
                self.parse_expr()?
            };
            let step = if self.match_word_ci("step") {
                Some(
                    if self.peek_is_ident() && self.check_next_kind(&TokenKind::LBrace) {
                        Expr::Identifier(self.expect_ident()?)
                    } else {
                        self.parse_expr()?
                    },
                )
            } else {
                None
            };
            let body = self.parse_block_stmts()?;
            return Ok(Stmt::ForRange {
                item,
                start: start_or_iter,
                end,
                inclusive,
                step,
                body,
            });
        }

        let body = self.parse_block_stmts()?;
        Ok(Stmt::ForIn {
            item,
            iterable: start_or_iter,
            body,
        })
    }

    fn expect_fat_arrow(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(TokenKind::Eq)?;
        self.expect_kind(TokenKind::Greater)?;
        Ok(())
    }

    fn parse_labeled_case_stmts(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace)
            && !self.check_word_ci("case")
            && !self.check_word_ci("default")
        {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_switch_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect_kind(TokenKind::LParen)?;
        let value = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::LBrace)?;

        let mut cases = Vec::new();
        let mut default = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.match_word_ci("case") {
                let case_value = self.parse_expr()?;
                let case_block = self.parse_block_stmts()?;
                cases.push((case_value, case_block));
                continue;
            }
            if self.match_word_ci("default") {
                default = self.parse_block_stmts()?;
                continue;
            }
            return self.error_here("Expected case/default in switch block");
        }

        self.expect_kind(TokenKind::RBrace)?;
        Ok(Stmt::Switch {
            value,
            cases,
            default,
        })
    }

    fn parse_match_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let value = if self.match_kind(&TokenKind::LParen) {
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            value
        } else {
            self.parse_expr()?
        };
        self.expect_kind(TokenKind::LBrace)?;

        if self.check(&TokenKind::RBrace) {
            self.expect_kind(TokenKind::RBrace)?;
            return Ok(Stmt::PatternMatch {
                value,
                arms: Vec::new(),
            });
        }

        if !self.check(&TokenKind::RBrace)
            && !self.check_word_ci("case")
            && !self.check_word_ci("default")
        {
            let mut arms = Vec::new();
            while !self.check(&TokenKind::RBrace) {
                let pattern = self.parse_pattern()?;
                let guard = if self.match_kind(&TokenKind::KeywordIf) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_fat_arrow()?;
                let body = self.parse_block_or_single_stmt()?;
                arms.push(MatchArm {
                    pattern,
                    guard,
                    body,
                });
            }
            self.expect_kind(TokenKind::RBrace)?;
            if arms.is_empty() {
                return self.error_here("E_HIGH_MATCH_EMPTY_001: match requires arms");
            }
            return Ok(Stmt::PatternMatch { value, arms });
        }

        let mut cases = Vec::new();
        let mut default = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.match_word_ci("case") {
                let case_value = self.parse_expr()?;
                let case_block = if self.match_kind(&TokenKind::Colon) {
                    self.parse_labeled_case_stmts()?
                } else {
                    self.expect_fat_arrow()?;
                    self.parse_block_stmts()?
                };
                cases.push((case_value, case_block));
                continue;
            }
            if self.match_word_ci("default") {
                default = if self.match_kind(&TokenKind::Colon) {
                    self.parse_labeled_case_stmts()?
                } else {
                    self.expect_fat_arrow()?;
                    self.parse_block_stmts()?
                };
                continue;
            }
            return self.error_here("Expected case/default in match block");
        }

        self.expect_kind(TokenKind::RBrace)?;
        Ok(Stmt::Match {
            value,
            cases,
            default,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, Diagnostic> {
        if self.match_ident_name("_") {
            return Ok(Pattern::Wildcard);
        }

        if self.match_kind(&TokenKind::LBracket) {
            let mut items = Vec::new();
            let mut rest = None;
            while !self.check(&TokenKind::RBracket) {
                if self.match_kind(&TokenKind::Dot) {
                    self.expect_kind(TokenKind::Dot)?;
                    rest = if self.peek_is_ident() {
                        Some(self.expect_ident()?)
                    } else {
                        Some("_".to_string())
                    };
                    if self.match_kind(&TokenKind::Comma) {
                        if !self.check(&TokenKind::RBracket) {
                            return self.error_here(
                                "E_HIGH_PATTERN_PARSE_001: array rest must be the final pattern",
                            );
                        }
                    }
                    break;
                }
                items.push(self.parse_pattern()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(TokenKind::RBracket)?;
            return Ok(Pattern::Array { items, rest });
        }

        if self.match_kind(&TokenKind::KeywordTrue) {
            return Ok(Pattern::Literal(Expr::Bool(true)));
        }
        if self.match_kind(&TokenKind::KeywordFalse) {
            return Ok(Pattern::Literal(Expr::Bool(false)));
        }
        if let Some(s) = self.match_string() {
            return Ok(Pattern::Literal(Expr::String(s)));
        }
        if let TokenKind::Char(ch) = self.current().kind.clone() {
            self.advance();
            let start = Expr::Char(ch);
            if self.match_kind(&TokenKind::Dot) {
                self.expect_kind(TokenKind::Dot)?;
                let inclusive = self.match_kind(&TokenKind::Eq);
                let end = self.parse_pattern_range_endpoint()?;
                return Ok(Pattern::Range {
                    start,
                    end,
                    inclusive,
                });
            }
            return Ok(Pattern::Literal(start));
        }
        if let Some(n) = self.match_number() {
            let start = Expr::Number(n);
            if self.match_kind(&TokenKind::Dot) {
                self.expect_kind(TokenKind::Dot)?;
                let inclusive = self.match_kind(&TokenKind::Eq);
                let end = self.parse_pattern_range_endpoint()?;
                return Ok(Pattern::Range {
                    start,
                    end,
                    inclusive,
                });
            }
            return Ok(Pattern::Literal(start));
        }

        if !self.peek_is_ident() {
            return self.error_here("E_HIGH_PATTERN_PARSE_001: expected pattern");
        }

        let mut path = vec![self.expect_ident()?];
        while self.match_kind(&TokenKind::Dot) {
            path.push(self.expect_ident()?);
        }

        if self.match_kind(&TokenKind::LBrace) {
            let mut fields = Vec::new();
            let mut has_rest = false;
            while !self.check(&TokenKind::RBrace) {
                if self.match_kind(&TokenKind::Dot) {
                    self.expect_kind(TokenKind::Dot)?;
                    has_rest = true;
                    if self.match_kind(&TokenKind::Comma) && !self.check(&TokenKind::RBrace) {
                        return self.error_here(
                            "E_HIGH_PATTERN_PARSE_001: struct rest must be the final pattern",
                        );
                    }
                    break;
                }
                let field_name = self.expect_ident()?;
                let pattern = if self.match_kind(&TokenKind::Colon) {
                    self.parse_pattern()?
                } else {
                    Pattern::Binding(field_name.clone())
                };
                fields.push(PatternField {
                    name: field_name,
                    pattern,
                });
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(TokenKind::RBrace)?;
            return Ok(Pattern::Struct {
                type_name: path.join("."),
                fields,
                has_rest,
            });
        }

        if path.len() == 1
            && matches!(
                path[0].as_str(),
                "Some" | "None" | "Ok" | "Err" | "some" | "none" | "ok" | "err"
            )
        {
            let (enum_name, variant_name) = match path[0].as_str() {
                "Some" | "some" => ("Option".to_string(), "Some".to_string()),
                "None" | "none" => ("Option".to_string(), "None".to_string()),
                "Ok" | "ok" => ("Result".to_string(), "Ok".to_string()),
                _ => ("Result".to_string(), "Err".to_string()),
            };
            let mut bindings = Vec::new();
            let mut nested: Option<Vec<Pattern>> = None;
            if self.match_kind(&TokenKind::LParen) {
                if !self.check(&TokenKind::RParen) {
                    loop {
                        let item = self.parse_variant_pattern_item()?;
                        match item {
                            VariantPatternItem::Binding(binding) if nested.is_none() => {
                                bindings.push(binding)
                            }
                            VariantPatternItem::Binding(binding) => nested
                                .as_mut()
                                .expect("nested items were initialized")
                                .push(Pattern::Binding(binding)),
                            VariantPatternItem::Pattern(pattern) => {
                                let items = nested.get_or_insert_with(|| {
                                    bindings.drain(..).map(Pattern::Binding).collect::<Vec<_>>()
                                });
                                items.push(pattern);
                            }
                        }
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(TokenKind::RParen)?;
            }
            if let Some(items) = nested {
                return Ok(Pattern::VariantPattern {
                    enum_name,
                    variant_name,
                    items,
                });
            }
            return Ok(Pattern::Variant {
                enum_name,
                variant_name,
                bindings,
            });
        }

        if path.len() == 2 {
            let mut bindings = Vec::new();
            let mut nested: Option<Vec<Pattern>> = None;
            if self.match_kind(&TokenKind::LParen) {
                if !self.check(&TokenKind::RParen) {
                    loop {
                        let item = self.parse_variant_pattern_item()?;
                        match item {
                            VariantPatternItem::Binding(binding) if nested.is_none() => {
                                bindings.push(binding)
                            }
                            VariantPatternItem::Binding(binding) => nested
                                .as_mut()
                                .expect("nested items were initialized")
                                .push(Pattern::Binding(binding)),
                            VariantPatternItem::Pattern(pattern) => {
                                let items = nested.get_or_insert_with(|| {
                                    bindings.drain(..).map(Pattern::Binding).collect::<Vec<_>>()
                                });
                                items.push(pattern);
                            }
                        }
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(TokenKind::RParen)?;
            }
            if let Some(items) = nested {
                return Ok(Pattern::VariantPattern {
                    enum_name: path[0].clone(),
                    variant_name: path[1].clone(),
                    items,
                });
            }
            return Ok(Pattern::Variant {
                enum_name: path[0].clone(),
                variant_name: path[1].clone(),
                bindings,
            });
        }

        if path.len() == 1 {
            return Ok(Pattern::Binding(path.remove(0)));
        }

        self.error_here("E_HIGH_PATTERN_PARSE_001: pattern path must be Enum.Variant")
    }

    fn parse_pattern_range_endpoint(&mut self) -> Result<Expr, Diagnostic> {
        if let Some(number) = self.match_number() {
            return Ok(Expr::Number(number));
        }
        if let TokenKind::Char(ch) = self.current().kind.clone() {
            self.advance();
            return Ok(Expr::Char(ch));
        }
        self.error_here(
            "E_HIGH_PATTERN_RANGE_001: range endpoint must be an integer or char literal",
        )
    }

    fn parse_variant_pattern_item(&mut self) -> Result<VariantPatternItem, Diagnostic> {
        if self.peek_is_ident() {
            let save = self.pos;
            if let Ok(binding) = self.parse_pattern_binding() {
                if self.check(&TokenKind::Comma) || self.check(&TokenKind::RParen) {
                    return Ok(VariantPatternItem::Binding(binding));
                }
            }
            self.pos = save;
        }
        self.parse_pattern().map(VariantPatternItem::Pattern)
    }

    fn parse_pattern_binding(&mut self) -> Result<String, Diagnostic> {
        let name = self.expect_ident()?;
        if self.match_kind(&TokenKind::Colon) {
            let ty = self.expect_type_name()?;
            Ok(format!("{}:{}", name, Self::render_call_type_arg(&ty)))
        } else {
            Ok(name)
        }
    }

    fn parse_select_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect_kind(TokenKind::LBrace)?;
        let mut branches = Vec::new();
        let mut default = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.match_word_ci("when") {
                self.expect_kind(TokenKind::LParen)?;
                let condition = self.parse_expr()?;
                self.expect_kind(TokenKind::RParen)?;
                let block = self.parse_block_stmts()?;
                branches.push((condition, block));
                continue;
            }
            if self.match_word_ci("default") {
                default = self.parse_block_stmts()?;
                continue;
            }
            return self.error_here("Expected when/default in select block");
        }

        self.expect_kind(TokenKind::RBrace)?;
        Ok(Stmt::Select { branches, default })
    }

    fn parse_block_stmts(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.expect_kind(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect_kind(TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parse_block_or_single_stmt(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        if self.check(&TokenKind::LBrace) {
            self.parse_block_stmts()
        } else {
            Ok(vec![self.parse_stmt()?])
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_conditional_expr()
    }

    fn parse_conditional_expr(&mut self) -> Result<Expr, Diagnostic> {
        let condition = self.parse_binary_expr(0)?;
        if !self.match_kind(&TokenKind::Question) {
            return Ok(condition);
        }
        if !self.conditional_question_has_top_level_colon() {
            return Ok(Expr::Unary {
                op: UnaryOp::TryPropagate,
                expr: Box::new(condition),
            });
        }
        let then_expr = self.parse_expr()?;
        self.expect_kind(TokenKind::Colon)?;
        let else_expr = self.parse_expr()?;
        Ok(Expr::IfElse {
            condition: Box::new(condition),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        })
    }

    fn conditional_question_has_top_level_colon(&self) -> bool {
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        for token in self.tokens.iter().skip(self.pos) {
            match &token.kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RParen => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        return false;
                    }
                    paren_depth = paren_depth.saturating_sub(1);
                }
                TokenKind::RBracket => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        return false;
                    }
                    bracket_depth = bracket_depth.saturating_sub(1);
                }
                TokenKind::RBrace => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        return false;
                    }
                    brace_depth = brace_depth.saturating_sub(1);
                }
                TokenKind::Colon if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    return true;
                }
                TokenKind::Semicolon | TokenKind::Comma | TokenKind::Eof
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 =>
                {
                    return false;
                }
                _ => {}
            }
        }
        false
    }

    fn render_call_type_arg(ty: &TypeName) -> String {
        match ty {
            TypeName::Simple(name) => name.clone(),
            TypeName::Vector(inner) => format!("vector<{}>", Self::render_call_type_arg(inner)),
            TypeName::Array(inner) => format!("array<{}>", Self::render_call_type_arg(inner)),
            TypeName::Union(items) => items
                .iter()
                .map(Self::render_call_type_arg)
                .collect::<Vec<_>>()
                .join("|"),
            TypeName::Generic { base, args } => {
                let rendered = args
                    .iter()
                    .map(Self::render_call_type_arg)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{base}<{rendered}>")
            }
        }
    }

    fn parse_optional_call_type_args(&mut self) -> Option<Vec<String>> {
        let save = self.pos;
        if !self.match_kind(&TokenKind::Less) {
            return None;
        }

        let first = match self.expect_type_generic_arg() {
            Ok(ty) => ty,
            Err(_) => {
                self.pos = save;
                return None;
            }
        };
        let mut args = vec![Self::render_call_type_arg(&first)];
        while self.match_kind(&TokenKind::Comma) {
            let Ok(ty) = self.expect_type_generic_arg() else {
                self.pos = save;
                return None;
            };
            args.push(Self::render_call_type_arg(&ty));
        }
        if self.expect_gt().is_err() {
            self.pos = save;
            return None;
        }
        Some(args)
    }

    // Compact low-level spelling. The AST always retains the runtime names so
    // sema, IR and every backend share one pointer contract.
    fn compact_pointer_call_name(name: &str) -> Option<&'static str> {
        if let Some(spec) = raw_api_v1_typed_alias(name) {
            return Some(spec.canonical);
        }
        if name.eq_ignore_ascii_case("read")
            || name.eq_ignore_ascii_case("pread")
            || name.eq_ignore_ascii_case("rd")
        {
            Some("ptrRead")
        } else if name.eq_ignore_ascii_case("write")
            || name.eq_ignore_ascii_case("pwrite")
            || name.eq_ignore_ascii_case("wr")
        {
            Some("ptrWrite")
        } else if name.eq_ignore_ascii_case("offset")
            || name.eq_ignore_ascii_case("poffset")
            || name.eq_ignore_ascii_case("off")
        {
            Some("ptrOffset")
        } else if name.eq_ignore_ascii_case("diff")
            || name.eq_ignore_ascii_case("pdiff")
            || name.eq_ignore_ascii_case("dist")
        {
            Some("ptrDiff")
        } else if name.eq_ignore_ascii_case("null")
            || name.eq_ignore_ascii_case("pnull")
            || name.eq_ignore_ascii_case("nil")
        {
            Some("ptrIsNull")
        } else if name.eq_ignore_ascii_case("region") || name.eq_ignore_ascii_case("pregion") {
            Some("regionFromPtr")
        } else if name.eq_ignore_ascii_case("at") || name.eq_ignore_ascii_case("pat") {
            Some("regionPtrAt")
        } else if name.eq_ignore_ascii_case("fromaddr")
            || name.eq_ignore_ascii_case("pfromaddr")
            || name.eq_ignore_ascii_case("from")
        {
            Some("ptrFromAddr")
        } else if name.eq_ignore_ascii_case("cast")
            || name.eq_ignore_ascii_case("pcast")
            || name.eq_ignore_ascii_case("asptr")
        {
            Some("ptrCast")
        } else if name.eq_ignore_ascii_case("addr")
            || name.eq_ignore_ascii_case("paddr")
            || name.eq_ignore_ascii_case("adr")
        {
            Some("ptrAddr")
        } else {
            None
        }
    }

    fn compact_pointer_name_is_unambiguous(name: &str) -> bool {
        if NC_RAW_API_V1_ALIASES.iter().any(|spec| {
            spec.domain == RawApiDomain::Pointer
                && spec.unambiguous_without_type_arguments
                && spec.compact.eq_ignore_ascii_case(name)
        }) {
            return true;
        }
        matches!(
            name.to_ascii_lowercase().as_str(),
            "pread"
                | "rd"
                | "pwrite"
                | "wr"
                | "poffset"
                | "off"
                | "pdiff"
                | "dist"
                | "pnull"
                | "nil"
                | "pregion"
                | "pat"
                | "pfromaddr"
                | "from"
                | "pcast"
                | "asptr"
                | "paddr"
                | "adr"
        )
    }

    fn compact_raw_call_name(name: &str, arity: usize) -> Option<&'static str> {
        if let Some(spec) = raw_api_v1_alias(name, arity) {
            return Some(spec.canonical);
        }
        match (name.to_ascii_lowercase().as_str(), arity) {
            ("at", 2) | ("pat", 2) => Some("regionPtrAt"),
            ("cat", 4) => Some("strConcatInto"),
            ("map", 6) | ("mmap", 6) => Some("vmMap"),
            ("protect", 3) | ("prot", 3) => Some("vmProtect"),
            ("unmap", 2) => Some("vmUnmap"),
            ("exec", 2) | ("rx", 2) => Some("vmExecutable"),
            _ => None,
        }
    }

    fn in_kernel_or_system_context(&self) -> bool {
        matches!(self.active_context, Some(Context::Kernel | Context::System))
    }

    fn infer_implicit_kernel_context(&self, container_name: &str) -> bool {
        let mut cursor = self.pos;
        let mut nested_braces = 0usize;
        let mut has_main_entry = false;

        while cursor < self.tokens.len() {
            match &self.tokens[cursor].kind {
                TokenKind::LBrace => nested_braces += 1,
                TokenKind::RBrace if nested_braces == 0 => break,
                TokenKind::RBrace => nested_braces -= 1,
                TokenKind::KeywordMain
                    if matches!(
                        self.tokens.get(cursor + 1).map(|token| &token.kind),
                        Some(TokenKind::Colon)
                    ) =>
                {
                    has_main_entry = true;
                }
                TokenKind::Identifier(_)
                    if matches!(
                        self.tokens.get(cursor + 1).map(|token| &token.kind),
                        Some(TokenKind::Identifier(word)) if word.eq_ignore_ascii_case("at")
                    ) =>
                {
                    return true;
                }
                TokenKind::Star
                    if matches!(
                        self.tokens.get(cursor + 1).map(|token| &token.kind),
                        Some(TokenKind::Number(width))
                            if matches!(width.as_str(), "8" | "16" | "32" | "64" | "128" | "256" | "512")
                    ) && matches!(
                        self.tokens.get(cursor + 2).map(|token| &token.kind),
                        Some(TokenKind::LParen)
                    ) =>
                {
                    return true;
                }
                _ => {}
            }
            cursor += 1;
        }

        container_name
            .get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("kernel"))
            && has_main_entry
    }

    fn compact_kernel_call_name(name: &str) -> Option<&'static str> {
        match name.to_ascii_lowercase().as_str() {
            "load8" => Some("mmio_read8"),
            "load16" => Some("mmio_read16"),
            "load32" => Some("mmio_read32"),
            "load64" => Some("mmio_read64"),
            "load128" => Some("mmio_read128"),
            "load256" => Some("mmio_read256"),
            "load512" => Some("mmio_read512"),
            "store8" => Some("mmio_write8"),
            "store16" => Some("mmio_write16"),
            "store32" => Some("mmio_write32"),
            "store64" => Some("mmio_write64"),
            "store128" => Some("mmio_write128"),
            "store256" => Some("mmio_write256"),
            "store512" => Some("mmio_write512"),
            "sendipi" => Some("apic_send_ipi"),
            "msrread" => Some("x2apic_read"),
            "msrwrite" => Some("x2apic_write"),
            "tscread" => Some("tsc_read"),
            "tscdeadline" => Some("tsc_deadline_write"),
            "dbgregs" => Some("debug_register_snapshot"),
            "dbgdump" => Some("debug_memory_dump"),
            "dbgtrace" => Some("debug_stack_trace"),
            "dbgsym" => Some("debug_symbol_lookup"),
            "rspsum" => Some("debug_rsp_checksum"),
            "rspframe" => Some("debug_rsp_frame"),
            "brkset" => Some("debug_breakpoint_install"),
            "brkdel" => Some("debug_breakpoint_remove"),
            "crashwrite" => Some("debug_crash_report"),
            "kaslr" => Some("kaslr_relocate"),
            "canaryset" => Some("stack_canary_write"),
            "canaryok" => Some("stack_canary_check"),
            "sanpoison" => Some("sanitizer_poison"),
            "sanclear" => Some("sanitizer_unpoison"),
            "sanok" => Some("sanitizer_check"),
            _ => None,
        }
    }

    fn compact_mmio_write_width(&self, starred: bool) -> Option<&'static str> {
        let base = self.pos + usize::from(starred);
        if starred && !self.check(&TokenKind::Star) {
            return None;
        }
        let TokenKind::Number(width) = &self.tokens.get(base)?.kind else {
            return None;
        };
        if !matches!(
            self.tokens.get(base + 1).map(|token| &token.kind),
            Some(TokenKind::LParen)
        ) {
            return None;
        }
        match width.as_str() {
            "8" => Some("8"),
            "16" => Some("16"),
            "32" => Some("32"),
            "64" => Some("64"),
            "128" => Some("128"),
            "256" => Some("256"),
            "512" => Some("512"),
            _ => None,
        }
    }

    fn parse_compact_mmio_write(&mut self, width: &str) -> Result<Stmt, Diagnostic> {
        let parsed_width = self
            .match_number()
            .expect("compact MMIO width was checked before parsing");
        debug_assert_eq!(parsed_width, width);
        self.expect_kind(TokenKind::LParen)?;
        let address = self.parse_expr()?;
        self.expect_kind(TokenKind::Comma)?;
        let value = self.parse_expr()?;
        if self.match_kind(&TokenKind::Comma) {
            return self.error_here("compact MMIO write expects address and value");
        }
        self.expect_kind(TokenKind::RParen)?;
        self.expect_kind(TokenKind::Semicolon)?;
        Ok(Stmt::Expr(Expr::Call {
            callee: format!("mmio_write{width}"),
            args: vec![address, value],
        }))
    }

    fn parse_binary_expr(&mut self, min_prec: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;

        while let Some((op, prec)) = self.current_binary_op() {
            if prec < min_prec {
                break;
            }
            self.advance();
            let right = self.parse_binary_expr(prec + 1)?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.check_word_ci("try") && !self.check_next_kind(&TokenKind::LParen) {
            self.advance();
            let value = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::TryPropagate,
                expr: Box::new(value),
            });
        }

        if self.check_word_ci("async") {
            let next = self.tokens.get(self.pos + 1).map(|tok| &tok.kind);
            let next_next = self.tokens.get(self.pos + 2).map(|tok| &tok.kind);
            let direct = matches!(next, Some(TokenKind::Pipe) | Some(TokenKind::OrOr));
            let moved = matches!(next, Some(TokenKind::KeywordMove))
                && matches!(next_next, Some(TokenKind::Pipe) | Some(TokenKind::OrOr));
            if direct || moved {
                self.advance();
                if moved {
                    self.expect_kind(TokenKind::KeywordMove)?;
                }
                return self.parse_closure_expr(moved, true);
            }
        }

        if self.check(&TokenKind::KeywordMove)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::Pipe) | Some(TokenKind::OrOr)
            )
        {
            self.expect_kind(TokenKind::KeywordMove)?;
            return self.parse_closure_expr(true, false);
        }

        if self.check_word_ci("await") && !self.check_next_kind(&TokenKind::LParen) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "await".to_string(),
                args: vec![expr],
            });
        }
        if self.check_word_ci("spawn") && !self.check_next_kind(&TokenKind::LParen) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "spawn".to_string(),
                args: vec![expr],
            });
        }
        if self.check_word_ci("join") && !self.check_next_kind(&TokenKind::LParen) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "join".to_string(),
                args: vec![expr],
            });
        }
        if self.check_word_ci("copy")
            && !self.check_next_kind(&TokenKind::LParen)
            && !self.check_next_kind(&TokenKind::Dot)
            && self.tokens.get(self.pos + 1).is_some_and(|token| {
                matches!(
                    token.kind,
                    TokenKind::Identifier(_)
                        | TokenKind::At
                        | TokenKind::KeywordTrue
                        | TokenKind::KeywordFalse
                        | TokenKind::KeywordDev
                        | TokenKind::String(_)
                        | TokenKind::Char(_)
                        | TokenKind::Number(_)
                        | TokenKind::LBracket
                        | TokenKind::LParen
                        | TokenKind::Pipe
                        | TokenKind::OrOr
                        | TokenKind::Star
                        | TokenKind::Amp
                        | TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Bang
                        | TokenKind::Tilde
                )
            })
        {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "copy".to_string(),
                args: vec![expr],
            });
        }
        if self.check_word_ci("clone") && self.nc_prefix_operand_follows() {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "clone".to_string(),
                args: vec![expr],
            });
        }
        if self.check(&TokenKind::KeywordMove)
            && !self.check_next_kind(&TokenKind::LParen)
            && !self.check_next_kind(&TokenKind::Dot)
        {
            self.expect_kind(TokenKind::KeywordMove)?;
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "move".to_string(),
                args: vec![expr],
            });
        }

        if self.match_kind(&TokenKind::Star) {
            let expr = self.parse_unary()?;
            if let Expr::PostIncrement { target } = expr {
                return Ok(Expr::PointerPostIncrementRead { target });
            }
            return Ok(Expr::MethodCall {
                receiver: Box::new(expr),
                method: "read".to_string(),
                args: Vec::new(),
            });
        }

        if self.match_kind(&TokenKind::Amp) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Call {
                callee: "addrOf".to_string(),
                args: vec![expr],
            });
        }

        if self.match_kind(&TokenKind::KeywordIf) {
            self.expect_kind(TokenKind::LParen)?;
            let condition = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;

            self.expect_kind(TokenKind::LBrace)?;
            let then_expr = self.parse_expr()?;
            self.expect_kind(TokenKind::RBrace)?;

            self.expect_kind(TokenKind::KeywordElse)?;
            self.expect_kind(TokenKind::LBrace)?;
            let else_expr = self.parse_expr()?;
            self.expect_kind(TokenKind::RBrace)?;

            return Ok(Expr::IfElse {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            });
        }

        let op = if self.match_kind(&TokenKind::Plus) {
            Some(UnaryOp::Plus)
        } else if self.match_kind(&TokenKind::Minus) {
            Some(UnaryOp::Minus)
        } else if self.match_kind(&TokenKind::Bang) {
            Some(UnaryOp::Not)
        } else if self.match_kind(&TokenKind::Tilde) {
            Some(UnaryOp::NotBin)
        } else {
            None
        };

        if let Some(op) = op {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        if matches!(self.current().kind, TokenKind::Pipe | TokenKind::OrOr) {
            return self.parse_closure_expr(false, false);
        }

        if self.match_kind(&TokenKind::KeywordTrue) {
            return Ok(Expr::Bool(true));
        }
        if self.match_kind(&TokenKind::KeywordFalse) {
            return Ok(Expr::Bool(false));
        }
        if self.match_kind(&TokenKind::KeywordDev) {
            let errors = self.parse_dev_error_list()?;
            return Ok(Expr::Dev(errors));
        }

        if self.check_ident_name("load")
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Identifier(_))
            )
            && matches!(
                self.tokens.get(self.pos + 2).map(|token| &token.kind),
                Some(TokenKind::LBracket | TokenKind::Less)
            )
        {
            self.expect_ident()?;
            let ty = self.expect_material_access_type()?;
            self.expect_kind(TokenKind::LBracket)?;
            let address = self.parse_expr()?;
            self.expect_kind(TokenKind::RBracket)?;
            return self.parse_postfix_expr(Expr::Call {
                callee: format!("ptrRead<{}>", Self::type_surface(&ty)),
                args: vec![address],
            });
        }

        if self.check_ident_name("addr") {
            self.expect_ident()?;
            let target = self.parse_plain_context_ref()?;
            return self.parse_postfix_expr(Expr::Call {
                callee: "addrOf".to_string(),
                args: vec![Expr::ContextRef(target)],
            });
        }

        if (self.check_ident_name("size") || self.check_ident_name("align"))
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(
                    TokenKind::Identifier(_)
                        | TokenKind::KeywordStr
                        | TokenKind::KeywordBinary
                        | TokenKind::KeywordFn
                        | TokenKind::KeywordRef
                )
            )
        {
            let operation = self.expect_ident()?;
            let ty = self.expect_type_name()?;
            let callee = if operation.eq_ignore_ascii_case("size") {
                "sizeOf"
            } else {
                "alignOf"
            };
            return self.parse_postfix_expr(Expr::Call {
                callee: format!("{}<{}>", callee, Self::type_surface(&ty)),
                args: Vec::new(),
            });
        }

        if self.check_ident_name("offset") {
            self.expect_ident()?;
            let owner = self.expect_ident()?;
            self.expect_kind(TokenKind::Dot)?;
            let field = self.expect_ident()?;
            return self.parse_postfix_expr(Expr::Call {
                callee: format!("offsetOf<{owner}>"),
                args: vec![Expr::Identifier(field)],
            });
        }

        if self.check_ident_name("b")
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::String(_))
            )
        {
            self.advance();
            let bytes = self
                .match_string()
                .expect("byte literal lookahead guaranteed a string");
            return self.parse_postfix_expr(Expr::Call {
                callee: "byteLiteral".into(),
                args: vec![Expr::String(bytes)],
            });
        }

        if let Some(s) = self.match_string() {
            return Ok(Expr::String(s));
        }

        if let TokenKind::Char(ch) = self.current().kind.clone() {
            self.advance();
            return Ok(Expr::Char(ch));
        }

        if let Some(n) = self.match_number() {
            return Ok(Expr::Number(n));
        }

        if self.match_kind(&TokenKind::LBracket) {
            let mut items = Vec::new();

            if !self.check(&TokenKind::RBracket) {
                loop {
                    items.push(self.parse_expr()?);
                    if self.match_kind(&TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
            }

            self.expect_kind(TokenKind::RBracket)?;
            return Ok(Expr::VectorLiteral(items));
        }

        if self.check_ident_name("fill") && self.check_next_kind(&TokenKind::LBracket) {
            return self.parse_fill_literal_expr();
        }

        if self.check(&TokenKind::At) {
            // Compatibility form: @fn(name, arg1, arg2, ...)
            // Keep @fn(path) as reference form by requiring at least one comma.
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::KeywordFn)
            ) && matches!(
                self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                Some(TokenKind::LParen)
            ) {
                let save = self.pos;
                self.expect_kind(TokenKind::At)?;
                self.expect_kind(TokenKind::KeywordFn)?;
                self.expect_kind(TokenKind::LParen)?;
                let callee_path = self.parse_ref_path()?;

                if self.match_kind(&TokenKind::Comma) {
                    let mut args = Vec::new();
                    loop {
                        args.push(self.parse_expr()?);
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                    self.expect_kind(TokenKind::RParen)?;

                    let expr = if callee_path.len() == 1 {
                        Expr::Call {
                            callee: callee_path[0].clone(),
                            args,
                        }
                    } else {
                        Expr::QualifiedCall {
                            path: callee_path,
                            args,
                        }
                    };
                    return self.parse_postfix_expr(expr);
                }

                self.pos = save;
            }

            let save = self.pos;
            let ctx = self.parse_context_ref()?;

            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                return Ok(Expr::StrictCheckExpr {
                    target: ctx,
                    ty,
                    value: Box::new(value),
                });
            }

            self.pos = save;
            let ctx = self.parse_context_ref()?;

            if self.check(&TokenKind::LParen) {
                let mut args = Vec::new();
                self.expect_kind(TokenKind::LParen)?;
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(TokenKind::RParen)?;

                let expr = if ctx.path.len() >= 2 {
                    let method = ctx.path.last().cloned().unwrap();
                    let receiver_path = ctx.path[..ctx.path.len() - 1].to_vec();

                    Expr::MethodCall {
                        receiver: Box::new(Expr::ContextRef(ContextRef::plain(receiver_path))),
                        method,
                        args,
                    }
                } else {
                    Expr::Call {
                        callee: ctx.path[0].clone(),
                        args,
                    }
                };

                return self.parse_postfix_expr(expr);
            }

            let expr = Expr::ContextRef(ctx);
            return self.parse_postfix_expr(expr);
        }

        if self.peek_is_ident() {
            let mut path = vec![self.expect_ident()?];

            loop {
                let dot_path = self.check(&TokenKind::Dot)
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.kind),
                        Some(TokenKind::Identifier(_))
                            | Some(TokenKind::KeywordFn)
                            | Some(TokenKind::KeywordMove)
                            | Some(TokenKind::KeywordState)
                            | Some(TokenKind::KeywordLabel)
                            | Some(TokenKind::KeywordBind)
                            | Some(TokenKind::KeywordBinary)
                    );
                let colon_path = self.check_double_colon()
                    && matches!(
                        self.tokens.get(self.pos + 2).map(|t| &t.kind),
                        Some(TokenKind::Identifier(_))
                            | Some(TokenKind::KeywordFn)
                            | Some(TokenKind::KeywordMove)
                            | Some(TokenKind::KeywordState)
                            | Some(TokenKind::KeywordLabel)
                            | Some(TokenKind::KeywordBind)
                            | Some(TokenKind::KeywordBinary)
                    );
                if dot_path {
                    self.advance();
                } else if colon_path {
                    self.expect_double_colon()?;
                } else {
                    break;
                }
                path.push(self.expect_context_segment()?);
            }

            if path.len() == 1 && self.in_kernel_or_system_context() {
                if let Some(canonical) = Self::compact_kernel_call_name(&path[0]) {
                    path[0] = canonical.to_string();
                }
            }

            if self.match_kind(&TokenKind::BangShift) {
                let ty = self.expect_type_name()?;
                self.expect_kind(TokenKind::Colon)?;
                let value = self.parse_expr()?;
                return Ok(Expr::StrictCheckExpr {
                    target: ContextRef::plain(path),
                    ty,
                    value: Box::new(value),
                });
            }

            // `Read*(p)`, `Write*(p, value)` and companions are the compact
            // C-shaped surface for the canonical typed pointer builtins. They
            // are intentionally normalized here, before semantic analysis.
            if path.len() == 1
                && Self::compact_pointer_call_name(&path[0]).is_some()
                && self.check(&TokenKind::Star)
                && matches!(
                    self.tokens.get(self.pos + 1).map(|token| &token.kind),
                    Some(TokenKind::LParen) | Some(TokenKind::Less)
                )
            {
                let canonical = Self::compact_pointer_call_name(&path[0])
                    .expect("compact pointer helper was checked");
                self.expect_kind(TokenKind::Star)?;
                let call_type_args = self.parse_optional_call_type_args();
                self.expect_kind(TokenKind::LParen)?;
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(TokenKind::RParen)?;
                let callee = match call_type_args {
                    Some(type_args) => format!("{}<{}>", canonical, type_args.join(",")),
                    None => canonical.to_string(),
                };
                return self.parse_postfix_expr(Expr::Call { callee, args });
            }

            if path.len() == 1 && self.match_kind(&TokenKind::LBrace) {
                let mut fields = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let name = self.expect_ident()?;
                        self.expect_kind(TokenKind::Colon)?;
                        fields.push((name, self.parse_expr()?));
                        if self.match_kind(&TokenKind::Comma) {
                            if self.check(&TokenKind::RBrace) {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(TokenKind::RBrace)?;
                return self.parse_postfix_expr(Expr::StructLiteral {
                    ty: path.remove(0),
                    fields,
                });
            }

            let call_type_args_save = self.pos;
            let call_type_args = self.parse_optional_call_type_args();

            if self.match_kind(&TokenKind::LParen) {
                let mut args = Vec::new();
                let mut named_fields = Vec::new();
                let uses_named_fields = self.peek_is_ident()
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|token| &token.kind),
                        Some(TokenKind::Colon)
                    );
                if !self.check(&TokenKind::RParen) {
                    loop {
                        if uses_named_fields {
                            let name = self.expect_ident()?;
                            self.expect_kind(TokenKind::Colon)?;
                            named_fields.push((name, self.parse_expr()?));
                        } else {
                            args.push(self.parse_expr()?);
                        }
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(TokenKind::RParen)?;
                let had_explicit_type_args = call_type_args.is_some();
                if path.len() == 1
                    && (Self::compact_pointer_name_is_unambiguous(&path[0])
                        || (had_explicit_type_args
                            && Self::compact_pointer_call_name(&path[0]).is_some()))
                {
                    path[0] = Self::compact_pointer_call_name(&path[0])
                        .expect("compact pointer call was checked")
                        .to_string();
                }
                if uses_named_fields {
                    if path.len() != 1 || call_type_args.is_some() {
                        return self.error_here(
                            "E_PARSE_STRUCT_LITERAL_001: named fields require an unqualified nominal type constructor",
                        );
                    }
                    return self.parse_postfix_expr(Expr::StructLiteral {
                        ty: path.remove(0),
                        fields: named_fields,
                    });
                }
                if let Some(type_args) = call_type_args {
                    if let Some(last) = path.last_mut() {
                        *last = format!("{}<{}>", last, type_args.join(","));
                    }
                }

                // Type metadata stays explicit while avoiding visual generic
                // noise: `sizeOf(T)` and `alignOf(T)` normalize to the same
                // canonical AST consumed by sema, ABI and every backend.
                if path.len() == 1 && !had_explicit_type_args {
                    let query = path[0].to_ascii_lowercase();
                    let type_arg = match args.first() {
                        Some(Expr::Identifier(name)) => Some(name.clone()),
                        _ => None,
                    };
                    if matches!(query.as_str(), "sizeof" | "alignof")
                        && args.len() == 1
                        && type_arg.is_some()
                    {
                        path[0] = format!("{}<{}>", path[0], type_arg.expect("checked"));
                        args.clear();
                    } else if query == "offsetof" && args.len() == 2 && type_arg.is_some() {
                        path[0] = format!("{}<{}>", path[0], type_arg.expect("checked"));
                        args.remove(0);
                    }
                }

                if path.len() == 1 {
                    if let Some(canonical) = Self::compact_raw_call_name(&path[0], args.len()) {
                        path[0] = canonical.to_string();
                    }
                }

                if path.len() == 1 {
                    return self.parse_postfix_expr(Expr::Call {
                        callee: path[0].clone(),
                        args,
                    });
                } else {
                    return self.parse_postfix_expr(Expr::QualifiedCall { path, args });
                }
            }

            if call_type_args.is_some() {
                self.pos = call_type_args_save;
            }

            if path.len() == 1 {
                return self.parse_postfix_expr(Expr::Identifier(path.remove(0)));
            }

            // v1.0.1: qualified symbols without call are first-class context references.
            // This preserves typed continuity for deep routes (module/resource/value paths)
            // instead of forcing textual fallbacks or parser-level rejection.
            if inspect_runtime_path_from_segments(&path).is_some() {
                return self.parse_postfix_expr(Expr::ContextRef(ContextRef::plain(path)));
            }
            return self.parse_postfix_expr(Expr::ContextRef(ContextRef::plain(path)));
        }

        if self.match_kind(&TokenKind::LParen) {
            let expr = self.parse_expr()?;
            self.expect_kind(TokenKind::RParen)?;
            return self.parse_postfix_expr(expr);
        }

        self.error_here("Invalid expression")
    }

    fn parse_fill_literal_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.expect_ident()?;
        self.expect_kind(TokenKind::LBracket)?;
        let value = self.parse_expr()?;
        if self.match_kind(&TokenKind::Comma) {
            self.expect_kind(TokenKind::Dot)?;
            self.expect_kind(TokenKind::Dot)?;
            self.expect_kind(TokenKind::Dot)?;
            self.expect_kind(TokenKind::Comma)?;
            let _end = self.parse_expr()?;
        }
        self.expect_kind(TokenKind::RBracket)?;
        Ok(Expr::ArrayLiteral(vec![value]))
    }

    fn parse_closure_expr(&mut self, is_move: bool, is_async: bool) -> Result<Expr, Diagnostic> {
        let mut params = Vec::new();

        if self.match_kind(&TokenKind::OrOr) {
            // no params
        } else {
            self.expect_kind(TokenKind::Pipe)?;
            if !self.check(&TokenKind::Pipe) {
                loop {
                    let name = self.expect_ident()?;
                    if !(self.check(&TokenKind::Colon) || self.check(&TokenKind::Shift2)) {
                        return self.error_here(
                            "E_HIGH_CLOSURE_PARAM_001: closure parameter must declare an explicit type",
                        );
                    }
                    if !self.match_kind(&TokenKind::Shift2) {
                        self.expect_kind(TokenKind::Colon)?;
                    }
                    let ty = self.expect_single_type_name()?;
                    params.push(ClosureParam { name, ty });
                    if self.match_kind(&TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
            }
            self.expect_kind(TokenKind::Pipe)?;
        }

        self.expect_kind(TokenKind::Arrow)?;
        let return_type = self.expect_type_name()?;
        let effects = self.parse_optional_effects()?;
        let captures = if self.match_ident_name("captures") {
            self.parse_closure_capture_list()?
        } else {
            Vec::new()
        };
        let body = self.parse_block_stmts()?;

        Ok(Expr::Closure(ClosureExpr {
            is_async,
            is_move,
            params,
            return_type,
            captures,
            effects,
            body,
        }))
    }

    fn parse_closure_capture_list(&mut self) -> Result<Vec<CaptureItem>, Diagnostic> {
        self.expect_kind(TokenKind::LParen)?;
        let mut captures = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let mode = if self.match_kind(&TokenKind::KeywordMove) {
                    CaptureMode::Move
                } else if self.match_kind(&TokenKind::KeywordRef) {
                    CaptureMode::Ref
                } else if self.match_ident_name("mut") {
                    CaptureMode::Mut
                } else {
                    CaptureMode::Value
                };
                let _ = self.match_kind(&TokenKind::At);
                let name = self.expect_ident()?;
                captures.push(CaptureItem { mode, name });
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
        }
        self.expect_kind(TokenKind::RParen)?;
        Ok(captures)
    }

    fn parse_dev_error_list(&mut self) -> Result<Vec<String>, Diagnostic> {
        self.expect_kind(TokenKind::LParen)?;
        let mut errors = Vec::new();
        errors.push(self.parse_dev_error_item()?);
        while self.match_kind(&TokenKind::Pipe) {
            errors.push(self.parse_dev_error_item()?);
        }
        self.expect_kind(TokenKind::RParen)?;
        Ok(errors)
    }

    fn parse_dev_error_item(&mut self) -> Result<String, Diagnostic> {
        let mut parts = vec![self.expect_ident()?];
        while self.match_kind(&TokenKind::Dot) {
            parts.push(self.expect_ident()?);
        }
        let name = parts.join(".");
        // Compatibility: tolerate contextual suffix in dev payloads.
        // Example: dev(irregular @security)
        if self.match_kind(&TokenKind::At) {
            let _ctx = self.expect_ident()?;
        }
        Ok(name)
    }

    fn parse_context_header_bracket(&mut self) -> Result<Context, Diagnostic> {
        self.expect_kind(TokenKind::LBracket)?;
        self.expect_kind(TokenKind::At)?;
        if self.check(&TokenKind::RBracket) {
            self.expect_kind(TokenKind::RBracket)?;
            return Ok(Context::Root);
        }
        let ctx = self.expect_ident()?;
        self.expect_kind(TokenKind::RBracket)?;
        Ok(parse_context(ctx))
    }

    fn parse_postfix_expr(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
        loop {
            if self.match_kind(&TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.match_kind(&TokenKind::Comma) { break; }
                    }
                }
                self.expect_kind(TokenKind::RParen)?;
                let shape = match &expr {
                    Expr::Identifier(_) => 1,
                    Expr::ContextRef(_) => 2,
                    _ => 3,
                };
                expr = match ncc_stage0_bridge::call_target(shape) {
                    1 => {
                        let Expr::Identifier(callee) = expr else { unreachable!() };
                        Expr::Call { callee, args }
                    }
                    2 => {
                        let Expr::ContextRef(context) = expr else { unreachable!() };
                        Expr::QualifiedCall { path: context.path, args }
                    }
                    3 => {
                        args.insert(0, expr);
                        Expr::Call { callee: "call".to_string(), args }
                    }
                    _ => unreachable!("invalid native call target ABI result"),
                };
                continue;
            }
            if self.check(&TokenKind::Question) && !self.question_has_top_level_colon_from_here() {
                self.advance();
                expr = Expr::Unary {
                    op: UnaryOp::TryPropagate,
                    expr: Box::new(expr),
                };
                continue;
            }

            if self.match_kind(&TokenKind::PlusPlus) {
                expr = match expr {
                    Expr::Identifier(name) => Expr::PostIncrement {
                        target: ContextRef::plain(vec![name]),
                    },
                    Expr::ContextRef(ctx) if ctx.path.len() == 1 => {
                        Expr::PostIncrement { target: ctx }
                    }
                    Expr::ContextRef(ctx) if ctx.path.len() >= 2 => {
                        Expr::FieldPostIncrement { target: ctx }
                    }
                    _ => {
                        return self.error_here(
                            "E_HIGH_POST_INCREMENT_TARGET_001: '++' expects a mutable symbol or field",
                        );
                    }
                };
                continue;
            }

            if self.match_kind(&TokenKind::Bang) {
                continue;
            }

            if self.match_kind(&TokenKind::LBracket) {
                if self.check(&TokenKind::RBracket) {
                    return self.error_here("invalid index expression");
                }
                if self.check(&TokenKind::Dot)
                    && matches!(
                        self.tokens.get(self.pos + 1).map(|t| &t.kind),
                        Some(TokenKind::Dot)
                    )
                {
                    self.advance(); // '.'
                    self.advance(); // '.'
                    let len = self.parse_expr()?;
                    self.expect_kind(TokenKind::RBracket)?;
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        method: "regionView".to_string(),
                        args: vec![len],
                    };
                    continue;
                }
                let mut indices = Vec::new();
                loop {
                    indices.push(self.parse_expr()?);
                    if self.match_kind(&TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
                self.expect_kind(TokenKind::RBracket)?;
                let idx = if indices.len() == 1 {
                    indices.remove(0)
                } else {
                    Expr::ArrayLiteral(indices)
                };
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(idx),
                };
                continue;
            }

            if self.check(&TokenKind::Dot)
                && matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Identifier(_))
                        | Some(TokenKind::KeywordState)
                        | Some(TokenKind::KeywordLabel)
                )
                && !matches!(
                    self.tokens.get(self.pos + 2).map(|t| &t.kind),
                    Some(TokenKind::LParen)
                )
            {
                self.advance(); // '.'
                let field = self.expect_ident()?;
                expr = Expr::MethodCall {
                    receiver: Box::new(expr),
                    method: field,
                    args: Vec::new(),
                };
                continue;
            }

            if self.check(&TokenKind::Dot)
                && matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Identifier(_))
                        | Some(TokenKind::KeywordState)
                        | Some(TokenKind::KeywordLabel)
                )
                && matches!(
                    self.tokens.get(self.pos + 2).map(|t| &t.kind),
                    Some(TokenKind::LParen)
                )
            {
                self.advance(); // '.'
                let method = self.expect_ident()?;
                self.expect_kind(TokenKind::LParen)?;
                let mut args = Vec::new();

                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }

                self.expect_kind(TokenKind::RParen)?;
                expr = Expr::MethodCall {
                    receiver: Box::new(expr),
                    method,
                    args,
                };
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn question_has_top_level_colon_from_here(&self) -> bool {
        debug_assert!(self.check(&TokenKind::Question));
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        for token in self.tokens.iter().skip(self.pos + 1) {
            match token.kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RParen => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        return false;
                    }
                    paren_depth = paren_depth.saturating_sub(1);
                }
                TokenKind::RBracket => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        return false;
                    }
                    bracket_depth = bracket_depth.saturating_sub(1);
                }
                TokenKind::RBrace => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        return false;
                    }
                    brace_depth = brace_depth.saturating_sub(1);
                }
                TokenKind::Colon if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    return true;
                }
                TokenKind::Semicolon | TokenKind::Comma | TokenKind::Eof
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 =>
                {
                    return false;
                }
                _ => {}
            }
        }
        false
    }

    fn parse_context_ref(&mut self) -> Result<ContextRef, Diagnostic> {
        self.expect_kind(TokenKind::At)?;
        if self.check(&TokenKind::Dot) {
            return self.error_here("invalid referential form after '@'");
        }
        if self.match_kind(&TokenKind::KeywordFn) {
            self.expect_kind(TokenKind::LParen)?;
            let path = self.parse_ref_path()?;
            self.expect_kind(TokenKind::RParen)?;
            return Ok(ContextRef::from_fn(path));
        }
        if self.check(&TokenKind::KeywordProbe)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::LParen)
            )
        {
            self.expect_kind(TokenKind::KeywordProbe)?;
            self.expect_kind(TokenKind::LParen)?;
            let path = self.parse_ref_path()?;
            self.expect_kind(TokenKind::RParen)?;
            return Ok(ContextRef::from_probe(path));
        }
        if !Self::is_context_segment_start(&self.current().kind) {
            return self.error_here("expected resolvable symbol after '@'");
        }

        let mut path = vec![self.expect_context_segment()?];
        while self.check(&TokenKind::Dot)
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
                    | Some(TokenKind::KeywordState)
                    | Some(TokenKind::KeywordLabel)
                    | Some(TokenKind::KeywordBind)
                    | Some(TokenKind::KeywordBinary)
            )
        {
            self.advance();
            path.push(self.expect_context_segment()?);
        }
        if self.check(&TokenKind::Dot) {
            return self.error_here("invalid referential form after '@'");
        }
        Ok(ContextRef::plain(path))
    }

    fn try_parse_method_array_field_projection_stmt(&mut self) -> Result<Option<Stmt>, Diagnostic> {
        let save = self.pos;
        self.expect_kind(TokenKind::At)?;
        if !Self::is_context_segment_start(&self.current().kind) {
            self.pos = save;
            return Ok(None);
        }
        let target = ContextRef::plain(vec![self.expect_context_segment()?]);
        if !self.match_kind(&TokenKind::Dot) {
            self.pos = save;
            return Ok(None);
        }
        let method = self.expect_ident()?;
        if !matches!(method.as_str(), "get" | "readAt") {
            self.pos = save;
            return Ok(None);
        }

        self.expect_kind(TokenKind::LParen)?;
        let index = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen)?;

        let mut fields = Vec::new();
        while self.check(&TokenKind::Dot)
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
                    | Some(TokenKind::KeywordState)
                    | Some(TokenKind::KeywordFlag)
                    | Some(TokenKind::KeywordLabel)
            )
            && !matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.kind),
                Some(TokenKind::LParen)
            )
        {
            self.advance();
            fields.push(self.expect_ident()?);
        }
        if fields.is_empty() {
            return self.error_here("array get lvalue expects a field path after get(index)");
        }

        if self.match_kind(&TokenKind::Shift2) {
            let _ty = self.expect_type_name()?;
            self.expect_projection_init_separator()?;
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Some(Stmt::Expr(Expr::MethodCall {
                receiver: Box::new(Expr::ContextRef(target)),
                method: "arrayFieldWrite".to_string(),
                args: vec![index, Expr::ContextRef(ContextRef::plain(fields)), value],
            })));
        }

        if self.match_kind(&TokenKind::BangShift) {
            let _ty = self.expect_type_name()?;
            self.expect_kind(TokenKind::Colon)?;
            let value = self.parse_expr()?;
            self.expect_kind(TokenKind::Semicolon)?;
            return Ok(Some(Stmt::Expr(Expr::MethodCall {
                receiver: Box::new(Expr::ContextRef(target)),
                method: "arrayFieldWrite".to_string(),
                args: vec![index, Expr::ContextRef(ContextRef::plain(fields)), value],
            })));
        }

        self.pos = save;
        Ok(None)
    }

    fn parse_context_ref_with_indices(&mut self) -> Result<IndexedContextRef, Diagnostic> {
        let target = self.parse_context_ref()?;
        let mut indices = Vec::new();

        while self.match_kind(&TokenKind::LBracket) {
            if self.check(&TokenKind::RBracket) {
                return self.error_here("invalid index expression");
            }

            loop {
                indices.push(self.parse_expr()?);
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }

            self.expect_kind(TokenKind::RBracket)?;
        }

        let mut indexed_fields = Vec::new();
        while !indices.is_empty()
            && self.check(&TokenKind::Dot)
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
                    | Some(TokenKind::KeywordState)
                    | Some(TokenKind::KeywordFlag)
                    | Some(TokenKind::KeywordLabel)
            )
            && !matches!(
                self.tokens.get(self.pos + 2).map(|t| &t.kind),
                Some(TokenKind::LParen)
            )
        {
            self.advance();
            indexed_fields.push(self.expect_ident()?);
        }
        let indexed_field = if indexed_fields.is_empty() {
            None
        } else {
            Some(indexed_fields)
        };

        Ok((target, indices, indexed_field))
    }

    fn parse_fn_ref_expr(&mut self) -> Result<FnRef, Diagnostic> {
        self.expect_kind(TokenKind::At)?;
        if !self.match_kind(&TokenKind::KeywordFn) {
            return self.error_here("Expected @fn(...)");
        }
        self.expect_kind(TokenKind::LParen)?;
        let path = self.parse_ref_path()?;
        self.expect_kind(TokenKind::RParen)?;
        Ok(FnRef { path })
    }

    fn parse_main_fn_ref_expr(&mut self) -> Result<FnRef, Diagnostic> {
        if self.check(&TokenKind::At) {
            return self.parse_fn_ref_expr();
        }

        match self.current().kind {
            TokenKind::Identifier(_) | TokenKind::KeywordBind | TokenKind::KeywordBinary => {
                let path = self.parse_ref_path()?;
                Ok(FnRef { path })
            }
            _ => self.error_here("Expected @fn(...) or qualified function path"),
        }
    }

    fn parse_main_profile(&mut self) -> Result<String, Diagnostic> {
        self.expect_kind(TokenKind::At)?;
        self.expect_kind(TokenKind::Less)?;
        let mut parts = vec![self.expect_ident()?];
        while self.match_kind(&TokenKind::Dot) {
            parts.push(self.expect_ident()?);
        }
        self.expect_kind(TokenKind::Greater)?;
        Ok(parts.join("."))
    }

    fn expect_ref_segment(&mut self) -> Result<String, Diagnostic> {
        match &self.current().kind {
            TokenKind::Identifier(name) => {
                let out = name.clone();
                self.advance();
                Ok(out)
            }
            TokenKind::KeywordBind => {
                self.advance();
                Ok("bind".to_string())
            }
            TokenKind::KeywordLocal => {
                self.advance();
                Ok("local".to_string())
            }
            TokenKind::KeywordBinary => {
                self.advance();
                Ok("binary".to_string())
            }
            _ => self.error_here("Expected identifier"),
        }
    }

    fn expect_context_segment(&mut self) -> Result<String, Diagnostic> {
        match &self.current().kind {
            TokenKind::KeywordBind => {
                self.advance();
                Ok("bind".to_string())
            }
            TokenKind::Identifier(name) => {
                let out = name.clone();
                self.advance();
                Ok(out)
            }
            kind => {
                if let Some(word) = Self::primitive_directive_word_from_keyword(kind) {
                    self.advance();
                    Ok(word.to_string())
                } else {
                    self.error_here("Expected identifier")
                }
            }
        }
    }

    fn is_context_segment_start(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Identifier(_)
                | TokenKind::KeywordState
                | TokenKind::KeywordLocal
                | TokenKind::KeywordRaw
                | TokenKind::KeywordLabel
                | TokenKind::KeywordBind
                | TokenKind::KeywordBinary
                | TokenKind::KeywordAnchor
                | TokenKind::KeywordSlot
                | TokenKind::KeywordPattern
                | TokenKind::KeywordMask
                | TokenKind::KeywordWindow
                | TokenKind::KeywordSignal
                | TokenKind::KeywordBus
                | TokenKind::KeywordTiming
                | TokenKind::KeywordEmit
                | TokenKind::KeywordSample
                | TokenKind::KeywordLatch
                | TokenKind::KeywordAssert
                | TokenKind::KeywordRupture
                | TokenKind::KeywordBridge
                | TokenKind::KeywordSeal
                | TokenKind::KeywordReg
                | TokenKind::KeywordFlag
                | TokenKind::KeywordInst
        )
    }

    fn parse_ref_path(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut path = vec![self.expect_ref_segment()?];
        while self.check(&TokenKind::Dot)
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Identifier(_))
                    | Some(TokenKind::KeywordState)
                    | Some(TokenKind::KeywordLocal)
                    | Some(TokenKind::KeywordLabel)
                    | Some(TokenKind::KeywordBind)
                    | Some(TokenKind::KeywordBinary)
            )
        {
            self.advance();
            path.push(self.expect_ref_segment()?);
        }
        Ok(path)
    }

    fn expect_type_name(&mut self) -> Result<TypeName, Diagnostic> {
        let mut types = Vec::new();

        types.push(self.expect_single_type_name()?);

        while self.match_kind(&TokenKind::Pipe) {
            types.push(self.expect_single_type_name()?);
        }

        if types.len() == 1 {
            Ok(types.remove(0))
        } else {
            Ok(TypeName::Union(types))
        }
    }

    fn expect_type_generic_arg(&mut self) -> Result<TypeName, Diagnostic> {
        match &self.current().kind {
            TokenKind::Number(n) => {
                let out = n.clone();
                self.advance();
                Ok(TypeName::Simple(out))
            }
            _ => self.expect_type_name(),
        }
    }

    fn expect_single_type_name(&mut self) -> Result<TypeName, Diagnostic> {
        if self.check(&TokenKind::KeywordFn)
            && matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::LParen)
            )
        {
            self.expect_kind(TokenKind::KeywordFn)?;
            self.expect_kind(TokenKind::LParen)?;
            let mut args = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    args.push(self.expect_type_name()?);
                    if self.match_kind(&TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
            }
            self.expect_kind(TokenKind::RParen)?;
            self.expect_kind(TokenKind::Arrow)?;
            args.push(self.expect_type_name()?);
            return Ok(TypeName::Generic {
                base: "fn".to_string(),
                args,
            });
        }

        let mut name = self.expect_type_ident()?;
        if name.eq_ignore_ascii_case("p") {
            name = "ptr".to_string();
        }

        if matches!(name.as_str(), "ptr" | "ref") && self.match_ident_name("mut") {
            name.push_str(" mut");
        }

        if name.eq_ignore_ascii_case("dyn") {
            if self.match_lt() {
                let trait_ref = self.expect_type_generic_arg()?;
                while self.match_kind(&TokenKind::Comma) {
                    let _ = self.expect_type_generic_arg()?;
                }
                self.expect_gt()?;
                return Ok(TypeName::Generic {
                    base: "dyn".to_string(),
                    args: vec![trait_ref],
                });
            }
            let trait_ref = self.expect_single_type_name()?;
            return Ok(TypeName::Generic {
                base: "dyn".to_string(),
                args: vec![trait_ref],
            });
        }

        while self.match_kind(&TokenKind::Dot) {
            name.push('.');
            name.push_str(&self.expect_type_ident()?);
        }

        let mut ty = TypeName::Simple(name);

        // --- GENERICS ---
        if self.match_lt() {
            let mut args = Vec::new();

            loop {
                args.push(self.expect_type_generic_arg()?);

                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }

            self.expect_gt()?;

            if let TypeName::Simple(base) = ty {
                if base == "vector" && args.len() == 1 {
                    ty = TypeName::Vector(Box::new(args.remove(0)));
                } else {
                    ty = TypeName::Generic { base, args };
                }
            }
        }

        // `skk*` and `skk**` are sovereign nominal pointer classes. All
        // other `T*` spellings normalize to the canonical `ptr<T>` AST form.
        if matches!(&ty, TypeName::Simple(base) if base.eq_ignore_ascii_case("skk")) {
            let mut depth = 0usize;
            while self.match_kind(&TokenKind::Star) {
                depth += 1;
            }
            if depth > 0 {
                ty = TypeName::Simple(format!("skk{}", "*".repeat(depth)));
            }
        } else {
            while self.match_kind(&TokenKind::Star) {
                ty = TypeName::Generic {
                    base: "ptr".to_string(),
                    args: vec![ty],
                };
            }
        }

        // --- ARRAYS ---
        while self.match_kind(&TokenKind::LBracket) {
            if self.match_kind(&TokenKind::RBracket) {
                ty = TypeName::Array(Box::new(ty));
                continue;
            }

            let len =
                match &self.current().kind {
                    TokenKind::Number(n) | TokenKind::Identifier(n) => {
                        let out = n.clone();
                        self.advance();
                        out
                    }
                    _ => return self.error_here(
                        "invalid array length syntax: expected integer literal or const identifier",
                    ),
                };

            self.expect_kind(TokenKind::RBracket)?;
            ty = TypeName::Generic {
                base: "array".to_string(),
                args: vec![ty, TypeName::Simple(len)],
            };
        }

        Ok(ty)
    }

    fn expect_type_ident(&mut self) -> Result<String, Diagnostic> {
        match self.advance().kind {
            TokenKind::Identifier(s) => Ok(s),
            TokenKind::KeywordStr => Ok("str".to_string()),
            TokenKind::KeywordBinary => Ok("binary".to_string()),
            TokenKind::KeywordFn => Ok("fn".to_string()),
            TokenKind::KeywordRef => Ok("ref".to_string()),
            _ => self.error_here("Expected identifier"),
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match &self.current().kind {
            TokenKind::OrOr => Some((BinaryOp::OrOr, 1)),
            TokenKind::AndAnd => Some((BinaryOp::AndAnd, 2)),
            TokenKind::Pipe => Some((BinaryOp::BitOr, 3)),
            TokenKind::Caret => Some((BinaryOp::BitXor, 4)),
            TokenKind::Amp => Some((BinaryOp::BitAnd, 5)),
            TokenKind::EqEq => Some((BinaryOp::EqEq, 6)),
            TokenKind::NotEq => Some((BinaryOp::NotEq, 6)),
            TokenKind::Greater => Some((BinaryOp::Greater, 7)),
            TokenKind::Less => Some((BinaryOp::Less, 7)),
            TokenKind::GreaterEq => Some((BinaryOp::GreaterEq, 7)),
            TokenKind::LessEq => Some((BinaryOp::LessEq, 7)),
            TokenKind::Shift2 => Some((BinaryOp::ShiftRight, 8)),
            TokenKind::Shift3 => Some((BinaryOp::ShiftRightLogical, 8)),
            TokenKind::ShiftLeft => Some((BinaryOp::ShiftLeft, 8)),
            TokenKind::Plus => Some((BinaryOp::Plus, 9)),
            TokenKind::Minus => Some((BinaryOp::Minus, 9)),
            TokenKind::Star => Some((BinaryOp::Star, 10)),
            TokenKind::Slash => Some((BinaryOp::Slash, 10)),
            TokenKind::Percent => Some((BinaryOp::Percent, 10)),
            _ => None,
        }
    }

    fn is_section_start(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::KeywordStc
                | TokenKind::KeywordLocal
                | TokenKind::KeywordBinary
                | TokenKind::KeywordReg
                | TokenKind::KeywordFlag
                | TokenKind::KeywordLabel
                | TokenKind::KeywordInst
                | TokenKind::KeywordFn
                | TokenKind::KeywordUse
                | TokenKind::KeywordMove
                | TokenKind::KeywordProbe
                | TokenKind::KeywordBind
                | TokenKind::KeywordRaw
                | TokenKind::KeywordStructure
                | TokenKind::KeywordStr
                | TokenKind::KeywordFinal
                | TokenKind::KeywordFa
                | TokenKind::KeywordRef
                | TokenKind::KeywordMain
                | TokenKind::KeywordSkkMain
                | TokenKind::KeywordAnchor
                | TokenKind::KeywordSlot
                | TokenKind::KeywordPattern
                | TokenKind::KeywordMask
                | TokenKind::KeywordWindow
                | TokenKind::KeywordSignal
                | TokenKind::KeywordBus
                | TokenKind::KeywordState
                | TokenKind::KeywordTiming
                | TokenKind::KeywordEmit
                | TokenKind::KeywordSample
                | TokenKind::KeywordLatch
                | TokenKind::KeywordAssert
                | TokenKind::KeywordRupture
                | TokenKind::KeywordBridge
                | TokenKind::KeywordSeal
        ) || matches!(&self.current().kind, TokenKind::Identifier(s)
            if s == "to"
                || s.eq_ignore_ascii_case("skk")
                || s == "module"
                || s == "use"
                || s == "export"
                || s == "interface"
                || s == "trait"
                || s == "impl"
                || s == "contract"
                || s == "enum"
                || s == "const"
                || s == "comptime"
                || s == "macro"
                || s == "risk"
                || s == "policy"
                || s == "authority")
            || self.is_type_contract_section_start()
            || self.is_foreign_identifier_section_start()
            || self.is_primitive_identifier_directive_section_start()
            || self.is_security_identifier_section_start()
            || self.peek_starts_canonical_type_decl()
            || self.peek_starts_function()
    }

    fn is_stc_boundary_start(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::KeywordStc
                | TokenKind::KeywordLocal
                | TokenKind::KeywordBinary
                | TokenKind::KeywordReg
                | TokenKind::KeywordFlag
                | TokenKind::KeywordLabel
                | TokenKind::KeywordInst
                | TokenKind::KeywordFn
                | TokenKind::KeywordUse
                | TokenKind::KeywordMove
                | TokenKind::KeywordProbe
                | TokenKind::KeywordBind
                | TokenKind::KeywordRaw
                | TokenKind::KeywordStructure
                | TokenKind::KeywordStr
                | TokenKind::KeywordFinal
                | TokenKind::KeywordFa
                | TokenKind::KeywordRef
                | TokenKind::KeywordSkkMain
                | TokenKind::KeywordAnchor
                | TokenKind::KeywordSlot
                | TokenKind::KeywordPattern
                | TokenKind::KeywordMask
                | TokenKind::KeywordWindow
                | TokenKind::KeywordSignal
                | TokenKind::KeywordBus
                | TokenKind::KeywordState
                | TokenKind::KeywordTiming
                | TokenKind::KeywordEmit
                | TokenKind::KeywordSample
                | TokenKind::KeywordLatch
                | TokenKind::KeywordAssert
                | TokenKind::KeywordRupture
                | TokenKind::KeywordBridge
                | TokenKind::KeywordSeal
        ) || matches!(&self.current().kind, TokenKind::Identifier(s)
            if s == "to"
                || s.eq_ignore_ascii_case("skk")
                || s == "module"
                || s == "use"
                || s == "export"
                || s == "interface"
                || s == "trait"
                || s == "impl"
                || s == "contract"
                || s == "enum"
                || s == "const"
                || s == "comptime"
                || s == "macro")
            || self.is_type_contract_section_start()
            || self.is_foreign_identifier_section_start()
            || self.is_primitive_identifier_directive_section_start()
            || self.is_security_identifier_section_start()
            || self.peek_starts_canonical_type_decl()
            || self.peek_starts_function()
    }

    fn is_local_boundary_start(&self) -> bool {
        self.is_stc_boundary_start()
    }

    fn current_starts_local_decl_keyword(&self) -> bool {
        match &self.current().kind {
            TokenKind::Identifier(name)
                if matches!(name.as_str(), "const" | "type" | "metadata") =>
            {
                matches!(
                    self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                    Some(TokenKind::Identifier(_))
                        | Some(TokenKind::KeywordState)
                        | Some(TokenKind::KeywordLabel)
                )
            }
            TokenKind::Identifier(name) => !matches!(
                name.as_str(),
                "to" | "skk"
                    | "module"
                    | "use"
                    | "export"
                    | "interface"
                    | "trait"
                    | "impl"
                    | "contract"
                    | "enum"
                    | "comptime"
                    | "macro"
                    | "risk"
                    | "policy"
                    | "authority"
                    | "foreign"
            ),
            TokenKind::KeywordPublic
            | TokenKind::KeywordPrivate
            | TokenKind::KeywordInternal
            | TokenKind::KeywordStrictly
            | TokenKind::KeywordState
            | TokenKind::KeywordLabel => true,
            _ => false,
        }
    }

    fn peek_starts_local_decl(&self) -> bool {
        self.current_starts_local_decl_keyword()
    }

    fn is_section_start_at_column_or_left(&self, column: usize) -> bool {
        self.current().column <= column && self.is_section_start()
    }

    fn peek_starts_stc_decl(&self) -> bool {
        Self::is_context_segment_start(&self.current().kind)
            || matches!(
                self.current().kind,
                TokenKind::KeywordPublic
                    | TokenKind::KeywordPrivate
                    | TokenKind::KeywordInternal
                    | TokenKind::KeywordStrictly
            )
    }

    fn generic_parameter_list_is_followed_by_lparen(&self, less_pos: usize) -> bool {
        let mut depth = 0i32;
        for (offset, token) in self.tokens.iter().enumerate().skip(less_pos) {
            match token.kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => depth -= 1,
                TokenKind::Shift2 => depth -= 2,
                TokenKind::Shift3 => depth -= 3,
                TokenKind::LBrace | TokenKind::Semicolon if depth > 0 => return false,
                _ => {}
            }
            if depth <= 0 {
                return matches!(
                    self.tokens.get(offset + 1).map(|next| &next.kind),
                    Some(TokenKind::LParen)
                );
            }
        }
        false
    }

    fn generic_parameter_list_is_followed_by_canonical_fn(&self, less_pos: usize) -> bool {
        let mut depth = 0i32;
        for (offset, token) in self.tokens.iter().enumerate().skip(less_pos) {
            match token.kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => depth -= 1,
                TokenKind::Shift2 => depth -= 2,
                TokenKind::Shift3 => depth -= 3,
                TokenKind::LBrace | TokenKind::Semicolon if depth > 0 => return false,
                _ => {}
            }
            if depth <= 0 {
                return matches!(
                    (
                        self.tokens.get(offset + 1).map(|next| &next.kind),
                        self.tokens.get(offset + 2).map(|next| &next.kind),
                        self.tokens.get(offset + 3).map(|next| &next.kind),
                        self.tokens.get(offset + 4).map(|next| &next.kind),
                    ),
                    (
                        Some(TokenKind::Colon),
                        Some(TokenKind::Colon),
                        Some(TokenKind::KeywordFn),
                        Some(TokenKind::LParen)
                    )
                );
            }
        }
        false
    }

    fn nc_prefix_operand_follows(&self) -> bool {
        // Translate lexer tags into the canonical nC token ABI; policy is native nC.
        let token = match self.tokens.get(self.pos + 1).map(|t| &t.kind) {
            None | Some(TokenKind::Eof) => 101,
            Some(TokenKind::Semicolon) => 14,
            Some(TokenKind::Comma) => 15,
            Some(TokenKind::RParen) => 8,
            Some(TokenKind::RBracket) => 10,
            Some(TokenKind::RBrace) => 6,
            Some(TokenKind::LParen) => 7,
            Some(TokenKind::LBracket) => 9,
            Some(TokenKind::Dot) => 16,
            Some(TokenKind::Colon) => 13,
            Some(TokenKind::Question) => 45,
            Some(TokenKind::Bang) => 33,
            Some(TokenKind::Eq) => 30,
            Some(TokenKind::LessDash) => 24,
            Some(TokenKind::PlusPlus) => 36,
            Some(TokenKind::PlusEq) => 37,
            Some(TokenKind::MinusMinus) => 39,
            Some(TokenKind::MinusEq) => 40,
            Some(TokenKind::StarEq) => 47,
            Some(TokenKind::OrOr) => 49,
            Some(TokenKind::AndAnd) => 48,
            Some(TokenKind::Pipe) => 25,
            Some(TokenKind::Caret) => 44,
            Some(TokenKind::Amp) => 43,
            Some(TokenKind::EqEq) => 31,
            Some(TokenKind::NotEq) => 32,
            Some(TokenKind::Greater | TokenKind::Gt) => 26,
            Some(TokenKind::Less | TokenKind::Lt) => 27,
            Some(TokenKind::GreaterEq) => 28,
            Some(TokenKind::LessEq) => 29,
            Some(TokenKind::Shift2) => 21,
            Some(TokenKind::Shift3) => 22,
            Some(TokenKind::ShiftLeft) => 46,
            Some(TokenKind::Plus) => 35,
            Some(TokenKind::Minus) => 38,
            Some(TokenKind::Star) => 18,
            Some(TokenKind::Slash) => 41,
            Some(TokenKind::Percent) => 42,
            Some(_) => 0,
        };
        ncc_stage0_bridge::prefix_next(token)
    }

    fn peek_starts_function(&self) -> bool {
        if self.check_word_ci("async") {
            if matches!(
                self.tokens.get(self.pos + 1).map(|tok| &tok.kind),
                Some(TokenKind::KeywordFn)
            ) {
                return match self.tokens.get(self.pos + 3).map(|tok| &tok.kind) {
                    Some(TokenKind::LParen) => true,
                    Some(TokenKind::Less) => {
                        self.generic_parameter_list_is_followed_by_lparen(self.pos + 3)
                    }
                    _ => false,
                };
            }
            if matches!(
                (
                    self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                    self.tokens.get(self.pos + 3).map(|tok| &tok.kind),
                    self.tokens.get(self.pos + 4).map(|tok| &tok.kind),
                ),
                (
                    Some(TokenKind::Colon),
                    Some(TokenKind::Colon),
                    Some(TokenKind::KeywordFn)
                )
            ) {
                return true;
            }
            if matches!(
                (
                    self.tokens.get(self.pos + 2).map(|tok| &tok.kind),
                    self.tokens.get(self.pos + 3).map(|tok| &tok.kind),
                    self.tokens.get(self.pos + 4).map(|tok| &tok.kind),
                ),
                (
                    Some(TokenKind::Colon),
                    Some(TokenKind::Colon),
                    Some(TokenKind::Identifier(word))
                ) if word.eq_ignore_ascii_case("in")
            ) {
                return true;
            }
            return match self.tokens.get(self.pos + 2).map(|tok| &tok.kind) {
                Some(TokenKind::LParen) => true,
                Some(TokenKind::Less) => {
                    self.generic_parameter_list_is_followed_by_lparen(self.pos + 2)
                }
                _ => false,
            };
        }
        if matches!(
            self.current().kind,
            TokenKind::KeywordPublic
                | TokenKind::KeywordPrivate
                | TokenKind::KeywordInternal
                | TokenKind::KeywordStrictly
        ) {
            return true;
        }
        if matches!(
            (
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                self.tokens.get(self.pos + 2).map(|token| &token.kind),
                self.tokens.get(self.pos + 3).map(|token| &token.kind),
                self.tokens.get(self.pos + 4).map(|token| &token.kind),
            ),
            (
                Some(TokenKind::Colon),
                Some(TokenKind::Colon),
                Some(TokenKind::Identifier(word)),
                Some(TokenKind::LParen)
            ) if word.eq_ignore_ascii_case("in")
        ) {
            return true;
        }
        let name_role = match &self.current().kind {
            TokenKind::Identifier(_) | TokenKind::KeywordBind => 1,
            kind if Self::is_context_segment_start(kind) => 2,
            _ => 0,
        };
        if !ncc_stage0_bridge::function_name(name_role) {
            return false;
        }
        if matches!(
            (
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                self.tokens.get(self.pos + 2).map(|token| &token.kind),
                self.tokens.get(self.pos + 3).map(|token| &token.kind),
                self.tokens.get(self.pos + 4).map(|token| &token.kind),
            ),
            (
                Some(TokenKind::Colon),
                Some(TokenKind::Colon),
                Some(TokenKind::KeywordFn),
                Some(TokenKind::LParen)
            )
        ) {
            return true;
        }
        match self.tokens.get(self.pos + 1).map(|token| &token.kind) {
            Some(TokenKind::LParen) => true,
            Some(TokenKind::Less) => {
                self.generic_parameter_list_is_followed_by_lparen(self.pos + 1)
                    || self.generic_parameter_list_is_followed_by_canonical_fn(self.pos + 1)
            }
            _ => false,
        }
    }

    fn peek_function_header_has_arrow(&self) -> bool {
        let Some(open) = self.tokens[self.pos..]
            .iter()
            .position(|token| matches!(token.kind, TokenKind::LParen))
            .map(|offset| self.pos + offset)
        else {
            return false;
        };
        let mut depth = 0usize;
        for index in open..self.tokens.len() {
            match self.tokens[index].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(index + 1).map(|token| &token.kind),
                            Some(TokenKind::Arrow)
                        );
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expect_ident(&mut self) -> Result<String, Diagnostic> {
        if Self::is_context_segment_start(&self.current().kind) {
            return self.expect_context_segment();
        }
        match self.advance().kind {
            TokenKind::KeywordFn => Ok("fn".to_string()),
            TokenKind::KeywordMove => Ok("move".to_string()),
            _ => self.error_here("Expected identifier"),
        }
    }

    fn expect_use_path_segment(&mut self) -> Result<String, Diagnostic> {
        match self.advance().kind {
            TokenKind::Identifier(s) => Ok(s),
            TokenKind::KeywordUse => Ok("use".into()),
            TokenKind::KeywordImport => Ok("import".into()),
            TokenKind::KeywordStc => Ok("stc".into()),
            TokenKind::KeywordLocal => Ok("local".into()),
            TokenKind::KeywordFn => Ok("fn".into()),
            TokenKind::KeywordMove => Ok("move".into()),
            TokenKind::KeywordProbe => Ok("probe".into()),
            TokenKind::KeywordBind => Ok("bind".into()),
            TokenKind::KeywordBinary => Ok("binary".into()),
            TokenKind::KeywordRaw => Ok("raw".into()),
            TokenKind::KeywordStructure => Ok("structure".into()),
            TokenKind::KeywordStr => Ok("str".into()),
            TokenKind::KeywordFinal => Ok("final".into()),
            TokenKind::KeywordFa => Ok("fa".into()),
            TokenKind::KeywordPublic => Ok("public".into()),
            TokenKind::KeywordPrivate => Ok("private".into()),
            TokenKind::KeywordInternal => Ok("internal".into()),
            TokenKind::KeywordStrictly => Ok("strictly".into()),
            TokenKind::KeywordReturn => Ok("return".into()),
            TokenKind::KeywordDev => Ok("dev".into()),
            TokenKind::KeywordIf => Ok("if".into()),
            TokenKind::KeywordElse => Ok("else".into()),
            TokenKind::KeywordWhile => Ok("while".into()),
            TokenKind::KeywordRef => Ok("ref".into()),
            TokenKind::KeywordMain => Ok("main".into()),
            TokenKind::KeywordSkkMain => Ok("skkMain".into()),
            TokenKind::KeywordTrue => Ok("true".into()),
            TokenKind::KeywordFalse => Ok("false".into()),
            TokenKind::KeywordLy => Ok("ly".into()),
            TokenKind::KeywordCnt => Ok("cnt".into()),
            TokenKind::KeywordCut => Ok("cut".into()),
            TokenKind::KeywordAnchor => Ok("anchor".into()),
            TokenKind::KeywordSlot => Ok("slot".into()),
            TokenKind::KeywordPattern => Ok("pattern".into()),
            TokenKind::KeywordMask => Ok("mask".into()),
            TokenKind::KeywordWindow => Ok("window".into()),
            TokenKind::KeywordSignal => Ok("signal".into()),
            TokenKind::KeywordBus => Ok("bus".into()),
            TokenKind::KeywordState => Ok("state".into()),
            TokenKind::KeywordTiming => Ok("timing".into()),
            TokenKind::KeywordEmit => Ok("emit".into()),
            TokenKind::KeywordSample => Ok("sample".into()),
            TokenKind::KeywordLatch => Ok("latch".into()),
            TokenKind::KeywordAssert => Ok("assert".into()),
            TokenKind::KeywordRupture => Ok("rupture".into()),
            TokenKind::KeywordBridge => Ok("bridge".into()),
            TokenKind::KeywordSeal => Ok("seal".into()),
            TokenKind::KeywordReg => Ok("reg".into()),
            TokenKind::KeywordFlag => Ok("flag".into()),
            TokenKind::KeywordLabel => Ok("label".into()),
            TokenKind::KeywordInst => Ok("inst".into()),
            _ => self.error_here("Expected identifier"),
        }
    }

    fn match_ident_name(&mut self, expected: &str) -> bool {
        match &self.current().kind {
            TokenKind::Identifier(s) if s == expected => {
                self.advance();
                true
            }
            _ => false,
        }
    }

    fn check_ident_name(&self, expected: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(s) if s == expected)
    }

    fn token_word(kind: &TokenKind) -> Option<&'static str> {
        match kind {
            TokenKind::KeywordIf => Some("if"),
            TokenKind::KeywordElse => Some("else"),
            TokenKind::KeywordWhile => Some("while"),
            TokenKind::KeywordReturn => Some("return"),
            TokenKind::KeywordDev => Some("dev"),
            TokenKind::KeywordRef => Some("ref"),
            TokenKind::KeywordMain => Some("main"),
            TokenKind::KeywordSkkMain => Some("skkmain"),
            TokenKind::KeywordBridge => Some("bridge"),
            TokenKind::KeywordTrue => Some("true"),
            TokenKind::KeywordFalse => Some("false"),
            _ => None,
        }
    }

    fn check_word_ci(&self, expected: &str) -> bool {
        match &self.current().kind {
            TokenKind::Identifier(s) => s.eq_ignore_ascii_case(expected),
            kind => Self::token_word(kind)
                .map(|w| w.eq_ignore_ascii_case(expected))
                .unwrap_or(false),
        }
    }

    fn match_word_ci(&mut self, expected: &str) -> bool {
        if self.check_word_ci(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_next_kind(&self, expected: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|t| std::mem::discriminant(&t.kind) == std::mem::discriminant(expected))
            .unwrap_or(false)
    }

    fn check_dotdot(&self) -> bool {
        self.check(&TokenKind::Dot) && self.check_next_kind(&TokenKind::Dot)
    }

    fn check_double_colon(&self) -> bool {
        self.check(&TokenKind::Colon) && self.check_next_kind(&TokenKind::Colon)
    }

    fn expect_double_colon(&mut self) -> Result<(), Diagnostic> {
        self.expect_kind(TokenKind::Colon)?;
        self.expect_kind(TokenKind::Colon)?;
        Ok(())
    }

    fn match_dotdot(&mut self) -> bool {
        if self.check_dotdot() {
            self.advance();
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_string(&mut self) -> Option<String> {
        if let TokenKind::String(s) = self.current().kind.clone() {
            self.advance();
            Some(s)
        } else {
            None
        }
    }

    fn match_number(&mut self) -> Option<String> {
        if let TokenKind::Number(n) = self.current().kind.clone() {
            self.advance();
            Some(n)
        } else {
            None
        }
    }

    fn expect_kind(&mut self, expected: TokenKind) -> Result<(), Diagnostic> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            self.error_here(format!("Expected {:?}", expected))
        }
    }

    fn match_kind(&mut self, expected: &TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(expected)
    }

    fn peek_is_ident(&self) -> bool {
        Self::is_context_segment_start(&self.current().kind)
            || matches!(
                self.current().kind,
                TokenKind::KeywordFn | TokenKind::KeywordMove
            )
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn error_here<T>(&self, msg: impl Into<String>) -> Result<T, Diagnostic> {
        Err(Diagnostic::new(
            msg,
            self.current().line,
            self.current().column,
        ))
    }
}

fn parse_context(s: String) -> Context {
    match s.as_str() {
        "system" => Context::System,
        "kernel" => Context::Kernel,
        "machine" => Context::Machine,
        "network" => Context::Network,
        "tooling" => Context::Tooling,
        "runtime" => Context::Runtime,
        "compiler" => Context::Compiler,
        "security" => Context::Security,
        "inspect" => Context::Inspect,
        "ai" => Context::Ai,
        "primitive" => Context::Primitive,
        _ => Context::Unknown(s),
    }
}

fn use_forbidden_in_mode_diagnostic(ctx: &Option<Context>) -> &'static str {
    match ctx {
        Some(Context::Machine) => {
            "E_USE_FORBIDDEN_IN_MACHINE_001: Este modo no usa imports. Declare capacidades/secciones mediante la gramatica propia del modo."
        }
        Some(Context::Primitive) => {
            "E_USE_FORBIDDEN_IN_PRIMITIVE_001: Este modo no usa imports. Declare capacidades/secciones mediante la gramatica propia del modo."
        }
        Some(Context::Kernel) => {
            "E_USE_FORBIDDEN_IN_KERNEL_001: Este modo no usa imports. Declare capacidades/secciones mediante la gramatica propia del modo."
        }
        _ => {
            "E_USE_MODE_FORBIDDEN_001: Este modo no usa imports. Declare capacidades/secciones mediante la gramatica propia del modo."
        }
    }
}

fn f10_context_label(ctx: &Option<Context>) -> Option<&'static str> {
    match ctx {
        Some(Context::System) => Some("system"),
        Some(Context::Kernel) => Some("kernel"),
        Some(Context::Tooling) => Some("tooling"),
        Some(Context::Compiler) => Some("compiler"),
        Some(Context::Ai) => Some("ai"),
        _ => None,
    }
}

#[cfg(test)]
fn example_fixture_path(dir: &str, name: &str) -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let current = manifest_dir.join("../../examples").join(dir).join(name);
    if current.exists() {
        return current;
    }

    let archived = manifest_dir
        .join("../../_archive/old_examples")
        .join(dir)
        .join(name);
    if archived.exists() {
        return archived;
    }

    let public_fixture = manifest_dir
        .join("../../tests/fixtures/old_examples")
        .join(dir)
        .join(name);
    if public_fixture.exists() {
        return public_fixture;
    }

    current
}

#[cfg(test)]
mod control_flow_parser_tests {
    use super::*;
    use ncc_lexer::lex;

    #[test]
    fn compact_elseif_and_canonical_else_if_share_the_nested_if_ast() {
        let src = r#"
controlFlowParse {
    fn:
        classify(x: int) -> int {
            public:
                if (@x < 0) {
                    return -1;
                } elseIf (@x == 0) {
                    return 0;
                } else if (@x == 1) {
                    return 1;
                } else {
                    return 2;
                }
        }
}
"#;
        let program = parse(lex(src).expect("control-flow fixture must lex"))
            .expect("control-flow fixture must parse");
        let function = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => functions.first(),
                _ => None,
            })
            .expect("classify function");

        let Stmt::If { else_block, .. } = &function.public_body[0] else {
            panic!("outer statement must be if");
        };
        let Stmt::If {
            else_block: nested_else,
            ..
        } = &else_block[0]
        else {
            panic!("elseIf must lower to a nested if");
        };
        assert!(
            matches!(nested_else.first(), Some(Stmt::If { .. })),
            "canonical else if must continue the same nested-if chain"
        );
    }

    #[test]
    fn raw_at_alias_is_normalized_only_when_called() {
        let src = r#"
rawAtContext {
    fn:
        read(region: memRegion<byte>, finish: int) -> p<byte> {
            at >> int <- 0;
            while (at < finish) {
                at += 1;
            }
            return at(region, at);
        }
}
"#;
        let program =
            parse(lex(src).expect("raw-at fixture must lex")).expect("raw-at fixture must parse");
        let function = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => functions.first(),
                _ => None,
            })
            .expect("read function");

        let Stmt::While { condition, .. } = &function.public_body[1] else {
            panic!("second statement must be while");
        };
        let Expr::Binary { left, .. } = condition else {
            panic!("while condition must be binary");
        };
        assert!(
            matches!(left.as_ref(), Expr::Identifier(name) if name == "at")
                || matches!(left.as_ref(), Expr::ContextRef(ctx) if ctx.path == ["at"]),
            "plain at identifier was unexpectedly rewritten: {left:?}"
        );

        let Stmt::Return(Expr::Call { callee, .. }) = &function.public_body[2] else {
            panic!("return must contain the compact at call");
        };
        assert_eq!(callee, "regionPtrAt");
    }

    #[test]
    fn checked_add_int64_preserves_the_typed_call_in_the_ast() {
        let src = r#"
checkedArithmeticParse {
    fn:
        run(left: int64, right: int64) -> int64 {
            public:
                return checkedAdd<int64>(@left, @right);
        }
}
"#;
        let program = parse(lex(src).expect("checked-add fixture must lex"))
            .expect("checked-add fixture must parse");
        let function = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => functions.first(),
                _ => None,
            })
            .expect("checked-add function");
        let Stmt::Return(Expr::Call { callee, args }) = &function.public_body[0] else {
            panic!("checkedAdd must remain an explicit call in the AST");
        };
        assert_eq!(callee, "checkedAdd<int64>");
        assert_eq!(args.len(), 2);
    }
}

#[cfg(test)]
mod machine_f2_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("machine/legacy_at", name)
    }

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        parse(tokens)
    }

    #[test]
    fn machine_at_minimal_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("machine_at_minimal_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected valid [@machine] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn machine_at_invalid_section_fails() {
        let src =
            std::fs::read_to_string(fixture_path("machine_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@machine] context")
                || msg.contains("section not allowed outside [@machine] context")
        );
    }

    #[test]
    fn machine_at_invalid_final_op_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("machine_at_final_invalid_op_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(
                "final section in [@machine] container only allows 'ret', 'halt' or 'jmp <label>'"
            ),
            "unexpected parser error: {msg}"
        );
    }

    #[test]
    fn machine_at_invalid_instruction_fails_in_parser() {
        let src = r#"
machineInvalidInstruction[@machine] {
    reg: ax;
    label: entryPoint;
    inst:
        foo ax, ax;
        jmp entryPoint;
    final:
        ret;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid machine instruction in [@machine] inst section"),
            "unexpected parser error: {msg}"
        );
    }

    #[test]
    fn machine_at_load_store_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("machine_at_load_store_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected valid machine load/store fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn machine_at_load_store_indexed_ok_parses() {
        let src = r#"
machineLoadStoreIndexed[@machine] {
    reg:
        ax;
        bx;
        cx;
        dx;

    label:
        entryPoint;

    inst:
        load ax, mem[bx + cx*4 + 16];
        store mem[bx + cx*2 - 8], dx;
        jmp entryPoint;

    final:
        ret;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected indexed machine load/store to parse, got {:?}",
            result
        );
    }

    #[test]
    fn machine_at_signed_overflow_branches_parse_in_canonical_and_x86_form() {
        let src = r#"
machineCheckedAdd[@machine] {
    reg:
        rax;
        rbx;
    label:
        entryPoint;
        overflow;
        finish;
    inst:
        entryPoint:
        add rax, rbx;
        jmp.ov overflow;
        add rax, rbx;
        jo overflow;
        jmp finish;
        overflow:
        halt;
        finish:
        ret;
    final:
        ret;
}
"#;
        parse_source(src).expect("jmp.ov and jo must lower to the same typed machine node");
    }

    #[test]
    fn machine_at_extended_isa_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("machine_at_extended_isa_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected extended ISA [@machine] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn machine_at_virtualization_operands_parse_as_typed_nodes() {
        let src = r#"
virtualizationCore[@machine] {
    reg:
        rax;
        rbx;
        r8;
        r9;
    inst:
        vmxon mem[rax];
        vmclear mem[rax + r8*4 + 16];
        vmptrld mem[rax];
        vmptrst mem[rbx];
        vmread r8, r9;
        vmread mem[rax], r9;
        vmwrite r8, r9;
        vmwrite r8, mem[rbx];
        invept r8, mem[rax];
        invvpid r8, mem[rbx];
        vmcall;
        vmxoff;
        vmlaunch;
        vmresume;
        vmrun;
        vmmcall;
        vmload;
        vmsave;
        stgi;
        clgi;
        invlpga;
    final:
        ret;
}
"#;
        let program = parse_source(src).expect("virtualization syntax must parse");
        let machine = program.containers[0]
            .as_machine_at()
            .expect("machine projection");
        let instructions = machine
            .sections
            .iter()
            .find_map(|section| match section {
                MachineAtSection::Inst(items) => Some(items),
                _ => None,
            })
            .expect("typed instruction section");
        assert_eq!(instructions.len(), 21);
        assert!(matches!(instructions[0], MachineInst::VmxOn { .. }));
        assert!(matches!(
            instructions[4],
            MachineInst::VmRead {
                destination: MachineRegOrMem::Reg(_),
                ..
            }
        ));
        assert!(matches!(instructions[8], MachineInst::InvEpt { .. }));
        assert!(matches!(instructions[14], MachineInst::VmRun));
        assert!(matches!(instructions[20], MachineInst::InvLpga));
    }

    #[test]
    fn machine_at_v1_0_1_isa_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("machine_at_isa_v1_0_1_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected v1.0.1 ISA [@machine] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn machine_at_invalid_load_store_form_fails_in_parser() {
        let src = r#"
machineInvalidLoadStore[@machine] {
    reg:
        ax;
        bx;

    label:
        entryPoint;

    inst:
        load ax, mem bx;
        jmp entryPoint;

    final:
        ret;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid machine instruction in [@machine] inst section"),
            "unexpected parser error: {msg}"
        );
    }

    #[test]
    fn machine_at_invalid_load_store_scale_fails_in_parser() {
        let src = r#"
machineInvalidLoadStoreScale[@machine] {
    reg:
        ax;
        bx;
        cx;

    label:
        entryPoint;

    inst:
        load ax, mem[bx + cx*3];
        jmp entryPoint;

    final:
        ret;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid machine instruction in [@machine] inst section"),
            "unexpected parser error: {msg}"
        );
    }

    #[test]
    fn machine_at_call_root_policy_ok_parses_with_typed_target() {
        let src =
            std::fs::read_to_string(fixture_path("machine_at_call_root_policy_ok.n")).unwrap();
        let program = parse_source(&src).expect("expected machine root call fixture to parse");

        let container = program
            .containers
            .first()
            .expect("fixture must contain one machine container");
        let insts = container
            .sections
            .iter()
            .find_map(|section| {
                if let Section::Inst(items) = section {
                    Some(items)
                } else {
                    None
                }
            })
            .expect("fixture must contain inst section");

        let call = insts
            .iter()
            .find_map(|inst| {
                if let MachineInst::Call { target } = inst {
                    Some(target)
                } else {
                    None
                }
            })
            .expect("fixture must contain call instruction");

        assert!(matches!(
            call,
            MachineCallTarget::RootPath(path)
                if path == &vec!["root".to_string(), "policy".to_string(), "bind".to_string()]
        ));
    }

    #[test]
    fn machine_at_call_non_root_qualified_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("machine_at_call_non_root_qualified_fail.n"))
                .unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid machine instruction in [@machine] inst section"),
            "unexpected parser error: {msg}"
        );
    }

    #[test]
    fn use_path_with_keyword_segment_reg_parses() {
        let src = r#"
use Machine.reg;

entry[@] {
    ref main ->> @fn(root.policy.bind);

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected use path with keyword segment to parse");
        assert_eq!(program.uses.len(), 1);
        assert_eq!(
            program.uses[0].path,
            vec!["Machine".to_string(), "reg".to_string()]
        );
    }

    #[test]
    fn import_decl_parses_like_use_decl() {
        let src = r#"
import system.net.core;

entry[@] {
    ref main ->> @fn(root.policy.bind);

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected import declaration to parse");
        assert_eq!(program.uses.len(), 1);
        assert_eq!(
            program.uses[0].path,
            vec!["system".to_string(), "net".to_string(), "core".to_string()]
        );
        assert!(!program.uses[0].wildcard);
    }

    #[test]
    fn import_wildcard_decl_parses_like_use_decl() {
        let src = r#"
import nC.runtime.*;

entry[@] {
    ref main ->> @fn(root.policy.bind);

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected wildcard import declaration to parse");
        assert_eq!(program.uses.len(), 1);
        assert_eq!(
            program.uses[0].path,
            vec!["nC".to_string(), "runtime".to_string()]
        );
        assert!(program.uses[0].wildcard);
    }

    #[test]
    fn canonical_text_pad_use_expands_to_runtime_module() {
        let src = r#"
use <text:pad>;

padImport :: runtime {
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("canonical text pad import must parse");
        assert_eq!(program.uses.len(), 1);
        assert_eq!(
            program.uses[0].path,
            vec!["nC".to_string(), "runtime".to_string(), "pad".to_string()]
        );
        assert!(!program.uses[0].wildcard);
    }
}

#[cfg(test)]
mod root_f0_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("root_at", name)
    }

    #[test]
    fn root_at_minimal_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("root_at_minimal_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected valid [@] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_probe_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("root_at_probe_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected valid root probe section to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_structure_section_parses_in_root_context() {
        let src = std::fs::read_to_string(fixture_path("root_at_invalid_section_fail.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected structure section in [@] root context to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_raw_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("root_at_raw_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected valid raw root file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_qualified_symbol_without_call_parses_as_context_ref() {
        let src = r#"
rootQualifiedSymbol[@] {
    stc:
        sink: str;

    fn:
        capture() -> bool {
            public:
                @sink >> str : system.policy.channel;
                return true;
        }

    final:
        return true;
}
"#;
        let program = parse_source(src).expect("qualified symbol without call should parse");
        let container = program
            .containers
            .first()
            .expect("fixture must contain one container");
        let fn_section = container
            .sections
            .iter()
            .find_map(|section| {
                if let Section::Fn(funcs) = section {
                    Some(funcs)
                } else {
                    None
                }
            })
            .expect("fixture must contain fn section");
        let func = fn_section
            .first()
            .expect("fixture must contain one function");
        let first_stmt = func
            .public_body
            .first()
            .expect("function must contain projection stmt");

        match first_stmt {
            Stmt::Projection { value, .. } => match value {
                Expr::ContextRef(ctx) => assert_eq!(
                    ctx.path,
                    vec![
                        "system".to_string(),
                        "policy".to_string(),
                        "channel".to_string()
                    ]
                ),
                other => panic!("expected context ref projection value, got {other:?}"),
            },
            other => panic!("expected projection statement, got {other:?}"),
        }
    }

    #[test]
    fn root_at_fn_referential_origin_is_typed_in_ast() {
        let src = r#"
rootFnOrigin[@] {
    stc:
        out: str;

    fn:
        capture() -> bool {
            public:
                @out >> str : @fn(worker.boot);
                return true;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("@fn referential origin should parse");
        let container = program
            .containers
            .first()
            .expect("fixture must contain one container");
        let fn_section = container
            .sections
            .iter()
            .find_map(|section| {
                if let Section::Fn(funcs) = section {
                    Some(funcs)
                } else {
                    None
                }
            })
            .expect("fixture must contain fn section");
        let func = fn_section
            .first()
            .expect("fixture must contain one function");
        let first_stmt = func
            .public_body
            .first()
            .expect("function must contain projection stmt");

        match first_stmt {
            Stmt::Projection { value, .. } => match value {
                Expr::ContextRef(ctx) => {
                    assert_eq!(ctx.path, vec!["worker".to_string(), "boot".to_string()]);
                    assert_eq!(ctx.origin, ContextRefOrigin::FnSyntax);
                }
                other => panic!("expected context ref projection value, got {other:?}"),
            },
            other => panic!("expected projection statement, got {other:?}"),
        }
    }

    #[test]
    fn root_at_probe_referential_origin_is_typed_in_ast() {
        let src = r#"
rootProbeOrigin[@] {
    stc:
        out: str;

    fn:
        capture() -> bool {
            public:
                @out >> str : @probe(worker.health);
                return true;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("@probe referential origin should parse");
        let container = program
            .containers
            .first()
            .expect("fixture must contain one container");
        let fn_section = container
            .sections
            .iter()
            .find_map(|section| {
                if let Section::Fn(funcs) = section {
                    Some(funcs)
                } else {
                    None
                }
            })
            .expect("fixture must contain fn section");
        let func = fn_section
            .first()
            .expect("fixture must contain one function");
        let first_stmt = func
            .public_body
            .first()
            .expect("function must contain projection stmt");

        match first_stmt {
            Stmt::Projection { value, .. } => match value {
                Expr::ContextRef(ctx) => {
                    assert_eq!(ctx.path, vec!["worker".to_string(), "health".to_string()]);
                    assert_eq!(ctx.origin, ContextRefOrigin::ProbeSyntax);
                }
                other => panic!("expected context ref projection value, got {other:?}"),
            },
            other => panic!("expected projection statement, got {other:?}"),
        }
    }

    #[test]
    fn root_at_context_ref_missing_symbol_after_at_fails_with_official_diagnostic() {
        let src = r#"
rootMissingAtSymbol[@] {
    stc:
        out: int;

    fn:
        public run() -> bool {
            public:
                @out >> int : @;
                return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(result.is_err(), "expected missing symbol after @ to fail");
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("expected resolvable symbol after '@'"),
            "unexpected parser diagnostic: {}",
            msg
        );
    }

    #[test]
    fn root_at_context_ref_invalid_dot_form_fails_with_official_diagnostic() {
        let src = r#"
rootInvalidAtDot[@] {
    stc:
        out: int;

    fn:
        public run() -> bool {
            public:
                @out >> int : @.;
                return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(result.is_err(), "expected invalid @. form to fail");
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("invalid referential form after '@'"),
            "unexpected parser diagnostic: {}",
            msg
        );
    }

    #[test]
    fn root_at_context_ref_invalid_double_dot_form_fails_with_official_diagnostic() {
        let src = r#"
rootInvalidAtDoubleDot[@] {
    stc:
        out: int;

    fn:
        public run() -> bool {
            public:
                @out >> int : @ghost..path;
                return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(result.is_err(), "expected invalid @a..b form to fail");
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("invalid referential form after '@'"),
            "unexpected parser diagnostic: {}",
            msg
        );
    }

    #[test]
    fn root_at_mature_suite_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("root_at_mature_suite_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected mature root suite fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_final_keyword_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("root_at_final_keyword_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@] file with final: to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_duplicate_final_parses_but_will_fail_in_sema() {
        let src = std::fs::read_to_string(fixture_path("root_at_duplicate_final_fail.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "parser should accept duplicate final syntactically, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_missing_final_parses_but_will_fail_in_sema() {
        let src = std::fs::read_to_string(fixture_path("root_at_missing_final_fail.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "parser should accept missing final syntactically, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_context_method_keyword_bind_parses() {
        let src = r#"
rootBindKeyword[@] {
    fn:
        public init() -> bool {
            public:
                @r >> addr : 0b00000001;
                @ok >> bool : @r.bind(1024);
                return @ok;
        }

    fa:
        return true;
}
"#;

        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected @context keyword method call to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_strictly_stc_marks_strict_flag() {
        let src = r#"
rootStrictStc[@] {
    stc:
        strictly mem: addr;

    fa:
        return true;
}
"#;

        let program = parse_source(src).expect("expected strictly stc fixture to parse");
        let container = program
            .containers
            .first()
            .expect("fixture must contain one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("fixture must contain stc section");

        let is_strict = stc_items.iter().any(|decl| match decl {
            StcDecl::Typed { name, strictly, .. } => name == "mem" && *strictly,
        });

        assert!(
            is_strict,
            "expected strictly stc declaration to set strict flag"
        );
    }

    #[test]
    fn root_at_stc_projection_initializer_forms_parse() {
        let src = r#"
rootStcInit[@] {
    stc:
        strictly machineWidth >> int : 32;
        strictly: stackBase: addr;
        bootBase >> addr : 0x7C00;

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected stc initializer fixture to parse");
        let container = program
            .containers
            .first()
            .expect("fixture must contain one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("fixture must contain stc section");

        assert_eq!(stc_items.len(), 3);
        let has_machine_width_init = stc_items.iter().any(|decl| match decl {
            StcDecl::Typed {
                name,
                strictly,
                init: Some(_),
                ..
            } => name == "machineWidth" && *strictly,
            _ => false,
        });
        assert!(
            has_machine_width_init,
            "expected strictly projection initializer in stc to be parsed"
        );
    }

    #[test]
    fn root_at_stc_accepts_state_and_label_as_field_names() {
        let src = r#"
rootStcKeywordFields[@] {
    stc:
        state: str;
        label: str;

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected stc keyword-like field names to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_bare_projection_statements_parse() {
        let src = r#"
rootBareProjection[@] {
    fn:
        public run() -> bool {
            public:
                value >> int : 1;
                value !> int : 1;
                return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected bare projection forms to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_function_without_public_and_with_raw_block_parses() {
        let src = r#"
rootFnCompat[@] {
    fn:
        run() -> bool {
            value >> int : 1;
            raw {
                value >> int : @value + 1;
            }
            return true;
        }

    fa:
        return true;
}
"#;

        let program = parse_source(src).expect("expected function compatibility fixture to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected one function");
        assert_eq!(
            run_fn.public_body.len(),
            3,
            "expected raw block statements to be flattened into function public body"
        );
    }

    #[test]
    fn root_at_extended_operator_surface_parses() {
        let src = r#"
rootOps[@] {
    fn:
        public run() -> bool {
            public:
                @x >> int : 7;
                @y >> int : 3;
                @z >> int : (~@x & @y) | (@x ^ @y);
                @w >> int : (@x + @y) << 1;
                @ok >> bool : !(@x == @y) && true || false;
                return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected extended operator surface to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_operator_precedence_for_shift_and_bitwise_is_stable() {
        let src = r#"
rootOpsPrec[@] {
    fn:
        public run() -> int {
            public:
                return 1 + 2 << 3 | 4 & 5;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected operator precedence fixture to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected one function");
        let return_expr = run_fn
            .public_body
            .iter()
            .find_map(|stmt| match stmt {
                Stmt::Return(expr) => Some(expr),
                _ => None,
            })
            .expect("expected return statement");

        // Expect: (((1 + 2) << 3) | (4 & 5))
        match return_expr {
            Expr::Binary {
                op: BinaryOp::BitOr,
                left,
                right,
            } => {
                assert!(
                    matches!(
                        &**left,
                        Expr::Binary {
                            op: BinaryOp::ShiftLeft,
                            ..
                        }
                    ),
                    "expected left side to be shift-left subtree, got {:?}",
                    left
                );
                assert!(
                    matches!(
                        &**right,
                        Expr::Binary {
                            op: BinaryOp::BitAnd,
                            ..
                        }
                    ),
                    "expected right side to be bit-and subtree, got {:?}",
                    right
                );
            }
            other => panic!("expected top-level bit-or expression, got {:?}", other),
        }
    }

    #[test]
    fn root_at_triple_shift_right_operator_parses_as_logical_shift() {
        let src = r#"
rootOpsShift3[@] {
    fn:
        run() -> int {
            value >> int : 8;
            value >> int : @value >>> 1;
            return @value;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected >>> operator fixture to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected one function");

        let shifted_expr = run_fn
            .public_body
            .iter()
            .find_map(|stmt| match stmt {
                Stmt::Projection { value, .. } => match value {
                    Expr::Binary { .. } => Some(value),
                    _ => None,
                },
                _ => None,
            })
            .expect("expected projection with binary >>> expression");

        assert!(
            matches!(
                shifted_expr,
                Expr::Binary {
                    op: BinaryOp::ShiftRightLogical,
                    ..
                }
            ),
            "expected >>> to map into logical right shift operator, got {:?}",
            shifted_expr
        );
    }

    #[test]
    fn root_at_nested_generic_triple_gt_closure_and_shift_right_are_disambiguated() {
        let src = r#"
rootPolyGeneric[@] {
    stc:
        arena: list<list<list<int>>>;

    fn:
        run() -> int {
            value >> int : 8;
            return @value >> 1;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src)
            .expect("expected nested generic closure + shift-right fixture to parse");
        let container = program.containers.first().expect("expected one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");

        let arena_ty = stc_items
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "arena" => Some(ty),
                _ => None,
            })
            .expect("expected arena type declaration");

        assert!(
            matches!(
                arena_ty,
                TypeName::Generic { base, args }
                    if base == "list"
                        && matches!(
                            args.first(),
                            Some(TypeName::Generic { base: b2, args: a2 })
                                if b2 == "list"
                                    && matches!(
                                        a2.first(),
                                        Some(TypeName::Generic { base: b3, args: a3 })
                                            if b3 == "list"
                                                && matches!(
                                                    a3.first(),
                                                    Some(TypeName::Simple(inner)) if inner == "int"
                                                )
                                    )
                        )
            ),
            "expected nested generic list<list<list<int>>> type, got {:?}",
            arena_ty
        );

        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let ret_expr = run_fn
            .public_body
            .iter()
            .find_map(|stmt| match stmt {
                Stmt::Return(expr) => Some(expr),
                _ => None,
            })
            .expect("expected return statement");

        assert!(
            matches!(
                ret_expr,
                Expr::Binary {
                    op: BinaryOp::ShiftRight,
                    ..
                }
            ),
            "expected return to keep `>>` as shift-right expression, got {:?}",
            ret_expr
        );
    }

    #[test]
    fn root_at_mem_index_expression_parses_inside_projection_value() {
        let src = r#"
rootMemIndexExpr[@] {
    fn:
        run() -> bool {
            src >> addr : 0x1000;
            value >> int : mem[@src];
            return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected mem[@addr] expression to parse, got {:?}",
            result
        );
    }

    #[test]
    fn array_type_generic_len_parse_ok() {
        let src = r#"
rootArrayGenericLen[@] {
    stc:
        arr: array<int, 5>;
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected array<T,N> syntax to parse");
        let container = program.containers.first().expect("expected one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let arr_ty = stc_items
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "arr" => Some(ty),
                _ => None,
            })
            .expect("expected arr declaration");
        assert!(
            matches!(
                arr_ty,
                TypeName::Generic { base, args }
                    if base == "array"
                        && matches!(args.first(), Some(TypeName::Simple(inner)) if inner == "int")
                        && matches!(args.get(1), Some(TypeName::Simple(len)) if len == "5")
            ),
            "expected array<int,5> to parse as generic with const length, got {:?}",
            arr_ty
        );
    }

    #[test]
    fn array_type_c_style_len_parse_ok() {
        let src = r#"
rootArrayCStyleLen[@] {
    stc:
        arr: int[5];
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected T[N] syntax to parse");
        let container = program.containers.first().expect("expected one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let arr_ty = stc_items
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "arr" => Some(ty),
                _ => None,
            })
            .expect("expected arr declaration");
        assert!(
            matches!(
                arr_ty,
                TypeName::Generic { base, args }
                    if base == "array"
                        && matches!(args.first(), Some(TypeName::Simple(inner)) if inner == "int")
                        && matches!(args.get(1), Some(TypeName::Simple(len)) if len == "5")
            ),
            "expected int[5] to normalize into array<int,5>-like generic form, got {:?}",
            arr_ty
        );
    }

    #[test]
    fn array_type_multidim_parse_ok() {
        let src = r#"
rootArrayMultiDim[@] {
    stc:
        grid: array<int, 4, 4>;
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected multidim array generic syntax to parse");
        let container = program.containers.first().expect("expected one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let grid_ty = stc_items
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "grid" => Some(ty),
                _ => None,
            })
            .expect("expected grid declaration");
        assert!(
            matches!(
                grid_ty,
                TypeName::Generic { base, args } if base == "array" && args.len() == 3
            ),
            "expected array<int,4,4> multidim generic shape, got {:?}",
            grid_ty
        );
    }

    #[test]
    fn path_generic_type_dinarray_binary_parse_ok() {
        let src = r#"
rootDinArrayBinaryType[@] {
    stc:
        dyn: dinArray.binary<int>;
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected path generic dinArray.binary<T> to parse");
        let container = program.containers.first().expect("expected one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let dyn_ty = stc_items
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "dyn" => Some(ty),
                _ => None,
            })
            .expect("expected dyn declaration");
        assert!(
            matches!(
                dyn_ty,
                TypeName::Generic { base, args }
                    if base == "dinArray.binary"
                        && matches!(args.first(), Some(TypeName::Simple(inner)) if inner == "int")
            ),
            "expected dinArray.binary<int> path-generic type, got {:?}",
            dyn_ty
        );
    }

    #[test]
    fn array_of_fn_type_parse_ok() {
        let src = r#"
rootArrayFnType[@] {
    stc:
        ops: array<fn<int, int, int>, 2>;
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected array<fn<...>,N> to parse");
        let container = program.containers.first().expect("expected one container");
        let stc_items = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let ops_ty = stc_items
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "ops" => Some(ty),
                _ => None,
            })
            .expect("expected ops declaration");
        assert!(
            matches!(
                ops_ty,
                TypeName::Generic { base, args }
                    if base == "array"
                        && args.len() == 2
                        && matches!(args.first(), Some(TypeName::Generic { base: inner_base, args: inner_args })
                            if inner_base == "fn" && inner_args.len() == 3)
            ),
            "expected array<fn<int,int,int>,2> type shape, got {:?}",
            ops_ty
        );
    }

    #[test]
    fn indexed_lvalue_dynamic_parse_ok() {
        let src = r#"
rootIndexedWriteDynamic[@] {
    fn:
        run() -> bool {
            arr >> array<int> : [1, 2, 3];
            i >> int : 1;
            v >> int : 9;
            @arr[@i] >> int : @v;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected indexed lvalue projection to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Expr(Expr::MethodCall { method, args, .. })
                    if method == "arrayWrite" && args.len() == 2
            )
        });
        assert!(
            found,
            "expected @arr[@i] >> ... to lower into arrayWrite method call"
        );
    }

    #[test]
    fn compact_pointer_bytes_surface_parse_ok() {
        let src = r#"
compactPointerBytes[@runtime] {
    stc:
        bytes: array<int, 4>;
        p: ptr<int>;
        i >> int = 0;
    fn:
        run() -> bool {
            public:
                p = alloc(4)!;
                guard i < 4 else {
                    return false;
                }
                match (i) {
                    0 => bytes[i] = 0x48;
                    _ => bytes[i] = 0;
                }
                *(p + i) = bytes[i];
                p++;
                return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected compact pointer byte surface to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        fn stmt_has_array_write(stmt: &Stmt) -> bool {
            match stmt {
                Stmt::Expr(Expr::MethodCall { method, args, .. }) => {
                    method == "arrayWrite" && args.len() == 2
                }
                Stmt::Match { cases, default, .. } | Stmt::Switch { cases, default, .. } => {
                    cases
                        .iter()
                        .any(|(_, body)| body.iter().any(stmt_has_array_write))
                        || default.iter().any(stmt_has_array_write)
                }
                Stmt::PatternMatch { arms, .. } => arms
                    .iter()
                    .any(|arm| arm.body.iter().any(stmt_has_array_write)),
                Stmt::GuardElse { else_block, .. } => else_block.iter().any(stmt_has_array_write),
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    then_block.iter().any(stmt_has_array_write)
                        || else_block.iter().any(stmt_has_array_write)
                }
                Stmt::While { body, .. }
                | Stmt::Loop { body }
                | Stmt::RepeatCount { body, .. }
                | Stmt::Unless { body, .. } => body.iter().any(stmt_has_array_write),
                _ => false,
            }
        }
        let found_array_write = run_fn.public_body.iter().any(stmt_has_array_write);
        let found_ptr_write = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Expr(Expr::MethodCall { method, .. }) if method == "write"
            )
        });
        let found_post_increment = run_fn
            .public_body
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Expr(Expr::PostIncrement { .. })));
        assert!(
            found_array_write,
            "expected bytes[i] = ... to lower into arrayWrite"
        );
        assert!(
            found_ptr_write,
            "expected *(p + i) = ... to lower into ptr write"
        );
        assert!(
            found_post_increment,
            "expected p++ to parse as post increment"
        );
    }

    #[test]
    fn c_pointer_suffixes_and_compact_helpers_normalize_to_canonical_ast() {
        let src = r#"
compactPointerCStyle[@runtime] {
    stc:
        bytes: byte*;
        matrix: int**;
        view: memRegion<int>;

    fn:
        run() -> bool {
            bytes !> byte* : alloc(8);
            Write*<byte>(bytes, 65);
            view = Region*<int>(Offset*<int>(bytes, 0), 2);
            return Read*<byte>(bytes) == 65;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("C-shaped pointer surface must parse");
        let container = program.containers.first().expect("expected container");
        let stc = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let bytes_ty = stc
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "bytes" => Some(ty),
                _ => None,
            })
            .expect("expected byte pointer declaration");
        let matrix_ty = stc
            .iter()
            .find_map(|decl| match decl {
                StcDecl::Typed { name, ty, .. } if name == "matrix" => Some(ty),
                _ => None,
            })
            .expect("expected double pointer declaration");
        assert!(
            matches!(
                bytes_ty,
                TypeName::Generic { base, args }
                    if base == "ptr"
                        && matches!(args.as_slice(), [TypeName::Simple(inner)] if inner == "byte")
            ),
            "byte* must normalize into ptr<byte>, got {bytes_ty:?}"
        );
        assert!(
            matches!(
                matrix_ty,
                TypeName::Generic { base, args }
                    if base == "ptr"
                        && matches!(args.as_slice(), [TypeName::Generic { base: inner_base, args: inner_args }]
                            if inner_base == "ptr"
                                && matches!(inner_args.as_slice(), [TypeName::Simple(inner)] if inner == "int"))
            ),
            "int** must normalize into ptr<ptr<int>>, got {matrix_ty:?}"
        );

        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => Some(functions),
                _ => None,
            })
            .expect("expected fn section");
        let run = functions
            .iter()
            .find(|function| function.name == "run")
            .expect("expected run function");
        let lowered_calls = format!("{:?}", run.public_body);
        assert!(lowered_calls.contains("ptrWrite<byte>"));
        assert!(lowered_calls.contains("regionFromPtr<int>"));
        assert!(lowered_calls.contains("ptrRead<byte>"));
    }

    #[test]
    fn sovereign_skk_pointer_classes_remain_nominal_types() {
        let src = r#"
sovereignPointerClasses[@] {
    stc:
        base: skk*;
        deep: skk**;
        bytes: byte*;
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("sovereign pointer classes must parse");
        let stc = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Stc(items) => Some(items),
                _ => None,
            })
            .expect("expected stc section");
        let ty = |wanted: &str| {
            stc.iter()
                .find_map(|decl| match decl {
                    StcDecl::Typed { name, ty, .. } if name == wanted => Some(ty),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing declaration {wanted}"))
        };

        assert!(matches!(ty("base"), TypeName::Simple(name) if name == "skk*"));
        assert!(matches!(ty("deep"), TypeName::Simple(name) if name == "skk**"));
        assert!(matches!(
            ty("bytes"),
            TypeName::Generic { base, args }
                if base == "ptr"
                    && matches!(args.as_slice(), [TypeName::Simple(inner)] if inner == "byte")
        ));
    }

    #[test]
    fn indexed_lvalue_multidim_parse_ok() {
        let src = r#"
rootIndexedWriteMultiDim[@] {
    fn:
        run() -> bool {
            grid >> array<int> : [1, 2, 3, 4];
            r >> int : 0;
            c >> int : 1;
            @grid[@r, @c] >> int : 9;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected multidim indexed lvalue to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Expr(Expr::MethodCall { method, args, .. })
                    if method == "arrayWrite" && args.len() == 3
            )
        });
        assert!(
            found,
            "expected @grid[@r,@c] >> ... to lower into arrayWrite with two indices + value"
        );
    }

    #[test]
    fn indexed_expr_multidim_parse_ok() {
        let src = r#"
rootIndexedExprMultiDim[@] {
    stc:
        m: matrix<int, 2, 2>;
        out: int;
    fn:
        run() -> bool {
            @m >> matrix<int, 2, 2> : [[1, 2], [3, 4]];
            @out >> int : @m[0, 1];
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected multidim indexed expr to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Projection {
                    value: Expr::Index { index, .. },
                    ..
                } if matches!(index.as_ref(), Expr::ArrayLiteral(items) if items.len() == 2)
            )
        });
        assert!(
            found,
            "expected @m[0,1] read to parse as Expr::Index with multi-index array literal"
        );
    }

    #[test]
    fn fill_literal_parse_ok() {
        let src = r#"
rootFillLiteral[@] {
    stc:
        cache: array<int, 100>;
    fn:
        run() -> bool {
            @cache >> array<int, 100> : fill[0,...,0];
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected fill literal to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Projection {
                    value: Expr::ArrayLiteral(items),
                    ..
                } if items.len() == 1
            )
        });
        assert!(
            found,
            "expected fill[...] to parse as singleton array literal"
        );
    }

    #[test]
    fn c_style_conditional_question_colon_parse_ok() {
        let src = r#"
questionColonExpr[@runtime] {
    stc:
        a: int;
        b: int;
        r: int;
    fn:
        run() -> bool {
            @a >> int : 4;
            @b >> int : 2;
            @r >> int : @a > @b ? @a : @b;
            return @r == 4;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected C-style ?: expression to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Projection {
                    value: Expr::IfElse { condition, then_expr, else_expr },
                    ..
                } if matches!(condition.as_ref(), Expr::Binary { op: BinaryOp::Greater, .. })
                    && matches!(then_expr.as_ref(), Expr::ContextRef(_))
                    && matches!(else_expr.as_ref(), Expr::ContextRef(_))
            )
        });
        assert!(found, "expected ?: to lower into Expr::IfElse");
    }

    #[test]
    fn c_style_conditional_question_colon_right_associative_parse_ok() {
        let src = r#"
questionColonNested[@runtime] {
    stc:
        a: int;
        b: int;
        c: int;
        r: int;
    fn:
        run() -> bool {
            @a >> int : 0;
            @b >> int : 1;
            @c >> int : 2;
            @r >> int : @a ? @b : @b ? @c : 99;
            return @r == 2;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected nested ?: expression to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Projection {
                    value: Expr::IfElse { else_expr, .. },
                    ..
                } if matches!(else_expr.as_ref(), Expr::IfElse { .. })
            )
        });
        assert!(found, "expected nested ?: to be right-associative");
    }

    #[test]
    fn postfix_try_parses_at_expression_end() {
        let src = r#"
postfixTryDeferred {
    fn:
        one() -> Result<int, str> {
            public:
                return Ok(1);
        }
        run() -> int {
            public:
                return one()?;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("postfix try must parse");
        let functions = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => Some(functions),
                _ => None,
            })
            .expect("function section");
        let run = functions
            .iter()
            .find(|function| function.name == "run")
            .expect("run function");
        assert!(matches!(
            &run.public_body[0],
            Stmt::Return(Expr::Unary {
                op: UnaryOp::TryPropagate,
                ..
            })
        ));
    }

    #[test]
    fn postfix_try_parses_inside_expression() {
        let src = r#"
postfixTryExpressionDeferred {
    fn:
        one() -> Result<int, str> {
            public:
                return Ok(1);
        }
        run() -> int {
            public:
                return one()? + 1;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("postfix try must parse inside an expression");
        let functions = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => Some(functions),
                _ => None,
            })
            .expect("function section");
        let run = functions
            .iter()
            .find(|function| function.name == "run")
            .expect("run function");
        assert!(matches!(
            &run.public_body[0],
            Stmt::Return(Expr::Binary {
                left,
                ..
            }) if matches!(
                left.as_ref(),
                Expr::Unary {
                    op: UnaryOp::TryPropagate,
                    ..
                }
            )
        ));
    }

    #[test]
    fn indexed_field_postfix_parse_ok() {
        let src = r#"
rootIndexedFieldPostfix[@] {
    stc:
        packets: vector<Packet>;
        i: int;
        out: qword;
    fn:
        run() -> bool {
            @out >> qword : @packets[@i].tag;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected indexed field postfix to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Projection {
                    value: Expr::MethodCall { receiver, method, args },
                    ..
                } if method == "tag"
                    && args.is_empty()
                    && matches!(receiver.as_ref(), Expr::Index { .. })
            )
        });
        assert!(
            found,
            "expected @packets[@i].tag to parse as a no-arg postfix field call over Expr::Index"
        );
    }

    #[test]
    fn get_field_postfix_parse_ok() {
        let src = r#"
rootGetFieldPostfix[@] {
    stc:
        packets: vector<Packet>;
        i: int;
        out: qword;
    fn:
        run() -> bool {
            @out >> qword : @packets.get(@i).tag;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected get field postfix to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Projection {
                    value: Expr::MethodCall { receiver, method, args },
                    ..
                } if method == "tag"
                    && args.is_empty()
                    && matches!(
                        receiver.as_ref(),
                        Expr::MethodCall { method: inner_method, args: inner_args, .. }
                            if inner_method == "get" && inner_args.len() == 1
                    )
            )
        });
        assert!(
            found,
            "expected @packets.get(@i).tag to parse as nested postfix method calls"
        );
    }

    #[test]
    fn indexed_field_lvalue_parse_ok() {
        let src = r#"
rootIndexedFieldLvalue[@] {
    stc:
        packets: vector<Packet>;
        i: int;
    fn:
        run() -> bool {
            @packets[@i].tag >> int : 42;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected indexed field lvalue to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Expr(Expr::MethodCall { method, args, .. })
                    if method == "arrayFieldWrite"
                        && args.len() == 3
                        && matches!(args.get(1), Some(Expr::ContextRef(ctx)) if ctx.path == ["tag"])
            )
        });
        assert!(
            found,
            "expected @packets[@i].tag >> ... to lower into arrayFieldWrite"
        );
    }

    #[test]
    fn indexed_nested_field_lvalue_parse_ok() {
        let src = r#"
rootIndexedNestedFieldLvalue[@] {
    stc:
        packets: vector<Packet>;
        i: int;
    fn:
        run() -> bool {
            @packets[@i].header.mark >> int : 42;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected indexed nested field lvalue to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Expr(Expr::MethodCall { method, args, .. })
                    if method == "arrayFieldWrite"
                        && args.len() == 3
                        && matches!(
                            args.get(1),
                            Some(Expr::ContextRef(ctx)) if ctx.path == ["header", "mark"]
                        )
            )
        });
        assert!(
            found,
            "expected @packets[@i].header.mark >> ... to lower into arrayFieldWrite"
        );
    }

    #[test]
    fn get_nested_field_lvalue_parse_ok() {
        let src = r#"
rootGetNestedFieldLvalue[@] {
    stc:
        packets: vector<Packet>;
        i: int;
    fn:
        run() -> bool {
            @packets.get(@i).header.mark >> int : 42;
            return true;
        }
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected get nested field lvalue to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected run function");
        let found = run_fn.public_body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::Expr(Expr::MethodCall { method, args, .. })
                    if method == "arrayFieldWrite"
                        && args.len() == 3
                        && matches!(
                            args.get(1),
                            Some(Expr::ContextRef(ctx)) if ctx.path == ["header", "mark"]
                        )
            )
        });
        assert!(
            found,
            "expected @packets.get(@i).header.mark >> ... to lower into arrayFieldWrite"
        );
    }

    #[test]
    fn root_at_while_loop_parses_in_function_body() {
        let src = r#"
rootWhileLoop[@] {
    fn:
        run() -> bool {
            x >> int : 2;
            while (@x > 0) {
                x >> int : @x - 1;
            }
            return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected while statement to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_canonical_print_surface_parses() {
        let src = r#"
rootPrintSurface[@] {
    fn:
        demo() -> bool {
            public:
                print("a");
                println("b");
                println();
                return true;
        }

    final:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected canonical print surface to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_print_sp_with_argument_fails_officially() {
        let src = r#"
rootPrintSpInvalid[@] {
    fn:
        demo() -> bool {
            public:
                print.sp("x");
                return true;
        }

    final:
        return true;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("E_PRINT_SURFACE_001"),
            "unexpected parser diagnostic for print.sp arg misuse: {msg}"
        );
    }

    #[test]
    fn root_at_println_without_args_parses() {
        let src = r#"
rootPrintlnNoArgs[@] {
    fn:
        demo() -> bool {
            public:
                println();
                return true;
        }

    final:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected println() without args to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_unknown_print_mode_fails_with_official_diagnostic() {
        let src = r#"
rootPrintInvalid[@] {
    fn:
        demo() -> bool {
            public:
                print.bad("x");
                return true;
        }

    final:
        return true;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("E_PRINT_SURFACE_001"),
            "unexpected parser diagnostic for unknown print mode: {msg}"
        );
    }

    #[test]
    fn root_at_print_dump_requires_two_arguments() {
        let src = r#"
rootPrintDumpArity[@] {
    fn:
        demo() -> bool {
            public:
                print.dump("x");
                return true;
        }

    final:
        return true;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("E_PRINT_SURFACE_001"),
            "unexpected parser diagnostic for print.dump arity: {msg}"
        );
    }

    #[test]
    fn root_at_print_dbg_rejects_two_arguments() {
        let src = r#"
rootPrintDbgArity[@] {
    fn:
        demo() -> bool {
            public:
                print.dbg("x", 1);
                return true;
        }

    final:
        return true;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("E_PRINT_SURFACE_001"),
            "unexpected parser diagnostic for print.dbg extra arg: {msg}"
        );
    }

    #[test]
    fn root_at_site_projection_print_sugar_parses_as_builtin_print_call() {
        let src = r#"
rootSiteProjectionSugar[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : 7;
                @value >> site.region.print;
                @value >> site.addr.print;
                @value >> site.class.print;
                @value >> site.type.print;
                @value >> site.state.print;
                @value >> site.owner.print;
                @value >> site.size.print;
                @value >> site.align.print;
                return true;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("site projection sugar must parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected one function");
        let builtins: Vec<String> = run_fn
            .public_body
            .iter()
            .filter_map(|stmt| match stmt {
                Stmt::Print {
                    kind: PrintKind::Std,
                    value: Expr::Call { callee, .. },
                } => Some(callee.clone()),
                _ => None,
            })
            .collect();
        assert!(
            builtins.contains(&"sitePrintRegion".to_string())
                && builtins.contains(&"sitePrintAddr".to_string())
                && builtins.contains(&"sitePrintClass".to_string())
                && builtins.contains(&"sitePrintType".to_string())
                && builtins.contains(&"sitePrintState".to_string())
                && builtins.contains(&"sitePrintOwner".to_string())
                && builtins.contains(&"sitePrintSize".to_string())
                && builtins.contains(&"sitePrintAlign".to_string()),
            "expected site.*.print sugar to normalize into builtin print calls, got {:?}",
            builtins
        );
    }

    #[test]
    fn root_at_site_projection_print_invalid_field_fails_officially() {
        let src = r#"
rootSiteProjectionInvalid[@] {
    stc:
        value: int;

    fn:
        run() -> bool {
            public:
                @value >> int : 1;
                @value >> site.layout.print;
                return true;
        }

    fa:
        return true;
}
"#;
        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid site projection field"),
            "unexpected parser diagnostic for invalid site projection field: {msg}"
        );
    }

    #[test]
    fn root_at_stc_then_to_main_without_fn_section_parses() {
        let src = r#"
rootStcToMain[@] {
    stc:
        a: int;
        b: int;

    to main {
        @a >> int : 1;
        @b >> int : @a + 1;
        print(@b);
    }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected stc -> to main sequence to parse without fn section, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_extended_control_flow_surface_parses() {
        let src = r#"
rootControlFlowSurface[@] {
    stc:
        items: list<int>;
        run: bool;
        i: int;

    fn:
        runFlow() -> bool {
            public:
                @run >> bool : true;
                @i >> int : 0;

                while (@run) {
                    break;
                }

                do {
                    @i >> int : @i + 1;
                } while (@i < 3);

                for (@i >> int : 0; @i < 3; @i >> int : @i + 1) {
                    continue;
                }

                for item in @items {
                    @i >> int : @item;
                }

                for n in 0..3 {
                    @i >> int : @n;
                }

                for n in 0..=3 step 1 {
                    @i >> int : @n;
                }

                async for a in @items {
                    @i >> int : @a;
                }

                parallel for p in 0..3 {
                    @i >> int : @p;
                }

                comptime for d in 3..=1 step -1 {
                    @i >> int : @d;
                }

                while let ok(v: int) = ok(1) {
                    @i >> int : @v;
                    break;
                }

                loop {
                    break;
                }

                repeat {
                    @i >> int : @i + 1;
                } until (@i > 5);

                same for (2) {
                    @i >> int : @i + 1;
                }

                same in v : @items {
                    @i >> int : @v;
                }

                each e in @items do {
                    print(@e);
                } while (@run);

                if (@run) {
                    @i >> int : 1;
                } else if (@i > 0) {
                    @i >> int : 2;
                } else {
                    @i >> int : 3;
                }

                Notif (@run) {
                    @i >> int : 4;
                } else {
                    @i >> int : 5;
                }

                when (@run) {
                    @i >> int : 6;
                }

                unless (@run) {
                    @i >> int : 7;
                }

                switch (@i) {
                    case 0 {
                        @i >> int : 8;
                    }
                    default {
                        @i >> int : 9;
                    }
                }

                match (@i) {
                    case 0 => {
                        @i >> int : 10;
                    }
                    default => {
                        @i >> int : 11;
                    }
                }

                guard (@run) else {
                    dev(irregular);
                }

                select {
                    when (@run) {
                        @i >> int : 12;
                    }
                    default {
                        @i >> int : 13;
                    }
                }

                labelA:
                while (@i > 0) {
                    break labelA;
                }

                outer:
                loop {
                    inner:
                    loop {
                        continue outer;
                    }
                }

                @i >> int : if (@run) { 1 } else { 0 };
                return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected extended control-flow surface to parse, got {:?}",
            result
        );
    }

    #[test]
    fn async_parallel_comptime_for_preserve_loop_flavor_in_ast() {
        let src = r#"
flavoredLoops[@runtime] {
    stc:
        items: array<int, 2>;
        sum: int;

    fn:
        run() -> bool {
            public:
                async for a in @items {
                    @sum >> int : @sum + @a;
                }
                parallel for p in 0..3 {
                    @sum >> int : @sum + @p;
                }
                comptime for c in 0..=2 {
                    @sum >> int : @sum + @c;
                }
                return true;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("flavored loop surface must parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected one function");
        let labels: Vec<&str> = run_fn
            .public_body
            .iter()
            .filter_map(|stmt| match stmt {
                Stmt::Labeled { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels,
            vec![
                "__ncc_loop_flavor_async",
                "__ncc_loop_flavor_parallel",
                "__ncc_loop_flavor_comptime"
            ]
        );
    }

    #[test]
    fn root_at_select_keyword_can_still_parse_as_call_expression() {
        let src = r#"
rootSelectCallCompat[@] {
    fn:
        run() -> bool {
            public:
                select("openai", "gpt-5.5");
                return true;
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected select(...) call expression to parse");
        let container = program.containers.first().expect("expected one container");
        let functions = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(funcs) => Some(funcs),
                _ => None,
            })
            .expect("expected fn section");
        let run_fn = functions.first().expect("expected one function");

        assert!(
            run_fn.public_body.iter().any(
                |stmt| matches!(stmt, Stmt::Expr(Expr::Call { callee, .. }) if callee == "select")
            ),
            "expected select(...) to remain callable as regular expression statement"
        );
    }

    #[test]
    fn root_at_fn_call_compat_form_parses_in_probe_condition() {
        let src = r#"
rootFnCallCompat[@] {
    fn:
        worker(src: addr, dst: addr) -> bool {
            return true;
        }

    probe:
        if (!@fn(worker, 0x1000, 0x2000)) {
            dev(CompilerFault);
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected @fn(call, ...) compatibility form to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_strict_check_dev_clause_parses_in_stmt_and_expr() {
        let src = r#"
rootStrictDevCompat[@] {
    stc:
        p: ptr<byte> | null;

    fn:
        run() -> bool {
            p !> ptr<byte> | null : dev(MemoryOverflow | irregular);
            if (@p !> ptr<byte> | null : dev(MemoryOverflow | irregular)) {
                return true;
            }
            return false;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected strict-check dev compatibility form to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_stc_str_type_parses() {
        let src = r#"
rootStrType[@] {
    stc:
        tag: str;

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected stc str type declaration to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_raw_colon_section_parses() {
        let src = r#"
rootRawColon[@] {
    stc:
        x: int;

    raw:
        x >> int : 1;

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected raw colon section form to parse, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_runtime_paths_print_with_canonical_surface() {
        let src = r#"
rootInspectPaths[@] {
    fn:
        run() -> bool {
            println(heap.events.count);
            println(heap.events.valid);
            println(heap.events.error);
            println(net.connected);
            println(net.last_payload);
            return true;
        }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_ok(),
            "expected runtime paths to parse through println, got {:?}",
            result
        );
    }

    #[test]
    fn root_at_ref_main_direct_path_parses_with_typed_fnref() {
        let src = r#"
worker[@] {
    fn:
        public boot() -> bool {
            public:
                return true;
        }

    fa:
        return true;
}

entry[@] {
    ref main ->> worker.boot;

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected direct path ref main to parse");
        let entry = program
            .containers
            .iter()
            .find(|c| c.name == "entry")
            .expect("fixture must contain entry container");
        let main = entry
            .sections
            .iter()
            .find_map(|section| match section {
                Section::MainEntry { kind, .. } => Some(kind),
                _ => None,
            })
            .expect("entry must contain main entry");
        assert!(matches!(
            main,
            MainEntryKind::RefFn(FnRef { path }) if path == &vec!["worker".to_string(), "boot".to_string()]
        ));
    }

    #[test]
    fn root_at_ref_main_inline_body_fails_with_official_diagnostic() {
        let src = r#"
entry[@] {
    fn:
        boot() -> bool {
            public:
                return true;
        }

    ref main ->> @fn(boot) {
        print("invalid");
    }

    fa:
        return true;
}
"#;
        let result = parse_source(src);
        assert!(
            result.is_err(),
            "expected ref main inline body to fail in parser"
        );
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("ref main cannot declare an inline body"),
            "unexpected parser diagnostic: {}",
            msg
        );
    }

    #[test]
    fn root_at_skk_main_direct_path_parses_with_typed_fnref() {
        let src = r#"
worker[@] {
    fn:
        public boot() -> bool {
            public:
                return true;
        }

    fa:
        return true;
}

entry[@] {
    skkMain ->> worker.boot;

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected direct path skkMain to parse");
        let entry = program
            .containers
            .iter()
            .find(|c| c.name == "entry")
            .expect("fixture must contain entry container");
        let main = entry
            .sections
            .iter()
            .find_map(|section| match section {
                Section::MainEntry { kind, .. } => Some(kind),
                _ => None,
            })
            .expect("entry must contain main entry");
        assert!(matches!(
            main,
            MainEntryKind::SkkFn(FnRef { path }) if path == &vec!["worker".to_string(), "boot".to_string()]
        ));
    }

    #[test]
    fn root_at_skk_main_two_word_alias_parses_with_typed_fnref() {
        let src = r#"
worker[@] {
    fn:
        public boot() -> bool {
            public:
                return true;
        }

    fa:
        return true;
}

entry[@] {
    skk main ->> worker.boot;

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected 'skk main' compatibility alias to parse");
        let entry = program
            .containers
            .iter()
            .find(|c| c.name == "entry")
            .expect("fixture must contain entry container");
        let main = entry
            .sections
            .iter()
            .find_map(|section| match section {
                Section::MainEntry { kind, .. } => Some(kind),
                _ => None,
            })
            .expect("entry must contain main entry");
        assert!(matches!(
            main,
            MainEntryKind::SkkFn(FnRef { path }) if path == &vec!["worker".to_string(), "boot".to_string()]
        ));
    }

    #[test]
    fn root_at_prefixed_context_header_parses() {
        let src = r#"
[@] prefixedRoot {
    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected prefixed [@] header to parse");
        let root = program.containers.first().expect("expected one container");
        assert_eq!(root.name, "prefixedRoot");
        assert!(matches!(root.context, Some(Context::Root)));
    }

    #[test]
    fn root_at_probe_section_function_compat_and_probe_call_parse() {
        let src = r#"
entry[@] {
    stc:
        ok: bool;

    probe:
        validar() -> bool {
            return true;
        }

    to main {
        @probe(validar);
        @ok >> bool : true;
    }

    fa:
        return true;
}
"#;
        let program =
            parse_source(src).expect("expected probe function compatibility surface to parse");
        let entry = program.containers.first().expect("expected one container");
        assert!(
            entry
                .sections
                .iter()
                .any(|section| matches!(section, Section::Fn(_))),
            "expected probe-declared compatibility function(s) to lower into fn section"
        );
    }

    #[test]
    fn probe_section_distinguishes_direct_calls_from_function_headers() {
        let src = r#"
entry[@] {
    stc:
        src >> addr : 0x1000;
        dst >> addr : 0x2000;

    worker :: fn(src >> addr, dst >> addr) -> bool { return true; }

    probe:
        worker(src, dst);

    fa:
        return true;
}
"#;
        parse_source(src).expect("a direct probe call must not become a function declaration");
    }
}

#[cfg(test)]
mod primitive_f1_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("primitive_at", name)
    }

    #[test]
    fn primitive_at_minimal_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("primitive_at_minimal_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected valid [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_extended_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("primitive_at_extended_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected extended [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_directive_surface_ok_parses() {
        let src =
            std::fs::read_to_string(fixture_path("primitive_at_directive_surface_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected directive-rich [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_material_blocks_ok_parses() {
        let src =
            std::fs::read_to_string(fixture_path("primitive_at_material_blocks_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected material block [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_boot_serial_material_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("primitive_at_boot_serial_material_ok.n"))
            .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected boot-serial material [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_boot_serial_short_header_ok_parses() {
        let src =
            std::fs::read_to_string(fixture_path("primitive_at_boot_serial_short_header_ok.n"))
                .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected short-header boot-serial [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_nested_header_form_parses() {
        let src = r#"
bootSurface[primitive[@]] {
    anchor:
        entryBase;

    bridge:
        root.policy.bind;
        machine.bootCore;

    seal:
        bootClosed;
}
"#;

        let program = parse_source(src).expect("expected nested primitive header form to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "bootSurface")
            .expect("fixture must contain bootSurface container");
        assert!(
            matches!(container.context, Some(Context::Primitive)),
            "expected nested header to map into [@primitive] context, got {:?}",
            container.context
        );
    }

    #[test]
    fn primitive_at_anchor_inline_clause_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("primitive_at_anchor_inline_clause_ok.n"))
            .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected inline-clause anchor [@primitive] file to parse, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_at_invalid_fn_fails() {
        let src =
            std::fs::read_to_string(fixture_path("primitive_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@primitive] context"),
            "unexpected primitive parser error: {msg}"
        );
    }

    #[test]
    fn primitive_at_bridge_root_machine_ok_parses_with_typed_targets() {
        let src = r#"
primitiveBridgeTargets[@primitive] {
    anchor:
        baseEdge;

    bridge:
        root.policy.bind;
        machine.machineCore;

    seal:
        primitiveClosed;
}
"#;

        let program = parse_source(src).expect("expected primitive bridge targets to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveBridgeTargets")
            .expect("fixture must contain primitiveBridgeTargets container");

        let bridge = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Bridge(items) => Some(items),
                _ => None,
            })
            .expect("fixture must contain bridge section");

        assert_eq!(
            bridge,
            &vec![
                PrimitiveBridgeTarget::RootPath(vec![
                    "root".into(),
                    "policy".into(),
                    "bind".into()
                ]),
                PrimitiveBridgeTarget::MachinePath(vec!["machine".into(), "machineCore".into()]),
            ]
        );
    }

    #[test]
    fn primitive_at_bridge_invalid_target_fails_in_parser() {
        let src = r#"
primitiveBridgeInvalid[@primitive] {
    anchor:
        baseEdge;

    bridge:
        rootToPrimitive;
}
"#;

        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(
                "bridge section in [@primitive] only allows 'root.<...>' or 'machine.<...>'"
            ),
            "unexpected primitive bridge parser error: {msg}"
        );
    }

    #[test]
    fn primitive_at_bridge_machine_nested_target_fails_in_parser() {
        let src = r#"
primitiveBridgeMachineNested[@primitive] {
    anchor:
        baseEdge;

    bridge:
        machine.machineCore.entry;
}
"#;

        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains(
                "bridge section in [@primitive] only allows 'root.<...>' or 'machine.<...>'"
            ),
            "unexpected primitive bridge parser error: {msg}"
        );
    }

    #[test]
    fn primitive_at_directive_section_outside_primitive_fails_in_parser() {
        let src = r#"
rootDirectiveOutside[@] {
    id:
        coreId;
}
"#;

        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed outside [@primitive] context"),
            "unexpected primitive directive parser error: {msg}"
        );
    }

    #[test]
    fn primitive_at_unknown_identifier_section_fails_with_official_diagnostic() {
        let src = r#"
primitiveUnknownSection[@primitive] {
    anchor:
        baseEdge;

    latencyy:
        drift;
}
"#;

        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@primitive] context"),
            "unexpected unknown primitive section parser error: {msg}"
        );
    }

    #[test]
    fn primitive_at_reg_directive_maps_to_register_kind() {
        let src = r#"
primitiveRegDirective[@primitive] {
    anchor:
        baseEdge;

    reg:
        shadowReg;

    bridge:
        root.policy.bind;

    seal:
        primitiveClosed;
}
"#;

        let program = parse_source(src).expect("expected primitive reg directive to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveRegDirective")
            .expect("fixture must contain primitiveRegDirective container");

        let directive = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::PrimitiveDirective(section) => Some(section),
                _ => None,
            })
            .expect("fixture must contain a primitive directive section");

        assert_eq!(directive.kind, PrimitiveDirectiveKind::Register);
        assert_eq!(
            directive.items,
            vec![PrimitiveDirectiveItem {
                atoms: vec![PrimitiveDirectiveAtom::Word("shadowReg".to_string())]
            }]
        );
    }

    #[test]
    fn primitive_at_dma_channel_directive_maps_to_typed_kind() {
        let src = r#"
primitiveDmaDirective[@primitive] {
    anchor:
        baseEdge;

    dmaChannel:
        ch0;

    bridge:
        root.policy.bind;

    seal:
        primitiveClosed;
}
"#;

        let program = parse_source(src).expect("expected primitive dmaChannel directive to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveDmaDirective")
            .expect("fixture must contain primitiveDmaDirective container");

        let directive = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::PrimitiveDirective(section) => Some(section),
                _ => None,
            })
            .expect("fixture must contain a primitive directive section");

        assert_eq!(directive.kind, PrimitiveDirectiveKind::DmaChannel);
        assert_eq!(
            directive.items,
            vec![PrimitiveDirectiveItem {
                atoms: vec![PrimitiveDirectiveAtom::Word("ch0".to_string())]
            }]
        );
    }

    #[test]
    fn primitive_at_advanced_physical_directives_map_to_typed_kinds() {
        let src = r#"
primitiveAdvancedPhysical[@primitive] {
    anchor:
        boot0;

    voltage:
        vcore;
    current:
        rail0;
    noise:
        emi0;
    metastability:
        cdc0;
    power:
        pstate0;
    powerGate:
        rail_gate0;
    temperature:
        sensor0;
    thermalState:
        thermal_nominal;
    throttleState:
        throttle_none;
    watchdog:
        wd0;
    interruptLine:
        irq0;
    dmaChannel:
        dma0;
    cacheState:
        l1_shared;
    tlbState:
        tlb_hot;
    firmwareRegion:
        fw0;
    bootRomRegion:
        rom0;
    biosRegion:
        bios0;
    uefiHandoff:
        handoff0;
    resetVector:
        reset0;
    bootWindow:
        bootwin0;
    bootAnchor:
        boot_anchor0;
    firmwareTable:
        fwtable0;
    deviceTree:
        dt0;
    acpiTable:
        acpi0;
    microcodeState:
        patch_state0;
    controller:
        ctrl0;
    peripheral:
        dev0;
    phy:
        phy0;
    lane:
        lane0;
    channel:
        ch0;
    pipelineState:
        pipe_nominal;
    branchPredictorState:
        bp_warm;
    reorderState:
        rob_clean;
    outOfOrder:
        ooo_enabled;
    physicalFault:
        thermal_trip;
    timingViolation:
        missed_deadline;
    thermalViolation:
        overheat;
    metastabilityRisk:
        cdc_risk;
    dmaViolation:
        dma_race;
    cacheCoherenceViolation:
        stale_line;
    mmioSideEffectViolation:
        read_clears_flag;

    bridge:
        root.policy.bind;

    seal:
        primitiveClosed;
}
"#;

        let program =
            parse_source(src).expect("expected advanced primitive physical directives to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveAdvancedPhysical")
            .expect("fixture must contain primitiveAdvancedPhysical container");

        let kinds: std::collections::HashSet<PrimitiveDirectiveKind> = container
            .sections
            .iter()
            .filter_map(|section| match section {
                Section::PrimitiveDirective(section) => Some(section.kind.clone()),
                _ => None,
            })
            .collect();

        for kind in [
            PrimitiveDirectiveKind::Voltage,
            PrimitiveDirectiveKind::Current,
            PrimitiveDirectiveKind::Noise,
            PrimitiveDirectiveKind::Metastability,
            PrimitiveDirectiveKind::Power,
            PrimitiveDirectiveKind::PowerGate,
            PrimitiveDirectiveKind::Temperature,
            PrimitiveDirectiveKind::ThermalState,
            PrimitiveDirectiveKind::ThrottleState,
            PrimitiveDirectiveKind::Watchdog,
            PrimitiveDirectiveKind::InterruptLine,
            PrimitiveDirectiveKind::DmaChannel,
            PrimitiveDirectiveKind::CacheState,
            PrimitiveDirectiveKind::TlbState,
            PrimitiveDirectiveKind::FirmwareRegion,
            PrimitiveDirectiveKind::BootRomRegion,
            PrimitiveDirectiveKind::BiosRegion,
            PrimitiveDirectiveKind::UefiHandoff,
            PrimitiveDirectiveKind::ResetVector,
            PrimitiveDirectiveKind::BootWindow,
            PrimitiveDirectiveKind::BootAnchor,
            PrimitiveDirectiveKind::FirmwareTable,
            PrimitiveDirectiveKind::DeviceTree,
            PrimitiveDirectiveKind::AcpiTable,
            PrimitiveDirectiveKind::MicrocodeState,
            PrimitiveDirectiveKind::Controller,
            PrimitiveDirectiveKind::Peripheral,
            PrimitiveDirectiveKind::Phy,
            PrimitiveDirectiveKind::Lane,
            PrimitiveDirectiveKind::Channel,
            PrimitiveDirectiveKind::PipelineState,
            PrimitiveDirectiveKind::BranchPredictorState,
            PrimitiveDirectiveKind::ReorderState,
            PrimitiveDirectiveKind::OutOfOrder,
            PrimitiveDirectiveKind::PhysicalFault,
            PrimitiveDirectiveKind::TimingViolation,
            PrimitiveDirectiveKind::ThermalViolation,
            PrimitiveDirectiveKind::MetastabilityRisk,
            PrimitiveDirectiveKind::DmaViolation,
            PrimitiveDirectiveKind::CacheCoherenceViolation,
            PrimitiveDirectiveKind::MmioSideEffectViolation,
        ] {
            assert!(
                kinds.contains(&kind),
                "expected directive kind '{kind:?}' to be parsed"
            );
        }
    }

    #[test]
    fn primitive_at_target_capability_bitfield_operation_and_material_sections_ok_parses() {
        let src = r#"
primitiveExtended[@primitive] target x86_64 {
    capability boot_serial_io:
        allow port.read;
        deny signal.sample;

    bitfield uart_status byte:
        bit 0 data_ready;
        bits 5..7 status_hi;

    type page_size = bytes(4096);

    operation uart0.emit_byte(value: byte):
        capability boot_serial_io;
        timing: await uart0.tx_ready == high timeout 100 cycles;
        body: write uart0.data <- value;

    port uart0.data:
        bus io8;
        width 8;

    signature boot_magic:
        required true;

    invariant boot_sector_exact:
        boot_sector.size == bytes(512);

    constraint page_entry_reserved:
        bits 52..62 reserved;

    entry boot_entry:
        mode real16;

    anchor:
        boot0;

    bridge:
        root.policy.bind;

    seal:
        primitiveClosed;
}
"#;

        let program = parse_source(src).expect("expected extended primitive sections to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveExtended")
            .expect("fixture must contain primitiveExtended container");

        assert!(matches!(
            container.sections.first(),
            Some(Section::PrimitiveTarget(arch)) if arch == "x86_64"
        ));

        let capability = container.sections.iter().find_map(|section| match section {
            Section::Capability(section) => Some(section),
            _ => None,
        });
        let capability = capability.expect("expected capability section");
        assert_eq!(capability.name, "boot_serial_io");
        assert_eq!(capability.rules.len(), 2);

        let bitfield = container.sections.iter().find_map(|section| match section {
            Section::Bitfield(section) => Some(section),
            _ => None,
        });
        let bitfield = bitfield.expect("expected bitfield section");
        assert_eq!(bitfield.name, "uart_status");
        assert_eq!(bitfield.base, "byte");
        assert_eq!(bitfield.entries.len(), 2);

        let type_alias = container.sections.iter().find_map(|section| match section {
            Section::PrimitiveTypeAlias(section) => Some(section),
            _ => None,
        });
        let type_alias = type_alias.expect("expected primitive type alias section");
        assert_eq!(type_alias.name, "page_size");
        assert_eq!(type_alias.value.atoms.len(), 4);

        let operation = container.sections.iter().find_map(|section| match section {
            Section::Operation(section) => Some(section),
            _ => None,
        });
        let operation = operation.expect("expected operation section");
        assert_eq!(
            operation.name,
            vec!["uart0".to_string(), "emit_byte".to_string()]
        );
        assert_eq!(operation.params.len(), 1);
        assert_eq!(operation.clauses.len(), 3);

        let port = container.sections.iter().find_map(|section| match section {
            Section::Port(section) => Some(section),
            _ => None,
        });
        let port = port.expect("expected port section");
        assert_eq!(port.name, vec!["uart0".to_string(), "data".to_string()]);
        assert_eq!(port.clauses.len(), 2);

        let signature = container.sections.iter().find_map(|section| match section {
            Section::Signature(section) => Some(section),
            _ => None,
        });
        let signature = signature.expect("expected signature section");
        assert_eq!(signature.name, vec!["boot_magic".to_string()]);
        assert_eq!(signature.clauses.len(), 1);

        let invariant = container.sections.iter().find_map(|section| match section {
            Section::Invariant(section) => Some(section),
            _ => None,
        });
        let invariant = invariant.expect("expected invariant section");
        assert_eq!(invariant.name, vec!["boot_sector_exact".to_string()]);
        assert_eq!(invariant.clauses.len(), 1);

        let constraint = container.sections.iter().find_map(|section| match section {
            Section::Constraint(section) => Some(section),
            _ => None,
        });
        let constraint = constraint.expect("expected constraint section");
        assert_eq!(constraint.name, vec!["page_entry_reserved".to_string()]);
        assert_eq!(constraint.clauses.len(), 1);

        let entry = container.sections.iter().find_map(|section| match section {
            Section::Entry(section) => Some(section),
            _ => None,
        });
        let entry = entry.expect("expected entry section");
        assert_eq!(entry.name, vec!["boot_entry".to_string()]);
        assert_eq!(entry.clauses.len(), 1);
    }

    #[test]
    fn primitive_at_nominal_event_contract_clauses_parse() {
        let src = r#"
primitiveNominalEvents[@primitive] {
    operation kernelEntry():
        sideEffect interrupt_preemption_contract;
        physicalEvent interrupt irq0;
        asyncHardwareEvent interrupt irq0;
        event physicalFault thermal_trip;
        haltLoop;

    anchor:
        boot0;

    bridge:
        root.policy.bind;

    seal:
        primitiveClosed;
}
"#;

        let program = parse_source(src).expect("expected nominal event clauses to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveNominalEvents")
            .expect("fixture must contain primitiveNominalEvents container");
        let operation = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Operation(op) if op.name.join(".") == "kernelEntry" => Some(op),
                _ => None,
            })
            .expect("fixture must contain kernelEntry operation");

        let clause_kinds: Vec<PrimitiveClauseKind> = operation
            .clauses
            .iter()
            .filter_map(PrimitiveDirectiveItem::clause_kind)
            .collect();
        assert!(
            clause_kinds.contains(&PrimitiveClauseKind::SideEffectContract),
            "operation must classify sideEffect as nominal clause kind"
        );
        assert!(
            clause_kinds.contains(&PrimitiveClauseKind::PhysicalEventContract),
            "operation must classify physicalEvent as nominal clause kind"
        );
        assert!(
            clause_kinds.contains(&PrimitiveClauseKind::AsyncHardwareEventContract),
            "operation must classify asyncHardwareEvent as nominal clause kind"
        );
        assert!(
            clause_kinds.contains(&PrimitiveClauseKind::EventPhysicalFault),
            "operation must keep legacy event classification compatibility"
        );
    }

    #[test]
    fn primitive_at_named_anchor_window_signal_bus_sections_ok_parses() {
        let src = r#"
primitiveMaterialBlocks[@primitive] target x86_64 {
    anchor boot0:
        at physaddr(0x0000_7C00);
        align 16;

    window boot_sector:
        base physaddr(0x0000_7C00);
        size bytes(512);

    signal uart0.tx_ready:
        sample volatile;
        stable after 1 cycle;

    bus legacy_io8:
        width 8;
        endian little;

    bridge:
        root.policy.bind;
        machine.machineCore;

    seal:
        primitiveClosed;
}
"#;

        let program = parse_source(src).expect("expected named primitive block sections to parse");
        let container = program
            .containers
            .iter()
            .find(|c| c.name == "primitiveMaterialBlocks")
            .expect("fixture must contain primitiveMaterialBlocks container");

        let anchor = container.sections.iter().find_map(|section| match section {
            Section::AnchorBlock(section) => Some(section),
            _ => None,
        });
        let anchor = anchor.expect("expected anchor block section");
        assert_eq!(anchor.name, vec!["boot0".to_string()]);
        assert_eq!(anchor.clauses.len(), 2);

        let window = container.sections.iter().find_map(|section| match section {
            Section::WindowBlock(section) => Some(section),
            _ => None,
        });
        let window = window.expect("expected window block section");
        assert_eq!(window.name, vec!["boot_sector".to_string()]);
        assert_eq!(window.clauses.len(), 2);

        let signal = container.sections.iter().find_map(|section| match section {
            Section::SignalBlock(section) => Some(section),
            _ => None,
        });
        let signal = signal.expect("expected signal block section");
        assert_eq!(
            signal.name,
            vec!["uart0".to_string(), "tx_ready".to_string()]
        );
        assert_eq!(signal.clauses.len(), 2);

        let bus = container.sections.iter().find_map(|section| match section {
            Section::BusBlock(section) => Some(section),
            _ => None,
        });
        let bus = bus.expect("expected bus block section");
        assert_eq!(bus.name, vec!["legacy_io8".to_string()]);
        assert_eq!(bus.clauses.len(), 2);
    }

    #[test]
    fn primitive_extended_section_outside_primitive_fails_in_parser() {
        let src = r#"
rootCapabilityOutside[@] {
    capability boot_serial_io:
        allow port.read;
}
"#;

        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed outside [@primitive] context"),
            "unexpected primitive extended parser error: {msg}"
        );
    }

    #[test]
    fn primitive_type_alias_outside_primitive_fails_in_parser() {
        let src = r#"
rootTypeAliasOutside[@] {
    type page_size = bytes(4096);
}
"#;

        let err = parse_source(src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed outside [@primitive] context"),
            "unexpected primitive type alias parser error: {msg}"
        );
    }

    #[test]
    fn primitive_at_colon_header_without_braces_parses() {
        let src = r#"
primitive[@] target x86_64:
    anchor:
        boot0;

    bridge:
        root.policy.bind;
        machine.machineCore;

    seal:
        primitiveClosed;
"#;

        let program = parse_source(src).expect("expected primitive colon header to parse");
        assert_eq!(program.containers.len(), 1);
        let container = &program.containers[0];
        assert_eq!(container.name, "primitive");
        assert!(matches!(container.context, Some(Context::Primitive)));
        assert!(matches!(
            container.sections.first(),
            Some(Section::PrimitiveTarget(arch)) if arch == "x86_64"
        ));
        assert!(container
            .sections
            .iter()
            .any(|section| matches!(section, Section::Bridge(_))));
        assert!(container
            .sections
            .iter()
            .any(|section| matches!(section, Section::Seal(_))));
    }

    #[test]
    fn primitive_at_colon_header_without_target_parses() {
        let src = r#"
primitive[@]:
    type page_size = bytes(4096);

    capability boot_serial_io:
        allow port.read;
        deny dma.map;

    anchor:
        boot0;

    bridge:
        root.policy.bind;
        machine.machineCore;

    seal:
        primitiveClosed;
"#;

        let program = parse_source(src).expect("expected primitive colon header without target");
        assert_eq!(program.containers.len(), 1);
        let container = &program.containers[0];
        assert_eq!(container.name, "primitive");
        assert!(matches!(container.context, Some(Context::Primitive)));
        assert!(container
            .sections
            .iter()
            .any(|section| matches!(section, Section::PrimitiveTypeAlias(_))));
        assert!(container
            .sections
            .iter()
            .any(|section| matches!(section, Section::Capability(_))));
    }
}

#[cfg(test)]
mod inspect_f7_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("inspect_at", name)
    }

    #[test]
    fn inspect_at_minimal_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("inspect_at_minimal_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected minimal [@inspect] fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn inspect_at_runtime_contract_ok_parses() {
        let src =
            std::fs::read_to_string(fixture_path("inspect_at_runtime_contract_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@inspect] runtime contract fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn inspect_at_heap_contract_missing_parses_for_sema() {
        let src = std::fs::read_to_string(fixture_path("inspect_at_heap_contract_missing_fail.n"))
            .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@inspect] heap contract negative fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn inspect_at_net_contract_missing_parses_for_sema() {
        let src = std::fs::read_to_string(fixture_path("inspect_at_net_contract_missing_fail.n"))
            .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@inspect] net contract negative fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn inspect_at_invalid_section_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("inspect_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@inspect] context"),
            "unexpected inspect parser error: {msg}"
        );
    }

    #[test]
    fn inspect_at_main_entry_parses_in_parser_v1_0_1_compat() {
        let src = std::fs::read_to_string(fixture_path("inspect_at_main_entry_fail.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@inspect] main entry fixture to parse in v1.0.1 compat mode, got {:?}",
            result
        );
    }
}

#[cfg(test)]
mod security_f8_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("security_at", name)
    }

    #[test]
    fn security_at_minimal_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("security_at_minimal_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected minimal [@security] fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_at_entities_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("security_at_entities_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@security] entity fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_named_sections_after_stc_are_not_absorbed_as_stc_decls() {
        let src = r#"
securityTyped[@security] {
    stc:
        defaultAction >> action : deny;
        audit >> bool : true;

    asset:
        paymentApi;

    final:
        return true;
}
"#;
        let program = parse_source(src).expect("security section parsing must succeed");
        let container = &program.containers[0];

        assert!(
            container
                .sections
                .iter()
                .any(|section| matches!(section, ncc_ast::Section::SecurityAsset(_))),
            "expected parsed security asset section after stc"
        );

        let has_asset_stc = container.sections.iter().any(|section| match section {
            ncc_ast::Section::Stc(decls) => decls.iter().any(|decl| {
                matches!(
                    decl,
                    ncc_ast::StcDecl::Typed { name, .. } if name == "asset"
                )
            }),
            _ => false,
        });
        assert!(
            !has_asset_stc,
            "security asset section must not be parsed as stc declaration"
        );
    }

    #[test]
    fn security_at_attack_import_without_lab_parses_for_sema() {
        let src =
            std::fs::read_to_string(fixture_path("security_at_attack_import_without_lab_fail.n"))
                .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@security] attack import negative fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_at_attack_import_with_lab_parses_for_sema() {
        let src = std::fs::read_to_string(fixture_path("security_at_attack_import_with_lab_ok.n"))
            .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@security] attack import lab fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_at_runtime_allow_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("security_at_runtime_allow_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected runtime [@] security allow fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_at_runtime_inspect_paths_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("security_at_runtime_inspect_paths_ok.n"))
            .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected runtime [@] security inspect fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_at_inspect_contract_missing_import_parses_for_sema() {
        let src = std::fs::read_to_string(fixture_path(
            "security_at_inspect_contract_missing_import_fail.n",
        ))
        .unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected inspect contract negative fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn security_at_invalid_section_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("security_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@security] context"),
            "unexpected security parser error: {msg}"
        );
    }

    #[test]
    fn security_at_main_entry_parses_in_parser_v1_0_1_compat() {
        let src = std::fs::read_to_string(fixture_path("security_at_main_entry_fail.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@security] main entry fixture to parse in v1.0.1 compat mode, got {:?}",
            result
        );
    }
}

#[cfg(test)]
mod network_f9_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("network_at", name)
    }

    #[test]
    fn network_at_minimal_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("network_at_minimal_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected minimal [@network] fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn network_at_flow_contract_ok_parses() {
        let src = std::fs::read_to_string(fixture_path("network_at_flow_contract_ok.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@network] flow fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn network_at_runtime_bad_port_fail_parses_for_runtime() {
        let src =
            std::fs::read_to_string(fixture_path("network_at_runtime_bad_port_fail.n")).unwrap();
        let result = parse_source(&src);
        assert!(
            result.is_ok(),
            "expected [@network] runtime bad-port fixture to parse, got {:?}",
            result
        );
    }

    #[test]
    fn network_at_invalid_section_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("network_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@network] context"),
            "unexpected network parser error: {msg}"
        );
    }
}

#[cfg(test)]
mod high_f10_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("f10_at", name)
    }

    #[test]
    fn f10_high_layer_minimal_fixtures_parse() {
        let fixtures = [
            "f10_system_minimal_ok.n",
            "f10_tooling_minimal_ok.n",
            "f10_compiler_minimal_ok.n",
            "f10_ai_minimal_ok.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F10 fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }

    #[test]
    fn f10_high_layer_import_fixtures_parse_for_sema() {
        let fixtures = [
            "f10_system_import_call_ok.n",
            "f10_tooling_import_call_ok.n",
            "f10_compiler_import_call_ok.n",
            "f10_system_import_context_fail.n",
            "f10_tooling_missing_final_fail.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F10 sema fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }

    #[test]
    fn f10_system_invalid_section_fails_in_parser() {
        let src = std::fs::read_to_string(fixture_path("f10_system_invalid_section_fail.n"))
            .expect("fixture must exist");
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@system] context"),
            "unexpected F10 parser error: {msg}"
        );
    }
}

#[cfg(test)]
mod system_f15_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("system_at", name)
    }

    #[test]
    fn system_at_operational_fixtures_parse_for_runtime() {
        let fixtures = [
            "system_at_lifecycle_ok.n",
            "system_at_recovery_ok.n",
            "system_at_runtime_empty_service_fail.n",
            "system_at_missing_baseline_stc_fail.n",
            "system_at_recover_without_degrade_fail.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F15 fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }
}

#[cfg(test)]
mod tooling_f16_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("tooling_at", name)
    }

    #[test]
    fn tooling_at_operational_fixtures_parse_for_runtime() {
        let fixtures = [
            "tooling_at_lifecycle_ok.n",
            "tooling_at_runtime_empty_target_fail.n",
            "tooling_at_missing_baseline_stc_fail.n",
            "tooling_at_publish_without_pack_fail.n",
            "tooling_at_runtime_inspect_paths_ok.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F16 fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }

    #[test]
    fn tooling_at_invalid_section_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("tooling_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@tooling] context"),
            "unexpected F16 parser error: {msg}"
        );
    }
}

#[cfg(test)]
mod compiler_f17_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("compiler_at", name)
    }

    #[test]
    fn compiler_at_operational_fixtures_parse_for_runtime() {
        let fixtures = [
            "compiler_at_lifecycle_ok.n",
            "compiler_at_alias_family_ok.n",
            "compiler_at_runtime_empty_unit_fail.n",
            "compiler_at_missing_baseline_stc_fail.n",
            "compiler_at_materialize_without_check_fail.n",
            "compiler_at_runtime_inspect_paths_ok.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F17 fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }

    #[test]
    fn compiler_at_invalid_section_fails_in_parser() {
        let src =
            std::fs::read_to_string(fixture_path("compiler_at_invalid_section_fail.n")).unwrap();
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@compiler] context"),
            "unexpected F17 parser error: {msg}"
        );
    }
}

#[cfg(test)]
mod kernel_f18_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("kernel_at", name)
    }

    #[test]
    fn kernel_at_operational_fixtures_parse_for_runtime() {
        let fixtures = [
            "kernel_at_lifecycle_ok.n",
            "kernel_at_alias_family_ok.n",
            "kernel_at_runtime_empty_task_fail.n",
            "kernel_at_runtime_inspect_paths_ok.n",
            "kernel_at_missing_baseline_stc_fail.n",
            "kernel_at_schedule_without_open_fail.n",
            "kernel_at_quarantine_after_panic_fail.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F18 fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }

    #[test]
    fn kernel_at_invalid_section_fails_in_parser() {
        let src = std::fs::read_to_string(fixture_path("kernel_at_invalid_section_fail.n"))
            .expect("fixture must exist");
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@kernel] context"),
            "unexpected F18 parser error: {msg}"
        );
    }
}

#[cfg(test)]
mod ai_f19_tests {
    use super::*;
    use ncc_lexer::lex;
    use std::path::PathBuf;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn fixture_path(name: &str) -> PathBuf {
        super::example_fixture_path("ai_at", name)
    }

    #[test]
    fn ai_at_operational_fixtures_parse_for_runtime() {
        let fixtures = [
            "ai_at_lifecycle_ok.n",
            "ai_at_alias_family_ok.n",
            "ai_at_runtime_empty_prompt_fail.n",
            "ai_at_runtime_inspect_paths_ok.n",
            "ai_at_missing_baseline_stc_fail.n",
            "ai_at_plan_without_model_fail.n",
            "ai_at_execute_without_critique_fail.n",
        ];

        for fixture in fixtures {
            let src = std::fs::read_to_string(fixture_path(fixture)).unwrap();
            let result = parse_source(&src);
            assert!(
                result.is_ok(),
                "expected F19 fixture '{}' to parse, got {:?}",
                fixture,
                result
            );
        }
    }

    #[test]
    fn ai_at_invalid_section_fails_in_parser() {
        let src = std::fs::read_to_string(fixture_path("ai_at_invalid_section_fail.n"))
            .expect("fixture must exist");
        let err = parse_source(&src).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("section not allowed in [@ai] context"),
            "unexpected F19 parser error: {msg}"
        );
    }
}

#[cfg(test)]
mod source_execution_mode_parser_tests {
    use super::*;
    use ncc_lexer::lex;

    fn parse_source(source: &str) -> Result<Program, Diagnostic> {
        parse(lex(source)?)
    }

    #[test]
    fn parser_accepts_open_package_profile_identifiers() {
        for profile in ["manual", "raw", "release_fast", "kernel", "vendor_lab"] {
            let program = parse_source(&format!(
                "package mode {{ profile: {profile}; }}\nentry {{ fa: return true; }}"
            ))
            .unwrap_or_else(|error| panic!("{profile} must parse: {error}"));
            assert_eq!(program.packages[0].profile.as_deref(), Some(profile));
        }
    }

    #[test]
    fn arbitrary_profiles_do_not_change_the_source_grammar() {
        let program =
            parse_source("package mode { profile: experimental_42; }\nentry { fa: return true; }")
                .expect("profile names are open metadata");
        assert_eq!(
            program.packages[0].profile.as_deref(),
            Some("experimental_42")
        );
    }

    #[test]
    fn compact_container_function_and_main_alias_preserve_the_canonical_ast() {
        let program =
            parse_source(
                "compactKernel {\nlocal:\n    scratch >> int = 0;\nverify() -> bool { return true; }\nmain: verify;\n}",
            )
                .expect("compact functions and main aliases must parse");
        let sections = &program.containers[0].sections;
        assert!(matches!(&sections[0], Section::Local(_)));
        assert!(matches!(&sections[1], Section::Fn(functions) if functions.len() == 1));
        assert!(matches!(
            &sections[2],
            Section::MainEntry {
                kind: MainEntryKind::RefFn(FnRef { path }),
                body
            } if path == &["verify".to_string()] && body.is_empty()
        ));
    }

    #[test]
    fn canonical_compact_surface_normalizes_imports_types_functions_and_initializers() {
        let source = r#"
use <io:output, render | memory:heap | data:json | process:args | text:text | audit:profile | core.sync:atomic | auth.system:process>;

manualRaw {
    const:
        CAP >> usize <- 4;

    Packet :: type [repr C | align 16 | endian little] {
        tag >> int;
        flags >> byte;
    }

    run :: fn(value >> int) -> bool {
        ok >> bool <- value == CAP;
        return ok ? true : false;
    }

    ref main ->> run;

    fa:
        return true;
}
"#;
        let program = parse_source(source).unwrap_or_else(|error| {
            panic!(
                "canonical compact source must parse: {error:?}\ntokens={:#?}",
                lex(source).expect("source must lex")
            )
        });

        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "output"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "heap"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "json"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "args"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "text"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "profile"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "atomic"]));
        assert!(program
            .uses
            .iter()
            .any(|decl| decl.path == ["nC", "runtime", "process"]));
        let sections = &program.containers[0].sections;
        assert!(matches!(
            sections.iter().find(|section| matches!(section, Section::TypeContracts(_))),
            Some(Section::TypeContracts(contracts))
                if matches!(contracts.as_slice(), [TypeContractDecl { name, clauses, .. }]
                    if name == "Packet"
                        && clauses.iter().any(|clause| matches!(clause, TypeContractClause::Layout(items)
                            if items == &["repr(C)", "align(16)", "endian(little)"])))
        ));
        assert!(matches!(
            sections.iter().find(|section| matches!(section, Section::Fn(_))),
            Some(Section::Fn(functions)) if functions.iter().any(|function| function.name == "run")
        ));
    }

    #[test]
    fn canonical_compact_functions_preserve_explicit_visibility() {
        let source = r#"
library {
    public ping :: fn() -> int { return 33; }
    private helper :: fn(value >> int) -> int { return value; }
    fa:
        return true;
}
"#;
        let program = parse_source(source).expect("visible canonical functions must parse");
        let functions = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => Some(functions),
                _ => None,
            })
            .expect("function section");
        assert!(matches!(functions[0].visibility, Some(Visibility::Public)));
        assert!(matches!(functions[1].visibility, Some(Visibility::Private)));
    }

    #[test]
    fn canonical_compact_indexed_writes_and_same_identifier_parse() {
        let source = r#"
indexed {
    stc:
        cells >> array<int, 4>;
        rows >> array<Packet, 2>;

    run :: fn() -> bool {
        same >> bool <- true;
        cells[2] = 77;
        rows[0].tag = 9;
        same = false;
        return !same;
    }

    ref main ->> run;
    fa:
        return true;
}
"#;
        parse_source(source).expect("indexed assignment and `same` identifiers must parse");
    }

    #[test]
    fn canonical_compact_loops_contextual_names_and_strict_checks_parse() {
        let source = r#"
compactFlow {
    stc:
        values >> array<int, 2>;

    stop :: fn() -> void { return; }

    run :: fn() -> bool {
        slot >> int <- 0;
        window >> int <- 2;
        ok >> bool <- slot !> int : slot;
        count >> int <- window;
        for index in 0..count {
            slot = slot + index;
        }
        same in value: values {
            slot += value;
        }
        return ok;
    }

    ref main ->> run;
    fa:
        return true;
}
"#;
        parse_source(source)
            .expect("canonical loops, contextual names and plain strict checks must parse");
    }

    #[test]
    fn canonical_compact_classic_for_and_async_function_parse() {
        let source = r#"
asyncFlow {
    fn:
        async fetch(value >> int) -> int {
            return value + 1;
        }

        run() -> bool effects(async.await) {
            sum >> int <- 0;
            twice >> fn(int) -> int <- |value >> int| -> int { return value * 2; };
            for (i >> int <- 0; i < 4; i += 1) {
                sum += i;
            }
            return twice(sum) == 12;
        }

    ref main ->> run;
    fa:
        return true;
}
"#;
        parse_source(source).expect("canonical async functions and classic for updates must parse");
    }

    #[test]
    fn canonical_compact_types_keep_advanced_contract_clauses() {
        let source = r#"
contracts {
    Packet :: type [repr C | align 16] {
        tag >> int;
        constructure:
            static(tag >> int) -> Packet;
        stable:
            deterministic;
        ffi:
            abi("nC.raw");
    }
    fa:
        return true;
}
"#;
        let program = parse_source(source).expect("advanced canonical type clauses must parse");
        let clauses = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::TypeContracts(contracts) => Some(&contracts[0].clauses),
                _ => None,
            })
            .expect("type contract clauses");
        assert!(clauses
            .iter()
            .any(|clause| matches!(clause, TypeContractClause::Constructure(_))));
        assert!(clauses
            .iter()
            .any(|clause| matches!(clause, TypeContractClause::Stable(_))));
        assert!(clauses
            .iter()
            .any(|clause| matches!(clause, TypeContractClause::Ffi(_))));
    }
}

#[cfg(test)]
mod destructure_constructure_nominal_parser_tests {
    use super::*;
    use ncc_lexer::lex;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn parser_supports_nominal_destructure_and_constructure_statements() {
        let src = r#"
entry[@] {
    fn:
        boot() -> bool {
            public:
                destructure.move(@payload);
                constructure.binary(1);
                return true;
        }

    ref main ->> @fn(boot);

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("parser should accept nominal statements");
        let mut saw_destructure = false;
        let mut saw_constructure = false;

        for container in &program.containers {
            for section in &container.sections {
                if let Section::Fn(items) = section {
                    for f in items {
                        for stmt in &f.public_body {
                            match stmt {
                                Stmt::Destructure { kind, .. } => {
                                    saw_destructure = true;
                                    assert_eq!(*kind, DestructureKind::Move);
                                }
                                Stmt::Constructure { kind, .. } => {
                                    saw_constructure = true;
                                    assert_eq!(*kind, ConstructureKind::Binary);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        assert!(saw_destructure, "expected nominal destructure stmt in AST");
        assert!(
            saw_constructure,
            "expected nominal constructure stmt in AST"
        );
    }
}

#[cfg(test)]
mod type_contract_parser_tests {
    use super::*;
    use ncc_lexer::lex;

    fn parse_source(src: &str) -> Result<ncc_ast::Program, ncc_diagnostics::Diagnostic> {
        let tokens = lex(src)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn contract_fixture(context: &str) -> String {
        format!(
            r#"
entry[{context}] {{
    type:
        User {{
            public:
                id: int;
                name: str;
            private:
                secret: text;
            internal:
                checksum: qword;
            layout:
                kind: unbinary;
                repr: C;
            bounds:
                name: len(1..64);
            invariant:
                @id >= 0;
            constructure:
                static(id: int, name: str) -> User;
            destructure:
                static -> {{ id, name }};
            copy:
                shallow allowed;
            clone:
                deep allowed;
            move:
                allowed;
            ffi:
                repr: C;
            ownership:
                name: owned;
            nullability:
                name: nonNull;
            methods:
                get(self: ref<User>) -> int;
            associated:
                make(id: int, name: str) -> User;
            validate:
                name: len(1..64);
            drop:
                release;
            default:
                zero;
            compare:
                eq;
            hash:
                stable;
            debug:
                printable;
            stable:
                deterministic;
        }}

    fa:
        return true;
}}
"#
        )
    }

    fn parse_first_contract(src: &str) -> TypeContractDecl {
        let program = parse_source(src).expect("expected type contract section to parse");
        let container = program.containers.first().expect("expected one container");
        let section = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::TypeContracts(items) => Some(items),
                _ => None,
            })
            .expect("expected type contract section");
        section.first().expect("expected one contract").clone()
    }

    #[test]
    fn parser_supports_type_contract_section() {
        let src = r#"
entry[@] {
    type:
        User {
            public:
                id: int;
                name: str;
            private:
                secret: text;
            layout:
                kind: unbinary;
            invariant:
                @id >= 0;
            constructure:
                static(id: int, name: str) -> User;
            destructure:
                static -> { id, name };
        }

    fa:
        return true;
}
"#;
        let program = parse_source(src).expect("expected type contract section to parse");
        let container = program.containers.first().expect("expected one container");
        let section = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::TypeContracts(items) => Some(items),
                _ => None,
            })
            .expect("expected type contract section");
        assert_eq!(section.len(), 1);
        assert_eq!(section[0].name, "User");
        assert!(
            section[0]
                .clauses
                .iter()
                .any(|clause| matches!(clause, TypeContractClause::Layout(_))),
            "expected layout clause in parsed type contract"
        );
    }

    #[test]
    fn compact_type_fields_are_public_without_a_visibility_block() {
        let src = r#"
compactTypes {
    type:
        Student {
            id >> int;
            name: str;
        }

    fa:
        return true;
}
"#;
        let contract = parse_first_contract(src);
        let fields = contract
            .clauses
            .iter()
            .find_map(|clause| match clause {
                TypeContractClause::Fields { visibility, fields } => Some((visibility, fields)),
                _ => None,
            })
            .expect("implicit public fields");
        assert!(matches!(fields.0, Visibility::Public));
        assert_eq!(fields.1.len(), 2);
        assert_eq!(fields.1[0].name, "id");
        assert_eq!(fields.1[1].name, "name");
    }

    #[test]
    fn local_record_accepts_compact_and_verbose_field_separators() {
        let src = r#"
compactLocalType {
    local:
        type Pair {
            left >> int;
            right: qword;
        }
        pair >> Pair;

    fn:
        run() -> bool {
            public:
                return true;
        }
}
"#;
        let program = parse_source(src).expect("compact local record must parse");
        let local = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Local(local) => Some(local),
                _ => None,
            })
            .expect("local section");
        let fields = local
            .decls
            .iter()
            .find_map(|decl| match decl {
                LocalDecl::TypeRecord { fields, .. } => Some(fields),
                _ => None,
            })
            .expect("local record fields");
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn parser_type_contract_requires_semicolon_in_clause() {
        let src = r#"
entry[@] {
    type:
        User {
            invariant:
                @id >= 0
        }

    fa:
        return true;
}
"#;
        let err = parse_source(src).expect_err("parser must reject missing semicolon");
        let msg = format!("{err}");
        assert!(
            msg.contains("Expected Semicolon"),
            "unexpected parser error: {msg}"
        );
    }

    #[test]
    fn type_contract_public_private_internal_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        let mut public_seen = false;
        let mut private_seen = false;
        let mut internal_seen = false;
        for clause in &contract.clauses {
            if let TypeContractClause::Fields { visibility, .. } = clause {
                match visibility {
                    Visibility::Public => public_seen = true,
                    Visibility::Private => private_seen = true,
                    Visibility::Internal => internal_seen = true,
                    _ => {}
                }
            }
        }
        assert!(public_seen && private_seen && internal_seen);
    }

    #[test]
    fn type_contract_layout_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        assert!(contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Layout(_))));
    }

    #[test]
    fn type_contract_bounds_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        assert!(contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Bounds(_))));
    }

    #[test]
    fn type_contract_invariant_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        assert!(contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Invariant(_))));
    }

    #[test]
    fn type_contract_constructure_destructure_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        let has_constructure = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Constructure(_)));
        let has_destructure = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Destructure(_)));
        assert!(has_constructure && has_destructure);
    }

    #[test]
    fn type_contract_copy_clone_move_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        let has_copy = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Copy(_)));
        let has_clone = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Clone(_)));
        let has_move = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Move(_)));
        assert!(has_copy && has_clone && has_move);
    }

    #[test]
    fn type_contract_ffi_repr_c_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        assert!(contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Ffi(_))));
    }

    #[test]
    fn type_contract_not_primitive_section_ok() {
        let src = contract_fixture("@runtime");
        let _ = parse_first_contract(&src);
    }

    #[test]
    fn type_contract_extended_sections_parse_ok() {
        let contract = parse_first_contract(&contract_fixture("@"));
        let has_methods = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Methods(_)));
        let has_associated = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Associated(_)));
        let has_validate = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Validate(_)));
        let has_drop = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Drop(_)));
        let has_stable = contract
            .clauses
            .iter()
            .any(|c| matches!(c, TypeContractClause::Stable(_)));
        assert!(has_methods && has_associated && has_validate && has_drop && has_stable);
    }

    #[test]
    fn type_contract_enum_header_parse_ok() {
        let src = r#"
entry[@] {
    type:
        PaymentState enum {
            Pending;
            Approved(id: int);
            Rejected(reason: str);
            layout:
                kind: unbinary;
        }

    fa:
        return true;
}
"#;
        let contract = parse_first_contract(src);
        assert_eq!(contract.kind, TypeContractKind::Enum);
        assert_eq!(contract.variants.len(), 3);
    }

    #[test]
    fn type_contract_tagged_union_header_parse_ok() {
        let src = r#"
entry[@] {
    type:
        Data tagged union {
            IntValue(value: int);
            TextValue(value: str);
            RawValue(bytes: memRegion<byte>);
            debug:
                printable;
        }

    fa:
        return true;
}
"#;
        let contract = parse_first_contract(src);
        assert_eq!(contract.kind, TypeContractKind::TaggedUnion);
        assert_eq!(contract.variants.len(), 3);
    }

    #[test]
    fn type_contract_generic_header_parse_ok() {
        let src = r#"
entry[@] {
    type:
        Box<T> {
            public:
                value: T;
            associated:
                make(value: T) -> Box<T>;
        }

    fa:
        return true;
}
"#;
        let contract = parse_first_contract(src);
        assert_eq!(contract.kind, TypeContractKind::Struct);
        assert_eq!(contract.generic_params, vec!["T".to_string()]);
    }
}

#[cfg(test)]
mod hosted_runtime_prelude_parser_tests {
    use ncc_ast::{Expr, Section, Stmt, TypeContractClause};

    #[test]
    fn hosted_prelude_calls_do_not_inject_visible_use_declarations() {
        let source = r#"
implicitRuntimeCalls {
    fn:
        run() -> bool {
            public:
                @root >> int : jsonParse("{}");
                @text >> str : strToUpper("nc");
                return @root > 0 && strEq(@text, "NC");
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
        let tokens = ncc_lexer::lex(source).expect("hosted prelude fixture must lex");
        let program = super::parse(tokens).expect("hosted prelude fixture must parse");
        assert!(
            program.uses.is_empty(),
            "the hosted runtime prelude is semantic/backend state, not source syntax"
        );
    }

    #[test]
    fn mobile_surface_accepts_constants_namespaces_var_and_single_statement_if() {
        let source = include_str!("../../../examples/12_apps/geocaller_mobile_backend.n");
        let tokens = ncc_lexer::lex(source).expect("mobile fixture must lex");
        let program = super::parse(tokens).expect("mobile fixture must parse");
        let container = &program.containers[0];

        assert!(container
            .sections
            .iter()
            .any(|section| { matches!(section, Section::ConstEval(items) if items.len() == 3) }));
        assert!(container.sections.iter().any(|section| match section {
            Section::Fn(functions) => functions.iter().any(|function| {
                function.name == "consultarUbicacion" && function.public_body.iter().any(|stmt| {
                    matches!(stmt, Stmt::Projection { value: Expr::QualifiedCall { path, .. }, .. }
                            if path == &["network".to_string(), "request".to_string()])
                })
            }),
            _ => false,
        }));
    }

    #[test]
    fn blockmaster_surface_accepts_global_structs_network_and_collections() {
        let source = include_str!("../../../examples/12_apps/blockmaster_remote_backend.n");
        let tokens = ncc_lexer::lex(source).expect("BlockMaster fixture must lex");
        let program = super::parse(tokens).expect("BlockMaster fixture must parse");
        let container = &program.containers[0];

        assert!(container.sections.iter().any(|section| {
            matches!(section, Section::TypeContracts(items) if items.iter().any(|item| item.name == "Bloqueo"))
        }));
        assert!(container
            .sections
            .iter()
            .any(|section| matches!(section, Section::ConstEval(items) if items.len() == 4)));
    }

    #[test]
    fn mobile_telephony_supplies_the_bloqueo_contract_when_source_omits_it() {
        let source = r#"
mobileRule :: runtime {
    stc:
        reglas >> Array<Bloqueo>;

    fn:
        public:
            run() -> bool {
                reglas.push(Bloqueo(numero: "+340", patron: "", origen: "PC"));
                return true;
            }
}
"#;
        let tokens = ncc_lexer::lex(source).expect("mobile type fixture must lex");
        let program = super::parse(tokens).expect("mobile type fixture must parse");
        let contracts = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::TypeContracts(contracts) => Some(contracts),
                _ => None,
            })
            .expect("telephony must provide a type contract section");
        let bloqueo = contracts
            .iter()
            .find(|contract| contract.name == "Bloqueo")
            .expect("telephony must provide Bloqueo");
        assert!(matches!(
            &bloqueo.clauses[0],
            TypeContractClause::Fields { fields, .. }
                if fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>()
                    == ["numero", "patron", "origen"]
        ));
    }
}

#[cfg(test)]
mod canonical_error_layout_parser_tests {
    use super::*;

    #[test]
    fn parses_union_explicit_layout_and_compile_time_queries() {
        let source = r#"
layoutParserFixture {
    type:
        Word union {
            public:
                low: dword;
                wide: qword;
            layout:
                repr(C);
                align(8);
                endian(little);
        }

    fn:
        run() -> bool {
            public:
                staticAssert(sizeOf<Word>() == 8);
                return offsetOf<Word>(wide) == 0;
        }

    fa:
        return true;
}
"#;
        let tokens = ncc_lexer::lex(source).expect("layout fixture must lex");
        let program = parse(tokens).expect("layout fixture must parse");
        let container = &program.containers[0];
        let contract = container
            .sections
            .iter()
            .find_map(|section| match section {
                Section::TypeContracts(contracts) => contracts.first(),
                _ => None,
            })
            .expect("union contract");
        assert_eq!(contract.kind, TypeContractKind::Union);
        assert!(contract.clauses.iter().any(|clause| matches!(
            clause,
            TypeContractClause::Layout(items)
                if items.iter().any(|item| item.contains("repr"))
        )));
    }

    #[test]
    fn parses_postfix_try_and_defer_as_typed_ast_nodes() {
        let source = r#"
errorParserFixture {
    fn:
        source() -> Result<int, str> {
            public:
                return ok(7);
        }

        relay() -> Result<int, str> {
            public:
                defer {
                    println("cleanup");
                }
                value >> int = source()?;
                return ok(value);
        }

    fa:
        return true;
}
"#;
        let tokens = ncc_lexer::lex(source).expect("error fixture must lex");
        let program = parse(tokens).expect("error fixture must parse");
        let functions = program.containers[0]
            .sections
            .iter()
            .find_map(|section| match section {
                Section::Fn(functions) => Some(functions),
                _ => None,
            })
            .expect("function section");
        let relay = functions
            .iter()
            .find(|function| function.name == "relay")
            .expect("relay function");
        assert!(matches!(relay.public_body[0], Stmt::Defer { .. }));
        assert!(matches!(
            &relay.public_body[1],
            Stmt::Projection {
                value: Expr::Unary {
                    op: UnaryOp::TryPropagate,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn bind_function_is_not_an_abi_binding() {
        for header in ["bind() -> int", "bind :: fn() -> int"] {
            let source = format!(
                "binding {{ {header} {{ return 7; }} run() -> int {{ return bind(); }} ref main ->> run; fa: return true; }}"
            );
            let program = parse(ncc_lexer::lex(&source).unwrap()).unwrap();
            assert!(program.containers[0].sections.iter().any(|section| matches!(
                section, Section::Fn(functions) if functions.iter().any(|function| function.name == "bind")
            )));
        }
    }
}
