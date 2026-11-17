use crate::core::Graph;
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope { Direct, Recursive, All }
pub fn visible(graph: &Graph, scope: Scope, focus: Option<&str>, query: &str, show_missing: bool) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(focus) = focus {
        ids.insert(focus.to_string());
        for edge in &graph.edges { if edge.source == focus { ids.insert(edge.target.clone()); } if edge.target == focus { ids.insert(edge.source.clone()); } }
    } else if scope == Scope::All {
        ids.extend(graph.nodes.iter().map(|n| n.id.clone()));
    } else {
        ids.extend(graph.nodes.iter().filter(|n| n.kind == "skill").map(|n| n.id.clone()));
        let mut queue: VecDeque<_> = ids.iter().cloned().collect();
        while let Some(source) = queue.pop_front() {
            for edge in graph.edges.iter().filter(|e| e.source == source) {
                if ids.insert(edge.target.clone()) && scope == Scope::Recursive { queue.push_back(edge.target.clone()); }
            }
        }
    }
    let query = query.to_lowercase();
    graph.nodes.iter().filter(|n| ids.contains(&n.id) && (show_missing || n.kind != "missing") && (query.is_empty() || format!("{} {} {}", n.id, n.title, n.summary).to_lowercase().contains(&query))).map(|n| n.id.clone()).collect()
}

#[cfg(test)] mod tests {
    use super::*;
    use crate::core::{Node, Edge};
    fn graph() -> Graph {
        Graph {root:String::new(), warnings:vec![], nodes:vec![node("a", "skill"),node("b", "document"),node("c", "document"),node("d", "missing")], edges:vec![edge("a","b"),edge("b","c"),edge("c","a"),edge("a","d")]}
    }
    fn node(id:&str,kind:&str)->Node {Node{id:id.into(), label:id.into(),kind:kind.into(),title:String::new(),summary:String::new(),headings:vec![]}}
    fn edge(a:&str,b:&str)->Edge {Edge{source:a.into(),target:b.into(),line:1,missing:false}}
    #[test] fn direct_and_recursive_terminate_on_cycle() { let g=graph(); assert_eq!(visible(&g,Scope::Direct,None,"",false),HashSet::from(["a".into(),"b".into()])); assert_eq!(visible(&g,Scope::Recursive,None,"",false).len(),3); }
    #[test] fn focus_query_and_missing() { let g=graph(); assert_eq!(visible(&g,Scope::All,Some("a"),"d",true),HashSet::from(["d".into()])); assert!(visible(&g,Scope::All,None,"d",false).is_empty()); }
}
