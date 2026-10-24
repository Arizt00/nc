# Release V43: Python to nC

Status: all seven replacements are functionally qualified against current
source-bound Windows and Ubuntu WSL global candidates. The public source payload
contains all 191 native family units, both host overrides and retained Stage0
compilation assets. The integrated batch gate subsequently retired these seven
originals. Their bytes remain in the verified recovery ZIP. Historical family
reports retain their original hashes; the final batch state is documented in
`docs/NATIVE_TOOLS_V43_RETIREMENT_CLOSURE.md`.

## Integration Contract

The canonical executable is `ncc-release-v43` (`.exe` on Windows).
The entry is `tools/tests/release43_main.n`; the public router is
`release43_cli.release43Cli.execute(args, command)`.

| Original | Explicit Command |
| --- | --- |
| package_public_beta.py | package-public-beta |
| prepare_public_beta_layout.py | prepare-public-beta-layout |
| verify_phase10_cicd_compat_v2_0_0.py | release-cicd-compat |
| verify_phase10_installer_matrix_v2_0_0.py | release-installer-matrix |
| verify_phase10_manifest_format_v2_0_0.py | release-manifest-format |
| verify_phase10_supply_chain_security_v2_0_0.py | release-supply-chain |
| verify_phase10_updates_v2_0_0.py | release-updates |

Production is nC and never launches Python. Temporary unchanged-original oracles
have been replaced by sealed goldens and native-only regression drivers.
The family has 21 source units and two actual runtime objects per host.
It reuses `release_io.n`, `tool_cli.n`, `cert_json.n`, `process_capture.n`,
`sha256.n`, `stream_sha43.n` and `physical43_json.n` without modifying them.
`bootstrap/release43-build.json` and `bootstrap/release43-public.json`
describe the source closure. The parent owns `bootstrap/native-tools-v43.json`,
global dispatch, source-binding checks and qualification receipts.

Current global candidates:

- Windows: `target/tmp/native43-windows/bin/ncc-release-v43.exe`,
  SHA256 `ddd8e03f86eab672fa1c9bd7a29eae6b1a8d48bf90aff7d7b594c085a41ff4dd`.
- Linux: `target/tmp/native43-linux/bin/ncc-release-v43`,
  SHA256 `ccd8387b05a50b58a65c1b8f57837b3cf85fb1f6413dac95c12e389c695a824e`.

## Reproducible Builds

Use the retained Stage0 compiler and native constructor documented in
`docs/PYTHON_RETIREMENT_V43.md`. After constructing `build-native-tools`:

```bash
build-native-tools --manifest bootstrap/native-tools-v43.json --root . --out target/native43 --ncc /absolute/path/to/retained/ncc --cc /usr/bin/gcc --family ncc-release-v43 --target x86_64-unknown-linux-gnu
build-native-tools --manifest bootstrap/release43-gate.json --root . --out target/release43-gate --ncc /absolute/path/to/retained/ncc --cc /usr/bin/gcc --target x86_64-unknown-linux-gnu
target/native43/bin/ncc-release-v43 prepare-public-beta-layout --root .
target/release43-gate/bin/ncc-release-layout-gate-v43 --root . --dist dist/public-beta/ncc --report layout-report.json
target/native43/bin/ncc-release-v43 package-public-beta --root .
```

For Windows, use target `x86_64-pc-windows-msvc`, executable suffix `.exe`
and `C:/msys64/mingw64/bin/gcc.exe`; prepend `C:/msys64/mingw64/bin`
to PATH. This target emits COFF linked by GNU GCC. It does not certify an MSVC
installation. Synchronize a project snapshot before linking it after any change.
WSL requires the Rust installation on PATH when a verifier runs Rust binaries.

The layout Gate imports a host adapter and is not a single-unit `ncc run`
input. Its six-source builder manifest and complete dependencies ship in the
public package. Missing manifests and missing native dependencies are tested
negative cases; checks are not weakened.

## Functional Evidence

Both global native-suite reports have ten passing cases, explicit candidate
paths/hashes and coverage of all seven originals:

- `bootstrap/release43-native-suite-windows-global-sealed.json`
- `bootstrap/release43-native-suite-linux-global-sealed.json`

Both global object manifests bind all 21 source units and two runtime objects:
`bootstrap/release43-windows-objects-global-sealed.json` and the Linux peer.
The ten tests plus their suite driver have 253 actual objects per host in
`bootstrap/release43-test-objects-{windows,linux}-r25.json`.

Package/layout evidence includes exact original ZIP bytes; TAR member order,
content, types, modes, ownership and timestamps; hard links, Linux symlinks,
Unicode/non-BMP paths, long PAX paths, filters, schemas and checksums. Layout
also covers copy2 modification timestamps, pruning, warnings, replacement,
`--keep`, boundaries and malformed JSON.

Prune truthiness agrees with 37 unchanged-original cases per host, including
NUL/surrogates, containers, nonfinite values, arbitrary-width integers and the
exact binary64 zero midpoint. Projection uses pure nC decimal comparison, not
libc rounding. Argparse negative-number classification agrees for all 1,114,112
scalars, including 680 Unicode 15.0 decimal digits, and 16 actual CLI pairs per
host. The retained CLI driver runs 43 native cases including the full bitmap.

The Windows number oracle uses UTF-8 stdio. Its default cp1252 reporting failure
after writing Unicode archives remains a recorded failure, not a green result.
The raw Windows report's `win32` platform label is preserved. The strict
qualification adapter `bootstrap/release43-cli-number-windows-global-strict-qualified.json`
normalizes it to `windows`, binds the raw SHA256, explicitly covers package/layout
and copies all sixteen actual comparison results unchanged with stable case names.
The Linux strict adapter adds the same explicit names and raw-report binding.

Verifier checks agree with the original: updates 4/4, CI 7/7, installer 13/13 and
manifest 7/7 on each host. Linux supply-chain agrees 7/7. On Windows both reject
the original missing GNU host and emit no report. Separate actual signed native
distributions pass 22 integrity cases per host, including wrong keys, signature,
artifact and SBOM mutations, missing signatures and empty inventories. No
product installation or hardware success is inferred from expected rejection.

The migrated-fixture regression verifies all seven originals are absent from
its fixture and executes the replacements with frozen expected checks. It
requires no Python. Assigned CI/workflow consumers and five scoped Stage0 test
modules use real .n/native contracts. Parent Stage0 consumer evidence is
separate from the family verifier qualification.

## Public Source Closure

The public manifest ships all ten families (191 source units, 211 actual
objects per host), including Release21 and Certification24, the builder,
global manifest, Gate, standalone native tests and frozen public goldens.
It also carries the Android, runtime and ghor include_str assets, diagnostics,
public Cargo-test fixtures and retirement documentation. Private caches,
credentials, internal reports and generated objects are excluded.

Validation extracts the ZIP to fresh short Windows and ext4 WSL roots, copies
only retained Stage0 as the external compiler prerequisite, builds the builder
from extracted source, then constructs all ten families from the extracted
manifest and sources. Cargo builds twelve real Stage0 binaries and executes
all five public performance contract tests from those extracted sources.
The extracted Gate and executable perform real positive/negative checks and
repackage the distribution. Archive membership/content and all source/object
bindings are checked; compiler success alone is not qualification.

`bootstrap/release43-tests.json` supports selected public unit, public_goldens
and cli_cases families with the same builder. All its source dependencies are
included. The aggregate development suite additionally needs the internal
release-contract fixture and real signed distribution supplied by its caller;
those private development gates are not advertised as public-package fixtures.
The suite arguments are ROOT STAGE0 FAMILY SIGNED_DISTRIBUTION TEST_BUILD REPORT.

Current archive paths, hashes, extracted candidates, object inventories,
commands, outcomes and per-original eligibility are indexed in
`docs/evidence/release-v43.json`. Historical candidates, receipts, false reports
and earlier public payloads remain historical and are not silently overwritten.

## Limits

The original manifest verifier reads generated channel TOML and has no external
TOML option. That contract and eight malformed TOML tests pass on both hosts;
general-purpose TOML parsing is not claimed. Nullable JSON releases are guarded.
`--distribution` is an explicit native extension for integrity-verified native
fixtures, not an original option or provider replacement.

The application JSON authority accepts the original consumed manifest contract
and skips ignored metadata without decoding it. Raw UTF-8, non-BMP characters,
paired escapes, ignored NUL/isolated surrogates/nonfinite/deep metadata and invalid
UTF-8 negatives are tested. This is not a general-purpose JSON library.

Independent ZIP64 qualification covers local/central size, offset 2147483647
and count 65535 boundaries: twelve native and twelve direct-original byte
comparisons. Large-count memory/IO performance is not certified.
Pure streaming SHA was independently certified with 1,082 checks per host,
including Unicode filenames and >2GiB+65. Gzip/archive extraction use native
utilities, not Python. Gzip volatile headers are not asserted byte-identical
to the original. Only the validated Windows/Linux ABIs are selected.

All originals and Stage0 are retained. No Stage3 promotion, remote CI,
unexecuted target, physical hardware or whole-repository closure is claimed.
