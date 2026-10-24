# Machine External WASM Compatibility

WASM validation uses `wasm-validate`, `wasm-objdump`, `wasm-tools`, `wasm2wat`, `file`, and hash tools when available.

The profile validates module parseability, section ordering, exports, names, and custom debug sections as external compatibility evidence.
