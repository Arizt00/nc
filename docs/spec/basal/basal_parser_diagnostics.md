# Basal Parser Diagnostics

Phase 1 parser diagnostics are basal-owned and do not reuse machine diagnostics.

| Diagnostic | Meaning |
|---|---|
| `E_BASAL_PARSE_001` | generic parse failure |
| `E_BASAL_EXPECTED_ROOT_001` | missing root declaration |
| `E_BASAL_EXPECTED_SECTION_001` | content outside a section |
| `E_BASAL_EXPECTED_IDENTIFIER_001` | expected identifier |
| `E_BASAL_EXPECTED_COLON_001` | expected `:` |
| `E_BASAL_EXPECTED_SEMICOLON_001` | expected `;` |
| `E_BASAL_EXPECTED_LBRACE_001` | expected `{` |
| `E_BASAL_EXPECTED_RBRACE_001` | expected `}` |
| `E_BASAL_INVALID_LITERAL_001` | invalid literal |
| `E_BASAL_INVALID_PATH_001` | invalid path |
| `E_BASAL_DUPLICATE_SECTION_001` | duplicate top-level section |
| `E_BASAL_UNKNOWN_SECTION_001` | unknown top-level section |
| `E_BASAL_SITE_SYNTAX_001` | invalid `site:` syntax |
| `E_BASAL_BIND_SYNTAX_001` | invalid `bind:` syntax |
| `E_BASAL_SEAL_SYNTAX_001` | invalid `seal:` syntax |
| `E_BASAL_EVIDENCE_SYNTAX_001` | invalid `evidence:` syntax |
| `E_BASAL_RECOVERY_001` | parser recovered and continued |

Each emitted diagnostic includes code, message, severity, span and optional help.
