# Machine Stack And Frame

The SysV64 checker validates stack alignment before `call` and callee-saved restoration for preserved registers.

Supported frame instructions include `push rbp`, `mov rbp, rsp`, `sub rsp, N`, `add rsp, N`, `pop rbp`, `leave`, and `ret`.

Diagnostics: `E_MACHINE_STACK_ALIGN_001`, `E_MACHINE_CALLEE_SAVED_001`, and `E_MACHINE_FRAME_001`.
