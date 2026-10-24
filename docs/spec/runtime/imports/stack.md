# `use <memory:stack>;`

## Contract

This explicit import exposes the material reservation of the current thread
stack and captures native return addresses. It does not replace `stackAlloc`,
automatic storage, unwind metadata or the raw pointer API.

```n
stackDepth() -> u64
stackLimit() -> u64
stackRemaining() -> u64
stackTrace() -> StackTraceHandle
stackTraceLen(StackTraceHandle) -> u64
stackTraceFrame(StackTraceHandle, int) -> Result<addr, MemoryError>
stackTraceRelease(StackTraceHandle) -> bool
```

`stackDepth` measures from the operating-system high bound to the current stack
pointer. `stackRemaining` measures from the current pointer to the low bound.
`stackLimit` is the whole reserved interval. Every measurement belongs to the
calling thread and may change between calls.

## Material provider

The seven `ncc_rt_stack_*` bodies are emitted directly into MCT LLVM. They are
not declarations owned by `runtime_hosted_native.c` and do not cause a hosted C
source or object to be generated.

- Windows uses `GetCurrentThreadStackLimits`, `RtlCaptureStackBackTrace`,
  `VirtualAlloc` and `VirtualFree`.
- Linux uses `pthread_getattr_np`, `pthread_attr_getstack`, `backtrace`, `mmap`
  and `munmap`.
- Darwin uses `pthread_get_stackaddr_np`, `pthread_get_stacksize_np`,
  `backtrace`, `mmap` and `munmap`.
- Android uses pthread stack bounds and captures the native caller address with
  `llvm.returnaddress` when the platform backtrace ABI is unavailable.

`stackTrace` allocates one 4096-byte page. The page contains the descriptor and
up to 64 pointer-width frame addresses inline. It is tagged with the stable
`NCSTACK1` magic and remains owned by the caller until `stackTraceRelease`.
There is no separate frame allocation and no implicit cleanup.

## Result ABI

`stackTraceFrame` retains the existing raw `Result<addr, MemoryError>` ABI:

- success: `(address << 1) | 1`;
- invalid handle: error code 1, encoded as `2`;
- invalid index: error code 3, encoded as `6`.

The 32-bit provider truncates pointers when constructing handles and zero
extends them when exposing the stable 64-bit language-level address payload.

## Targets and diagnostics

The provider admits Win32/Win64, Linux i386/x86_64, Android 32/64 and Darwin
64-bit targets. A target without a native stack-bound and frame-capture ABI is
rejected with `E_MCT_LLVM_STACK_TARGET_001`; it is never replaced by a zero or
interpreter-only body.

The import remains mandatory. Missing `use <memory:stack>;`, an unknown item,
invalid arity or incompatible handle type is rejected before object emission.
An invalid frame index returns `MemoryError`; it does not perform an unchecked
descriptor read.

## Evidence

The executable fixture is
`examples/runtime/families/memory_stack_basic.n`. The retained Win64 object
defines all seven symbols and the PE64 executable imports only the four Win32
stack/memory boundaries plus `ExitProcess`. ELF64 x86_64, ELF32 i386 and Mach-O
arm64 objects are also retained. Cross-object generation is not host execution.
