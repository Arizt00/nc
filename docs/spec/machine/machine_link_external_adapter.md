# Machine Link External Adapter

Phase 12 defines an external linker adapter lane but does not use external tools as the closure authority. The adapter reports environment, command, arguments, stdout/stderr, exit code, artifact path, hash, and blocked-by-environment status when used.

Phase 13 external compatibility validation is now closed for declared profiles.
External tools remain compatibility readers; they do not replace the nC verifier,
disassembler, roundtrip engine or linker as authority.
