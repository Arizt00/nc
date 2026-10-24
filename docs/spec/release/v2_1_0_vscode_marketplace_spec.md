# nC v2.1.0 - VS Code Marketplace Preparation Spec

## Objective

Prepare `tools/vscode/nc-language` for Marketplace publication readiness without publishing.

## Required outputs

- `README.md` with clear plugin/nC explanation and boundaries.
- `MARKETPLACE.md` with short/long description and checklist.
- `CHANGELOG.md` with 2.1.0 notes.
- `package.json` metadata updated for 2.1.0.
- VSIX generation path `dist/nc-language-2.1.0.vsix`.

## Non-goals

- No publish command execution.
- No semantic authority in plugin.
- No claims that QEMU equals physical hardware.

## Authority model

- ncc is semantic source of truth.
- plugin surface is editor/tooling integration.

## License

SPDX-License-Identifier: MIT OR Apache-2.0
