# Machine Macro Directive Object Integration

Macro-expanded code emits through the existing native encoders. Directive data emits into object `.data`, reserve directives emit `.bss` size and symbols, and incbin emits bounded payload bytes with hashes.

Object-aware disasm keeps code decoding unchanged and exposes data sections. Roundtrip compares expanded code, symbols, and data-section symbols; original macro spelling is not required for semantic closure.

