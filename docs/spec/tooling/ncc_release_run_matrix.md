# ncc release run matrix

`ncc run` is the release entrypoint used by terminals and IDE run buttons. It
must accept the active file directly, without requiring a project wrapper.

## Shell matrix

The release matrix covers:

- Windows CMD: `cmd /C ncc run <file>`
- PowerShell: `powershell -NoProfile -Command "ncc run <file>"`
- WSL: `sh -lc 'ncc run <file>'`
- Kali/Ubuntu/Linux: `sh -lc 'ncc run <file>'`
- macOS: `sh -lc 'ncc run <file>'`

The active host is executed by integration tests. Other hosts are kept as
compile-time platform tests so the matrix is exercised when the suite runs on
that OS.

## File matrix

`ncc run` dispatches by extension:

- `.n`: high-level nC execution with project bridge, single-file workspace,
  native-fast or interpreter route selection.
- `.nb`: basal source verification through `basal check`.
- `.nbb`: basal binary verification through `basal check`.
- `.nm`: machine source verification through `machine check`.

For `.n`, the runner first resolves the active file exactly as given. If the
argument is only a filename, it searches the current workspace tree while
skipping build/cache/archive directories and fails with an ambiguity diagnostic
when two files share the same name. If a matching `bridge.json` owns the selected
`main.n`, `ncc run main.n` delegates to project execution. If no bridge exists
but the file imports `use <project:...>`, it builds a single-file workspace and
runs the linked program.

Plain `ncc run file.n` auto-selects `native-fast` only after backend object
emission proves that executable sections have real bytes and no unresolved
symbols remain. Hosted input, type-contract guard surfaces and runtime helpers
that still require the interpreter stay on the interpreter route unless the user
explicitly requests a native profile.

All successful routes write `.ncc/run/last.json` where the source directory can
own runtime metadata.

## Project matrix

The release tests cover:

- one loose file without `bridge.json`;
- a project `main.n` with `bridge.json`;
- `use <project:child>` imports;
- route cache generation;
- bare filename resolution from folders containing many `.n` files;
- paths and filenames with spaces.
