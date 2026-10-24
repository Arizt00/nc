# nC - B1 Backend Tiers

Status: Active Base
Version: v2.0.0-B1

## 1. Tier Definitions

### Tier1

Guaranteed production line:

- CI coverage required;
- official diagnostics required;
- deterministic backend behavior required;
- release-critical path.

### Tier2

Supported but not release-critical:

- functional coverage required;
- narrower CI matrix allowed;
- may have constraints in tooling integration.

### Tier3

Experimental or specialized:

- no production guarantee;
- explicit caveats required;
- may be partial by design.

## 2. Governance Rule

Tier is part of the backend contract (`BackendSupportTierIR`).

Support promises must align with tier declaration.
No implicit promotion by convenience.

## 3. Promotion Requirements

A target can move up a tier only after:

- stable target spec;
- passing conformance tests for its lane;
- explicit docs update in matrix/status artifacts.
