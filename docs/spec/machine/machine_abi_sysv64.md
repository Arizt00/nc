# ABI SysV64

SysV64 contract:

| Field | Value |
|---|---|
| args | `rdi rsi rdx rcx r8 r9` |
| return | `rax` |
| stack align | 16 |
| red zone | allowed |
| callee-saved | `rbx rbp r12 r13 r14 r15` |
| caller-saved | `rax rcx rdx rsi rdi r8 r9 r10 r11` |

Run `ncc machine abi sysv64 --json`.
