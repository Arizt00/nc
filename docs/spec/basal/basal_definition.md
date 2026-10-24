# Basal Definition

Basal is the root semantic-binary authority for nC programs. It defines identity, contract, binary shape, site, seal, evidence, and bridge intent before runtime, compiler, primitive, or machine behavior is allowed to materialize anything.

Normative definitions:

| Term | Meaning |
|---|---|
| `[@]` | Root modal context. |
| `.nb` | Textual basal semantic source. |
| `.nbb` | Binary basal source with magic, version, hashes, and payload. |
| root container | A named container declared as `name[@] { ... }`. |
| root symbol | A symbol under root authority, usually declared in `stc:`. |
| root contract | The `root:` identity and authority record. |
| root binary | The `binary:` contract describing stable binary form. |
| root site | A semantic-material location declaration, not a physical address. |
| root seal | A hash-bound closure over root contract, binary contract, and evidence. |
| root evidence | Reportable proof material for the root contract. |

Basal can declare contracts. It cannot silently import runtime conveniences, perform primitive mutation, or request machine lowering while machine closure is absent.
