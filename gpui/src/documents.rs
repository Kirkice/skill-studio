//! Per-file sessions; editor entities retain selection, scroll, search and undo.
pub struct Tab<E> {
    pub id: String,
    pub token: u64,
    pub editor: E,
    pub view: crate::files::View,
    pub original: String,
    pub buffer: gpui::SharedString,
    pub dirty: bool,
    pub preview_body: Option<gpui::SharedString>,
    pub conflict: bool,
    pub pinned: bool,
    pub preview: bool,
    pub preview_scroll: gpui::ScrollHandle,
}
impl<E> Tab<E> {
    pub fn update_buffer(&mut self,buffer:gpui::SharedString){
        if !self.view.is_text()||self.buffer==buffer{return;}
        self.buffer=buffer;self.dirty=self.buffer.as_ref()!=self.original;
        self.preview_body=None;
        if self.dirty{self.pinned=true;}
        if self.preview{self.set_preview(true);}
    }
    pub fn set_original(&mut self,original:String){self.original=original;self.dirty=self.buffer.as_ref()!=self.original;}
    pub fn set_preview(&mut self,value:bool){
        self.preview=value&&self.view.is_text();
        if self.preview&&self.preview_body.is_none(){self.preview_body=Some(crate::preview::body(self.buffer.as_ref()).into());}
    }
}
pub struct Documents<E> { pub tabs: Vec<Tab<E>>, next_token:u64 }
impl<E> Default for Documents<E> {fn default()->Self{Self{tabs:vec![],next_token:0}}}
impl<E> Documents<E> {
    pub fn get(&self,id:&str)->Option<&Tab<E>>{self.tabs.iter().find(|tab|tab.id==id)}
    pub fn get_mut(&mut self,id:&str)->Option<&mut Tab<E>>{self.tabs.iter_mut().find(|tab|tab.id==id)}
    pub fn insert(&mut self,id:String,editor:E,original:String,pinned:bool)->u64{
        self.next_token+=1;let token=self.next_token;
        let buffer=original.clone().into();
        self.tabs.push(Tab{id,token,editor,view:Default::default(),original,buffer,dirty:false,preview_body:None,conflict:false,pinned,preview:false,preview_scroll:gpui::ScrollHandle::new()});token
    }
    pub fn remove(&mut self,id:&str)->Option<String>{
        let index=self.tabs.iter().position(|tab|tab.id==id)?;
        self.tabs.remove(index);
        self.tabs.get(index.min(self.tabs.len().saturating_sub(1))).map(|tab|tab.id.clone())
    }
    pub fn reusable_preview(&self,mut dirty:impl FnMut(&Tab<E>)->bool)->Option<String>{
        self.tabs.iter().find(|tab|!tab.pinned&&!dirty(tab)).map(|tab|tab.id.clone())
    }
    pub fn pin(&mut self,id:&str){if let Some(tab)=self.get_mut(id){tab.pinned=true;}}
    pub fn clear(&mut self){self.tabs.clear();}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn sessions_are_independent_and_adjacent_tab_is_selected_after_close(){
        let mut docs=Documents::default();
        docs.insert("a".into(),String::from("edited a"),"a".into(),true);
        docs.insert("b".into(),String::from("b"),"b".into(),false);
        assert_eq!(docs.get("a").unwrap().editor,"edited a");
        assert_eq!(docs.get("b").unwrap().editor,docs.get("b").unwrap().original);
        docs.pin("b");assert!(docs.get("b").unwrap().pinned);
        assert_eq!(docs.remove("a"),Some("b".into()));assert!(docs.remove("b").is_none());
    }
    #[test]fn dirty_and_pinned_tabs_are_never_reused_as_previews(){
        let mut docs=Documents::default();
        docs.insert("dirty".into(),"edited".to_string(),"original".into(),false);
        docs.insert("pinned".into(),"same".to_string(),"same".into(),true);
        docs.insert("preview".into(),"same".to_string(),"same".into(),false);
        assert_eq!(docs.reusable_preview(|tab|tab.editor!=tab.original),Some("preview".into()));
        docs.remove("preview");assert!(docs.reusable_preview(|tab|tab.editor!=tab.original).is_none());
    }
    #[test]fn cached_buffer_tracks_undo_save_and_preview_invalidation(){
        let mut docs=Documents::default();docs.insert("a.md".into(),(),"# A".into(),false);
        let tab=docs.get_mut("a.md").unwrap();tab.set_preview(true);assert!(tab.preview_body.is_some());
        tab.set_preview(false);tab.update_buffer("# B".into());assert!(tab.dirty&&tab.pinned);assert!(tab.preview_body.is_none());
        tab.update_buffer("# A".into());assert!(!tab.dirty);
        tab.update_buffer("# B".into());tab.set_original("# B".into());assert!(!tab.dirty);
        tab.set_preview(true);assert_eq!(tab.preview_body.as_ref().unwrap().as_ref(),"# B");
    }
    #[test]fn readonly_files_cannot_become_dirty_or_markdown_previews(){
        let mut docs=Documents::default();docs.insert("binary.md".into(),(),String::new(),true);
        let tab=docs.get_mut("binary.md").unwrap();
        tab.view=crate::files::View::Unsupported{stamp:crate::files::Stamp{bytes:9,modified:None},reason:"binary".into()};
        tab.update_buffer("accidental edit".into());tab.set_preview(true);
        assert!(!tab.dirty&&!tab.preview);assert!(tab.buffer.is_empty());assert!(tab.preview_body.is_none());
    }
    #[test]fn reopened_files_get_new_tokens_for_async_result_protection(){
        let mut docs=Documents::default();let old=docs.insert("a".into(),(),String::new(),false);
        docs.clear();let new=docs.insert("a".into(),(),String::new(),false);assert_ne!(old,new);
    }
}
