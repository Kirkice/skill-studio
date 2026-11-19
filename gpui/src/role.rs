use crate::core::{Graph, Node};
use crate::layout::Roles;

// Build once per graph/override update, not once for every rendered node.
pub fn index(graph: &Graph, overrides: &Roles) -> std::collections::HashMap<String, String> {
    use std::collections::HashSet;
    let skills: HashSet<_> = graph.nodes.iter().filter(|n| n.kind == "skill").map(|n| n.id.as_str()).collect();
    let children: HashSet<_> = graph.edges.iter().filter(|e| skills.contains(e.source.as_str())).map(|e| e.target.as_str()).collect();
    graph.nodes.iter().map(|node| {
        let value = overrides.get(&node.id).cloned().unwrap_or_else(|| {
            let parent = children.contains(node.id.as_str());
            match node.kind.as_str() {
                "skill" if parent => "skill", "skill" => "main",
                "document" if parent && ["流程", "工作流", "编写", "构建", "验收", "authoring", "workflow", "builder", "acceptance"].iter().any(|s| node.title.to_lowercase().contains(s)) => "subflow",
                "document" => "document", "script" => "script", "missing" => "missing", _ => "file",
            }.into()
        });
        (node.id.clone(), value)
    }).collect()
}
pub fn role<'a>(node: &'a Node, graph: &Graph, overrides: &'a Roles) -> &'a str {
    if let Some(role) = overrides.get(&node.id) { return role; }
    let skill_parent = graph.edges.iter().any(|e| e.target == node.id && graph.nodes.iter().any(|n| n.id == e.source && n.kind == "skill"));
    match node.kind.as_str() {
        "skill" if skill_parent => "skill",
        "skill" => "main",
        "document" if skill_parent && ["流程", "工作流", "编写", "构建", "验收", "authoring", "workflow", "builder", "acceptance"].iter().any(|s| node.title.to_lowercase().contains(s)) => "subflow",
        "document" => "document",
        "script" => "script",
        "missing" => "missing",
        _ => "file",
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn cached_roles_match_individual_classification(){
        let nodes=[("a","skill","入口"),("b","skill","关联"),("c","document","编写流程"),("d","document","安全规范")].into_iter().map(|(id,kind,title)|Node{id:id.into(),kind:kind.into(),title:title.into(),label:id.into(),summary:String::new(),headings:vec![]}).collect();
        let graph=Graph{root:String::new(),entries:vec![],warnings:vec![],flows:vec![],nodes,edges:["b","c","d"].into_iter().map(|target|crate::core::Edge{source:"a".into(),target:target.into(),line:1,missing:false}).collect()};
        for overrides in [Roles::new(),Roles::from([("c".into(),"document".into())])]{let cache=index(&graph,&overrides);for n in &graph.nodes{assert_eq!(cache[&n.id],role(n,&graph,&overrides));}}
    }
}
pub fn label(role: &str) -> &'static str { match role { "main" => "Main Skill", "skill" => "Linked Skill", "subflow" => "Subflow", "document" => "Reference", "script" => "Script", "missing" => "Broken reference", _ => "Resource" } }
pub fn color(role: &str) -> u32 { match role { "main" => 0xb5a2ce, "skill" => 0x96b2d7, "subflow" => 0x8cbdc3, "document" => 0x818793, "script" => 0xa6bd91, "missing" => 0xdc8c95, _ => 0xd2d4d9 } }
