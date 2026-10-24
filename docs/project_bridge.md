# Project bridge

Project bridge connects `.n` files without forcing every example to become a large monolith.

Public convention:

```n
use <project:modulo>;
```

The bridge metadata belongs in `bridge.json` for project-level graphs and `.ncc/code/*.json`
for per-file formalization. Public release packages hide transient `.ncc` cache content.
