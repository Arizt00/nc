#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="$ROOT_DIR/var/primitive/hardware-identity-qemu-smoke"
mkdir -p "$OUT_DIR"

IMAGE_PATH="$OUT_DIR/hardware_identity_uart.sov"
ELF_PATH="$OUT_DIR/hardware_identity_uart.elf"
LOG_PATH="$OUT_DIR/qemu_serial.log"
QEMU_STDIO_LOG="$OUT_DIR/qemu_stdio.log"
EVIDENCE_PATH="$OUT_DIR/material_evidence.json"

BUILD_CMD=(
  cargo run -p ncc-cli -- build-sovereign
  examples/primitive/hardware_identity_uart.ncp
  "$IMAGE_PATH"
  --profile primitive
  --target x86_64-unknown-none
  --entry kernelEntry
  --linker link/x86_64_baremetal.ld
)

echo "==> Building primitive hardware identity image"
"${BUILD_CMD[@]}"

if [[ ! -s "$IMAGE_PATH" ]]; then
  echo "E_PRIMITIVE_LOWERING_UNAVAILABLE_001: build produced empty image at $IMAGE_PATH" >&2
  exit 2
fi

RAW_SIZE="$(stat -c%s "$IMAGE_PATH")"
if [[ "$RAW_SIZE" != "512" ]]; then
  echo "E_PRIMITIVE_LOWERING_UNAVAILABLE_001: boot image must be exactly 512 bytes (found $RAW_SIZE)" >&2
  exit 2
fi

BOOT_SIG="$(od -An -tx1 -j510 -N2 "$IMAGE_PATH" | tr -d '[:space:]')"
if [[ "$BOOT_SIG" != "55aa" ]]; then
  echo "E_PRIMITIVE_LOWERING_UNAVAILABLE_001: boot image is missing 0x55AA signature (got $BOOT_SIG)" >&2
  exit 2
fi

echo "==> Building primitive hardware identity ELF"
cargo run -p ncc-cli -- build-sovereign \
  examples/primitive/hardware_identity_uart.ncp \
  "$ELF_PATH" \
  --profile primitive \
  --target x86_64-unknown-none \
  --entry kernelEntry \
  --linker link/x86_64_baremetal.ld \
  --image-format elf

if [[ ! -s "$ELF_PATH" ]]; then
  echo "E_PRIMITIVE_ELF_EMIT_001: build produced empty ELF at $ELF_PATH" >&2
  exit 2
fi

ELF_MAGIC="$(od -An -tx1 -N4 "$ELF_PATH" | tr -d '[:space:]')"
if [[ "$ELF_MAGIC" != "7f454c46" ]]; then
  echo "E_PRIMITIVE_ELF_EMIT_001: invalid ELF magic for $ELF_PATH (got $ELF_MAGIC)" >&2
  exit 2
fi

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "E_PRIMITIVE_CLI_QEMU_MISSING_001: missing dependency 'qemu-system-x86_64'" >&2
  exit 3
fi

echo "==> Running QEMU serial smoke (emulated environment)"
rm -f "$LOG_PATH"
QEMU_CMD=(
  qemu-system-x86_64
  -nographic
  -serial "file:$LOG_PATH"
  -monitor none
  -no-reboot
  -no-shutdown
  -drive "format=raw,file=$IMAGE_PATH,if=floppy"
  -boot a
)

if command -v timeout >/dev/null 2>&1; then
  set +e
  timeout 8s "${QEMU_CMD[@]}" >"$QEMU_STDIO_LOG" 2>&1
  QEMU_CODE=$?
  set -e
else
  set +e
  "${QEMU_CMD[@]}" >"$QEMU_STDIO_LOG" 2>&1
  QEMU_CODE=$?
  set -e
fi

if [[ ! -s "$LOG_PATH" ]]; then
  echo "E_PRIMITIVE_QEMU_SMOKE_001: qemu serial output was empty" >&2
  exit 4
fi

LOG_HEX="$(od -An -tx1 "$LOG_PATH" | tr -d '[:space:]')"
EXPECTED_HEX="6e43207072696d697469766520616c697665"

if [[ "$LOG_HEX" != *"$EXPECTED_HEX"* ]]; then
  echo "E_PRIMITIVE_QEMU_SMOKE_001: expected serial bytes for 'nC primitive alive' not found" >&2
  echo "log: $LOG_PATH" >&2
  exit 4
fi

cat > "$EVIDENCE_PATH" <<JSON
{
  "schema": "nc.primitive.qemu.smoke.v1",
  "environment_class": "emulated-qemu",
  "physical_hardware_certified": false,
  "image": "$IMAGE_PATH",
  "image_size": $RAW_SIZE,
  "boot_signature": "$BOOT_SIG",
  "elf": "$ELF_PATH",
  "elf_magic": "$ELF_MAGIC",
  "serial_log": "$LOG_PATH",
  "expected_ascii": "nC primitive alive",
  "expected_hex": "$EXPECTED_HEX",
  "qemu_exit": $QEMU_CODE
}
JSON

echo "OK: primitive hardware identity QEMU smoke passed (emulated evidence)"
echo "evidence: $EVIDENCE_PATH"
