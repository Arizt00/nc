# Basal Foreign C++ ABI

Basal does not implement C++ as a source language. It declares C++ ABI contracts
through `foreign:` and proves that those contracts can be compiled, linked,
executed, hashed, and sealed as external evidence.

The official C++ ABI surface for Basal is:

```n
foreign:
    cpp_template_int:
        language: cpp;
        symbol: nc_cpp_call_template_int;
        abi: itanium-cxxabi;
        layout: repr_cxx;
        ownership: caller;
        header: "nc_cpp_abi_full.hpp";
        artifact: "libnc_cpp_abi_full.a";
        link: static;
```

The required fields are the same as all Basal foreign bindings:
`language`, `symbol`, `abi`, `layout`, and `ownership`. C++ contracts should use
`language: cpp` and an explicit ABI such as `itanium-cxxabi`. Static and dynamic
link declarations are preserved as contract metadata and evidence artifact kind.

Closed C++ ABI evidence currently covers:

- Itanium-compatible symbol contracts, including selected mangled symbols.
- Template instantiation through real C++ compilation.
- Virtual dispatch through a vtable-backed class hierarchy.
- Destructor and RAII execution.
- Exception boundaries caught inside the C++ ABI lane.
- Static archive and shared library declarations as Basal contract metadata.
- Object/executable hashing in the NBB0 `external_artifacts` evidence record.

`basal integrate-evidence` generates a real C++ source lane for each declared
symbol, compiles it with `c++`, compiles a C driver with `cc`, links both
artifacts with `c++`, executes the result, and stores hashes in the NBB0 machine
evidence section. If any command fails, Basal emits
`E_BASAL_EVIDENCE_EXTERNAL_LINK_001` and does not claim integrated evidence.

The full C++ toolchain conformance matrix remains in compiler:

```bash
ncc compiler ffi-cpp-abi --json --strict
```

Basal depends on that closed compiler contract and records the external
artifact evidence inside the root seal. It does not add C++ imports, does not
change NBB0 magic, and does not invent fake C++ evidence.
