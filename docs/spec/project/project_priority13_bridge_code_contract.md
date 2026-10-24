# Priority 13.A Project Bridge and Code Identity Contract

Priority 13.A closes the first material layer for normal multi-file `.n` projects.
It does not close semantic cross-file linking; that is Priority 13.B.

Baseical project shape:

```text
project/
├── main.n
├── bridge.json
├── src/
│   ├── memoria.n
│   ├── scheduler.n
│   └── drivers.n
└── .ncc/
    ├── code/
    │   ├── memoria.json
    │   ├── scheduler.json
    │   └── drivers.json
    ├── lowering/
    │   ├── memoria.lowering.json
    │   └── scheduler.lowering.json
    └── obj/
        └── l64/
            ├── memoria.o
            ├── memoria.o.json
            ├── scheduler.o
            └── scheduler.o.json
```

`main.n` is generated automatically and may be empty. The default name is
`main.n`, but `ncc project init --main <name.n>` may choose another mother file.

`bridge.json` is the global graph between mother files and child files. It is
regenerable and records the visible project graph through `units` and `edges`.

`.ncc/code/<unit>.json` is intentionally tiny. It is not a backend dump and must
not expose `native_fast`, `llvm`, `machine`, `basal` or MCT internals. Its public
contract is identity only:

```json
{
  "schema": "ncc.code.v1",
  "unit": "memoria",
  "source": "src/memoria.n",
  "hash": "sha256:...",
  "bridge": "../../bridge.json"
}
```

`code.json` formalizes the source identity between the `.n` world and compiler
materialization. The `.n` file remains the source of truth and still lowers
through parser, AST, sema, IR, MCT and backend bytes.

`use stoud.project { memoria };` is the official compact import for project graph
edges.

Priority 13.D applies the same tiny identity contract to the mother file:
`main.n` gets `.ncc/code/main.json`, and `bridge.json` records that identity
with the main unit. After `ncc project sync`, editing `main.n` without syncing
again is rejected before parser/linker execution because the source bytes no
longer match the stored hash.

Priority 13.B adds the real compiler path:

- `ncc project check <dir>` loads `bridge.json`, verifies every `.ncc/code/*.json`
  hash against the current source bytes, parses every unit, links an in-memory
  project program, runs sema, lowers IR, emits backend IR and writes
  `.ncc/lowering/*.lowering.json`.
- `ncc project emit-ir <dir>` returns the same linked IR materialization as JSON.
- `ncc project run <dir>` executes the mother `main.n` against the linked AST.
- `ncc run <dir>/main.n` auto-detects the adjacent/ancestor `bridge.json` when
  that file is the bridge `main`, then links and validates the project through
  the portable runtime path. This route intentionally does not require every
  function to have native object bytes, so rich runtime IO projects can run
  while the physical backend remains byte-strict.
- child `to main`/`ref main` sections are compiled as source but stripped from
  the linked entrypoint set, so only the mother file owns the program entry.

The runtime project path is incremental. Every unit has an AST cache under
`.ncc/cache/ast/<unit>-<source-hash-prefix>.ast.json`. The cache is accepted
only when the current source bytes hash to the same `sha256:` value recorded by
`bridge.json` and `.ncc/code/<unit>.json`; corrupt or stale cache files are
ignored and regenerated. This means a folder with many `.n` files still reads
source bytes for identity, but avoids lexing/parsing clean units repeatedly.

`ncc project incremental <dir>` now tracks five material states:

- `source_status`: the `.n` bytes versus the bridge hash;
- `code_status`: the tiny `.ncc/code/*.json` identity;
- `lowering_status`: the unit lowering JSON identity;
- `linked_lowering_status`: the linked lowering JSON identity;
- `object_status`: the physical `.o/.obj` bytes versus `object_hash`.

The persisted `.ncc/incremental/status.json` is valid only when its fingerprint
matches the current target, unit hashes, code/lowering/object states, bridge
edges and link order. A dirty child propagates to every parent that depends on
it, and missing linked lowering marks the full linked program stale.

`ncc run <file.n>` records the selected route in `.ncc/run/last.json`. With
`--show-return`, the console return line also includes `profile=...` and
`route=...`. The default route is now `auto`: backend-lowerable single-file
programs, including implicit single-file workspaces, use `native-fast-path`
without requiring `--real`. The auto route is deliberately conservative: project
imports, `to main` entrypoints, `input`, FS/TCP/process/thread/time/random
runtime surfaces and other hosted-runtime markers stay on the interpreter unless
the user explicitly asks for a backend profile. Project bridge runs still report
`route=project-bridge-interpreter`; forced or auto native executions report the
native backend route such as `native-fast-path` or `llvm-mct-path`.

The `<file.n>` argument is resolved through the universal source resolver before
the compiler reads bytes. It accepts relative paths, absolute paths, paths with
spaces, Windows drive paths such as `C:\...\main.n` from WSL/Kali/Ubuntu, and
`/mnt/c/...` paths from Windows shims. If the argument is only a file name, the
resolver searches the current directory, `src/` and any directories listed in
`NCC_RUN_PATH`; missing or ambiguous matches fail with `E_NCC_PATH_001` or
`E_NCC_PATH_002` before parser execution.

Project materialization is protected by a cache lock so IDE runs, terminal runs
and background checks can target the same project without corrupting
`.ncc/code`, `.ncc/lowering`, `.ncc/obj`, `.ncc/cache` or `.ncc/incremental`.
The lock is an atomic directory:

```text
.ncc/locks/project-cache.lock
```

While held, ncc writes:

```text
.ncc/locks/project-cache.lock/owner.json
.ncc/locks/last_report.json
```

All artifact writes use a temporary file in the same directory followed by a
rename, so a reader never observes half-written JSON/object bytes. Abandoned
locks are treated as stale after 600 seconds. The wait timeout defaults to
30000 ms and can be overridden with `NCC_CACHE_LOCK_TIMEOUT_MS`. If the timeout
expires, ncc emits `E_PROJECT_CACHE_LOCK_001` with the concrete lock path and
the operation that was waiting.

`nide init` generated Run configurations execute the active file through the
terminal, keep stdin open, respect `$PROJECT_DIR$` as cwd, use `cmd.exe /C` on
Windows-style projects and `sh` on Unix-style projects, and pass
`--show-return` so the IDE visibly reports the nC route used by the button.

`.ncc/lowering/*.lowering.json` may be large. It is internal compiler material
and records source bytes, UTF-8 character bytes, SHA-256 digest bytes, IR bytes
and backend debug bytes. Unlike `.ncc/code/*.json`, lowering JSON is not a public
identity file and may change as backend/MCT representation evolves.

13.B intentionally permits project graph cycles at explicit `bridge.json` level.
Repeated container names are accepted as independent unit material during
`project check`.

Implicit single-file workspaces are stricter because there is no checked
`bridge.json` authority. `use <project:hijo>` searches the mother file's
folder, `src/` and every directory in `NCC_PROJECT_PATH`. Every discovered child
gets `.ncc/code/<unit>.json` and `.ncc/cache/ast/<unit>-<hash>.ast.json`; files
outside the mother root keep an absolute normalized source path in code identity.
Import cycles fail with `E_SINGLE_FILE_IMPORT_CYCLE_001` and print the chain
(`a -> b -> a`). Ambiguous child names fail with `E_SINGLE_FILE_IMPORT_002` and
list all candidate paths so the programmer can rename, move, or narrow
`NCC_PROJECT_PATH`.

Priority 13.C adds explicit unit-qualified resolution for duplicate containers.
When two child files export the same container name, the mother file may select
the exact unit with:

```n
use stoud.project { a, b };

mainApp {
    ref main ->> a.sameName.boot;

    fa:
        return true;
}
```

The linker materializes duplicate child containers internally as
`<unit>__<container>` and rewrites `a.sameName.boot` to that backend-visible
container path before sema, IR and backend lowering. The syntax remains compact
at source level, but the linked program is no longer ambiguous.

This resolver also applies to qualified calls and context paths shaped as
`unit.container.member`. `bridge.json` and `.ncc/code/*.json` remain identity
files; the materialized names live in the linked AST/lowering path, not in the
public code identity contract.

Priority 13.E tightens diagnostics:

- `ncc project sync` rejects `use <project:unit>` if `unit` is not a known
  source unit in the project graph, using `E_PROJECT_BRIDGE_EDGE_003`.
- `ncc project check/run` rejects `unit.container.member` when `unit` exists but
  `container` is not declared inside that unit, using
  `E_PROJECT_UNIT_QUALIFIED_001`.

This keeps project failure modes at the bridge/linker layer instead of allowing
broken graphs to fall through into generic sema or runtime errors.

Priority 13.F strengthens `ncc project doctor`. The command now reports bridge
units, edges, code identity files and `identity_errors` without running the
program. A stale `main.n` or child source produces `status: "error"` with the
same identity diagnostic used by `project check`, while a healthy project keeps
`status: "ok"`.

Priority 13.G makes linker materialization auditable. The linked lowering file
`.ncc/lowering/__linked.lowering.json` includes:

- `project_linker.status = "closed_unit_namespace_materialization"`;
- `materialized_containers[]` entries with unit name, source container,
  materialized container, duplicate flag and byte-level hex for each visible
  linker string;
- `unit_qualified_prefix` so `a.sameName` can be traced to the backend-visible
  materialized name such as `a__sameName`.

This gives the backend/MCT side an explicit byte contract for project namespaces
instead of hiding the transformation inside debug output.

Priority 13.H adds `ncc project graph <dir>`. It compiles and links the project
without running `main`, writes the same lowering material as `check`, and emits a
graph report containing units, edges, linked container count and the
`project_linker.materialized_containers` table. With `--json`, this is the
official machine-readable way to inspect how a multi-file project will be
resolved before execution.

Priority 13.2.A adds real per-unit object artifacts. `ncc project check`,
`ncc project graph`, `ncc project emit-ir` and `ncc project run` write one
backend object for every bridge unit under
`.ncc/obj/<target-id>/<unit>.o` (`.obj` for COFF targets) plus a sidecar
`.json` next to it. The physical target id is compact and collision-free for
the declared matrix: `w64`, `l64`, `mx64`, `ma64`, `l32`, `la64`, `armv7`,
`rv32`, `rv64` and `wasm32`. The complete backend triple remains in the object
sidecar and `bridge.json`; it is not inferred from the directory name. The
object file is the backend
emitter's native bytes for the materialized unit AST: unit-qualified paths are
rewritten, duplicate child containers use the same `<unit>__<container>` names
as linked lowering, and child `main` entries are stripped before object
emission.

The sidecar schema is `ncc.project.object.v1` and records:

- ABI: target triple, ABI model, object format, relocation model, pointer width
  and runtime/linker model;
- hashes: source hash and SHA-256 hash of the emitted object bytes;
- bytes: object byte count and `object_bytes_hex`;
- symbols: exported backend symbols plus the source-visible bridge symbols;
- imports: project unit imports, source `use` paths and unresolved backend
  symbols.

`sync` may create the same contract early so `doctor` can validate a physical
bridge immediately. For empty sources this is a contract object only; for
non-empty sources and all compile actions the `.o/.obj` path contains backend
object bytes and the JSON sidecar is the stable inspection surface.

Priority 13.2.B makes `bridge.json` the physical project linker manifest. The
schema remains `ncc.bridge.v1`, but each unit entry now carries linker material:

```json
{
  "name": "memoria",
  "source": "src/memoria.n",
  "code": ".ncc/code/memoria.json",
  "source_hash": "sha256:...",
  "object": ".ncc/obj/l64/memoria.o",
  "object_hash": "sha256:...",
  "symbols": [
    {
      "name": "memoriaUnit",
      "kind": "container",
      "container": "memoriaUnit",
      "visibility": "unspecified"
    }
  ],
  "imports": [],
  "deps": []
}
```

On COFF targets the object extension is `.obj`; ELF64 and Mach-O targets use
`.o`. The object path must be relative, must stay under `.ncc/obj/<target>/`,
and must hash to the recorded `object_hash`. `symbols` is the source-visible
linker symbol table extracted from the AST: containers, functions, types,
enums, traits, contracts, foreign symbols and main entries. `imports` records
official `use <project:...>` targets found in source, while `deps` is the validated
dependency set derived from bridge edges. For this phase they must match.

The top-level bridge also records:

```json
{
  "linker": "ncc.project.physical_linker.v1",
  "object_dir": ".ncc/obj",
  "target": {
    "target_triple": "x86_64-unknown-linux-gnu",
    "object_format": "elf64",
    "abi": "sysv64"
  },
  "link_order": ["memoria", "scheduler", "main"]
}
```

`link_order` is dependency-first and cycle-tolerant: cycles remain legal at
bridge level, but every known unit must appear exactly once. `ncc project check`,
`emit-ir`, `graph` and `run` load units through this order and reject unknown,
duplicate or incomplete link entries before parser/sema work proceeds.

`ncc project doctor --json` validates this physical bridge without running the
program. Missing objects, object hash mismatches, unsafe object paths, invalid
`imports`/`deps` or broken `link_order` produce `status: "error"` with
`bridge_errors`; stale source or stale `.ncc/code/*.json` identity continues to
report through `identity_errors`.

Standalone byte-strict closure extends the hosted standalone path for normal interactive
projects on the native x86_64 object route. High-level `if`/`while` over scalar
state, including `stc` globals, lowers to native branches, global load/store
relocations and real object bytes. Compact `str` values in this route use the
hosted C-string ABI for literals, parameters and local `-> str` returns, so menu
titles and generated catalogue names can cross functions without falling back to
the interpreter.

Executable object sections are now byte-strict: `.text.*` and machine sections
must carry native `bytes`. The ELF/COFF/Mach-O writers reject executable
sections that only contain backend text lines, instead of materializing textual
IR as accidental machine code. This keeps `ncc project link` honest: a linked
standalone either contains real executable bytes for every function it reaches,
or fails before producing a misleading binary.

`.ncc/code/*.json` is intentionally unchanged by 13.2.B. It remains the tiny
source identity file and does not absorb object bytes, backend internals or
linker symbol tables.

Priority 13.2.B-portability adds explicit backend target selection for projects.
The default profile follows the host OS/arch, and the same source graph can be
checked or materialized for another OS with either official nC profiles or
target triples:

```bash
ncc project targets
ncc project check . --cc nC.win64
ncc project check . --cc nC.aapcs64
ncc project check . --target aarch64-unknown-linux-gnu
```

`ncc project targets --json` is the machine-readable matrix. The current closed
object profiles are:

- `nC.sysv64`: `x86_64-unknown-linux-gnu`, ELF64, SysV64;
- `nC.win64`: `x86_64-pc-windows-msvc`, COFF, Win64;
- `nC.aapcs64`: `aarch64-apple-darwin`, Mach-O, AAPCS64;
- `nC.macos64`: `x86_64-apple-darwin`, Mach-O, SysV64;
- `nC.aarch64-linux`: `aarch64-unknown-linux-gnu`, ELF64, AAPCS64.

`ncc run main.n` remains the portable runtime path and does not require a
target flag. Cross-target `project check/graph/emit-ir/run` materializes object
bytes and bridge metadata for that target, and therefore rejects functions whose
backend route has not produced native bytes yet. Producing and executing a final
standalone binary for a non-host OS still requires a native or cross linker for
that OS; the bridge records this as `link_support =
"requires-native-or-cross-linker"` instead of pretending that Windows can run a
Mach-O executable or Linux can run a PE/COFF executable directly.

Priority 13 IDE/build integration adds two read-oriented commands over the same
physical bridge:

```bash
ncc project model . --json
ncc project incremental . --json
```

`project model` emits `ncc.project.model.v1`. It is the stable Project Model
for IDEs and LSP servers: root, main file, target profile, units, edges,
dependency-first `link_order`, `.ncc/code` identity status, object status,
source hashes, exported symbols and diagnostics. It does not run user code and
does not lower the linked program.

`project incremental` emits `ncc.project.incremental.v1` and also writes:

```text
.ncc/incremental/status.json
```

The incremental identity is deliberately material: source hash, code identity,
object hash and bridge edges. A changed child marks that unit dirty and then
propagates `dependency_changed=<unit>` to every dependent unit. The command
reports `rebuild_units` in bridge `link_order` and `cached_units` for units
whose bytes are still reusable. When the graph is clean and bridge/object
identity sidecars validate, `project check` returns `ncc.project.cached_check.v1`
and reports `semantic_validation=skipped_linked_clean_cache` instead of
reanalyzing the linked program. `project run` uses the same clean-cache signal
to skip linked IR/backend semantic revalidation while still linking the AST
needed for execution. Dirty roots or dependency propagation force the full
linked validation path again.

## Single-file workspace mode

Normal `.n` and `.nc` files do not need `ncc project init` to be runnable from
an IDE. When `ncc check file.n` or `ncc run file.n` sees no enclosing
`bridge.json`, the CLI creates an implicit single-file workspace rooted at the
file directory. It writes compact identity files under `.ncc/code/<unit>.json`
and runtime lowering files under `.ncc/lowering/`, but it does not create a
visible `bridge.json` or require a generated `main.n`.

`use stoud.project { modulo };` works in this mode as a C-like sibling import. The
resolver searches by unit name in the file directory and recursively under
`src/`, skipping `.ncc`, `target` and `node_modules`. Imported children can
import other children the same way. Cycles are accepted at graph level, and
unit-qualified calls such as `modulo.contenedor.funcion()` are materialized by
the same project linker rewrite used by formal projects.

Ambiguous or missing sibling units are hard diagnostics:

- `E_SINGLE_FILE_IMPORT_001` for missing or invalid import unit names.
- `E_SINGLE_FILE_IMPORT_002` for multiple candidate files with the same unit
  name.
- `E_SINGLE_FILE_IMPORT_005` when a project import resolves to a non-normal
  source surface such as `[@machine]` or `primitive[@]`.

This mode is meant for editor Run buttons and small C-style file groups. A
formal `bridge.json` still wins when present, so larger packages keep the full
project model, physical linker manifest and object sidecars.

Priority 13.2.C adds `ncc project link <dir>`. This command compiles the same
physical bridge used by `check`/`graph`, rewrites `bridge.json` with current
object hashes, builds the linked MCT graph and serializes the final image with
the internal target writer. It does not invoke a C compiler or external linker.

The default output is:

```text
.ncc/bin/<main-stem>
```

or `.ncc/bin/<main-stem>.exe` for COFF/Windows targets and
`.ncc/bin/<main-stem>.wasm` for WASM. `--out <path>` overrides that path. The
selected target and its complete ABI remain explicit in `bridge.json` and the
final-image record.

Because nC object symbols are named as nC material symbols such as
`mainObj.entry`, the linked project retains its hosted entry in the LTO MCT
artifact and materializes it directly in the final image:

```text
.ncc/incremental/m/<target-id>/lto.ncmct
.ncc/obj/<target-id>/lto.<o|obj|wasm>
```

The final image writer selects the hosted entry, applies local and runtime
relocations and converts the nC return contract into an operating-system
process exit code:

- `bool`: `true -> 0`, `false -> 1`;
- `int`/integer/char scalar: returned directly as process exit code;
- `void`: returns `0`.

The final image entry in `bridge.json` uses
`nc.bootstrap.project.final-image.v1` and records target, format, bytes,
SHA-256, runtime provider count and whether the image is self-contained. At
this stage `project link` requires
`ref main ->> ...;` or `skkMain` so the physical linker can bind a concrete
object symbol. Inline `to main` remains supported by `ncc project run` and is
rejected by `project link` with `E_PROJECT_LINK_002`.

`project link` no longer emits, compiles or retains C source. MCT builds the
final relocation plan and the internal PE/ELF/Mach-O/Wasm writer serializes the
operating-system image. Hosted calls remain canonical imports named
`nC.runtime.<family>.<function>`; an image with providers is executable only
when the corresponding nC runtime provider binary is installed next to it or
available through the system loader.
