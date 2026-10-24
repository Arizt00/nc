global transferPortBlocks
section .text

transferPortBlocks:
    insb
    insw
    insd
    outsb
    outsw
    outsd
    ret
