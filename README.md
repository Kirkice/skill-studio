# skill studio

本地优先的 Skill 引用图谱与源文件编辑器。桌面界面使用 Rust + GPUI，不依赖 Node.js、React、Tauri 或 WebView。

## 运行与验证

需要 Rust 1.95.0。Windows 需要 Visual Studio C++ Build Tools（MSVC 和 Windows SDK）。

```sh
cargo run --release
cargo test --locked
cargo build --release --locked
```

可执行文件：`gpui/target/release/skill-studio-gpui.exe`。

## 功能

- 打开本地工作区，扫描 Skill、Markdown、脚本及明确的本地引用；显示标题、摘要和章节。
- 文件搜索、直接引用 / 递归引用 / 全部文件、聚焦当前节点、失效引用开关。
- 原生图谱画布、引用线及行号标签、节点拖动、缩放、滚动、缩略图和分层自动布局。
- 点击节点打开文档；点击引用行号标签打开来源并定位引用行。
- 主 Skill、关联 Skill、内部子流程、参考文档、脚本等自动分类；点击编辑面板的节点类型可循环切换及恢复自动分类。
- 布局和节点分类保存在系统用户配置目录，不改写源文件；支持重置、导入及导出 JSON。
- Graphite 深色原生界面、GPUI Base 可拖动分栏、统一按钮/输入框状态、居中弹窗和可键盘操作的命令面板；Esc 关闭浮层。
- 与 Herdr 对齐的字体与设计令牌：内嵌 Geist（UI 13px）和 JetBrains Mono（编辑器默认 14px），使用相同的 foreground/muted/dim、canvas/chrome/popover 派生规则；字体许可证见 `THIRD-PARTY-NOTICES.md`。
- Settings 提供 Graphite / Ink / Moss / Paper 主题、六种强调色及编辑字号、换行、行号设置，实时生效并保存在用户配置目录的 `skill-studio/settings.json`。
- 应用界面、菜单、提示和错误信息统一使用英文；工作区文件内容和标题保持原文。
- 内嵌 SVG 线性图标：刷新、聚焦、失效引用、布局、缩放、保存、菜单与命令面板采用图标按钮，悬停显示操作说明及快捷键；文件列表和命令菜单也使用统一图标。
- 多行编辑、语法高亮、文档内查找/替换（Ctrl/Cmd+F）、行列与编码信息，编辑器基础撤销 / 选择 / 复制粘贴能力由 GPUI Base 提供。编辑区和行号栏使用统一深色主题；节点类型与文档操作通过独立菜单选择。
- 安全保存：外部修改检查、同目录临时文件替换、保留权限；未保存时阻止切换和关闭窗口，可确认放弃修改。
- 自动监听文件变化并批量刷新，外部修改当前文档时提示而不覆盖编辑内容；可手动刷新和重新加载。
- 创建 Skill 目录和模板；名称仅允许小写字母、数字及连字符。
- 可搜索命令面板：Ctrl/Cmd+Shift+P；保存文档：Ctrl/Cmd+S。

工作区为文件访问边界；不执行脚本、不上传文件、不调用模型。面向可信的本地目录，不能作为对抗恶意并发文件替换的安全沙箱。最大编辑文件 2MB。

## 打包与迁移

`.github/workflows/gpui-build.yml` 构建 Windows EXE ZIP 和 macOS .app ZIP，产物未签名。macOS 流程暂未验证。

旧 Tauri/React 源码和构建配置已删除，删除前的源码备份保存在项目同级的 `skill-studio-tauri-backup-*.zip`（包含当时未提交的修改）。历史 WebView 布局不能直接读取；可从备份启动原版导出 JSON 后导入原生版。负坐标布局会整体平移到可见区域。

Windows 已验证编译、单元测试、启动示例工作区、点击打开文档和命令面板快捷键，并通过截图检查深色编辑器及弹窗。图谱拖动、中文输入、文件对话框和全部菜单流程仍需进一步交互验收。可用 `cargo run --release -- --workspace <目录>` 直接打开工作区。

## 结构

- `gpui/src/main.rs`：应用入口、工作区状态和操作。
- `gpui/src/ui/`：原生工作台、图谱视图、统一控件、主题和设置面板。
- `gpui/src/settings.rs`：主题与编辑器偏好的验证和原子持久化。
- `gpui/src/core.rs`：扫描、引用解析和安全文件操作。
- `gpui/src/filter.rs`、`role.rs`、`layout.rs`：图谱筛选、节点分类、布局持久化。
- `gpui/src/watch.rs`、`highlight.rs`、`commands.rs`：文件监听、语法高亮和命令定义。
- `examples/workspace/`：演示工作区。
