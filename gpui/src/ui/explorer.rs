use crate::Studio;
use super::{controls::*,icons::{icon,Icon},theme::*,theme::rgb};
use gpui::*;
use crate::file_ops::Op;
actions!(explorer_keys,[CopyEntry,PasteEntry,RenameEntry,DeleteEntry]);
pub(crate) fn bind_keys(cx:&mut App){
    let m=if cfg!(target_os="macos"){"cmd"}else{"ctrl"};
    cx.bind_keys([KeyBinding::new(&format!("{m}-c"),CopyEntry,Some("ExplorerFiles")),KeyBinding::new(&format!("{m}-v"),PasteEntry,Some("ExplorerFiles")),KeyBinding::new("f2",RenameEntry,Some("ExplorerFiles")),KeyBinding::new("delete",DeleteEntry,Some("ExplorerFiles"))]);
}
#[derive(Clone)]
struct EntryDrag {root:String,paths:std::sync::Arc<Vec<String>>}
fn batch_roots(mut paths:Vec<String>)->Vec<String>{
    paths.retain(|p|!p.is_empty());
    paths.sort_unstable();paths.dedup();
    let all:std::collections::HashSet<_>=paths.iter().map(String::as_str).collect();
    paths.iter().filter(|path|!path.match_indices('/').any(|(i,_)|all.contains(&path[..i]))).cloned().collect()
}
fn parent(path:&str)->String{path.rsplit_once('/').map_or("",|(dir,_)|dir).into()}
use gpui::prelude::FluentBuilder;

fn context_origin(x:f32,y:f32,width:f32,height:f32)->(f32,f32){
    (x.clamp(8.,(width-246.-8.).max(8.)),y.clamp(8.,(height-216.-8.).max(8.)))
}

impl Studio {
    fn select_explorer_range(&mut self,path:&str,m:Modifiers,w:&mut Window,cx:&mut Context<Self>)->bool{
        let toggle=m.control||m.platform;
        if m.shift{
            let paths:Vec<_>=self.explorer.rows.iter().map(|&i|self.explorer.entries[i].path.as_str()).collect();
            let end=paths.iter().position(|&id|id==path);
            let start=self.explorer_selection.as_deref().and_then(|a|paths.iter().position(|&id|id==a)).or(end);
            if !toggle{self.explorer_multi.clear();}
            if let (Some(a),Some(b))=(start,end){self.explorer_multi.extend(paths[a.min(b)..=a.max(b)].iter().map(|p|p.to_string()));}
            if self.explorer_selection.is_none(){self.explorer_selection=Some(path.into());}
        }else if toggle{
            if !self.explorer_multi.insert(path.into()){self.explorer_multi.remove(path);}
            self.explorer_selection=Some(path.into());
        }else{
            self.explorer_multi.clear();self.explorer_multi.insert(path.into());self.explorer_selection=Some(path.into());
        }
        self.explorer_focus.focus(w,cx);cx.notify();
        m.shift||toggle
    }
    fn selected_entries(&self)->Vec<String>{
        batch_roots(self.explorer.entries.iter().filter(|e|self.explorer_multi.contains(&e.path)).map(|e|e.path.clone()).collect())
    }
    fn copy_entries(&mut self,paths:Vec<String>,cx:&mut Context<Self>){
        if !paths.is_empty()&&let Some(g)=&self.graph {
            self.status=format!("Copied {} item(s)",paths.len());self.explorer_copy=Some((g.root.clone(),paths));cx.notify();
        }
    }
    fn paste_entry(&mut self,target:String,w:&mut Window,cx:&mut Context<Self>){
        let Some((root,paths))=self.explorer_copy.clone() else{return;};
        if self.graph.as_ref().is_none_or(|g|g.root!=root){return;}
        let dir=if self.explorer.entries.iter().any(|e|e.path==target&&e.directory){target}else{parent(&target)};
        self.file_operations(paths,Op::Copy(dir),w,cx);
    }
    fn file_operations(&mut self,paths:Vec<String>,op:Op,w:&mut Window,cx:&mut Context<Self>){
        if paths.is_empty()||self.workspace_file.is_some()||self.saving||!self.composition_ready(w,cx){return;}
        let Some(root)=self.graph.as_ref().map(|g|g.root.clone())else{return;};
        let copy=matches!(&op,Op::Copy(_));
        if let Op::Move(dir)=&op && paths.iter().any(|p|dir==p||dir.strip_prefix(p).is_some_and(|s|s.starts_with('/'))){
            self.status="Cannot move a folder into itself or its descendants".into();cx.notify();return;
        }
        if !copy&&!self.guard(cx){return;}
        if !copy{
            let affected:Vec<_>=self.documents.tabs.iter().filter(|t|paths.iter().any(|path|t.id==*path||t.id.strip_prefix(path).is_some_and(|s|s.starts_with('/')))).map(|t|t.id.clone()).collect();
            for id in affected{self.remove_tab(&id,w,cx);}
        }
        self.open_ticket+=1;self.saving=true;self.status="Updating files…".into();cx.notify();
        cx.spawn_in(w,async move|this,cx|{
            let (targets,errors)=cx.background_executor().spawn(async move{
                let mut targets=vec![];let mut errors=vec![];
                for path in paths{
                    match crate::file_ops::run(std::path::Path::new(&root),&path,op.clone()){
                        Ok(Some(target))=>targets.push(target),Ok(None)=>{},Err(e)=>errors.push(format!("{path}: {e}")),
                    }
                }
                (targets,errors)
            }).await;
            let _=this.update_in(cx,|this,w,cx|{
                this.saving=false;
                this.explorer_selection=targets.first().cloned();this.explorer_multi=targets.into_iter().collect();
                this.refresh(w,cx);
                if !errors.is_empty(){
                    let mut detail=errors.iter().take(8).cloned().collect::<Vec<_>>().join("\n");
                    if errors.len()>8{detail.push_str(&format!("\n… and {} more",errors.len()-8));}
                    let _=w.prompt(PromptLevel::Warning,"Some items could not be processed",Some(&detail),&[PromptButton::ok("OK")],cx);
                }
                cx.notify();
            });
        }).detach();
    }
    fn begin_rename(&mut self,root:String,path:String,w:&mut Window,cx:&mut Context<Self>){
        let name=path.rsplit('/').next().unwrap_or(&path).to_owned();
        self.explorer_menu=None;self.explorer_rename=Some((root,path));
        self.skill_name.update(cx,|input,cx|{input.set_value(name,w,cx);input.focus(w,cx);input.select_all(w,cx);});cx.notify();
    }
    fn confirm_rename(&mut self,w:&mut Window,cx:&mut Context<Self>){
        if self.skill_name.update(cx,|input,cx|input.marked_text_range(w,cx).is_some()){return;}
        if let Some((root,path))=self.explorer_rename.take(){
            let name=self.skill_name.read(cx).value().to_string();
            self.skill_name.update(cx,|input,cx|input.set_value("",w,cx));
            self.explorer_focus.focus(w,cx);
            if self.graph.as_ref().is_some_and(|g|g.root==root){self.file_operations(vec![path],Op::Rename(name),w,cx);}
            cx.notify();
        }
    }
    fn delete_entries(&mut self,root:String,paths:Vec<String>,w:&mut Window,cx:&mut Context<Self>){
        if paths.is_empty(){return;}
        let detail=if paths.len()==1{paths[0].clone()}else{format!("{} selected items:\n{}",paths.len(),paths.iter().take(8).cloned().collect::<Vec<_>>().join("\n"))};
        let answer=w.prompt(PromptLevel::Warning,"Permanently delete the selected items?",Some(&detail),&[PromptButton::ok("Delete"),PromptButton::cancel("Cancel")],cx);
        cx.spawn_in(w,async move|this,cx|{
            if answer.await.ok()==Some(0){let _=this.update_in(cx,|this,w,cx|if this.graph.as_ref().is_some_and(|g|g.root==root){this.file_operations(paths,Op::Delete,w,cx);});}
        }).detach();
    }
    pub(crate) fn close_explorer_menu(&mut self,window:&mut Window,cx:&mut Context<Self>){
        self.explorer_menu=None;
        if self.active_text()&&!self.is_preview(){self.editor.update(cx,|e,cx|e.focus(window,cx));}else{self.focus_handle.focus(window,cx);}
        cx.notify();
    }
    pub(crate) fn reveal_in_file_manager(&mut self,path:std::path::PathBuf,window:&mut Window,cx:&mut Context<Self>){
        #[cfg(target_os="windows")]{
            self.status="正在资源管理器中定位…".into();cx.notify();
            cx.spawn_in(window,async move|this,cx|{
                let result=cx.background_executor().spawn(async move{crate::files::reveal_windows(path)}).await;
                let _=this.update_in(cx,|this,_,cx|{this.status=match result{Ok(())=>"已在资源管理器中显示".into(),Err(e)=>format!("无法在资源管理器中定位：{e}")};cx.notify();});
            }).detach();
        }
        #[cfg(not(target_os="windows"))]{let _=window;cx.reveal_path(&path);}
    }
    pub(super) fn explorer_context_menu(&self,window:&Window,cx:&mut Context<Self>)->Option<Stateful<Div>>{
        let menu=self.explorer_menu.as_ref()?;
        let (x,y)=context_origin(menu.x,menu.y,f32::from(window.viewport_size().width),f32::from(window.viewport_size().height));
        let name=menu.path.rsplit('/').next().unwrap_or(&menu.path).to_string();
        let root=menu.root.clone();let path=menu.path.clone();
        let operations=self.workspace_file.is_none()&&!self.saving;
        let can_paste=operations&&self.explorer_copy.as_ref().is_some_and(|(r,_)|r==&root);
        let paths=self.selected_entries();
        let single=operations&&self.explorer_multi.len()==1;
        let action=|id,icon,label,enabled|icon_label(id,icon,label,false,enabled).w_full().h(px(28.)).justify_start()
            .bg(rgb(POPOVER)).hover(move|s|s.bg(rgb(if enabled{HOVER}else{POPOVER})));
        Some(div().id("explorer-context-menu").absolute().left(px(x)).top(px(y)).w(px(246.)).p(px(6.)).rounded(px(8.)).border_1().border_color(rgb(BORDER)).bg(rgb(POPOVER)).shadow_lg().occlude()
            .child(div().h(px(24.)).px(px(8.)).truncate().text_size(px(11.)).text_color(rgb(DIM)).child(name))
            .child(div().h(px(1.)).my(px(4.)).bg(rgb(BORDER)))
            .child(action("explorer-rename-action",Icon::Replace,crate::i18n::text("Rename"),single).on_click(cx.listener({let (root,path)=(root.clone(),path.clone());move|this,_,w,cx|if single{this.begin_rename(root.clone(),path.clone(),w,cx);}})))
            .child(action("explorer-delete",Icon::Close,crate::i18n::text("Delete"),operations).on_click(cx.listener({let (root,paths)=(root.clone(),paths.clone());move|this,_,w,cx|if operations{this.explorer_menu=None;this.delete_entries(root.clone(),paths.clone(),w,cx);cx.notify();}})))
            .child(action("explorer-copy",Icon::Copy,crate::i18n::text("Copy"),operations).on_click(cx.listener(move|this,_,_,cx|if operations{this.explorer_menu=None;this.copy_entries(paths.clone(),cx); })))
            .child(action("explorer-paste",Icon::Paste,crate::i18n::text("Paste"),can_paste).on_click(cx.listener({let path=path.clone();move|this,_,w,cx|if can_paste{this.explorer_menu=None;this.paste_entry(path.clone(),w,cx);cx.notify();}})))
            .child(icon_label("explorer-show-in-folder",Icon::FolderOpen,if cfg!(target_os="macos"){crate::i18n::text("在 Finder 中显示")}else{crate::i18n::text("在资源管理器中显示")},false,true).w_full().h(px(32.)).justify_start().bg(rgb(POPOVER))
                .on_click(cx.listener(move|this,_,w,cx|{
                    this.close_explorer_menu(w,cx);
                    if this.graph.as_ref().map(|g|g.root.as_str())!=Some(root.as_str()){return;}
                    match crate::core::safe(std::path::Path::new(&root),&path){
                        Ok(path)=>this.reveal_in_file_manager(path,w,cx),
                        Err(e)=>{this.status=format!("无法定位文件或文件夹：{e}");cx.notify();}
                    }
                })))
            .on_mouse_down_out(cx.listener(|this,_,_,cx|{this.explorer_menu=None;cx.notify();})))
    }
    pub(super) fn sidebar(&self,cx:&mut Context<Self>)->Div{
        let name=self.graph.as_ref().and_then(|g|std::path::Path::new(&g.root).file_name()).map(|s|s.to_string_lossy().into_owned()).unwrap_or_else(||"Local workspace".into());
        let header=div().px(px(14.)).pt(px(12.)).pb(px(10.)).flex().flex_col().gap(px(10.))
            .child(div().h(px(24.)).flex().items_center().justify_between().child(caption(crate::i18n::text("EXPLORER"))))
            .child(div().flex().items_center().gap(px(8.)).child(icon(Icon::FolderOpen).size(px(15.)).text_color(rgb(ACCENT)))
                .child(div().flex_1().min_w(px(0.)).truncate().font_weight(FontWeight::SEMIBOLD).text_size(px(13.)).child(name)))
            .child(input("file-search",&self.search));
        let actions=div().h(px(32.)).px(px(12.)).flex_none().flex().items_center().gap(px(4.))
            .child(caption(crate::i18n::text("FILES")))
            .child(div().flex_1().text_size(px(10.)).text_color(rgb(DIM)).child(format!("{}",self.explorer.matched_files)))
            .child(icon_button("explorer-refresh",Icon::Refresh,if self.refreshing{"正在刷新文件树和 Graph…"}else{"刷新文件树和 Graph（保留编辑器内容）"},self.refreshing,self.graph.is_some()&&!self.refreshing&&!self.saving).w(px(23.)).h(px(23.))
                .on_click(cx.listener(|this,_,w,cx|{if this.graph.is_some()&&!this.refreshing&&!this.saving{this.status="Refreshing files and graph…".into();this.refresh(w,cx);cx.notify();}})))
            .child(icon_button("explorer-reveal",Icon::Focus,"Reveal active file",false,self.selected.is_some()).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,_,cx|{if let Some(id)=&this.selected{this.explorer.reveal(id);cx.notify();}})))
            .child(icon_button("explorer-collapse",Icon::CollapseAll,"Collapse all folders",false,self.graph.is_some()).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,_,cx|{this.explorer.collapse_all();cx.notify();})));
        let mut list=div().id("file-list").flex_1().min_h(px(0.)).overflow_y_scroll().pb(px(8.));
        let root=self.graph.as_ref().map(|g|g.root.clone()).unwrap_or_default();
        let enabled=self.workspace_file.is_none()&&!root.is_empty();
        if enabled{
            list=list.track_focus(&self.explorer_focus).key_context("ExplorerFiles")
                .on_action(cx.listener(|this,_:&DeleteEntry,w,cx|{
                    cx.stop_propagation();
                    if !this.saving&&this.explorer_rename.is_none()
                        &&let Some(root)=this.graph.as_ref().map(|g|g.root.clone()){
                        this.delete_entries(root,this.selected_entries(),w,cx);
                    }
                }))
                .on_action(cx.listener(|this,_:&RenameEntry,w,cx|{if this.explorer_multi.len()==1&&!this.saving&&let Some(root)=this.graph.as_ref().map(|g|g.root.clone())&&let Some(path)=this.explorer_multi.iter().next().cloned(){this.begin_rename(root,path,w,cx);}cx.stop_propagation();}))
                .on_action(cx.listener(|this,_:&CopyEntry,_,cx|{this.copy_entries(this.selected_entries(),cx);cx.stop_propagation();}))
                .on_action(cx.listener(|this,_:&PasteEntry,w,cx|{if this.explorer_multi.len()==1{this.paste_entry(this.explorer_multi.iter().next().unwrap().clone(),w,cx);}else if this.explorer_selection.as_deref()==Some(""){this.paste_entry(String::new(),w,cx);}cx.stop_propagation();}))
                .child(div().id("explorer-root-drop").px(px(13.)).h(px(23.)).flex().items_center().text_size(px(10.)).text_color(rgb(DIM)).child(crate::i18n::text("WORKSPACE ROOT"))
                    .on_click(cx.listener(|this,_,w,cx|{this.explorer_multi.clear();this.explorer_selection=Some(String::new());this.explorer_focus.focus(w,cx);cx.notify();}))
                    .on_drop(cx.listener(|this,drag:&EntryDrag,w,cx|{if this.graph.as_ref().is_some_and(|g|g.root==drag.root){this.file_operations(drag.paths.as_ref().clone(),Op::Move(String::new()),w,cx);}cx.stop_propagation();})));
        }
        let drag_paths=std::sync::Arc::new(self.selected_entries());
        for &index in &self.explorer.rows{
            let entry=&self.explorer.entries[index];
            let path=entry.path.clone();let context_path=path.clone();let title=entry.title.clone();
            let selected=if let Some(menu)=&self.explorer_menu{menu.path==entry.path||self.explorer_multi.contains(&entry.path)}else{self.explorer_multi.contains(&entry.path)||(self.explorer_selection.is_none()&&!entry.directory&&self.selected.as_deref()==Some(&entry.path))};
            let expanded=entry.directory&&self.explorer.expanded(&entry.path);
            let glyph=if entry.directory{if expanded{Icon::FolderOpen}else{Icon::Folder}}else if entry.kind=="skill"{Icon::Skill}else if entry.kind=="script"||matches!(entry.name.rsplit('.').next(),Some("json"|"toml"|"yaml"|"yml"|"css"|"html")){Icon::Code}else{Icon::File};
            let color=if entry.directory{MUTED}else if entry.kind=="skill"{ACCENT}else if matches!(glyph,Icon::Code){WARNING}else if entry.name.ends_with(".md"){0x89aacd}else{DIM};
            let hint:SharedString=if entry.directory{format!("{} · {} files\n{}",if expanded{"Collapse folder"}else{"Expand folder"},entry.files,path).into()}else if title.is_empty(){path.clone().into()}else{format!("{path}\n{title}").into()};
            let mut row=div().id(("explorer-entry",index)).relative().h(px(30.)).mx(px(6.)).mb(px(1.)).pr(px(8.)).pl(px(8.+entry.depth as f32*16.)).rounded(px(4.)).flex().items_center().gap(px(5.)).overflow_hidden()
                .aria_label(hint.clone()).bg(rgb(if selected{SELECTED}else{PANEL})).hover(|s|s.bg(rgb(if selected{SELECTED}else{HOVER})))
                .tooltip(move|_,cx|cx.new(|_|Hint(hint.clone())).into())
                .on_mouse_down(MouseButton::Right,cx.listener(move|this,event:&MouseDownEvent,w,cx|{
                    cx.stop_propagation();w.prevent_default();
                    if !this.composition_ready(w,cx){return;}
                    if let Some(graph)=&this.graph{this.explorer_menu=Some(crate::explorer::ContextMenu{root:graph.root.clone(),path:context_path.clone(),x:f32::from(event.position.x),y:f32::from(event.position.y)});if !this.explorer_multi.contains(&context_path){this.explorer_multi.clear();this.explorer_multi.insert(context_path.clone());}this.explorer_selection=Some(context_path.clone());this.explorer_focus.focus(w,cx);cx.notify();}
                }));
            let renaming=self.explorer_rename.as_ref().is_some_and(|(r,id)|r==&root&&id==&path);
            if enabled&&!renaming{
                let paths=if self.explorer_multi.contains(&path){drag_paths.clone()}else{std::sync::Arc::new(vec![path.clone()])};
                row=row.on_drag(EntryDrag{root:root.clone(),paths},|drag,_,_,cx|cx.new(|_|Hint(if drag.paths.len()==1{drag.paths[0].clone().into()}else{format!("{} items",drag.paths.len()).into()})));
                if entry.directory{
                    let folder=path.clone();
                    row=row.on_drop(cx.listener(move|this,drag:&EntryDrag,w,cx|{if this.graph.as_ref().is_some_and(|g|g.root==drag.root){this.file_operations(drag.paths.as_ref().clone(),Op::Move(folder.clone()),w,cx);}cx.stop_propagation();}));
                }
            }
            for level in 0..entry.depth{
                row=row.child(div().absolute().left(px(13.+level as f32*16.)).top(px(0.)).bottom(px(0.)).w(px(1.)).bg(rgb(BORDER)));
            }
            row=row.children(selected.then(||div().absolute().left(px(0.)).top(px(5.)).bottom(px(5.)).w(px(2.)).rounded_full().bg(rgb(ACCENT))))
                .child(if entry.directory{icon(if expanded{Icon::ChevronDown}else{Icon::ChevronRight}).size(px(12.)).text_color(rgb(DIM)).into_any_element()}else{div().w(px(12.)).flex_none().into_any_element()})
                .child(icon(glyph).size(px(15.)).flex_none().text_color(rgb(color)))
                .child(div().flex_1().min_w(px(0.)).truncate().text_size(px(12.5)).text_color(rgb(if selected{TEXT}else if entry.directory{MUTED}else{TEXT})).when(entry.kind=="skill",|s|s.font_weight(FontWeight::MEDIUM)).child(if renaming{
                    div().id("explorer-rename-inline").w_full().h(px(24.)).rounded(px(3.)).border_1().border_color(rgb(ACCENT))
                        .child(gpui_base::InputBase::new("explorer-rename-input").size_full().px(px(4.))
                            .child(div().size_full().flex().items_center().child(gpui_base::Input::new(&self.skill_name))))
                        .on_mouse_down_out(cx.listener(|this,_,w,cx|{
                            if this.explorer_rename.take().is_some(){
                                this.skill_name.update(cx,|input,cx|input.set_value("",w,cx));cx.notify();
                            }
                        }))
                        .capture_key_down(cx.listener(|this,e:&KeyDownEvent,w,cx|{
                            if e.keystroke.key=="enter"&&!this.skill_name.update(cx,|input,cx|input.marked_text_range(w,cx).is_some()){
                                w.prevent_default();cx.stop_propagation();this.confirm_rename(w,cx);
                            }
                        })).into_any_element()
                }else{entry.name.clone().into_any_element()}));
            if renaming{list=list.child(row);continue;}
            if entry.directory{
                row=row.child(div().flex_none().text_size(px(10.)).text_color(rgb(DIM)).child(format!("{}",entry.files)))
                    .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|{if this.select_explorer_range(&path,event.modifiers(),w,cx){return;}if event.click_count()==1{this.explorer.toggle(&path);}cx.notify();}));
            }else{
                row=row.children(self.documents.get(&path).is_some_and(|tab|tab.dirty).then(||div().flex_none().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(WARNING))))
                    .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|{if this.select_explorer_range(&path,event.modifiers(),w,cx){return;}this.select_file(path.clone(),event.click_count()>1,w,cx);if event.click_count()==1{this.explorer_focus.focus(w,cx);cx.notify();}}));
            }
            list=list.child(row);
        }
        if self.explorer.rows.is_empty(){list=list.child(div().p(px(18.)).text_size(px(12.)).text_color(rgb(DIM)).child(if self.graph.is_some(){crate::i18n::text("No matching files")}else{crate::i18n::text("Open a folder to explore its files and Skills.")}));}
        div().size_full().min_w(px(0.)).overflow_hidden().bg(rgb(PANEL)).flex().flex_col().child(header).child(actions).child(list)
            .child(div().h(px(38.)).px(px(12.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between()
                .child(div().text_size(px(10.)).text_color(rgb(DIM)).child(if self.workspace_file.is_some(){crate::i18n::text("SINGLE-FILE SCOPE")}else{crate::i18n::text("LOCAL WORKSPACE")}))
                .child(icon_button("new-skill",Icon::Plus,"New Skill",false,self.graph.is_some()).w(px(24.)).h(px(24.)).on_click(cx.listener(|this,_,w,cx|this.command(crate::commands::Command::Create,w,cx)))))
    }
}

#[cfg(test)]mod tests{
    use super::{context_origin,batch_roots};
    #[test]fn batch_ignores_descendants_of_selected_folders(){assert_eq!(batch_roots(["folder/a","folder","folder2/b","folder",""].map(String::from).to_vec()),vec!["folder","folder2/b"]);}
    #[test]fn context_menu_stays_inside_right_and_bottom_edges(){assert_eq!(context_origin(1438.,898.,1440.,900.),(1186.,676.));}
    #[test]fn context_menu_respects_top_left_margin(){assert_eq!(context_origin(-5.,-2.,1440.,900.),(8.,8.));}
}
