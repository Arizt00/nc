# Compiler Mode Resolver

## Objetivo
Resolver modo efectivo (`[@runtime]`, `[@machine]`, `primitive[@]`, etc.) por contenedor/contexto.

## Reglas
- Modo inválido -> diagnóstico estable.
- Cruces de modo prohibidos -> rechazo semántico.
