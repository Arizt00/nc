# nC v2.1.0 — Visual Identity Integration Spec

## Scope

Integrate real nC logo assets and official palette pack across:

- VS Code plugin
- JetBrains plugin
- Ghidra extension
- Theme source-of-truth files
- Marketplace/docs references

## Baseical logo chain

- `tools/assets/logo/nc-logo-source.png`
- `tools/assets/logo/nc-logo-source.svg`

Derived assets:

- VS Code: `assets/nc-logo-128.png`, `assets/nc-logo-256.png`, `assets/nc-logo.svg`
- JetBrains: `META-INF/pluginIcon.svg`, `META-INF/pluginIcon_dark.svg`
- Ghidra: `data/nc-logo.png`, `data/nc-logo.svg`
- Docs: `docs/assets/nc-logo.png`, `docs/assets/nc-logo.svg`

Rule: `build/` is output only; never source asset.

## Official palette pack

- nC Sovereign Dark
- nC Dust Rose
- nC Aqua Runtime
- nC Crimson Diagnostics
- nC Systems Spectrum

Source files:

- `tools/theme/nc_colors.json`
- `tools/theme/nc_colors.md`
- `tools/theme/nc_semantic_tokens.json`
- `tools/theme/nc_visual_identity.json`

## JetBrains migration contract

- `org.jetbrains.intellij.platform` 2.x
- Java toolchain 21
- Kotlin JVM target 21
- sinceBuild 242
- untilBuild 261.*
- RustRover dependency path configured

## Marketplace constraints

- No auto-publish commands executed.
- Metadata prepared only.
- License remains `MIT OR Apache-2.0`.
