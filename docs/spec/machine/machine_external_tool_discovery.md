# Machine External Tool Discovery

The discovery pass records tool name, path, version text, detection command, supported profiles, and environment status.

Missing tools are reported as `blocked_by_environment`. They are not counted as compatibility success.

Covered tool families include ELF readers, COFF readers, Mach-O readers, DWARF readers, WASM validators, linkers, archive tools, and hash/file helpers.
