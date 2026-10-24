# Machine VEX Encoding Patterns

Fase 1A records VEX encoding patterns without declaring full VEX encoding complete.

Each `VexEncodingPattern` captures prefix bytes, VEX2 fields, VEX3 fields, opcode bytes, ModRM policy, SIB policy, immediate policy, displacement policy, relocation policy and byte-pattern examples where native bytes already exist.

The VEX2 field model is:

- inverted R;
- vvvv;
- L;
- pp.

The VEX3 field model is:

- inverted R;
- inverted X;
- inverted B;
- m-mmmm;
- W;
- vvvv;
- L;
- pp.

Fase 1 remains open until every table entry with status `closed` has native encoding, verifier coverage, tests, examples, docs and report evidence.
