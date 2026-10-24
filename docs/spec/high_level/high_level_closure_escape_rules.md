# High Level Fase 11 Closure Escape Rules

Status: closed. The Phase 11 examples remain valid.

Escape analysis in Fase 11 is conservative and explicit.

Allowed:

```n
make() -> fn() -> int {
    public:
        @owned >> int : 7;
        return move || -> int captures(move @owned) {
            return @owned;
        };
}
```

Rejected:

```n
make() -> fn() -> int {
    public:
        @local >> int : 7;
        return || -> int captures(ref @local) {
            return @local;
        };
}
```

The rejected form emits `E_HIGH_CLOSURE_RETURN_BORROW_001`. Async and generator closures apply the
same ownership rule while storing accepted captures in their persistent frame. Reflection-derived
implicit captures are not permitted.
