# Machine Definition

`[@machine]` declares machine code as a contract. It can produce MachineIR, textual assembly, raw bytes, object files, reports, diagnostics, and evidence.

`[@machine]` must not use high-level `fn:` or `fa:` sections. A routine is materialized through `symbol:`, `entry:`, `proc:` or `routine:`, `frame:`, `instr:`, `ret`, `hlt`, `jmp`, `verify:`, and `seal:`.

Normative phrases:

C allows the programmer to approach the machine. Assembly exposes instructions. `[@machine]` requires the program to declare the machine as a contract.

Assembly lets the programmer speak to the machine. `[@machine]` requires proof that the statement is coherent.
