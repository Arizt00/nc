# nC v2.1.0 - Plugin Marketplace Description Spec

## Goal

Standardize Marketplace-facing descriptions across VS Code, JetBrains, and Ghidra plugin surfaces.

## Required statements

- What nC is.
- What each plugin is.
- What plugin does and does not do.
- ncc as source of truth.
- MIT OR Apache-2.0 licensing.
- nC Sovereign Dark mention.
- file extension support.
- no implicit publish behavior.

## Short description (official)

Official language support for nC: syntax highlighting, semantic tokens, snippets, nC Sovereign Dark theme, file icons, and integration with ncc, nclsp and nc-sys-lsp.

## Long description baseline

nC Language is the official Visual Studio Code integration for nC, a contract-driven programming language and toolchain designed around explicit computational layers, modes and semantic boundaries.

nC is not just another high-level syntax. It aims to make runtime services, compiler behavior, system interaction, inspection, kernel boundaries, machine-level execution, primitive hardware semantics and basal/root concepts visible through language contracts.

This extension provides syntax highlighting, semantic token support, snippets, official nC file icons, the nC Sovereign Dark theme and integration points for ncc, nclsp and nc-sys-lsp workflows.

The extension does not replace ncc. Semantic validation, diagnostics and language authority remain in the ncc toolchain. The plugin presents and routes nC tooling; it does not invent language semantics.

Supported files:
.n, .nb, .nbb, .nm, .ncp and .nc.

License:
MIT OR Apache-2.0.
