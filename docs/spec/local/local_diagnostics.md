# local diagnostics

Baseical diagnostics introduced for the local surface:

| Diagnostic | Meaning |
| --- | --- |
| `E_LOCAL_DECL_001` | malformed local declaration |
| `E_LOCAL_DUP_SYMBOL_001` | duplicate local value, const, metadata or field |
| `E_LOCAL_TYPE_DUP_001` | duplicate local type |
| `E_LOCAL_TYPE_UNKNOWN_001` | unknown local type reference |
| `E_LOCAL_TYPE_RECURSIVE_001` | recursive local type definition |
| `E_LOCAL_TYPE_ANY_LEAK_001` | `Any` appears in local type surface |
| `E_LOCAL_CONST_EVAL_001` | local const initializer is not valid compile-time expression |
| `E_LOCAL_CONST_MUTATION_001` | assignment attempts to mutate local const |
| `E_LOCAL_PUBLIC_PRIVATE_TYPE_001` | public local type exposes a private marker |
| `E_LOCAL_MODE_FORBIDDEN_001` | `local:` used in machine, primitive or kernel context |

Diagnostic policy:

- do not reuse machine diagnostics for high-level local errors;
- do not silently accept forbidden contexts;
- do not silently downgrade `local const` to mutable state.
