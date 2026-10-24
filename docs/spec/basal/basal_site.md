# Basal Site

`site:` declares semantic-material location. It is not a physical address and does not cross into primitive mutation.

Example:

```n
site:
    rootCounter:
        symbol: counter;
        origin: root;
        offset: 0;
        size: 8;
        alignment: 8;
        owner: root;
        state: sealed;
        evidence: required;
```

A site can feed later lowering, reports, and evidence. It cannot authorize MMIO, port I/O, or direct hardware effects.

Diagnostics include `E_BASAL_SITE_001`, `E_BASAL_SITE_SYMBOL_001`, and `E_BASAL_SITE_PRIMITIVE_CROSSING_001`.
