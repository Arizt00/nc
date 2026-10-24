# Compiler Extreme E2E Closure

## Objetivo
Cerrar `[@compiler]` como pipeline formal con fronteras estrictas de fase.

## Pipeline
`lexer -> parser -> AST -> sema -> resolver(type/operator/condition/import/flow) -> nIR -> verifier -> lowering -> backend`.

## Leyes
- Lexer no hace sema.
- Parser no inventa tipos.
- Backend no corrige sema.
- nIR inválida se rechaza.

## CLI
- `ncc compiler doctor`
- `ncc compiler pipeline <file>`
- `ncc compiler tokens|ast|sema|imports|types|operators|conditions|nir|lower|backend <file>`
- `ncc compiler diagnostics --json`
- `ncc compiler self-check`
- `ncc compiler ffi-smoke --json [--strict]`

## Related docs
- `docs/spec/compiler/compiler_ffi_real_link_run.md`
- `docs/spec/language/bounds_type_contracts.md`
