# 历史字形裁剪表

原来的 `shared/glyph_clip_polygons.json` 字符表及其生成器已经删除。Rust 内核不再保留旧的逐字符查表输入。

当前实现根据真实字体轮廓在运行时派生退让几何，规则见 [glyph-kernel.zh-CN.md](./glyph-kernel.zh-CN.md)。不要再增加字符特例，也不要把旧 manifest 恢复成渲染 fallback。
