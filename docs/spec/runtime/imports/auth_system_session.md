# `auth.system:session`

`use <auth.system:session>;` importa una tabla raw de autoridad sobre sesiones.
Cada registro ocupa 64 bytes y contiene key, principal, handle, rights, flags,
owner, generation y state. El programa aporta la memoria y controla su vida.

Las 13 operaciones `authSystemSession*` se materializan como
`__ncc_auth_system_session_*`. No abren sesiones WTS ni recursos; para ello
existe el import manual `process:session`.
