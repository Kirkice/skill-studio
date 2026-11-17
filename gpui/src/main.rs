#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
mod core;
mod watch;
mod layout;
mod filter;
mod role;
mod highlight;
mod commands;
mod ui;
mod settings;
use gpui::*;
use gpui_base::input::{EditorState, InputState};
actions!(studio, [TogglePalette, SaveDocument, CloseOverlay, FindDocument]);

struct Studio { find_open: bool, find_query: Entity<InputState>, replace_query: Entity<InputState>, preferences: settings::Settings, settings_open: bool, focus_handle: FocusHandle, graph: Option<core::Graph>, selected: Option<String>, editor: Entity<EditorState>, original: String, status: String, conflict: bool, generation: u64, watcher: Option<notify::RecommendedWatcher>, positions: layout::Positions, roles: layout::Roles, drag: Option<(String, Point<Pixels>, layout::Position)>, zoom: f32, drag_moved: bool, palette: bool, palette_index: usize, role_menu: bool, document_menu: bool, command_search: Entity<InputState>, search: Entity<InputState>, skill_name: Entity<InputState>, creating: bool, scope: filter::Scope, focus: bool, show_missing: bool }
impl Studio {
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_none(){return;}
        self.find_open=true;
        self.find_query.update(cx,|s,cx|s.focus(window,cx));
        let query=self.find_query.read(cx).value().to_string();
        self.editor.update(cx,|s,cx|s.set_search_query(query,true,cx));cx.notify();
    }
    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open=false;self.editor.update(cx,|s,cx|{s.close_search(cx);s.focus(window,cx);});cx.notify();
    }
    fn apply_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        ui::theme::install(&self.preferences,cx);
        let ext=self.selected.as_deref().and_then(|s|std::path::Path::new(s).extension()).and_then(|s|s.to_str()).unwrap_or("txt").to_string();
        let wrap=self.preferences.soft_wrap;let numbers=self.preferences.line_numbers;
        self.editor.update(cx,|editor,cx|{editor.set_editor_style(ui::theme::editor_style());editor.set_soft_wrap(wrap,window,cx);editor.set_line_number(numbers,window,cx);editor.set_highlighter(ext,cx);});
        for input in [&self.search,&self.skill_name,&self.command_search,&self.find_query,&self.replace_query] {input.update(cx,|state,_|state.set_editor_style(ui::theme::input_style()));}
        self.status=match settings::save(&self.preferences){Ok(())=>"Settings saved".into(),Err(e)=>format!("Settings applied, but could not be saved: {e}")};
        cx.notify();
    }
    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette=false;self.creating=false;self.document_menu=false;self.role_menu=false;
        self.settings_open=!self.settings_open;self.focus_handle.focus(window,cx);cx.notify();
    }
    fn dirty(&self, cx: &App) -> bool { self.selected.is_some() && self.editor.read(cx).value().as_ref() != self.original }
    fn guard(&mut self, cx: &mut Context<Self>) -> bool {
        if self.dirty(cx) { self.status = "Unsaved changes. Save or discard before continuing.".into(); cx.notify(); false } else { true }
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
                    Ok(updated) => { self.graph = Some(updated); self.status = "Saved".into(); }
                    Err(e) => self.status = format!("Saved, but indexing failed: {e}"),
                }
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
        cx.notify();
    }
    fn reference(&mut self, source: String, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.select(source.clone(), window, cx);
        if self.selected.as_deref() == Some(&source) {
            self.editor.update(cx, |editor, cx| editor.set_cursor_position(gpui_base::input::Position::new(line.saturating_sub(1).min(u32::MAX as usize) as u32, 0), window, cx));
            self.status = format!("Reference: {source} · line {line}"); cx.notify();
        }
    }
    fn command(&mut self, command: commands::Command, window: &mut Window, cx: &mut Context<Self>) {
        if command.needs_workspace() && self.graph.is_none() { self.status = "Open a workspace first".into(); cx.notify(); return; }
        self.palette = false;
        self.role_menu = false;
        self.document_menu = false;
        self.focus_handle.focus(window,cx);
        match command {
            commands::Command::Open => self.open(window,cx), commands::Command::Refresh => self.refresh(window,cx), commands::Command::Save => self.save(cx), commands::Command::SaveLayout => self.save_layout(cx), commands::Command::Import => self.import_layout(window,cx), commands::Command::Export => self.export_layout(window,cx), commands::Command::Reset => self.reset_layout(cx), commands::Command::Create => {self.creating=true;self.skill_name.update(cx,|input,cx|input.focus(window,cx));}, commands::Command::Settings => self.open_settings(window,cx), commands::Command::Close => {},
        }
        cx.notify();
    }
    fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette=false; self.creating=false; self.role_menu=false; self.document_menu=false; self.settings_open=false;
        if self.find_open{self.close_find(window,cx);}else{self.focus_handle.focus(window,cx);}
        cx.notify();
    }
    fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.creating=false; self.role_menu=false; self.document_menu=false; self.settings_open=false; self.palette_index=0;
        self.palette = !self.palette;
        if self.palette { self.command_search.update(cx, |input,cx| {input.set_value("",window,cx);input.focus(window,cx);}); } else { self.focus_handle.focus(window,cx); }
        cx.notify();
    }
    fn save_layout(&mut self, cx: &mut Context<Self>) {
        if let Some(graph) = &self.graph {
            self.status = match layout::save(&graph.root, &self.positions, &self.roles) { Ok(()) => "Layout saved".into(), Err(e) => format!("Failed to save layout: {e}") };
            cx.notify();
        }
    }
    fn reset_layout(&mut self, cx: &mut Context<Self>) {
        self.positions.clear();
        self.save_layout(cx);
    }
    fn create_skill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.graph.as_ref().map(|g| g.root.clone()) else { self.status = "Open a workspace first".into(); cx.notify(); return };
        let name = self.skill_name.read(cx).value().to_string();
        match core::create_skill(std::path::Path::new(&root), &name) {
            Ok(()) => { self.creating = false; self.focus_handle.focus(window,cx); self.skill_name.update(cx, |input, cx| input.set_value("", window, cx)); self.refresh(window, cx); self.status = format!("Created Skill: {name}"); }
            Err(e) => self.status = format!("Creation failed: {e}"),
        }
        cx.notify();
    }
    fn export_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.graph.as_ref().map(|g| g.root.clone()) else { return };
        let directory = dirs::download_dir().or_else(dirs::home_dir).unwrap_or_else(std::env::temp_dir);
        let receiver = cx.prompt_for_new_path(&directory, Some("skill-studio-layout.json"));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = receiver.await else { return };
            let _ = this.update_in(cx, |this, _, cx| {
                if this.graph.as_ref().map(|g| &g.root) != Some(&root) { return; }
                this.status = match layout::write_file(&path, &this.positions, &this.roles) { Ok(()) => "Layout exported".into(), Err(e) => format!("Export failed: {e}") };
                cx.notify();
            });
        }).detach();
    }
    fn import_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.graph.as_ref().map(|g| g.root.clone()) else { return };
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Import layout JSON".into()) });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return }; let Some(path) = paths.into_iter().next() else { return };
            let result = cx.background_executor().spawn(async move { let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?; if metadata.len() > 2_000_000 { return Err("Layout file exceeds 2 MB".into()); } let bytes = std::fs::read(path).map_err(|e| e.to_string())?; layout::decode(&bytes) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.graph.as_ref().map(|g| &g.root) != Some(&root) { return; }
                match result {
                    Ok(data) => {
                        let answer = window.prompt(PromptLevel::Warning, "Replace current layout?", Some("Importing replaces the saved layout for this workspace."), &[PromptButton::ok("Import"), PromptButton::cancel("Cancel")], cx);
                        cx.spawn_in(window, async move |this, cx| {
                            if answer.await.ok() != Some(0) { return; }
                            let _ = this.update_in(cx, |this, _, cx| {
                                if this.graph.as_ref().map(|g| &g.root) != Some(&root) { return; }
                                this.positions = data.positions; this.roles = data.roles; this.save_layout(cx);
                            });
                        }).detach();
                    }
                    Err(e) => { this.status = format!("Import failed: {e}"); cx.notify(); }
                }
            });
        }).detach();
    }
    fn start_watcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.watcher = None;
        let Some(root) = self.graph.as_ref().map(|g| g.root.clone()) else { return };
        let (watcher, events) = match watch::start(std::path::Path::new(&root)) {
            Ok(value) => value,
            Err(e) => { self.status = format!("File watching unavailable. Refresh manually: {e}"); cx.notify(); return; }
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
                        Err(e) => { studio.status = format!("Automatic refresh failed: {e}"); cx.notify(); return; }
                    }
                    if studio.selected == selected && studio.original == original {
                        studio.conflict = result.1.is_some_and(|doc| doc.map_or(true, |doc| doc.content != original));
                    }
                    if studio.conflict { studio.status = "File changed or deleted externally; editor content retained".into(); }
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
                    Ok(graph) => { studio.graph = Some(graph); studio.status = "Index refreshed".into(); }
                    Err(e) => { studio.status = format!("Refresh failed: {e}"); cx.notify(); return; }
                }
                if studio.selected == selected && studio.original == original {
                    studio.conflict = document.is_some_and(|doc| doc.map_or(true, |doc| doc.content != original));
                }
                if studio.conflict { studio.status = "File changed or deleted externally; editor content retained".into(); }
                cx.notify();
            });
        }).detach();
    }
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.guard(cx) { return; }
        let (Some(graph), Some(id)) = (&self.graph, &self.selected) else { return };
        match core::read_document(std::path::Path::new(&graph.root), id) {
            Ok(doc) => { self.original = doc.content.clone(); self.editor.update(cx, |editor, cx| editor.set_value(doc.content, window, cx)); self.conflict = false; self.status = "Reloaded from disk".into(); }
            Err(e) => self.status = format!("Reload failed: {e}"),
        }
        cx.notify();
    }
    fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.dirty(cx) { return; }
        let selected = self.selected.clone();
        let before = self.editor.read(cx).value();
        let answer = window.prompt(PromptLevel::Warning, "Discard unsaved changes?", Some("This restores the content last opened or saved."), &[PromptButton::ok("Discard changes"), PromptButton::cancel("Cancel")], cx);
        cx.spawn_in(window, async move |this, cx| {
            if answer.await.ok() != Some(0) { return; }
            let _ = this.update_in(cx, |this, window, cx| {
                if this.selected != selected || this.editor.read(cx).value() != before { this.status = "Document changed; edits were not discarded".into(); cx.notify(); return; }
                let value = this.original.clone();
                this.editor.update(cx, |editor, cx| editor.set_value(value, window, cx));
                this.status = "Changes discarded".into();
                cx.notify();
            });
        }).detach();
    }
}

impl Studio {
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.guard(cx) { return; }
        let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Open Skill workspace".into()) });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let result = cx.background_executor().spawn(async move { core::open_workspace(&path) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(graph) => { if !this.guard(cx) { return; } this.generation += 1; this.status = format!("{} files · {} references", graph.nodes.len(), graph.edges.len()); let loaded = layout::load(&graph.root); let loaded = loaded.unwrap_or_else(|e| { this.status = format!("Failed to load layout: {e}"); layout::LayoutFile { version: 1, ..Default::default() } }); this.positions = loaded.positions; this.roles = loaded.roles; this.zoom = 1.; this.focus = false; this.graph = Some(graph); this.selected = None; this.original.clear(); this.conflict = false; this.start_watcher(window, cx); }
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
            Ok(doc) => { self.selected = Some(id); self.original = doc.content.clone(); self.conflict = false; self.editor.update(cx, |editor, cx| { let extension = std::path::Path::new(self.selected.as_deref().unwrap_or("")).extension().and_then(|s| s.to_str()).unwrap_or("txt"); editor.set_highlighter(extension.to_string(), cx); editor.set_value(doc.content, window, cx); }); }
            Err(e) => self.status = e,
        }
        cx.notify();
    }
}
fn main() {
    let mut args=std::env::args_os().skip(1);
    let initial_workspace=if args.next().as_deref()==Some(std::ffi::OsStr::new("--workspace")){args.next().map(std::path::PathBuf::from)}else{None};
    gpui_platform::application().with_assets(ui::icons::Assets).run(move |cx| {
        gpui_base::init(cx);
        let loaded=settings::load();
        let mut settings_warning=loaded.as_ref().err().map(|e|format!("Settings could not be loaded: {e}"));
        let preferences=loaded.unwrap_or_default();
        if let Err(e)=ui::theme::load_fonts(cx){settings_warning=Some(format!("Bundled fonts could not be loaded: {e}"));}
        ui::theme::install(&preferences,cx);
        let modifier = if cfg!(target_os = "macos") { "cmd" } else { "ctrl" };
        cx.bind_keys([KeyBinding::new(&format!("{modifier}-shift-p"), TogglePalette, None), KeyBinding::new(&format!("{modifier}-s"), SaveDocument, None), KeyBinding::new("escape", CloseOverlay, None), KeyBinding::new(&format!("{modifier}-f"), FindDocument, None)]);
        let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1440.),px(900.)), cx))), titlebar: Some(TitlebarOptions { title: Some("skill studio".into()), ..Default::default() }), ..Default::default() };
        cx.open_window(options, move |window, cx| {
            let editor = cx.new(|cx| {
                let mut state = EditorState::new(window, cx).line_number(preferences.line_numbers).soft_wrap(preferences.soft_wrap).replaceable(true);
                state.set_highlighter_factory(std::rc::Rc::new(|language| Some(Box::new(highlight::Highlighter::new(language)))), cx);
                state.set_editor_style(ui::theme::editor_style());
                state
            });
            let search = cx.new(|cx| { let mut input = InputState::new(window, cx).placeholder("Search files or Skills…"); input.set_editor_style(ui::theme::input_style()); input });
            let skill_name = cx.new(|cx| { let mut input = InputState::new(window, cx).placeholder("my-new-skill"); input.set_editor_style(ui::theme::input_style()); input });
            let command_search = cx.new(|cx| { let mut input=InputState::new(window,cx).placeholder("Type a command…"); input.set_editor_style(ui::theme::input_style()); input });
            let find_query=cx.new(|cx|{let mut input=InputState::new(window,cx).placeholder("Find in document…");input.set_editor_style(ui::theme::input_style());input});
            let replace_query=cx.new(|cx|{let mut input=InputState::new(window,cx).placeholder("Replace with…");input.set_editor_style(ui::theme::input_style());input});
            let studio = cx.new(|cx| {
                cx.observe(&find_query,|this: &mut Studio,query,cx|{if this.find_open{let query=query.read(cx).value().to_string();this.editor.update(cx,|s,cx|s.set_search_query(query,true,cx));}cx.notify();}).detach();
                cx.observe(&command_search, |this: &mut Studio, _, cx| { this.palette_index=0; cx.notify(); }).detach();
                cx.observe(&editor, |_, _, cx| cx.notify()).detach();
                cx.observe(&search, |_, _, cx| cx.notify()).detach();
                Studio { find_open: false, find_query, replace_query, preferences, settings_open: false, focus_handle: cx.focus_handle(), graph: None, selected: None, editor, original: String::new(), status: settings_warning.unwrap_or_else(||"Ready".into()), conflict: false, generation: 0, watcher: None, positions: layout::Positions::new(), roles: layout::Roles::new(), drag: None, zoom: 1., drag_moved: false, palette: false, palette_index: 0, role_menu: false, document_menu: false, command_search, search, skill_name, creating: false, scope: filter::Scope::Direct, focus: false, show_missing: false }
            });
            studio.read(cx).focus_handle.clone().focus(window, cx);
            if let Some(path)=&initial_workspace {
                studio.update(cx, |this,cx| {
                    match core::open_workspace(path) {
                        Ok(graph)=>{let loaded=layout::load(&graph.root).unwrap_or_else(|_|layout::LayoutFile{version:1,..Default::default()});this.positions=loaded.positions;this.roles=loaded.roles;this.graph=Some(graph);this.start_watcher(window,cx);},
                        Err(e)=>this.status=e,
                    }
                });
            }
            let weak = studio.downgrade();
            window.on_window_should_close(cx, move |_, cx| weak.update(cx, |this, cx| this.guard(cx)).unwrap_or(true));
            studio
        }).expect("Unable to open window");
        cx.activate(true);
    });
}
