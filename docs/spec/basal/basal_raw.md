# Basal Raw Surface

`raw` is the explicit basal region for absolute low-level authority. It permits direct address and site memory operations when the source declares that intent.

The compiler does not present raw operations as safe. It requires explicit syntax, width where memory is touched, source spans, hazard records, and an evidence boundary that remains pending until Basal Fase 6.

Baseical form:

```nb
raw {
    value >> dword : mem[@src];
    mem[@dst] >> dword : @value;
}
```

Raw may operate over memory, `addr`, sites, fixed-width bytes, bitwise expressions, shifts, and machine-lowerable primitive operations. The compiler records hazards such as `raw_memory_read`, `raw_memory_write`, `addr_based_access`, width, endian, alignment status, permission status, bounds status, and hazard acceptance.

Permission checks are performed when a site is known. Alignment and bounds checks are performed when the site declares enough metadata. Address-only access remains allowed in raw and is recorded as an address-based hazard.

`raw unchecked` is reserved as a documented future extension. Phase 5.5 recognizes raw hazards but does not convert them into final evidence or a final seal.

Diagnostics include `E_BASAL_RAW_001`, `E_BASAL_RAW_AUTHORITY_001`, `E_BASAL_RAW_ADDR_001`, `E_BASAL_RAW_SITE_001`, `E_BASAL_RAW_MEM_READ_001`, `E_BASAL_RAW_MEM_WRITE_001`, `E_BASAL_RAW_PERMISSION_001`, `E_BASAL_RAW_ALIGN_001`, `E_BASAL_RAW_WIDTH_001`, `E_BASAL_RAW_ENDIAN_001`, `E_BASAL_RAW_BOUNDS_001`, `E_BASAL_RAW_LOWERING_001`, `E_BASAL_RAW_HAZARD_001`, and `E_BASAL_RAW_EVIDENCE_BOUNDARY_001`.

Relationship to machine lowering:

- Raw read operations become load intents in the lowering plan.
- Raw write operations become store intents in the lowering plan.
- Raw hazards carry spans into BasalIR and lowering.
- Raw evidence boundaries stay pending until the evidence phase.

Relationship to NBB0:

- Raw blocks produce executable surface records.
- Hazard summaries are stored as structured surface records.
- The pending evidence section remains pending.

Anti-patterns:

- Memory access without an explicit width.
- Writing a read-only site.
- Treating a raw block as proof of safety.
- Emitting final evidence from raw in Phase 5.5.
