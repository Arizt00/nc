# nC Runtime Import: nc.runtime.process

## 1. Purpose
`nc.runtime.process` is the registered runtime import surface for the `process` family. It exposes operational functions under explicit import, capability, policy, and diagnostic contracts.

## 2. Canonical compact import
```n
use stoud.process { process };
```

## 3. Aliases
`runtime.process`

## 4. Mode availability
Allowed: `[@runtime]` examples and runtime-aware compiler analysis.

Denied: root and primitive under strict runtime import rules.

## 5. Exported functions
| Function | Signature | Return | Capability | Policy | Errors | Notes |
|---|---|---|---|---|---|---|
| `processId` | `processId() -> int` | `int` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | devuelve el PID real del proceso `ncc run` |
| `parentProcessId` | `parentProcessId() -> int` | `int` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | devuelve PPID real en Linux/WSL o `0` si la plataforma no lo expone |
| `spawn` | `spawn(str, any) -> Result<ProcessHandle, ProcessError>` | `Result<ProcessHandle, ProcessError>` | `process.spawn` | `command.no-injection, imports.official-runtime-required, process.audit.required, process.no-shell-by-default, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lanza un hijo real con `Command::new` y devuelve un handle nominal opaco |
| `spawnPipe` | `spawnPipe(str, any) -> Result<ProcessHandle, ProcessError>` | `Result<ProcessHandle, ProcessError>` | `process.spawn` | `command.no-injection, imports.official-runtime-required, process.audit.required, process.no-shell-by-default, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lanza un hijo con stdin/stdout/stderr reales en modo pipe |
| `spawnIn` | `spawnIn(str, any, str) -> Result<ProcessHandle, ProcessError>` | `Result<ProcessHandle, ProcessError>` | `process.spawn` | `command.no-injection, imports.official-runtime-required, process.audit.required, process.no-shell-by-default, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lanza un hijo con working directory real |
| `spawnWithEnv` | `spawnWithEnv(str, any, any) -> Result<ProcessHandle, ProcessError>` | `Result<ProcessHandle, ProcessError>` | `process.spawn` | `command.no-injection, imports.official-runtime-required, process.audit.required, process.no-shell-by-default, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lanza un hijo con overrides `KEY=VALUE` |
| `spawnWithOptions` | `spawnWithOptions(str, any, str, any, bool, bool) -> Result<ProcessHandle, ProcessError>` | `Result<ProcessHandle, ProcessError>` | `process.spawn` | `command.no-injection, imports.official-runtime-required, process.audit.required, process.no-shell-by-default, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | ruta completa cwd/env/pipes/grupo |
| `spawnGroup` | `spawnGroup(str, any) -> Result<ProcessHandle, ProcessError>` | `Result<ProcessHandle, ProcessError>` | `process.spawn, process.signal` | `command.no-injection, imports.official-runtime-required, process.audit.required, process.no-shell-by-default, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lanza un hijo con metadatos de grupo de procesos |
| `wait` | `wait(any) -> Result<int, ProcessError>` | `Result<int, ProcessError>` | `process.wait` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | consume el handle y devuelve el exit code real |
| `waitStatus` | `waitStatus(any) -> Result<str, ProcessError>` | `Result<str, ProcessError>` | `process.wait` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | consume el handle y devuelve `kind/success/code/signal/timed_out` |
| `waitTimeout` | `waitTimeout(any, int) -> Result<str, ProcessError>` | `Result<str, ProcessError>` | `process.wait, process.kill` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | consume el handle y mata el proceso si supera el timeout |
| `writeStdin` | `writeStdin(any, str) -> Result<int, ProcessError>` | `Result<int, ProcessError>` | `process.spawn` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | escribe en stdin pipe |
| `closeStdin` | `closeStdin(any) -> Result<bool, ProcessError>` | `Result<bool, ProcessError>` | `process.spawn` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | cierra stdin pipe sin consumir handle |
| `readStdout` | `readStdout(any, int) -> Result<str, ProcessError>` | `Result<str, ProcessError>` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lee stdout pipe |
| `readStderr` | `readStderr(any, int) -> Result<str, ProcessError>` | `Result<str, ProcessError>` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | lee stderr pipe |
| `statusCode` | `statusCode(str) -> int` | `int` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | extrae exit code de `waitStatus/waitTimeout` |
| `statusSignal` | `statusSignal(str) -> int` | `int` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | extrae señal de `waitStatus/waitTimeout` |
| `statusSuccess` | `statusSuccess(str) -> bool` | `bool` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | extrae éxito de `waitStatus/waitTimeout` |
| `statusTimedOut` | `statusTimedOut(str) -> bool` | `bool` | `process.inspect` | `imports.official-runtime-required, runtime.no-silent-fallback` | `none` | extrae timeout de `waitStatus/waitTimeout` |
| `kill` | `kill(any) -> Result<bool, ProcessError>` | `Result<bool, ProcessError>` | `process.kill` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | mata el hijo real, consume el handle y limpia la tabla |
| `signal` | `signal(any, int) -> Result<bool, ProcessError>` | `Result<bool, ProcessError>` | `process.signal` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | manda señal real al hijo |
| `signalGroup` | `signalGroup(any, int) -> Result<bool, ProcessError>` | `Result<bool, ProcessError>` | `process.signal` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | manda señal real al grupo |
| `killGroup` | `killGroup(any) -> Result<bool, ProcessError>` | `Result<bool, ProcessError>` | `process.kill, process.signal` | `imports.official-runtime-required, runtime.no-silent-fallback` | `E_RUNTIME_PROCESS_001` | mata el grupo y consume el handle |

## 6. Type contracts
Function signatures are the active type contract surface for this import. `ProcessHandle` and `ProcessError` are runtime nominal values, represented internally as tagged runtime records instead of plain `int`/`str`.

## 7. Ownership rules
Ownership follows the declared argument and return contracts. Memory-bearing families additionally require origin, bounds, alias, or lifecycle policies when listed below.

## 8. Nullability rules
Nullability is explicit through `Option`, nullable pointer contracts, or family-specific refinement policies. Silent null widening is not part of this import contract.

## 9. Bounds rules
Bounds-sensitive operations are governed by the required capabilities and policies listed below. Out-of-range or invalid state must surface a official runtime diagnostic.

## 10. Side effects
Side effects are limited to the family purpose and guarded by the declared capabilities. IO, filesystem, process, network, security, and memory families require their listed policy gates.

## 11. Capabilities
- `process.spawn`
- `process.wait`
- `process.kill`
- `process.signal`
- `process.inspect`

## 12. Policies
- `command.no-injection`
- `imports.official-runtime-required`
- `process.audit.required`
- `process.no-shell-by-default`
- `process.timeout.required`
- `runtime.no-silent-fallback`

## 13. Errors
- `E_RUNTIME_PROCESS_001`: fallo del sistema operativo al lanzar, esperar o matar
  un proceso, comando vacío, argumentos no textuales, handle desconocido o handle
  ya consumido.

El runtime mantiene un registro interno de procesos vivos. `spawn` inserta el
proceso hijo en esa tabla y `wait`/`kill` eliminan la entrada. Cualquier segundo
uso del mismo handle produce `Result.err(ProcessError)`.

## 14. Positive examples
- `examples/runtime/families/process_basic.n`
- `examples/runtime/runtime_real_process_id.n`
- `examples/runtime/runtime_real_process_spawn_wait.n`
- `examples/runtime/runtime_real_process_spawn_kill.n`
- `examples/runtime/runtime_real_process_pipes_stdio.n`
- `examples/runtime/runtime_real_process_stderr_pipe.n`
- `examples/runtime/runtime_real_process_env_cwd.n`
- `examples/runtime/runtime_real_process_timeout_group_signal.n`

## 15. Negative examples
- `examples/runtime/negative/runtime_real_process_invalid_spawn_result.n`
- `examples/runtime/negative/runtime_real_process_double_wait_result.n`
- `examples/runtime/negative/runtime_real_process_wait_invalid_handle_result.n`
- `examples/runtime/negative/runtime_real_process_kill_missing_process_result.n`
- `examples/runtime/negative/runtime_real_process_bad_env_result.n`
- `examples/runtime/negative/runtime_real_process_read_stdout_without_pipe_result.n`
- `examples/runtime/negative/runtime_real_process_kill_group_plain_handle_result.n`
- Negative missing-use behavior is verified by `crates/ncc-cli/tests/runtime_family_closure.rs` using the first exported symbol for each discovered family.

## 16. Strict mode
`ncc runtime check <file> --strict` accepts explicit runtime imports in `[@runtime]` and rejects runtime imports in forbidden modes.

## 17. Runtime report shape
The generated report for this family is stored at `reports/runtime_closure/runtime_reports/process.json` and `reports/runtime_closure/runtime_reports/process.md`.

## 18. Industrial conformance checklist
- import identity checked
- export signatures checked
- capabilities checked
- policies checked
- example check passed
- runtime JSON report generated
- runtime Markdown report generated

## 19. Known non-goals
This import does not make runtime calls available without `use`, does not bypass capability/policy checks, and does not cross into `[@machine]` or `primitive[@]`.

## 20. Evidence
- exports command: `ncc runtime exports nc.runtime.process --json`
- check command: `ncc runtime check examples/runtime/families/process_basic.n --strict --json`
- report command: `ncc runtime report examples/runtime/families/process_basic.n --json`
- evidence hash: `3ea795067d0f528843ac0d5c605df78a1de9e61a064f2983162fef3740abcfab`

## 21. Native materialization

The native `spawn/wait` path is defined by LLVM/MCT as
`ncc_rt_process_spawn` and `ncc_rt_process_wait`; it is not supplied by the
hosted C provider. Windows resolves executable and script suffixes before
`CreateProcessA`. POSIX builds construct `argv` in caller-visible memory and use
`fork/execvp/waitpid`. Waiting consumes and releases the raw handle record.
