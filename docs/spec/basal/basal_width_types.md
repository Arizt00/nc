# Basal Width Types

Basal executable source uses explicit real-width types:

- `byte`: 8-bit unsigned.
- `word`: 16-bit unsigned.
- `dword`: 32-bit unsigned.
- `qword`: 64-bit unsigned.
- `sbyte`: 8-bit signed.
- `sword`: 16-bit signed.
- `sdword`: 32-bit signed.
- `sqword`: 64-bit signed.
- `addr`: target/profile address width.
- `bool`: official basal boolean.
- `char`: explicit character scalar.
- `str`: string with documented encoding policy.
- `bytes<N>`: fixed byte array.

New basal examples avoid ambiguous `int`. If it appears in executable surface, `E_BASAL_INT_AMBIGUOUS_001` is emitted.

Bitwise operators operate over fixed width. `>>>` is logical right shift. Overflow policy is explicit-width checked for constants and lowering-profile-owned for raw runtime effects.
