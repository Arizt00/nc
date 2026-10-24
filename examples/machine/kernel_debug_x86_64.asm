global debugEntry
section .text

debugEntry:
    int3
    int1
    ud2
    mov rax, dr0
    mov dr7, rax
    ret
