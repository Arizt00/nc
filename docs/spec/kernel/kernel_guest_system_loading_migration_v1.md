# Kernel guest loading and migration contract v1

Kernel code keeps the compact nC surface. The loader and migration engine are machine/kernel services;
kernel sources do not import hosted filesystem, process or thread runtimes.

## Ownership

- Machine owns image parsing, physical layout, DMA, queue ordering, object bytes and transport frames.
- Kernel owns VM policy, vCPU lifecycle, pause barriers, device quiesce order and failure recovery.
- Hosted CLI owns files, TCP sockets, retained evidence and certificate verification.

No heap allocation, scheduler, firmware selection or device emulation is inserted implicitly into a
kernel function. Raw addresses and MMIO remain explicit. Snapshot creation requires every vCPU to reach
the pause barrier and every migratable device to serialize its declared schema version.

## Required quiesce order

1. stop new device submissions;
2. drain or version in-flight requests;
3. pause all vCPUs and retain pending interrupts;
4. drain the final dirty generation;
5. capture devices and virtual time;
6. prepare destination state;
7. commit transactionally;
8. resume exactly one side.

Clean guest shutdown maps ACPI S5, UEFI ResetSystem or SBI SRST to one `GuestPowerAction`. Reboot resets
vCPU registers, transport state and asserted IRQs before rematerializing the selected boot plan.
