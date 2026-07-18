#!/usr/bin/env bash
set -euo pipefail

package_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
prefix="${HOME}/.local"
shell_config="auto"
modify_path=true

usage() {
  cat <<'EOF'
Install ChemCore CLI.

Usage: ./install.sh [options]

Options:
  --prefix <path>         Installation prefix (default: $HOME/.local)
  --shell-config <value>  auto, none, or a shell startup file path (default: auto)
  --no-modify-path        Do not update a shell startup file
  -h, --help              Show this help

Examples:
  ./install.sh
  ./install.sh --prefix "$HOME/chemcore-cli"
  sudo ./install.sh --prefix /usr/local --no-modify-path
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

[[ "$prefix" != *$'\n'* ]] || { echo "Prefix must not contain a newline" >&2; exit 2; }
prefix="$(mkdir -p "$prefix" && cd "$prefix" && pwd)"

resolve_shell_config() {
  if [[ "$shell_config" == "none" ]] || [[ "$modify_path" != true ]]; then
    printf '%s' ""
  elif [[ "$shell_config" != "auto" ]]; then
    printf '%s' "$shell_config"
  else
    case "${SHELL:-}" in
      */zsh) printf '%s' "${HOME}/.zshrc" ;;
      */bash) printf '%s' "${HOME}/.bashrc" ;;
      *) printf '%s' "${HOME}/.profile" ;;
    esac
  fi
}

config_path="$(resolve_shell_config)"
marker_begin="# >>> chemcore-cli >>>"
marker_end="# <<< chemcore-cli <<<"

remove_path_block() {
  local path="$1"
  local temporary
  [[ -f "$path" ]] || return 0
  temporary="$(mktemp "${path}.chemcore.XXXXXX")"
  awk -v begin="$marker_begin" -v end="$marker_end" '
    $0 == begin { skipping = 1; next }
    $0 == end { skipping = 0; next }
    !skipping { print }
  ' "$path" > "$temporary"
  chmod --reference="$path" "$temporary" 2>/dev/null || true
  mv "$temporary" "$path"
}

install -Dm755 "$package_dir/bin/chemcore-cli" "$prefix/bin/chemcore-cli"
install -Dm644 "$package_dir/share/chemcore/chemcore-cli-guide.md" \
  "$prefix/share/chemcore/chemcore-cli-guide.md"
install -Dm644 "$package_dir/share/chemcore/chemcore-cli-guide.zh-CN.md" \
  "$prefix/share/chemcore/chemcore-cli-guide.zh-CN.md"
install -Dm644 "$package_dir/share/chemcore/LICENSE" "$prefix/share/chemcore/LICENSE"
install -Dm644 "$package_dir/share/chemcore/VERSION" "$prefix/share/chemcore/VERSION"
install -Dm644 "$package_dir/share/chemcore/SHA256SUMS" "$prefix/share/chemcore/SHA256SUMS"
install -Dm755 "$package_dir/uninstall.sh" "$prefix/share/chemcore/uninstall.sh"
mkdir -p "$prefix/plugins"

if [[ -n "$config_path" ]]; then
  mkdir -p "$(dirname "$config_path")"
  touch "$config_path"
  remove_path_block "$config_path"
  display_bin="$prefix/bin"
  if [[ "$display_bin" == "$HOME/"* ]]; then
    display_bin="\$HOME/${display_bin#"$HOME/"}"
  fi
  {
    printf '\n%s\n' "$marker_begin"
    printf 'export PATH="%s:$PATH"\n' "$display_bin"
    printf '%s\n' "$marker_end"
  } >> "$config_path"
fi

"$prefix/bin/chemcore-cli" version --pretty >/dev/null

echo "ChemCore CLI installed to $prefix/bin/chemcore-cli"
echo "Plugin directory: $prefix/plugins"
if [[ -n "$config_path" ]]; then
  echo "PATH updated in $config_path"
  echo "Open a new shell or run: source \"$config_path\""
elif [[ ":${PATH}:" != *":$prefix/bin:"* ]]; then
  echo "Add $prefix/bin to PATH before invoking chemcore-cli."
fi
echo "Uninstall with: $prefix/share/chemcore/uninstall.sh --prefix \"$prefix\""
