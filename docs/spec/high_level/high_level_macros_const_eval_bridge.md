# Macro const-eval bridge

Macros can request pure compile-time evaluation through `constEval expr;`. This bridge reuses the Fase 13 const-eval engine and inherits its guarantees:

- pure.
- deterministic.
- bounded.
- cacheable.
- no filesystem, process, system, FFI, machine, heap mutation, reflection mutation, dynamic eval, private metadata, construction leak, or runtime call.

Bridge failures use macro diagnostics:

- `E_HIGH_MACRO_CONST_EVAL_FORBIDDEN_EFFECT_001` for forbidden effects.
- `E_HIGH_MACRO_CONST_EVAL_VISIBILITY_001` for visibility or private metadata violations.
- `E_HIGH_MACRO_CONST_EVAL_FAILED_001` for other rejected const-eval expressions.

The bridge does not create a separate evaluator. It is a macro-facing entry point into the closed Fase 13 evaluator.
