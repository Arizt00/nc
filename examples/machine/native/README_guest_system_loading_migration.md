# Guest loading and migration runbook

The init program is [guest_init_poweroff_x86_64.nm](guest_init_poweroff_x86_64.nm). It is assembled and
linked by ncc, loaded as `/init`, loads VirtIO modules, reads `/dev/vda`, emits progress markers, syncs and
powers off through Linux. It contains no C source and the executed VM path does not use QEMU.
The retained bundle is self-contained: boot inputs are copied under `sources/`, hashed and can be
verified after the original files are removed or the bundle is moved to another directory.

```bash
ncc machine build-exe \
  examples/machine/native/guest_init_poweroff_x86_64.nm \
  --target x86_64-unknown-linux-gnu \
  --out target/guest-init

ncc machine virtualization boot-guest \
  --boot bzimage \
  --kernel bzImage \
  --initrd initrd.cpio.gz \
  --read-only-disk disk.raw \
  --memory-mib 512 \
  --vcpus 1 \
  --expect-serial NCC_VIRTIO_DISK_OK \
  --bundle target/evidence/guest-linux \
  --mmio-trace-out target/evidence/guest-linux/mmio-trace.json

ncc machine virtualization verify-guest-certificate \
  target/evidence/guest-linux/guest-certificate.json

ncc machine virtualization certify-migration \
  --memory-mib 16 \
  --vcpus 8 \
  --bundle target/evidence/live-migration

ncc machine virtualization verify-migration-certificate \
  target/evidence/live-migration/migration-certificate.json
```
