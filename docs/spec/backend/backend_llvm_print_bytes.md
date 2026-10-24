# Backend LLVM print bytes

Status: phase closed for hosted sysv64 smoke.

This phase lowers the high-level print surface to real hosted backend bytes for MCT LLVM functions reached through `ref main ->> ...;`.

## Surface

Covered statements:

- `print(expr)`
- `println(expr)`
- `println()`
- `print.sp()`
- `print.nl()`
- the named `print.*` family parsed by the front end

`print.err` writes to file descriptor 2. The rest of the family writes to file descriptor 1 unless a later runtime policy gives a variant a stronger channel contract.

## LLVM lowering

Literal text is emitted as byte arrays:

- source string bytes come from `TextLiteralBytesIR`, the shared UTF-8 lowering
  described in `docs/spec/backend/backend_text_byte_lowering.md`;
- `println` and `print.nl` append one newline byte;
- the final write length is the payload length, not the nul terminator length.

Dynamic `str`/`ptr` values are treated as hosted C strings for this phase:

- `strlen(ptr)` computes the payload length;
- `write(fd, ptr, len)` emits the bytes;
- `println` emits its newline through a second `write`.

Scalar fallback values lower through a local stack byte buffer:

- `snprintf(buffer, cap, format, value)` renders the scalar into bytes;
- `write(fd, buffer, rendered_len)` performs the observable output;
- the backend does not mix buffered `printf` with raw `write`, preserving output order under captured stdout.

## Main continuity

The existing LLVM main wrapper remains authoritative:

- `ref main ->> boot;` resolves the function signature;
- `i1` return values map to process exit status;
- print byte effects execute inside the referenced function before the wrapper returns.

## Evidence

Fixture:

- `examples/backend/mct_llvm_print_bytes.n`

Required checks:

- parser/sema accept the print surface;
- IR contains `StatementIR::Print`;
- LLVM declares `write`, `strlen`, and `snprintf` when needed;
- emitted LLVM contains concrete `[N x i8]` byte arrays for literal text;
- `mct llvm-exe-smoke` links with clang and runtime stdout is byte-stable.
