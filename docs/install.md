# Install and run nC

## Native V81 kit

Download the Windows x86-64 or Linux x86-64 package from [Releases](https://github.com/Arizt00/nc/releases/tag/v0.1.0-bootstrap.81). Verify its digest against `SHA256SUMS.txt` and extract it into a new directory. This workflow requires no Rust, Cargo or LLVM installation.

On Windows, keep `ncc.exe`, `ncruntime.dll` and `hello.n` together:

```powershell
.\ncc.exe self-test
.\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\hello.exe
```

On Linux or WSL x86-64, keep `ncc`, `libncruntime.so.2` and `hello.n` together:

```bash
chmod +x ncc
./ncc self-test
./ncc compiler emit-executable hello.n --target x86_64-unknown-linux-gnu --out hello
chmod +x hello
LD_LIBRARY_PATH="$PWD${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./hello
```

Expected output: `Hello, nC public beta`. Exit code: `0`.

Linux was tested on Ubuntu 24.04 under WSL with glibc 2.39. Kit imports require glibc 2.34 symbols or later; this binary condition does not certify every distribution. The compiler carries RUNPATH `$ORIGIN`. The generated example needs the explicit provider search path.

Use `compiler sema hello.n` to analyze the program and `compiler emit-ir hello.n` to inspect its IR. The partial native CLI also supports tested `compiler emit-object` and `compiler emit-executable` routes. `--help`, `check` and `machine` can return `E_BOOTSTRAP_STAGE0_REQUIRED_001` with exit code 69.

## Stage0 development build

This section applies to contributors building the retained Rust implementation. It is not required to run the native kit. Install Git, the Rust components declared in `rust-toolchain.toml`, a compatible archiver and a verified native seed with its provider.

From the repository root on Linux or WSL:

```bash
export NC_STAGE0_POLICY_COMPILER="$HOME/nc-seed/ncc"
export NC_STAGE0_POLICY_ARCHIVER=ar
export CARGO_BUILD_JOBS=1
cargo build --locked -p ncc-cli --bin ncc
```

In a Windows MSVC developer terminal:

```powershell
$env:NC_STAGE0_POLICY_COMPILER = 'C:\nc-seed\ncc.exe'
$env:NC_STAGE0_POLICY_ARCHIVER = 'llvm-lib.exe'
$env:CARGO_BUILD_JOBS = '1'
cargo build --locked -p ncc-cli --bin ncc
```

Replace the seed path with your verified kit. MSVC `lib.exe` is another compatible Windows archiver. The native bridge compiles `bootstrap/nc/stage0_policy.n` during the build. Missing seed configuration causes an explicit error.

Once built, invoke Stage0 directly:

```bash
./target/debug/ncc check examples/00_hello/hello.n
./target/debug/ncc emit-ir examples/00_hello/hello.n
```

In PowerShell, use `.\target\debug\ncc.exe` with the same arguments. Cargo is not needed for each language-level check.

See the [language guide](public/origen-y-guia.md) for syntax and the [release status](public/estado.md) for supported routes.
