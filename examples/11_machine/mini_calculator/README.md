# Mini calculadora nC Machine

Calculadora interactiva nativa escrita exclusivamente en nC `[@machine]`.
No contiene C, C++, Rust, Java, Python ni ensamblador externo. El enlazador de
nC produce directamente un ELF64 para Linux/WSL y un PE32+ para Windows.

## Capacidades

- suma, resta, multiplicacion y division de enteros `int64`;
- entrada decimal con signo;
- soporte completo de `-9223372036854775808` a `9223372036854775807`;
- deteccion de entrada invalida y valores fuera de rango;
- deteccion de overflow en las cuatro operaciones;
- deteccion de division entre cero;
- pausa final para que la ventana no desaparezca al abrir el `.exe` con doble clic;
- entrada interactiva o redirigida.

## Construir en Linux o WSL

Desde la raiz del repositorio:

```bash
mkdir -p examples/11_machine/mini_calculator/dist
ncc machine build-exe \
  examples/11_machine/mini_calculator/mini_calculator_linux_x86_64.nm \
  --target x86_64-unknown-linux-gnu \
  --out examples/11_machine/mini_calculator/dist/mini-calculator-linux
chmod +x examples/11_machine/mini_calculator/dist/mini-calculator-linux
./examples/11_machine/mini_calculator/dist/mini-calculator-linux
```

WSL utiliza el mismo ELF64 y los mismos comandos.

## Construir para Windows

En PowerShell, con `ncc` disponible en `PATH`:

```powershell
New-Item -ItemType Directory -Force .\examples\11_machine\mini_calculator\dist | Out-Null
ncc machine build-exe `
  .\examples\11_machine\mini_calculator\mini_calculator_windows_x86_64.nm `
  --target x86_64-pc-windows-msvc `
  --out .\examples\11_machine\mini_calculator\dist\mini-calculator-windows.exe
.\examples\11_machine\mini_calculator\dist\mini-calculator-windows.exe
```

Tambien se puede generar el PE64 desde WSL:

```bash
ncc machine build-exe \
  examples/11_machine/mini_calculator/mini_calculator_windows_x86_64.nm \
  --target x86_64-pc-windows-msvc \
  --out examples/11_machine/mini_calculator/dist/mini-calculator-windows.exe
```

El ejecutable de Windows importa solamente `GetStdHandle`, `ReadFile`,
`WriteFile` y `ExitProcess` desde el sistema. No enlaza un CRT. El ELF de
Linux/WSL usa directamente las llamadas de sistema `read`, `write` y `exit`.

## Codigos de salida

| Codigo | Significado |
|---:|---|
| `0` | calculo correcto |
| `2` | entero invalido o fuera de `int64` |
| `3` | operador no valido |
| `4` | division entre cero |
| `5` | desbordamiento aritmetico |
