#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: build_phase10_ncup_distribution_v2_0_0.sh [options]

Options:
  --out-dir <dir>           Output root directory (default: var/releases/v2_0_0/phase10_p1_dist)
  --targets <csv>           Target triples CSV
  --host-only               Build/package only the rustc host target
  --allow-partial           Keep packaging successful targets even if some fail
  --sign-key-env <ENV>      Signing key env name for integrity bundle (default: NCC_RELEASE_SIGNING_KEY)
  -h|--help|help            Show this help
USAGE
}

OUT_DIR="var/releases/v2_0_0/phase10_p1_dist"
TARGETS_CSV="x86_64-unknown-linux-gnu,x86_64-pc-windows-msvc,aarch64-apple-darwin,wasm32-wasi"
HOST_ONLY=0
ALLOW_PARTIAL=0
SIGN_KEY_ENV="NCC_RELEASE_SIGNING_KEY"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --out-dir" >&2; exit 2; }
      OUT_DIR="$1"
      ;;
    --targets)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --targets" >&2; exit 2; }
      TARGETS_CSV="$1"
      ;;
    --host-only)
      HOST_ONLY=1
      ;;
    --allow-partial)
      ALLOW_PARTIAL=1
      ;;
    --sign-key-env)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --sign-key-env" >&2; exit 2; }
      SIGN_KEY_ENV="$1"
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
  shift || true
done

if [[ $HOST_ONLY -eq 1 ]]; then
  resolve_rustc() {
    local raw_home home_dir candidate
    for raw_home in "${USERPROFILE:-}" "${HOME:-}"; do
      [[ -n "$raw_home" ]] || continue
      home_dir="$raw_home"
      if command -v cygpath >/dev/null 2>&1; then
        home_dir="$(cygpath -u "$raw_home" 2>/dev/null || printf '%s' "$raw_home")"
      fi
      [[ -n "$home_dir" ]] || continue
      local candidate
      for candidate in "$home_dir"/.rustup/toolchains/*/bin/rustc "$home_dir"/.rustup/toolchains/*/bin/rustc.exe "$home_dir"/.cargo/bin/rustc "$home_dir"/.cargo/bin/rustc.exe; do
        if [[ -x "$candidate" ]]; then
          printf '%s\n' "$candidate"
          return 0
        fi
      done
    done
    if command -v rustup >/dev/null 2>&1; then
      rustup which rustc 2>/dev/null && return 0
    fi
    if command -v rustc >/dev/null 2>&1; then
      command -v rustc
      return 0
    fi
    return 1
  }
  RUSTC_BIN="$(resolve_rustc)" || { echo "rustc not found; install Rust or expose rustc in PATH" >&2; exit 127; }
  HOST_TARGET="$("$RUSTC_BIN" -vV | awk '/^host:/ {print $2}')"
  TARGETS_CSV="$HOST_TARGET"
fi

IFS=',' read -r -a TARGETS <<< "$TARGETS_CSV"
if [[ ${#TARGETS[@]} -eq 0 ]]; then
  echo "no targets resolved" >&2
  exit 2
fi

PACKAGE_DIR="$OUT_DIR/packages"
INTEGRITY_DIR="$OUT_DIR/integrity"
STAGING_DIR="$OUT_DIR/staging"
MANIFESTS_DIR="$OUT_DIR/manifests"
REPORT_PATH="$OUT_DIR/distribution_report.json"
SUMMARY_PATH="$OUT_DIR/.summary.tsv"
LOGS_DIR="$OUT_DIR/logs"

rm -rf "$PACKAGE_DIR" "$INTEGRITY_DIR" "$STAGING_DIR" "$MANIFESTS_DIR" "$SUMMARY_PATH"
mkdir -p "$OUT_DIR" "$PACKAGE_DIR" "$INTEGRITY_DIR" "$STAGING_DIR" "$MANIFESTS_DIR" "$LOGS_DIR"
mkdir -p "$(dirname "$SUMMARY_PATH")"

BINS=(ncc nfmt nlint nbuild npkg nide nclsp ncup ncup-init)
FAILED=0

printf "target\tstatus\tartifact\tsha256\tmessage\n" > "$SUMMARY_PATH"

for target in "${TARGETS[@]}"; do
  target="$(echo "$target" | xargs)"
  [[ -n "$target" ]] || continue
  echo "[F10-P1] build target=$target"
  target_log="$LOGS_DIR/${target}.build.log"
  build_ok=1
  {
    cargo build -p ncc-cli --release --target "$target" \
      --bin ncc --bin nfmt --bin nlint --bin nbuild --bin npkg --bin nide --bin nclsp --bin ncup --bin ncup-init
  } >"$target_log" 2>&1 || build_ok=0

  if [[ $build_ok -eq 0 ]]; then
    FAILED=1
    printf "%s\tfailed\t\t\tbuild failed (see %s)\n" "$target" "$target_log" >> "$SUMMARY_PATH"
    echo "[F10-P1] build log tail for $target:" >&2
    tail -n 120 "$target_log" >&2 || true
    if [[ $ALLOW_PARTIAL -eq 0 ]]; then
      echo "[F10-P1] build failed for $target (strict mode)" >&2
      break
    fi
    continue
  fi

  target_stage="$STAGING_DIR/$target"
  mkdir -p "$target_stage/bin"
  for bin in "${BINS[@]}"; do
    src="target/$target/release/$bin"
    if [[ "$target" == *windows* ]]; then
      src="${src}.exe"
    fi
    if [[ ! -f "$src" ]]; then
      FAILED=1
      printf "%s\tfailed\t\t\tmissing binary %s\n" "$target" "$src" >> "$SUMMARY_PATH"
      build_ok=0
      break
    fi
    cp "$src" "$target_stage/bin/"
  done
  if [[ $build_ok -eq 0 ]]; then
    if [[ $ALLOW_PARTIAL -eq 0 ]]; then
      break
    fi
    continue
  fi

  cat > "$target_stage/manifest.v1.json" <<JSON
{
  "schema": "nc.phase10.distribution.bundle.v1",
  "version": "2.0.0",
  "target": "$target",
  "bins": ["ncc","nfmt","nlint","nbuild","npkg","nide","nclsp","ncup","ncup-init"],
  "state": "frozen"
}
JSON

  artifact=""
  if [[ "$target" == *windows* ]]; then
    artifact="$PACKAGE_DIR/nc-v2.0.0-$target.zip"
    if command -v zip >/dev/null 2>&1; then
      (
        cd "$target_stage"
        zip -qr "$ROOT_DIR/$artifact" .
      )
    else
      python3 - <<'PY' "$target_stage" "$artifact"
import pathlib, sys, zipfile
stage = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
out.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
    for path in sorted(stage.rglob("*")):
        if path.is_file():
            zf.write(path, path.relative_to(stage).as_posix())
PY
    fi
  else
    artifact="$PACKAGE_DIR/nc-v2.0.0-$target.tar.gz"
    tar -C "$target_stage" -czf "$artifact" .
  fi

  sha="$(sha256sum "$artifact" | awk '{print $1}')"
  printf "%s\tsuccess\t%s\t%s\tok\n" "$target" "$artifact" "$sha" >> "$SUMMARY_PATH"
done

if [[ $FAILED -eq 1 && $ALLOW_PARTIAL -eq 0 ]]; then
  python3 - <<'PY' "$SUMMARY_PATH" "$REPORT_PATH" "$TARGETS_CSV"
import csv, json, pathlib, sys
summary_path, report_path, targets_csv = sys.argv[1:4]
rows = []
with open(summary_path, "r", encoding="utf-8") as f:
    reader = csv.DictReader(f, delimiter="\t")
    for row in reader:
        rows.append(row)
report = {
    "schema": "nc.phase10.distribution.report.v1",
    "version": "2.0.0",
    "status": "error",
    "targets_requested": [x.strip() for x in targets_csv.split(",") if x.strip()],
    "targets": rows,
}
pathlib.Path(report_path).parent.mkdir(parents=True, exist_ok=True)
pathlib.Path(report_path).write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
print(f"ERROR: distribution report written: {report_path}")
PY
  exit 1
fi

"${BASH:-bash}" tools/release/sign_artifacts_v1_0_1.sh "$PACKAGE_DIR" "$INTEGRITY_DIR" --key-env "$SIGN_KEY_ENV"

python3 - <<'PY' "$SUMMARY_PATH" "$REPORT_PATH" "$TARGETS_CSV" "$OUT_DIR" "$INTEGRITY_DIR" "$MANIFESTS_DIR"
import csv, datetime, json, pathlib, sys, os
summary_path, report_path, targets_csv, out_dir, integrity_dir, manifests_dir = sys.argv[1:7]
rows = []
with open(summary_path, "r", encoding="utf-8") as f:
    reader = csv.DictReader(f, delimiter="\t")
    for row in reader:
        rows.append(row)
status = "ok"
if any(row.get("status") != "success" for row in rows):
    status = "partial"
report = {
    "schema": "nc.phase10.distribution.report.v1",
    "version": "2.0.0",
    "generated_at": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "status": status,
    "targets_requested": [x.strip() for x in targets_csv.split(",") if x.strip()],
    "targets": rows,
    "output_dir": out_dir,
    "integrity_bundle": {
        "checksums": str(pathlib.Path(integrity_dir) / "SHA256SUMS"),
        "signature": str(pathlib.Path(integrity_dir) / "SHA256SUMS.sig"),
        "sbom": str(pathlib.Path(integrity_dir) / "SBOM.ncc.v1.json"),
        "attestation": str(pathlib.Path(integrity_dir) / "attestation.v1.json"),
    },
}
pathlib.Path(report_path).parent.mkdir(parents=True, exist_ok=True)
pathlib.Path(report_path).write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")

success_rows = [row for row in rows if row.get("status") == "success"]
if not success_rows:
    raise SystemExit("no successful artifacts found in distribution summary")

def toml_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')

channels = ["stable", "beta", "nightly"]
components = ["compiler", "runtime", "stdlib", "fmt", "lint", "lsp", "docs", "sources"]
min_ncup_version = "2.0.0"
breaking_notes = []
base_url = os.environ.get(
    "NC_RELEASE_BASE_URL",
    "https://releases.nc-lang.org/v2.0.0/phase10_p1_dist/packages",
)
manifests_root = pathlib.Path(manifests_dir)
manifests_root.mkdir(parents=True, exist_ok=True)

for channel in channels:
    channel_dir = manifests_root / "channels" / channel
    channel_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = channel_dir / "channel-manifest.toml"
    lines = [
        'schema = "nc.release.channel.manifest.v1"',
        'version = "2.0.0"',
        'state = "frozen"',
        f'channel = "{channel}"',
        f'min_ncup_version = "{min_ncup_version}"',
        "breaking_notes = []",
        "",
        "[compatibility]",
        f'min_ncup_version = "{min_ncup_version}"',
        "breaking_notes = []",
        "",
        "[components]",
        "items = [" + ", ".join(f'\"{c}\"' for c in components) + "]",
        "",
    ]
    for row in success_rows:
        target = row.get("target", "").strip()
        artifact = pathlib.Path(row.get("artifact", "").strip()).name
        sha = row.get("sha256", "").strip()
        url = f"{base_url}/{artifact}"
        toolchain_id = f"{channel}-{target}"
        lines.extend(
            [
                "[[entries]]",
                f'toolchain_id = "{toml_escape(toolchain_id)}"',
                f'target = "{toml_escape(target)}"',
                f'artifact = "{toml_escape(artifact)}"',
                f'url = "{toml_escape(url)}"',
                f'sha256 = "{toml_escape(sha)}"',
                "components = [" + ", ".join(f'\"{c}\"' for c in components) + "]",
                "",
            ]
        )
    manifest_path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")

release_manifest = {
    "schema": "nc.release.manifest.v1",
    "version": "2.0.0",
    "state": "frozen",
    "generated_at": datetime.datetime.now(datetime.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z"),
    "min_ncup_version": min_ncup_version,
    "breaking_notes": breaking_notes,
    "channels": {
        channel: {
            "manifest": f"channels/{channel}/channel-manifest.toml",
            "entry_count": len(success_rows),
        }
        for channel in channels
    },
    "components": components,
    "packages": [
        {
            "target": row.get("target", "").strip(),
            "artifact": pathlib.Path(row.get("artifact", "").strip()).name,
            "sha256": row.get("sha256", "").strip(),
            "url": f"{base_url}/{pathlib.Path(row.get('artifact', '').strip()).name}",
        }
        for row in success_rows
    ],
    "integrity_bundle": {
        "checksums": "../integrity/SHA256SUMS",
        "signature": "../integrity/SHA256SUMS.sig",
        "sbom": "../integrity/SBOM.ncc.v1.json",
        "attestation": "../integrity/attestation.v1.json",
    },
}
(manifests_root / "release-manifest.json").write_text(
    json.dumps(release_manifest, indent=2, ensure_ascii=True) + "\n",
    encoding="utf-8",
)
print(f"OK: distribution report {report_path}")
PY

cp "$REPORT_PATH" docs/status/F10_P1_distribution_report.json
echo "[F10-P1] distribution bundles ready in $OUT_DIR"
