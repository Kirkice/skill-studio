use super::{controls::*, theme::rgb, theme::*};
use crate::{Studio, commands::Command};
use gpui::*;

const LABELS: [&str; 5] = ["File", "Edit", "View", "Debug", "Help"];
const WIDTHS: [f32; 5] = [34., 36., 40., 52., 40.];
const START: f32 = 36.;
pub(super) const TITLE_HEIGHT: f32 = 44.;
const HELP: usize = 4;
const LANGUAGE: usize = 5;
#[derive(Clone, Copy)]
enum Item {
    Command(Command),
    Exit,
    Close,
    Undo,
    Redo,
    Copy,
    Paste,
    Find,
    Replace,
    Palette,
    Explorer,
    Problems,
    Terminal,
    Preview,
    Flow,
    References,
    Git,
    Refresh,
    Start,
    Stop,
    Return,
    Trace,
    Guide,
    Keys,
    About,
    LanguageMenu,
    HelpMenu,
    Language(crate::i18n::Language),
}
impl Studio {
    pub(super) fn title_menus(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_none()
            .children(LABELS.iter().enumerate().map(|(i, label)| {
                let active =
                    self.title_menu == Some(i) || (i == HELP && self.title_menu == Some(LANGUAGE));
                button(("menu-title", i), *label, active, true)
                    .w(px(WIDTHS[i]))
                    .h(px(28.))
                    .rounded(px(4.))
                    .px(px(0.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.title_menu = if this.title_menu == Some(i)
                            || (i == HELP && this.title_menu == Some(LANGUAGE))
                        {
                            None
                        } else {
                            Some(i)
                        };
                        cx.notify();
                    }))
            }))
    }
    pub(super) fn title_menu_popup(&self, w: &Window, cx: &mut Context<Self>) -> Option<Div> {
        use Item::*;
        let index = self.title_menu?;
        let has = self.graph.is_some();
        let doc = self.selected.is_some();
        let text = self.active_text();
        let editing = text && !self.is_preview();
        let debug = self.pi_trace.read(cx).active();
        let m = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        let mut rows = div()
            .w(px(264.))
            .p(px(5.))
            .bg(rgb(POPOVER))
            .rounded(px(7.))
            .border_1()
            .border_color(rgb(BORDER))
            .shadow_lg()
            .occlude();
        let items: Vec<(&str, String, bool, bool, Item)> = match index {
            0 => vec![
                (
                    "打开工作区…",
                    "".into(),
                    true,
                    false,
                    Command(crate::commands::Command::Open),
                ),
                (
                    "新建 Skill…",
                    "".into(),
                    has,
                    false,
                    Command(crate::commands::Command::Create),
                ),
                (
                    "保存",
                    format!("{m}+S"),
                    self.dirty(cx) && !self.saving,
                    false,
                    Command(crate::commands::Command::Save),
                ),
                ("关闭标签", format!("{m}+W"), doc, false, Close),
                ("退出", "".into(), true, false, Exit),
            ],
            1 => vec![
                ("撤销", format!("{m}+Z"), editing, false, Undo),
                (
                    "重做",
                    if cfg!(target_os = "macos") {
                        "Cmd+Shift+Z"
                    } else {
                        "Ctrl+Y"
                    }
                    .into(),
                    editing,
                    false,
                    Redo,
                ),
                ("复制", format!("{m}+C"), true, false, Copy),
                ("粘贴", format!("{m}+V"), true, false, Paste),
                ("查找", format!("{m}+F"), text, false, Find),
                ("替换", "".into(), text, false, Replace),
            ],
            2 => vec![
                ("命令面板", format!("{m}+Shift+P"), true, false, Palette),
                ("Explorer", "".into(), true, self.explorer_visible, Explorer),
                ("Problems", "".into(), true, self.problems_visible, Problems),
                (
                    "终端",
                    "Ctrl+`".into(),
                    true,
                    self.terminal_visible,
                    Terminal,
                ),
                (
                    "Markdown 预览",
                    format!("{m}+Shift+V"),
                    self.selected
                        .as_deref()
                        .is_some_and(crate::preview::supported),
                    self.is_preview(),
                    Preview,
                ),
                ("流程图", "".into(), has, self.flow_mode, Flow),
                ("引用图", "".into(), has, !self.flow_mode, References),
                ("Git Diff", "".into(), has, self.git.enabled, Git),
                ("刷新图谱", "".into(), has, false, Refresh),
                (
                    "保存布局",
                    "".into(),
                    has,
                    false,
                    Command(crate::commands::Command::SaveLayout),
                ),
                (
                    "导入布局…",
                    "".into(),
                    has,
                    false,
                    Command(crate::commands::Command::Import),
                ),
                (
                    "导出布局…",
                    "".into(),
                    has,
                    false,
                    Command(crate::commands::Command::Export),
                ),
                (
                    "重置布局",
                    "".into(),
                    has,
                    false,
                    Command(crate::commands::Command::Reset),
                ),
                (
                    "设置",
                    "".into(),
                    true,
                    false,
                    Command(crate::commands::Command::Settings),
                ),
            ],
            3 => vec![
                ("Start Debug", "".into(), has && !debug, false, Start),
                ("Stop Debug", "".into(), debug, false, Stop),
                ("返回调试终端", "".into(), debug, false, Return),
                (
                    "Open Pi Trace",
                    "".into(),
                    debug || self.pi_trace_visible,
                    false,
                    Trace,
                ),
            ],
            LANGUAGE => vec![
                ("Help", "‹".into(), true, false, HelpMenu),
                (
                    "English",
                    String::new(),
                    true,
                    self.preferences.language == crate::i18n::Language::En,
                    Language(crate::i18n::Language::En),
                ),
                (
                    "简体中文",
                    String::new(),
                    true,
                    self.preferences.language == crate::i18n::Language::ZhCn,
                    Language(crate::i18n::Language::ZhCn),
                ),
            ],
            _ => vec![
                ("使用指南", "".into(), true, false, Guide),
                ("快捷键", "".into(), true, false, Keys),
                ("Language", "›".into(), true, false, LanguageMenu),
                ("关于 Skill Studio", "".into(), true, false, About),
            ],
        };
        let left = (START + WIDTHS[..index.min(HELP)].iter().sum::<f32>())
            .min((f32::from(w.viewport_size().width) - 270.).max(0.));
        for (i, (label, key, enabled, checked, item)) in items.into_iter().enumerate() {
            if index == 2 && matches!(i, 5 | 9 | 13) {
                rows = rows.child(div().h(px(1.)).my(px(3.)).bg(rgb(BORDER)));
            }
            rows = rows.child(
                div()
                    .id(("title-menu-item", i))
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .text_color(rgb(if enabled { TEXT } else { DIM }))
                    .hover(move |s| s.bg(rgb(if enabled { HOVER } else { POPOVER })))
                    .child(
                        div()
                            .w(px(14.))
                            .child(if enabled && checked { "✓" } else { "" }),
                    )
                    .child(div().flex_1().child(crate::i18n::text(label)))
                    .child(div().text_color(rgb(DIM)).child(key))
                    .on_click(cx.listener(move |this, _, w, cx| {
                        if enabled {
                            this.title_menu = None;
                            this.menu_item(item, w, cx);
                            cx.notify();
                        }
                    })),
            );
        }
        Some(
            div()
                .absolute()
                .inset_0()
                .child(
                    div()
                        .id("title-menu-backdrop")
                        .absolute()
                        .top(px(TITLE_HEIGHT))
                        .bottom(px(0.))
                        .left(px(0.))
                        .right(px(0.))
                        .occlude()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.title_menu = None;
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _, _, cx| {
                                this.title_menu = None;
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        ),
                )
                .child(
                    div()
                        .id("title-menu-popup")
                        .absolute()
                        .left(px(left))
                        .top(px(TITLE_HEIGHT - 1.))
                        .max_h(px((f32::from(w.viewport_size().height) - 48.).max(40.)))
                        .overflow_y_scroll()
                        .child(rows)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                ),
        )
    }
    fn menu_item(&mut self, item: Item, w: &mut Window, cx: &mut Context<Self>) {
        use gpui_base::input;
        match item {
            Item::Command(command) => self.command(command, w, cx),
            Item::Exit => {
                if self.request_close(w, cx) {
                    w.remove_window();
                }
            }
            Item::Close => {
                if let Some(id) = self.selected.clone() {
                    self.close_tab(id, w, cx);
                }
            }
            Item::Undo | Item::Redo | Item::Copy | Item::Paste => {
                let action: Box<dyn Action> = match item {
                    Item::Undo => {
                        self.editor.update(cx, |e, cx| e.focus(w, cx));
                        Box::new(input::Undo)
                    }
                    Item::Redo => {
                        self.editor.update(cx, |e, cx| e.focus(w, cx));
                        Box::new(input::Redo)
                    }
                    Item::Copy if self.explorer_focus.is_focused(w) => {
                        Box::new(super::explorer::CopyEntry)
                    }
                    Item::Paste if self.explorer_focus.is_focused(w) => {
                        Box::new(super::explorer::PasteEntry)
                    }
                    Item::Copy if self.terminal.read(cx).focus.is_focused(w) => {
                        Box::new(super::terminal::CopyTerminal)
                    }
                    Item::Paste if self.terminal.read(cx).focus.is_focused(w) => {
                        Box::new(super::terminal::PasteTerminal)
                    }
                    Item::Copy => Box::new(input::Copy),
                    _ => Box::new(input::Paste),
                };
                w.defer(cx, move |w, cx| w.dispatch_action(action, cx));
            }
            Item::Find => self.open_find(w, cx),
            Item::Replace => {
                self.open_find(w, cx);
                self.replace_query
                    .update(cx, |input, cx| input.focus(w, cx));
            }
            Item::Palette => self.toggle_palette(w, cx),
            Item::Explorer => self.toggle_explorer(w, cx),
            Item::Problems => self.toggle_problems(w, cx),
            Item::Terminal => self.toggle_terminal(w, cx),
            Item::Preview => self.toggle_preview(w, cx),
            Item::Flow => self.flow_mode = true,
            Item::References => self.flow_mode = false,
            Item::Git => self.git.enabled = !self.git.enabled,
            Item::Refresh => self.refresh_graph(w, cx),
            Item::Start => self.start_pi_debug(w, cx),
            Item::Stop => self.pi_trace.update(cx, |t, cx| t.stop(cx)),
            Item::Return => self.show_terminal(w, cx),
            Item::Trace => self.pi_trace.update(cx, |t, cx| t.show_details(cx)),
            Item::Guide => cx.open_url("https://github.com/Kirkice/skill-studio#readme"),
            Item::Keys => {
                let _=w.prompt(PromptLevel::Info,"快捷键",Some("Ctrl/Cmd+Shift+P · 命令面板\nCtrl/Cmd+S · 保存\nCtrl/Cmd+F · 查找\nCtrl/Cmd+Shift+V · Markdown 预览\nCtrl+` · 终端\nF2 / Delete · 文件树重命名 / 删除"),&[PromptButton::ok("关闭")],cx);
            }
            Item::LanguageMenu => self.title_menu = Some(LANGUAGE),
            Item::HelpMenu => self.title_menu = Some(HELP),
            Item::Language(language) => {
                self.preferences.language = language;
                crate::i18n::set(language);
                self.apply_preferences(w, cx);
            }
            Item::About => {
                let description = format!(
                    "{} {}\n{}: {}\n\n{}",
                    crate::i18n::text("Version"),
                    env!("CARGO_PKG_VERSION"),
                    crate::i18n::text("Author"),
                    env!("CARGO_PKG_AUTHORS"),
                    crate::i18n::text("A local workspace for authoring, inspecting and debugging Skills."),
                );
                let _ = w.prompt(
                    PromptLevel::Info,
                    "Skill Studio",
                    Some(&description),
                    &[PromptButton::ok("关闭")],
                    cx,
                );
            }
        }
    }
}
