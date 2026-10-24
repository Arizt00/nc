#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: build_phase10_distribution_os_v2_0_0.sh [options]

Options:
  --out-dir <dir>         Output root (default: var/releases/v2_0_0/phase10_p3_dist)
  --os <list>             OS set: all|windows|linux|macos|csv (default: all)
  --host-only             Build only current host OS target
  --plan-only             Do not compile binaries; generate layout + package plans
  --allow-deferred        Allow deferred package artifacts when native packagers are unavailable
  --strict                Fail on deferred artifacts
  --sign-key-env <ENV>    Signing key env for integrity bundles (default: NCC_RELEASE_SIGNING_KEY)
  --layout-report <path>  Layout verifier report path (default: docs/status/F10_P3_distribution_os_report.json)
  -h|--help|help          Show this help
USAGE
}

OUT_DIR="var/releases/v2_0_0/phase10_p3_dist"
OS_ARG="all"
HOST_ONLY=0
PLAN_ONLY=0
ALLOW_DEFERRED=1
SIGN_KEY_ENV="NCC_RELEASE_SIGNING_KEY"
LAYOUT_REPORT_PATH="docs/status/F10_P3_distribution_os_report.json"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --out-dir" >&2; exit 2; }
      OUT_DIR="$1"
      ;;
    --os)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --os" >&2; exit 2; }
      OS_ARG="$1"
      ;;
    --host-only)
      HOST_ONLY=1
      ;;
    --plan-only)
      PLAN_ONLY=1
      ;;
    --allow-deferred)
      ALLOW_DEFERRED=1
      ;;
    --strict)
      ALLOW_DEFERRED=0
      ;;
    --sign-key-env)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --sign-key-env" >&2; exit 2; }
      SIGN_KEY_ENV="$1"
      ;;
    --layout-report)
      shift
      [[ $# -gt 0 ]] || { echo "missing value for --layout-report" >&2; exit 2; }
      LAYOUT_REPORT_PATH="$1"
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

BINS=(ncc nfmt nlint nbuild npkg nide nclsp ncup ncup-init)

host_uname="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$host_uname" in
  linux*) HOST_OS="linux" ;;
  darwin*) HOST_OS="macos" ;;
  msys*|mingw*|cygwin*) HOST_OS="windows" ;;
  *) HOST_OS="linux" ;;
esac

resolve_rustc() {
  local raw_home home_dir candidate
  for raw_home in "${USERPROFILE:-}" "${HOME:-}"; do
    [[ -n "$raw_home" ]] || continue
    home_dir="$raw_home"
    if command -v cygpath >/dev/null 2>&1; then
      home_dir="$(cygpath -u "$raw_home" 2>/dev/null || printf '%s' "$raw_home")"
    fi
    [[ -n "$home_dir" ]] || continue
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
host_target="$("$RUSTC_BIN" -vV | awk '/^host:/ {print $2}')"

default_target_for_os() {
  case "$1" in
    windows) echo "x86_64-pc-windows-msvc" ;;
    linux) echo "x86_64-unknown-linux-gnu" ;;
    macos) echo "aarch64-apple-darwin" ;;
    *) echo "x86_64-unknown-linux-gnu" ;;
  esac
}

if [[ $HOST_ONLY -eq 1 ]]; then
  OS_LIST=("$HOST_OS")
else
  if [[ "$OS_ARG" == "all" ]]; then
    OS_LIST=(windows linux macos)
  else
    IFS=',' read -r -a OS_LIST <<< "$OS_ARG"
  fi
fi

PACKAGES_DIR="$OUT_DIR/packages"
STAGING_DIR="$OUT_DIR/staging"
INTEGRITY_DIR="$OUT_DIR/integrity"
LOG_DIR="$OUT_DIR/logs"
SUMMARY_TSV="$OUT_DIR/distribution_os_summary.tsv"
REPORT_PATH="$OUT_DIR/distribution_os_report.json"

rm -rf "$PACKAGES_DIR" "$STAGING_DIR" "$INTEGRITY_DIR" "$LOG_DIR"
mkdir -p "$PACKAGES_DIR" "$STAGING_DIR" "$INTEGRITY_DIR" "$LOG_DIR"
mkdir -p "$PACKAGES_DIR/windows" "$PACKAGES_DIR/linux" "$PACKAGES_DIR/macos"

printf "os\ttarget\tstage\tpackage_status\tnotes\n" > "$SUMMARY_TSV"
SELECTED_OS=()

dir_has_files() {
  local dir="$1"
  if [[ ! -d "$dir" ]]; then
    return 1
  fi
  find "$dir" -type f -print -quit | grep -q .
}

write_layout_files() {
  local os="$1"
  local target="$2"
  local toolchain_dir="$3"
  local stage_mode="$4"

  local logical_bins_json="{}"
  for bin in "${BINS[@]}"; do
    local filename="$bin"
    if [[ "$os" == "windows" ]]; then
      filename="${bin}.exe"
    fi
    logical_bins_json="$(python3 - <<'PY' "$logical_bins_json" "$bin" "$filename"
import json,sys
obj=json.loads(sys.argv[1])
obj[sys.argv[2]]='bin/'+sys.argv[3]
print(json.dumps(obj,separators=(',',':')))
PY
)"
  done

  cat > "$toolchain_dir/manifest.v1.json" <<JSON
{
  "schema": "nc.phase10.distribution.bundle.v2",
  "version": "2.0.0",
  "state": "frozen",
  "os": "$os",
  "target": "$target",
  "stage_mode": "$stage_mode",
  "logical_layout_id": "nc.toolchain.layout.v1",
  "bins": ["ncc","nfmt","nlint","nbuild","npkg","nide","nclsp","ncup","ncup-init"]
}
JSON

  python3 - <<'PY' "$toolchain_dir/layout.v1.json" "$os" "$target" "$logical_bins_json"
import json,sys
out,os_name,target,obj=sys.argv[1:5]
layout={
  "schema":"nc.phase10.toolchain.layout.v1",
  "version":"2.0.0",
  "state":"frozen",
  "logical_layout_id":"nc.toolchain.layout.v1",
  "os":os_name,
  "target":target,
  "logical_bins":json.loads(obj),
  "required_metadata":["manifest.v1.json","layout.v1.json"]
}
with open(out,'w',encoding='utf-8') as f:
  json.dump(layout,f,indent=2,ensure_ascii=True)
  f.write("\n")
PY
}

stage_toolchain() {
  local os="$1"
  local target="$2"
  local stage_root="$STAGING_DIR/${os}-${target}"
  local toolchain_dir="$stage_root/toolchain"
  mkdir -p "$toolchain_dir/bin"

  local mode="real"
  if [[ $PLAN_ONLY -eq 1 ]]; then
    mode="plan"
  fi

  if [[ $PLAN_ONLY -eq 0 ]]; then
    if [[ "$os" != "$HOST_OS" ]]; then
      mode="deferred"
    fi
  fi

  if [[ "$mode" == "real" ]]; then
    local log="$LOG_DIR/${os}-${target}.build.log"
    if ! cargo build -p ncc-cli --release --target "$target" \
      --bin ncc --bin nfmt --bin nlint --bin nbuild --bin npkg --bin nide --bin nclsp --bin ncup --bin ncup-init >"$log" 2>&1; then
      if [[ $ALLOW_DEFERRED -eq 1 ]]; then
        mode="deferred"
      else
        echo "build failed for $os/$target (see $log)" >&2
        exit 1
      fi
    fi
  fi

  for bin in "${BINS[@]}"; do
    local filename="$bin"
    local src="target/$target/release/$bin"
    if [[ "$os" == "windows" ]]; then
      filename="${bin}.exe"
      src="${src}.exe"
    fi

    if [[ "$mode" == "real" && -f "$src" ]]; then
      cp "$src" "$toolchain_dir/bin/$filename"
      if [[ "$os" != "windows" ]]; then
        chmod +x "$toolchain_dir/bin/$filename"
      fi
    else
      if [[ "$os" == "windows" ]]; then
        printf 'deferred placeholder for %s\n' "$bin" > "$toolchain_dir/bin/$filename"
      else
        cat > "$toolchain_dir/bin/$filename" <<SH
#!/usr/bin/env bash
set -euo pipefail
echo "deferred placeholder for $bin ($os/$target)" >&2
exit 127
SH
        chmod +x "$toolchain_dir/bin/$filename"
      fi
    fi
  done

  write_layout_files "$os" "$target" "$toolchain_dir" "$mode"
  echo "$mode"
}

emit_deferred_json() {
  local out="$1"
  local reason="$2"
  python3 - <<'PY' "$out" "$reason"
import json,sys,pathlib
out,reason=sys.argv[1:3]
pathlib.Path(out).parent.mkdir(parents=True,exist_ok=True)
with open(out,'w',encoding='utf-8') as f:
  json.dump({"schema":"nc.phase10.distribution.deferred.v1","version":"2.0.0","state":"frozen","status":"deferred","reason":reason},f,indent=2,ensure_ascii=True)
  f.write("\n")
PY
}

package_windows() {
  local target="$1"
  local mode="$2"
  local stage_root="$STAGING_DIR/windows-${target}"
  local pkg_dir="$PACKAGES_DIR/windows"
  local zip_out="$pkg_dir/nc-v2.0.0-${target}-portable.zip"

  if command -v zip >/dev/null 2>&1; then
    (cd "$stage_root" && zip -qr "$ROOT_DIR/$zip_out" .)
  else
    python3 - <<'PY' "$stage_root" "$zip_out"
import pathlib,sys,zipfile
stage=pathlib.Path(sys.argv[1])
out=pathlib.Path(sys.argv[2])
out.parent.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED) as zf:
  for path in sorted(stage.rglob('*')):
    if path.is_file():
      zf.write(path,path.relative_to(stage).as_posix())
PY
  fi

  local winget_dir="$pkg_dir/winget/nc/2.0.0"
  mkdir -p "$winget_dir"
  local zip_sha
  zip_sha="$(sha256sum "$zip_out" | awk '{print $1}')"
  cat > "$winget_dir/nc.installer.yaml" <<YAML
PackageIdentifier: nC.nC
PackageVersion: 2.0.0
Installers:
  - Architecture: x64
    InstallerType: zip
    InstallerUrl: https://example.invalid/nc-v2.0.0-${target}-portable.zip
    InstallerSha256: ${zip_sha}
YAML
  cat > "$winget_dir/nc.defaultLocale.yaml" <<YAML
PackageIdentifier: nC.nC
PackageVersion: 2.0.0
PackageName: nC Toolchain
Publisher: nC Language Project
License: Proprietary-internal
ShortDescription: nC toolchain distribution package.
YAML
  cat > "$winget_dir/nc.yaml" <<YAML
PackageIdentifier: nC.nC
PackageVersion: 2.0.0
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
YAML

  if [[ $PLAN_ONLY -eq 1 ]]; then
    emit_deferred_json "$pkg_dir/nc-v2.0.0-${target}.msi.deferred.json" "plan-only mode"
  elif command -v wixl >/dev/null 2>&1; then
    emit_deferred_json "$pkg_dir/nc-v2.0.0-${target}.msi.deferred.json" "wixl pipeline declared but not yet materialized in this script"
  else
    emit_deferred_json "$pkg_dir/nc-v2.0.0-${target}.msi.deferred.json" "wixl/candle/light not available on current host"
  fi

  if [[ "$mode" == "deferred" && $ALLOW_DEFERRED -eq 0 ]]; then
    echo "windows package is deferred but strict mode is active" >&2
    exit 1
  fi
}

package_linux() {
  local target="$1"
  local mode="$2"
  local stage_root="$STAGING_DIR/linux-${target}"
  local pkg_dir="$PACKAGES_DIR/linux"
  local tar_out="$pkg_dir/nc-v2.0.0-${target}.tar.gz"

  tar -C "$stage_root" -czf "$tar_out" .
  cp tools/release/install_ncup_linux_v2_0_0.sh "$pkg_dir/install_ncup_linux_v2_0_0.sh"

  if [[ $PLAN_ONLY -eq 1 ]]; then
    emit_deferred_json "$pkg_dir/nc-toolchain_2.0.0_amd64.deb.deferred.json" "plan-only mode"
  elif command -v dpkg-deb >/dev/null 2>&1; then
    local deb_root
    deb_root="$(mktemp -d)"
    mkdir -p "$deb_root/DEBIAN" "$deb_root/opt/nc/v2.0.0"
    cp -R "$stage_root/toolchain" "$deb_root/opt/nc/v2.0.0/"
    cat > "$deb_root/DEBIAN/control" <<CTRL
Package: nc-toolchain
Version: 2.0.0
Section: devel
Priority: optional
Architecture: amd64
Maintainer: nC Release Team <release@example.invalid>
Description: nC toolchain distribution bundle
CTRL
    local dpkg_build_cmd=(dpkg-deb --build)
    if dpkg-deb --help 2>/dev/null | grep -q -- '--root-owner-group'; then
      dpkg_build_cmd+=(--root-owner-group)
    fi
    if ! "${dpkg_build_cmd[@]}" "$deb_root" "$pkg_dir/nc-toolchain_2.0.0_amd64.deb" >/dev/null; then
      rm -rf "$deb_root"
      if [[ $ALLOW_DEFERRED -eq 1 ]]; then
        rm -f "$pkg_dir/nc-toolchain_2.0.0_amd64.deb"
        emit_deferred_json "$pkg_dir/nc-toolchain_2.0.0_amd64.deb.deferred.json" "dpkg-deb build failed on current host"
      else
        echo "dpkg-deb build failed for linux/$target in strict mode" >&2
        exit 1
      fi
    else
      rm -rf "$deb_root"
    fi
  else
    emit_deferred_json "$pkg_dir/nc-toolchain_2.0.0_amd64.deb.deferred.json" "dpkg-deb not available"
  fi

  if [[ $PLAN_ONLY -eq 1 ]]; then
    emit_deferred_json "$pkg_dir/nc-toolchain-2.0.0.rpm.deferred.json" "plan-only mode"
  elif command -v rpmbuild >/dev/null 2>&1; then
    local rpm_top
    rpm_top="$(mktemp -d)"
    mkdir -p "$rpm_top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS,RPMDB,tmp}
    local rpm_changelog_date
    rpm_changelog_date="$(LC_ALL=C date -u '+%a %b %d %Y')"
    local src_tar="$rpm_top/SOURCES/nc-toolchain-2.0.0.tar.gz"
    tar -C "$stage_root" -czf "$src_tar" .
    cat > "$rpm_top/SPECS/nc-toolchain.spec" <<SPEC
Name: nc-toolchain
Version: 2.0.0
Release: 1
Summary: nC toolchain distribution bundle
License: Proprietary-internal
BuildArch: x86_64
Source0: nc-toolchain-2.0.0.tar.gz

%description
nC toolchain distribution bundle.

%prep
mkdir -p %{_builddir}/nc-toolchain
cd %{_builddir}/nc-toolchain
tar -xzf %{SOURCE0}

%install
mkdir -p %{buildroot}/opt/nc/v2.0.0
cp -a %{_builddir}/nc-toolchain/toolchain %{buildroot}/opt/nc/v2.0.0/

%files
/opt/nc/v2.0.0/toolchain

%changelog
* ${rpm_changelog_date} nC Release Team <release@example.invalid> - 2.0.0-1
- Initial package
SPEC
    if ! rpmbuild \
      --define "_topdir $rpm_top" \
      --define "_dbpath %{_topdir}/RPMDB" \
      --define "_tmppath %{_topdir}/tmp" \
      --define "_build_id_links none" \
      -bb "$rpm_top/SPECS/nc-toolchain.spec" >/dev/null; then
      rm -rf "$rpm_top"
      if [[ $ALLOW_DEFERRED -eq 1 ]]; then
        emit_deferred_json "$pkg_dir/nc-toolchain-2.0.0.rpm.deferred.json" "rpmbuild failed on current host"
      else
        echo "rpmbuild failed for linux/$target in strict mode" >&2
        exit 1
      fi
    else
      find "$rpm_top/RPMS" -name '*.rpm' -exec cp {} "$pkg_dir/" \;
      rm -rf "$rpm_top"
    fi
  else
    emit_deferred_json "$pkg_dir/nc-toolchain-2.0.0.rpm.deferred.json" "rpmbuild not available"
  fi

  if [[ "$mode" == "deferred" && $ALLOW_DEFERRED -eq 0 ]]; then
    echo "linux package is deferred but strict mode is active" >&2
    exit 1
  fi
}

package_macos() {
  local target="$1"
  local mode="$2"
  local stage_root="$STAGING_DIR/macos-${target}"
  local pkg_dir="$PACKAGES_DIR/macos"
  local tar_out="$pkg_dir/nc-v2.0.0-${target}.tar.gz"

  tar -C "$stage_root" -czf "$tar_out" .

  if [[ $PLAN_ONLY -eq 1 ]]; then
    emit_deferred_json "$pkg_dir/nc-v2.0.0-${target}.pkg.deferred.json" "plan-only mode"
  elif command -v pkgbuild >/dev/null 2>&1 && command -v productbuild >/dev/null 2>&1; then
    emit_deferred_json "$pkg_dir/nc-v2.0.0-${target}.pkg.deferred.json" "pkgbuild/productbuild detected but packaging path is runner-native only"
  else
    emit_deferred_json "$pkg_dir/nc-v2.0.0-${target}.pkg.deferred.json" "pkgbuild/productbuild not available on current host"
  fi

  local brew_dir="$pkg_dir/brew-tap/Formula"
  mkdir -p "$brew_dir"
  local tar_sha
  tar_sha="$(sha256sum "$tar_out" | awk '{print $1}')"
  cat > "$brew_dir/nc-toolchain.rb" <<RUBY
class NcToolchain < Formula
  desc "nC toolchain bundle"
  homepage "https://example.invalid/nc"
  url "https://example.invalid/nc-v2.0.0-${target}.tar.gz"
  sha256 "${tar_sha}"
  license "Proprietary-internal"

  def install
    prefix.install Dir["toolchain"]
    bin.install_symlink Dir["#{prefix}/toolchain/bin/*"]
  end
end
RUBY
  cat > "$pkg_dir/brew-tap/README.md" <<MD
# nC Brew Tap (v2.0.0)

Tap path generated by F10-P3 distribution pipeline.
MD

  if [[ "$mode" == "deferred" && $ALLOW_DEFERRED -eq 0 ]]; then
    echo "macos package is deferred but strict mode is active" >&2
    exit 1
  fi
}

for os_name in "${OS_LIST[@]}"; do
  os_name="$(echo "$os_name" | xargs | tr '[:upper:]' '[:lower:]')"
  case "$os_name" in
    windows|linux|macos) ;;
    *)
      echo "unsupported os selector: $os_name" >&2
      exit 2
      ;;
  esac

  if [[ $HOST_ONLY -eq 1 ]]; then
    target="$host_target"
  else
    target="$(default_target_for_os "$os_name")"
  fi

  mode="$(stage_toolchain "$os_name" "$target")"
  SELECTED_OS+=("$os_name")
  notes="ok"
  if [[ "$mode" != "real" ]]; then
    notes="${mode}"
  fi

  case "$os_name" in
    windows) package_windows "$target" "$mode" ;;
    linux) package_linux "$target" "$mode" ;;
    macos) package_macos "$target" "$mode" ;;
  esac

  printf "%s\t%s\t%s\t%s\t%s\n" "$os_name" "$target" "$mode" "built" "$notes" >> "$SUMMARY_TSV"
done

for os_name in "${SELECTED_OS[@]}"; do
  pkg_os_dir="$PACKAGES_DIR/$os_name"
  integ_os_dir="$INTEGRITY_DIR/$os_name"
  if dir_has_files "$pkg_os_dir"; then
    tools/release/sign_artifacts_v1_0_1.sh "$pkg_os_dir" "$integ_os_dir" --key-env "$SIGN_KEY_ENV"
  else
    echo "SKIP: no artifacts to sign under $pkg_os_dir"
  fi
done

"${NCC_RELEASE_VERIFY:-ncc-release}" release-layout \
  --staging "$STAGING_DIR" \
  --report "$LAYOUT_REPORT_PATH"

python3 - <<'PY' "$SUMMARY_TSV" "$REPORT_PATH" "$OUT_DIR"
import csv, datetime, json, pathlib, sys
summary, report, out_dir = sys.argv[1:4]
rows=[]
with open(summary, 'r', encoding='utf-8') as f:
  reader=csv.DictReader(f, delimiter='\t')
  for row in reader:
    rows.append(row)
status='ok'
if any(r.get('stage') in ('plan','deferred') for r in rows):
  status='partial'
payload={
  'schema':'nc.phase10.distribution.os.pipeline.report.v1',
  'version':'2.0.0',
  'state':'frozen',
  'generated_at': datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'status': status,
  'out_dir': out_dir,
  'rows': rows,
}
path=pathlib.Path(report)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, indent=2, ensure_ascii=True)+'\n', encoding='utf-8')
print(f'OK: phase10 os distribution pipeline report {path}')
PY

echo "OK: F10-P3 distribution by OS complete -> $OUT_DIR"
