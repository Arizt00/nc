# Publication validation

Executed on 24 October 2026. The `ncc-ast` regression is fixed: **118 tests pass, none fail**. The full workspace run finishes with **4,759 passed, 475 failed and 0 ignored**. The workspace is not passing.

These source tests ran against the development working tree on Windows x86-64 with MSVC. The public repository is a curated export that omits historical archives and local build artifacts. The results do not certify a clean public clone. V81 reconstruction evidence is retained in its dossier; the complete B/C reconstruction was not repeated for this publication.

## String catalog correction

`stoud_text_string_is_manual_nc_with_explicit_release` expected 26 exports although `charFromCode(int) -> str` was already present in the implementation and typed API catalog, bringing the total to 27. The replacement assertion compares the exact symbol sets maintained by import lowering and the typed runtime catalog. A separate cardinality check rejects duplicates, and an explicit assertion requires `charFromCode`. Release and emitted-byte assertions remain in place.

The compiler change is confined to this regression test. It does not alter parsing, semantic analysis, IR generation or runtime behavior. The [string contract](../spec/runtime/imports/string.md) documents the existing API and its validation route.

## Source test results

| Run | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Targeted `ncc-ast` library | 118 | 0 | 0 |
| Complete workspace, including AST | 4759 | 475 | 0 |

The targeted result is included in the workspace result; the rows must not be added together. The full run completed 194 test targets, of which 51 reported failures, and returned exit code 101. No tests were skipped by a name filter. Platform-conditional tests retain their existing configuration.

The Rust test harness was used because these are Rust implementation tests. Language programs were checked with `ncc` directly.

```bash
cargo test --locked -p ncc-ast --lib -- --nocapture
cargo test --locked --workspace --no-fail-fast -- --nocapture
```

The environment used `CARGO_BUILD_JOBS=1`, the V81 B2 Windows compiler as `NC_STAGE0_POLICY_COMPILER`, and MSVC `lib.exe` as `NC_STAGE0_POLICY_ARCHIVER`. [Installation](../install.md) describes Stage0 seed configuration.

The remaining failures occur outside the corrected assertion. Observed cases include legacy fixtures absent at their expected paths, rejection of older ordinary-container root examples, a primitive boot payload above its 510-byte limit, and older high-level semantic expectations that no longer match the compiler. These observations identify failing contracts; they do not establish a single cause or justify changing expected results merely to obtain a passing run.

The [machine-readable result](validation-results.json) lists every target, its counts, failure names and rerun arguments. It also records the full local log digest and the corrected AST source digest. Failed targets are summarized below.

| Target | Passed | Failed |
| --- | ---: | ---: |
| `ncc_cli: unittests src/lib.rs` | 1187 | 96 |
| `tests/backend_native_fast_matrix.rs` | 5 | 1 |
| `tests/basal_closure.rs` | 202 | 10 |
| `tests/canonical_spec_error_layout.rs` | 4 | 1 |
| `tests/debug_adapter_incremental.rs` | 3 | 2 |
| `tests/driver_factory_suite.rs` | 1 | 1 |
| `tests/expert_release_suite.rs` | 2 | 2 |
| `tests/expert_release_type_contract_checks.rs` | 0 | 17 |
| `tests/ghor_bootstrap_generation_contract.rs` | 3 | 1 |
| `tests/ghor_bootstrap_stage0.rs` | 3 | 1 |
| `tests/ghor_orchestrator.rs` | 72 | 1 |
| `tests/high_level_closures.rs` | 17 | 25 |
| `tests/high_level_phase1.rs` | 15 | 2 |
| `tests/high_level_phase10.rs` | 41 | 2 |
| `tests/high_level_phase12.rs` | 18 | 6 |
| `tests/high_level_phase13.rs` | 38 | 19 |
| `tests/high_level_phase2.rs` | 34 | 1 |
| `tests/high_level_phase5.rs` | 23 | 1 |
| `tests/high_level_phase6.rs` | 32 | 2 |
| `tests/high_level_phase8.rs` | 4 | 2 |
| `tests/high_level_phase9.rs` | 27 | 16 |
| `tests/hosted_runtime_prelude.rs` | 5 | 3 |
| `tests/import_surface_stoud.rs` | 26 | 2 |
| `tests/industrial_line_bands_suite.rs` | 0 | 1 |
| `tests/kernel_firmware_artifact_isolation.rs` | 0 | 1 |
| `tests/kernel_physical_smp_artifacts.rs` | 0 | 1 |
| `tests/local_surface.rs` | 4 | 2 |
| `tests/machine_closure.rs` | 767 | 12 |
| `tests/machine_virtualization_vmcs_vmcb.rs` | 3 | 1 |
| `tests/memory_conditions_option_result.rs` | 3 | 3 |
| `tests/native_fast_path.rs` | 14 | 4 |
| `tests/nc_stage2_lexer_differential.rs` | 0 | 52 |
| `tests/official_arrays_lists_indexing_matrix.rs` | 103 | 61 |
| `tests/performance_harness.rs` | 14 | 2 |
| `tests/pointer_model.rs` | 8 | 6 |
| `tests/project_bridge_priority13.rs` | 35 | 5 |
| `tests/release_run_matrix.rs` | 2 | 8 |
| `tests/runtime_async_event_loop_contract.rs` | 3 | 1 |
| `tests/runtime_console_golden.rs` | 21 | 5 |
| `tests/runtime_errors_priority9_contract.rs` | 3 | 1 |
| `tests/runtime_family_closure.rs` | 10 | 4 |
| `tests/runtime_family_priority12.rs` | 2 | 6 |
| `tests/runtime_generics_closure.rs` | 17 | 3 |
| `tests/runtime_json_native_pe.rs` | 0 | 1 |
| `tests/runtime_process_thread_contract.rs` | 10 | 1 |
| `tests/runtime_render_concat_closure.rs` | 24 | 4 |
| `ncc_diagnostics: unittests src/lib.rs` | 11 | 1 |
| `tests/registry_governance.rs` | 2 | 1 |
| `ncc_ir: unittests src/lib.rs` | 446 | 14 |
| `tests/nc_runtime_json_mct.rs` | 0 | 1 |
| `ncc_sema: unittests src/lib.rs` | 481 | 59 |

`git diff --check` passes for the AST change. The earlier repository-wide formatting check reported existing differences; no broad formatting rewrite was applied.

## Direct ncc checks

The retained Stage0 executable passes all three routes for `examples/runtime/families/string_basic.n`:

```powershell
.\target\debug\ncc.exe check examples/runtime/families/string_basic.n
.\target\debug\ncc.exe emit-ir examples/runtime/families/string_basic.n
.\target\debug\ncc.exe run examples/runtime/families/string_basic.n
```

All return exit code 0. The program exercises `charFromCode(90) == "Z"`, manual release and string operations, and reports `manual nC string provider: true`.

The `native_string_release_bad_arity_v56.n`, `native_string_release_bad_type_v56.n` and `native_string_release_bad_implicit_v56.n` fixtures under `tools/tests` each return exit code 1 as expected. The bad-type case reports `E_RUNTIME_GENERIC_ARG_MISMATCH_001`.

The native V81 compiler builds `tools/tests/native_text_render_v66.n`; the executable prints `text-render-ok` and returns its expected code 205. The English guide's counter program passes native semantic analysis. Compact runtime imports shown in the reference pass Stage0 analysis.

## Portable release kits

The refreshed Windows ZIP and Linux TAR.GZ were extracted into new directories. Their manifests were verified, then `self-test`, semantic analysis, IR emission, executable generation and hello execution were run through the packaged `ncc`. Each command returned 0 and hello printed exactly `Hello, nC public beta`.

Windows x86-64 and Ubuntu 24.04.4 under WSL x86-64 were checked. These kit routes require no Rust, Cargo or LLVM installation. Linux was tested with glibc 2.39; imports require symbols through GLIBC_2.34. Generated Linux programs need the documented provider load path.

The compiler and runtime binaries retain their V81 B2 identities:

| File | SHA-256 |
| --- | --- |
| Windows ncc.exe | `4ff2300d30fbe99a4c541481e0fab9df284d77c32d1df082084440b9718854c9` |
| Windows ncruntime.dll | `83d67b0723e18eb8c9898eae286abf8ffe3bc63bd610d7aea7924de2d70f5dbc` |
| Linux ncc | `e3d6fc744f0768d19a3f44ca93eddcb90cc3f89caee536f68f8f5ce486eae1e7` |
| Linux libncruntime.so.2 | `92ac32740801e0ebdb572cd953bc258fa73843bf950a8cf3f80e7c0ea4681b5e` |

The release archives now contain English instructions. Their updated hashes and the English PDF hashes are published in `SHA256SUMS.txt`. All five published release assets were downloaded and compared byte-for-byte by SHA-256 with the local release files. The original release tag remains unchanged.

## Website and documents

The website, sixteen library chapters, manuals, installation and quickstart guides, current V81 dossier, repository entry points and contribution templates are in English. Historical technical archives and source fixture text retain their original wording.

The static check passes for 38 HTML pages, 543 local references and 16 searchable chapters. Eighteen legacy Spanish URLs redirect to English routes. The live homepage, library, downloads and a chapter return English HTML; both hosted PDFs match the release files.

Edge checks cover code copying, search and Escape handling, empty filters, persistent theme, the mobile menu, platform selection, command copying, keyboard phase tabs, expandable questions and reading history. No horizontal overflow was found at widths of 390, 768 or 1440 pixels on the checked routes.

Both PDFs contain nine pages: eighteen pages in total. Every page was rendered and visually reviewed. Text bounds remain inside the page. Editable Markdown sources are retained.

The public export uses an explicit file selection and records source hashes in `PUBLICATION.json`. Private history, caches and local reports are excluded; native kits are release assets. Passing publication checks do not imply a passing compiler workspace or global Stage3 promotion.
