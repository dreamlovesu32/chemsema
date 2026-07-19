# 公共 CDXML/CDX 往返基准集

这个基准使用公开、许可证清楚的 ChemDraw CDXML/CDX 文件，避免让保密科研文档成为
公开测试和论文结论不可替代的依据。上游文件下载到 Git 忽略的 `tmp/` 目录，不直接
vendoring 到 ChemSema 仓库。

当前固定版本的清单包含五个上游项目、共 413 个文件：

| 来源 | 许可证 | CDXML | CDX | 主要覆盖 |
| --- | --- | ---: | ---: | --- |
| RDKit | BSD-3-Clause | 94 | 126 | 解析回归、query、模板和专利结构 |
| Indigo | Apache-2.0 | 123 | 28 | 分子、反应、渲染和异常输入测试 |
| cdxml-toolkit | MIT | 34 | 2 | 完整线性、换行和分支反应路线图 |
| SAMPL6 | MIT | 1 | 2 | 已发表的主客体结构 |
| SAMPL9 | MIT | 2 | 1 | 已发表的主客体结构 |

其中两个文件是故意构造的异常输入；另有四个 `.cdx` 实际保存 Base64 传输文本，并非
原始 CDX 字节，因此单独分类。其余 407 个文件作为正向往返案例。

## 复现方法

```bash
npm run benchmark:cdxml-public:fetch
cargo build -p chemsema-cli
npm run benchmark:cdxml-public
```

可用 `CHEMSEMA_PUBLIC_CDXML_DIR` 修改下载目录。详细报告写入未跟踪的
`tmp/public-cdxml-roundtrip/report.json`。传入 `--strict-counts` 后，任何对象或计数漂移
都会让命令返回失败。

ChemSema 1.0.0-beta.1 的首轮基线可以导入并重新导入全部 407 个正向案例，也能拒绝两个
负向案例。其中 364 个案例精确保留分子、节点、键、对象、资源、样式和对象类型计数；
其余 43 个案例往返成功但存在计数漂移。大多数漂移来自导出时显式增加 group 对象，少数
涉及分子/资源或样式变化，需要继续做语义和视觉分析。计数一致性只是第一层信号，并不
等价于完整的文档保真度。

清单固定每个上游 commit，并记录许可证链接。语料变化时应更新清单、重新运行基准并提交
新的版本化 summary，而不是静默覆盖旧基线。

表中的许可证是各上游仓库公开声明的仓库许可证。下载器让文件继续留在原上游仓库中，
适合做可复现的外部基准；如果以后要把这些文件重新打包成独立数据集，还应逐文件复核
来源和署名要求，尤其是 RDKit 中源自专利的 fixtures。
