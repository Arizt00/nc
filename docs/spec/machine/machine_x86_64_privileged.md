# Machine x86_64 Privileged

Privileged/system instruction families include `hlt`, `cli`, `sti`, control/debug registers, descriptor tables, MSR operations, invalidation, VMX/SVM, syscall/sysret, and interrupt return.

Current status:

- `hlt` encodes and is profile-gated.
- `cli`, `sti`, `syscall`, and `int` are rejected in userland profile.
- control registers and descriptor-table operations remain gated/deferred.

The verifier must keep userland strict: privileged instructions require explicit profile authority.
