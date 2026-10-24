# Machine Debug And Unwind

Phase 5 closes the debug and unwind lane for machine object emission without changing the machine base or advancing object-aware disassembly. The closed lane covers ELF64 DWARF sections, ELF64 `.eh_frame`, COFF `.pdata`/`.xdata`, Mach-O compact unwind with DWARF debug sections, and WASM name/custom debug sections.

The machine verifier connects debug and unwind declarations to `target`, `abi`, `object`, `proc`, `frame`, `stack`, `cfi`, `emit`, `verify`, and `seal`. Debug metadata is rejected when the source file or line range is missing or invalid. Unwind metadata is rejected when a declared frame has no CFI, when CFI contradicts the frame model, or when format-specific ranges and policies are invalid.

Phase 5 closed before object-aware disassembly, roundtrip and cross-architecture
parity were implemented. Those later lanes are now closed by Phase 6, Phase 7
and Phase 8 for the declared machine profiles.
