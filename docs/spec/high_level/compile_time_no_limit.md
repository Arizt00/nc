# Compile-time noLimit

`noLimit` is forbidden in pure const eval. It belongs only to `comptime unsafe noLimit:` and only when the block explicitly allows `unboundedLoops` or `unboundedRecursion`.

```n
comptime unsafe noLimit:
    noLimitAudit {
        allow unboundedLoops;
        allow unboundedRecursion;
        unbounded.loop("declared");
        unbounded.recursion("declared");
    }
```

Using unbounded authority without `noLimit` fails. Using `noLimit` without a matching unbounded capability also fails. A passing `noLimit` block marks cache disabled and records the hazard in the audit report.
