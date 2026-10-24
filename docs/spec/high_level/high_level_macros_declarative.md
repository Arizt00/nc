# Declarative macros

Declarative macros generate items from typed macro parameters. The current surface supports `ident`, `type`, `expr`, `str`, and `item` parameters. Arguments are checked at invocation time; a mismatch emits `E_HIGH_MACRO_PATTERN_001`.

```n
macro:
    declarative makeValue(name: ident, value: expr) {
        generate fn name -> int = value;
    }
    expand makeValue(answer, 42);
```

Generated names are not inserted silently. They are recorded in the macro expansion report, checked against existing symbols, and assigned hygiene marks. Exported generated symbols must be declared through the macro hygiene policy when the declaration intentionally publishes them.

Declarative macros remain compile-time only. They do not execute runtime code and do not access private metadata unless they are declared through the unsafe macro lane with the proper authority.
