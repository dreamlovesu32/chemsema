# ChemCore

> 维护者开场留在这里。

## 技术概览

ChemCore 是围绕共享 Rust 内核构建的跨平台化学文档编辑器。内核负责文档模型、编辑行为、命中测试、化学标签逻辑、面向 CDXML/CDX 的导入导出，以及 render primitive 生成。浏览器、桌面端和 Office/OLE 宿主都消费这套共享行为，不在各自前端里重新实现化学规则。

## 仓库结构

```text
chemcore/
  crates/chemcore-engine/          Rust 文档、编辑、渲染、CDXML 和 WASM 内核
  crates/chemcore-desktop-service/ 桌面端原生 engine session 与文件能力
  apps/chemcore-desktop/           Tauri Windows 桌面应用
  apps/chemcore-office/            Windows Office/OLE 集成服务
  viewer/                          浏览器编辑器宿主和生成的 WASM package
  docs/                            架构、格式、渲染和行为文档
  examples/                        ChemCore 原生文档示例
  scripts/                         构建、验证和回归辅助脚本
  shared/                          Rust 和 viewer 共用 JSON 数据
```

## 环境要求

- Rust stable，Windows 桌面路径需要 MSVC toolchain
- Node.js 和 npm
- Python 3，用于本地静态服务和部分可选分析脚本
- `npm run build:engine-wasm` 会在需要时安装 `wasm-pack`
- 桌面 shell 与 Office/OLE 集成需要 Windows

## 快速开始

```bash
npm install
cargo test
npm run build:engine-wasm
```

在仓库根目录启动浏览器编辑器：

```bash
python -m http.server 8765 --bind 127.0.0.1 --directory .
```

然后打开：

```text
http://127.0.0.1:8765/viewer/
```

运行 Windows 桌面端：

```bash
npm run desktop:dev
```

构建 release 二进制：

```bash
npm run desktop:build-fast
cargo build -p chemcore-office --release
```

为当前用户注册 Office/OLE 集成：

```bash
npm run office:register-dev
```

取消注册：

```bash
npm run office:unregister-dev
```

## 验证

主要验证命令：

```bash
npm run verify
```

它会运行 Rust 测试、重建浏览器 engine WASM、检查 viewer JavaScript 语法，并确认 `viewer/engine` 生成物已同步。

常用定向命令：

```bash
npm test
cargo test -p chemcore-engine
cargo test -p chemcore-office
npm run build:engine-wasm
node --check viewer/app.js
```

部分脚本会和本机桌面应用或 Office 做输出对照。这些流程是可选的，可能需要 Windows 专有软件、本地文档，或用 `CHEMCORE_PYTHON` 指向装有分析依赖的 Python 环境。

## 设计文档

- [docs/architecture.md](./docs/architecture.md)
- [docs/format-v0.1.md](./docs/format-v0.1.md)
- [docs/project-rules.zh-CN.md](./docs/project-rules.zh-CN.md)
- [docs/implicit-hydrogen-rules.zh-CN.md](./docs/implicit-hydrogen-rules.zh-CN.md)
- [docs/abbreviation-recognition-rules.zh-CN.md](./docs/abbreviation-recognition-rules.zh-CN.md)
- [docs/bond-rendering-rules.zh-CN.md](./docs/bond-rendering-rules.zh-CN.md)
- [docs/editor-command-history.md](./docs/editor-command-history.md)
- [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)

## 许可证

ChemCore 使用 Apache License, Version 2.0 授权。见 [LICENSE](./LICENSE) 和 [NOTICE](./NOTICE)。
