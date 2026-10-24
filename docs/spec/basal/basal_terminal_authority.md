# Basal Terminal Authority

El IDE asiste. El terminal sentencia.

For Basal Fase 6, terminal authority includes `integrate-evidence`, `seal`, `verify-seal`, `evidence-report`, and `seal-report`.

Basal source may be edited from an IDE, and an IDE may call the CLI, but the CLI is the authority for check, tables, IR, lowering, NBB emission, NBB verification, reports, evidence, and seal phases.

Baseical commands:

```sh
ncc basal check file.nb
ncc basal tables file.nb --json
ncc basal ir file.nb --json
ncc basal verify-ir file.nb --json
ncc basal lower file.nb --json
ncc basal verify-lowering file.nb --json
ncc basal surface file.nb --json
ncc basal verify-surface file.nb --json
ncc basal emit-nbb file.nb --out file.nbb
ncc basal verify-nbb file.nbb --json
ncc basal report file.nb --json
```

Host-specific execution depends on the installed `ncc` binary, host permissions, target profile, and platform support. Windows, Linux, macOS, and WSL runs are CLI-governed.
