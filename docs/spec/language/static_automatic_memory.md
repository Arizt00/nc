# Static and automatic memory

## Source contract

nC normally expresses read-only static data with `immutable`; its physical
section is selected by lowering and is not part of the ordinary source contract:

```n
data align(64) counter >> int = 7;
bss zeroes >> array<byte, 4096>;
immutable tag >> array<byte, 4> <- [1, 2, 3, 4];
tls align(64) errno >> int = 0;
percpu bss ticks >> u64;
weak data hook >> p<void> = null;
common scratch >> array<byte, 256>;
```

When working directly on a concrete lowering component, the source must name
that capability explicitly:

```n
rodata.ncc;

directLowering {
    stc:
        rodata section(".rodata.nc.contract") tag >> array<byte, 4> <- [1, 2, 3, 4];
        comdat("shared.config") rodata config >> int <- 42;
}
```

Without `rodata.ncc;`, `rodata` and `readonly` are rejected.
This keeps native section spelling out of normal application code while
retaining deliberate compiler and backend experiments.

The same declaration can expose the compiler-bundled `rodataNcc` API to a
single-file program. Injection is lazy: only a source that both declares
`rodata.ncc;` and references `rodataNcc` receives the embedded nC record-pack
reader and lexicon. This is not project resolution and does not require a
second source or data file beside the program.

The qualifiers are composable and order-independent. `align(N)` requires a
non-zero power of two. `section("name")` preserves the requested object-section
name; Mach-O may use the explicit `"__SEGMENT,__section"` form.

`static` inside a function declares one object for the lifetime of the module.
Its initializer must be object-materializable and constant. Dynamic work is
written explicitly in a `module.init` function, so no hidden guard variable,
lock or allocation is introduced.

## Duration and placement

| Surface | Material storage |
| --- | --- |
| initialized static | `.data` |
| zero-fill static | `.bss` / NOBITS / zerofill |
| `immutable` static | lowering-selected read-only segment |
| `tls` initialized/zero | `.tdata` / `.tbss` and native TLS flags |
| `percpu` initialized/zero | `.data..percpu` / `.bss..percpu` |
| custom section | exact requested section contract |

TLS is allocated by the hosted loader and thread runtime. Per-CPU storage is a
kernel/loader contract: the image contains the template; the kernel chooses
how many copies exist and binds the architecture's current-CPU base register.

## Linkage

`weak` emits a weak definition. `common` emits tentative zero-fill storage.
`comdat("group")` emits link-once/COMDAT storage. ELF uses `SHT_GROUP`, COFF uses
COMDAT selection-any, and Mach-O uses a coalesced section plus weak definition.
These forms allow a real linker to coalesce or discard unreferenced members;
entry points, exported symbols and lifecycle arrays remain roots.

## Module lifecycle

```n
initialize() -> void effects(module.init) { }
finalize() -> void effects(module.fini) { }
```

Lifecycle functions are synchronous, monomorphic, parameterless and return
`void`. The backend emits `.init_array`/`.fini_array`, `.CRT$XCU`/`.CRT$XTU`, or
`__mod_init_func`/`__mod_term_func`. Constructors retain source order;
destructors run in reverse lifecycle order through the platform loader.

## Automatic and dynamic stack

```n
mark >> p<byte> = stackMark();
items >> p<int> = vla<int>(count);
raw >> p<byte> = stackAlloc(bytes, 64);
stackRestore(mark);
```

`vla<T>(count)` computes `count * sizeOf<T>()`, aligns the allocation to `T`,
and returns raw stack storage. `stackAlloc(bytes, align)` accepts an explicit
runtime alignment. `stackRestore` invalidates every pointer into allocations
made after the matching mark. No heap fallback or cleanup is inserted.

Large hosted frames use the target probe contract (`__chkstk` on Windows and
page touching on hosted Unix). Guard pages are owned by the OS thread stack;
kernel and firmware profiles must install them in their loader/page-table
contract. Unwind metadata follows DWARF CFI, Windows `.pdata/.xdata`, Mach-O
compact-unwind/DWARF, or ARM EHABI according to the target.
