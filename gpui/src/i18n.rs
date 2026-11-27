use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    En,
    ZhCn,
}
thread_local! { static CURRENT: Cell<Language> = const { Cell::new(Language::En) }; }
pub fn set(language: Language) {
    CURRENT.set(language);
}
pub fn text(s: &str) -> &str {
    for &(english, chinese) in STRINGS {
        if s == english || s == chinese {
            return if CURRENT.get() == Language::ZhCn {
                chinese
            } else {
                english
            };
        }
    }
    s
}
const STRINGS: &[(&str, &str)] = &[
    ("Author", "作者"),
    ("Version", "版本"),
    (
        "A local workspace for authoring, inspecting and debugging Skills.",
        "用于编写、检查和调试 Skill 的本地工作台。",
    ),
    (
        "Ctrl/Cmd+Shift+P · Command Palette\nCtrl/Cmd+S · Save\nCtrl/Cmd+F · Find\nCtrl/Cmd+Shift+V · Markdown Preview\nCtrl+` · Terminal\nF2 / Delete · Rename / Delete",
        "Ctrl/Cmd+Shift+P · 命令面板\nCtrl/Cmd+S · 保存\nCtrl/Cmd+F · 查找\nCtrl/Cmd+Shift+V · Markdown 预览\nCtrl+` · 终端\nF2 / Delete · 文件树重命名 / 删除",
    ),
    ("File", "文件"),
    ("Edit", "编辑"),
    ("View", "视图"),
    ("Debug", "调试"),
    ("Help", "帮助"),
    ("Language", "语言"),
    ("Open workspace…", "打开工作区…"),
    ("New Skill…", "新建 Skill…"),
    ("Save", "保存"),
    ("Close tab", "关闭标签"),
    ("Exit", "退出"),
    ("Undo", "撤销"),
    ("Redo", "重做"),
    ("Copy", "复制"),
    ("Paste", "粘贴"),
    ("Find", "查找"),
    ("Replace", "替换"),
    ("Command Palette", "命令面板"),
    ("Explorer", "文件资源管理器"),
    ("Problems", "问题"),
    ("Terminal", "终端"),
    ("Markdown Preview", "Markdown 预览"),
    ("Skill Flow", "流程图"),
    ("Reference Graph", "引用图"),
    ("Refresh Graph", "刷新图谱"),
    ("Save layout", "保存布局"),
    ("Import layout…", "导入布局…"),
    ("Export layout…", "导出布局…"),
    ("Reset layout", "重置布局"),
    ("Settings", "设置"),
    ("Start Debug", "启动调试"),
    ("Stop Debug", "停止调试"),
    ("Return to Debug terminal", "返回调试终端"),
    ("Open Pi Trace", "打开 Pi 追踪"),
    ("User Guide", "使用指南"),
    ("Keyboard Shortcuts", "快捷键"),
    ("About Skill Studio", "关于 Skill Studio"),
    ("Close", "关闭"),
    ("Open workspace", "打开工作区"),
    ("New Skill", "新建 Skill"),
    ("Save document", "保存文档"),
    ("Refresh workspace", "刷新工作区"),
    ("Close command palette", "关闭命令面板"),
    ("Import layout", "导入布局"),
    ("Export layout", "导出布局"),
    ("EXPLORER", "文件资源管理器"),
    ("FILES", "文件"),
    ("WORKSPACE ROOT", "工作区根目录"),
    ("LOCAL WORKSPACE", "本地工作区"),
    ("SINGLE-FILE SCOPE", "单文件范围"),
    ("Local workspace", "本地工作区"),
    ("Rename", "重命名"),
    ("Delete", "删除"),
    ("Rename file or folder", "重命名文件或文件夹"),
    ("Permanently delete this item?", "永久删除此项目？"),
    ("Cancel", "取消"),
    ("Don't Save", "不保存"),
    ("Save All", "全部保存"),
    ("Unsaved changes", "未保存的修改"),
    ("Document", "文档"),
    ("Document actions", "文档操作"),
    ("Select a file", "请选择文件"),
    (
        "Open a document from the graph or workspace",
        "从图谱或工作区打开文档",
    ),
    ("Edit source", "编辑源码"),
    ("Markdown preview · read only", "Markdown 预览 · 只读"),
    ("Rendered Markdown", "Markdown 渲染预览"),
    ("No matching files", "没有匹配的文件"),
    (
        "Open a folder to explore its files and Skills.",
        "打开文件夹以浏览文件和 Skill。",
    ),
    ("No matching commands", "没有匹配的命令"),
    ("Find in document…", "在文档中查找…"),
    ("Replace with…", "替换为…"),
    ("Search files or Skills…", "搜索文件或 Skill…"),
    ("Type a command…", "输入命令…"),
    ("Reference graph", "引用图"),
    ("Skill orchestration", "Skill 编排"),
    ("Document stages · candidate", "文档阶段 · 候选"),
    ("Start with a workspace", "打开工作区开始"),
    (
        "Explore references between Skills, documents and scripts.",
        "查看 Skill、文档与脚本之间的引用。",
    ),
    (
        "No SKILL.md entry found. Switch to References or open a Skill workspace.",
        "未找到 SKILL.md 入口。请切换到引用图或打开 Skill 工作区。",
    ),
    ("Select reference location", "选择引用位置"),
    ("OVERVIEW", "概览"),
    ("MAIN SKILL", "主 Skill"),
    ("SUBFLOW ROOT", "子流程入口"),
    ("SUBFLOW", "子流程"),
    ("PHASE / STEP", "阶段 / 步骤"),
    ("● RUNNING", "● 运行中"),
    ("SKIPPED", "已跳过"),
    ("Time", "耗时"),
    ("Tokens", "令牌"),
    ("CTX", "上下文"),
    ("No problems found", "未发现问题"),
    (
        "No problems match the active filters",
        "没有符合当前筛选条件的问题",
    ),
    ("Open a workspace to run checks", "请打开工作区以运行检查"),
    ("Run workspace checks", "运行工作区检查"),
    ("Hide Problems", "隐藏问题"),
    ("Errors", "错误"),
    ("Warnings", "警告"),
    ("Info", "提示"),
    ("Workspace", "工作区"),
    ("APPEARANCE", "外观"),
    ("EDITOR", "编辑器"),
    ("PI TRACE", "PI 追踪"),
    ("Pi Trace", "Pi 追踪"),
    ("Theme", "主题"),
    ("Accent color", "强调色"),
    ("Font size", "字号"),
    ("Word wrap", "自动换行"),
    ("Line numbers", "行号"),
    (
        "Changes apply immediately and are saved on this device.",
        "修改立即生效，并保存在本机。",
    ),
    ("No workspace files are modified", "不会修改工作区文件"),
    ("Run telemetry", "运行指标"),
    ("No tool calls recorded yet.", "尚无工具调用记录。"),
    ("Pi Trace is unavailable", "Pi 追踪不可用"),
    ("Pi terminal connected", "Pi 终端已连接"),
    ("Pi running", "Pi 正在运行"),
    ("Run finished · listening", "运行结束 · 正在监听"),
    ("Waiting for Pi", "等待 Pi 连接"),
    ("Debug off", "调试未开启"),
    ("TERMINAL", "终端"),
    ("Ready", "就绪"),
    ("Saved", "已保存"),
    ("Saving…", "正在保存…"),
    (
        "Release to open the file or folder",
        "松开鼠标以打开文件或文件夹",
    ),
    (
        "Folders open as workspaces · Files open by type",
        "文件夹打开工作区 · 文件按内容类型打开",
    ),
    (
        "Open only · No copying or execution",
        "只打开 · 不复制 · 不自动执行",
    ),
    (
        "Candidate flow: stages follow document order; citation edges do not prove execution, conditions, or returns.",
        "候选流程：按文档阶段顺序排列；调用边来自引用，不代表已确认的执行、条件或返回关系。",
    ),
    ("Show in File Explorer", "在资源管理器中显示"),
    ("Show in Finder", "在 Finder 中显示"),
    ("Hide Explorer", "收起文件资源管理器"),
    ("Show Explorer", "显示文件资源管理器"),
    ("Show / hide Problems", "显示 / 隐藏问题"),
    ("Show / hide Terminal", "显示 / 隐藏终端"),
    (
        "Refresh workspace index and clear Debug highlights",
        "刷新工作区索引并清除调试高亮",
    ),
    ("Open Pi Trace details", "打开 Pi 追踪详情"),
    ("Copy Pi command", "复制 Pi 启动命令"),
    (
        "Expand Terminal to the Document area",
        "放大到整个 Document 区域",
    ),
    ("Restore Document and Terminal split", "还原文档/终端分栏"),
    (
        "Hide Terminal without stopping its session",
        "收起终端，进程继续运行",
    ),
    ("Close Terminal and stop its session", "关闭终端并结束会话"),
    ("Terminal font size", "终端字号"),
    ("FILE PREVIEW", "文件预览"),
    ("Read-only · first frame", "只读 · 首帧"),
    (
        "Source unchanged · No transcoding or execution",
        "源文件未修改 · 未自动转码 · 不会自动执行",
    ),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locale_roundtrip() {
        set(Language::En);
        assert_eq!(text("文件"), "File");
        set(Language::ZhCn);
        assert_eq!(text("File"), "文件");
        assert_eq!(text("example.md"), "example.md");
    }
}
