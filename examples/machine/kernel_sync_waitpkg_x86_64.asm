global waitForKernelEvent
section .text

waitForKernelEvent:
    mfence
    umonitor rax
    umwait eax
    tpause eax
    serialize
    ret
