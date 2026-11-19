//! Shared native-drop/open route; all disk reads and image decoding stay off the UI thread.
use crate::{Studio,core,files,layout};
use gpui::*;
use gpui_base::input::{EditorState,InputEvent};
use std::{path::PathBuf,rc::Rc};
impl Studio{
    pub(crate) fn open_external(&mut self,paths:Vec<PathBuf>,window:&mut Window,cx:&mut Context<Self>){
        self.dragging_external=false;
        if !self.composition_ready(window,cx){return;}
        self.open_ticket+=1;let ticket=self.open_ticket;
        let root=self.graph.as_ref().map(|g|g.root.clone());
        cx.spawn_in(window,async move|this,cx|{
            let result=cx.background_executor().spawn(async move{files::resolve_drop(&paths,root.as_deref())}).await;
            let _=this.update_in(cx,|this,window,cx|{
                if this.open_ticket!=ticket{return;}
                match result{
                    Ok(files::DropTarget::File{relative:Some(id),..})=>this.select_file(id,true,window,cx),
                    Ok(files::DropTarget::File{path,..})=>this.open_scope(path,true,window,cx),
                    Ok(files::DropTarget::Folder(path))=>{
                        if this.workspace_file.is_none()&&this.graph.as_ref().is_some_and(|g|std::path::Path::new(&g.root)==path){this.refresh(window,cx);}
                        else{this.open_scope(path,false,window,cx);}
                    }
                    Err(e)=>{this.status=e;cx.notify();}
                }
            });
        }).detach();cx.notify();
    }
    fn open_scope(&mut self,path:PathBuf,single:bool,window:&mut Window,cx:&mut Context<Self>){
        if !self.composition_ready(window,cx)||!self.guard(cx){return;}
        self.open_ticket+=1;let ticket=self.open_ticket;
        self.status=if single{"Opening file · bounded single-file scope…"}else{"Opening folder…"}.into();cx.notify();
        cx.spawn_in(window,async move|this,cx|{
            let result=cx.background_executor().spawn(async move{
                let graph=if single{core::scan_file(&path)}else{core::open_workspace(&path)}?;
                let file=if single{graph.nodes.first().map(|n|n.id.clone())}else{None};
                let saved=if single{layout::LayoutFile{version:1,..Default::default()}}else{layout::load(&graph.root).unwrap_or_else(|_|layout::LayoutFile{version:1,..Default::default()})};
                Ok::<_,String>((graph,file,saved))
            }).await;
            let _=this.update_in(cx,|this,window,cx|{
                if this.open_ticket!=ticket{return;}
                match result{
                    Ok((graph,file,saved))=>{
                        if !this.composition_ready(window,cx)||!this.guard(cx){return;}
                        this.generation+=1;this.positions=saved.positions;this.roles=saved.roles;this.zoom=1.;this.focus=false;
                        this.graph_revision+=1;
                        let has_skills=graph.nodes.iter().any(|n|n.kind=="skill");
                        this.scope=if single||!has_skills{crate::filter::Scope::All}else{crate::filter::Scope::Direct};
                        if single||!has_skills{this.flow_mode=has_skills&& !graph.flows.is_empty();}
                        this.graph=Some(graph);this.workspace_file=file.clone();this.documents.clear();this.selected=None;this.original.clear();this.conflict=false;this.find_open=false;
                        this.start_watcher(window,cx);
                        if let Some(id)=file{this.select_file(id,true,window,cx);}else{this.status="Folder opened · drop a file to open it in place".into();}
                    }
                    Err(e)=>this.status=format!("Open failed: {e}"),
                }cx.notify();
            });
        }).detach();
    }
    pub(crate) fn request_file(&mut self,id:String,pin:bool,reload:bool,preview:bool,window:&mut Window,cx:&mut Context<Self>){self.request_file_at(id,pin,reload,preview,None,window,cx);}
    pub(crate) fn request_file_at(&mut self,id:String,pin:bool,reload:bool,preview:bool,line:Option<usize>,window:&mut Window,cx:&mut Context<Self>){
        self.open_ticket+=1;let ticket=self.open_ticket;
        if self.selected.as_deref()!=Some(id.as_str())&&!self.composition_ready(window,cx){return;}
        if !reload&&self.documents.get(&id).is_some(){
            if pin{self.documents.pin(&id);}
            if preview&&crate::preview::supported(&id){if let Some(tab)=self.documents.get_mut(&id){tab.set_preview(true);}}
            if line.is_some(){if let Some(tab)=self.documents.get_mut(&id){tab.preview=false;}}
            self.activate_tab(&id,window,cx);self.jump_open_line(line,window,cx);return;
        }
        let Some(root)=self.graph.as_ref().map(|g|g.root.clone())else{return;};
        let expected=self.documents.get(&id).map(|t|(t.token,t.buffer.clone()));
        if reload&&self.documents.get(&id).is_some_and(|t|t.dirty){self.status="Save or discard edits before reloading".into();cx.notify();return;}
        self.status=format!("Loading {id}…");cx.notify();
        cx.spawn_in(window,async move|this,cx|{
            let file_root=root.clone();let file_id=id.clone();
            let result=cx.background_executor().spawn(async move{files::load(std::path::Path::new(&file_root),&file_id)}).await;
            let _=this.update_in(cx,|this,window,cx|{
                if this.open_ticket!=ticket||this.graph.as_ref().map(|g|g.root.as_str())!=Some(&root){return;}
                if !this.composition_ready(window,cx){return;}
                if reload&&expected.as_ref().is_some_and(|(token,buffer)|this.documents.get(&id).is_none_or(|t|t.token!=*token||t.buffer!=*buffer)){
                    this.status="Document changed while loading; current edits were retained".into();cx.notify();return;
                }
                match result{Ok(loaded)=>this.install_file(id,pin,preview,line,loaded,window,cx),Err(e)=>{this.status=format!("Cannot open file: {e}");cx.notify();}}
            });
        }).detach();
    }
    fn jump_open_line(&mut self,line:Option<usize>,window:&mut Window,cx:&mut Context<Self>){
        if let Some(line)=line.filter(|_|self.active_text()){
            self.editor.update(cx,|e,cx|e.set_cursor_position(gpui_base::input::Position::new(line.saturating_sub(1).min(u32::MAX as usize) as u32,0),window,cx));
            self.status=format!("Reference: {} · line {line}",self.selected.as_deref().unwrap_or(""));
        }
    }
    fn install_file(&mut self,id:String,pin:bool,preview:bool,line:Option<usize>,loaded:files::Loaded,window:&mut Window,cx:&mut Context<Self>){
        let old=self.documents.get(&id).map(|tab|(tab.pinned,tab.preview));
        if old.is_some(){self.documents.remove(&id);}
        else if let Some(reusable)=self.documents.reusable_preview(|tab|self.saving||tab.dirty){self.documents.remove(&reusable);}
        let editable=loaded.view.is_text();
        let editor=cx.new(|cx|{
            let mut state=EditorState::new(window,cx).line_number(self.preferences.line_numbers).soft_wrap(self.preferences.soft_wrap).replaceable(true);
            state.set_highlighter_factory(Rc::new(|language|Some(Box::new(crate::highlight::Highlighter::new(language)))),cx);
            state.set_editor_style(crate::ui::theme::editor_style());
            let ext=std::path::Path::new(&id).extension().and_then(|s|s.to_str()).unwrap_or("txt").to_ascii_lowercase();
            state.set_highlighter(ext,cx);state.set_value(loaded.content.clone(),window,cx);state.set_disabled(!editable,cx);state
        });
        let token=self.documents.insert(id.clone(),editor.clone(),loaded.content,pin||old.is_some_and(|v|v.0));
        if let Some(tab)=self.documents.get_mut(&id){tab.view=loaded.view;tab.set_preview(line.is_none()&&(preview||old.is_some_and(|v|v.1))&&crate::preview::supported(&id));}
        if editable{
            let changed_id=id.clone();
            cx.subscribe(&editor,move|this,editor,event,cx|{
                if !matches!(event,InputEvent::Change){return;}
                if let Some(tab)=this.documents.get_mut(&changed_id).filter(|tab|tab.token==token){tab.update_buffer(editor.read(cx).value());cx.notify();}
            }).detach();
            let observed_id=id.clone();let mut last_cursor=editor.read(cx).cursor_position();
            cx.observe(&editor,move|this,editor,cx|{let cursor=editor.read(cx).cursor_position();if cursor!=last_cursor{last_cursor=cursor;if this.selected.as_deref()==Some(&observed_id){cx.notify();}}}).detach();
        }
        if let Some(graph)=&mut self.graph{if !graph.entries.iter().any(|e|e.id==id){graph.entries.push(core::WorkspaceEntry{id:id.clone(),directory:false});self.graph_revision+=1;}}
        self.activate_tab(&id,window,cx);
        self.status=if editable{format!("Opened {id} · UTF-8 source")}else{format!("Opened {id} · read-only preview")};self.jump_open_line(line,window,cx);cx.notify();
    }
}
