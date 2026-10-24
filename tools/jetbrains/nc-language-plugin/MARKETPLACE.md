# Marketplace publication guide — nC Language Plugin (JetBrains)

## Plugin name

nC Language Plugin

## Short description

Official JetBrains integration for nC with syntax support, palette-aligned highlighting, real nC logo icons, and routing to ncc/nclsp/nc-sys-lsp.

## Migration note

This plugin is migrated to IntelliJ Platform Gradle Plugin 2.x and configured for build line 261 with Java 21/Kotlin JVM 21.

## Version 1.0.0.21v

This release declares the professional IDE parity profile: live `ncc` diagnostics, semantic tokens, completion, hover documentation, definitions, references, document symbols, safe rename, quick fixes, run/debug actions, toolchain resolution, stable icons and lane-aware low-level routing comparable to mature C/C++/Rust/Java integrations.

## Visual identity

- pluginIcon.svg and pluginIcon_dark.svg are provided in META-INF.
- File type icons are separated by extension through `NcIcons.SourceFile`, `BasalFile`,
  `BasalBinaryFile`, `MachineFile` and `PrimitiveFile`.
- Baseical logo geometry is sourced from `site/public/assets/nc-logo.svg`.
- Palette pack aligned with `tools/theme/nc_colors.json` and `tools/theme/nc_visual_identity.json`.
- `.n`, `.nb`, `.nbb`, `.nm` and `.ncp` lanes are represented by stable color keys.
- Basal/root, primitive, machine, kernel, pointer, raw hazard and FFI lanes are represented by stable color keys.
- Text colors use the Java/Python/C/C++/Rust-inspired Polyglot Systems palette.
- Auto-import and pointer/FFI capabilities are tracked in `capabilities.contract.json`.
- Strong completion covers compact imports, imported runtime/heap helpers,
  struct fields, aggregate element fields, sibling project modules and
  entrypoint references.

## License

MIT OR Apache-2.0

## Publish note

Do not run JetBrains Marketplace publish flows unless explicitly authorized by the project owner.
