#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
mod core;
mod watch;
mod layout;
use gpui::*;
use gpui_base::{Editor, InputBase};
use gpui_base::input::{EditorState, InputEditorStyle};
actions!(studio, [TogglePalette, SaveDocument]);

struct Studio { graph: Option<core::Graph>, selected: Option<String>, editor: Entity<EditorState>, original: String, status: String, conflict: bool, generation: u64, watcher: Option<notify::RecommendedWatcher>, positions: layout::Positions, drag: Option<(String, Point<Pixels>, layout::Position)>, zoom: f32, palette: bool }
impl Studio {
    fn dirty(&self, cx: &App) -> bool { self.selected.is_some() && self.editor.read(cx).value().as_ref() != self.original }
    fn guard(&mut self, cx: &mut Context<Self>) -> bool {
        if self.dirty(cx) { self.status = "文件有未保存修改，请先保存或点击放弃修改".into(); cx.notify(); false } else { true }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        let (Some(graph), Some(id)) = (&self.graph, &self.selected) else { return };
        let content = self.editor.read(cx).value().to_string();
        match core::save_document(std::path::Path::new(&graph.root), id, &content, &self.original) {
            Ok(()) => {
                self.generation += 1;
                self.original = content;
                self.conflict = false;
                match core::scan_workspace(std::path::Path::new(&graph.root)) {
                    Ok(updated) => { self.graph = Some(updated); self.status = "已保存".into(); }
                    Err(e) => self.status = format!("已保存，但索引刷新失败：{e}"),
                }
            }
            Err(e) => self.status = format!("保存失败：{e}"),
        }
        cx.notify();
    }
    fn save_layout(&mut self, cx: &mut Context<Self>) {
        if let Some(graph) = &self.graph {
            self.status = match layout::save(&graph.root, &self.positions) { Ok(()) => "布局已保存".into(), Err(e) => format!("布局保存失败：{e}") };
            cx.notify();
        }
    }
    fn reset_layout(&mut self, cx: &mut Context<Self>) {
        self.positions.clear();
        self.save_layout(cx);
    }
    fn start_watcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.watcher = None;
        let Some(root) = self.graph.as_ref().map(|g| g.root.clone()) else { return };
        let (watcher, events) = match watch::start(std::path::Path::new(&root)) {
            Ok(value) => value,
            Err(e) => { self.status = format!("文件监听不可用，请手动刷新：{e}"); cx.notify(); return; }
        };
        self.watcher = Some(watcher);
        cx.spawn_in(window, async move |this, cx| {
            while events.recv().await.is_ok() {
                let snapshot = this.update_in(cx, |studio, _, _| {
                    if studio.graph.as_ref().map(|g| &g.root) != Some(&root) { return None; }
                    studio.generation += 1;
                    Some((studio.generation, studio.selected.clone(), studio.original.clone()))
                });
                let Ok(Some((generation, selected, original))) = snapshot else { break };
                let scan_root = root.clone();
                let scan_selected = selected.clone();
                let result = cx.background_executor().spawn(async move {
                    let path = std::path::Path::new(&scan_root);
                    let graph = core::scan_workspace(path);
                    let document = scan_selected.map(|id| core::read_document(path, &id));
                    (graph, document)
                }).await;
                let applied = this.update_in(cx, |studio, _, cx| {
                    if studio.generation != generation || studio.graph.as_ref().map(|g| &g.root) != Some(&root) { return; }
                    match result.0 {
                        Ok(graph) => studio.graph = Some(graph),
                        Err(e) => { studio.status = format!("自动刷新失败：{e}"); cx.notify(); return; }
                    }
                    if studio.selected == selected && studio.original == original {
                        studio.conflict = result.1.is_some_and(|doc| doc.map_or(true, |doc| doc.content != original));
                    }
                    if studio.conflict { studio.status = "当前文件已被外部修改或删除；编辑内容未覆盖".into(); }
                    cx.notify();
                });
                if applied.is_err() { break; }
            }
        }).detach();
    }
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.graph.as_ref().map(|g| g.root.clone()) else { return };
        self.generation += 1;
        let generation = self.generation;
        let selected = self.selected.clone();
        let original = self.original.clone();
        cx.spawn_in(window, async move |this, cx| {
            let scan_root = root.clone();
            let scan_selected = selected.clone();
            let (graph, document) = cx.background_executor().spawn(async move {
                let path = std::path::Path::new(&scan_root);
                (core::scan_workspace(path), scan_selected.map(|id| core::read_document(path, &id)))
            }).await;
            let _ = this.update_in(cx, |studio, _, cx| {
                if studio.generation != generation || studio.graph.as_ref().map(|g| &g.root) != Some(&root) { return; }
                match graph {
                    Ok(graph) => { studio.graph = Some(graph); studio.status = "索引已刷新".into(); }
                    Err(e) => { studio.status = format!("刷新失败：{e}"); cx.notify(); return; }
                }
                if studio.selected == selected && studio.original == original {
                    studio.conflict = document.is_some_and(|doc| doc.map_or(true, |doc| doc.content != original));
                }
                if studio.conflict { studio.status = "当前文件已被外部修改或删除；编辑内容未覆盖".into(); }
                cx.notify();
            });
        }).detach();
    }
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.guard(cx) { return; }
        let (Some(graph), Some(id)) = (&self.graph, &self.selected) else { return };
        match core::read_document(std::path::Path::new(&graph.root), id) {
            Ok(doc) => { self.original = doc.content.clone(); self.editor.update(cx, |editor, cx| editor.set_value(doc.content, window, cx)); self.conflict = false; self.status = "已从磁盘重新加载".into(); }
            Err(e) => self.status = format!("重新加载失败：{e}"),
        }
        cx.notify();
    }
    fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dirty(cx) { return; }
        let selected = self.selected.clone();
        let before = self.editor.read(cx).value();
        let answer = window.prompt(PromptLevel::Warning, "放弃未保存的修改？", Some("此操作会恢复到上次打开或保存时的内容。"), &[PromptButton::ok("放弃修改"), PromptButton::cancel("取消")], cx);
        cx.spawn_in(window, async move |this, cx| {
            if answer.await.ok() != Some(0) { return; }
            let _ = this.update_in(cx, |this, window, cx| {
                if this.selected != selected || this.editor.read(cx).value() != before { this.status = "文档已变化，未放弃修改".into(); cx.notify(); return; }
                let value = this.original.clone();
                this.editor.update(cx, |editor, cx| editor.set_value(value, window, cx));
                this.status = "已放弃本次修改".into();
                cx.notify();
            });
        }).detach();
    }
}

impl Studio {
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.guard(cx) { return; }
        let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("打开 Skill 工作区".into()) });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let result = cx.background_executor().spawn(async move { core::open_workspace(&path) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(graph) => { if !this.guard(cx) { return; } this.generation += 1; this.status = format!("{} 个文件 · {} 条引用", graph.nodes.len(), graph.edges.len()); let loaded = layout::load(&graph.root); this.positions = loaded.unwrap_or_else(|e| { this.status = format!("布局加载失败：{e}"); layout::Positions::new() }); this.zoom = 1.; this.graph = Some(graph); this.selected = None; this.original.clear(); this.conflict = false; this.start_watcher(window, cx); }
                    Err(e) => this.status = e,
                }
                cx.notify();
            });
        }).detach();
    }
    fn select(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.as_deref() == Some(id.as_str()) { return; }
        if !self.guard(cx) { return; }
        let Some(graph) = &self.graph else { return };
        match core::read_document(std::path::Path::new(&graph.root), &id) {
            Ok(doc) => { self.selected = Some(id); self.original = doc.content.clone(); self.conflict = false; self.editor.update(cx, |editor, cx| editor.set_value(doc.content, window, cx)); }
            Err(e) => self.status = e,
        }
        cx.notify();
    }
}
impl Render for Studio {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dirty = self.dirty(cx);
        let mut files = div().id("files").w(px(300.)).h_full().overflow_y_scroll().flex().flex_col().gap(px(6.)).p(px(12.)).bg(rgb(0x111827));
        if let Some(graph) = &self.graph {
            for (i, node) in graph.nodes.iter().enumerate().filter(|(_,n)| n.kind != "missing") {
                let id = node.id.clone();
                let title = if node.title.is_empty() { node.label.clone() } else { node.title.clone() };
                files = files.child(div().id(("file", i)).p(px(9.)).rounded(px(6.)).bg(rgb(0x202b40)).cursor_pointer()
                    .child(title).child(div().text_size(px(11.)).text_color(rgb(0x93a4bc)).child(node.id.clone()))
                    .on_click(cx.listener(move |this, _, window, cx| this.select(id.clone(), window, cx))));
            }
        }
        let mut graph_panel = div().flex_1().min_w(px(0.)).h_full().flex().flex_col();
        if let Some(graph) = &self.graph {
            let zoom = self.zoom;
            let positions: layout::Positions = graph.nodes.iter().enumerate().map(|(i,n)| (n.id.clone(), self.positions.get(&n.id).copied().unwrap_or_else(|| layout::default_position(i)))).collect();
            let width = positions.values().map(|p| p.x + 340.).fold(1000_f32, f32::max) * zoom;
            let height = positions.values().map(|p| p.y + 180.).fold(800_f32, f32::max) * zoom;
            let edges: Vec<_> = graph.edges.iter().filter_map(|e| Some((*positions.get(&e.source)?, *positions.get(&e.target)?, e.missing))).collect();
            let mut layer = div().relative().w(px(width)).h(px(height)).child(canvas(|_, _, _| (), move |bounds, _, window, _| {
                for (from, to, missing) in edges {
                    let start = bounds.origin + point(px((from.x + 240.) * zoom), px((from.y + 45.) * zoom));
                    let end = bounds.origin + point(px(to.x * zoom), px((to.y + 45.) * zoom));
                    let mut line = PathBuilder::stroke(px(1.5));
                    if missing { line = line.dash_array(&[px(5.), px(4.)]); }
                    line.move_to(start); line.line_to(end);
                    if let Ok(path) = line.build() { window.paint_path(path, if missing { rgb(0xfb7185) } else { rgb(0x7184a0) }); }
                }
            }).absolute().size_full());
            for (i,node) in graph.nodes.iter().enumerate() {
                let pos = positions[&node.id];
                let id = node.id.clone(); let select_id = id.clone();
                let title = if node.title.is_empty() { node.label.clone() } else { node.title.clone() };
                let kind = node.kind.clone(); let missing = kind == "missing";
                let bg = if self.selected.as_deref() == Some(&id) { 0x303557 } else { 0x1b273c };
                layer = layer.child(div().id(("node",i)).absolute().left(px(pos.x*zoom)).top(px(pos.y*zoom)).w(px(240.*zoom)).h(px(90.*zoom)).overflow_hidden().p(px(10.*zoom)).rounded(px(8.)).border_1().border_color(if missing { rgb(0xfb7185) } else if kind == "skill" { rgb(0xa78bfa) } else { rgb(0x52647c) }).bg(rgb(bg)).cursor_pointer()
                    .child(div().text_size(px(11.*zoom)).text_color(rgb(0xa78bfa)).child(kind))
                    .child(div().text_size(px(14.*zoom)).child(title))
                    .child(div().text_size(px(10.*zoom)).text_color(rgb(0x93a4bc)).child(node.id.clone()))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, e: &MouseDownEvent, _, cx| { this.drag = Some((id.clone(), e.position, pos)); cx.stop_propagation(); }))
                    .on_click(cx.listener(move |this, _, window, cx| { if !missing { this.select(select_id.clone(), window, cx); } })));
            }
            let controls = div().h(px(42.)).flex().items_center().gap(px(8.)).px(px(10.))
                .child(format!("引用图谱 · {} 节点 · {} 引用", graph.nodes.len(), graph.edges.len()))
                .child(div().id("zoom-out").cursor_pointer().child("－").on_click(cx.listener(|this, _, _, cx| { this.zoom = (this.zoom - 0.2).max(0.4); cx.notify(); })))
                .child(format!("{:.0}%", zoom * 100.))
                .child(div().id("zoom-in").cursor_pointer().child("＋").on_click(cx.listener(|this, _, _, cx| { this.zoom = (this.zoom + 0.2).min(2.); cx.notify(); })))
                .child(div().id("reset-layout").cursor_pointer().child("重置布局").on_click(cx.listener(|this, _, _, cx| this.reset_layout(cx))));
            graph_panel = graph_panel.child(controls).child(div().id("graph-scroll").relative().flex_1().min_h(px(0.)).overflow_scroll()
                .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                    if let Some((id, start, pos)) = &this.drag {
                        let x = (pos.x + f32::from(e.position.x - start.x) / this.zoom).clamp(0., 100000.);
                        let y = (pos.y + f32::from(e.position.y - start.y) / this.zoom).clamp(0., 100000.);
                        this.positions.insert(id.clone(), layout::Position { x, y }); cx.notify();
                    }
                }))
                .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| { if this.drag.take().is_some() { this.save_layout(cx); } }))
                .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| { if this.drag.take().is_some() { this.save_layout(cx); } }))
                .child(layer));
        } else { graph_panel = graph_panel.child(div().p(px(18.)).child("选择一个工作区以查看 Skill 和引用关系")); }
        let editor_state = self.editor.clone();
        let mut editor = div().w(px(420.)).h_full().flex().flex_col().gap(px(12.)).p(px(16.)).bg(rgb(0x111827))
            .child(div().text_size(px(15.)).child(self.selected.clone().unwrap_or_else(|| "文档编辑".into())));
        if self.selected.is_some() {
            editor = editor.child(div().flex().gap(px(8.))
                .child(div().id("save").p(px(8.)).rounded(px(6.)).bg(rgb(0x7654d8)).cursor_pointer().child(if dirty { "保存 · 未保存" } else { "保存" }).on_click(cx.listener(|this, _, _, cx| this.save(cx))))
                .child(div().id("discard").p(px(8.)).rounded(px(6.)).bg(rgb(0x29354c)).cursor_pointer().child("放弃修改").on_click(cx.listener(|this, _, window, cx| this.discard(window, cx)))));
            if self.conflict {
                editor = editor.child(div().p(px(8.)).bg(rgb(0x493325)).child("磁盘文件已变化；编辑内容未覆盖。先保存或放弃本地修改，再重新加载。")
                    .child(div().id("reload").p(px(6.)).cursor_pointer().child("重新加载磁盘文件").on_click(cx.listener(|this, _, window, cx| this.reload(window, cx)))));
            }
            editor = editor.child(InputBase::new("document-editor").flex_1().min_h(px(0.)).w_full().overflow_hidden().on_mouse_down(MouseButton::Left, move |_, window, cx| {
                editor_state.update(cx, |state, cx| state.focus(window, cx));
            }).child(div().size_full().child(Editor::new(&self.editor))));
        }
        let palette = if self.palette { Some(div().absolute().top(px(60.)).right(px(440.)).w(px(300.)).p(px(14.)).rounded(px(10.)).bg(rgb(0x29354c)).flex().flex_col().gap(px(8.))
            .child("命令面板")
            .child(div().id("cmd-open").cursor_pointer().p(px(6.)).child("打开工作区").on_click(cx.listener(|this, _, w, cx| { this.palette = false; this.open(w, cx); })))
            .child(div().id("cmd-refresh").cursor_pointer().p(px(6.)).child("刷新工作区").on_click(cx.listener(|this, _, w, cx| { this.palette = false; this.refresh(w, cx); })))
            .child(div().id("cmd-save").cursor_pointer().p(px(6.)).child("保存文档").on_click(cx.listener(|this, _, _, cx| { this.palette = false; this.save(cx); })))
            .child(div().id("cmd-layout").cursor_pointer().p(px(6.)).child("保存布局").on_click(cx.listener(|this, _, _, cx| { this.palette = false; this.save_layout(cx); })))
            .child(div().id("cmd-reset").cursor_pointer().p(px(6.)).child("重置布局").on_click(cx.listener(|this, _, _, cx| { this.palette = false; this.reset_layout(cx); })))
            .child(div().id("cmd-close").cursor_pointer().p(px(6.)).child("关闭").on_click(cx.listener(|this, _, _, cx| { this.palette = false; cx.notify(); })))) } else { None };
        div().relative().size_full().flex().flex_col().bg(rgb(0x0b1120)).text_color(rgb(0xdce4f3))
            .on_action(cx.listener(|this, _: &TogglePalette, _, cx| { this.palette = !this.palette; cx.notify(); }))
            .on_action(cx.listener(|this, _: &SaveDocument, _, cx| this.save(cx)))
            .child(div().h(px(60.)).px(px(20.)).flex().items_center().justify_between()
                .child(div().text_size(px(20.)).child("◈  skill studio  ·  GPUI"))
                .child(div().flex().gap(px(8.))
                    .child(div().id("palette").p(px(10.)).rounded(px(7.)).bg(rgb(0x29354c)).cursor_pointer().child("命令").on_click(cx.listener(|this, _, _, cx| { this.palette = !this.palette; cx.notify(); })))
                    .child(div().id("refresh").p(px(10.)).rounded(px(7.)).bg(rgb(0x29354c)).cursor_pointer().child("刷新").on_click(cx.listener(|this, _, window, cx| this.refresh(window, cx))))
                    .child(div().id("open").p(px(10.)).rounded(px(7.)).bg(rgb(0x7654d8)).cursor_pointer().child("打开工作区").on_click(cx.listener(|this, _, window, cx| this.open(window, cx))))))
            .child(div().flex_1().min_h(px(0.)).flex().child(files).child(graph_panel).child(editor))
            .child(div().h(px(30.)).px(px(16.)).text_size(px(11.)).child(self.status.clone()))
            .children(palette)
    }
}
fn main() {
    gpui_platform::application().run(|cx| {
        gpui_base::init(cx);
        let modifier = if cfg!(target_os = "macos") { "cmd" } else { "ctrl" };
        cx.bind_keys([KeyBinding::new(&format!("{modifier}-shift-p"), TogglePalette, None), KeyBinding::new(&format!("{modifier}-s"), SaveDocument, None)]);
        let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1440.),px(900.)), cx))), titlebar: Some(TitlebarOptions { title: Some("skill studio".into()), ..Default::default() }), ..Default::default() };
        cx.open_window(options, |window, cx| {
            let editor = cx.new(|cx| {
                let mut state = EditorState::new(window, cx).line_number(true);
                state.set_editor_style(InputEditorStyle { foreground: rgb(0xdce4f3).into(), muted_foreground: rgb(0x8796ae).into(), caret: rgb(0xdce4f3).into(), selection: hsla(0.65, 0.6, 0.5, 0.45), ..Default::default() });
                state
            });
            let studio = cx.new(|cx| {
                cx.observe(&editor, |_, _, cx| cx.notify()).detach();
                Studio { graph: None, selected: None, editor, original: String::new(), status: "GPUI 迁移预览".into(), conflict: false, generation: 0, watcher: None, positions: layout::Positions::new(), drag: None, zoom: 1., palette: false }
            });
            let weak = studio.downgrade();
            window.on_window_should_close(cx, move |_, cx| weak.update(cx, |this, cx| this.guard(cx)).unwrap_or(true));
            studio
        }).expect("无法打开窗口");
        cx.activate(true);
    });
}
