# Machine Entry Bridge For Backend Point 1

Status: closed for the current `ncc build` entry handoff.

The high-level program object and the OS process entry are separate artifacts.
`ncc build app.n` emits:

- `app.obj`: bytes from optimized high-level backend object IR;
- `app.nm`: generated Machine source for the entry bridge;
- `app.entry.obj`: bytes emitted from `app.nm`;
- `app.exe`: linked final executable.

The bridge is target-aware:

- Win64 reserves shadow space, calls the resolved high-level entry, preserves
  the return value when needed and calls `ExitProcess` for the direct PE route.
- SysV64 aligns the stack for the hosted entry and calls the resolved symbol.
- AArch64/RISC-V bridge forms are represented in the same generator and are
  admitted when the selected object/link route supports them.
- The bridge can call the generated `__ncc_stc_init.<container>` symbol before
  the user entry when static container initialization is present.

The `.nm` bridge is intentionally not a full instruction-for-instruction dump
of the high-level function body. The body is materialized in `app.obj`; the
bridge is the process entry and ABI handoff layer.

Machine validation applied by the build:

- parse/check generated `.nm`;
- verify target block, registers, operands, symbols, encoding and object
  emission;
- emit `.entry.obj`;
- link `.entry.obj` with the high-level program object.

Point 1 did not require adding a new ISA row. It reused existing encoded forms:

- x86_64 stack adjust, call, mov, ret;
- Win64 external calls to `ExitProcess`;
- RIP-relative data access when return-capture formatting is used;
- AArch64/RISC-V call-return bridge forms in the generator.

If later backend features require an instruction missing from Machine, that ISA
row must be added in the same backend change and covered by a machine test.
