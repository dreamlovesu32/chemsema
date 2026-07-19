#!/usr/bin/env bash
set -euo pipefail

prefix="${HOME}/.local"
shell_config="auto"
modify_path=true

usage() {
  cat <<'EOF'
Uninstall ChemSema CLI.

Usage: uninstall.sh [options]

Options:
  --prefix <path>         Installation prefix (default: $HOME/.local)
  --shell-config <value>  auto, none, or the startup file used at install time
  --no-modify-path        Do not update a shell startup file
  -h, --help              Show this help
EOF
}

while (($#)); do
  case "$1" in
    --prefix)
      [[ $# -ge 2 ]] || { echo "--prefix requires a path" >&2; exit 2; }
      prefix="$2"
      shift 2
      ;;
    --shell-config)
      [[ $# -ge 2 ]] || { echo "--shell-config requires auto, none, or a path" >&2; exit 2; }
      shell_config="$2"
      shift 2
      ;;
    --no-modify-path)
      modify_path=false
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$shell_config" == "none" ]] || [[ "$modify_path" != true ]]; then
  config_path=""
elif [[ "$shell_config" != "auto" ]]; then
  config_path="$shell_config"
else
  case "${SHELL:-}" in
    */zsh) config_path="${HOME}/.zshrc" ;;
    */bash) config_path="${HOME}/.bashrc" ;;
    *) config_path="${HOME}/.profile" ;;
  esac
fi

marker_begin="# >>> chemsema-cli >>>"
marker_end="# <<< chemsema-cli <<<"

if [[ -d "$prefix/plugins" ]] && find "$prefix/plugins" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
  echo "ChemSema plugins are still installed under $prefix/plugins." >&2
  echo "Uninstall them before removing ChemSema CLI." >&2
  exit 1
fi
if [[ -n "$config_path" && -f "$config_path" ]]; then
  temporary="$(mktemp "${config_path}.chemsema.XXXXXX")"
  awk -v begin="$marker_begin" -v end="$marker_end" '
    $0 == begin { skipping = 1; next }
    $0 == end { skipping = 0; next }
    !skipping { print }
  ' "$config_path" > "$temporary"
  chmod --reference="$config_path" "$temporary" 2>/dev/null || true
  mv "$temporary" "$config_path"
fi

rm -f \
  "$prefix/bin/chemsema-cli" \
  "$prefix/share/chemsema/chemsema-cli-guide.md" \
  "$prefix/share/chemsema/chemsema-cli-guide.zh-CN.md" \
  "$prefix/share/chemsema/LICENSE" \
  "$prefix/share/chemsema/VERSION" \
  "$prefix/share/chemsema/SHA256SUMS" \
  "$prefix/share/chemsema/uninstall.sh"
rmdir "$prefix/plugins" "$prefix/share/chemsema" "$prefix/share" "$prefix/bin" "$prefix" 2>/dev/null || true

echo "ChemSema CLI removed from $prefix"
if [[ -n "$config_path" ]]; then
  echo "ChemSema PATH block removed from $config_path"
fi
