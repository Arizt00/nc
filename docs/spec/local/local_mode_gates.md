# local mode gates

`local:` is allowed in normal/high-level nC containers and selected high-level mode containers.

Allowed:

- normal nC containers;
- `[@runtime]`;
- `[@compiler]`;
- `[@system]`;
- `[@security]`;
- `[@inspect]`;
- `[@ai]`.

Forbidden:

- `[@machine]`;
- `[@primitive]`;
- `[@kernel]`;
- basal `.nb`.

The parser emits `E_LOCAL_MODE_FORBIDDEN_001` for machine, primitive and kernel contexts.
Basal remains on its independent `.nb` parser and is not modified by this surface.
