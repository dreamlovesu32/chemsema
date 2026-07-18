# 在 Linux 上安装 ChemCore CLI

ChemCore 使用科研命令行软件常见的 `linux-x86_64.tar.gz` 便携发行方式。
压缩包中的 CLI 本体是单个可执行文件；外围目录提供指南、许可证、校验值和
安装/卸载脚本。

Windows 维护者可以通过 WSL 构建发行包：

```powershell
npm run cli:linux:package
```

在原生 Linux 中，同一命令会通过 Cargo 构建并打包原生 release 二进制。
产物和压缩包 SHA-256 文件位于 `dist/chemcore-cli/`。

便携使用时，解压后直接运行 `bin/chemcore-cli`。安装到用户主目录下的独立
目录时：

```bash
./install.sh --prefix "$HOME/chemcore-cli"
source ~/.zshrc  # 登录 shell 是 bash 时使用 ~/.bashrc
chemcore-cli doctor --pretty
```

安装器根据 `$SHELL` 选择 `.zshrc` 或 `.bashrc`，也可以通过
`--shell-config` 指定。它只管理一段带明确标记的 PATH 配置，重复安装会安全
替换该区块。安装到 `/usr/local`、使用 module file、容器或手动管理 shell
配置时，可以传入 `--no-modify-path`。

安装器同时创建稳定扩展根目录 `<prefix>/plugins`。插件安装器先验证
ChemCore CLI，再安装到独立子目录（例如
`<prefix>/plugins/chemcore-reaction`），无需再次修改 PATH。

卸载用户主目录下的独立安装：

```bash
"$HOME/chemcore-cli/share/chemcore/uninstall.sh" \
  --prefix "$HOME/chemcore-cli"
```

卸载器只删除 ChemCore 安装文件和对应 PATH 标记区块；目录为空时才会删除。
卸载 ChemCore 前需要先卸载其插件。
