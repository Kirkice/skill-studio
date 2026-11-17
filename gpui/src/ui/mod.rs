mod controls;
pub mod theme;
pub mod icons;
mod graph;
mod settings;
use crate::{commands::Command, filter, role, Studio, TogglePalette, SaveDocument, CloseOverlay, FindDocument};
use controls::*;
use icons::{icon,Icon,file_icon,command_icon};
use theme::*;
use gpui::*;
use theme::rgb;
use gpui_base::{Editor, InputBase, h_resizable, resizable_panel};
use std::collections::HashSet;

impl Studio {
    fn sidebar(&self, visible: &HashSet<String>, cx: &mut Context<Self>) -> Div {
        let name = self.graph.as_ref().and_then(|g|std::path::Path::new(&g.root).file_name()).map(|s|s.to_string_lossy().into_owned()).unwrap_or_else(||"Local workspace".into());
        let header = div().p(px(12.)).flex().flex_col().gap(px(12.))
            .child(div().flex().items_center().justify_between().child(caption("WORKSPACE")).child(icon_button("sidebar-open",Icon::FolderOpen,"Open workspace",false,true).on_click(cx.listener(|this,_,w,cx|this.open(w,cx)))))
            .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).truncate().child(name))
            .child(input("file-search",&self.search));
        let mut list=div().id("file-list").flex_1().min_h(px(0.)).overflow_y_scroll().px(px(6.)).pb(px(8.));
        if let Some(graph)=&self.graph {
            let count=graph.nodes.iter().filter(|n|n.kind=="skill").count();
            list=list.child(div().px(px(8.)).py(px(10.)).flex().justify_between().child(caption("SKILLS & FILES")).child(caption(format!("{count} skills"))));
            for (i,node) in graph.nodes.iter().enumerate().filter(|(_,n)|n.kind!="missing"&&visible.contains(&n.id)) {
                let id=node.id.clone(); let selected=self.selected.as_deref()==Some(&id);
                let kind=role::role(node,graph,&self.roles);
                let title=if node.title.is_empty(){node.label.clone()}else{node.title.clone()};
                list=list.child(div().id(("file",i)).px(px(10.)).py(px(8.)).mb(px(2.)).rounded(px(5.)).flex().gap(px(9.)).cursor_pointer()
                    .bg(rgb(if selected {SELECTED}else{PANEL})).hover(|s|s.bg(rgb(HOVER)))
                    .child(icon(file_icon(kind)).size(px(14.)).text_color(rgb(MUTED)).mt(px(2.)))
                    .child(div().flex_1().min_w(px(0.)).flex().flex_col().gap(px(3.)).child(div().truncate().text_size(px(13.)).child(title)).child(div().truncate().text_size(px(11.5)).text_color(rgb(DIM)).child(node.id.clone())))
                    .on_click(cx.listener(move|this,_,w,cx|this.select(id.clone(),w,cx))));
            }
            if visible.is_empty(){list=list.child(div().p(px(16.)).text_color(rgb(MUTED)).child("No matching files"));}
        } else {list=list.child(div().p(px(16.)).text_size(px(12.)).text_color(rgb(MUTED)).child("Open a folder to explore Skills and local references."));}
        div().size_full().bg(rgb(PANEL)).flex().flex_col().child(header).child(list)
            .child(div().p(px(10.)).border_t_1().border_color(rgb(BORDER)).child(icon_label("new-skill",Icon::Plus,"New Skill",false,self.graph.is_some()).w_full().on_click(cx.listener(|this,_,w,cx|this.command(Command::Create,w,cx)))))
    }

    fn document_panel(&self,cx:&mut Context<Self>)->Div {
        let dirty=self.dirty(cx);
        let cursor=self.editor.read(cx).cursor_position();
        let mut panel=div().size_full().bg(rgb(BG)).flex().flex_col();
        let title=self.selected.as_deref().and_then(|s|s.rsplit('/').next()).unwrap_or("Document").to_string();
        panel=panel.child(bar().bg(rgb(PANEL)).child(div().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(if dirty{WARNING}else{GREEN})))
            .child(div().flex_1().truncate().text_size(px(12.8)).child(title))
            .child(icon_button("document-save",Icon::Save,if cfg!(target_os="macos"){"Save document · Cmd+S"}else{"Save document · Ctrl+S"},dirty,self.selected.is_some()&&dirty).on_click(cx.listener(|this,_,_,cx|{if this.dirty(cx){this.save(cx)}})))
            .child(icon_button("editor-find",Icon::Search,"Find in document · Ctrl/Cmd+F",self.find_open,self.selected.is_some()).on_click(cx.listener(|this,_,w,cx|this.open_find(w,cx))))
            .child(icon_button("editor-wrap",Icon::Wrap,"Toggle word wrap",self.preferences.soft_wrap,self.selected.is_some()).on_click(cx.listener(|this,_,w,cx|{this.preferences.soft_wrap=!this.preferences.soft_wrap;this.apply_preferences(w,cx);})))
            .child(icon_button("editor-numbers",Icon::Numbers,"Toggle line numbers",self.preferences.line_numbers,self.selected.is_some()).on_click(cx.listener(|this,_,w,cx|{this.preferences.line_numbers=!this.preferences.line_numbers;this.apply_preferences(w,cx);})))
            .child(icon_button("document-menu",Icon::More,"Document actions",false,self.selected.is_some()).on_click(cx.listener(|this,_,_,cx|{if this.selected.is_some(){this.document_menu=!this.document_menu;cx.notify();}}))));
        if let Some(id)=&self.selected {
            panel=panel.child(div().px(px(14.)).py(px(8.)).text_size(px(11.5)).text_color(rgb(DIM)).truncate().child(id.clone()));
            if self.conflict {
                panel=panel.child(div().mx(px(12.)).mb(px(8.)).p(px(10.)).rounded(px(6.)).bg(rgb(0x393126)).flex().flex_col().gap(px(8.)).text_color(rgb(WARNING))
                    .child("This file changed on disk. Your edits are preserved.")
                    .child(button("reload-document","Reload from disk",false,true).on_click(cx.listener(|this,_,w,cx|this.reload(w,cx)))));
            }
            if self.find_open {
                let count=self.editor.read(cx).search_session().matcher.label();
                panel=panel.child(div().px(px(10.)).pb(px(8.)).flex().flex_col().gap(px(6.))
                    .child(div().flex().items_center().gap(px(4.)).child(div().flex_1().min_w(px(0.)).child(input("document-find",&self.find_query))).child(caption(count))
                        .child(icon_button("find-previous",Icon::ChevronDown,"Previous match",false,true).on_click(cx.listener(|this,_,_,cx|{this.editor.update(cx,|s,cx|s.previous_search_match(cx));})))
                        .child(icon_button("find-next",Icon::ArrowRight,"Next match",false,true).on_click(cx.listener(|this,_,_,cx|{this.editor.update(cx,|s,cx|s.next_search_match(cx));})))
                        .child(icon_button("close-find",Icon::Close,"Close find",false,true).on_click(cx.listener(|this,_,w,cx|this.close_find(w,cx)))))
                    .child(div().flex().items_center().gap(px(4.)).child(div().flex_1().min_w(px(0.)).child(input("document-replace",&self.replace_query)))
                        .child(button("replace-one","Replace",false,true).on_click(cx.listener(|this,_,w,cx|{let replacement=this.replace_query.read(cx).value().to_string();this.editor.update(cx,|s,cx|s.replace_current_search_match(&replacement,w,cx));})))
                        .child(button("replace-all","All",false,true).on_click(cx.listener(|this,_,w,cx|{let replacement=this.replace_query.read(cx).value().to_string();let count=this.editor.update(cx,|s,cx|s.replace_all_search_matches(&replacement,w,cx));this.status=format!("Replaced {count} matches (not saved)");cx.notify();}))))
                    .capture_key_down(cx.listener(|this,event:&KeyDownEvent,_,cx|{if event.keystroke.key=="enter"{this.editor.update(cx,|s,cx|s.next_search_match(cx));cx.stop_propagation();}})));
            }
            let editor=self.editor.clone();
            panel=panel.child(InputBase::new("source-editor").flex_1().min_h(px(0.)).w_full().overflow_hidden().bg(rgb(BG)).font_family(MONO_FONT).text_size(px(self.preferences.font_size as f32)).line_height(px(self.preferences.font_size as f32*1.65))
                .on_mouse_down(MouseButton::Left,move|_,w,cx|editor.update(cx,|s,cx|s.focus(w,cx))).child(div().size_full().child(Editor::new(&self.editor))));
            let kind=self.graph.as_ref().and_then(|g|g.nodes.iter().find(|n|&n.id==id).map(|n|role::role(n,g,&self.roles))).unwrap_or("file");
            panel=panel.child(div().h(px(32.)).flex_none().px(px(10.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between()
                .child(icon_label("node-role",file_icon(kind),role::label(kind),self.role_menu,true).child(icon(Icon::ChevronDown).size(px(12.))).text_color(rgb(role::color(kind))).on_click(cx.listener(|this,_,_,cx|{this.role_menu=!this.role_menu;cx.notify();})))
                .child(caption(format!("Ln {}, Col {} · UTF-8",cursor.line+1,cursor.character+1))));
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
                .child(div().flex().justify_end().gap(px(8.)).child(button("cancel-create","Cancel",false,true).on_click(cx.listener(|this,_,w,cx|this.close_overlay(w,cx))))
                    .child(primary("create-skill","Create Skill").on_click(cx.listener(|this,_,w,cx|this.create_skill(w,cx))))))
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
                .child(icon_label("doc-reload",Icon::Refresh,"Reload from disk",false,true).mx(px(8.)).mb(px(6.)).justify_start().on_click(cx.listener(|this,_,w,cx|{this.document_menu=false;this.reload(w,cx);})))
                .child(icon_label("doc-discard",Icon::Undo,"Discard unsaved changes…",false,self.dirty(cx)).mx(px(8.)).mb(px(8.)).justify_start().on_click(cx.listener(|this,_,w,cx|{this.document_menu=false;this.discard(w,cx);})));
        }
        Some(div().absolute().inset_0().occlude().bg(rgba(0x00000080)).flex().justify_center().items_start().pt(px(90.)).child(dialog)
            .on_mouse_down(MouseButton::Left,cx.listener(|this,_,w,cx|this.close_overlay(w,cx))))
    }
}

impl Render for Studio {
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
        let query=self.search.read(cx).value().to_string();
        let visible=self.graph.as_ref().map(|g|filter::visible(g,self.scope,if self.focus{self.selected.as_deref()}else{None},&query,self.show_missing)).unwrap_or_default();
        let sidebar=self.sidebar(&visible,cx);
        let graph=self.graph_view(&visible,cx);
        let editor=self.document_panel(cx);
        let overlay=self.overlays(window,cx);
        let root=self.graph.as_ref().map(|g|g.root.clone()).unwrap_or_else(||"No workspace open".into());
        let status=self.status.clone();
        div().relative().size_full().flex().flex_col().bg(rgb(BG)).text_color(rgb(TEXT)).text_size(px(UI_SIZE)).line_height(px(20.)).font_family(UI_FONT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this,_:&TogglePalette,w,cx|this.toggle_palette(w,cx)))
            .on_action(cx.listener(|this,_:&FindDocument,w,cx|this.open_find(w,cx)))
            .on_action(cx.listener(|this,_:&SaveDocument,_,cx|this.save(cx)))
            .on_action(cx.listener(|this,_:&CloseOverlay,w,cx|this.close_overlay(w,cx)))
            .child(div().h(px(42.)).flex_none().px(px(14.)).flex().items_center().gap(px(10.)).border_b_1().border_color(rgb(BORDER)).bg(rgb(PANEL))
                .child(icon(Icon::Skill).size(px(20.)).text_color(rgb(ACCENT)))
                .child(div().text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child("skill studio"))
                .child(separator()).child(div().flex_1().min_w(px(0.)).truncate().text_color(rgb(DIM)).text_size(px(12.)).child(root))
                .child(icon_button("command-trigger",Icon::Command,if cfg!(target_os="macos"){"Command palette · Cmd+Shift+P"}else{"Command palette · Ctrl+Shift+P"},self.palette,true).on_click(cx.listener(|this,_,w,cx|this.toggle_palette(w,cx))))
                .child(icon_button("settings-trigger",Icon::Settings,"Settings",self.settings_open,true).on_click(cx.listener(|this,_,w,cx|this.open_settings(w,cx)))))
            .child(div().flex_1().min_h(px(0.)).child(h_resizable("studio-panes")
                .child(resizable_panel().size(px(250.)).size_range(px(190.)..px(420.)).child(sidebar))
                .child(resizable_panel().size_range(px(300.)..px(10000.)).child(graph))
                .child(resizable_panel().size(px(480.)).size_range(px(300.)..px(1000.)).child(editor))))
            .child(div().h(px(26.)).flex_none().px(px(12.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().gap(px(8.)).bg(rgb(PANEL))
                .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(if self.conflict{WARNING}else{GREEN})))
                .child(div().flex_1().truncate().text_size(px(11.5)).text_color(rgb(MUTED)).child(status))
                .child(caption(if self.watcher.is_some(){"Live index · Local first"}else{"Local first · Scripts never run"})))
            .children(overlay)
    }
}
