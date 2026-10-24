# Reproducing V81

English edition of the retained reconstruction procedure. This recipe describes the V81 verification environment and its prerequisites.

## Snapshot and generations

`bootstrap/frontend-v81-build.json` identifies 74 compiler sources and validation sources. Each builder verifies SHA-256 while copying. Generation snapshots are retained in the evidence workspace. The builder rejects destinations containing `.ncc`.

Compile `build_native_compiler81.n` with sha256, elf_entry79 and coff_entry79 children at `src/sha256.n`, `src/elf_entry.n` and `src/coff_entry.n`. The runtime builder uses sha256 and the native publisher `link_native_providers73.n`.

```text
compiler-builder REPO SEED DIRECTORY-B SEED-PROVIDER
B/ncc self-test
runtime-builder REPO B/ncc RUNTIME-PROJECT-B PUBLICATION-B PUBLISHER
compiler-builder REPO B/ncc DIRECTORY-C PUBLICATION-B
C/ncc self-test
runtime-builder REPO C/ncc RUNTIME-PROJECT-C PUBLICATION-C PUBLISHER
```

Place providers beside Windows executables. On Linux, use `LD_LIBRARY_PATH` for consumers without RUNPATH. The unit-linked compiler uses RUNPATH `$ORIGIN`. System dependencies and linkers are explicit; no helper C source is compiled.

## Alignment programs

```text
ncc compiler emit-mct tools/tests/native_overalignment81.n --target TARGET --out case.ncmct
ncc compiler emit-object tools/tests/native_overalignment81.n --target TARGET --out case.obj
ncc compiler emit-executable tools/tests/native_overalignment81.n --target TARGET --out case.exe
```

Windows uses `x86_64-pc-windows-msvc`. Linux uses `x86_64-unknown-linux-gnu`, .o objects and executable permission on the ELF. Execute and compare exact stdout and exit codes. For the transitive fixture, copy calls as main.n and place api/leaf in src/.

The powers_globals and powers_locals fixtures cover sixteen powers of two from 32 to 1048576. Globals aligned above 8192 cannot be represented in COFF: require object rejection and execute direct PE output. Large locals remain representable in COFF; link with sufficient stack reserve.

Example link.exe options for the 8192-byte COFF global fixture:

```text
/nodefaultlib /entry:nc_start /subsystem:console /machine:x64
/align:8192 /Brepro /incremental:no
```

For powers_locals and max_local on Windows, add `/stack:16777216,65536`. The nC entry calls main and exits through ExitProcess; do not use /entry:main. Linux links with _start, GNU ld, -z noexecstack, -z separate-code and --build-id=none. The max_abi test declares `ulimit -s 32768` before execution on Linux and uses the 16 MiB Windows reserve.

`native_overalignment_verify81.n` accepts an object/image and a JSON report path. It reads COFF/PE/ELF and checks sections, padding, addresses and the fixture's three pointers. Retain LLVM/GNU disassembly and section inspection as well.

## Cache and adversarial checks

Run project link cold and warm in the wide project, without --no-incremental. Save `.ncc/incremental/project-cache.json` from each pass: one AST and two materials should be restored warm.

MCT controls copy the cache into new projects, alter only derived material and recompute its manifest digest. Use alignment=24, alignment=16 with 8192-aligned symbols, or global offset=1. The emitter must reject all three without writing a final image. Do not alter positive sources or binaries to accommodate a failed check.

## Corpus and runtime

```text
runner verify-programs --root REPO --candidate B --baseline C --stage0 STAGE0
    --out NEW-DIRECTORY --corpus bootstrap/frontend-v69-native-programs.json
```

Repeat C/B on both hosts serially. On Windows, prepare the output directory with ncruntime.dll before invoking the runner. It requires a new capture directory but does not automatically publish that provider.

The 38 contracts use runtime_processes_verify76.n. Prepare file.dat with the five-byte content bytes, Unix mtime 1700000000 and Linux mode 0644; names containing first.dat/segundo.dat; empty/stress directories empty; create/missing absent. These literal fixture names are retained. Do not reuse mutated fixtures to conceal failures.

## Equivalence and custody

runtime_artifacts_verify76.n, with its sha256 child, accepts:

```json
{"artifacts":[{"name":"case","reference":"path-B","candidate":"path-C","normalization":"none"}]}
```

Compare distinct, nonempty files. Retain reports with SHA-256, compared byte counts and first_difference=-1. Keep JSON containing timings/paths separate from binary materials; do not normalize the latter.

The evidence ZIP retains deduplicated payloads, a manifest and complete subsequent reads of every entry. The outer seal contains ZIP and manifest hashes. Certification is local Windows/WSL x86-64, not other hardware, hosts or global Stage3 promotion. The source distribution does not include the complete evidence payload archive.
