# `auth.security:sandbox`

`use <auth.security:sandbox>;` importa metadatos raw de sandbox. Cada registro
de 64 bytes contiene id, principal, root, rights, restrictions, flags,
generation y state. `authSandboxAllows` interpreta restrictions como mascara de
denegacion, pero no crea ni aplica un sandbox del sistema operativo.
