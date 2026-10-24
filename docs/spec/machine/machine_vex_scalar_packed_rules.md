# Machine VEX Scalar And Packed Rules

Phase 1D adds native encoding for packed double and scalar single/double family segments.

Packed forms:

- Packed double forms may use `xmm` or `ymm` registers according to the operand matrix.
- The vector width sets the VEX `L` bit.
- Memory forms are checked for the expected vector memory width before byte emission.

Scalar forms:

- Scalar single and scalar double closed forms use `xmm` registers.
- Scalar forms reject `ymm` operands with `E_MACHINE_REGISTER_WIDTH_001`.
- Scalar memory operands require the scalar memory width declared by the operand matrix.
- The verifier runs before encoding and blocks invalid width combinations.

Common encoding contract:

- The encoder emits VEX3 prefixes for closed Phase 1D family forms.
- `vvvv` encodes the first source register.
- ModRM encodes the destination and second source or memory operand.
- Immediate bytes are emitted only for forms that declare `imm8`.
- No textual assembler export is used as a substitute for byte emission.

Phase 1D increases closed encoding coverage but does not close the entire VEX phase. Remaining deferred forms are tracked in the Phase 1D reports.
