# Compile-time Diagnostics

Const eval diagnostics use `E_HIGH_CONST_EVAL_*`. The major groups are forbidden effects, runtime calls, system/process access, FFI, machine access, reflection mutation, dynamic eval, private metadata, `Any` leaks, construction leaks, side effects, step/recursion limits, overflow, division by zero, aggregate size and visibility.

Comptime unsafe diagnostics use `E_HIGH_COMPTIME_*`. They report missing `allow`, filesystem/process/system errors, heap/reflection/private/dynamic/construction/side-effect authority errors, unbounded loop or recursion misuse, `noLimit` misuse and unknown effects.

Diagnostics are explicit by design. A compile-time operation must not fall back to runtime execution, fake metadata or silent acceptance.
