# Machine Roundtrip WASM

WASM roundtrip emits the module subset supported by `[@machine]`, then decodes the code section and name/debug custom sections through the Phase 6 module-aware disassembler.

The source `return` instruction may normalize to a code-section `end` opcode when the function body and module hash remain stable.
