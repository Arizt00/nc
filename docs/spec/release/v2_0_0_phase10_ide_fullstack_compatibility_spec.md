# nC Phase10 IDE Fullstack Compatibility Spec v2.0.0

Estado: `frozen`

## Reglas

1. JetBrains/VSCode/Ghidra DEBEN conservar enrutado modal estable.
2. El backend low-level DEBE mantenerse en `nc-sys-lsp`.
3. `nclsp` DEBE conservar rol general high-level.
4. Cambios breaking en contratos IDE REQUIEREN RFC.

## Estabilidad de comandos

- `ncup which ncc --json`
- `nclsp validate ...`
- `nc-sys-lsp analyze ...`

## Prohibiciones

- Prohibido elevar un IDE a autoridad del lenguaje.
- Prohibido romper ruta low-level sin migración documentada.
