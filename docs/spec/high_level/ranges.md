# High Level Ranges

Fase 8 supports finite integer ranges.

Forms:

```n
for i in 0..10 {
    ...
}

for i in 0..=10 step 2 {
    ...
}
```

Rules:

- bounds must be integer-like.
- `step`, when present, must be integer-like.
- literal step `0` is rejected.
- the loop item type is `int`.
- unbounded ranges and streams are deferred.
- floating-point ranges are not part of Fase 8.

Diagnostics:

- `E_HIGH_RANGE_001`
- `E_HIGH_RANGE_BOUND_001`
- `E_HIGH_RANGE_STEP_001`
- `E_HIGH_RANGE_STEP_ZERO_001`
- `E_HIGH_STREAM_UNBOUNDED_DEFERRED_001`
