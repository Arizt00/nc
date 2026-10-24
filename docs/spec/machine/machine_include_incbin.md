# Machine Include And Incbin

`include` reads `.nmi` machine fragments under a workspace-relative policy. Includes have depth limits, cycle detection, deterministic order, and source-path evidence.

`incbin` reads a bounded binary payload into data, applies alignment, emits the payload hash, and publishes a symbol for object inspection.

Diagnostics include missing files, cycles, depth limit, size limit, and forbidden paths.

