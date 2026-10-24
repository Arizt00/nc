# Contributing to nC

Start with behavior another developer can reproduce: a compiler failure, incorrect diagnostic, broken example or inaccurate contract.

## Reproductions

Include host and architecture, compiler version or SHA-256, runtime provider, exact command, minimal source, observed output and expected result. Distinguish the native kit from Stage0. [Current status](docs/public/estado.md) describes the supported routes.

Syntax proposals must explain the decision they express and its representation in parser, AST, semantic analysis, IR and materialization. Parser acceptance alone does not close a phase.

## Code changes

Keep each change focused on a concrete problem. Preserve established diagnostics unless their revision is part of the change. Add positive and negative fixtures at contract boundaries. ABI, memory and backend changes must identify the profile and emitted behavior being checked.

Invoke an already-built compiler for language fixtures. With Stage0 available, run from the repository root on Linux or WSL:

```bash
./target/debug/ncc check examples/00_hello/hello.n
./target/debug/ncc emit-ir examples/11_machine/checked_add_overflow_machine.n
```

On Windows, use `.\target\debug\ncc.exe`. For supported native-kit routes, use `compiler sema` and `compiler emit-ir`. These are distinct interfaces.

Rust implementation changes also require Rust formatting and tests. Configure the seed as described in [installation](docs/install.md):

```bash
cargo fmt --all -- --check
cargo test --locked --workspace --no-fail-fast -- --nocapture
```

Some suites need additional tools or environments. Report executed checks, untested routes and exact failures. Do not weaken expectations to conceal a regression.

## Documentation and website

Editable manuals are in `docs/public/`. Generate and check the static website with:

```bash
node tools/publishing/build-site.mjs
node tools/publishing/check-public.mjs
```

The website generator needs only Node. PDF generation uses `build-manuals.mjs` and Playwright. These are documentation dependencies, not language runtime dependencies. Check links, search, mobile layout and examples after an edit. Public documentation and contribution discussions use English.

## Submitting a change

Describe the concrete problem, before-and-after behavior, affected contract, validation and remaining limits. Keep changes reviewable and preserve source provenance.

Discuss technical decisions respectfully. Do not include credentials, personal data, caches or reports containing private paths in issues or pull requests.
