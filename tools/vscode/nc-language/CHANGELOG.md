# Changelog

## 1.0.0.21v

- Established `1.0.0.21v` as the new plugin version baseline (`1.0.0-21v` in VS Code SemVer).
- Added extension-aware semantic coloring so machine and primitive vocabulary is active only in its own file lane.
- Added distinct output, pointer, color, control-flow and loop-control token families.
- Added context-bound colors for kernel `at` and each 8/16/32/64/128/256/512-bit MMIO family.
- Added tagless kernel detection from `name at address`, `*width(...)` and
  `kernel... { ... main: ... }`, while ignoring comments and strings.
- Made `p<T>` and the `pRead`/`pWrite` family the official compact editor surface while retaining verbose aliases as compatibility input.
- Removed `print.*` members from completion and highlighting; source output is `print` or `println`.
- Treated terminal, String/Char, time, `exists` and `listDir` as backend prelude APIs without import suggestions.
- Kept hover definitions self-contained and capped at 50 words without repository paths.
- Preserved Run, Debug, diagnostics, navigation, inlay hints, project graphs and `nfmt --compact` integration.
- Made `<-` the material-write color and marks removed `:=` as an editor error.
- Added dedicated visibility colors for `public`, `internal` and `private`, matching their object/LLVM linkage contracts.
- Updated snippets and completion to grouped `fn:` declarations, `name >> type` parameters and compact type constructors.

Future plugin releases continue from this baseline.
