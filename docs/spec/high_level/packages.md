# High Level Packages

A package groups local high-level modules and provides distribution metadata for the public API report.

```n
package geometry_core {
    version: "0.1.0";
    profile: stable;
    module math.geometry;
}
```

Rules:

- Package names are lowercase identifiers.
- Versions use `major.minor.patch`.
- Duplicate module entries inside a package are rejected.
- Package profiles can restrict wildcard imports, for example `secure`.
- Package registry and dependency solving are not part of Phase 10.

Diagnostics: `E_HIGH_PACKAGE_NAME_001`, `E_HIGH_PACKAGE_VERSION_001`, `E_HIGH_PACKAGE_MODULE_DUP_001`, `E_HIGH_PACKAGE_EXPORT_PRIVATE_001`, `E_HIGH_PACKAGE_API_SURFACE_001`.
