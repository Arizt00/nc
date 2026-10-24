# Changelog

## 1.0.0.21v

- Established `1.0.0.21v` as the new JetBrains plugin version baseline.
- Added extension-aware machine and primitive highlighting.
- Added distinct output, pointer, color, control-flow and loop-control colors.
- Added context-bound colors for kernel `at` and each 8/16/32/64/128/256/512-bit MMIO family.
- Added tagless kernel detection from `name at address`, `*width(...)` and
  `kernel... { ... main: ... }`, while ignoring comments and strings.
- Made `p<T>` and compact pointer operations the official completion and inlay surface.
- Removed `print.*` members; output uses `print` and `println`.
- Aligned the bundled semantic catalog with the backend prelude and source-path-free hover contract.
- Preserved Run, Debug, navigation, completion, inlay hints and compiler diagnostics.

Future plugin releases continue from this baseline.
