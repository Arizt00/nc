# High Level Match Guards

Status: closed limited in Fase 6.

Match guards are allowed for pure boolean-like expressions.

```n
match (@load) {
    Load.Value(v) if @v > 0 => return @v;
    Load.Value(v) => return 0;
    Load.Empty => return 0;
}
```

Guard rules:

- Guard expression must be bool-like.
- Guard expression may use pattern bindings.
- Guards may call typed functions. Their result must be `bool`, and normal function effect checking
  still applies at the call site.
- Guarded arms do not count toward exhaustive coverage because they are conditional.

This keeps Fase 6 deterministic and avoids introducing the Fase 7+ effect/error algebra early.
