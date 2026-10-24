# primitive target matrix

| Target | Environment | Execution | Max Evidence | Protocols |
|---|---|---|---|---|
| x86_64-unknown-none | freestanding | artifact | freestanding_built | UART |
| x86_64-protocol-lab | emulated-qemu | qemu | qemu_emulated | UART, PCIE, USB, SATA, NVME, I2C, SPI, ETHERNET, HDMI, DISPLAYPORT |
| x86_64-linux-host-serial | host_real_serial | host serial device | hardware_observed | UART |
| x86_64-linux-portio | privileged_host | privileged port I/O | blocked | UART |
| generic-mmio-lab | gated-mmio-lab | safe MMIO mapping | blocked | UART |
