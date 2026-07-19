# ChemSema CLI for Linux x86_64

This archive is a self-contained, headless ChemSema CLI distribution. It does
not require Rust, Cargo, or Node.js.

## Portable use

Extract the archive and invoke the binary directly:

```bash
tar -xzf chemsema-cli-*-linux-x86_64.tar.gz
cd chemsema-cli-*-linux-x86_64
./bin/chemsema-cli doctor --pretty
```

You can keep the extracted directory anywhere and add its `bin` directory to
`PATH`, like many scientific software distributions.

## Install

The default user installation prefix is `$HOME/.local`:

```bash
./install.sh
```

For a dedicated directory under your home folder:

```bash
./install.sh --prefix "$HOME/chemsema-cli"
```

The installer detects zsh or bash from `$SHELL`, adds one marked PATH block to
the corresponding startup file, and replaces that block on reinstall. Override
the file with `--shell-config <path>`, or use `--no-modify-path`.

Every installation creates `<prefix>/plugins`. Extensions install into their
own directory below it and do not add another PATH entry. Uninstall plugins
before uninstalling the core CLI.

For a shared system installation:

```bash
sudo ./install.sh --prefix /usr/local --no-modify-path
```

## Uninstall

Use the uninstall script copied into the installation:

```bash
"$HOME/chemsema-cli/share/chemsema/uninstall.sh" --prefix "$HOME/chemsema-cli"
```

Pass the same `--shell-config` or `--no-modify-path` option used for install.
