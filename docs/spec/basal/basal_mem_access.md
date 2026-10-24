# Basal Memory Access

Basal executable source recognizes:

```nb
mem[@addr]
mem[siteName]
mem[@siteRef]
```

Memory access requires width by context:

```nb
value >> dword : mem[@src];
mem[@dst] >> dword : @value;
```

Known sites are checked for read/write permission, declared alignment, declared bounds, and lowering possibility. Address-only raw memory access is allowed and recorded as a hazard with a pending evidence boundary.

Diagnostics include `E_BASAL_MEM_001`, `E_BASAL_MEM_READ_001`, `E_BASAL_MEM_WRITE_001`, `E_BASAL_MEM_WIDTH_001`, `E_BASAL_MEM_SITE_001`, `E_BASAL_MEM_ADDR_001`, `E_BASAL_MEM_PERMISSION_001`, and `E_BASAL_MEM_LOWERING_001`.
