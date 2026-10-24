# ncc Benchmark Harness

Status: closed for Performance Bench Harness.

`ncc perf native-fast` is the official benchmark harness owned by `ncc`.
It measures real local compilation and execution paths, emits structured JSON,
and compares nC against local C and Rust toolchains without inventing evidence.

## Command

```text
ncc perf native-fast --json
ncc perf native-fast --iterations 5 --out target/tmp/ncc-perf.json --json
ncc perf native-fast --corpus --c-matrix --iterations 25 --out target/tmp/ncc-perf-corpus.json --json
ncc perf trend --input target/tmp/ncc-perf-corpus.json --out target/tmp/ncc-perf-dashboard.json --json
ncc perf certify --input target/tmp/ncc-perf-corpus.json --host linux-x86_64 --out target/tmp/ncc-perf-cert.json --json
ncc perf budget --input target/tmp/ncc-perf-corpus.json --baseline target/tmp/ncc-perf-prev.json --out target/tmp/ncc-perf-budget.json --fail-on-regression
ncc perf compare --iterations 5 --json
```

`compare` is accepted as an alias for `native-fast`.
The C-style matrix is enabled by default and can be made explicit with
`--c-matrix`; `--no-c-matrix` exists only as a diagnostic escape hatch.

## Suite

The closed suite is `native_fast_small_medium_large`.

Small:

- `examples/performance/native_fast/minimal_return.n`
- `examples/performance/native_fast/minimal_return.c`
- `examples/performance/native_fast/minimal_return.rs`

Medium:

- `examples/performance/native_fast/bench_medium.n`
- `examples/performance/native_fast/bench_medium.c`
- `examples/performance/native_fast/bench_medium.rs`

Large:

- `examples/performance/native_fast/bench_large.n`
- `examples/performance/native_fast/bench_large.c`
- `examples/performance/native_fast/bench_large.rs`

The names are deliberately modest. These are integration fixtures that execute
quickly while still touching different compiler stages: process entry, calls,
arithmetic, pointer read/write, pointer offset/diff and emitted object bytes.
The large fixture must remain self-contained: `addrOf`, `ptrRead<T>`,
`ptrWrite<T>`, `ptrOffset<T>` and `ptrDiff<T>` lower to native bytes and must not
leave external linker symbols.

Each fixed benchmark also emits a C-style comparison matrix:

- `clang -O2`
- `clang -O3`
- `gcc -O2`
- `gcc -O3`

Rows are real build/run attempts. Missing compilers are recorded as
`missing_toolchain`; failed compiles or links are recorded as `failed`. The
harness does not substitute `cc` for a missing `clang` or `gcc` row.

## Long-Running Corpus

`ncc perf native-fast --corpus` upgrades the suite from the fixed
small/medium/large trio to every `.n` fixture in
`examples/performance/native_fast`.

The corpus mode keeps the same top-level schema:

```text
ncc.performance.native_fast.compare.v1
```

It changes `suite` to `native_fast_corpus`, sets `corpus_mode: true`, and emits a
`corpus` object with fixture counts and baseline availability. It builds and
runs every fixture that has a C comparison baseline, because those are the rows
where a C-style comparison exists. nC-only fixtures remain in the corpus as
inventory rows with `enumerated` nC status and explicit missing C rows. Matching
`.c` and `.rs` baselines are used when they exist. Missing baselines are reported
as `missing_fixture`; the harness must not fabricate C/Rust evidence for nC-only
fixtures.

For every corpus fixture, the JSON keeps the same C matrix shape. Fixtures
without a `.c` baseline still receive four explicit matrix rows, each marked
`missing_fixture`, so trend tooling can distinguish "not present" from "not
measured".

The corpus includes optimizer-guidance fixtures with real C baselines:

- `optimizer_inline_chain.n` / `.c`: pure call chain for inline pressure.
- `optimizer_pgo_hot_loop.n` / `.c`: loop-contained hot call for PGO hotness and
  hot inline bias.
- `optimizer_lto_cross_container.n` / `.c`: same-file cross-container call and
  dead private helper for LTO import/internalization/DCE pressure.

## Metrics

The nC case reports a `pipeline` object with:

- source bytes and source characters;
- read duration;
- lexer duration;
- parser duration;
- semantic analysis duration;
- IR lowering duration;
- backend emission duration;
- object IR duration;
- ELF object byte emission duration;
- optimizer duration;
- hosted executable link duration;
- emitted object bytes;
- hosted executable bytes;
- backend object stats including native text bytes.

For the large nC pointer fixture, the backend report must show zero unresolved
symbols and zero relocations for pointer intrinsics.

Every case reports:

- build duration in microseconds;
- run duration in microseconds;
- binary size;
- exit status;
- toolchain command;
- missing-toolchain status when a baseline compiler is absent.

Every benchmark reports `overhead_vs_c` and `overhead_vs_rust` ratios for build
time, runtime and binary size when the baseline exists.

Every benchmark also reports:

- `c_matrix`: the real `clang/gcc x O2/O3` build/run rows;
- `c_matrix_summary`: row counts, missing toolchains, missing fixtures, failed
  rows and fastest measured C row;
- `overhead_vs_c_matrix`: nC/C ratios against every matrix row.

The top-level `c_style_corpus` object summarizes the retained corpus:
benchmark count, microbenchmark/program classification, planned rows, measured
rows, missing toolchain rows, missing fixture rows, failed rows and the CI
artifact names that retain the evidence.

The top-level `optimizer_guidance` object summarizes release-fast optimizer
signals collected from the same real nC builds:

- inline expansions and whole-program inline candidates;
- PGO instrumented sites, loop-hot call sites, hot inline biases and cold
  functions;
- LTO linkage units, imported call edges, cross-container inlines,
  internalized symbols and whole-program dead-function removals;
- ranked `top_inline`, `top_pgo` and `top_lto` rows for tuning decisions.

This guidance is intentionally derived from actual corpus compilation reports.
It is the input surface for changing inline budgets, PGO weighting and LTO import
thresholds; missing optimizer reports remain absent rather than fabricated.

## CI Trend Dashboard

`ncc perf trend --input <report.json> [--input <report2.json> ...]` consumes one
or more native-fast reports and emits:

```text
ncc.performance.trend_dashboard.v1
```

The dashboard includes report summaries, host labels, benchmark rows and per
benchmark series for nC build time, runtime, binary size and C/Rust runtime
ratios when present. It also preserves the C matrix rows and aggregate
`c_style_corpus` summary. Multiple `--input` flags are cumulative; the CI
aggregate dashboard consumes every uploaded host report instead of overwriting
earlier inputs.

## Performance Budgets

`ncc perf budget --input <report.json> [--input <report2.json> ...]` emits:

```text
ncc.performance.budget.v1
```

The budget report is row based. For every benchmark it records the host class,
the nC build/run timings, the fastest measured real C matrix row, nC/C ratios
and any failed budget checks. Host classes currently have explicit thresholds
for Linux, Windows and macOS on x86_64 and AArch64, plus a conservative generic
fallback for other CI hosts.

Rows with a C baseline compare nC against the best successful
`clang/gcc x O2/O3` row. Rows without a C baseline are retained as `inventory`
and do not count as regressions. A row becomes `warning` when the C evidence is
incomplete but not failed, and `regressed` when runtime, build time, nC/C ratio
or historical ratio exceeds the host policy.

Multiple `--baseline` flags are optional and cumulative. A baseline may be a
native-fast report or a retained `ncc.performance.budget.v1` report from an
earlier CI run. When provided, current nC build and run timings are compared
against the matching previous `host_class::benchmark` row.
`--fail-on-regression` exits non-zero after writing the JSON report if any row
is `regressed`, so CI retains the artifact that explains the failure.

## CI Historical Baseline Promotion

The multi-host workflow promotes the previous successful retained aggregate
artifact into the next run's baseline. During the aggregate job it:

- resolves the latest successful `ncc-backend-industrial-multi-host.yml` run on
  the comparable branch;
- downloads its `ncc-backend-industrial-performance-aggregate` artifact when it
  is still retained;
- passes the previous `perf-budget-report.json` to `ncc perf budget` through
  `--baseline`;
- writes `perf-baseline-source.json` and `perf-baseline-inputs.txt` describing
  the baseline source.

If no retained baseline exists yet, the workflow writes a `missing` baseline
source manifest and still runs the host-class budgets. It must not fabricate
historical rows.

## Multi-Host Performance Certification

`ncc perf certify --input <report.json> [--input <report2.json> ...]` emits:

```text
ncc.performance.multi_host_certification.v1
```

The certificate is `certified` when every input is a native-fast report, every
report status is `ok`, and every measured nC benchmark in those reports built
and ran successfully. With two or more distinct report host labels the
certificate level becomes `multi_host`; a single report is recorded as a
`single_host_artifact`.

Multiple `--input` flags are cumulative for certification too. The certificate
keeps one host row per input report, records report count, distinct host count
and an aggregate C-style corpus summary.

The certificate covers measured CI artifacts only. It does not claim arbitrary
hardware parity or full optimizer parity against C/C++/Rust.

## JSON Compatibility

The top-level schema remains:

```text
ncc.performance.native_fast.compare.v1
```

The historical top-level `cases` field is preserved and points at the small
benchmark in the fixed suite so older tests and tooling keep working. In corpus
mode, `cases` points at the first deterministic corpus fixture. New tooling
should prefer `benchmarks[]`.

The C-style additions are additive:

```text
c_style_corpus
optimizer_guidance
benchmarks[].c_matrix
benchmarks[].c_matrix_summary
benchmarks[].overhead_vs_c_matrix
```

## Honesty Rules

The harness must not invent external evidence.

- If `cc` is missing, the C case reports `missing_toolchain`.
- If `rustc` is missing, the Rust case reports `missing_toolchain`.
- If the nC native route fails, the nC case reports `failed` and includes the
  error.
- A report with missing external toolchains may still be structurally valid, but
  it is not a full C/Rust comparison.

## Boundary

This harness closes the local measurement surface, long-running corpus mode,
trend-dashboard artifact shape, measured-host certification contract and
host-class budget report with retained CI baseline promotion. It does not close
total performance parity. It is the baseline that later optimizer, allocator,
runtime and FFI work must use.
