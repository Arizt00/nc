# Macro hygiene

Fase 14 uses explicit hygiene records for each macro declaration and expansion. Generated symbols carry expansion marks derived from container, macro name, and generated item name. This allows diagnostics to distinguish intentional exports from accidental capture.

Rules:

- Implicit capture is rejected with `E_HIGH_MACRO_CAPTURE_001`.
- Explicit capture must be declared with `capture name;`.
- Generated symbol collisions emit `E_HIGH_MACRO_SYMBOL_COLLISION_001`.
- Private generated output leaks emit `E_HIGH_MACRO_PRIVATE_LEAK_001`.
- `Any` leaks emit `E_HIGH_MACRO_ANY_LEAK_001`.

Example:

```n
macro:
    declarative hygienic(name: ident) {
        generate fn temp -> int = 1;
        generate fn name -> int = temp;
    }
    expand hygienic(publicName);
```

The expansion report records hygiene status, private leak policy, any leak policy, and generated marks.
