# nC-gas-intel-compat-v1

Status: closed.

This profile covers GNU assembler Intel syntax with `.intel_syntax noprefix`, `.text/.data/.bss/.section`, `.globl`, `.extern`, `.byte/.word/.long/.quad`, `.space/.zero`, `.align/.p2align`, `.equ/.set`, labels, comments, Intel operand order, Intel memory operands, and relocation expressions.

Import lowers profile syntax to `.nm`; export and import-back preserve section, symbol, directive, memory, and instruction semantics. AT&T `%register` syntax belongs to the GAS AT&T profile.
