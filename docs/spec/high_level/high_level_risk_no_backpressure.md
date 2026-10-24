# High Level Risk No Backpressure

No-backpressure authority is split by stream and channel surfaces.

Rules:

- `accept streamNoBackpressure` requires `allow streamNoBackpressure`.
- `accept channelNoBackpressure` requires `allow channelNoBackpressure`.
- `accept noBackpressure` requires both capabilities.

Example:

```n
risk:
    allow streamNoBackpressure;
    allow channelNoBackpressure;
    accept noBackpressure "combinedDataPlane";
```

Diagnostics:

- `E_HIGH_RISK_STREAM_BACKPRESSURE_001`
- `E_HIGH_RISK_CHANNEL_BACKPRESSURE_001`

The report marks these risks as nondeterministic capacity hazards.
