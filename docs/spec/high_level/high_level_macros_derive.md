# Derive macros

Derive macros generate trait implementation records from declared type metadata. The syntax records the derive target and trait name in the macro declaration.

```n
macro:
    derive Printable for Data {
        generate impl Printable;
    }
```

Fase 14 does not perform private field disclosure. Public metadata can participate in derive generation; private metadata access requires the unsafe macro lane and explicit `private.metadata` authority.

Derived output is checked against symbol collision and generated AST rules. Any leak of `Any` through public generated output emits `E_HIGH_MACRO_ANY_LEAK_001`.
