#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: setup_ncup_ci_v2_0_0.sh [options]

Options:
  --provider <github|gitlab|generic>    CI provider label (default: generic)
  --home <path>                         NCUP_HOME destination for CI install
  --channel <stable|beta|nightly>       Toolchain channel (default: stable)
  --target <triple>                     Toolchain target triple
  --components <csv>                    Components CSV (default: compiler,runtime,stdlib,fmt,lint,lsp)
  --profile <minimal|default|complete>  Install profile (default: default)
  --manifest-path <path>                Local manifest used for cache hash
  --manifest-url <url>                  Optional manifest URL (online mode)
  --cache-root <path>                   Cache root path
  --offline                             Enforce offline mode
  --offline-bundle <path>               Predownloaded toolchain bundle (.tar.gz|.tgz|.zip|dir)
  --no-cache                            Disable cache read/write
  --force-reinit                        Force ncup init --force for the selected home
  --json                                Emit machine-readable JSON summary
  -h|--help|help                        Show this help
USAGE
}

PROVIDER="generic"
NCUP_HOME=""
CHANNEL="stable"
TARGET=""
COMPONENTS_CSV="compiler,runtime,stdlib,fmt,lint,lsp"
PROFILE="default"
MANIFEST_PATH=""
MANIFEST_URL=""
CACHE_ROOT=""
OFFLINE=0
OFFLINE_BUNDLE=""
NO_CACHE=0
FORCE_REINIT=0
JSON_MODE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --provider)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --provider" >&2; exit 2; }
      PROVIDER="$1"
      ;;
    --home)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --home" >&2; exit 2; }
      NCUP_HOME="$1"
      ;;
    --channel)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --channel" >&2; exit 2; }
      CHANNEL="$1"
      ;;
    --target)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --target" >&2; exit 2; }
      TARGET="$1"
      ;;
    --components)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --components" >&2; exit 2; }
      COMPONENTS_CSV="$1"
      ;;
    --profile)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --profile" >&2; exit 2; }
      PROFILE="$1"
      ;;
    --manifest-path)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --manifest-path" >&2; exit 2; }
      MANIFEST_PATH="$1"
      ;;
    --manifest-url)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --manifest-url" >&2; exit 2; }
      MANIFEST_URL="$1"
      ;;
    --cache-root)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --cache-root" >&2; exit 2; }
      CACHE_ROOT="$1"
      ;;
    --offline)
      OFFLINE=1
      ;;
    --offline-bundle)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --offline-bundle" >&2; exit 2; }
      OFFLINE_BUNDLE="$1"
      ;;
    --no-cache)
      NO_CACHE=1
      ;;
    --force-reinit)
      FORCE_REINIT=1
      ;;
    --json)
      JSON_MODE=1
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

if [[ "$PROVIDER" != "github" && "$PROVIDER" != "gitlab" && "$PROVIDER" != "generic" ]]; then
  echo "invalid --provider '$PROVIDER' (expected github|gitlab|generic)" >&2
  exit 2
fi

if [[ "$CHANNEL" != "stable" && "$CHANNEL" != "beta" && "$CHANNEL" != "nightly" ]]; then
  echo "invalid --channel '$CHANNEL' (expected stable|beta|nightly)" >&2
  exit 2
fi

if [[ -z "$CACHE_ROOT" ]]; then
  case "$PROVIDER" in
    github)
      CACHE_ROOT="${RUNNER_TOOL_CACHE:-${HOME}/.cache}/ncup"
      ;;
    gitlab)
      CACHE_ROOT="${CI_PROJECT_DIR:-${ROOT_DIR}}/.cache/ncup"
      ;;
    *)
      CACHE_ROOT="${XDG_CACHE_HOME:-${HOME}/.cache}/ncup"
      ;;
  esac
fi

if [[ -z "$NCUP_HOME" ]]; then
  case "$PROVIDER" in
    github)
      NCUP_HOME="${RUNNER_TEMP:-${ROOT_DIR}/.tmp}/ncup-home"
      ;;
    gitlab)
      NCUP_HOME="${CI_PROJECT_DIR:-${ROOT_DIR}}/.ncup-ci-home"
      ;;
    *)
      NCUP_HOME="${ROOT_DIR}/.ncup-ci-home"
      ;;
  esac
fi

mkdir -p "$CACHE_ROOT" "$NCUP_HOME"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command '$1'" >&2
    exit 127
  }
}

require_cmd python3
require_cmd cargo
require_cmd tar

NCC_BIN="${ROOT_DIR}/target/debug/ncc"
if [[ ! -x "$NCC_BIN" && -x "${NCC_BIN}.exe" ]]; then
  NCC_BIN="${NCC_BIN}.exe"
fi
if [[ ! -x "$NCC_BIN" ]]; then
  cargo build -p ncc-cli --bin ncc >/dev/null
fi
if [[ ! -x "$NCC_BIN" && -x "${NCC_BIN}.exe" ]]; then
  NCC_BIN="${NCC_BIN}.exe"
fi
[[ -x "$NCC_BIN" ]] || {
  echo "ncc binary not available at '$NCC_BIN'" >&2
  exit 1
}

MODEL_JSON="$($NCC_BIN phase10-toolchain-model --json)"

if [[ -z "$TARGET" ]]; then
  TARGET="$(python3 - <<'PY' "$MODEL_JSON"
import json,sys
m=json.loads(sys.argv[1])
rows=m.get('toolchains') or []
for row in rows:
    ident=str(row.get('id',''))
    if ident.startswith('stable-') and '-' in ident:
        print(ident.split('-',1)[1])
        sys.exit(0)
if rows:
    ident=str(rows[0].get('id',''))
    if '-' in ident:
        print(ident.split('-',1)[1])
        sys.exit(0)
raise SystemExit('no target could be resolved from phase10-toolchain-model')
PY
)"
fi

TOOLCHAIN_ID="${CHANNEL}-${TARGET}"
python3 - <<'PY' "$MODEL_JSON" "$TOOLCHAIN_ID"
import json,sys
m=json.loads(sys.argv[1])
toolchain_id=sys.argv[2]
ids={str(row.get('id','')) for row in (m.get('toolchains') or [])}
if toolchain_id not in ids:
    avail=', '.join(sorted(v for v in ids if v))
    raise SystemExit(f"ci setup: unknown/unsupported toolchain id '{toolchain_id}'. available: [{avail}]")
PY

if [[ -n "$MANIFEST_URL" && $OFFLINE -eq 1 ]]; then
  echo "offline mode cannot use --manifest-url; provide --manifest-path or local default" >&2
  exit 2
fi

resolved_manifest=""
tmp_manifest=""
if [[ -n "$MANIFEST_URL" ]]; then
  tmp_manifest="$(mktemp -t ncup_manifest_XXXXXX.json)"
  python3 - <<'PY' "$MANIFEST_URL" "$tmp_manifest"
import sys,urllib.request
url, out = sys.argv[1:3]
with urllib.request.urlopen(url, timeout=30) as resp:
    data = resp.read()
with open(out, 'wb') as f:
    f.write(data)
PY
  resolved_manifest="$tmp_manifest"
elif [[ -n "$MANIFEST_PATH" ]]; then
  resolved_manifest="$MANIFEST_PATH"
else
  if [[ -f "var/releases/v2_0_0/phase10_p1_dist/manifests/release-manifest.json" ]]; then
    resolved_manifest="var/releases/v2_0_0/phase10_p1_dist/manifests/release-manifest.json"
  else
    resolved_manifest="docs/spec/release/v2_0_0_phase10_manifest_format_contract.json"
  fi
fi

if [[ ! -f "$resolved_manifest" ]]; then
  echo "manifest path '$resolved_manifest' does not exist" >&2
  exit 2
fi

MANIFEST_HASH="$(python3 - <<'PY' "$resolved_manifest"
import hashlib,sys
p=sys.argv[1]
h=hashlib.sha256()
with open(p,'rb') as f:
    for chunk in iter(lambda: f.read(1024*1024), b''):
        h.update(chunk)
print(h.hexdigest())
PY
)"

CACHE_KEY="ncup-${TOOLCHAIN_ID}-${PROFILE}-${MANIFEST_HASH}"
CACHE_DIR="${CACHE_ROOT}/${CACHE_KEY}"
CACHE_ARCHIVE="${CACHE_DIR}/toolchain.tgz"
CACHE_META="${CACHE_DIR}/metadata.json"

IFS=',' read -r -a COMPONENTS <<< "$COMPONENTS_CSV"
COMPONENT_ROWS=()
for raw in "${COMPONENTS[@]}"; do
  value="$(echo "$raw" | xargs)"
  [[ -n "$value" ]] || continue
  COMPONENT_ROWS+=("$value")
done
if [[ ${#COMPONENT_ROWS[@]} -eq 0 ]]; then
  echo "components list cannot be empty" >&2
  exit 2
fi

if [[ $FORCE_REINIT -eq 1 || ! -f "$NCUP_HOME/manifests/toolchains.v1.json" ]]; then
  init_args=(
    ncup init
    --home "$NCUP_HOME"
    --default-toolchain "$TOOLCHAIN_ID"
    --profile "$PROFILE"
    --yes
    --skip-first-run
  )
  if [[ $FORCE_REINIT -eq 1 ]]; then
    init_args+=(--force)
  fi
  "$NCC_BIN" "${init_args[@]}" >/dev/null
fi

"$NCC_BIN" ncup toolchain install "$TOOLCHAIN_ID" --home "$NCUP_HOME" --profile "$PROFILE" --set-default >/dev/null

copy_tree_content() {
  local src="$1"
  local dst="$2"
  mkdir -p "$dst"
  cp -a "$src/." "$dst/"
}

resolve_extracted_source_dir() {
  local root="$1"
  if [[ -d "$root/bin" ]]; then
    echo "$root"
    return 0
  fi
  local candidate
  candidate="$(find "$root" -mindepth 1 -maxdepth 1 -type d | head -n 1 || true)"
  if [[ -n "$candidate" && -d "$candidate/bin" ]]; then
    echo "$candidate"
    return 0
  fi
  return 1
}

extract_offline_bundle_into_toolchain() {
  local bundle="$1"
  local toolchain_dir="$2"

  rm -rf "$toolchain_dir"
  mkdir -p "$toolchain_dir"

  if [[ -d "$bundle" ]]; then
    local src_dir
    src_dir="$(resolve_extracted_source_dir "$bundle")" || {
      echo "offline bundle directory '$bundle' does not contain a toolchain bundle root with bin/" >&2
      return 1
    }
    copy_tree_content "$src_dir" "$toolchain_dir"
    return 0
  fi

  if [[ ! -f "$bundle" ]]; then
    echo "offline bundle '$bundle' not found" >&2
    return 1
  fi

  local tmpdir
  tmpdir="$(mktemp -d -t ncup_offline_bundle_XXXXXX)"
  if [[ "$bundle" == *.zip ]]; then
    require_cmd unzip
    unzip -q "$bundle" -d "$tmpdir"
  else
    tar -xzf "$bundle" -C "$tmpdir"
  fi

  local src_dir
  src_dir="$(resolve_extracted_source_dir "$tmpdir")" || {
    rm -rf "$tmpdir"
    echo "offline bundle '$bundle' extracted but no bundle root with bin/ found" >&2
    return 1
  }
  copy_tree_content "$src_dir" "$toolchain_dir"
  rm -rf "$tmpdir"
  return 0
}

CACHE_HIT=0
OFFLINE_BUNDLE_USED=0
TOOLCHAIN_DIR="$NCUP_HOME/toolchains/$TOOLCHAIN_ID"

if [[ $NO_CACHE -eq 0 && -f "$CACHE_ARCHIVE" ]]; then
  mkdir -p "$NCUP_HOME/toolchains"
  rm -rf "$TOOLCHAIN_DIR"
  tar -xzf "$CACHE_ARCHIVE" -C "$NCUP_HOME/toolchains"
  CACHE_HIT=1
else
  if [[ $OFFLINE -eq 1 ]]; then
    if [[ -z "$OFFLINE_BUNDLE" ]]; then
      echo "offline mode requires --offline-bundle" >&2
      exit 2
    fi
    extract_offline_bundle_into_toolchain "$OFFLINE_BUNDLE" "$TOOLCHAIN_DIR"
    OFFLINE_BUNDLE_USED=1
  fi
fi

"$NCC_BIN" ncup target add "$TARGET" --toolchain "$TOOLCHAIN_ID" --home "$NCUP_HOME" >/dev/null
for component in "${COMPONENT_ROWS[@]}"; do
  "$NCC_BIN" ncup component add "$component" --toolchain "$TOOLCHAIN_ID" --home "$NCUP_HOME" >/dev/null
done
"$NCC_BIN" ncup toolchain default "$TOOLCHAIN_ID" --home "$NCUP_HOME" >/dev/null

DOCTOR_OUTPUT="$($NCC_BIN ncup doctor --home "$NCUP_HOME" 2>&1)"

if [[ $NO_CACHE -eq 0 && $CACHE_HIT -eq 0 ]]; then
  mkdir -p "$CACHE_DIR"
  tar -C "$NCUP_HOME/toolchains" -czf "$CACHE_ARCHIVE" "$TOOLCHAIN_ID"
  python3 - <<'PY' "$CACHE_META" "$TOOLCHAIN_ID" "$PROFILE" "$CHANNEL" "$TARGET" "$MANIFEST_HASH"
import datetime,json,pathlib,sys
path,toolchain_id,profile,channel,target,manifest_hash = sys.argv[1:7]
payload = {
  "schema": "ncup.ci.cache.metadata.v1",
  "version": "2.0.0",
  "toolchain_id": toolchain_id,
  "profile": profile,
  "channel": channel,
  "target": target,
  "manifest_hash": manifest_hash,
  "generated_at": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00","Z")
}
pathlib.Path(path).write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
PY
fi

SUMMARY_JSON="$(python3 - <<'PY' "$PROVIDER" "$NCUP_HOME" "$CHANNEL" "$TARGET" "$TOOLCHAIN_ID" "$PROFILE" "$MANIFEST_HASH" "$resolved_manifest" "$CACHE_ROOT" "$CACHE_KEY" "$CACHE_ARCHIVE" "$CACHE_HIT" "$OFFLINE" "$OFFLINE_BUNDLE_USED" "$NO_CACHE" "$DOCTOR_OUTPUT" "$COMPONENTS_CSV"
import json,sys
(provider, home, channel, target, toolchain_id, profile, manifest_hash, manifest_path, cache_root, cache_key, cache_archive, cache_hit, offline_mode, offline_bundle_used, no_cache, doctor_output, components_csv) = sys.argv[1:18]
components = [c.strip() for c in components_csv.split(',') if c.strip()]
payload = {
  "schema": "nc.ci.setup.report.v1",
  "version": "2.0.0",
  "status": "ok",
  "provider": provider,
  "home": home,
  "channel": channel,
  "target": target,
  "toolchain_id": toolchain_id,
  "profile": profile,
  "components": components,
  "manifest": {
    "path": manifest_path,
    "sha256": manifest_hash,
  },
  "cache": {
    "enabled": no_cache == "0",
    "root": cache_root,
    "key": cache_key,
    "archive": cache_archive,
    "hit": cache_hit == "1",
  },
  "offline": {
    "enabled": offline_mode == "1",
    "bundle_used": offline_bundle_used == "1",
  },
  "doctor": doctor_output.strip(),
}
print(json.dumps(payload, ensure_ascii=True))
PY
)"

if [[ $JSON_MODE -eq 1 ]]; then
  echo "$SUMMARY_JSON"
else
  echo "OK: ncup ci setup provider=$PROVIDER toolchain=$TOOLCHAIN_ID profile=$PROFILE"
  echo "manifest sha256: $MANIFEST_HASH"
  echo "cache key: $CACHE_KEY (hit=$CACHE_HIT)"
  if [[ $OFFLINE -eq 1 ]]; then
    echo "offline bundle used: $OFFLINE_BUNDLE_USED"
  fi
fi

if [[ -n "$tmp_manifest" && -f "$tmp_manifest" ]]; then
  rm -f "$tmp_manifest"
fi
