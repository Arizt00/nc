# ncc Native Control-Flow Lowering

Status: closed for native scalar control-flow V1.

This pass extends the release-fast native path beyond scalar returns and
register/stack arithmetic. Backend-lowerable root functions can now emit real
x86_64 control-flow bytes for scalar `if/else`, `while`, `do while`, `loop`,
`repeat until`, `same for (count)`, classic `for`, scalar `for range`,
`break`, `continue`, labeled loop targets, and multiple returns.

The lowering is deliberately native and structural:

- conditions lower to `cmp`/`jcc` when they are scalar comparisons;
- non-comparison conditions lower to `test reg, reg` plus `jz` or `jnz`;
- unconditional branches lower to `jmp rel32`;
- branch labels are resolved inside the function section without external
  relocations;
- each lowered function has a single scalar epilogue label, so multiple returns
  preserve stack teardown and ABI return location;
- scalar locals and expression temporaries use fixed stack slots, not dynamic
  `push` spills, so `rsp`-relative local addressing remains stable.

Covered source forms:

- `if (...) { ... } else { ... }`
- `while (...) { ... }`
- `do { ... } while (...);`
- `loop { ... }`
- `repeat { ... } until (...);`
- `same for (N) { ... }`
- `for (init; condition; step) { ... }`
- `for i in start .. end { ... }`
- `for i in start ..= end { ... }`
- `break`, `continue`, and labeled loop forms.

The V1 surface is intentionally scalar. Collection iteration (`for x in list`)
continues to require the runtime collection path until collection iterators are
lowered to native memory cursors. Calls inside branch bodies are still handled
by the existing function call plan when they are visible to backend planning;
general interleaved call scheduling remains a later native scheduler/allocator
phase.

Validation fixtures live in `examples/performance/native_fast/control_*.n`.
Every fixture returns `42` through `ncc run --native-fast --show-return`.
