# Install ChemCore CLI On Linux

ChemCore publishes a portable `linux-x86_64.tar.gz` archive, following the
common distribution model used by scientific command-line software. The CLI
inside the archive is a single executable; the surrounding directories contain
guides, license information, checksums, and install/uninstall helpers.

Build the archive from a Windows maintainer checkout with WSL:

```powershell
npm run cli:linux:package
```

On native Linux the same command builds with Cargo and packages the native
release binary. Output is written to `dist/chemcore-cli/` together with an
archive SHA-256 file.

For portable use, extract the archive and run `bin/chemcore-cli`. For a
dedicated home-directory installation:

```bash
./install.sh --prefix "$HOME/chemcore-cli"
source ~/.zshrc  # or ~/.bashrc when bash is the login shell
chemcore-cli doctor --pretty
```

The installer selects `.zshrc` or `.bashrc` from `$SHELL`, unless
`--shell-config` specifies a file. It owns a marked PATH block and safely
replaces it during reinstall. Use `--no-modify-path` for `/usr/local`, module
files, containers, or manually managed shell configuration.

Uninstall a dedicated home installation with:

```bash
"$HOME/chemcore-cli/share/chemcore/uninstall.sh" \
  --prefix "$HOME/chemcore-cli"
```

The uninstaller removes only the installed ChemCore files and its marked PATH
block. It removes directories only when they are empty.
