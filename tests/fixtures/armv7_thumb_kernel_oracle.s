.syntax unified
.thumb
.cpu cortex-m4
.text
.global probe
probe:
    mrs r0, primask
    msr primask, r0
    mrs r1, control
    msr control, r1
    ldrex r0, [r1]
    strex r2, r0, [r1]
    clrex
    ands r0, r1
    orrs r0, r1
    eors r0, r1
    muls r0, r1, r0
    lsls r0, r0, #3
    lsrs r0, r0, #4
    asrs r0, r0, #5
    blx r3
    cbz r0, done
    cbnz r1, done
done:
    bx lr
