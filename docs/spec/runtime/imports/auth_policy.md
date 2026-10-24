# `auth.security:policy`

`use <auth.security:policy>;` importa una tabla raw de reglas. Cada registro de
64 bytes contiene id, subject, resource, rights, denied, flags, priority y
state. `authPolicyAllows` solo concede bits presentes en rights y ausentes de
denied. No se impone ninguna politica automaticamente.
