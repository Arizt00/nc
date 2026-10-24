# nC Import Surface Overview

Status: canonical compact facade closed; legacy qualified forms are input-only.

The legacy official form remains:

```n
use stoud.io { output, render };
```

The official standard/project facade form is:

```n
use stoud.project { biblioteca, usuarios };
use stoud.io { output, render, input };
```

Authority and compiler-domain imports retain explicit qualification because
they are not standard-library selection:

```n
use auth.system { permission, process };
use dom.compiler { diagnostics, parse, sema };
```

`stoud` means **Standard Typed Operational Use Domain**. In Spanish: **Dominio Estandar Tipado de Uso Operacional**.

Rules:

- `stoud` is standard, typed and operational; raw memory APIs do not promise safety.
- `auth` carries authority, but normal `.n` imports no longer require an explicit `risk:`, `policy:` or `authority:` block.
- `dom` is for specialized compiler/high/inspect/ai/tooling domains.
- `use nC.runtime.*` remains the internal truth of resolution.
- `[@machine]`, `primitive[@]`, basal `.nb`, and future `[@kernel]` do not use imports.
- Wildcards are denied in canonical standard, `auth` and `dom` imports.

## Backend/MCT Lowering

The import facade is now connected to backend/MCT activation lowering:

- `ncc imports backend --json` lists every supported import item grouped by
  domain and family.
- `ncc imports backend <file.n> --json` filters that manifest to the imports
  expanded from a concrete source file.
- `ncc mct imports --json` exposes the same manifest from Modal Container
  Translate.
- Every supported item receives a deterministic backend symbol and concrete
  activation bytes for `nC.sysv64`, `nC.win64`, `nC.aapcs64` and `nC.portable`.
- Runtime-backed imports are marked `closed_real_backing_bytes` and expose their
  concrete `runtime_symbols`, for example `nC.runtime.process.spawn`,
  `nC.runtime.thread.threadJoin`, `nC.runtime.tcp.tcpRecv` or
  `nC.runtime.fs.fsync`.
- `ncc run archivo.n` uses those runtime APIs directly; no `--real` flag is
  needed for the closed runtime surface.
- `auth` imports lower to authority gates; they do not fake filesystem,
  process or security side effects.
