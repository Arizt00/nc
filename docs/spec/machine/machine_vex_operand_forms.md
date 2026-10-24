# Machine VEX Operand Forms

Fase 1A records VEX operand forms as structured data. Each `VexOperandForm` declares operand count, destination, sources, immediate slot, register-register support, register-memory support, memory-register support, xmm and ymm participation, memory participation, immediate participation, width, and invalid-form diagnostics.

The current closed form evidence is limited to selected register-register xmm and ymm forms. Memory and immediate forms remain debt for Fase 1B and later Fase 1 work.

Invalid forms are rejected explicitly:

- scalar register where a vector register is required;
- mismatched vector width;
- invalid memory width;
- unexpected immediate;
- missing VEX source operand encoded through vvvv.
