#!/usr/bin/env bash
set -euo pipefail

archive="${1:-}"
[[ -f "$archive" ]] || {
  echo "Usage: scripts/test-linux-cli-package.sh <archive.tar.gz>" >&2
  exit 2
}

temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
tar -xzf "$archive" -C "$temporary"
package_dir="$(find "$temporary" -mindepth 1 -maxdepth 1 -type d -name 'chemsema-cli-*-linux-x86_64' -print -quit)"
[[ -n "$package_dir" ]] || { echo "Package root not found" >&2; exit 1; }

# Portable extraction: no installation and no shell changes.
"$package_dir/bin/chemsema-cli" version --pretty >/dev/null
"$package_dir/bin/chemsema-cli" doctor --pretty >/dev/null

# Dedicated scientific-software prefix with zsh PATH management.
test_home="$temporary/home"
mkdir -p "$test_home"
HOME="$test_home" SHELL=/usr/bin/zsh "$package_dir/install.sh" \
  --prefix "$test_home/chemsema-cli"
test -x "$test_home/chemsema-cli/bin/chemsema-cli"
test -d "$test_home/chemsema-cli/plugins"
grep -F '# >>> chemsema-cli >>>' "$test_home/.zshrc" >/dev/null
PATH="$test_home/chemsema-cli/bin:$PATH" chemsema-cli capabilities >/dev/null
HOME="$test_home" SHELL=/usr/bin/zsh \
  "$test_home/chemsema-cli/share/chemsema/uninstall.sh" \
  --prefix "$test_home/chemsema-cli"
test ! -e "$test_home/chemsema-cli/bin/chemsema-cli"
! grep -F '# >>> chemsema-cli >>>' "$test_home/.zshrc" >/dev/null

# Conventional per-user prefix without changing shell configuration.
HOME="$test_home" SHELL=/bin/bash "$package_dir/install.sh" \
  --prefix "$test_home/.local" --no-modify-path
test -x "$test_home/.local/bin/chemsema-cli"
test -d "$test_home/.local/plugins"
test ! -e "$test_home/.bashrc"
HOME="$test_home" SHELL=/bin/bash \
  "$test_home/.local/share/chemsema/uninstall.sh" \
  --prefix "$test_home/.local" --no-modify-path
test ! -e "$test_home/.local/bin/chemsema-cli"

echo "Linux CLI package install/uninstall tests passed."
