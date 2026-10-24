# nC - B1 Runtime Boundary

Status: Active Base
Version: v2.0.0-B1

## 1. Core Rule

nC must keep a strict boundary between backend, runtime and host.

## 2. Backend Responsibilities

Backend owns:

- lowering from typed IR into target-representable ops;
- calling convention and frame discipline;
- data layout enforcement;
- object/module emission contracts;
- relocation modeling;
- target metadata propagation (`target_spec`).

Backend does not own process services or host syscalls.

## 3. Runtime Responsibilities

Runtime owns:

- builtin behavior at execution time;
- console family semantics and render modes;
- managed runtime memory services;
- runtime-level inspection and trace services.

Runtime does not decide machine ABI/layout policies.

## 4. Host Responsibilities

Host owns:

- OS process model and loader behavior;
- real stdout/stderr endpoints;
- filesystem/network/time facilities (unless embedded mode replaces them);
- external linker/toolchain execution where applicable.

## 5. Anti-Drift Rule

If a concern is not clearly assigned to one of the three boundaries, it is a critical design debt and must be documented before expansion.
