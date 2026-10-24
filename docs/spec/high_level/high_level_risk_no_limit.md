# High Level Risk No Limit

`noLimit` is explicit authority for disabling ordinary boundedness expectations in a declared risk context.

Accepted loop risk:

```n
risk:
    allow noLimit;
    allow unboundedLoop;
    accept unboundedLoop "operatorBoundedByInputContract";
```

Accepted recursion risk:

```n
risk:
    allow noLimit;
    allow unboundedRecursion;
    accept unboundedRecursion "compileTimeTreeDepth";
```

Diagnostics:

- `E_HIGH_RISK_NOLIMIT_001`
- `E_HIGH_RISK_UNBOUNDED_LOOP_001`
- `E_HIGH_RISK_UNBOUNDED_RECURSION_001`

The report marks no-limit authority as cache-disabling and termination-not-guaranteed.
