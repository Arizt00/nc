# Machine VEX Valid And Invalid Forms

Una forma VEX valida describe una combinacion aceptada de operandos. Una forma invalida describe una familia de errores que el verifier debe rechazar con diagnostic baseico.

## Valid Forms

Las formas validas se expresan como patrones:

- `reg,reg`
- `reg,mem`
- `mem,reg`
- `reg,reg,reg`
- `reg,reg,mem`
- `reg,reg,imm8`
- `mem,reg,imm8`
- `reg,reg,reg,imm8`
- `reg,reg,mem,imm8`
- `zero_operand`

En esta notacion `reg` significa registro vectorial VEX (`xmm` o `ymm`) salvo que una instruccion declare otra clase en una fase posterior. `mem` siempre debe tener anchura permitida por la entrada.

## Invalid Forms

Las invalid forms principales son:

- `operand_count_mismatch` -> `E_MACHINE_INSTR_OPERAND_001` o `E_MACHINE_VEX_VVVV_001`
- `scalar_register_where_vector_required` -> `E_MACHINE_REGISTER_CLASS_001`
- `xmm_ymm_width_mix` -> `E_MACHINE_REGISTER_WIDTH_001`
- `bad_memory_width` -> `E_MACHINE_MEM_WIDTH_001`
- `unexpected_immediate` -> `E_MACHINE_IMM_WIDTH_001`
- `immediate_width_mismatch` -> `E_MACHINE_IMM_WIDTH_001`
- `missing_feature_gate` -> `E_MACHINE_FEATURE_REQUIRED_001`
- `unsupported_by_design` -> `E_MACHINE_VEX_UNSUPPORTED_BY_DESIGN_001`

Fase 1B exige que todas las entradas del manifest tengan al menos una forma valida y una familia de invalid forms.
