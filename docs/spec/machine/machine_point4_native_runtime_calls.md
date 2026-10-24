# Machine Point 4 - Native runtime call handoff

Status: implemented for the machine/linker handoff required by Backend Point 4.

Machine does not own high-level runtime semantics. It owns the final material
boundary: imports, thunks, relocations, object linking and executable loader
metadata.

## Closed

- Backend object unresolved symbols are classified using catalog-derived runtime
  admission instead of a handwritten runtime name list.
- Phase12/internal OS linkers receive a complete runtime-symbol view for
  official runtime calls and backend bridge symbols.
- PE64 keeps real import thunk materialization for supported OS-backed runtime
  calls such as:
  - `printf`/output thunks through console writes;
  - `readInt` through console input;
  - `fswrite` through `CreateFileA`, `WriteFile`, `CloseHandle`;
  - `openUri` through `ShellExecuteA`;
  - media keys through `keybd_event`;
  - `sleep` through `Sleep`.
- ELF/Mach-O/COFF reports can now identify runtime imports using the same
  catalog-derived admission surface.
- Host C/OS symbols used by MCT remain admitted even when they are lower-level
  than the high-level runtime name, for example `write`, `strlen`, `memcpy`,
  `usleep` and `printf`.

## ISA impact

No new instruction encoding family was required for Point 4. The change is in
symbol authority and linker admission:

- call instructions already exist for the supported machine profiles;
- import thunks and dynamic metadata from previous points remain the material
  path;
- runtime-call completeness now depends on catalog-derived symbol coverage, not
  on adding more opcode forms.

## Machine truth boundary

Machine can prove that calls and imports are materialized. It does not prove
that every runtime family implementation is internally optimized or scheduler
complete; that belongs to the Runtime phase.

The machine invariant is:

```text
official runtime symbol -> admitted backend symbol -> object unresolved/import
                         -> internal/external linker materialization
```

Catalog-only symbols are excluded from this invariant until their runtime
backing is implemented.
