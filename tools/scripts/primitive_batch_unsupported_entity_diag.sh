#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

WORK_DIR="$(mktemp -d)"
TARGET_TMP_PREFIX="tmp-primitive-unsupported-batch"
LOG_DIR="$ROOT_DIR/var/primitive/unsupported_diag_batch"
mkdir -p "$LOG_DIR"
RESULTS_PSV="$LOG_DIR/results.psv"
EVIDENCE_JSON="$LOG_DIR/unsupported_diag_batch_report.json"
EVIDENCE_MD="$LOG_DIR/unsupported_diag_batch_report.md"
>"$RESULTS_PSV"
rm -f "$LOG_DIR/results.tsv"

cleanup() {
  rm -rf "$WORK_DIR"
  rm -f "$ROOT_DIR"/targets/${TARGET_TMP_PREFIX}-*.nctarget
}
trap cleanup EXIT

write_target_file() {
  local target_name="$1"
  local target_file="$ROOT_DIR/targets/${target_name}.nctarget"
  cat >"$target_file" <<EOF
name = "${target_name}"
arch = "x86_64"
isa = "x86_64"
bits = 64
endian = "little"
os = "none"
environment = "freestanding"
binary_format = "elf"
pointer_width = 64
page_size = 4096
runtime = "none"
stdlib = "core-only"

supports_port_io = true
supports_mmio = true
supports_interrupts = true
supports_volatile = true
supports_fence = true
supports_atomic = true
supports_halt = true
supports_serial_com1 = true

supports_dma = true
supports_cache_model = true
supports_tlb_model = true
supports_pipeline_model = true
supports_firmware_model = true
supports_electrical_model = true
supports_clock_model = true
supports_edge_model = true
supports_metastability_model = true
supports_thermal_model = true
supports_power_model = true

supports_power_gate_model = true
supports_thermal_state_model = true
supports_throttle_state_model = true
supports_cache_state_model = true
supports_tlb_state_model = true
supports_pipeline_state_model = true
supports_branch_predictor_model = true
supports_reorder_model = true
supports_out_of_order_model = true

supports_firmware_region_model = true
supports_boot_rom_region_model = true
supports_bios_region_model = true
supports_uefi_handoff_model = true
supports_reset_vector_model = true
supports_boot_window_model = true
supports_boot_anchor_model = true
supports_firmware_table_model = true
supports_device_tree_model = true
supports_acpi_table_model = true
supports_microcode_state_model = true

protocols = ["UART"]
firmware_bios = "supported"
firmware_uefi = "supported"
firmware_bootrom = "supported"
firmware_microcode = "target_declared"
EOF
}

set_target_kv() {
  local target_name="$1"
  local key="$2"
  local value="$3"
  local file="$ROOT_DIR/targets/${target_name}.nctarget"
  if ! sed -i "s#^${key} = .*#${key} = ${value}#g" "$file"; then
    echo "FAIL: unable to set ${key} in ${file}" >&2
    exit 1
  fi
}

run_expect_diag() {
  local case_name="$1"
  local source_file="$2"
  local target_name="$3"
  local expected_diag="$4"
  local expected_diag_secondary="${5:-}"
  local log_file="$LOG_DIR/${case_name}.log"

  if cargo run -p ncc-cli -- primitive check \
    "$source_file" \
    --target "$target_name" \
    --environment freestanding \
    --strict >"$log_file" 2>&1; then
    cat "$log_file"
    echo "FAIL: ${case_name} unexpectedly succeeded; expected ${expected_diag}" >&2
    exit 1
  fi

  if ! grep -Fq "$expected_diag" "$log_file"; then
    cat "$log_file"
    echo "FAIL: ${case_name} missing expected diagnostic ${expected_diag}" >&2
    exit 1
  fi
  if [[ -n "$expected_diag_secondary" ]] && ! grep -Fq "$expected_diag_secondary" "$log_file"; then
    cat "$log_file"
    echo "FAIL: ${case_name} missing expected secondary diagnostic ${expected_diag_secondary}" >&2
    exit 1
  fi

  echo "OK: ${case_name} -> ${expected_diag}"
  printf "%s|%s|%s|%s|%s|%s\n" \
    "$case_name" \
    "$target_name" \
    "$source_file" \
    "$expected_diag" \
    "$expected_diag_secondary" \
    "$log_file" >>"$RESULTS_PSV"
}

escape_json() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

ADVANCED_FIXTURE="$WORK_DIR/advanced_physical_semantics.ncp"
UART_FIXTURE="examples/primitive/hardware_identity_uart.ncp"
USB_FIXTURE="$WORK_DIR/protocol_usb_unsupported.ncp"

cat >"$ADVANCED_FIXTURE" <<'EOF'
primitive[@] target x86_64:
    voltage:
        vcore;
    current:
        rail0;
    noise:
        emi0;
    clock:
        core_clk;
    edge:
        rising;
    metastability:
        cdc0;
    power:
        pstate0;
    powerGate:
        pg0;
    temperature:
        sensor0;
    thermalState:
        thermal_state0;
    throttleState:
        throttle_state0;
    watchdog:
        wd0;
    interruptLine:
        irq0;
    dmaChannel:
        dma0;
    cacheState:
        l1_clean;
    tlbState:
        tlb_hot;
    firmwareRegion:
        fw0;
    bootRomRegion:
        rom0;
    biosRegion:
        bios0;
    uefiHandoff:
        handoff0;
    resetVector:
        reset0;
    bootWindow:
        bootwin0;
    bootAnchor:
        boot0;
    firmwareTable:
        fwtable0;
    deviceTree:
        dt0;
    acpiTable:
        acpi0;
    microcodeState:
        patch_state0;
    controller:
        uart_ctrl;
    peripheral:
        uart0;
    phy:
        uart_phy;
    lane:
        lane0;
    channel:
        ch0;
    pipelineState:
        fetch_decode;
    branchPredictorState:
        bp0;
    reorderState:
        rob0;
    outOfOrder:
        ooo_enabled;
    physicalFault:
        thermal_trip;
    timingViolation:
        missed_deadline;
    thermalViolation:
        overheat;
    metastabilityRisk:
        cdc_risk;
    dmaViolation:
        dma_race;
    cacheCoherenceViolation:
        stale_line;
    mmioSideEffectViolation:
        read_clears_flag;
    capability hw:
        allow port.write;
        allow protocol.uart;
    bus io8:
        width 8;
    port uart0.data:
        bus io8;
        addr ioaddr(0x03F8);
    operation kernelEntry():
        capability hw;
        protocol UART;
        writes uart0.data;
        event physicalFault overheat;
        write uart0.data <- 0x41;
        barrier io;
        haltLoop;
    rupture:
        thermal_trip;
    anchor boot0:
        addr physaddr(0x0000_7C00);
    entry kernelEntry:
        anchor boot0;
        mode real16;
    bridge:
        root.policy.bind;
        root.policy.guard;
        root.memory.resolve;
        machine.machineCore;
    seal:
        primitiveClosed;
EOF

cat >"$USB_FIXTURE" <<'EOF'
primitive[@]:
    controller:
        usb_ctrl;
    peripheral:
        usb0;
    operation kernelEntry():
        protocol USB;
        haltLoop;
    anchor:
        boot0;
    bridge:
        root.policy.bind;
        root.policy.guard;
        root.memory.resolve;
        machine.machineCore;
    seal:
        primitiveClosed;
EOF

echo "primitive unsupported diagnostics batch gate"
echo "root: $ROOT_DIR"
echo "log_dir: $LOG_DIR"

base_target="${TARGET_TMP_PREFIX}-base"
write_target_file "$base_target"

target_power="${TARGET_TMP_PREFIX}-power"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_power}.nctarget"
set_target_kv "$target_power" "name" "\"${target_power}\""
set_target_kv "$target_power" "supports_power_model" "false"
set_target_kv "$target_power" "supports_power_gate_model" "false"
run_expect_diag "power_model_unsupported" "$ADVANCED_FIXTURE" "$target_power" "E_PRIMITIVE_POWER_001"

target_thermal="${TARGET_TMP_PREFIX}-thermal"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_thermal}.nctarget"
set_target_kv "$target_thermal" "name" "\"${target_thermal}\""
set_target_kv "$target_thermal" "supports_thermal_model" "false"
set_target_kv "$target_thermal" "supports_thermal_state_model" "false"
set_target_kv "$target_thermal" "supports_throttle_state_model" "false"
run_expect_diag "thermal_model_unsupported" "$ADVANCED_FIXTURE" "$target_thermal" "E_PRIMITIVE_THERMAL_001"

target_cache="${TARGET_TMP_PREFIX}-cache"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_cache}.nctarget"
set_target_kv "$target_cache" "name" "\"${target_cache}\""
set_target_kv "$target_cache" "supports_cache_model" "false"
set_target_kv "$target_cache" "supports_cache_state_model" "false"
run_expect_diag "cache_model_unsupported" "$ADVANCED_FIXTURE" "$target_cache" "E_PRIMITIVE_CACHE_001"

target_tlb="${TARGET_TMP_PREFIX}-tlb"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_tlb}.nctarget"
set_target_kv "$target_tlb" "name" "\"${target_tlb}\""
set_target_kv "$target_tlb" "supports_tlb_model" "false"
set_target_kv "$target_tlb" "supports_tlb_state_model" "false"
run_expect_diag "tlb_model_unsupported" "$ADVANCED_FIXTURE" "$target_tlb" "E_PRIMITIVE_TLB_001"

target_pipeline="${TARGET_TMP_PREFIX}-pipeline"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_pipeline}.nctarget"
set_target_kv "$target_pipeline" "name" "\"${target_pipeline}\""
set_target_kv "$target_pipeline" "supports_pipeline_model" "false"
set_target_kv "$target_pipeline" "supports_pipeline_state_model" "false"
set_target_kv "$target_pipeline" "supports_branch_predictor_model" "false"
set_target_kv "$target_pipeline" "supports_reorder_model" "false"
set_target_kv "$target_pipeline" "supports_out_of_order_model" "false"
run_expect_diag "pipeline_model_unsupported" "$ADVANCED_FIXTURE" "$target_pipeline" "E_PRIMITIVE_TARGET_CAPABILITY_001"

target_firmware="${TARGET_TMP_PREFIX}-firmware"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_firmware}.nctarget"
set_target_kv "$target_firmware" "name" "\"${target_firmware}\""
set_target_kv "$target_firmware" "supports_firmware_model" "false"
run_expect_diag "firmware_model_unsupported" "$ADVANCED_FIXTURE" "$target_firmware" "E_PRIMITIVE_FIRMWARE_001"

target_bootrom="${TARGET_TMP_PREFIX}-bootrom"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_bootrom}.nctarget"
set_target_kv "$target_bootrom" "name" "\"${target_bootrom}\""
set_target_kv "$target_bootrom" "supports_boot_rom_region_model" "false"
run_expect_diag "bootrom_model_unsupported" "$ADVANCED_FIXTURE" "$target_bootrom" "E_PRIMITIVE_BOOTROM_001"

target_microcode="${TARGET_TMP_PREFIX}-microcode"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_microcode}.nctarget"
set_target_kv "$target_microcode" "name" "\"${target_microcode}\""
set_target_kv "$target_microcode" "firmware_microcode" "\"opaque\""
run_expect_diag "microcode_opaque_unsupported" "$ADVANCED_FIXTURE" "$target_microcode" "E_PRIMITIVE_MICROCODE_001"

run_expect_diag "port_io_cross_target_unsupported" "$UART_FIXTURE" "aarch64-unknown-none" "E_PRIMITIVE_PORT_IO_ENV_001" "E_PRIMITIVE_PORT_UNSUPPORTED_001"

target_protocol="${TARGET_TMP_PREFIX}-protocol"
cp "$ROOT_DIR/targets/${base_target}.nctarget" "$ROOT_DIR/targets/${target_protocol}.nctarget"
set_target_kv "$target_protocol" "name" "\"${target_protocol}\""
run_expect_diag "protocol_usb_unsupported" "$USB_FIXTURE" "$target_protocol" "E_PRIMITIVE_PROTOCOL_ENV_001" "E_PRIMITIVE_USB_001"

TOTAL_CASES="$(wc -l <"$RESULTS_PSV" | tr -d ' ')"

{
  echo "# Primitive Unsupported Diagnostics Batch Report"
  echo
  echo "Generated: $(date -Is)"
  echo "Root: $ROOT_DIR"
  echo "Log dir: $LOG_DIR"
  echo "Total cases: $TOTAL_CASES"
  echo
  echo "## Cases"
  while IFS='|' read -r case_name target_name source_file expected_diag expected_diag_secondary log_file; do
    echo "- ${case_name}"
    echo "  - target: \`${target_name}\`"
    echo "  - source: \`${source_file}\`"
    echo "  - expected: \`${expected_diag}\`"
    if [[ -n "$expected_diag_secondary" ]]; then
      echo "  - expected_secondary: \`${expected_diag_secondary}\`"
    fi
    echo "  - log: \`${log_file}\`"
  done <"$RESULTS_PSV"
} >"$EVIDENCE_MD"

{
  echo "{"
  echo "  \"schema\": \"nc.primitive.unsupported_diag_batch.v1\","
  echo "  \"generated_at\": \"$(date -Is)\","
  echo "  \"root\": \"$(escape_json "$ROOT_DIR")\","
  echo "  \"log_dir\": \"$(escape_json "$LOG_DIR")\","
  echo "  \"total_cases\": ${TOTAL_CASES},"
  echo "  \"status\": \"ok\","
  echo "  \"cases\": ["
  first=1
  while IFS='|' read -r case_name target_name source_file expected_diag expected_diag_secondary log_file; do
    if [[ $first -eq 0 ]]; then
      echo ","
    fi
    first=0
    matched_primary="false"
    matched_secondary="true"
    if grep -Fq "$expected_diag" "$log_file"; then
      matched_primary="true"
    fi
    if [[ -n "$expected_diag_secondary" ]]; then
      if grep -Fq "$expected_diag_secondary" "$log_file"; then
        matched_secondary="true"
      else
        matched_secondary="false"
      fi
    fi
    echo -n "    {"
    echo -n "\"case\":\"$(escape_json "$case_name")\","
    echo -n "\"target\":\"$(escape_json "$target_name")\","
    echo -n "\"source\":\"$(escape_json "$source_file")\","
    echo -n "\"expected_diag\":\"$(escape_json "$expected_diag")\","
    echo -n "\"expected_diag_secondary\":\"$(escape_json "$expected_diag_secondary")\","
    echo -n "\"matched_primary\":${matched_primary},"
    echo -n "\"matched_secondary\":${matched_secondary},"
    echo -n "\"log\":\"$(escape_json "$log_file")\""
    echo -n "}"
  done <"$RESULTS_PSV"
  echo
  echo "  ]"
  echo "}"
} >"$EVIDENCE_JSON"

echo "Evidence JSON: $EVIDENCE_JSON"
echo "Evidence Markdown: $EVIDENCE_MD"
echo "OK: primitive unsupported diagnostics batch gate passed"
