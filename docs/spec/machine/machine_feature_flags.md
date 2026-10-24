# Machine Feature Flags

Feature flags bind instructions to CPU capabilities.

Current command:

`ncc machine features x86_64 --json`

Current status:

- declared baseline, SSE/SSE2, AVX, AVX2, EVEX and AVX-512 profile features
  participate in closed profile evidence.
- CPU profiles are reported as contracts.
- feature families outside the declared profile remain explicit non-claims.

Feature-gated instructions must not encode unless their required feature is available in the selected target/profile.
