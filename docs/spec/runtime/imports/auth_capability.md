# `auth.security:capability`

`use <auth.security:capability>;` importa una tabla raw de capacidades. Cada
registro de 64 bytes contiene id, subject, resource, rights, scope, flags,
generation y state. El programa crea, copia, consulta y destruye los registros
sin ownership ni validacion oculta.
