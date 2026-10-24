# `auth.system:service`

`use <auth.system:service>;` importa una tabla raw de autoridad sobre servicios.
Cada registro ocupa 64 bytes y contiene key, handle, process, rights, flags,
owner, generation y state. El programa aporta la memoria y controla su vida.

Las 13 operaciones `authService*` se materializan como
`__ncc_auth_service_*`. No consultan ni modifican el Service Control Manager;
para ello existe el import manual `process:service`.
