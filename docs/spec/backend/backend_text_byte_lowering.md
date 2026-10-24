# Backend text byte lowering

Status: closed for UTF-8 literal text lowering.

This contract defines how nC lowers every source text scalar to concrete backend bytes.
The official encoding is UTF-8. A source string is not treated as an abstract host
string once it reaches IR/backend lowering: it becomes `TextLiteralBytesIR`.

## Contract

For every string literal, nC records:

- source text;
- encoding: `Utf8`;
- scalar count;
- payload byte length;
- nul-terminated byte length;
- payload bytes;
- payload bytes plus `0x00` terminator for hosted C-compatible string arguments;
- one `TextByteUnitIR` per Unicode scalar.

Each text byte unit records:

- character index;
- byte offset;
- byte length;
- scalar text;
- Unicode codepoint;
- functional role;
- human function;
- backend byte action;
- exact UTF-8 bytes;
- uppercase hexadecimal byte view.

Example:

```text
"Añe🙂"
```

lowers to:

```text
A  -> 41
ñ  -> C3 B1
e  -> 65
🙂 -> F0 9F 99 82
NUL terminator for C-compatible literals -> 00
```

## Source escapes

The lexer decodes source string escapes before byte lowering. Supported escapes
are:

- `\n`, `\r`, `\t`, `\0`;
- `\"`, `\'`, `\\`;
- `\xNN`, interpreted as Unicode scalar `U+00NN` before UTF-8 lowering;
- `\u{H...}`, interpreted as a Unicode scalar value with one to six hex digits.

Invalid escapes fail during lexing instead of being silently preserved as literal
backslashes. After decoding, `TextLiteralBytesIR` lowers the resulting scalar
stream to concrete UTF-8 bytes. For example:

```text
"A\n\u{1F642}"
```

lowers to payload bytes:

```text
41 0A F0 9F 99 82
```

## Backend use

Native object emission uses the same lowering for `.rodata` string literals. The
payload is emitted into read-only data with a final NUL byte. Native-fast call
arguments receive a local `{ data_ptr, byte_len }` descriptor in `.rodata.<function>`:
text code receives an `Abs64` relocation to the descriptor symbol, and the descriptor
receives an `Abs64` relocation to the local payload symbol. The recorded byte length
does not include the final NUL.

LLVM lowering uses the same byte lowering for string literals and `print`/`println`
literal output. Literal `print` writes the exact payload bytes. `println` appends
one newline byte to the payload before the hosted `write` call. The printed payload
does not include a NUL terminator.

## CLI inspection

The official inspection surface is:

```bash
ncc text-bytes <file.n> --json
```

The report lists every source string literal found by the lexer, including its
line, column, payload bytes, NUL-terminated bytes and per-scalar byte units. This
is intentionally earlier than semantic lowering, so programmers can inspect text
bytes while editing source.

## Boundary

This phase does not change nC string semantics or add a new text encoding. It makes
the existing UTF-8 behavior explicit, testable, and shared by native object and LLVM
lowering.

The functional role catalog is specified in
[`backend_character_byte_catalog.md`](backend_character_byte_catalog.md).
