# MachineIR

MachineIR preserves module, target, routines, blocks, instructions, operands, labels, symbols, relocations, frame, ABI, flags, encoding hash, verify contract, and seal contract.

`ncc machine emit-ir <file.nm> --json` emits the machine-readable projection.

High-level `fn:` and `fa:` cannot survive into MachineIR because they are rejected before IR emission.
