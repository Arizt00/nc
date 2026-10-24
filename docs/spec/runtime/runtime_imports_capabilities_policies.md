# Runtime Imports, Capabilities and Policies

Runtime calls are import-gated and policy/capability-gated.

Examples:
- input functions require `use stoud.io { input };`
- output functions require `use stoud.io { output };`
- rendering/parsing require `use stoud.io { render };` / `use stoud.text { parse };`

Core diagnostics:
- `E_RUNTIME_USE_001`
- `E_RUNTIME_CAPABILITY_001`
- `E_RUNTIME_POLICY_001`
