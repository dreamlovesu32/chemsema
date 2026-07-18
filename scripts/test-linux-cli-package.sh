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
package_dir="$(find "$temporary" -mindepth 1 -maxdepth 1 -type d -name 'chemcore-cli-*-linux-x86_64' -print -quit)"
[[ -n "$package_dir" ]] || { echo "Package root not found" >&2; exit 1; }

# Portable extraction: no installation and no shell changes.
"$package_dir/bin/chemcore-cli" version --pretty >/dev/null
"$package_dir/bin/chemcore-cli" doctor --pretty >/dev/null

# Dedicated scientific-software prefix with zsh PATH management.
test_home="$temporary/home"
mkdir -p "$test_home"
HOME="$test_home" SHELL=/usr/bin/zsh "$package_dir/install.sh" \
  --prefix "$test_home/chemcore-cli"
test -x "$test_home/chemcore-cli/bin/chemcore-cli"
grep -F '# >>> chemcore-cli >>>' "$test_home/.zshrc" >/dev/null
PATH="$test_home/chemcore-cli/bin:$PATH" chemcore-cli capabilities >/dev/null
HOME="$test_home" SHELL=/usr/bin/zsh \
  "$test_home/chemcore-cli/share/chemcore/uninstall.sh" \
  --prefix "$test_home/chemcore-cli"
test ! -e "$test_home/chemcore-cli/bin/chemcore-cli"
! grep -F '# >>> chemcore-cli >>>' "$test_home/.zshrc" >/dev/null

# Conventional per-user prefix without changing shell configuration.
HOME="$test_home" SHELL=/bin/bash "$package_dir/install.sh" \
  --prefix "$test_home/.local" --no-modify-path
test -x "$test_home/.local/bin/chemcore-cli"
test ! -e "$test_home/.bashrc"
HOME="$test_home" SHELL=/bin/bash \
  "$test_home/.local/share/chemcore/uninstall.sh" \
  --prefix "$test_home/.local" --no-modify-path
test ! -e "$test_home/.local/bin/chemcore-cli"

echo "Linux CLI package install/uninstall tests passed."
