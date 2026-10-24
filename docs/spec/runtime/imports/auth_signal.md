# `auth.system:signal`

`use <auth.system:signal>;` importa una tabla raw de autoridad sobre signals.
Cada registro ocupa 64 bytes y contiene key, target, code, rights, flags, owner,
generation y state. El programa aporta la memoria y controla su vida.

Las 13 operaciones `authSignal*` se materializan como `__ncc_auth_signal_*`.
No crean eventos, handlers ni excepciones; para ello existe el import manual
`process:signal`.
