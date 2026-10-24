# WASM Machine Artifact

`ncc machine emit-wasm <file.nm> --target wasm32-unknown-unknown --out <file.wasm>` emits a WASM module for the `wasm32` machine target.

Current release evidence covers a minimal exported `main` routine returning `i32`. The module is verified by magic/version through `ncc machine verify-object` and executed locally through Node in `ncc machine link-smoke`.

Evidence fixture: `examples/machine/wasm32_const_main.nm`.
