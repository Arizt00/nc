# Native and machine debugging v1

## High-level `.n`

The IDE builds a debug executable with:

```text
ncc compiler build-native --profile debug --strict --no-run
```

The executable contains source-linked DWARF or PDB information. VS Code then
uses LLDB-DAP, with GDB-DAP as fallback. Breakpoint verification, stepping,
stack frames, local variables, expression evaluation, console output,
termination and restart are performed by the native debugger against the real
process.

## Machine `.nm`

Low-level files use `ncc debug-adapter` and the machine debug bridge. The DAP
exposes register and current-instruction scopes, evaluates register expressions
such as `$rax + 2`, and supports clean terminate and restart requests. Machine
and source debugging are separate routes because their execution authority and
observable state differ.

## Adapter selection

`nc.debugAdapterPath` can name `lldb-dap`, `lldb-vscode` or GDB. Automatic
selection prefers LLDB-DAP, including an LLVM installation discovered inside
WSL, and then falls back to GDB-DAP. Missing debugger binaries produce an
actionable error instead of launching the old evaluator-level adapter.

## Verification

The retained integration test builds nC with debug symbols and drives GDB when
available, or LLDB on LLVM/macOS/Windows hosts, through a breakpoint,
backtrace, locals, expression evaluation, step-in, step-out, step-over, kill,
restart and normal exit. A separate framed-DAP test verifies `.nm` register
evaluation and restart. Direct root `to main` bodies carry exact source spans,
so their breakpoints are backed by emitted debug metadata too.
