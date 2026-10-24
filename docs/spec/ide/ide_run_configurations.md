# nC IDE Run Configurations

Status: active tooling contract.

This document defines the practical IDE Run surface for launching `ncc run`
without manually typing a terminal command.

## Authority

- `ncc run <file.n>` remains the execution authority.
- IDE files only delegate to `ncc`; they do not replace parser, sema, IR,
  runtime or backend behavior.
- Generated files are project-local and safe to regenerate with `--force`.
- Active-file paths are passed to `ncc run` as-is. The CLI universal source
  resolver officializes PowerShell/CMD paths, WSL `/mnt/c/...` paths, paths
  with spaces and bare file names before the IDE wrapper reads source bytes.

## CLI

Baseical commands:

```bash
ncc ide init
ncc ide init --ide vscode
ncc ide init --ide jetbrains
ncc ide init --ide jetbrains --file examples/helloWorld.n
ncc ide init --profile native-fast --target linux-x64,windows-x64 --toolchain auto,stable
```

Equivalent direct tooling command:

```bash
nide init
```

Options:

- `--ide <vscode|jetbrains|all>` selects generated targets. Default: `all`.
- `--project <dir>` selects the project root. Default: current directory.
- `--file <file.n|file.nb|file.nbb>` adds a configured-file Run entry in addition to active-file
  entries.
- `--profile <auto|interpreted|native-fast|native-ffi|debug|release|release-fast|release-small|unsafe-fast|instrumented>[,...]`
  generates selectable execution profiles. Default: `auto`.
- `--target <host|x86_64-unknown-linux-gnu|x86_64-pc-windows-msvc|x86_64-pc-windows-gnu|x86_64-apple-darwin|aarch64-apple-darwin|aarch64-unknown-linux-gnu|wasm32-wasi|x86_64-unknown-sovereign-nc>[,...]`
  generates target-aware Run entries. Aliases such as `linux-x64`,
  `windows-x64`, `macos-arm64` and `linux-arm64` are normalized to official
  triples. Default: `host`.
- `--toolchain <auto|ncc|stable|beta|nightly|toolchain-id|path>[,...]` chooses
  how the generated script resolves the `ncc` executable. `auto` uses `ncc`
  from PATH, a file path is executed directly, and channel/toolchain ids are
  resolved through `ncup which ncc --toolchain <id>`. Default: `auto`.
- `--force` overwrites existing generated IDE files.
- `--dry-run` prints planned artifacts without writing.
- `--json` emits a machine-readable `nide.ide.init.v1` report.

The JSON report includes `profiles`, `targets` and `toolchains` so IDEs and
tests can verify the generated matrix without scraping XML or shell text.

## VS Code

Generated file:

- `.vscode/tasks.json`
- `.ncc/ide/nc-run.<cmd|sh>`
- `.ncc/ide/nc-check.<cmd|sh>`
- `.ncc/ide/nc-format.<cmd|sh>`

Generated tasks:

- `nC: Run Active File` -> `.ncc/ide/nc-run.<cmd|sh> ${file} auto host auto`
- `nC: Check Active File` -> `.ncc/ide/nc-check.<cmd|sh> ${file} auto host auto`
- `nC: Format Active File` -> `.ncc/ide/nc-format.<cmd|sh> ${file} auto host auto`
- `nC: Run Active File (Profile/Target)` appears when a non-default profile,
  target or toolchain matrix is requested. It exposes VS Code inputs
  `ncProfile`, `ncTarget` and `ncToolchain`.
- `nC: Run Configured File` and `nC: Run Configured File (Profile/Target)` are
  added when `--file` is provided.

## JetBrains

Generated directory:

- `.idea/runConfigurations`

Native plugin surface:

- `NcRunConfigurationType` registers a real JetBrains Run Configuration named
  `nC`.
- `NcRunConfigurationProducer` creates a temporary configuration from the active
  `.n`, `.nc`, `.nb`, `.nbb` or `.ncp` file, so the IDE Run button can
  launch the selected nC file without hand-written shell configuration.
- `nC: Run Active File` and `nC: Debug Active File` are plugin actions under the
  Tools menu. They create an in-memory configuration and execute it through the
  JetBrains Run or Debug executor.
- The native configuration resolves `ncc` through `ncup which ncc --json`, then
  falls back to `ncc` from `PATH` if `ncup` is unavailable.
- Run profiles map to real `ncc run` flags:
  - `auto`: `ncc run <file>`; the compiler chooses `native-fast-path` for
    backend-lowerable single-file programs and keeps project/input/runtime-hosted
    surfaces on the interpreter.
  - `debug`: `ncc run <file> --profile debug`
  - `interpreted`: `ncc run <file> --interpreted`
  - `real` / `native-fast`: `ncc run <file> --native-fast`
  - `native-ffi`: `ncc run <file> --native-ffi`
  - `release`, `release-fast`, `release-small`, `unsafe-fast`, `instrumented`:
    `ncc run <file> --profile <profile>`
- CC profile, entry symbol, linker path and show-return are passed as
  `--cc`, `--entry`, `--linker` and `--show-return`.
- Target triple is stored in the run configuration and exported as
  `NCC_TARGET_TRIPLE`. It is not passed as `--target` to top-level `ncc run`
  because that command does not currently accept a `--target` flag.
- Debug launches under the JetBrains Debug executor and exports
  `NCC_IDE_DEBUG=1` plus `NCC_DEBUGGER_FRONTEND=jetbrains`. Source breakpoints
  and adapter-level stepping are backed by `ncc debug-adapter`. `.nm` is editable
  and checkable in JetBrains, but does not create Run/Debug configurations; use
  `ncc machine debug-bridge` explicitly in a terminal for CPU/register state. DAP attach
  requests use `ncc machine debug-attach` for a real OS attach/snapshot/detach
  and expose `Live OS Registers` plus `Live OS Attach` scopes when the host
  permits debugger attachment.

Generated run configurations:

- `nC Run Active File` -> `.ncc/ide/nc-run.<cmd|sh> "$FilePath$" auto host auto`
- `nC Check Active File` -> `.ncc/ide/nc-check.<cmd|sh> "$FilePath$" auto host auto`
- `nC Format Active File` -> `.ncc/ide/nc-format.<cmd|sh> "$FilePath$" auto host auto`
- `nC Debug Adapter Source Map` -> `.ncc/ide/nc-debug-adapter.<cmd|sh> "$FilePath$" debug host auto`
- `nC Run Configured File` -> `ncc run "$PROJECT_DIR$/..."` when `--file` is
  provided.
- `nC Run Active File [<profile> @ <target> / <toolchain>]` for each requested
  non-default matrix row. These entries are normal JetBrains Run Configurations,
  so target/toolchain selection is a dropdown action, not a manual terminal
  edit.

The generated JetBrains configs still use JetBrains' `ShConfigurationType`, but
the interpreter is host-aware:

- Windows: `%COMSPEC% /C`, normally `C:\Windows\System32\cmd.exe /C`
- Linux, WSL, Kali, macOS and other Unix-like hosts: `/bin/sh` or the host
  shell when it is a real `sh`-compatible executable

The generated IDE scripts accept `.n`, `.nb`, `.nbb`, `.nc` and `.ncp`. Machine
`.nm` commands are intentionally terminal-only.
When `bridge.json` exists, Run falls back to the project `main.n` and Check uses
`ncc project check .`, so the IDE button behaves like a project run/check entry
instead of accidentally executing README files or inactive documents.
For files outside a formal project, `use <project:hijo>` may resolve siblings,
`src/` children or paths listed in `NCC_PROJECT_PATH`; cycles and ambiguous
children are diagnosed by `ncc run` before the IDE receives runtime output.
Concurrent IDE and terminal executions share the same project cache lock at
`.ncc/locks/project-cache.lock`, so generated `.ncc/code`, `.ncc/lowering`,
`.ncc/obj`, AST cache and incremental status files are written atomically rather
than being interleaved by two ncc processes.

## Profile/Target Semantics

- File Run without `bridge.json` delegates to `ncc run <file>` in `auto` mode,
  or appends the selected profile flags (`--interpreted`, `--native-fast`,
  `--native-ffi` or `--profile <profile>`) when a non-auto profile is selected.
- File Run with a non-host target maps the target to the nearest supported
  backend CC profile for current `ncc run`: `nC.sysv64`, `nC.win64`,
  `nC.aapcs64`, `nC.portable` or `nC.sovereign`.
- Project Run with `bridge.json` delegates to `ncc project run .`; when the
  target is not `host`, it passes `--target <triple>`, which is the current
  project-level target authority.
- Project Check with `bridge.json` delegates to `ncc project check .` and also
  passes `--target <triple>` for non-host targets.
- Single-file Check with a non-host target uses `ncc emit-backend <file> --cc
  <profile>` so target lowering is exercised instead of doing a plain semantic
  check only.

The same generated wrapper scripts are used by VS Code and JetBrains. On
Windows they are `.cmd` files executed through `%COMSPEC% /C`; on Linux, WSL,
Kali and macOS they are `.sh` files executed through `/bin/sh` or the host shell
when compatible.

## Non-Overwrite Rule

`nide init` does not overwrite existing IDE files by default. Existing files are
reported as `skipped_exists`; use `--force` when the project intentionally wants
to regenerate them.
