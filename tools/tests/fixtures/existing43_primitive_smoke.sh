#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
out="$root/var/primitive/qemu-smoke"
mkdir -p "$out"
cp -p "$root/test-input/boot_serial_hello.sov" "$out/boot_serial_hello.sov"
cp -p "$root/test-input/boot_serial_hello.elf" "$out/boot_serial_hello.elf"
cp -p "$root/test-input/material_evidence.txt" "$out/material_evidence.txt"
cp -p "$root/test-input/qemu_serial.log" "$out/qemu_serial.log"
printf '%s\n' 'CONTROLLED PRIMITIVE TEST INPUTS, NOT QEMU OR PHYSICAL EXECUTION'
case "${EXISTING43_PRIMITIVE_CASE:-}" in
    command-nul) printf 'before\0after\n'; printf 'error\0tail\n' >&2 ;;
    command-newlines) printf '\v\f\rfirst\r\nsecond\rthird\r\n\v\f'; printf '\v\ferr\r\nline\r\v\f' >&2 ;;
esac
exit "${EXISTING43_PRIMITIVE_SMOKE_CODE:-0}"
