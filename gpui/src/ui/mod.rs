mod controls;
pub mod theme;
pub mod icons;
pub(crate) mod graph;
mod settings;
mod preview;
mod chrome;
mod explorer;
mod file_preview;
pub(crate) mod terminal;
use crate::{commands::Command, filter, role, Studio, TogglePalette, SaveDocument, CloseOverlay, FindDocument, CloseDocument, TogglePreview, ToggleTerminal};
use controls::*;
use icons::{icon,Icon,file_icon,command_icon};
use theme::*;
use gpui::*;
use theme::rgb;
use gpui_base::{Editor, InputBase};

impl Studio {
    fn document_panel(&self,cx:&mut Context<Self>)->Div {
        let dirty=self.dirty(cx);
        let preview=self.is_preview();
        let text=self.active_text();
        let markdown=text&&self.selected.as_deref().is_some_and(crate::preview::supported);
        let cursor=self.editor.read(cx).cursor_position();
        let mut panel=div().size_full().min_w(px(0.)).overflow_hidden().bg(rgb(BG)).flex().flex_col();
        if !self.documents.tabs.is_empty(){
            let mut tabs=div().id("document-tabs").h(px(36.)).flex_none().overflow_x_scroll().bg(rgb(PANEL)).border_b_1().border_color(rgb(BORDER)).flex();
            for tab in &self.documents.tabs{
                let id=tab.id.clone();let close_id=id.clone();let active=self.selected.as_deref()==Some(id.as_str());
                let dirty=self.tab_dirty(&id,cx);let filename=id.rsplit('/').next().unwrap_or(&id);
                let duplicates=self.documents.tabs.iter().filter(|t|t.id.rsplit('/').next()==Some(filename)).count()>1;
                let label=if duplicates{id.rsplit('/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/")}else{filename.into()};
                let label=if tab.pinned{label}else{format!("{label} · preview")};
                tabs=tabs.child(div().id(("document-tab",tab.token)).h_full().flex_none().max_w(px(260.)).px(px(10.)).flex().items_center().gap(px(6.))
                    .bg(rgb(if active{BG}else{PANEL})).border_b_2().border_color(rgb(if active{ACCENT}else{PANEL})).cursor_pointer()
                    .child(icon(if matches!(tab.view,crate::files::View::Image{..}){Icon::Image}else{Icon::File}).size(px(13.)).text_color(rgb(DIM)))
                    .child(div().max_w(px(185.)).truncate().text_size(px(12.)).child(label))
                    .children(dirty.then(||div().text_color(rgb(WARNING)).child("●")))
                    .child(icon_button(("close-tab",tab.token),Icon::Close,"Close tab · Ctrl/Cmd+W",false,true).w(px(20.)).h(px(24.))
                        .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
                        .on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.close_tab(close_id.clone(),w,cx);})))
                    .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|{if event.click_count()>1{this.documents.pin(&id);}this.activate_tab(&id,w,cx);})));
            }
            panel=panel.child(tabs);
        }
        let title=self.selected.as_deref().and_then(|s|s.rsplit('/').next()).unwrap_or("Document").to_string();
        panel=panel.child(bar().bg(rgb(PANEL)).child(div().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(if dirty{WARNING}else{GREEN})))
            .child(div().flex_1().truncate().text_size(px(12.8)).child(title))
            .children(text.then(||icon_button("document-save",Icon::Save,if cfg!(target_os="macos"){"Save document · Cmd+S"}else{"Save document · Ctrl+S"},dirty,dirty).on_click(cx.listener(|this,_,w,cx|{if this.dirty(cx){this.save(w,cx)}}))))
            .children(markdown.then(||icon_button("markdown-preview",if preview{Icon::Code}else{Icon::Eye},if preview{"Show Markdown source · Ctrl/Cmd+Shift+V"}else{"Preview Markdown · Ctrl/Cmd+Shift+V"},preview,true).on_click(cx.listener(|this,_,w,cx|this.toggle_preview(w,cx)))))
            .children(text.then(||icon_button("editor-find",Icon::Search,"Find in document · Ctrl/Cmd+F",self.find_open,true).on_click(cx.listener(|this,_,w,cx|this.open_find(w,cx)))))
            .children((text&&!preview).then(||icon_button("editor-wrap",Icon::Wrap,"Toggle word wrap",self.preferences.soft_wrap,self.selected.is_some()).on_click(cx.listener(|this,_,w,cx|{this.preferences.soft_wrap=!this.preferences.soft_wrap;this.apply_preferences(w,cx);}))))
            .children((text&&!preview).then(||icon_button("editor-numbers",Icon::Numbers,"Toggle line numbers",self.preferences.line_numbers,self.selected.is_some()).on_click(cx.listener(|this,_,w,cx|{this.preferences.line_numbers=!this.preferences.line_numbers;this.apply_preferences(w,cx);}))))
            .child(icon_button("document-terminal",Icon::Terminal,"显示/收起本地终端 · Ctrl+`",self.terminal_visible,true).on_click(cx.listener(|this,_,w,cx|this.toggle_terminal(w,cx))))
            .child(icon_button("document-menu",Icon::More,"Document actions",false,self.selected.is_some()).on_click(cx.listener(|this,_,_,cx|{if this.selected.is_some(){this.document_menu=!this.document_menu;cx.notify();}}))));
        if let Some(id)=&self.selected {
            panel=panel.child(div().px(px(14.)).py(px(8.)).text_size(px(11.5)).text_color(rgb(DIM)).truncate().child(id.clone()));
            if self.conflict {
                panel=panel.child(div().mx(px(12.)).mb(px(8.)).p(px(10.)).rounded(px(6.)).bg(rgb(0x393126)).flex().flex_col().gap(px(8.)).text_color(rgb(WARNING))
                    .child("This file changed on disk. Your current view and edits are retained.")
                    .child(icon_button("reload-document",Icon::Refresh,"Reload document from disk",false,true).on_click(cx.listener(|this,_,w,cx|this.reload(w,cx)))));
            }
            if !text{return panel.child(self.non_text_panel(cx));}
            if self.find_open {
                let count=self.editor.read(cx).search_session().matcher.label();
                panel=panel.child(div().px(px(10.)).pb(px(8.)).flex().flex_col().gap(px(6.))
                    .child(div().flex().items_center().gap(px(4.)).child(div().flex_1().min_w(px(0.)).child(input("document-find",&self.find_query))).child(caption(count))
                        .child(icon_button("find-previous",Icon::ChevronDown,"Previous match",false,true).on_click(cx.listener(|this,_,_,cx|{this.editor.update(cx,|s,cx|s.previous_search_match(cx));})))
                        .child(icon_button("find-next",Icon::ArrowRight,"Next match",false,true).on_click(cx.listener(|this,_,_,cx|{this.editor.update(cx,|s,cx|s.next_search_match(cx));})))
                        .child(icon_button("close-find",Icon::Close,"Close find",false,true).on_click(cx.listener(|this,_,w,cx|this.close_find(w,cx)))))
                    .child(div().flex().items_center().gap(px(4.)).child(div().flex_1().min_w(px(0.)).child(input("document-replace",&self.replace_query)))
                        .child(icon_button("replace-one",Icon::Replace,"Replace current match",false,true).on_click(cx.listener(|this,_,w,cx|{let replacement=this.replace_query.read(cx).value().to_string();this.editor.update(cx,|s,cx|s.replace_current_search_match(&replacement,w,cx));})))
                        .child(icon_button("replace-all",Icon::ReplaceAll,"Replace all matches",false,true).on_click(cx.listener(|this,_,w,cx|{let replacement=this.replace_query.read(cx).value().to_string();let count=this.editor.update(cx,|s,cx|s.replace_all_search_matches(&replacement,w,cx));this.status=format!("Replaced {count} matches (not saved)");cx.notify();}))))
                    .capture_key_down(cx.listener(|this,event:&KeyDownEvent,_,cx|{if event.keystroke.key=="enter"{this.editor.update(cx,|s,cx|s.next_search_match(cx));cx.stop_propagation();}})));
            }
            if preview{panel=panel.child(self.markdown_preview(cx));}else{
            let editor=self.editor.clone();
            panel=panel.child(InputBase::new("source-editor").flex_1().min_h(px(0.)).w_full().overflow_hidden().bg(rgb(BG)).font_family(MONO_FONT).text_size(px(self.preferences.font_size as f32)).line_height(px(self.preferences.font_size as f32*1.65))
                .on_mouse_down(MouseButton::Left,move|_,w,cx|editor.update(cx,|s,cx|s.focus(w,cx))).child(div().size_full().child(Editor::new(&self.editor))));
            }
            let kind=self.graph.as_ref().and_then(|g|g.nodes.iter().find(|n|&n.id==id).map(|n|role::role(n,g,&self.roles))).unwrap_or("file");
            panel=panel.child(div().h(px(32.)).flex_none().px(px(10.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between()
                .child(icon_label("node-role",file_icon(kind),role::label(kind),self.role_menu,true).child(icon(Icon::ChevronDown).size(px(12.))).text_color(rgb(role::color(kind))).on_click(cx.listener(|this,_,_,cx|{this.role_menu=!this.role_menu;cx.notify();})))
                .child(caption(if preview{"Rendered Markdown".into()}else{format!("Ln {}, Col {} · UTF-8",cursor.line+1,cursor.character+1)})));
        } else {
            panel=panel.child(div().flex_1().flex().flex_col().items_center().justify_center().gap(px(10.)).text_color(rgb(MUTED))
                .child(icon(Icon::File).size(px(32.))).child("Select a file").child(div().text_size(px(11.)).child("Open a document from the graph or workspace")));
        }
        panel
    }

    fn overlays(&self,window:&Window,cx:&mut Context<Self>)->Option<Div> {
        if !self.palette&&!self.creating&&!self.role_menu&&!self.document_menu&&!self.settings_open{return None;}
        let max_width=f32::from(window.bounds().size.width).min(580.)-48.;
        let mut dialog=div().id("dialog").occlude().w(px(max_width)).max_h(relative(0.85)).bg(rgb(POPOVER)).rounded(px(10.)).border_1().border_color(rgb(BORDER)).shadow_lg().flex().flex_col().overflow_hidden()
            .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation());
        if self.settings_open {
            dialog=dialog.child(self.settings_panel(cx));
        } else if self.palette {
            dialog=dialog.child(div().p(px(14.)).border_b_1().border_color(rgb(BORDER)).child(input("command-query",&self.command_search)));
            let query=self.command_search.read(cx).value().to_lowercase();
            let rows:Vec<_>=Command::ALL.iter().copied().filter(|c|c.label().to_lowercase().contains(&query)).collect();
            let mut list=div().id("command-list").max_h(px(340.)).overflow_y_scroll().p(px(6.));
            for (i,command) in rows.iter().copied().enumerate(){
                let enabled=(!command.needs_workspace()||self.graph.is_some())&&(!matches!(command,Command::Save)||self.selected.is_some());
                list=list.child(icon_label(("command-row",i),command_icon(command),command.label(),i==self.palette_index,enabled).w_full().h(px(34.)).justify_start()
                    .on_click(cx.listener(move|this,_,w,cx|{if enabled{this.command(command,w,cx)}})));
            }
            if rows.is_empty(){list=list.child(div().p(px(20.)).text_color(rgb(MUTED)).child("No matching commands"));}
            dialog=dialog.child(list).child(div().px(px(14.)).py(px(10.)).border_t_1().border_color(rgb(BORDER)).text_size(px(10.)).text_color(rgb(MUTED)).child("↑ ↓ Navigate    Enter Run    Esc Close"))
                .capture_key_down(cx.listener(|this,event:&KeyDownEvent,w,cx|{
                    let query=this.command_search.read(cx).value().to_lowercase();
                    let rows:Vec<_>=Command::ALL.iter().copied().filter(|c|c.label().to_lowercase().contains(&query)).collect();
                    match event.keystroke.key.as_str(){
                        "down"=>{this.palette_index=(this.palette_index+1).min(rows.len().saturating_sub(1));cx.stop_propagation();cx.notify();},
                        "up"=>{this.palette_index=this.palette_index.saturating_sub(1);cx.stop_propagation();cx.notify();},
                        "enter"=>{if let Some(command)=rows.get(this.palette_index).copied(){this.command(command,w,cx);}cx.stop_propagation();},_=>{}
                    }
                }));
        } else if self.creating {
            dialog=dialog.child(div().p(px(20.)).flex().flex_col().gap(px(14.)).child(div().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child("New Skill"))
                .child(div().text_color(rgb(MUTED)).child("Create a folder and SKILL.md. Use lowercase letters, numbers and hyphens."))
                .child(input("new-skill-name",&self.skill_name)).child(div().text_size(px(11.)).text_color(rgb(WARNING)).child(self.status.clone()))
                .child(div().flex().justify_end().gap(px(8.)).child(icon_button("cancel-create",Icon::Close,"Cancel · Esc",false,true).on_click(cx.listener(|this,_,w,cx|this.close_overlay(w,cx))))
                    .child(primary_icon("create-skill",Icon::Check,"Create Skill · Enter").on_click(cx.listener(|this,_,w,cx|this.create_skill(w,cx))))))
                .on_key_down(cx.listener(|this,event:&KeyDownEvent,w,cx|{if event.keystroke.key=="enter"{this.create_skill(w,cx);cx.stop_propagation();}}));
        } else if self.role_menu {
            dialog=dialog.child(div().p(px(16.)).child(caption("NODE APPEARANCE")));
            for (i,kind) in std::iter::once("auto").chain(crate::layout::ROLES.iter().copied()).enumerate(){
                let active=self.selected.as_ref().and_then(|id|self.roles.get(id)).map(|s|s.as_str()).unwrap_or("auto")==kind;
                dialog=dialog.child(icon_label(("role-option",i),if kind=="auto"{Icon::Layers}else{file_icon(kind)},if kind=="auto"{"Auto-detect"}else{role::label(kind)},active,true).mx(px(8.)).mb(px(4.)).justify_start()
                    .on_click(cx.listener(move|this,_,_,cx|{if let Some(id)=&this.selected{if kind=="auto"{this.roles.remove(id);}else{this.roles.insert(id.clone(),kind.into());}this.role_menu=false;this.save_layout(cx);}})));
            }
        } else {
            dialog=dialog.child(div().p(px(16.)).child(caption("Document actions")))
                .child(icon_label("close-current-tab",Icon::Close,"Close current tab",false,self.selected.is_some()).mx(px(8.)).mb(px(6.)).justify_start().on_click(cx.listener(|this,_,w,cx|{this.document_menu=false;if let Some(id)=this.selected.clone(){this.close_tab(id,w,cx);}})))
                .child(icon_label("close-other-saved-tabs",Icon::File,"Close other saved tabs",false,true).mx(px(8.)).mb(px(6.)).justify_start().on_click(cx.listener(|this,_,_,cx|{this.document_menu=false;this.close_other_saved_tabs(cx);})))
                .child(icon_label("doc-reload",Icon::Refresh,"Reload from disk",false,true).mx(px(8.)).mb(px(6.)).justify_start().on_click(cx.listener(|this,_,w,cx|{this.document_menu=false;this.reload(w,cx);})))
                .child(icon_label("doc-discard",Icon::Undo,"Discard unsaved changes…",false,self.dirty(cx)).mx(px(8.)).mb(px(8.)).justify_start().on_click(cx.listener(|this,_,w,cx|{this.document_menu=false;this.discard(w,cx);})));
        }
        Some(div().absolute().inset_0().occlude().bg(rgba(0x00000080)).flex().justify_center().items_start().pt(px(90.)).child(dialog)
            .on_mouse_down(MouseButton::Left,cx.listener(|this,_,w,cx|this.close_overlay(w,cx))))
    }
}

impl Render for Studio {
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
        let width=f32::from(window.viewport_size().width);self.panes.fit(width);
        let guide=self.panes.guide(width);
        let query=self.search.read(cx).value().to_string();
        let visible=self.graph.as_ref().map(|g|filter::visible(g,self.scope,if self.focus{self.selected.as_deref()}else{None},&query,self.show_missing)).unwrap_or_default();
        self.explorer.sync(self.graph_revision,self.graph.as_ref());
        self.explorer.filter(&query,self.focus.then_some(&visible));
        let sidebar=self.sidebar(cx);
        let graph=self.graph_view(&visible,cx);
        let editor=self.document_panel(cx);
        let available=(f32::from(window.viewport_size().height)-70.).max(300.);
        self.terminal_height=crate::terminal::dock_height(self.terminal_height,available);
        let mut document=div().relative().size_full().flex().flex_col().overflow_hidden().child(div().flex_1().min_h(px(0.)).child(editor));
        if self.terminal_visible{
            document=document.child(div().id("terminal-divider").h(px(6.)).w_full().flex_none().cursor_row_resize().bg(rgb(BORDER)).hover(|s|s.bg(rgb(ACCENT)))
                .on_mouse_down(MouseButton::Left,cx.listener(|this,e:&MouseDownEvent,w,cx|{this.terminal_drag=Some(crate::terminal::DockDrag{origin:f32::from(e.position.y),height:this.terminal_height,draft:this.terminal_height});this.focus_handle.focus(w,cx);cx.stop_propagation();cx.notify();})))
                .child(div().h(px(self.terminal_height)).w_full().flex_none().child(self.terminal.clone()))
                .children(self.terminal_drag.as_ref().map(|d|div().absolute().bottom(px(d.draft+2.)).left(px(0.)).right(px(0.)).h(px(2.)).bg(rgb(ACCENT))));
        }
        let overlay=self.overlays(window,cx);
        let explorer_menu=self.explorer_context_menu(window,cx);
        let root=self.graph.as_ref().map(|g|g.root.clone()).unwrap_or_else(||"No workspace open".into());
        let status=if self.panes.dragging(){"Resize preview · release to apply · Esc to cancel".into()}else{self.status.clone()};
        let titlebar=self.titlebar(root,window,cx);
        let resize_events=chrome::resize_events(cx.entity().downgrade());
        div().id("studio-root").relative().size_full().flex().flex_col().bg(rgb(BG)).text_color(rgb(TEXT)).text_size(px(UI_SIZE)).line_height(px(20.)).font_family(UI_FONT)
            .track_focus(&self.focus_handle)
            .on_drop(cx.listener(|this,paths:&ExternalPaths,w,cx|{cx.stop_propagation();this.open_external(paths.paths().to_vec(),w,cx);}))
            .on_drag_move(cx.listener(|this,_:&DragMoveEvent<ExternalPaths>,_,cx|{if !this.dragging_external{this.dragging_external=true;cx.notify();}}))
            .on_file_drop_exit(cx.listener(|this,_,_,cx|{if this.dragging_external{this.dragging_external=false;cx.notify();}}))
            .on_action(cx.listener(|this,_:&TogglePalette,w,cx|this.toggle_palette(w,cx)))
            .on_action(cx.listener(|this,_:&FindDocument,w,cx|this.open_find(w,cx)))
            .on_action(cx.listener(|this,_:&SaveDocument,w,cx|this.save(w,cx)))
            .on_action(cx.listener(|this,_:&CloseDocument,w,cx|{if let Some(id)=this.selected.clone(){this.close_tab(id,w,cx);}}))
            .on_action(cx.listener(|this,_:&TogglePreview,w,cx|this.toggle_preview(w,cx)))
            .on_action(cx.listener(|this,_:&ToggleTerminal,w,cx|this.toggle_terminal(w,cx)))
            .on_action(cx.listener(|this,_:&CloseOverlay,w,cx|{if this.terminal_drag.take().is_some()||this.panes.cancel(){cx.notify();}else{this.close_overlay(w,cx);}}))
            .child(resize_events)
            .child(titlebar)
            .child(div().relative().flex_1().min_h(px(0.)).flex().overflow_hidden()
                .child(div().w(px(self.panes.workspace)).h_full().flex_none().child(sidebar))
                .child(self.pane_handle(crate::panes::Divider::Workspace,"workspace-divider",cx))
                .child(div().flex_1().min_w(px(150.)).h_full().child(graph))
                .child(self.pane_handle(crate::panes::Divider::Editor,"editor-divider",cx))
                .child(div().w(px(self.panes.editor)).h_full().flex_none().child(document))
                .children(guide.map(|x|div().absolute().left(px(x-1.)).top(px(0.)).bottom(px(0.)).w(px(2.)).bg(rgb(ACCENT)))))
            .child(div().h(px(26.)).flex_none().px(px(12.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().gap(px(8.)).bg(rgb(PANEL))
                .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(if self.conflict{WARNING}else{GREEN})))
                .child(div().flex_1().truncate().text_size(px(11.5)).text_color(rgb(MUTED)).child(status))
                .child(caption(if self.watcher.is_some(){"Live index · Local first"}else{"Local first · Explicit terminal"})))
            .children(self.dragging_external.then(||div().id("external-drop-target").absolute().inset_0().occlude()
                .on_drop(cx.listener(|this,paths:&ExternalPaths,w,cx|{cx.stop_propagation();this.open_external(paths.paths().to_vec(),w,cx);}))
                .bg(rgba(0x101a2ba8)).flex().items_center().justify_center()
                .child(div().mx(px(20.)).p(px(28.)).rounded(px(14.)).border_1().border_color(rgb(ACCENT)).bg(rgb(POPOVER)).shadow_lg().flex().flex_col().items_center().gap(px(12.))
                    .child(icon(Icon::FolderOpen).size(px(36.)).text_color(rgb(ACCENT)))
                    .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child("拖入文件或文件夹"))
                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child("文件夹打开工作区 · 文件按内容类型打开"))
                    .child(div().text_size(px(11.)).text_color(rgb(GREEN)).child("只打开 · 不复制 · 不自动执行")))))
            .children(overlay)
            .children(explorer_menu)
    }
}
