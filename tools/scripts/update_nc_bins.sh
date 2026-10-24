#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: tools/scripts/update_nc_bins.sh [--quick] [--source-dir DIR] [--bin-dir DIR]

Builds release binaries and installs the nC command suite into DIR.

Default DIR:
  $HOME/.local/bin

Modes:
  default   run fmt/check/test/clippy before building release binaries
  --quick   skip validation and only build/install release binaries
  --source-dir DIR
            install an already-built complete suite without rebuilding it

Installed when present:
  ncc nclsp nc-sys-lsp ghor nfmt nlint nbuild nide npkg ncup ncup-init nwal

Required native companion:
  ghor-stage1
EOF
}

quick=false
bin_dir="${HOME}/.local/bin"
bin_dir_explicit=false
source_dir=""
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$1" | awk '{print $NF}'
  else
    echo "no SHA-256 tool found (sha256sum, shasum or openssl)" >&2
    return 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --quick)
      quick=true
      shift
      ;;
    --bin-dir)
      [[ $# -ge 2 ]] || { echo "--bin-dir expects a directory" >&2; exit 2; }
      bin_dir="$2"
      bin_dir_explicit=true
      shift 2
      ;;
    --source-dir)
      [[ $# -ge 2 ]] || { echo "--source-dir expects a directory" >&2; exit 2; }
      source_dir="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$source_dir" ]]; then
  if [[ "$quick" == false ]]; then
    cargo fmt --all -- --check
    cargo check --workspace --all-targets
    cargo test --workspace --all-targets
    cargo clippy --workspace --all-targets -- -D warnings
  fi

  cargo build --release -p ncc-cli --bins

  target_dir="${CARGO_TARGET_DIR:-target}"
  if [[ "$target_dir" != /* ]]; then
    target_dir="$PWD/$target_dir"
  fi
  GO111MODULE=off go build -trimpath -o "$target_dir/release/nc-sys-lsp" tools/nc-sys-lsp/main.go
  source_dir="$target_dir/release"
  "$source_dir/ncc" build tools/ghor/nc/main.n "$source_dir/ghor-stage1" \
    --out-dir "$target_dir/ghor-stage1-artifacts" \
    --profile release-fast \
    --no-incremental
elif [[ ! -d "$source_dir" ]]; then
  echo "source directory does not exist: $source_dir" >&2
  exit 1
fi

source_dir="$(cd "$source_dir" && pwd)"
"$source_dir/ghor-stage1" >/dev/null

target_spec_source=""
for candidate in "$source_dir/targets" "$source_dir/../../targets" "$repo_root/targets"; do
  if [[ -d "$candidate" ]] && compgen -G "$candidate/*.nctarget" >/dev/null; then
    target_spec_source="$(cd "$candidate" && pwd)"
    break
  fi
done
if [[ -z "$target_spec_source" ]]; then
  echo "missing Primitive target registry" >&2
  exit 1
fi
link_source="$(cd "$target_spec_source/../link" && pwd)"
link_files=("$link_source"/*)
if [[ ! -f "${link_files[0]}" ]]; then
  echo "missing Primitive linker resources: $link_source" >&2
  exit 1
fi

mkdir -p "$bin_dir"

bins=(ncc nclsp nc-sys-lsp ghor nfmt nlint nbuild nide npkg ncup ncup-init nwal)
companions=(ghor-stage1)
artifacts=("${bins[@]}" "${companions[@]}")
mirror_dirs=()
if [[ "$bin_dir_explicit" == false ]]; then
  for bin in "${bins[@]}"; do
    active_path="$(command -v "$bin" 2>/dev/null || true)"
    [[ -n "$active_path" ]] || continue
    active_dir="$(dirname "$active_path")"
    case "$active_dir" in
      "$HOME/.cargo/bin"|"$HOME/bin")
        if [[ "$active_dir" != "$bin_dir" ]]; then
          already_listed=false
          for mirror_dir in "${mirror_dirs[@]}"; do
            if [[ "$mirror_dir" == "$active_dir" ]]; then
              already_listed=true
              break
            fi
          done
          if [[ "$already_listed" == false ]]; then
            mirror_dirs+=("$active_dir")
          fi
        fi
        ;;
    esac
  done
fi
for bin in "${artifacts[@]}"; do
  src="$source_dir/$bin"
  if [[ ! -s "$src" ]]; then
    echo "missing or empty release binary: $src" >&2
    exit 1
  fi
done

stage="$(mktemp -d "$bin_dir/.ncup-stage.XXXXXX")"
backup="$(mktemp -d "$bin_dir/.ncup-backup.XXXXXX")"
target_destination="$(cd "$bin_dir/.." && pwd)/share/nC/targets"
link_destination="$(cd "$bin_dir/.." && pwd)/share/nC/link"
mkdir -p "$stage/targets" "$backup/targets" "$target_destination" \
  "$stage/link" "$backup/link" "$link_destination"
for index in "${!mirror_dirs[@]}"; do
  mkdir -p "$backup/mirror-$index"
  for bin in "${artifacts[@]}"; do
    mirror_path="${mirror_dirs[$index]}/$bin"
    if [[ -f "$mirror_path" ]]; then
      cp -p -- "$mirror_path" "$backup/mirror-$index/$bin"
    else
      : > "$backup/mirror-$index/$bin.absent"
    fi
  done
done
target_specs=("$target_spec_source"/*.nctarget)
cleanup() {
  rm -rf -- "$stage" "$backup"
}
trap cleanup EXIT

for bin in "${artifacts[@]}"; do
  src="$source_dir/$bin"
  install -m 755 "$src" "$stage/$bin"
  if [[ -f "$bin_dir/$bin" ]]; then
    cp -p -- "$bin_dir/$bin" "$backup/$bin"
  else
    : > "$backup/$bin.absent"
  fi
done

for src in "${target_specs[@]}"; do
  name="$(basename "$src")"
  install -m 644 "$src" "$stage/targets/$name"
  if [[ -f "$target_destination/$name" ]]; then
    cp -p -- "$target_destination/$name" "$backup/targets/$name"
  else
    : > "$backup/targets/$name.absent"
  fi
done

for src in "${link_files[@]}"; do
  name="$(basename "$src")"
  install -m 644 "$src" "$stage/link/$name"
  if [[ -f "$link_destination/$name" ]]; then
    cp -p -- "$link_destination/$name" "$backup/link/$name"
  else
    : > "$backup/link/$name.absent"
  fi
done

rollback() {
  for bin in "${artifacts[@]}"; do
    if [[ -f "$backup/$bin" ]]; then
      install -m 755 "$backup/$bin" "$bin_dir/$bin"
    elif [[ -f "$backup/$bin.absent" ]]; then
      rm -f -- "$bin_dir/$bin"
    fi
  done
  for src in "${target_specs[@]}"; do
    name="$(basename "$src")"
    if [[ -f "$backup/targets/$name" ]]; then
      install -m 644 "$backup/targets/$name" "$target_destination/$name"
    elif [[ -f "$backup/targets/$name.absent" ]]; then
      rm -f -- "$target_destination/$name"
    fi
  done
  for src in "${link_files[@]}"; do
    name="$(basename "$src")"
    if [[ -f "$backup/link/$name" ]]; then
      install -m 644 "$backup/link/$name" "$link_destination/$name"
    elif [[ -f "$backup/link/$name.absent" ]]; then
      rm -f -- "$link_destination/$name"
    fi
  done
  for index in "${!mirror_dirs[@]}"; do
    for bin in "${artifacts[@]}"; do
      mirror_path="${mirror_dirs[$index]}/$bin"
      if [[ -f "$backup/mirror-$index/$bin" ]]; then
        install -m 755 "$backup/mirror-$index/$bin" "$mirror_path"
      elif [[ -f "$backup/mirror-$index/$bin.absent" ]]; then
        rm -f -- "$mirror_path"
      fi
    done
  done
}

for bin in "${artifacts[@]}"; do
  if ! install -m 755 "$stage/$bin" "$bin_dir/$bin"; then
    rollback
    echo "installation failed for $bin; previous binary suite restored" >&2
    exit 1
  fi
  expected="$(hash_file "$stage/$bin")"
  actual="$(hash_file "$bin_dir/$bin")"
  if [[ "$expected" != "$actual" ]]; then
    rollback
    echo "checksum mismatch for $bin; previous binary suite restored" >&2
    exit 1
  fi
  echo "updated: $bin_dir/$bin sha256:$actual"
done

for index in "${!mirror_dirs[@]}"; do
  mirror_dir="${mirror_dirs[$index]}"
  mkdir -p "$mirror_dir"
  for bin in "${artifacts[@]}"; do
    mirror_path="$mirror_dir/$bin"
    if ! install -m 755 "$stage/$bin" "$mirror_path"; then
      rollback
      echo "installation failed for active command mirror $mirror_path; previous binary suite restored" >&2
      exit 1
    fi
    expected="$(hash_file "$stage/$bin")"
    actual="$(hash_file "$mirror_path")"
    if [[ "$expected" != "$actual" ]]; then
      rollback
      echo "checksum mismatch for active command mirror $mirror_path; previous binary suite restored" >&2
      exit 1
    fi
    echo "updated active command: $mirror_path sha256:$actual"
  done
done


for src in "${target_specs[@]}"; do
  name="$(basename "$src")"
  if ! install -m 644 "$stage/targets/$name" "$target_destination/$name"; then
    rollback
    echo "installation failed for Primitive target $name; previous suite restored" >&2
    exit 1
  fi
  expected="$(hash_file "$stage/targets/$name")"
  actual="$(hash_file "$target_destination/$name")"
  if [[ "$expected" != "$actual" ]]; then
    rollback
    echo "checksum mismatch for Primitive target $name; previous suite restored" >&2
    exit 1
  fi
done
echo "updated Primitive targets: $target_destination count:${#target_specs[@]}"

for src in "${link_files[@]}"; do
  name="$(basename "$src")"
  if ! install -m 644 "$stage/link/$name" "$link_destination/$name"; then
    rollback
    echo "installation failed for Primitive linker resource $name; previous suite restored" >&2
    exit 1
  fi
  expected="$(hash_file "$stage/link/$name")"
  actual="$(hash_file "$link_destination/$name")"
  if [[ "$expected" != "$actual" ]]; then
    rollback
    echo "checksum mismatch for Primitive linker resource $name; previous suite restored" >&2
    exit 1
  fi
done
echo "updated Primitive link resources: $link_destination count:${#link_files[@]}"

json_bin_dir="${bin_dir//\\/\\\\}"
json_bin_dir="${json_bin_dir//\"/\\\"}"
cat > "$bin_dir/.ncup-last-update.json" <<EOF
{
  "schema": "ncup.complete_update.install.v1",
  "status": "ok",
  "managed_bin_count": ${#bins[@]},
  "companion_artifact_count": ${#companions[@]},
  "managed_artifact_count": ${#artifacts[@]},
  "bin_dir": "$json_bin_dir"
}
EOF

echo
echo "Installed command paths:"
for bin in "${bins[@]}"; do
  if command -v "$bin" >/dev/null 2>&1; then
    command -v "$bin"
  elif [[ -x "$bin_dir/$bin" ]]; then
    echo "$bin_dir/$bin"
  else
    echo "not on PATH: $bin" >&2
  fi
done

echo "Native companions:"
for companion in "${companions[@]}"; do
  echo "$bin_dir/$companion"
done
