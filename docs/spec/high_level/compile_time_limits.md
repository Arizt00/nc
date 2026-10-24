# Compile-time Limits

Pure const eval has bounded execution:

- step budget;
- node budget;
- recursion budget;
- checked integer arithmetic;
- aggregate and string size accounting;
- exhaustive enum match checking.

Loops are rejected from the pure subset. Recursive const functions are diagnosed rather than executed indefinitely. Indexing outside aggregate bounds fails. These rules keep const eval deterministic and suitable for caching.
