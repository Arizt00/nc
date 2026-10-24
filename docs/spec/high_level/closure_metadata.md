# Closure Metadata

`closureInfo(value)` returns limited closure metadata for values with `fn(...) -> ...` type.

It may expose:

- parameter count and types;
- return type;
- declared effects;
- capture count and capture modes when safe.

It must not expose captured values or private captured data. Captured values are runtime state, not public metadata.
