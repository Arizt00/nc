# auth Imports

`auth` imports authority/capability surfaces:

```n
use <auth.resource:handle, resource, session>;
use <auth.security:audit, capability, policy, sandbox>;
use <auth.system:dir, env, file, os, path, permission, process, service, session, signal, thread>;
```

Rules:

- `auth` no longer requires an explicit `risk:`, `policy:` or `authority:` block in normal `.n` code.
- Authority lowers through LLVM/MCT/Machine without an implicit permission gate.
- `auth` is forbidden in machine, primitive, basal and kernel modes.
- `auth.system:file` is a caller-owned table of file authority records. It does
  not open, read, write, close or own files.
- `auth.system:dir` is a caller-owned table of directory authority records. It
  does not create, enumerate, remove or change directories.
- `auth.system:path` stores borrowed path bytes and numeric authority metadata.
  It does not own, normalize, canonicalize or inspect the path.
- `auth.system:env` stores borrowed value bytes and numeric authority metadata.
  It does not read or mutate the process environment.
- `auth.system:permission` stores caller-defined grant and deny masks. It does
  not request, inspect or mutate operating-system permissions.
- `auth.system:service`, `signal` and `session` store authority records only.
  Their hosted operations remain explicit under the `process` families.
- `auth.security:policy`, `capability` and `sandbox` are caller-owned metadata
  tables. They never impose safety or operating-system isolation implicitly.
- File I/O belongs to `stoud::fs`; operations such as `writeFile` and `readFile`
  remain explicit calls made by the program.
