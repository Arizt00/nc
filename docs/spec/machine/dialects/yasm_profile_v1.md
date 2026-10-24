# nC-yasm-compat-v1

Status: closed.

This profile covers the YASM NASM-compatible subset declared by nC: Intel operands, `bits 64`, `global`, `extern`, sections, data and reserve directives, `align`, `equ`, `%define`, basic macros, include/incbin, comments, symbol expressions, and Intel memory addressing. Import, export, and import-back share the same lossless contract as NASM.

YASM object/virtual modes outside the nC object profile are unsupported by design and must produce diagnostics.
