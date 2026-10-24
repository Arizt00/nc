# stoud Imports

`stoud` is the standard typed facade. Its canonical external families are:

```n
use stoud.collections { list, queue, result, vector };
use stoud.fs { path, read, write };
use stoud.io { output, render, input };
use stoud.math { core, numeric, trig };
use stoud.memory { ptr, heap, region };
use stoud.net { dns, http, tcp, tls, udp };
use stoud.process { kill, spawn, wait };
use stoud.thread { join, sleep, spawn };
use stoud.time { clock, timer };
```

The official family order is `io`, `memory`, `collections`, `math`, `time`,
`fs`, `net`, `process`, `thread`. Editor tooling and generated documentation
read the external catalog at `tools/vscode/nc-language/data/stoud_stdlib_catalog.json`.
Each catalog function records parameters, return value, possible errors and
whether it lowers to backend/runtime bytes or to a native compiler intrinsic.

`stoud` does not contain authority-bearing `system`. A request such as:

```n
use stoud.system { fs };
```

fails with `E_USE_STOUD_AUTH_REQUIRED_001` and must be expressed as `auth::system` with explicit policy.

Filesystem imports are no longer owned by `io`. The canonical spelling is:

```n
use stoud.fs { append, path, read, sync, write };
```

Concrete file operations are written as calls in the program body, similar to C/C++ after including the relevant header:

```n
writeFile("generadorTexto.txt", texto);
readFile("generadorTexto.txt");
```

Legacy `use::stoud::io::<fs, fsread, fswrite, fsappend, fsync>` remains
accepted as a compatibility alias, but new examples and formatter output use
`stoud::fs`.

Process and thread imports are also normal external standard libraries:

```n
use stoud.process { spawn, wait };
use stoud.thread { join, spawn };
```

They expand to typed runtime families such as `nC.runtime.process` and
`nC.runtime.thread`. Calls keep the same backend contract as direct runtime
imports: names, ABI, symbols and lowering metadata are retained in the import
manifest.

## Pointer/region imports

Raw pointer and heap operations are native-prelude/backend families and need no import. The compatibility facade for pointer-region roundtrips is:

```n
use stoud.collections { result };
use stoud.memory { ptr, heap, region };
```

It expands to:

```n
use stoud.collections { result };
use stoud.memory { ptr, heap, region };
```

`memory` does not hide pointer hazards. Raw operations permit null, aliasing,
misalignment, out-of-range access, UAF and double free as undefined behavior.
Auditors are optional and never change generated code. The program may still
document a risk as evidence:

```n
risk:
    accept heapRegionView "regionFromPtr opens a typed heap view";
```

The `accept heapRegionView` entry is optional evidence. Type mismatches remain
diagnosable; lifetime, generation and bounds auditing is opt-in.
