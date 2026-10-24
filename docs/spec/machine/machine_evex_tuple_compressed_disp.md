# Machine EVEX Tuple And Compressed Displacement

Phase 3 models tuple-based disp8 scaling:

- full 128-bit tuple: scale 16.
- full 256-bit tuple: scale 32.
- full 512-bit tuple: scale 64.
- m32 broadcast tuple: scale 4.

The verifier rejects memory displacement that cannot be represented under the active tuple scale for the closed carriers.

Diagnostic:
- `E_MACHINE_EVEX_COMPRESSED_DISP_001`
