# Machine Macro Hygiene

Macro local labels are hygienic by default. Each invocation receives deterministic generated labels, and the expansion report maps generated labels back to the macro-local source name.

Non-hygienic mode is explicit and diagnosed when it collides. Duplicate macro-local labels in expanded code use `E_MACHINE_MACRO_LABEL_COLLISION_001`; escaping or missing generated labels use macro label diagnostics.

This keeps repeated macro invocations stable for object emission and roundtrip comparison.

