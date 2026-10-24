# Basal Report Surface

`report:` is the official basal output surface. It produces structured basal report intent and does not depend on runtime printing.

Baseical forms:

```nb
report:
    status "root::validated";
    emit output_site;
    emit mem[output_site];
    emit hash(output_site);
```

Reportable values include width types, `addr`, `bool`, `char`, `str`, `bytes<N>`, sites, memory views, hashes, root summaries, site summaries, bind summaries, lowering summaries, and NBB summaries.

Runtime output calls are rejected with `E_BASAL_REPORT_RUNTIME_PRINT_FORBIDDEN_001`.
