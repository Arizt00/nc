# Backend MCT/LLVM Declared Parity

Status: closed for declared LLVM/MCT parity before GCC.

This phase closes the handoff gate from LLVM/MCT work into GCC work. The claim
is intentionally precise:

- ncc-native remains integrated;
- LLVM is integrated through MCT for the declared ncc backend surface;
- `ncc mct parity --json` emits the handoff certificate;
- GCC has now entered through the C99 bridge adapter gate;
- arbitrary external LLVM universe parity is not claimed.

The schema is:

```text
nc.mct.declared_parity.v1
```

The closed status is:

```text
mct_llvm_declared_parity_ready_for_gcc
```

## Command

```sh
ncc mct parity --json
ncc mct parity --list
```

The JSON report includes closed gates, non-blocking external-universe limits and
the GCC entry contract.

## Boundary

This phase means the current declared MCT/LLVM surface had no blocking gate
before GCC. The first GCC gate is now integrated as a C99 bridge for the
declared scalar subset. It does not mean ncc implements every LLVM feature,
every target attribute, every possible future language surface, or full GCC
parity itself.
