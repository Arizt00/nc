# Machine Roundtrip Mach-O

Mach-O roundtrip reads `__TEXT,__text`, load-command section metadata, symbols and relocation metadata.

Object-format symbol decoration is an explicit normalization rule. It is accepted only when the semantic symbol target is identical.
