# Machine Macro Directive IR

Phase 10 extends MachineIR with macro definitions, macro invocations, macro expansions, directive records, conditionals, repeats, includes, incbins, constants, data bytes, and BSS size.

The verifier sees both the final expanded instruction stream and enough origin metadata for reports. This means macros and directives cannot bypass feature gates, register checks, object emission, or roundtrip comparison.

