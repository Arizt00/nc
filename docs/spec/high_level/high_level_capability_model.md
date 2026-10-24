# High Level Capability Model

Status: consolidated.

Capabilities are grouped by const evaluation, comptime unsafe, macros, risk authority, runtime, compiler and inspect/ai. The default policy is conservative: pure deterministic work is allowed, hazardous work requires explicit policy, and runtime/compiler semantics are reflected rather than changed.
