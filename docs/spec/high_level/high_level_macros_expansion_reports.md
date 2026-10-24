# Macro expansion reports

Fase 14 exposes macro reports through:

```sh
ncc high-level macro-expand file.n --json
ncc high-level macro-report file.n --json
ncc high-level macro-hygiene file.n --json
```

Reports include:

- declarations.
- invocations.
- generated items.
- expansion status.
- source map status.
- hygiene status.
- unsafe effect audit entries.

The global manifest is available through:

```sh
ncc high-level macros --json
```

Report-only commands return JSON with `has_errors = false` for accepted files and include official diagnostics for rejected source when the caller uses `ncc check`.
