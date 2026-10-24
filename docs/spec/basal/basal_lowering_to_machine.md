# Basal Lowering To Machine

Lowering maps verified BasalIR into a machine-facing plan without executing final emission.

Root lowering records:

- root id and kind;
- target machine profile;
- entry symbol if declared;
- source span;
- lowering status;
- pending evidence status.

Site lowering records:

- site id and kind;
- size and alignment;
- normalized permissions;
- target section selected by site kind;
- source span;
- lowering status.

Bind lowering records:

- source and destination;
- local resolution status;
- machine symbol resolution status;
- target triple/profile when present;
- relocation intent for machine symbols;
- pending evidence boundary;
- source and destination spans.

The machine backend remains sovereign. Phase 4 only proves the path from basal semantics to machine references.
