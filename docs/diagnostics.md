# Diagnostics

Public diagnostics should use this shape:

```text
[33:21] Expected Semicolon
33 |     println("hola")
                         ^
```

Messages must point at the real user file and avoid private generated paths when possible.
