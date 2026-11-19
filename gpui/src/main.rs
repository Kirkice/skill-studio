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
mod documents;
mod preview;
mod flow;
mod meaning;
mod panes;
mod explorer;
mod files;
mod opening;
use gpui::*;
use gpui_base::input::{EditorState, InputState, InputEvent};
actions!(studio, [TogglePalette, SaveDocument, CloseOverlay, FindDocument, CloseDocument, TogglePreview]);

struct Studio { open_ticket:u64, workspace_file:Option<String>, dragging_external:bool, explorer:explorer::Explorer, panes:panes::Panes, flow_canvas:Option<Entity<ui::graph::flow::FlowCanvas>>, flow_mode:bool, documents:documents::Documents<Entity<EditorState>>, graph_canvas: Option<Entity<ui::graph::GraphCanvas>>, graph_revision: u64, saving: bool, refreshing: bool, refresh_pending: bool, find_open: bool, find_query: Entity<InputState>, replace_query: Entity<InputState>, preferences: settings::Settings, settings_open: bool, focus_handle: FocusHandle, graph: Option<core::Graph>, selected: Option<String>, editor: Entity<EditorState>, original: String, status: String, conflict: bool, generation: u64, watcher: Option<notify::RecommendedWatcher>, positions: layout::Positions, roles: layout::Roles, zoom: f32, palette: bool, palette_index: usize, role_menu: bool, document_menu: bool, command_search: Entity<InputState>, search: Entity<InputState>, skill_name: Entity<InputState>, creating: bool, scope: filter::Scope, focus: bool, show_missing: bool }
impl Studio {
    fn active_text(&self)->bool{self.selected.as_ref().and_then(|id|self.documents.get(id)).is_some_and(|t|t.view.is_text())}
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.active_text(){return;}
        if let Some(id)=&self.selected{if let Some(tab)=self.documents.get_mut(id){tab.preview=false;}}
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
        if let Some(canvas)=&self.graph_canvas{canvas.update(cx,|_,cx|cx.notify());}
        if let Some(canvas)=&self.flow_canvas{canvas.update(cx,|_,cx|cx.notify());}
        let wrap=self.preferences.soft_wrap;let numbers=self.preferences.line_numbers;
        for tab in self.documents.tabs.iter().filter(|t|t.view.is_text()){
            let ext=std::path::Path::new(&tab.id).extension().and_then(|e|e.to_str()).unwrap_or("txt").to_string();
            tab.editor.update(cx,|editor,cx|{editor.set_editor_style(ui::theme::editor_style());editor.set_soft_wrap(wrap,window,cx);editor.set_line_number(numbers,window,cx);editor.set_highlighter(ext,cx);});
        }
        for input in [&self.search,&self.skill_name,&self.command_search,&self.find_query,&self.replace_query] {input.update(cx,|state,_|state.set_editor_style(ui::theme::input_style()));}
        self.status=match settings::save(&self.preferences){Ok(())=>"Settings saved".into(),Err(e)=>format!("Settings applied, but could not be saved: {e}")};
        cx.notify();
    }
    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette=false;self.creating=false;self.document_menu=false;self.role_menu=false;
        self.settings_open=!self.settings_open;self.focus_handle.focus(window,cx);cx.notify();
    }
    fn dirty(&self,_cx:&App)->bool{self.selected.as_ref().and_then(|id|self.documents.get(id)).is_some_and(|tab|tab.dirty)}
    fn tab_dirty(&self,id:&str,_cx:&App)->bool{
        self.documents.get(id).is_some_and(|tab|tab.dirty)
    }
    fn guard(&mut self,cx:&mut Context<Self>)->bool{
        if self.saving{self.status="Save in progress. Please wait.".into();cx.notify();return false;}
        if self.documents.tabs.iter().any(|tab|self.tab_dirty(&tab.id,cx)){
            self.status="Unsaved tabs. Save or close each modified tab before changing workspace or exiting.".into();cx.notify();return false;
        }true
    }
    fn save(&mut self,window:&mut Window,cx:&mut Context<Self>){
        if let Some(id)=self.selected.clone(){self.save_tab(id,false,window,cx);}
    }
    fn save_tab(&mut self,id:String,close_after:bool,window:&mut Window,cx:&mut Context<Self>){
        if self.selected.as_deref()==Some(id.as_str())&&!self.composition_ready(window,cx){return;}
        if self.saving{return;}
        let (Some(graph),Some(tab))=(&self.graph,self.documents.get(&id))else{return;};
        if !tab.view.is_text(){self.status="Read-only files cannot be saved by the text editor".into();cx.notify();return;}
        let root=graph.root.clone();let token=tab.token;
        let content=tab.editor.read(cx).value().to_string();let original=tab.original.clone();
        self.documents.pin(&id);self.saving=true;self.status="Saving…".into();cx.notify();
        cx.spawn_in(window,async move|this,cx|{
            let save_root=root.clone();let save_id=id.clone();let saved=content.clone();
            let result=cx.background_executor().spawn(async move{core::save_document(std::path::Path::new(&save_root),&save_id,&saved,&original)}).await;
            let _=this.update_in(cx,|this,window,cx|{
                this.saving=false;
                if this.graph.as_ref().map(|g|g.root.as_str())!=Some(root.as_str()){return;}
                match result{
                    Ok(())=>{
                        let mut can_close=false;
                        if let Some(tab)=this.documents.get_mut(&id).filter(|tab|tab.token==token){
                            tab.set_original(content.clone());tab.conflict=false;
                            can_close=tab.editor.read(cx).value().as_ref()==content;
                        }
                        if this.selected.as_deref()==Some(&id){this.original=content;this.conflict=false;}
                        if close_after&&can_close{this.remove_tab(&id,window,cx);}
                        this.generation+=1;this.status=if close_after&&!can_close{"Saved snapshot; newer edits kept open"}else{"Saved"}.into();
                        this.refresh(window,cx);
                    }
                    Err(e)=>{this.status=format!("Save failed: {e}");if this.refresh_pending{this.refresh(window,cx);}}
                }cx.notify();
            });
        }).detach();
    }
    fn composition_ready(&mut self,window:&mut Window,cx:&mut Context<Self>)->bool{
        let composing=self.active_text()&&self.editor.update(cx,|editor,cx|editor.marked_text_range(window,cx).is_some());
        if composing{self.status="Finish IME input with Enter or Esc before switching tabs".into();cx.notify();}
        !composing
    }
    fn activate_tab(&mut self,id:&str,window:&mut Window,cx:&mut Context<Self>){
        if self.selected.as_deref()!=Some(id)&&!self.composition_ready(window,cx){return;}
        let Some(tab)=self.documents.get(id)else{return;};
        self.explorer.reveal(id);
        self.selected=Some(id.into());self.editor=tab.editor.clone();self.original=tab.original.clone();self.conflict=tab.conflict;
        self.role_menu=false;self.document_menu=false;
        if !tab.view.is_text()||tab.preview{self.find_open=false;}
        if self.find_open{let query=self.find_query.read(cx).value().to_string();self.editor.update(cx,|e,cx|e.set_search_query(query,true,cx));}
        if !tab.view.is_text()||tab.preview{self.focus_handle.focus(window,cx);}else{self.editor.update(cx,|editor,cx|editor.focus(window,cx));}cx.notify();
    }
    fn remove_tab(&mut self,id:&str,window:&mut Window,cx:&mut Context<Self>){
        let active=self.selected.as_deref()==Some(id);
        let next=self.documents.remove(id);
        if active{
            self.selected=None;self.original.clear();self.conflict=false;
            if let Some(next)=next{self.activate_tab(&next,window,cx);}else{self.find_open=false;self.focus_handle.focus(window,cx);}
        }cx.notify();
    }
    fn close_tab(&mut self,id:String,window:&mut Window,cx:&mut Context<Self>){
        if self.selected.as_deref()==Some(id.as_str())&&!self.composition_ready(window,cx){return;}
        if self.saving{self.status="Wait for the current save to finish".into();cx.notify();return;}
        let Some(tab)=self.documents.get(&id)else{return;};
        if !self.tab_dirty(&id,cx){self.remove_tab(&id,window,cx);return;}
        let token=tab.token;let before=tab.editor.read(cx).value();
        let answer=window.prompt(PromptLevel::Warning,"Save changes before closing?",Some(&id),&[PromptButton::ok("Save"),PromptButton::ok("Discard"),PromptButton::cancel("Cancel")],cx);
        cx.spawn_in(window,async move|this,cx|{
            let answer=answer.await.ok();
            let _=this.update_in(cx,|this,window,cx|{
                let Some(tab)=this.documents.get(&id).filter(|tab|tab.token==token)else{return;};
                if this.saving{return;}
                match answer{
                    Some(0)=>this.save_tab(id,true,window,cx),
                    Some(1) if tab.editor.read(cx).value()==before=>this.remove_tab(&id,window,cx),
                    Some(1)=>{this.status="Document changed while confirming; tab kept open".into();cx.notify();},
                    _=>{}
                }
            });
        }).detach();
    }
    fn close_other_saved_tabs(&mut self,cx:&mut Context<Self>){
        if self.saving{return;}
        let ids:Vec<_>=self.documents.tabs.iter().filter(|tab|Some(&tab.id)!=self.selected.as_ref()&&!self.tab_dirty(&tab.id,cx)).map(|tab|tab.id.clone()).collect();
        for id in ids{self.documents.remove(&id);}cx.notify();
    }
    fn reference(&mut self,source:String,line:usize,window:&mut Window,cx:&mut Context<Self>){
        if self.composition_ready(window,cx){self.request_file_at(source,false,false,false,Some(line),window,cx);}
    }
    fn command(&mut self, command: commands::Command, window: &mut Window, cx: &mut Context<Self>) {
        if command.needs_workspace() && self.graph.is_none() { self.status = "Open a workspace first".into(); cx.notify(); return; }
        self.palette = false;
        self.role_menu = false;
        self.document_menu = false;
        self.focus_handle.focus(window,cx);
        match command {
            commands::Command::Open => self.open(window,cx), commands::Command::Refresh => self.refresh(window,cx), commands::Command::Save => self.save(window,cx), commands::Command::SaveLayout => self.save_layout(cx), commands::Command::Import => self.import_layout(window,cx), commands::Command::Export => self.export_layout(window,cx), commands::Command::Reset => self.reset_layout(cx), commands::Command::Create => {self.creating=true;self.skill_name.update(cx,|input,cx|input.focus(window,cx));}, commands::Command::Settings => self.open_settings(window,cx), commands::Command::Close => {},
        }
        cx.notify();
    }
    fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette=false; self.creating=false; self.role_menu=false; self.document_menu=false; self.settings_open=false;
        if let Some(canvas)=&self.graph_canvas{canvas.update(cx,|canvas,cx|canvas.close_reference_menu(cx));}
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
        let scope=self.workspace_file.clone();
        let started=if scope.is_some(){watch::start_single(std::path::Path::new(&root))}else{watch::start(std::path::Path::new(&root))};
        let (watcher, events) = match started {
            Ok(value) => value,
            Err(e) => { self.status = format!("File watching unavailable. Refresh manually: {e}"); cx.notify(); return; }
        };
        self.watcher = Some(watcher);
        cx.spawn_in(window, async move |this, cx| {
            while events.recv().await.is_ok() {
                let active=this.update_in(cx,|studio,window,cx|{
                    if studio.graph.as_ref().map(|g|g.root.as_str())!=Some(root.as_str())||studio.workspace_file!=scope{return false;}
                    studio.refresh(window,cx);true
                });
                if !matches!(active,Ok(true)){break;}
            }
        }).detach();
    }
    fn refresh(&mut self, window:&mut Window, cx:&mut Context<Self>){
        if self.graph.is_none(){return;}
        self.refresh_pending=true;
        if self.saving||self.refreshing{return;}
        self.refreshing=true;
        let root=self.graph.as_ref().unwrap().root.clone();
        cx.spawn_in(window,async move|this,cx|{
            // One debounce window for manual refresh + atomic-write watcher events.
            cx.background_executor().timer(std::time::Duration::from_millis(220)).await;
            let snapshot=this.update_in(cx,|studio,window,cx|{
                if studio.graph.as_ref().map(|g|g.root.as_str())!=Some(root.as_str())||studio.saving{
                    studio.refreshing=false;
                    if studio.refresh_pending&&!studio.saving{studio.refresh(window,cx);}
                    return None;
                }
                studio.refresh_pending=false;studio.generation+=1;
                Some((studio.generation,studio.workspace_file.clone(),studio.documents.tabs.iter().map(|tab|(tab.id.clone(),tab.token,tab.original.clone(),tab.view.stamp())).collect::<Vec<_>>()))
            });
            let Ok(Some((generation,scope,sessions)))=snapshot else{return;};
            let scan_root=root.clone();
            let (graph,document)=cx.background_executor().spawn(async move{
                let path=std::path::Path::new(&scan_root);
                (if let Some(file)=scope{core::scan_file(&path.join(file))}else{core::scan_workspace(path)},sessions.into_iter().map(|(id,token,original,stamp)|{
                    let changed=if let Some(stamp)=stamp{core::safe(path,&id).and_then(|p|files::stamp(&p)).map_or(true,|now|now!=stamp)}else{core::read_document(path,&id).map_or(true,|doc|doc.content!=original)};
                    (id,token,original,changed)
                }).collect::<Vec<_>>())
            }).await;
            let _=this.update_in(cx,|studio,window,cx|{
                studio.refreshing=false;
                if studio.graph.as_ref().map(|g|g.root.as_str())!=Some(root.as_str()){
                    if studio.refresh_pending{studio.refresh(window,cx);}return;
                }
                if studio.generation==generation&&!studio.saving{
                    match graph{
                        Ok(mut graph)=>{
                            if studio.workspace_file.is_some(){for tab in &studio.documents.tabs{if !graph.entries.iter().any(|e|e.id==tab.id){graph.entries.push(core::WorkspaceEntry{id:tab.id.clone(),directory:false});}}}
                            studio.graph_revision+=1;studio.graph=Some(graph);studio.status="Index refreshed".into();
                        }
                        Err(e)=>{studio.status=format!("Refresh failed: {e}");}
                    }
                    for(id,token,original,changed)in document{
                        if let Some(tab)=studio.documents.get_mut(&id).filter(|tab|tab.token==token&&tab.original==original){
                            tab.conflict=changed;
                        }
                    }
                    studio.conflict=studio.selected.as_ref().and_then(|id|studio.documents.get(id)).is_some_and(|tab|tab.conflict);
                    if studio.conflict{studio.status="File changed or deleted externally; editor content retained".into();}
                }else{studio.refresh_pending=true;}
                if studio.refresh_pending{studio.refresh(window,cx);}
                cx.notify();
            });
        }).detach();
    }
    fn reload(&mut self,window:&mut Window,cx:&mut Context<Self>){
        if self.saving||self.dirty(cx){self.status="Save or discard this tab before reloading".into();cx.notify();return;}
        if let Some(id)=self.selected.clone(){self.request_file(id,true,true,false,window,cx);}
    }
    fn discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving || !self.dirty(cx) { return; }
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
    fn open(&mut self,window:&mut Window,cx:&mut Context<Self>){
        if !self.composition_ready(window,cx){return;}
        let paths=cx.prompt_for_paths(PathPromptOptions{files:false,directories:true,multiple:false,prompt:Some("Open workspace folder".into())});
        cx.spawn_in(window,async move|this,cx|{
            if let Ok(Ok(Some(paths)))=paths.await{let _=this.update_in(cx,|this,window,cx|this.open_external(paths,window,cx));}
        }).detach();
    }
    fn select_file(&mut self,id:String,pin:bool,window:&mut Window,cx:&mut Context<Self>){self.request_file(id,pin,false,false,window,cx);}


}
fn main() {
    let mut args=std::env::args_os().skip(1);
    let initial_workspace=if matches!(args.next().as_deref(),Some(flag)if flag==std::ffi::OsStr::new("--workspace")||flag==std::ffi::OsStr::new("--open")){args.next().map(std::path::PathBuf::from)}else{None};
    gpui_platform::application().with_assets(ui::icons::Assets).run(move |cx| {
        gpui_base::init(cx);
        let loaded=settings::load();
        let mut settings_warning=loaded.as_ref().err().map(|e|format!("Settings could not be loaded: {e}"));
        let preferences=loaded.unwrap_or_default();
        if let Err(e)=ui::theme::load_fonts(cx){settings_warning=Some(format!("Bundled fonts could not be loaded: {e}"));}
        ui::theme::install(&preferences,cx);
        let modifier = if cfg!(target_os = "macos") { "cmd" } else { "ctrl" };
        cx.bind_keys([KeyBinding::new(&format!("{modifier}-shift-p"), TogglePalette, None), KeyBinding::new(&format!("{modifier}-s"), SaveDocument, None), KeyBinding::new(&format!("{modifier}-w"), CloseDocument, None), KeyBinding::new(&format!("{modifier}-shift-v"), TogglePreview, None), KeyBinding::new("escape", CloseOverlay, None), KeyBinding::new(&format!("{modifier}-f"), FindDocument, None)]);
        let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1440.),px(900.)), cx))), titlebar: None, app_owns_titlebar_drag:true, window_min_size:Some(size(px(640.),px(480.))), ..Default::default() };
        cx.open_window(options, move |window, cx| {
            window.set_window_title("skill studio");
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
                cx.subscribe(&find_query,|this: &mut Studio,query,event,cx|{if matches!(event,InputEvent::Change){if this.find_open{let query=query.read(cx).value().to_string();this.editor.update(cx,|s,cx|s.set_search_query(query,true,cx));}cx.notify();}}).detach();
                cx.subscribe(&command_search, |this: &mut Studio, _, event,cx| { if matches!(event,InputEvent::Change){this.palette_index=0;cx.notify();} }).detach();
                cx.subscribe(&search, |_: &mut Studio, _, event,cx| {if matches!(event,InputEvent::Change){cx.notify();}}).detach();
                Studio { open_ticket:0, workspace_file:None, dragging_external:false, explorer:Default::default(), panes:Default::default(), flow_canvas:None, flow_mode:true, documents:Default::default(), graph_canvas: None, graph_revision: 0, saving: false, refreshing: false, refresh_pending: false, find_open: false, find_query, replace_query, preferences, settings_open: false, focus_handle: cx.focus_handle(), graph: None, selected: None, editor, original: String::new(), status: settings_warning.unwrap_or_else(||"Ready".into()), conflict: false, generation: 0, watcher: None, positions: layout::Positions::new(), roles: layout::Roles::new(), zoom: 1., palette: false, palette_index: 0, role_menu: false, document_menu: false, command_search, search, skill_name, creating: false, scope: filter::Scope::Direct, focus: false, show_missing: false }
            });
            studio.read(cx).focus_handle.clone().focus(window, cx);
            if let Some(path)=&initial_workspace {
                studio.update(cx,|this,cx|this.open_external(vec![path.clone()],window,cx));
            }
            let weak = studio.downgrade();
            window.on_window_should_close(cx, move |_, cx| weak.update(cx, |this, cx| this.guard(cx)).unwrap_or(true));
            studio
        }).expect("Unable to open window");
        cx.activate(true);
    });
}
