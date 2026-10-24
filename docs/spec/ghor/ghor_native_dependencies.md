# ghor Native Dependencies

`ghor` can model external native libraries without pretending that they are
Cargo crates. Native dependencies are declared in `ghor.toml` and flow through
`ghor manifest`, `ghor metadata`, `ghor link`, `ghor graph`, `ghor lock`, and
package metadata.

Baseical inline form:

```toml
[native-dependencies]
libm = { language = "c", abi = "c", kind = "system", headers = ["math.h"], libs = ["m"], symbols = ["sqrt"], link_mode = "dynamic", system = true }
nc_math = { language = "cpp", abi = "itanium-cxxabi", kind = "library", headers = ["nc_math.hpp"], include_paths = ["native/include"], link_paths = ["native/lib"], libs = ["nc_math"], symbols = ["nc_math_add"], artifact = "native/lib/libnc_math.a", checksum = "sha256:<hash>", link_mode = "static" }
```

Baseical table form:

```toml
[native-dependencies.nc_math]
language = "cpp"
abi = "itanium-cxxabi"
kind = "library"
headers = ["nc_math.hpp"]
include_paths = ["native/include"]
link_paths = ["native/lib"]
libs = ["nc_math"]
symbols = ["nc_math_add"]
artifact = "native/lib/libnc_math.a"
checksum = "sha256:<hash>"
link_mode = "static"
```

Supported languages are `c`, `cpp`, `rust`, `zig`, `go`, and `asm`. Supported
ABI labels are `c`, `sysv64`, `win64`, `aapcs64`, `itanium-cxxabi`,
`msvc-cxxabi`, `rust-cabi`, `zig-cabi`, `go-carchive`, and `asm`.

Native dependencies may describe headers, include paths, library names, link
paths, frameworks, exported symbols, flags, pkg-config packages, local artifacts
with mandatory SHA-256 checksums, target-specific ABI and link mode policy.

`ghor native-deps --json` emits a direct report. `ghor link --json` includes
computed `-I`, `-L`, `-l`, and framework flags as a plan. This point does not
invoke a native linker directly; it closes the manifest and dependency surface
so the next point can perform physical external linking.

Source-level `foreign:` contracts are surfaced separately by
`ghor ffi-manifest --json`. That command delegates extraction to the compiler
FFI link manifest collector and then combines the result with these
`[native-dependencies]` records. `ghor metadata`, `ghor link`, `ghor package`
manifests, and `ghor.lock.meta.json` include the compiler FFI manifest so
package evidence can track artifacts declared in source as well as artifacts
declared in `ghor.toml`.

Registry operations preserve the same native/FFI evidence. `ghor publish`
stores package manifest and FFI manifest hashes in the registry index. `ghor
fetch`, `ghor install`, and `ghor vendor` validate the package manifest, the
embedded `ffi_link_manifest`, and the FFI manifest hash before accepting a
package. A corrupt or failed source-level FFI manifest is rejected before bytes
are installed or vendored.

Diagnostics:

- `E_GHOR_NATIVE_LANGUAGE_001`
- `E_GHOR_NATIVE_ABI_001`
- `E_GHOR_NATIVE_LINK_MODE_001`
- `E_GHOR_NATIVE_LINK_001`
- `E_GHOR_NATIVE_ARTIFACT_001`
- `E_GHOR_NATIVE_CHECKSUM_001`
- `E_GHOR_NATIVE_FIELD_001`
- `E_GHOR_FFI_MANIFEST_001`
