# nC Syntax

The public `.n` surface is compact, sectioned and manual/raw. The compiler
parses, checks types, preserves semantics during optimization and materializes
bytes. It does not add hidden ownership, cleanup or scheduling.

## Imports

```nC
use <io:input, output, render>;
use <memory:region, heap>;
```

`<...>` selects the library, `io` identifies the module, `:` introduces its
items and `,` separates them. Expanded imports are compatibility representations.

## Containers and sections

```nC
counter {
    const:
        LIMIT >> int <- 100;

    type:
        Sample {
            public:
                value >> int;
            layout:
                repr(C);
        }

    stc:
        calls >> u64 <- 0;

    local:
        scratch >> int <- 0;

    fn:
        private:
            clamp(value >> int) -> int {
                if (value > LIMIT) { return LIMIT; }
                return value;
            }

        public:
            run() -> bool {
                calls += 1;
                return clamp(7) == 7;
            }

    ref main ->> run;

    fa:
        return true;
}
```

An ordinary container is named without a role suffix. `:: kernel`, `:: machine`,
`:: primitive`, `:: runtime` and other roles select distinct material contracts.
Only one `stc:` section is allowed. `run` has no special status; `ref main`
selects it as the entry point.

Visibility is part of the material symbol:

- `public`: exported API; LLVM uses default visibility and the object exposes a global symbol.
- `internal`: visible across package units; LLVM uses `hidden` while the object retains a linkable global symbol.
- `private`: visible only within its container; LLVM uses `private` and the object contains a local symbol.

The same rules apply to functions, data and fields where the contract admits
visibility. They do not introduce hidden dispatch or wrappers.

Parameters use `name >> type`. In `constructure:`, the canonical form is
`static(...) -> Type;`. `nfmt` migrates `static :: in(...)` and
`static :: fn(...)` without changing the typed contract.

For a basic entry point of up to 30 logical statements, use:

```nC
hello {
    to main { println("Hello"); }
    fa:
        return true;
}
```

## Types and memory

`>> T <- value` declares a typed place and writes its initial bits. The same
arrow writes existing places. `=` remains a historical compatibility form
that `nfmt` converts to `<-`; `:=` has been removed. A declaration without
an initializer does not promise initialized bytes.

```nC
raw >> byte* <- cast(alloc(64));
numbers >> int* <- cast(alloc(8 * sizeOf(int)));
wr(numbers, 7);
wr(off(numbers, 1), 11);
value >> int <- rd(numbers);
free(numbers);
```

`T*`, `p<T>` and `ptr<T>` reach the same typed AST. Raw semantics permit null,
misalignment, aliasing, overflow, out-of-bounds access, use-after-free and
double free. Compilation does not guarantee stable results for undefined behavior.

## Strings

```nC
message >> str <- "result: " + (ok ? "true" : "false");
println(message);
```

`str + str` lowers to the native concatenation operation. The resulting pointer
remains under manual program control. `cat(dst, cap, left, right)` accepts
caller-provided storage when address and capacity must be controlled.

## ABI and layout

Contracts for `repr(C)`, `packed`, `align`, unions, bitfields, flexible arrays,
TLS, per-CPU data, varargs, aggregate returns and function pointers are retained
through AST, IR, machine, object and linking representations. `abi` and `bind`
remain available to specify registers, stack, preserved state and returns.

## Control flow

```nC
if (ready) { consume(); }
else if (retry) { recover(); }
else { stop(); }

while (active) { step(); }
guard valid else return false;
```

`&&`, `||`, `!`, comparisons, arithmetic, compound writes and the conditional
operator preserve source evaluation order.

## Formatting

`nfmt --compact` preserves comments and `const/type/stc/local/fn/fa` sections.
It retains visibility and grouped functions. `state {}` migrates to `stc:`;
the generated `shutdown` final migrates to `fa:`.
