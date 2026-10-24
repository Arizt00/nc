# Basal Release Seal

Basal Phase 7 seals the release state after Phase 6 integrated real machine
evidence and final seal verification into NBB0.

The release seal covers:

- final basal doctor state
- machine doctor and native-doctor state
- conformance matrix
- evidence bundle manifest
- CLI matrix
- examples and negative examples matrices
- search gate output
- worktree classification and commit plan

The release seal is documentation and evidence packaging. It does not change
Basal parser, tables, IR, lowering, NBB0 serialization, machine evidence, or
final seal semantics.

The release seal is not considered clean until the worktree is classified. It
does not delete inherited dirty files and does not hide unrelated debt.
