# nC-llvm-mc-compat-v1

Status: closed.

This profile covers the LLVM MC subset declared by nC: `.text/.data/.section`, `.globl`, `.type/.size`, Intel-flavor x86 operands for the v1 profile, data directives, relocation expressions, labels, comments, import, export, and import-back.

Target-specific pseudo operations outside the nC v1 profile are unsupported by design and must be reported.
