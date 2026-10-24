# Machine EH Frame

ELF64 unwind emission writes `.eh_frame` with a CIE and FDE for the emitted machine procedure range. The CIE records code alignment, data alignment, return-address register, and initial CFA rules. The FDE records a non-empty PC range and CFI instruction bytes derived from the machine `cfi:` section.

The verifier rejects frame metadata without CFI, contradictory frame/CFI declarations, and empty FDE ranges. `machine unwind-info` reports `.eh_frame` section hashes and `machine verify-object` exposes the debug/unwind summary.
