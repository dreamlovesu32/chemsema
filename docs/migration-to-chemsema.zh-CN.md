# 项目改名：ChemCore → ChemSema

项目于 2026 年 7 月 19 日由 **ChemCore** 正式改名为 **ChemSema**。新名称更准确地
表达项目关注的核心：化学含义、文档语义，以及人与软件、agent 之间可靠而可审计的
操作。

这次改名是明确承认的品牌迁移，但不会重写 Git 历史。当前源码、package 与 crate
名称、命令、环境变量、文档和仓库路径统一使用 `ChemSema` 或 `chemsema`；既有提交和
tag 保持原样，以保留 commit hash、签名和来源关系。

兼容承诺：

- GitHub 旧仓库地址依靠仓库改名机制继续跳转，并且以后不会重新占用旧仓库名。
- 旧 GitHub Pages 路径由永久兼容页转到
  <https://dreamlovesu32.github.io/chemsema/>。
- 本地每次提交前和 GitHub Actions 每日任务都会检查这两个入口，以便平台行为变化时
  尽快发现。
- `.ccjs`、`.ccjz` 等既有文档扩展名保持不变。

ChemSema 的公开版本线从 `1.0.0-beta.1` 重新开始。由于旧 Git 历史中已经存在
`v1.0.0-beta.1` tag，新品牌采用不会混淆的 tag：`chemsema-v1.0.0-beta.1`。
