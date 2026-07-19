#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cli_path="$root_dir/target/release/chemsema-cli"
out_dir="$root_dir/dist/chemsema-cli"
version=""

usage() {
  cat <<'EOF'
Package the ChemSema CLI Linux x86_64 portable distribution.

Usage: scripts/package-linux-cli.sh --version <version> [options]

Options:
  --cli <path>       Linux chemsema-cli executable
  --out-dir <path>   Output directory
  --version <value>  Package version
EOF
}

while (($#)); do
  case "$1" in
    --cli) cli_path="$2"; shift 2 ;;
    --out-dir) out_dir="$2"; shift 2 ;;
    --version) version="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -f "$cli_path" ]] || { echo "CLI binary not found: $cli_path" >&2; exit 1; }
if [[ -z "$version" ]]; then
  version="$($cli_path --version | awk '{ print $NF }')"
fi
[[ -n "$version" ]] || { echo "Could not determine the CLI version" >&2; exit 1; }

package_name="chemsema-cli-${version}-linux-x86_64"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
stage="$temporary/$package_name"
mkdir -p "$stage/bin" "$stage/share/chemsema" "$stage/plugins" "$out_dir"

install -m755 "$cli_path" "$stage/bin/chemsema-cli"
install -m755 "$root_dir/packaging/linux-cli/install.sh" "$stage/install.sh"
install -m755 "$root_dir/packaging/linux-cli/uninstall.sh" "$stage/uninstall.sh"
install -m644 "$root_dir/packaging/linux-cli/README.md" "$stage/README.md"
install -m644 "$root_dir/docs/chemsema-cli-guide.md" "$stage/share/chemsema/chemsema-cli-guide.md"
install -m644 "$root_dir/docs/chemsema-cli-guide.zh-CN.md" "$stage/share/chemsema/chemsema-cli-guide.zh-CN.md"
install -m644 "$root_dir/LICENSE" "$stage/share/chemsema/LICENSE"
printf '%s\n' "$version" > "$stage/share/chemsema/VERSION"
(
  cd "$stage"
  sha256sum bin/chemsema-cli share/chemsema/chemsema-cli-guide.md \
    share/chemsema/chemsema-cli-guide.zh-CN.md share/chemsema/LICENSE \
    > share/chemsema/SHA256SUMS
)

archive="$out_dir/$package_name.tar.gz"
tar -C "$temporary" -czf "$archive" "$package_name"
(
  cd "$out_dir"
  sha256sum "$(basename "$archive")" > "$(basename "$archive").sha256"
)

echo "$archive"
