# nC-fasm-compat-v1

Status: closed.

This profile covers the FASM subset declared by nC: `format ELF64`, declared section forms, `public`, `extrn`, `db/dw/dd/dq`, `rb/rw/rd/rq`, `equ`, full `repeat` expansion, full profile macro expansion, local labels, comments, memory operands, import, export, and import-back.

`repeat` is closed for literal counts, constants, CTFE counts, `%` counter use, nesting, local label hygiene, repeat-inside-macro, macro invocation inside repeat, expansion limits, diagnostics, deterministic MachineIR lowering, export, and import-back.

Macros are closed for no-parameter, parameterized, multi-parameter, variadic, local-label, macro-calls-macro, repeat-inside-macro, data directive, instruction emit, register operand, memory operand, hygiene, deterministic generated labels, recursion detection, recursion limit, expansion report, export, and import-back forms.

FASM `virtual`, assembler variables, and PE import machinery are outside this profile and are diagnostics.
