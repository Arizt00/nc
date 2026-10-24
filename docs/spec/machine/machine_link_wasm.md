# Machine Link WASM

WASM linking is closed for `wasm32-unknown-unknown`. The profile covers linked module artifacts, imports/exports, code section fixups, name section preservation, and `nC.debug` custom section preservation.

Traditional native unwind is not part of WASM; debug metadata is preserved through custom/name sections.
