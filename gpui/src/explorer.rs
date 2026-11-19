//! Cached filesystem tree, independent of graph layout and editor repainting.
use crate::core::Graph;
use std::collections::{BTreeMap,HashSet};

#[derive(Debug)]
pub struct Entry {
    pub path:String, pub name:String, pub title:String, pub kind:String,
    pub depth:usize, pub directory:bool, pub files:usize,
    parent:Option<usize>, end:usize, search:String,
}
#[derive(Default)]
struct Branch { children:BTreeMap<String,Branch>, file:bool, title:String, kind:String }
impl Branch {
    fn insert(&mut self,path:&str,directory:bool,title:&str,kind:&str){
        let mut node=self;
        for name in path.split('/').filter(|s|!s.is_empty()){node=node.children.entry(name.into()).or_default();}
        if !directory{node.file=true;node.title=title.into();node.kind=kind.into();}
    }
    fn flatten(&self,prefix:&str,depth:usize,parent:Option<usize>,out:&mut Vec<Entry>){
        let mut children=self.children.iter().collect::<Vec<_>>();
        children.sort_by_key(|(name,node)|(node.file,name.to_lowercase(),*name));
        for (name,node) in children {
            let path=if prefix.is_empty(){name.clone()}else{format!("{prefix}/{name}")};
            let index=out.len();
            out.push(Entry{search:format!("{path} {}",node.title).to_lowercase(),path:path.clone(),name:name.clone(),title:node.title.clone(),kind:node.kind.clone(),depth,directory:!node.file,files:usize::from(node.file),parent,end:index+1});
            if !node.file{
                node.flatten(&path,depth+1,Some(index),out);
                out[index].end=out.len();
                out[index].files=out[index+1..].iter().filter(|e|!e.directory).count();
            }
        }
    }
}
#[derive(Default)]
pub struct Explorer {
    root:String, revision:u64, pub entries:Vec<Entry>, pub rows:Vec<usize>,
    collapsed:HashSet<String>, search_collapsed:HashSet<String>, query:String, focus:Option<HashSet<String>>, dirty:bool,
    pub matched_files:usize,
}
impl Explorer {
    pub fn sync(&mut self,revision:u64,graph:Option<&Graph>){
        let Some(graph)=graph else{if !self.root.is_empty(){*self=Self::default();}return;};
        if self.root==graph.root&&self.revision==revision{return;}
        let different_workspace=self.root!=graph.root;
        if different_workspace{self.collapsed.clear();self.search_collapsed.clear();}
        self.root=graph.root.clone();self.revision=revision;
        let mut tree=Branch::default();
        for entry in &graph.entries{tree.insert(&entry.id,entry.directory,"","file");}
        for node in graph.nodes.iter().filter(|n|n.kind!="missing"){tree.insert(&node.id,false,&node.title,&node.kind);}
        self.entries.clear();tree.flatten("",0,None,&mut self.entries);
        let folders=self.entries.iter().filter(|e|e.directory).map(|e|e.path.clone()).collect::<HashSet<_>>();
        self.collapsed.retain(|path|folders.contains(path));
        if different_workspace{self.collapsed.extend(self.entries.iter().filter(|e|e.directory&&e.depth>=2).map(|e|e.path.clone()));}
        self.dirty=true;
    }
    pub fn toggle(&mut self,path:&str){let folds=if self.query.is_empty(){&mut self.collapsed}else{&mut self.search_collapsed};if !folds.remove(path){folds.insert(path.into());}self.dirty=true;}
    pub fn collapse_all(&mut self){let folds=self.entries.iter().filter(|e|e.directory).map(|e|e.path.clone()).collect();if self.query.is_empty(){self.collapsed=folds;}else{self.search_collapsed=folds;}self.dirty=true;}
    pub fn reveal(&mut self,path:&str){
        for (index,_) in path.match_indices('/') {let normal=self.collapsed.remove(&path[..index]);let search=self.search_collapsed.remove(&path[..index]);if normal||search{self.dirty=true;}}
    }
    pub fn expanded(&self,path:&str)->bool{if self.query.is_empty(){!self.collapsed.contains(path)}else{!self.search_collapsed.contains(path)}}
    pub fn filter(&mut self,query:&str,focus:Option<&HashSet<String>>){
        let query=query.trim().to_lowercase();
        if !self.dirty&&self.query==query&&self.focus.as_ref()==focus{return;}
        if self.query!=query{self.search_collapsed.clear();}
        self.query=query;self.focus=focus.cloned();self.dirty=false;
        let words=self.query.split_whitespace().collect::<Vec<_>>();
        let mut included=self.entries.iter().map(|e|words.iter().all(|w|e.search.contains(w))&&self.focus.as_ref().is_none_or(|ids|!e.directory&&ids.contains(&e.path))).collect::<Vec<_>>();
        for i in (0..included.len()).rev(){if included[i]{if let Some(parent)=self.entries[i].parent{included[parent]=true;}}}
        self.matched_files=self.entries.iter().enumerate().filter(|(i,e)|included[*i]&&!e.directory).count();
        self.rows.clear();let mut i=0;
        while i<self.entries.len(){
            let e=&self.entries[i];if included[i]{self.rows.push(i);}
            if e.directory&&!self.expanded(&e.path){i=e.end;}else{i+=1;}
        }
    }
}
#[cfg(test)]mod tests {
    use super::*;
    use crate::core::{Graph,WorkspaceEntry,Node};
    fn graph()->Graph{Graph{root:"test".into(),entries:[("z.md",false),("shader",true),("shader/empty",true),("shader/references/a.md",false),("shader/SKILL.md",false),("shader/settings.json",false)].into_iter().map(|(id,directory)|WorkspaceEntry{id:id.into(),directory}).collect(),nodes:vec![Node{id:"shader/references/a.md".into(),label:"a.md".into(),kind:"document".into(),title:"中文知识库".into(),summary:String::new(),headings:vec![]}],edges:vec![],warnings:vec![],flows:vec![]}}
    #[test]fn directories_first_with_empty_folders_and_non_graph_files(){
        let mut e=Explorer::default();e.sync(1,Some(&graph()));e.filter("",None);
        assert_eq!(e.entries[e.rows[0]].path,"shader");
        assert!(e.entries.iter().any(|e|e.path=="shader/empty"&&e.directory));
        assert_eq!(e.matched_files,4);assert_eq!(e.entries.last().unwrap().path,"z.md");
    }
    #[test]fn search_reveals_ancestors_without_destroying_folds(){
        let mut e=Explorer::default();e.sync(1,Some(&graph()));e.collapse_all();e.filter("",None);assert_eq!(e.rows.len(),2);
        e.filter("中文",None);assert_eq!(e.rows.len(),3);assert_eq!(e.matched_files,1);
        e.filter("",None);assert_eq!(e.rows.len(),2);
        e.reveal("shader/references/a.md");e.filter("",None);assert!(e.rows.iter().any(|&i|e.entries[i].path=="shader/references/a.md"));
    }
    #[test]fn refresh_preserves_folds_but_different_workspace_resets_them(){
        let mut e=Explorer::default();let mut g=graph();e.sync(1,Some(&g));e.toggle("shader");e.sync(2,Some(&g));assert!(!e.expanded("shader"));
        g.root="other".into();e.sync(3,Some(&g));assert!(e.expanded("shader"));
    }
    #[test]fn search_results_can_be_folded_independently(){
        let mut e=Explorer::default();e.sync(1,Some(&graph()));e.filter("中文",None);assert_eq!(e.rows.len(),3);
        e.toggle("shader");e.filter("中文",None);assert_eq!(e.rows.len(),1);
        e.filter("",None);assert!(e.expanded("shader"));
    }
    #[test]fn focus_keeps_only_files_and_their_ancestors(){
        let mut e=Explorer::default();e.sync(1,Some(&graph()));let focus=HashSet::from(["shader/SKILL.md".into()]);e.filter("",Some(&focus));
        assert_eq!(e.rows.len(),2);assert_eq!(e.matched_files,1);
    }
}
