# Runtime List/Queue Identity

Industrial rule:
- `nC.runtime.list` and `nC.list` resolve as **list family identity**.
- `nC.runtime.queue` resolves as **queue family identity**.
- `runtime exports nC.runtime.list` must expose only `list*` exports.
- `runtime exports nC.runtime.queue` must expose only `queue*` exports.

`list` and `queue` may share internal machinery, but export identity must stay explicit and auditable.
