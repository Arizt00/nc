# Machine Mode

Baseical machine source uses `.nm` and a `[@machine]` container.

Allowed sections: `target:`, `abi:`, `section:`, `symbol:`, `extern:`, `export:`, `entry:`, `proc:`, `routine:`, `block:`, `label:`, `instr:`, `frame:`, `stack:`, `reloc:`, `verify:`, `emit:`, and `seal:`.

Forbidden sections: `fn:`, `fa:`, `stc:`, and high-level `type:`.

Direct runtime imports are rejected. Primitive physical mutation is not a machine-mode shortcut. Basal lowering may depend on machine only after machine evidence gates pass.
