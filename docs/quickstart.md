# Quickstart

Extract the [native V81 kit](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81) and run commands from its directory. The kit already contains `hello.n`:

```nc
use <io:output>;

helloCompacto {
    to main { println("Hello, nC public beta"); }
    fa:
        return true;
}
```

On Linux or WSL:

```bash
chmod +x ncc
./ncc compiler sema hello.n
./ncc compiler emit-ir hello.n
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

On Windows:

```powershell
.\ncc.exe compiler sema hello.n
.\ncc.exe compiler emit-ir hello.n
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

The program prints `Hello, nC public beta` and exits with code 0. Keep the matching runtime provider beside the executables.

The native kit and Stage0 have different CLIs. Stage0 routes such as `check`, `build`, `run` and `machine` must not be substituted into this recipe. See [installation](install.md), [syntax](syntax.md) and the [language guide](public/origen-y-guia.md) for details.
