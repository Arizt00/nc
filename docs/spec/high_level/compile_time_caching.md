# Compile-time Caching

Pure const eval is cacheable because it has no side effects, bounded execution, checked arithmetic and visibility-safe metadata. A cache key can be built from source spans, expression material, const function identity, type metadata tokens and phase policy version.

`comptime unsafe` is not automatically cacheable. Its audit can mark a block as non-deterministic or cache disabled. Filesystem writes/deletes, process execution, system state reads, dynamic eval authority, side-effect reports and `noLimit` disable pure const caching.

The model separates cacheability from success. A comptime unsafe block may pass validation and still declare that its result should not be cached as a pure value.
