# Machine Directives Overview

Phase 10 closes official nC machine directives for data, reserve, alignment, origin policy, constants, include, incbin, conditional assembly, and bounded repetition.

Directives lower to explicit MachineIR records and object sections. Data and incbin bytes are hashed, BSS reservations are sized, and object emission publishes data symbols so disasm and roundtrip can observe them.

ASM dialect import/export is not part of this phase. The directive contract is nC-native.

