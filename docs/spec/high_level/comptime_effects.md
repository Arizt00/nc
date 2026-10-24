# Comptime Effects

Comptime effects are real host actions performed only by explicit authority. Filesystem effects use real read/write/list/create/delete/metadata calls. Process effects spawn a declared executable or record its status. System effects read host environment/user/path data. Heap, reflection, private metadata, construction leak and side-effect reports mutate only the compile-time audit state for that block.

The audit model records:

- effect kind;
- source location;
- capability;
- deterministic/cacheable flags;
- reproducibility impact;
- detail string;
- whether `noLimit` was active.

The compiler does not treat an audited effect as pure. It reports the effect boundary so later pipeline stages can reproduce or reject it.
