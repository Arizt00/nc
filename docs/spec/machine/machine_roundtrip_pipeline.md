# Machine Roundtrip Pipeline

`ncc machine roundtrip <file.nm> --target <target> --json` executes the native pipeline from source to object and back to a normalized MachineIR report.

The report includes source path, target, object format, original IR summary, emitted artifact hash, disasm report hash, normalized IR hash, comparison status, accepted normalizations and diagnostics.

Strict behavior is the default: any unaccepted diff returns a failing command result.
