# Runtime Process Priority 5 Contract

Status: closed for the real runtime lane used by `ncc run archivo.n`.

This typed hosted surface is distinct from the allocation-explicit
`use <process:command>;` ABI. Native tooling such as ghor uses the raw ABI when
it needs inspectable handles, byte buffers and cross-host stdout capture without
creating `Result`, `str` or collection values implicitly.

This priority strengthens `use auth.system { process };` beyond
`spawn/wait/kill`. The runtime now owns a real process registry whose entries
keep the host child process, optional stdio pipes and optional process-group
metadata.

## Surface

All fallible calls return `Result<T,ProcessError>`.

| Function | Return | Contract |
|---|---|---|
| `spawn(cmd, args)` | `Result<ProcessHandle,ProcessError>` | spawn with stdin/stdout/stderr set to null |
| `spawnPipe(cmd, args)` | `Result<ProcessHandle,ProcessError>` | spawn with real stdin/stdout/stderr pipes |
| `spawnIn(cmd, args, cwd)` | `Result<ProcessHandle,ProcessError>` | spawn with a real working directory |
| `spawnWithEnv(cmd, args, env)` | `Result<ProcessHandle,ProcessError>` | spawn with `KEY=VALUE` environment overrides |
| `spawnWithOptions(cmd, args, cwd, env, pipe_stdio, process_group)` | `Result<ProcessHandle,ProcessError>` | full process creation path |
| `spawnGroup(cmd, args)` | `Result<ProcessHandle,ProcessError>` | create a process-group backed handle where supported |
| `writeStdin(handle, text)` | `Result<int,ProcessError>` | write bytes to a piped child stdin |
| `closeStdin(handle)` | `Result<bool,ProcessError>` | close child stdin without consuming the handle |
| `readStdout(handle, maxBytes)` | `Result<str,ProcessError>` | read text from child stdout pipe |
| `readStderr(handle, maxBytes)` | `Result<str,ProcessError>` | read text from child stderr pipe |
| `wait(handle)` | `Result<int,ProcessError>` | consume handle and return legacy exit code |
| `waitStatus(handle)` | `Result<str,ProcessError>` | consume handle and return full status text |
| `waitTimeout(handle, timeoutMs)` | `Result<str,ProcessError>` | consume handle; kill on timeout; return status text |
| `signal(handle, sig)` | `Result<bool,ProcessError>` | send host signal to child without consuming handle |
| `signalGroup(handle, sig)` | `Result<bool,ProcessError>` | send host signal to child process group |
| `kill(handle)` | `Result<bool,ProcessError>` | consume handle and kill child |
| `killGroup(handle)` | `Result<bool,ProcessError>` | consume handle and kill child process group |

`statusCode`, `statusSignal`, `statusSuccess` and `statusTimedOut` decode the
stable status text returned by `waitStatus` and `waitTimeout`.

## Status Text

The stable status string uses semicolon-separated fields:

```text
kind=exit;success=true;code=0;signal=0;timed_out=false
```

Timeouts return `kind=timeout` and `timed_out=true`. On Unix, signal exits carry
the real signal number. On non-Unix targets the signal field is `0` unless the
host backend exposes equivalent information.

## Ownership

`ProcessHandle` is linear for terminal operations:

- `wait`, `waitStatus`, `waitTimeout`, `kill` and `killGroup` consume it.
- `writeStdin`, `closeStdin`, `readStdout`, `readStderr`, `signal` and
  `signalGroup` borrow it and keep it alive.

Using an unknown, already-consumed or wrong-kind handle returns
`Result.err(ProcessError)`.

## Evidence

Positive examples:

- `examples/runtime/runtime_real_process_pipes_stdio.n`
- `examples/runtime/runtime_real_process_stderr_pipe.n`
- `examples/runtime/runtime_real_process_env_cwd.n`
- `examples/runtime/runtime_real_process_timeout_group_signal.n`

Negative examples:

- `examples/runtime/negative/runtime_real_process_bad_env_result.n`
- `examples/runtime/negative/runtime_real_process_read_stdout_without_pipe_result.n`
- `examples/runtime/negative/runtime_real_process_kill_group_plain_handle_result.n`

Tests:

```bash
cargo test -p ncc-cli --test runtime_process_thread_contract -- --nocapture
cargo test -p ncc-cli --test backend_mct_runtime_symbols -- --nocapture
```
