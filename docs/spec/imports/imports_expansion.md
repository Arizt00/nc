# Import Expansion

`use::...` expands before semantic import resolution.

Examples:

```n
use stoud.io { output, render, input };
```

expands to:

```n
use stoud.io { output, render, input };
```

```n
use stoud.io { fs };
```

expands to the official file IO runtime module.

The operation itself is written in code:

```n
writeFile("out.txt", "text");
```

```n
use auth.system { process };
```

expands to official available authority paths while preserving high-risk metadata in manifests and doctors.

## Backend Activation Expansion

Semantic expansion is followed by backend activation lowering:

```text
ncc imports backend examples/imports/stoud_io_basic.n --json
```

Every supported import item receives a symbol:

```text
__nc_import_activate_<domain>_<family>_<item>
```

The symbol emits concrete activation bytes per MCT profile. The activation
bytes prove backend materialization and linkage shape; runtime side effects
remain owned by the expanded runtime/system/domain symbol.
