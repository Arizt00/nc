# High Level Fase 15 Risk Authority Overview

High Level Fase 15 closes the risk authority and expert policy surface inside the existing high-level language. It does not create a separate mandatory `expert {}` mode. Authority is declared through ordinary high-level sections so the same parser, AST, semantic verifier, CLI, reports, diagnostics, source spans, and release doctors remain the single source of truth.

The official section is:

```n
risk:
    allow race;
    accept dataRace "sharedCounter";
    report raceAuthority;
```

`policy:` and `authority:` are compatibility aliases for the same AST section. Reports normalize them to the high-level risk authority model.

Fase 15 does not make risky code safe by declaration. It makes risk explicit, auditable, and rejectable when authority is missing. The compiler accepts declared risk only when the corresponding `allow` capability is present, spans are preserved, diagnostics are official, and the report records the accepted risk.

Closed in this phase:

- risk policy parsing and AST.
- capability allow gates.
- risk accept diagnostics.
- low-level bridge authority.
- no-limit authority.
- risk/authority CLI reports.
- unification preparation metadata for future high-level/runtime/compiler work.

Not started:

- Fase 16 unified doctor.
- runtime/compiler merge.
- any separate expert syntax mode.
