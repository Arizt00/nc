# Backend character byte catalog

Status: closed for UTF-8 scalar classification and byte lowering.

This document defines how nC treats characters that appear inside source text
literals. The backend lowers every Unicode scalar to real UTF-8 bytes through
`TextLiteralBytesIR`. Each scalar also receives a functional role so diagnostics,
debug reports and byte inspection can explain what the character is doing.

The rule is simple:

- outside a string literal, characters may be grammar syntax;
- inside a string literal, characters are payload;
- escaped source text is decoded first;
- every payload scalar is emitted as exact UTF-8 bytes;
- string literals used by hosted C-compatible lowering receive an extra final
  `0x00` terminator outside the payload.

## Baseical unit record

Every scalar produces one `TextByteUnitIR` record:

| Field | Meaning |
|---|---|
| `char_index` | Scalar index in the decoded string. |
| `byte_offset` | UTF-8 payload byte offset. |
| `byte_len` | Number of emitted UTF-8 bytes. |
| `scalar` | The decoded scalar as text. |
| `codepoint` | Unicode scalar value, for example `U+1F642`. |
| `role` | Functional character role. |
| `function` | Human description of the role. |
| `backend_action` | Backend byte action. |
| `bytes` | Concrete emitted bytes. |
| `hex` | Uppercase hex view of the bytes. |

## Source escapes

Escapes are decoded before role classification:

| Source | Decoded scalar | Role | Bytes |
|---|---:|---|---|
| `\n` | newline | `line_break` | `0A` |
| `\r` | carriage return | `line_break` | `0D` |
| `\t` | tab | `tab` | `09` |
| `\0` | NUL | `nul` | `00` |
| `\"` | quote | `quote` | `22` |
| `\'` | apostrophe | `quote` | `27` |
| `\\` | backslash | `backslash` | `5C` |
| `\x41` | `A` | `ascii_letter` | `41` |
| `\u{1F642}` | `🙂` | `emoji_or_pictograph` | `F0 9F 99 82` |

`\xNN` means Unicode scalar `U+00NN`, then UTF-8 lowering. Raw arbitrary byte
strings are a separate future surface; normal `str` remains valid Unicode text.

## Functional roles

| Role | Character family | Function inside strings | Backend action |
|---|---|---|---|
| `nul` | `\0` | Payload NUL scalar. It is not the automatic terminator; it is real data. | `emit_payload_byte_00` |
| `line_break` | `\n`, `\r` | Encoded line separation in text payload. | `emit_utf8_payload_bytes` |
| `tab` | `\t` | Horizontal alignment in text payload. | `emit_utf8_payload_bytes` |
| `space` | ASCII space | Visible word separation. | `emit_utf8_payload_bytes` |
| `whitespace` | Other Unicode whitespace | Preserved text separation. | `emit_utf8_payload_bytes` |
| `control` | Other valid control scalars | Preserved control payload if represented as Unicode scalar. | `emit_utf8_payload_bytes` |
| `ascii_letter` | `A-Z`, `a-z` | Basic Latin text. | `emit_utf8_payload_bytes` |
| `ascii_digit` | `0-9` | Basic decimal digit text. | `emit_utf8_payload_bytes` |
| `non_ascii_letter` | `ñ`, `ü`, `λ`, `Ж`, `中`, etc. | International alphabetic text. | `emit_utf8_payload_bytes` |
| `non_ascii_number` | Unicode numeric scalars | Non-ASCII numeric text. | `emit_utf8_payload_bytes` |
| `quote` | `"`, `'`, `«`, `»`, smart quotes | Delimiter outside strings; payload after escape or direct Unicode quote inside strings. | `emit_utf8_payload_bytes` |
| `backslash` | `\` | Escape introducer in source; payload after decoding. | `emit_utf8_payload_bytes` |
| `bracket` | `(`, `)`, `[`, `]`, `{`, `}`, `<`, `>` | Delimiter outside strings; payload inside strings. | `emit_utf8_payload_bytes` |
| `separator` | `:`, `;`, `,`, `.`, `¿`, `¡` | Grammar separator outside strings; textual punctuation inside strings. | `emit_utf8_payload_bytes` |
| `operator_marker` | `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `~`, `!`, `=`, `?`, `@`, `#`, backtick | Operator or reference syntax outside strings; payload marker inside strings. | `emit_utf8_payload_bytes` |
| `currency_symbol` | `$`, `€`, `£`, `¥`, `₿`, Unicode currency block | Monetary marker text. | `emit_utf8_payload_bytes` |
| `math_symbol` | `≤`, `≥`, `≠`, `∞`, `√`, `∑`, `π`, Unicode math block | Mathematical marker text. | `emit_utf8_payload_bytes` |
| `arrow_symbol` | `→`, `←`, `↔`, `⇒`, Unicode arrow blocks | Direction or flow marker text. | `emit_utf8_payload_bytes` |
| `combining_mark` | `U+0300..U+036F` and related mark ranges | Modifies the preceding scalar, for example `e` + combining acute. | `emit_utf8_payload_bytes` |
| `emoji_or_pictograph` | `🙂`, `🚀`, `⚙`, pictographic ranges | Pictographic text payload. | `emit_utf8_payload_bytes` |
| `punctuation` | Other ASCII punctuation | Punctuation payload. | `emit_utf8_payload_bytes` |
| `unicode_symbol` | Any other valid Unicode scalar | General text symbol. | `emit_utf8_payload_bytes` |

## ASCII punctuation quick map

| Character | Role | Outside string | Inside string |
|---:|---|---|---|
| `@` | `operator_marker` | nC reference/context marker. | UTF-8 byte `40`. |
| `>` | `bracket` | Generic/operator component. | UTF-8 byte `3E`. |
| `<` | `bracket` | Generic/operator component. | UTF-8 byte `3C`. |
| `:` | `separator` | Type/value separator. | UTF-8 byte `3A`. |
| `;` | `separator` | Statement terminator. | UTF-8 byte `3B`. |
| `,` | `separator` | List separator. | UTF-8 byte `2C`. |
| `.` | `separator` | Path/member punctuation. | UTF-8 byte `2E`. |
| `"` | `quote` | String delimiter. | UTF-8 byte `22` when escaped as payload. |
| `'` | `quote` | Quote character. | UTF-8 byte `27`. |
| `\` | `backslash` | Escape introducer in strings. | UTF-8 byte `5C` when escaped as payload. |
| `+` | `operator_marker` | Addition/operator marker. | UTF-8 byte `2B`. |
| `-` | `operator_marker` | Subtraction/arrow component. | UTF-8 byte `2D`. |
| `*` | `operator_marker` | Multiplication/pointer marker. | UTF-8 byte `2A`. |
| `/` | `operator_marker` | Division/comment component. | UTF-8 byte `2F`. |
| `%` | `operator_marker` | Remainder/operator marker. | UTF-8 byte `25`. |
| `&` | `operator_marker` | Bitwise/reference-style marker. | UTF-8 byte `26`. |
| `|` | `operator_marker` | Bitwise/union-style marker. | UTF-8 byte `7C`. |
| `^` | `operator_marker` | Bitwise xor marker. | UTF-8 byte `5E`. |
| `~` | `operator_marker` | Bitwise not marker. | UTF-8 byte `7E`. |
| `!` | `operator_marker` | Negation/verification marker component. | UTF-8 byte `21`. |
| `=` | `operator_marker` | Equality/assignment-like marker. | UTF-8 byte `3D`. |
| `?` | `operator_marker` | Optional/unknown marker. | UTF-8 byte `3F`. |
| `#` | `operator_marker` | Tooling/tag marker if a surface uses it. | UTF-8 byte `23`. |
| `$` | `currency_symbol` | Reserved or ordinary symbol by surface. | UTF-8 byte `24`. |

## Backend contract

LLVM lowering, native `.rodata`, hosted print lowering and `ncc text-bytes` all
consume the same `TextLiteralBytesIR`. No backend may reinterpret text through
the host locale. If a character is valid Unicode, it is encoded as UTF-8 bytes
and surfaced with its role. If a future mode wants non-UTF-8 raw bytes, it must
use a dedicated byte literal or buffer surface rather than overloading `str`.
