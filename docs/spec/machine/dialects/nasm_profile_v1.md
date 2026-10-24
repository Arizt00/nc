# nC-nasm-compat-v1

Status: closed.

This profile covers the NASM Intel-syntax subset declared by nC: `bits 64`, `global`, `extern`, `section`, `db/dw/dd/dq`, `resb/resw/resd/resq`, `align`, `equ`, `times`, `%define`, basic `%macro/%endmacro`, `%include`, `incbin`, labels, local labels, comments, immediates, symbol expressions, and Intel memory operands such as `[rax]`, `[rax + 8]`, `[rax + rbx*2]`, `[rax + rbx*4 + 16]`, and `[rel symbol]`.

Import lowers NASM into official `[@machine]` IR and must pass `ncc machine check`. Export renders the same profile from `.nm`. Import-back is required to be lossless for profile examples. Unsupported constructs such as `struc`, `absolute`, and non-64-bit modes are diagnostics.
