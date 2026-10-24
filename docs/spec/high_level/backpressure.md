# Channel Backpressure

Status: closed for Fase 9.

Backpressure is explicit. Fase 9 rejects plain `send` so source code cannot hide whether a producer blocks or handles a full channel.

Accepted forms:

```n
@ok >> bool : sendBlocking(@ch, 3);
@attempt >> Result<bool, str> : trySend(@ch, 3);
```

Rejected form:

```n
send(@ch, 3);
```

The rejected form emits `E_HIGH_CHANNEL_BACKPRESSURE_001`.
