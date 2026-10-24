# Machine Debug Bridge

Status: closed for `[@machine]` register-level source/byte stepping.

`ncc machine debug-bridge` is the low-level debugger authority consumed by the
native DAP adapter when the debug target is a `.nm` file.

## Commands

```bash
ncc machine debug-bridge probe --json
ncc machine debug-bridge file.nm --target x86_64-unknown-linux-gnu --trace --json
ncc machine debug-bridge file.nm --pc 4 --json
ncc machine debug-attach probe --json
ncc machine debug-attach self-test --json
ncc machine debug-attach --pid <pid> --json
```

## Contract

The bridge reads the official `.nm` machine source, validates it through the
existing machine audit path, encodes instruction bytes, then emits:

- target, ABI and object format;
- source debug file and line range;
- bytecode length and hash;
- instruction stream with PC byte offsets and raw bytes;
- register snapshots and final register file;
- unwind frame/CFI metadata;
- diagnostics when the machine source is invalid.

For x86_64, the bridge executes closed register semantics for common scalar
instructions such as `push`, `pop`, `mov`, `add`, `sub`, `inc`, `dec`, `xor`,
`nop`, `ret`, `hlt` and `int`. Unsupported instructions remain step-visible
with exact bytes and PC movement under `opaque_step_bytes_preserved`.

## Live OS Attach

`ncc machine debug-attach` is the live operating-system debugger boundary. It
uses the host debug API to attach to a process, stop it, collect a snapshot and
detach immediately:

- Linux: `ptrace(PTRACE_ATTACH)`, `waitpid`, `PTRACE_GETREGS` on x86_64 and
  `PTRACE_DETACH`;
- Windows: `DebugActiveProcess` followed by `DebugActiveProcessStop`;
- macOS: `ptrace(PT_ATTACH)`, `waitpid` and `PT_DETACH`.

The command is intentionally a snapshot bridge, not a claim of permanent
ptrace ownership. A failed attach returns a typed `E_MACHINE_DEBUG_ATTACH_*`
diagnostic with the OS error. The `self-test` mode launches a child process and
attaches to that child so the feature can be validated without selecting an
external target.

The deterministic `.nm` bridge and the live OS attach bridge are complementary:
`debug-bridge` owns byte/instruction/register stepping for nC `[@machine]`
programs, while `debug-attach` owns real process attach/snapshot/detach through
the host OS.

## DAP Integration

`ncc debug-adapter` advertises this bridge in `--probe --json`. When the launched
program is `.nm`, the DAP `scopes` request exposes:

- `Machine Registers`;
- `Machine Instruction`.

The `variables` request then returns register names, hex values, ABI roles and
the current machine instruction state.

For DAP `attach`, the adapter calls `ncc machine debug-attach` internally and
exposes:

- `Live OS Registers`;
- `Live OS Attach`.

Those scopes show the captured OS register snapshot and attach metadata. A
continue request terminates the snapshot debug session because the process has
already been detached.
