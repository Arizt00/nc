# JetBrains native run/debug contract

Status: active plugin contract.

The JetBrains plugin owns a native Run Configuration type for nC. Generated
`.idea/runConfigurations` entries remain supported, but the plugin can now run
or debug a selected nC file directly from IDE actions.

## Runnable files

The plugin accepts:

- `.n`
- `.nc`
- `.nb`
- `.nbb`
- `.nm`
- `.ncp`

The same extension list is used by file types, active-file actions and the
automatic run-configuration producer.

## Toolchain resolution

Execution resolves the compiler in this order:

1. `ncup which ncc --json`
2. `ncc` from `PATH`

The process receives:

- `NCC_IDE=jetbrains`
- `NCC_IDE_TOOLCHAIN_SOURCE`
- `NCC_IDE_TOOLCHAIN_ID` when `ncup` reports one
- `NCC_TARGET_TRIPLE` when the run configuration declares a target

## Profiles

The profile field maps to top-level `ncc run`:

- `debug`: no extra flag
- `interpreted`: `--interpreted`
- `real` and `native-fast`: `--native-fast`
- `native-ffi`: `--native-ffi`
- release-like profiles: `--profile <name>`

`--cc`, `--entry`, `--linker`, `--show-return` and user program arguments are
also supported.

The native plugin launches `GeneralCommandLine(ncc)` directly with the project
root as working directory. It does not require a shell script interpreter, so on
Windows it does not depend on `sh`; generated `nide init` run configurations use
`cmd.exe` on Windows projects and `sh` only on Unix-like hosts.

## Debug

`nC: Debug Active File` uses the JetBrains Debug executor and launches the
resolved nC toolchain with:

- `NCC_IDE_DEBUG=1`
- `NCC_DEBUGGER_FRONTEND=jetbrains`

The native nC debug adapter is available as:

```bash
ncc debug-adapter
ncc debug-adapter --probe --json
ncc debug-adapter --source-map file.n --json
```

The adapter speaks Debug Adapter Protocol over `Content-Length` framed stdio
messages and supports `initialize`, `launch`, `setBreakpoints`,
`configurationDone`, `threads`, `stackTrace`, `continue`, stepping requests,
`disconnect` and `terminate`. Generated `nide init` projects also include
`.ncc/ide/nc-debug-adapter.cmd` or `.sh`.

Source breakpoints and adapter-level stepping are therefore closed for the IDE
surface: if a configured source breakpoint is hit, the adapter emits `stopped`,
answers `stackTrace` for the source line and resumes execution on `continue`.
Deep native CPU/register stepping is closed for `.nm` targets through:

```bash
ncc machine debug-bridge file.nm --trace --json
ncc machine debug-attach --pid <pid> --json
```

The DAP adapter advertises both bridges in `--probe --json`. It exposes
`Machine Registers` / `Machine Instruction` scopes when the launched file is
`[@machine]`, and `Live OS Registers` / `Live OS Attach` scopes for DAP attach
requests. The machine bridge consumes debug metadata, encoded bytes, PC offsets,
CFI and register semantics. The live attach bridge uses the host OS debugger API
for attach/snapshot/detach and reports typed diagnostics when OS permissions
deny the operation.
