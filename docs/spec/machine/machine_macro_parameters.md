# Machine Macro Parameters

Macro parameters are typed at declaration time. Phase 10 supports register classes, vector/opmask classes, immediates, memory-like operands, labels, symbols, sections, defaults, and variadic capture.

Argument checks run during expansion. Wrong arity uses `E_MACHINE_MACRO_ARG_001`, wrong type uses `E_MACHINE_MACRO_PARAM_001`, and invalid variadic use uses `E_MACHINE_MACRO_VARIADIC_001`.

The verifier receives only expanded instructions plus origin metadata, so parameterized macros cannot bypass operand, feature, or encoding gates.

