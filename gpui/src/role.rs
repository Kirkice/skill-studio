use crate::core::{Graph, Node};
use crate::layout::Roles;

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
pub fn label(role: &str) -> &'static str { match role { "main" => "Main Skill", "skill" => "Linked Skill", "subflow" => "Subflow", "document" => "Reference", "script" => "Script", "missing" => "Broken reference", _ => "Resource" } }
pub fn color(role: &str) -> u32 { match role { "main" => 0xb5a2ce, "skill" => 0x96b2d7, "subflow" => 0x8cbdc3, "document" => 0x818793, "script" => 0xa6bd91, "missing" => 0xdc8c95, _ => 0xd2d4d9 } }
