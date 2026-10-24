# Basal .nbb Reader

`ncc basal read-nbb <file.nbb> --json` reads the binary artifact, checks header fields, parses the section table, validates hashes, and reconstructs the structural records.

Legacy `NBB1` fixtures remain readable for compatibility tests, but Phase 5 emission uses `NBB0`.
