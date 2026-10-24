# Machine Mach-O Unwind

Mach-O emission preserves debug/unwind section names for machine metadata. Phase 5 writes `__compact_unwind` and DWARF debug sections in `__DWARF`.

The verifier requires an explicit Mach-O unwind policy. Valid policies are compact unwind and DWARF fallback. Missing policy produces `E_MACHINE_MACHO_UNWIND_POLICY_001`.

This is a format-correct unwind/debug lane for object metadata. Object-aware disassembly remains a later phase.
