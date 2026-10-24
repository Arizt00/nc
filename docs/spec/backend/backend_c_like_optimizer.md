# Backend C-Like Optimizer Contract

Status: closed for the current ncc native-fast/backend scope.

This phase makes the normal optimized native path behave like a C-style build
pipeline for supported `.n` programs:

1. parse, sema and type-contract gates run first;
2. the AST optimizer applies pure single-return inlining, constant folding,
   pure algebraic simplification and unreachable statement pruning;
3. the whole-program optimizer builds a function index across all loaded
   containers, resolves unique calls to concrete symbols, inlines pure callees
   across container/file boundaries, records LTO imported edges/internalized
   symbols, removes globally unreachable private functions, and applies
   loop-hot PGO inline bias before lowering;
4. the release-fast SSA pass applies mem2reg promotion, constant propagation,
   copy propagation, CSE/GVN, conservative LICM, CFG simplification evidence,
   aggressive post-propagation folding and dead temporary elimination;
5. the loop optimizer applies dependency analysis, safe unroll, invariant
   hoisting, induction strength reduction, bounds-check elimination proofs and
   SIMD vectorization candidate selection;
6. backend lowering emits typed backend IR with concrete ABI/frame/layout
   records;
7. ABI/layout classification uses target data-layout widths for structs,
   fixed arrays, dynamic descriptors, strings, slices, pointers, aggregate
   returns and external calls: SysV64/AAPCS64 pass and return <=16-byte
   register aggregates in multiple machine words, Win64 uses the C ABI indirect
   aggregate rule above 8 bytes, `str`/pointers stay pointer-word, `slice<T>`
   is a two-word descriptor and `dinArray<T>`/`list<T>`/`queue<T>` are
   three-word descriptors;
8. release zero-cost elision removes only proved-redundant release-only
   scaffolding: constant-proven bounds checks, true runtime/assert/security
   guards, debug/trace/inspect print families with discardable operands,
   private local metadata with pure values, and duplicate or linearly
   unreachable native traps. Normal stdout/stderr prints, volatile/MMIO/port
   effects, unknown calls and unproved traps are preserved;
9. backend alias analysis canonicalizes memory locations for stack slots,
   globals, symbols and pointer-derived addresses, classifies operations as
   `must-alias`, `no-alias` or `may-alias`, preserves no-alias memory state,
   forwards alias-proven loads and removes overwritten must-alias stores;
10. the industrial register allocator builds fixed-point CFG liveness, live
   intervals and live segments, colors the interference graph, coalesces
   non-interfering copies, scores spills with loop/profile/traffic cost,
   rematerializes cheap values, selects caller/callee-saved registers and
   reports stack traffic avoided/emitted;
11. target CPU tuning resolves `--cpu native|host|<profile>` or the explicit
   CPU profile, validates it against the backend target, carries feature rows
   into each function backend plan and filters SIMD/vector schedules by real
   feature availability and preferred vector width;
12. the industrial instruction selector chooses flag-preserving `lea`,
   `cmovcc`, `setcc`, complex base/index/scale/disp addressing modes,
   compare/test flag producers suitable for macro-fused branches, branchless
   simple scalar `if/else` selection with `cmovcc`, SIMD selection rows and
   CPU peephole rows before native byte emission;
13. the backend optimizer applies scalar constant folding, finite FP literal
   folding for MCT/LLVM (`sqrt/exp/sin/cos/log/atan2`, arithmetic and
   comparisons), stack load/store forwarding, dead stack-store elimination,
   no-op register move elimination and machine peephole cleanup;
14. object emission reports real text/data/relocation bytes for the selected
   target profile.

Scalar whole-function return folding is path-sensitive. It folds only when
every reachable path terminates with the same constant and the removed body is
free of calls, memory effects, global projections, loops, traps and opaque
statements. Parameter-dependent guarded returns remain native branches. A
fallback `return` cannot replace earlier conditional returns, and a constant
return cannot erase a preceding loop.

The source-of-truth CLI evidence is:

```bash
ncc optimize <file.n> --json
ncc compiler optimize <file.n> --json
ncc compiler backend <file.n> --json
ncc perf native-fast --json
```

`ncc compiler backend` now renders the optimized backend text and embeds the
same optimizer report contract under `backend.optimizer`.

## Report Counters

`OptimizationReportIR` exposes:

- `inline_expansions`
- `whole_program.functions_scanned`
- `whole_program.roots`
- `whole_program.call_edges`
- `whole_program.inline_candidates`
- `whole_program.cross_container_inlines`
- `whole_program.devirtualized_calls`
- `whole_program.dead_functions_removed`
- `lto.linkage_units`
- `lto.imported_call_edges`
- `lto.internalized_symbols`
- `pgo.instrumented_sites`
- `pgo.hot_call_sites`
- `pgo.hot_inline_biases`
- `pgo.cold_functions`
- `constant_folds`
- `statements_removed`
- `backend_constant_folds`
- `backend_load_store_forwards`
- `backend_dead_stores_removed`
- `backend_noop_moves_removed`
- `backend_ops_removed`
- `machine_ops_removed`
- `ssa.mem2reg_promotions`
- `ssa.constant_propagations`
- `ssa.copy_propagations`
- `ssa.cse_gvn_eliminations`
- `ssa.licm_invariants`
- `ssa.cfg_simplifications`
- `ssa.aggressive_folds`
- `ssa.dead_temporaries_removed`
- `ssa.temporaries_eliminated`
- `loop_optimizer.unrolls`
- `loop_optimizer.unrolled_iterations`
- `loop_optimizer.invariant_hoists`
- `loop_optimizer.strength_reductions`
- `loop_optimizer.bounds_checks_eliminated`
- `loop_optimizer.vectorization_candidates`
- `loop_optimizer.dependency_analyses`
- `loop_optimizer.dependency_independent`
- `loop_optimizer.dependency_carried`
- `alias_analysis.queries`
- `alias_analysis.no_alias`
- `alias_analysis.must_alias`
- `alias_analysis.may_alias`
- `alias_analysis.scope_partitions`
- `alias_analysis.forwarded_loads`
- `alias_analysis.dead_stores_removed`
- `alias_analysis.preserved_independent_memory_ops`
- `register_allocation.functions`
- `register_allocation.live_intervals`
- `register_allocation.live_segments`
- `register_allocation.peak_live`
- `register_allocation.interference_edges`
- `register_allocation.coalesced_groups`
- `register_allocation.coalesced_locals`
- `register_allocation.spill_candidates`
- `register_allocation.spilled_locals`
- `register_allocation.rematerialized_locals`
- `register_allocation.caller_saved_residents`
- `register_allocation.callee_saved_residents`
- `register_allocation.stack_traffic_bytes_avoided`
- `register_allocation.stack_traffic_bytes_emitted`
- `instruction_selection.functions`
- `instruction_selection.lea`
- `instruction_selection.cmov`
- `instruction_selection.setcc`
- `instruction_selection.complex_addressing`
- `instruction_selection.flag_ops`
- `instruction_selection.macro_fusion_pairs`
- `instruction_selection.simd_rows`
- `instruction_selection.cpu_peepholes`
- `instruction_selection.bytes_saved_estimate`
- `cpu_tuning.functions`
- `cpu_tuning.host_detected_functions`
- `cpu_tuning.profiles`
- `cpu_tuning.targets`
- `cpu_tuning.preferred_vector_width_bits`
- `cpu_tuning.feature_rows`
- `cpu_tuning.selected_simd_rows`
- `cpu_tuning.schedule_rows`
- `cpu_tuning.features`
- `abi_call_lowering.call_sites`
- `aggregate_layout.layouts`
- `release_zero_cost.bounds_checks_eliminated`
- `release_zero_cost.runtime_checks_eliminated`
- `release_zero_cost.debug_prints_eliminated`
- `release_zero_cost.traps_eliminated`
- `release_zero_cost.metadata_eliminated`
- `release_zero_cost.security_checks_eliminated`
- `release_zero_cost.total_eliminated`

Every non-zero transformation counter must also have debugger UX evidence in
`debugger_ux.transformations`.

## Boundary

This does not claim formal whole-program optimizer equivalence with GCC, LLVM,
MSVC or rustc on arbitrary code. Unsupported patterns must remain visible as
ordinary backend code or diagnostics; they must not be silently marked as
optimized.

The measurable release-fast chain is exposed by `ncc optimize <file.n> --json`
under `release_fast_pipeline`:

```text
.n -> parser_ast -> semantic_contracts -> ssa_optimized_ir
   -> machine_backend_ir -> object -> executable
```

The instruction-selection evidence is exposed under
`release_fast_pipeline.instruction_selection.passes` with:

- `lea_selection`
- `cmov_selection`
- `setcc_boolean_materialization`
- `complex_addressing_modes`
- `flags_and_macro_fusion`
- `simd_instruction_selection`
- `cpu_specific_peepholes`

CPU tuning evidence is exposed under
`release_fast_pipeline.target_cpu_tuning` with the selected profiles, host
detection count, feature rows, selected SIMD rows and schedule rows.

Whole-program evidence is exposed under `release_fast_pipeline.whole_program`,
`release_fast_pipeline.lto` and `release_fast_pipeline.pgo`; those sections are
fed by real AST call graph analysis, direct-call resolution, global DCE and
loop-hot inline bias before backend lowering.

The C comparison baseline remains `ncc perf native-fast --json`; the long CI
form is `ncc perf native-fast --corpus --c-matrix --iterations 25`, which
retains real `clang/gcc x O2/O3` timings inside `c_style_corpus` and
`benchmarks[].c_matrix`. Optimizer payloads reference that schema instead of
fabricating benchmark numbers.
