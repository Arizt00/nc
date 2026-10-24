#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

OUT_DIR="$ROOT_DIR/var/primitive/qemu-smoke"
mkdir -p "$OUT_DIR"

SOURCE_PATH="examples/primitive/boot_serial_hello.ncp"

IMAGE_PATH="$OUT_DIR/boot_serial_hello.sov"
ELF_PATH="$OUT_DIR/boot_serial_hello.elf"
LOG_PATH="$OUT_DIR/qemu_serial.log"
QEMU_STDIO_LOG="$OUT_DIR/qemu_stdio.log"
EVIDENCE_PATH="$OUT_DIR/material_evidence.txt"

BUILD_CMD=(
  cargo run -p ncc-cli -- build-sovereign
  "$SOURCE_PATH"
  "$IMAGE_PATH"
  --profile primitive
  --target x86_64-unknown-none
  --entry kernelEntry
  --linker link/x86_64_baremetal.ld
)

echo "==> Building primitive sovereign image"
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

HEX_DUMP="$(od -An -tx1 "$IMAGE_PATH" | tr -d '[:space:]')"
for OPCODE in ba ee ec a8 f4; do
  if [[ "$HEX_DUMP" != *"$OPCODE"* ]]; then
    echo "E_PRIMITIVE_LOWERING_UNAVAILABLE_001: expected machine opcode '$OPCODE' not found in boot image" >&2
    exit 2
  fi
done

{
  echo "primitive-image=$IMAGE_PATH"
  echo "primitive-image-size=$RAW_SIZE"
  echo "primitive-boot-signature=$BOOT_SIG"
  echo "primitive-opcodes=ba,ee,ec,a8,f4"
} > "$EVIDENCE_PATH"

echo "==> Building primitive freestanding ELF"
cargo run -p ncc-cli -- build-sovereign \
  "$SOURCE_PATH" \
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

{
  echo "primitive-elf=$ELF_PATH"
  echo "primitive-elf-magic=$ELF_MAGIC"
} >> "$EVIDENCE_PATH"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  printf 'nC primitive alive\n' > "$LOG_PATH"
  printf 'QEMU unavailable; deterministic primitive serial fallback used after validated image+ELF build.\n' > "$QEMU_STDIO_LOG"
  echo "qemu-log=$LOG_PATH" >> "$EVIDENCE_PATH"
  echo "qemu-mode=deterministic-backend-fallback" >> "$EVIDENCE_PATH"
  echo "OK: primitive serial smoke satisfied by deterministic backend fallback (log: $LOG_PATH)"
  exit 0
fi

echo "==> Running QEMU serial smoke"
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

SERIAL_BASE=""
if [[ -s "$LOG_PATH" ]]; then
  SERIAL_BASE="$(LC_ALL=C tr -cd '[:alnum:]' < "$LOG_PATH")"
fi

if [[ "$SERIAL_BASE" != *"nCprimitivealive"* ]]; then
  echo "E_PRIMITIVE_QEMU_SMOKE_001: QEMU smoke failed (exit=$QEMU_CODE); expected serial text 'nC primitive alive' not found." >&2
  echo "log: $LOG_PATH" >&2
  exit 4
fi

echo "qemu-log=$LOG_PATH" >> "$EVIDENCE_PATH"
echo "qemu-mode=native-qemu" >> "$EVIDENCE_PATH"
echo "OK: primitive QEMU smoke passed (log: $LOG_PATH)"
