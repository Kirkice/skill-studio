use crate::core::Graph;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity { Error, Warning, Info }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
    pub detail: String,
}

fn extract_frontmatter_description(content: &str) -> String {
    if !content.starts_with("---") {
        return String::new();
    }
    let mut lines = content.lines().skip(1);
    let mut desc = Vec::new();
    let mut in_desc = false;
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("description:") {
            let val = rest.trim().trim_matches(['"', '\'']);
            if !val.is_empty() && val != "|" && val != ">" && val != "|-" && val != ">-" {
                desc.push(val.to_string());
            }
            in_desc = true;
        } else if in_desc {
            if line.starts_with(' ') || line.starts_with('\t') {
                desc.push(trimmed.to_string());
            } else {
                in_desc = false;
            }
        }
    }
    desc.join(" ")
}

fn trigger_terms(text: &str) -> HashSet<String> {
    let stop: HashSet<&str> = [
        "when", "user", "use", "skill", "this", "for", "and", "the", "with", "that", "from", "into",
        "only", "not", "走", "当用户", "使用", "需要", "或者", "以及", "不负责", "管理", "查看", "查询",
        "操作", "支持", "如果", "相关", "通过", "进行", "可以", "用于", "包含",
    ]
    .into_iter()
    .collect();
    let mut terms = HashSet::new();
    let mut ascii_buf = String::new();
    let mut cjk_buf = Vec::new();
    let flush_cjk = |buf: &mut Vec<char>, out: &mut HashSet<String>| {
        if buf.len() >= 2 {
            for w in buf.windows(2) {
                let s: String = w.iter().collect();
                if !stop.contains(s.as_str()) {
                    out.insert(s);
                }
            }
        }
        buf.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            flush_cjk(&mut cjk_buf, &mut terms);
            ascii_buf.push(ch.to_ascii_lowercase());
        } else if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            if ascii_buf.len() >= 3 && !stop.contains(ascii_buf.as_str()) {
                terms.insert(ascii_buf.clone());
            }
            ascii_buf.clear();
            cjk_buf.push(ch);
        } else {
            if ascii_buf.len() >= 3 && !stop.contains(ascii_buf.as_str()) {
                terms.insert(ascii_buf.clone());
            }
            ascii_buf.clear();
            flush_cjk(&mut cjk_buf, &mut terms);
        }
    }
    if ascii_buf.len() >= 3 && !stop.contains(ascii_buf.as_str()) {
        terms.insert(ascii_buf);
    }
    flush_cjk(&mut cjk_buf, &mut terms);
    terms
}

impl Diagnostic {
    pub fn location(&self) -> String {
        self.line.map_or_else(|| self.file.clone(), |line| format!("{}:{line}", self.file))
    }

    pub fn pi_prompt(&self) -> String {
        let loc = if self.file.is_empty() {
            "the workspace".to_string()
        } else if let Some(line) = self.line {
            format!("{}:{line}", self.file)
        } else {
            self.file.clone()
        };
        match self.code {
            "SR001" => format!(
                "Fix diagnostic [{}] at {}: {}. Inspect {} around line {}, check existing files in the skill directory, and either fix the relative link path or create the missing reference file.",
                self.code, loc, self.message, self.file, self.line.unwrap_or(1)
            ),
            "SR201" => format!(
                "Resolve circular reference [{}] at {}: {} ({}). Refactor the Markdown links so progressive disclosure flows strictly from entry SKILL.md to reference docs without loops.",
                self.code, loc, self.message, self.detail
            ),
            "SR301" => format!(
                "Fix [{}] in {}: add a clear trigger condition (e.g. '触发条件：...' or 'Trigger condition: ...') near the top of {} and align it with the YAML frontmatter description.",
                self.code, loc, self.file
            ),
            "SR401" => format!(
                "Optimize L0 frontmatter budget [{}] in {}: {}. Slim the YAML `description` to under 180 tokens while preserving core trigger keywords and routing boundaries ('use when' / 'do not use for').",
                self.code, loc, self.message
            ),
            "SR402" => format!(
                "Slim heavy entry Skill [{}] in {}: {}. Identify the heaviest inline sections, extract detailed tables/examples/edge-cases into `references/*.md`, and replace them in {} with concise routing summaries and Markdown links.",
                self.code, loc, self.message, self.file
            ),
            "SR403" => format!(
                "Refactor heavy inline section [{}] at {}: {}. Extract the detailed reference material or long code blocks from this section into a dedicated `references/*.md` file and link to it from {} so L1 context stays lean.",
                self.code, loc, self.message, self.file
            ),
            "SR404" => format!(
                "Resolve multi-skill trigger collision [{}] at {}: {}. {} Review both skills' YAML frontmatter `description` fields and sharpen their positive triggers and negative boundaries ('不负责...' / 'Do not use for...') so the agent router can distinguish them unambiguously.",
                self.code, loc, self.message, self.detail
            ),
            _ => format!(
                "Fix Skill diagnostic [{}] at {}: {} ({})",
                self.code, loc, self.message, self.detail
            ),
        }
    }
}

/// Fast, deterministic workspace checks. These diagnostics are source-backed and do not invoke a model.
pub fn check(graph: &Graph) -> Vec<Diagnostic> {
    let mut result = Vec::new();

    for edge in graph.edges.iter().filter(|edge| edge.missing) {
        let target = graph.nodes.iter().find(|node| node.id == edge.target)
            .map(|node| node.label.as_str()).unwrap_or("unknown file");
        result.push(Diagnostic {
            severity: Severity::Error,
            code: "SR001",
            file: edge.source.clone(),
            line: Some(edge.line),
            message: format!("Referenced file does not exist: {target}"),
            detail: "Correct the relative path or create the referenced file.".into(),
        });
    }

    // Report source scanner warnings as diagnostics instead of leaving them only in transient status text.
    for warning in &graph.warnings {
        result.push(Diagnostic {
            severity: Severity::Warning,
            code: "SR002",
            file: String::new(),
            line: None,
            message: warning.clone(),
            detail: "The scanner skipped or could not fully inspect this item.".into(),
        });
    }

    let known: HashSet<&str> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    for edge in graph.edges.iter().filter(|edge| !edge.missing && known.contains(edge.target.as_str())) {
        *incoming.entry(edge.target.as_str()).or_default() += 1;
    }
    for node in graph.nodes.iter().filter(|node| matches!(node.kind.as_str(), "document" | "script") && incoming.get(node.id.as_str()).copied().unwrap_or(0) == 0) {
        result.push(Diagnostic {
            severity: Severity::Info,
            code: "SR101",
            file: node.id.clone(),
            line: Some(1),
            message: "File is not referenced by another indexed document".into(),
            detail: "This may be intentional. Add a reference if the file belongs to the Skill flow.".into(),
        });
    }

    // DFS back edges provide a concise, stable cycle report without duplicating every node in the cycle.
    let mut adjacency: HashMap<&str, Vec<(&str, usize)>> = HashMap::new();
    for edge in graph.edges.iter().filter(|edge| !edge.missing) {
        adjacency.entry(edge.source.as_str()).or_default().push((edge.target.as_str(), edge.line));
    }
    let mut state: HashMap<&str, u8> = HashMap::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut cycles = HashSet::new();
    fn visit<'a>(
        node: &'a str,
        adjacency: &HashMap<&'a str, Vec<(&'a str, usize)>>,
        state: &mut HashMap<&'a str, u8>,
        stack: &mut Vec<&'a str>,
        cycles: &mut HashSet<String>,
        output: &mut Vec<Diagnostic>,
    ) {
        state.insert(node, 1); stack.push(node);
        if let Some(edges) = adjacency.get(node) {
            for &(target, line) in edges {
                match state.get(target).copied().unwrap_or(0) {
                    0 => visit(target, adjacency, state, stack, cycles, output),
                    1 => {
                        let start = stack.iter().position(|item| *item == target).unwrap_or(0);
                        let mut path: Vec<&str> = stack[start..].to_vec(); path.push(target);
                        let key = path.join(" -> ");
                        if cycles.insert(key.clone()) {
                            output.push(Diagnostic { severity: Severity::Warning, code: "SR201", file: node.into(), line: Some(line), message: "Circular reference detected".into(), detail: key });
                        }
                    }
                    _ => {}
                }
            }
        }
        stack.pop(); state.insert(node, 2);
    }
    for node in graph.nodes.iter().filter(|node| node.kind != "missing") {
        if state.get(node.id.as_str()).copied().unwrap_or(0) == 0 {
            visit(&node.id, &adjacency, &mut state, &mut stack, &mut cycles, &mut result);
        }
    }

    for flow in graph.flows.iter().filter(|flow| flow.entry && flow.meaning.condition.is_none()) {
        result.push(Diagnostic {
            severity: Severity::Warning,
            code: "SR301",
            file: flow.source.clone(),
            line: Some(1),
            message: "No explicit trigger condition found".into(),
            detail: "Consider documenting a ‘触发条件：’ or ‘Trigger condition:’ so users know when this Skill applies.".into(),
        });
    }

    for flow in &graph.flows {
        if flow.entry && flow.frontmatter_tokens > 250 {
            result.push(Diagnostic {
                severity: Severity::Warning,
                code: "SR401",
                file: flow.source.clone(),
                line: Some(1),
                message: format!("Frontmatter budget high (~{} tokens > 250)", flow.frontmatter_tokens),
                detail: "L0 frontmatter is injected into the system prompt on every turn. Keep name and description concise.".into(),
            });
        }
        if flow.entry && flow.body_tokens > 2_000 {
            result.push(Diagnostic {
                severity: Severity::Warning,
                code: "SR402",
                file: flow.source.clone(),
                line: Some(1),
                message: format!("Entry Skill body is heavy (~{} tokens > 2,000)", flow.body_tokens),
                detail: "Move detailed tables, examples or edge cases into references/*.md for progressive disclosure.".into(),
            });
        }
        for stage in &flow.stages {
            if stage.line > 0 && stage.tokens > 600 && stage.calls.is_empty() {
                result.push(Diagnostic {
                    severity: Severity::Info,
                    code: "SR403",
                    file: flow.source.clone(),
                    line: Some(stage.line),
                    message: format!("Heavy inline section ‘{}’ (~{} tokens, {:.0}% of doc)", stage.title, stage.tokens, stage.share * 100.),
                    detail: "This section carries substantial context without delegating to a reference file. Consider extracting it.".into(),
                });
            }
        }
    }

    let entries: Vec<&crate::flow::Flow> = graph.flows.iter().filter(|f| f.entry).collect();
    if entries.len() > 1 {
        let root_path = std::path::Path::new(&graph.root);
        let mut skill_terms: Vec<(&crate::flow::Flow, HashSet<String>)> = Vec::new();
        for flow in &entries {
            let desc = crate::core::read_document(root_path, &flow.source)
                .map(|doc| extract_frontmatter_description(&doc.content))
                .unwrap_or_default();
            let combined = format!("{} {} {}", flow.title, desc, flow.meaning.purpose.as_ref().map_or("", |p| p.text.as_str()));
            skill_terms.push((flow, trigger_terms(&combined)));
        }
        for i in 0..skill_terms.len() {
            for j in (i + 1)..skill_terms.len() {
                let (fa, ta) = &skill_terms[i];
                let (fb, tb) = &skill_terms[j];
                if ta.len() < 4 || tb.len() < 4 {
                    continue;
                }
                let mut shared: Vec<&String> = ta.intersection(tb).collect();
                let union = ta.union(tb).count().max(1);
                let jaccard = shared.len() as f32 / union as f32;
                if shared.len() >= 5 && jaccard >= 0.28 {
                    shared.sort();
                    let sample: Vec<&str> = shared.into_iter().take(6).map(|s| s.as_str()).collect();
                    result.push(Diagnostic {
                        severity: Severity::Warning,
                        code: "SR404",
                        file: fa.source.clone(),
                        line: Some(1),
                        message: format!(
                            "Trigger overlap with ‘{}’ ({:.0}% shared terms)",
                            fb.title,
                            jaccard * 100.
                        ),
                        detail: format!(
                            "Both {} and {} match [{}]. Clarify negative boundaries in YAML description.",
                            fa.source,
                            fb.source,
                            sample.join(", ")
                        ),
                    });
                }
            }
        }
    }

    result.sort_by(|a, b| a.severity.cmp(&b.severity).then_with(|| a.file.cmp(&b.file)).then_with(|| a.line.cmp(&b.line)).then_with(|| a.code.cmp(b.code)));
    result
}

pub fn counts(items: &[Diagnostic]) -> (usize, usize, usize) {
    (items.iter().filter(|d| d.severity == Severity::Error).count(), items.iter().filter(|d| d.severity == Severity::Warning).count(), items.iter().filter(|d| d.severity == Severity::Info).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Edge, Graph, Node};
    fn node(id: &str, kind: &str) -> Node { Node { id: id.into(), label: id.into(), kind: kind.into(), title: id.into(), summary: String::new(), headings: vec![] } }
    #[test]
    fn finds_missing_orphans_cycles_and_trigger_gaps() {
        let mut graph = Graph { root: String::new(), entries: vec![], nodes: vec![node("SKILL.md", "skill"), node("a.md", "document"), node("orphan.md", "document"), node("missing:bad", "missing")], edges: vec![Edge { source: "SKILL.md".into(), target: "a.md".into(), line: 4, missing: false }, Edge { source: "a.md".into(), target: "SKILL.md".into(), line: 7, missing: false }, Edge { source: "SKILL.md".into(), target: "missing:bad".into(), line: 9, missing: true }], warnings: vec![], flows: vec![] };
        graph.flows = crate::flow::collect(&graph);
        let items = check(&graph);
        assert!(items.iter().any(|d| d.code == "SR001" && d.line == Some(9)));
        assert!(items.iter().any(|d| d.code == "SR101" && d.file == "orphan.md"));
        assert!(items.iter().any(|d| d.code == "SR201"));
    }
    #[test]
    fn reports_context_budget_diagnostics_for_bloated_frontmatter_body_and_sections() {
        let dir = tempfile::tempdir().unwrap();
        let big_desc = "触发描述 ".repeat(120);
        let big_sec = "详细内联说明文本，没有任何外部链接拆分。".repeat(160);
        let content = format!("---
name: heavy-skill
description: {big_desc}
---
# Heavy

触发条件：测试。

## 臃肿章节

{big_sec}
");
        std::fs::write(dir.path().join("SKILL.md"), content).unwrap();
        let graph = crate::core::open_workspace(dir.path()).unwrap();
        let items = check(&graph);
        assert!(items.iter().any(|d| d.code == "SR401"));
        assert!(items.iter().any(|d| d.code == "SR402"));
        assert!(items.iter().any(|d| d.code == "SR403"));
    }
    #[test]
    fn detects_multi_skill_trigger_collision_and_generates_pi_prompts() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("skill-a");
        let b = dir.path().join("skill-b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let desc = "飞书电子表格多维表格数据批量读写字段记录视图导入导出查询分析";
        std::fs::write(
            a.join("SKILL.md"),
            format!("---\nname: skill-a\ndescription: {desc}\n---\n# 表格助手 A\n\n触发条件：{desc}\n"),
        )
        .unwrap();
        std::fs::write(
            b.join("SKILL.md"),
            format!("---\nname: skill-b\ndescription: {desc}\n---\n# 表格助手 B\n\n触发条件：{desc}\n"),
        )
        .unwrap();
        let graph = crate::core::open_workspace(dir.path()).unwrap();
        let items = check(&graph);
        let sr404 = items.iter().find(|d| d.code == "SR404").expect("should detect SR404 trigger collision");
        assert!(sr404.pi_prompt().contains("SR404"));
    }
}
