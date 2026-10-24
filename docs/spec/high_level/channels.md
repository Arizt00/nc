# High Level Channels

Status: closed-limited for Fase 9.

Channels are bounded: `channel<T,N>`, where `N` is a positive integer capacity. Unbounded channels are forbidden in Fase 9.

```n
@ch >> channel<int, 4> : channelCreate<int, 4>();
@sent >> bool : sendBlocking(@ch, 1);
@item >> Option<int> : recv(@ch);
```

Rules:

- `channel<T,N>` requires item type `T` and positive capacity `N`.
- `channel<any,N>` is forbidden.
- `channelCreate<T,N>()` creates a bounded channel value.
- plain `send` is rejected because backpressure must be explicit.
- `sendBlocking` returns `bool`.
- `trySend` returns `Result<bool,str>`.
- `recv` returns `Option<T>`.
- `tryRecv` returns `Result<Option<T>,str>`.
- `recvBlocking` returns `T`.

Diagnostics:

- `E_HIGH_CHANNEL_UNBOUNDED_001`.
- `E_HIGH_CHANNEL_CAPACITY_001`.
- `E_HIGH_CHANNEL_ITEM_ANY_001`.
- `E_HIGH_CHANNEL_BACKPRESSURE_001`.
- `E_HIGH_CHANNEL_SEND_001`.
- `E_HIGH_CHANNEL_RECV_001`.
