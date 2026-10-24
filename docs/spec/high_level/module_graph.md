# Module Graph

The module graph is built from declared modules and their section-level imports.

```n
consumer {
    module:
        path: app.consumer;

    use:
        math.geometry.area;
}
```

Edges point from the importing module to the imported module. The semantic
module graph must be acyclic. Cycles fail with `E_HIGH_MODULE_CYCLE_001`.

Wildcard imports contribute graph edges just like exact imports. Top-level legacy imports remain supported for existing runtime/compiler surfaces.

Project graph inspection is broader than semantic module validation. The IDE
graph reads `bridge.json`, `.ncc/code/*.json` and sources to show mothers,
children, edges, duplicate containers and cycles. Project cycles are displayed
for inspection and do not become editor warnings by themselves; semantic module
cycles still remain compiler errors.
