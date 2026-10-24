# Runtime Prompt Contract

A prompt is output that must become visible before input is consumed.

## Baseical Pattern
```n
print("ID del usuario: ");
flush();
@userId >> int : readInt();
```

The contract is:

- `print` materializes prompt text.
- `flush` synchronizes stdout.
- `readInt` performs a defensive stdout flush before reading.
- Piped input is echoed after the prompt to retain terminal-like ordering in golden tests.

## Regression Guard
The sequence below is forbidden in positive runtime output:

```text
ID del usuario: Base del rectangulo: Altura del rectangulo:
```

The industrial test suite checks `rectanguloEnterpriseStress.n` with piped input and verifies that each prompt is followed by its corresponding input value before the next prompt appears.
