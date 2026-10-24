# Basal Routine Surface

`routine:` is the official basal routine surface. It is exclusive to `[@]` and represents a root routine that can be planned for lowering to `[@machine]`.

Baseical form:

```nb
routine:
    copyDword(src: addr, dst: addr) -> bool {
        raw {
            value >> dword : mem[@src];
            mem[@dst] >> dword : @value;
        }
        return true;
    }
```

Routine parameters and returns use explicit basal width types. A routine is not a high-level function and does not call runtime output APIs. BasalIR records params, return type, ABI profile, raw blocks, memory accesses, spans, and lowering status. The lowering plan records `routine` surface records with pending evidence boundaries.

`fn:` is not official basal executable syntax. Phase 5.5 rejects it with `E_BASAL_FN_LEGACY_001` so migration is visible.
