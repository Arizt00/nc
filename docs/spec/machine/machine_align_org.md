# Machine Align And Org

`align` pads directive data to a power-of-two boundary and records the padding evidence hash. Bad alignment values use `E_MACHINE_ALIGN_001`.

`org` is policy gated. It is allowed only in raw, flat, firmware, baremetal, or kernel-style profiles. Object-oriented userland profiles reject it with `E_MACHINE_ORG_FORBIDDEN_001`.

