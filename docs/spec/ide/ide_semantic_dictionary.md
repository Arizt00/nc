# nC IDE Semantic Dictionary

Status: closed for editor-assist hover and go-to-definition.

This dictionary is shared by the VS Code and JetBrains plugins through `nc_intellisense_catalog.json`. It does not replace `ncc`, does not invent semantics, and does not authorize machine, basal, primitive, or kernel behavior. It explains tokens and routes the user to the relevant official docs while compiler diagnostics remain authoritative.

## Covered Families

- Numeric and text types: `int`, `bool`, `str`, `string`, `byte`, `word`, `dword`, `qword`, `signed`, `unsigned`.
- Collection types: `array<T, N>`, `list<T>`, `queue<T>`, `dinArray<T>`.
- Pointer and memory types: `addr`, `ptr<T>`, `ptr mut<T>`, `ref<T>`, `ref mut<T>`, `site<T>`, `memRegion<T>`, `slice<T>`, `span<T>`.
- Modes: `[@runtime]`, `[@compiler]`, basal `[@]`, `[@machine]`, `primitive[@]`, future `[@kernel]`, and common authority/inspection modes.
- Operators: arithmetic, bitwise, shifts, routing arrows, namespace separators, references, and verification markers.
- Baseical surfaces: `type:`, `stc:`, `fn:`, `fa:`, `const:`, `routine:`, `raw`, `verify:`, and `report:`.
- Memory projections: `@x >> int : value`, `mem[site]`, and `mem[@addr]`.

## Hover

Hover must provide a human explanation long enough to teach the token in context. The explanation names the surface, the risk class, and the reason the construct exists. For dangerous constructs, the hover states that the editor only explains and `ncc` remains responsible for diagnostics, evidence, and lowering.

## Go To Definition

Go to definition has two levels:

- Local declarations resolve to their source position when the symbol is declared in the current file.
- Catalog entries resolve to official docs such as type, pointer, operator, basal, machine, primitive, imports, or this IDE dictionary document.

## Token Matching

The provider must resolve both word-like and symbolic tokens. Word-like examples include `int`, `memRegion`, and `routine`. Symbolic examples include `>>`, `>>>`, `::`, `!>`, `[@machine]`, `primitive[@]`, `mem[site]`, and `mem[@addr]`.

## Authority

The IDE assists. The compiler decides. Hover, completion, auto-import, and go-to-definition are editor assistance surfaces only. They must never silently accept invalid imports, forbidden modes, fake evidence, fake lowering, or kernel/machine authority through ordinary high-level code.
