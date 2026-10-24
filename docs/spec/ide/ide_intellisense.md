# nC IDE IntelliSense

Status: closed for editor assistance.

This document defines the nC IDE IntelliSense surface for VS Code and JetBrains IDEs.
The editor assists, predicts and explains. The compiler remains authoritative.

## Authority

- `ncc` owns parsing, semantic validation, diagnostics, lowering and evidence.
- IDE plugins may suggest imports, complete symbols, show documentation and navigate to local or spec definitions.
- IDE plugins must not silently accept invalid code or replace compiler diagnostics.
- IDE Run entries are launch wrappers only; see `docs/spec/ide/ide_run_configurations.md` for the `ncc ide init` / `nide init` contract.

## Shared Catalog

Both IDE integrations consume `nc_intellisense_catalog.json`.

The catalog contains:

- symbols such as `readInt`, `println`, `regionFromPtr`, `ptrFromAddr`, `parse` and `fs`;
- types such as `addr`, `ptr<T>`, `ref<T>`, `site<T>`, `memRegion<T>`, `slice<T>` and `span<T>`;
- diagnostics such as `E_PTR_NULL_DEREF_001` and mode/import boundary diagnostics;
- preferred facade imports such as `use stoud.io { input };`;
- legacy official imports such as `use stoud.io { input };`;
- risk metadata and links to docs.

## VS Code

VS Code provides:

- completion items for functions, types, diagnostics and main-family snippets;
- auto-import text edits using preferred compact standard imports and explicit `auth/dom` imports;
- missing-import quick fixes;
- hover explanations for symbols, pointer types, diagnostics and runtime families;
- go-to definition for local declarations and catalog docs;
- find references and safe rename for local symbols;
- semantic tokens and color lanes for `.n`, `.nb`, `.nbb`, `.nm` and `.ncp`.

## JetBrains

JetBrains provides:

- completion items from the same catalog;
- auto-import insertion for `.n` and `.nc` source files;
- documentation hover/quick documentation;
- go-to declaration for local declarations and catalog docs;
- weak-warning quick fixes for missing imports;
- existing syntax highlighting, file type recognition and icon lanes.

## Auto-Import Base

Preferred imports use the new import surface:

```n
use stoud.fs { read, write };
use stoud.io { output, render, input };
use stoud.memory { ptr, region, buffer };
use auth.system { permission, process };
use dom.compiler { diagnostics, parse, sema };
```

Legacy imports remain recognized:

```n
use stoud.io { output, input };
use stoud.memory { region };
```

The plugin suppresses duplicate import hints when either the facade import or the legacy import already exists.

Completion, navigation, inlay hints and formatting emit `name :: fn`,
`Name :: type`, `name >> Type <- value` and `ref main ->> name;`. Legacy forms
are recognized for migration but are never the first completion.

## Boundaries

The plugin does not add imports to `.nb`, `.nbb`, `.nm` or `.ncp` as an authority shortcut.
Basal, machine and primitive keep their own bases. Editor completion may explain known terms, but import quick fixes are limited to high-level/source files.

## Validation

The release smoke for this surface must validate:

- catalog JSON parsing in both plugin folders;
- VS Code smoke tests;
- JetBrains Kotlin compilation;
- no non-authoritative editor prediction presented as compiler truth.
