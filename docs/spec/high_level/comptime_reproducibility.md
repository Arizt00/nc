# Comptime Reproducibility

Fase 13 classifies comptime unsafe blocks by reproducibility impact.

`DeterministicCacheable` means the block did not record non-deterministic or cache-disabled effects. `NonDeterministicCacheDisabled` means at least one effect depends on host state, writes state, spawns a process, reads environment, uses dynamic eval authority, records a construction leak, or uses `noLimit`.

The reproducibility classification is part of the audit result. It is not evidence for macros or later compile-time code generation; it is the boundary that allows those later features to refuse unsafe inputs unless explicitly accepted.
