# Native Tools V43 Commands

Current consumers use the nC families declared in
`bootstrap/native-tools-v43.json`. Build orchestration is implemented by
`tools/scripts/build_native_tools43.n`; it copies family sources, synchronizes
each project snapshot, emits native objects and links the executable. Stage0
remains the compiler for this process. The 46 original scripts have been
retired after native qualification on both hosts. Their exact bytes remain
in the verified recovery ZIP; historical comparison evidence is unchanged.
See `docs/NATIVE_TOOLS_V43_RETIREMENT_CLOSURE.md` for the scoped closure.

Run the following commands from the repository root. They build all families;
to build one family, replace the empty `--family` value with its executable
basename. Building a consumer does not itself qualify its functional parity.

## Windows x64

PowerShell, with GCC and the family link dependencies installed in MSYS2:

```powershell
$root = (Get-Location).Path
$stage0 = Join-Path $root 'target/x86_64-pc-windows-gnu/release/ncc.exe'
$bootstrap = Join-Path $root 'target/native-tools-v43-bootstrap-windows'
$out = Join-Path $root 'target/native-tools-v43-windows'
$env:PATH = 'C:/msys64/mingw64/bin;C:/msys64/usr/bin;' + $env:PATH
New-Item -ItemType Directory -Force -Path $bootstrap | Out-Null
Copy-Item tools/scripts/build_native_tools43.n (Join-Path $bootstrap 'main.n')
& $stage0 project sync $bootstrap --main main.n --target x86_64-pc-windows-msvc
if ($LASTEXITCODE) { throw 'builder project sync failed' }
& $stage0 project link $bootstrap --main main.n --target x86_64-pc-windows-msvc --out (Join-Path $bootstrap 'bin/build-native-tools.exe') --linker C:/msys64/mingw64/bin/gcc.exe
if ($LASTEXITCODE) { throw 'builder project link failed' }
& (Join-Path $bootstrap 'bin/build-native-tools.exe') --manifest bootstrap/native-tools-v43.json --root $root --out $out --ncc $stage0 --cc C:/msys64/mingw64/bin/gcc.exe --target x86_64-pc-windows-msvc --family ''
if ($LASTEXITCODE) { throw 'native family build failed' }
$env:PATH = (Join-Path $out 'bin') + ';' + $env:PATH
ncc-documents-v43.exe build_nc_manual --help
```

The retained Stage0 binary is the Windows GNU compiler build. Its native
project target here is `x86_64-pc-windows-msvc`, linked by MinGW GCC as required
by the V43 builder. Executable basenames have the `.exe` suffix on Windows;
PowerShell also resolves them without that suffix after the PATH addition.

## Ubuntu WSL x64

In `wsl -d Ubuntu`, change to this repository's `/mnt/c/.../codex_bundle` path:

```bash
set -euo pipefail
root="$PWD"
stage0="$root/target/release/ncc"
bootstrap="$root/target/native-tools-v43-bootstrap-linux"
out="$root/target/native-tools-v43-linux"
mkdir -p "$bootstrap"
cp tools/scripts/build_native_tools43.n "$bootstrap/main.n"
"$stage0" project sync "$bootstrap" --main main.n --target x86_64-unknown-linux-gnu
"$stage0" project link "$bootstrap" --main main.n --target x86_64-unknown-linux-gnu --out "$bootstrap/bin/build-native-tools" --linker /usr/bin/gcc
"$bootstrap/bin/build-native-tools" --manifest bootstrap/native-tools-v43.json --root "$root" --out "$out" --ncc "$stage0" --cc /usr/bin/gcc --target x86_64-unknown-linux-gnu --family ''
export PATH="$out/bin:$PATH"
ncc-documents-v43 build_nc_manual --help
```

Use separate `--out` directories when building both hosts concurrently.
Windows and Linux snapshots are not interchangeable; always synchronize
before linking. OpenSSL development/link libraries are required by families
that use the corresponding crypto support objects. macOS and ARM hosts are
not qualified by these two x64 adapters.

## GitHub Actions

After building the Stage0 compiler, use the shared composite action:

```yaml
- id: native_tools
  uses: ./.github/actions/build-native-tools
  with:
    stage0: target/release/ncc
    family: ncc-tools-v43
- shell: bash
  env:
    NCC_NATIVE_TOOLS: ${{ steps.native_tools.outputs.bin-dir }}/ncc-tools-v43
  run: bash tools/release/qa_phase10_external_ops_v2_0_0.sh
```

The example is Linux x64. Windows callers supply the actual Stage0 `.exe`
path and use the `.exe` consumer suffix. The action chooses the x64 target
and exposes `bin-dir`; an unsupported host fails instead of claiming a build.

## Family Entry Points

Commands below preserve the original options after the family subcommand.
Invoke the family from the repository root, or pass the command's explicit
repository option. CLI help is not evidence of product/output equivalence.

| Family | Current commands |
| --- | --- |
| `ncc-tools-v43` | `route-vscode`, `route-ghidra`, `mct-smoke`, `cert-matrix`, `migrate-low-level`, `editor-industrial`, `editor-lsp-smoke`, `release-imports`, `release-ncup-disk`, `release-ncup-commands`, `release-ncup-resolution`, `release-first-run`, `release-policy`, `release-docs`, `release-final`, `dist-telemetry`, `dist-external-ops`, `verify-parser-material`, `verify-lexer-material`, `verify-parser-modules` |
| `ncc-release-v43` | `prepare-public-beta-layout`, `package-public-beta`, `release-cicd-compat`, `release-installer-matrix`, `release-manifest-format`, `release-supply-chain`, `release-updates` |
| `ncc-ide-v43` | `route-jetbrains`, `ide43-jetbrains`, `ide43-vscode`, `ide43-ghidra`, `ide43-fullstack`, `ide43-association` |
| `ncc-closures-v43` | `generate-compiler-closure`, `generate-machine-closure`, `generate-primitive-real-closure`, `generate-runtime-closure` |
| `ncc-documents-v43` | `build_nc_manual`, `build_public_pdf_library` |
| `ncc-certification-v43` | `exact-head-evidence`, `portable-hypervisor-iommu`, `primitive-release-evidence`, `certify-final-pipeline`, compatibility command `certify-raw-optimizer` |
| `ncc-mobile-v43` | Direct controller CLI, without an extra family subcommand |
| `ncc-mobile-protocol-v43` | Direct protocol conformance CLI, without an extra family subcommand |

Document generation writes the original default output locations. For replay,
use an isolated `--root` snapshot, and `--out` for the public library. Do not
run generation against canonical documents merely to check command routing.
