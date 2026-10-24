# nC-gas-att-compat-v1

Status: closed.

This profile covers GNU assembler AT&T syntax: source-first operands, `%register`, `$immediate`, `disp(base,index,scale)` memory forms, `b/w/l/q` suffixes, `.text/.data/.bss/.section`, `.globl`, `.type/.size`, data directives, alignment, `.equ/.set`, comments, labels, and relocation expressions.

Import normalizes AT&T operand order into official `[@machine]` destination-first IR. Export and import-back preserve semantics inside the declared profile.
