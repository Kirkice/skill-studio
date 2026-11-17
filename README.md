# SkillsRider

本地优先的 Skill 引用图谱与源文件编辑器。Tauri 2 / Rust / React / TypeScript。

## 开发

需要 Node.js 22+、Rust stable 和 Tauri 系统依赖。

- Windows：Visual Studio C++ Build Tools（MSVC + Windows SDK）、WebView2。
- macOS：Xcode Command Line Tools。

```sh
npm ci
npm run tauri dev
```

仅运行 `npm run dev` 可以查看前端，但工作区功能需要 Tauri 桌面宿主。

## 打包

```sh
npm run desktop:build
```

默认使用 Release 构建，Windows 下不显示调试控制台。仅构建可直接运行的程序（不生成安装包）：

```sh
npm run desktop:exe
```

Windows EXE 位于 `src-tauri/target/release/skills-rider.exe`，仍需系统安装 WebView2。

安装包产物位于 `src-tauri/target/release/bundle/`。Windows 和 macOS 分别在对应系统构建；macOS 支持 Intel / Apple Silicon。正式分发需要自行配置代码签名和 macOS 公证。GitHub Actions 提供未签名的双平台构建。

## 当前 MVP

- 打开任意本地目录，递归扫描 SKILL.md、Markdown 和常见脚本。
- 文件图谱、搜索、聚焦当前文件及其直接引用 / 被引用文件。
- 默认显示 Skill 直接引用，可切换递归引用或全部文件；失效引用默认隐藏。
- 节点显示原文标题、摘要、前三个章节及引用数量，中文内容保持原文。
- 主 Skill、关联 Skill、内部子流程、参考文档、脚本采用不同节点样式；子流程自动识别标为推定，右侧可手动校正，随布局保存。设计规则见 [NODE-DESIGN.md](NODE-DESIGN.md)。
- 拖动状态独立于编辑器和文件树，松开鼠标后才提交布局变更。
- 节点可拖动，点击「保存布局」后按工作区路径保存在应用本地存储，下次打开恢复；「重置布局」后需再次保存。不修改 Markdown，布局不随工作区跨设备同步。
- 识别 Markdown 链接、图片链接、独立行内代码形式的文件路径。
- 引用边携带源文件行号；点击边打开源文件，状态栏显示行号。
- 失效引用标红；忽略远程 URL、锚点和工作区外的已存在文件。
- 直接编辑文本文件；保存时检查外部修改，使用同目录临时文件替换。
- 新建 Skill 目录和标准 SKILL.md 模板。
- 不执行脚本，不上传文件，不调用模型。

打开 `examples/workspace` 可查看示例。

## 范围与限制

这不是执行流程图：边表示文本中的文件引用，不代表运行时调用。

目前仅支持手动刷新（保存和创建后自动刷新）。尚未实现自然语言 / CLI / MCP 工具提取、文件自动监听、重命名联动、删除、拖线写回和 Markdown 渲染预览。代码块中的命令不会被当作确定引用；行内文件路径仍可能是示例，应结合原文判断。尚未处理 URL 编码文件名。

工作区为扫描边界；不跟随目录符号链接；文件读写进行 canonicalize 边界检查。面向可信的本地工作区，不应把它当成针对恶意并发文件替换的安全沙箱。最大可编辑文件 2MB；扫描超大目录仍需后续优化。新建 Skill 名称限制为小写字母、数字及连字符。

## 验证

```sh
npm run build
cd src-tauri
cargo test
cargo check
```

## 结构

- `src/`：三栏桌面 UI、React Flow 图谱、CodeMirror 编辑器。
- `src-tauri/src/main.rs`：工作区状态、扫描、引用解析与文件操作。
- `examples/workspace/`：演示数据。

后续可把扫描与解析拆为独立 Rust crate，以复用到 CLI / 编辑器插件。
