# Basal Verify And Probe

`verify:` is the official basal verification surface. It can call basal routines, inspect constants, inspect sites, inspect `mem[...]`, use `dev(...)`, and produce verification records for later evidence phases.

`probe:` remains a documented transitional alias. The parser accepts it, records `E_BASAL_PROBE_ALIAS_001` as a warning, and maps it into the verify surface. New docs and examples use `verify:`.

Phase 5.5 does not execute final machine evidence. Verification is modeled, checked for known routine calls, and preserved in BasalIR/lowering/NBB surface records.
