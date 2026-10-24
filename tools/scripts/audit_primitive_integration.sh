#!/usr/bin/env bash
set -u

OUT="reports/primitive/primitive_integration_audit_$(date +%Y%m%d_%H%M%S).md"

{
echo "# primitive[@] Integration Audit"
echo
echo "Workspace: $(pwd)"
echo "Fecha: $(date -Is)"
echo

echo "## 1. Workspace"
pwd
echo
ls -lah Cargo.toml crates/ncc-cli/src/main.rs 2>&1
echo

echo "## 2. Cargo metadata/check"
cargo metadata --no-deps --format-version 1 >/tmp/nc_metadata.json 2>/tmp/nc_metadata.err
echo "cargo metadata exit=$?"
cat /tmp/nc_metadata.err
echo
cargo check --workspace --all-targets 2>&1
echo

echo "## 3. primitive keywords / implementación"
rg -n "primitive\[@\]|Primitive|anchor|slot|pattern|mask|window|signal|bus|state|timing|emit|sample|latch|assert|rupture|bridge|seal|material witness|witness|mmio|port|volatile|fence|qemu|x86_64-unknown-none|hardware_identity_uart|E_PRIMITIVE" crates examples tests docs targets tools 2>/dev/null || true
echo

echo "## 4. CLI primitive help"
cargo run -p ncc-cli -- primitive doctor 2>&1 || true
echo
cargo run -p ncc-cli -- primitive target x86_64-unknown-none 2>&1 || true
echo

echo "## 5. Ejemplos primitive"
find examples -maxdepth 4 -type f \( -name "*.ncp" -o -name "*primitive*" \) -print 2>/dev/null
echo

echo "## 6. Target files"
find targets -maxdepth 3 -type f -print 2>/dev/null
echo
for f in targets/*.nctarget targets/**/*.nctarget; do
  [ -f "$f" ] || continue
  echo "### $f"
  sed -n '1,220p' "$f"
  echo
done

echo "## 7. hardware_identity_uart.ncp check/witness/map/lower/report"
if [ -f examples/primitive/hardware_identity_uart.ncp ]; then
  cargo run -p ncc-cli -- primitive check examples/primitive/hardware_identity_uart.ncp 2>&1 || true
  echo
  cargo run -p ncc-cli -- primitive witness examples/primitive/hardware_identity_uart.ncp --json 2>&1 || true
  echo
  cargo run -p ncc-cli -- primitive map examples/primitive/hardware_identity_uart.ncp 2>&1 || true
  echo
  cargo run -p ncc-cli -- primitive lower examples/primitive/hardware_identity_uart.ncp 2>&1 || true
  echo
  cargo run -p ncc-cli -- primitive report examples/primitive/hardware_identity_uart.ncp --md-out reports/primitive/hardware_identity_uart_report.md 2>&1 || true
else
  echo "MISSING: examples/primitive/hardware_identity_uart.ncp"
fi
echo

echo "## 8. Tests primitive"
cargo test --workspace primitive -- --nocapture 2>&1 || true
echo

echo "## 9. Deuda técnica"
rg -n "TODO|FIXME|HACK|unimplemented!|todo!|panic!|stub|placeholder" crates examples tests docs tools 2>/dev/null || true
echo

echo "## 10. Git status"
git status --short 2>&1 || true
} | tee "$OUT"

echo
echo "Reporte generado: $OUT"
