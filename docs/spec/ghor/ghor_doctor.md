# ghor Doctor

`ghor doctor --json` checks:

- repository `ncc` availability;
- machine doctor;
- machine native doctor with `asm_complete_total=true`;
- basal doctor with `basal_closed=true`;
- installed toolchain drift;
- target matrix;
- foreign host tool probes.
- registry/network closure:
  - filesystem registry transport;
  - HTTP and HTTPS registry transport through `curl`;
  - publish, search, info, yank, and fetch;
  - SHA-256 verification;
  - cache writes under `target/ghor/registry-cache`.

Doctor status is `closed`, `warning`, or `error`. Warnings record environment drift without hiding it.
