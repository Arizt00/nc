# Machine Bridge To Basal

Basal requires machine evidence for final `.nbb` lowering. Machine now provides
closed declared-profile evidence beyond the original Level 4 x86_64 object
milestone.

Remaining basal dependency is basal-owned: consuming the machine evidence in the
`.nbb` writer/seal flow. Updated machine multi-host CI artifacts are still
required before claiming the latest exact-head external workflow run.

Bridge rule: basal may reference machine only after `ncc machine doctor` reports
a non-broken Level 4 or higher state; current declared-profile closure satisfies
that threshold.
