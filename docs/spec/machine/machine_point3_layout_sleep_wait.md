# Machine Point 3 - Layout evidence and wait/sleep imports

Status: implemented for the machine surfaces touched by Backend Point 3.

Machine does not compute high-level struct layout independently. It consumes the
backend object, MCT object or metadata produced by the high-level backend. The
machine responsibility for this point is to preserve symbols, imports and real
loader materialization for the wait/sleep calls that backend emits.

## Closed

- PE64 keeps `sleep` as an internal runtime thunk and imports the real WinAPI
  symbol `Sleep` from `KERNEL32.dll`.
- The PE import table/IAT route materializes `Sleep` like the other supported
  runtime OS calls; it is not a deferred assembler dependency.
- Hosted POSIX MCT lowering emits `usleep` as a normal C ABI symbol. The final
  object/executable route treats it as a runtime/host import rather than an
  unknown high-level call.
- Aggregate layout evidence is retained before machine encoding through
  `TypeAbiLayoutIR`, MCT LLVM comments and backend object metadata. Machine
  should not recalculate different offsets for the same high-level type.

## ISA impact

No new instruction encoding family was required for this point. The generated
machine work is call/import/linker work:

- Win64 import thunk calls `Sleep`.
- POSIX MCT object calls `usleep`.
- Struct and vector-of-struct addressing continues to use the existing address
  generation path: base pointer, scaled index and field displacement.

If a future direct high-level machine path bypasses MCT for aggregate bodies, it
must reuse the same size/alignment/offset descriptor and prove emitted
addressing bytes against these tests.

## Evidence

- `phase12_pe_loader_import_symbols` maps the high-level `sleep` runtime surface
  to `Sleep`.
- `phase12_pe_import_dll("Sleep")` returns `KERNEL32.dll`.
- `phase12_pe_internal_sleep_thunk` emits real x86_64 Win64 bytes around the
  import call and returns `true`.
- IR tests prove POSIX and Win64 sleep lowering select the correct symbol before
  object/link materialization.

## Pending outside this point

- Direct native machine instruction selection for all aggregate body operations
  on every target, when the direct high-level backend replaces MCT for those
  bodies.
- Runtime scheduler internals for futures/tasks/events. Those belong to the
  Runtime phase, not the machine encoding layer.
