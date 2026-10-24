# Runtime Interactive Runner

`ncc run` preserves console ordering for interactive programs.

## Behavior
- stdout is flushed before every runtime `read*`.
- explicit `flush()` is honored.
- `println` flushes after writing a newline.
- piped stdin is echoed after prompt visibility, matching the useful part of terminal echo for captured transcripts.

## Report Shape
`ncc runtime report <file> --json` includes:

```json
{
  "console_surface": {
    "print_calls": 0,
    "println_calls": 0,
    "flush_calls": 0,
    "read_calls": 0,
    "prompt_patterns": 0,
    "prompt_order_risk": 0,
    "interactive_required": true,
    "stdout_flush_supported": true,
    "prompt_order_guaranteed": true
  }
}
```

## Non-goals
This runner does not provide terminal editing, history, cursor control, or asynchronous TTY APIs. It guarantees prompt/input ordering for the runtime console surface.
