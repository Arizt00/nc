# Basal .nbb Writer

`ncc basal emit-nbb <file.nb> --out <file.nbb>` runs the source pipeline through Fase 4 verification before writing bytes.

The writer refuses to emit when parsing, table construction, BasalIR verification, or lowering verification produces an error. It writes the artifact only after all sections, section hashes, sizes, offsets, and the structural checksum are computed.

The writer does not integrate final machine evidence and does not produce a final seal.
