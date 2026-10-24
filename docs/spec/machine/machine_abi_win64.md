# ABI Win64

Win64 contract:

| Field | Value |
|---|---|
| args | `rcx rdx r8 r9` |
| return | `rax` |
| shadow space | 32 bytes |
| stack align | 16 |
| red zone | forbidden |
| callee-saved | `rbx rbp rdi rsi r12 r13 r14 r15` |

Run `ncc machine abi win64 --json`.
