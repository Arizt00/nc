# SiteIR

Status: closed for Basal Fase 3.

`BasalSiteIr` models normalized site declarations:

- site id.
- site kind.
- optional size and alignment.
- normalized permissions.
- owner root.
- source span.
- semantic status.

The verifier rejects duplicate sites, invalid kinds, invalid sizes, invalid alignments, invalid permissions, write+execute permission combinations, owner/root inconsistencies, and lost source spans.
