# High Level Numeric Types

Status: closed current scalar surface.

Baseical numeric families:

- integer default: `int` = `int32` / `sdword`;
- unsigned default: `uint` = `uint32` / `dword`;
- floating default: `ft` = `ft64`;
- unsigned width aliases: `byte`, `word`, `dword`, `qword`;
- signed width aliases: `sbyte`, `sword`, `sdword`, `sqword`;
- explicit signed widths: `int8`, `int16`, `int32`, `int64`;
- explicit unsigned widths: `uint8`, `uint16`, `uint32`, `uint64`;
- floating widths: `ft32`, `ft64`;
- scalar non-numeric values: `bool`, `char`, `void`;
- pointer/text scalar contracts: `addr`, `ptr<T>`, `ref<T>`, `site<T>`,
  `fn<...>`, `dyn<T>`, `str`, `string`, `text`, `bytes`.

Rules:

- integer literals infer as `int`;
- decimal literals infer as `ft`;
- `char` is a one-byte scalar literal and compares as an ordered scalar;
- implicit narrowing is rejected when a wider named width flows into a narrower named width;
- integer literal projection into a narrower integer width is accepted only when
  the literal value fits that width;
- bitwise operators require integer-like operands;
- shift operators require integer-like operands on both sides;
- same-family numeric widening is accepted by the current surface;
- same-width signed/unsigned identity remains documented as a policy boundary for later explicit-cast work.

## Closed Scalar Contract

The current compiler treats scalar identity as a shared contract, not as private
tables in parser, sema, runtime or backend:

| Surface names | Baseical contract | Range / storage intent |
| --- | --- | --- |
| `bool` | `bool` | logical byte-backed scalar |
| `char` | `char` | one byte, `0..255` |
| `byte`, `u8`, `uint8` | `byte` | unsigned 8-bit |
| `sbyte`, `i8`, `int8` | `sbyte` | signed 8-bit |
| `word`, `u16`, `uint16`, `ushort` | `word` | unsigned 16-bit |
| `sword`, `i16`, `int16`, `short` | `sword` | signed 16-bit |
| `dword`, `u32`, `uint32`, `uint` | `dword` | unsigned 32-bit |
| `sdword`, `i32`, `int32`, `int` | `sdword` | signed 32-bit |
| `qword`, `u64`, `uint64`, `ulong` | `qword` | unsigned 64-bit |
| `sqword`, `i64`, `int64`, `long` | `sqword` | signed 64-bit |
| `ft32`, `f32`, `float` | `ft32` | 32-bit float storage |
| `ft64`, `f64`, `double`, `ft` | `ft64` | 64-bit float storage |
| `addr`, pointer/reference families | pointer contract | target pointer width |
| `str`, `string`, `text`, `bytes` | text/bytes descriptor | pointer-width descriptor |
| `void`, `unit` | `void` | no value storage |

Sema rejects literal narrowing that cannot fit the destination scalar, for
example `@x >> byte : 256`. Runtime validates dynamic values against the same
contract, so `@x >> uint : -1` fails even when the value reaches execution.
`char` is intentionally byte-sized in this scalar layer; wider Unicode payloads
belong to text/string lowering, not to `char`.

Arithmetic overflow is an operator-level execution policy. Const-eval uses
checked integer arithmetic. Ordinary runtime/native `+` retains the source
mode's existing policy; nC does not silently turn every addition into a check.

The explicit import-free intrinsic
`checkedAdd<int64>(left, right) -> int64` closes signed 64-bit addition across
the current compiler stack:

- both operands must be `int64` or its signed 64-bit aliases;
- the exact mathematical sum is returned when it fits `int64`;
- overflow never wraps and never returns a truncated value;
- interpreter execution reports `E_RUNTIME_ARITHMETIC_OVERFLOW_001`;
- LLVM lowering uses `llvm.sadd.with.overflow.i64` and traps only on its flag;
- the x86-64 native scalar backend emits flag-producing `ADD`, `JNO`, then
  `UD2` on the overflow edge;
- AArch64 and other LLVM targets receive the target's equivalent checked-add
  sequence through the LLVM intrinsic.

This is deliberately a scalar-returning exact operation. A recoverable
`Result<int64, ArithmeticOverflow>` surface is not encoded through the current
single-word tagged `Result` representation because shifting a full-width
`int64` payload would discard a bit. That future surface requires a lossless
aggregate Result ABI first.

Examples:

```n
@a >> int32 : 7;
@b >> int64 : @a;
@small >> uint8 : 255;
@value >> int : 3 & 1;
@value >> int : @value >>> 1;
@letter >> char : 'A';
@left >> int64 : 9223372036854775800;
@right >> int64 : 7;
@exact >> int64 : checkedAdd<int64>(@left, @right);
```

Invalid examples:

```n
@small >> int32 : @source; // source: uint64
@tiny >> int8 : 128;
@small >> byte : 256;
@unsigned >> uint : -1;
@value >> int : true & false;
@value >> int : 1 << "bad";
@ratio >> ft32 : 1.5; // decimal literal is `ft`; explicit narrowing remains required
@wrong >> int : checkedAdd(1, 2); // checkedAdd v1 is the closed int64 surface
```
