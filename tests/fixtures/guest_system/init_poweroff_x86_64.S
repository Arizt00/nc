.section .rodata
message:
    .ascii "NCC_GUEST_BOOT_OK\n"
message_end:
dev_dir:
    .asciz "/dev"
dev_console:
    .asciz "/dev/console"
dev_serial:
    .asciz "/dev/ttyS0"
dev_kmsg:
    .asciz "/dev/kmsg"
devtmpfs:
    .asciz "devtmpfs"

.section .text
.global _start
.type _start, @function
_start:
    mov $83, %rax
    lea dev_dir(%rip), %rdi
    mov $0755, %rsi
    syscall

    mov $165, %rax
    lea devtmpfs(%rip), %rdi
    lea dev_dir(%rip), %rsi
    lea devtmpfs(%rip), %rdx
    xor %r10, %r10
    xor %r8, %r8
    syscall

    mov $2, %rax
    lea dev_serial(%rip), %rdi
    mov $0x902, %rsi
    xor %rdx, %rdx
    syscall
    test %rax, %rax
    jns 1f

    mov $2, %rax
    lea dev_console(%rip), %rdi
    mov $2, %rsi
    xor %rdx, %rdx
    syscall
    test %rax, %rax
    js 2f

1:
    mov %rax, %r12

    mov $33, %rax
    mov %r12, %rdi
    mov $1, %rsi
    syscall
    mov $33, %rax
    mov %r12, %rdi
    mov $2, %rsi
    syscall

2:
    mov $1, %rax
    mov $1, %rdi
    lea message(%rip), %rsi
    mov $(message_end - message), %rdx
    syscall

    mov $2, %rax
    lea dev_kmsg(%rip), %rdi
    mov $0x801, %rsi
    xor %rdx, %rdx
    syscall
    test %rax, %rax
    js 3f
    mov %rax, %rdi
    mov $1, %rax
    lea message(%rip), %rsi
    mov $(message_end - message), %rdx
    syscall

3:
    mov $162, %rax
    syscall

    mov $169, %rax
    mov $0xfee1dead, %rdi
    mov $0x28121969, %rsi
    mov $0x4321fedc, %rdx
    xor %r10, %r10
    syscall

4:
    mov $34, %rax
    syscall
    jmp 4b
.size _start, . - _start
