# Machine Macros Overview

Phase 10 defines macros as official nC machine constructs. A macro has a name, typed parameters, an auditable body, optional local labels, and an expansion record that lowers into MachineIR before verification and encoding.

Macros are not an imported assembler dialect. They exist inside the `[@machine]` contract and expand into normal `instr` nodes, so the existing verifier, encoder, object writer, disassembler, and roundtrip pipeline continue to see native machine semantics.

Status: closed for Phase 10. ASM dialect import/export remains not started.

