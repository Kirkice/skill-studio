use pulldown_cmark::{Event, Parser, Tag};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Serialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub headings: Vec<String>,
}
#[derive(Serialize)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub line: usize,
    pub missing: bool,
}
#[derive(Serialize)]
pub struct Graph {
    pub root: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
pub struct Document {
    pub content: String,
}
// Only explicit Markdown links and standalone inline-code file paths are indexed.
fn references(content: &str) -> Vec<(String, usize)> {
    Parser::new(content)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            let dest = match event {
                Event::Start(Tag::Link { dest_url, .. })
                | Event::Start(Tag::Image { dest_url, .. }) => dest_url.to_string(),
                Event::Code(code)
                    if !code.chars().any(char::is_whitespace)
                        && matches!(
                            Path::new(code.as_ref())
                                .extension()
                                .and_then(|e| e.to_str()),
                            Some("md" | "py" | "sh" | "js" | "ts" | "ps1" | "rs")
                        ) =>
                {
                    code.to_string()
                }
                _ => return None,
            };
            if dest.starts_with('#') || dest.contains(':') || dest.starts_with("//") {
                return None;
            }
            let dest = dest.split(['#', '?']).next().unwrap_or("").to_string();
            if dest.is_empty() {
                return None;
            }
            Some((
                dest,
                content[..range.start]
                    .bytes()
                    .filter(|b| *b == b'\n')
                    .count()
                    + 1,
            ))
        })
        .collect()
}
fn metadata(content: &str) -> (String, String, Vec<String>) {
    let body = if content.starts_with("---") {
        content
            .lines()
            .skip(1)
            .position(|l| l.trim() == "---")
            .map(|n| content.lines().skip(n + 2).collect::<Vec<_>>().join("\n"))
            .unwrap_or_else(|| content.to_string())
    } else {
        content.to_string()
    };
    let mut headings = Vec::new();
    let mut paragraph = String::new();
    let mut summary = String::new();
    let mut heading = String::new();
    let mut in_heading = false;
    let mut in_paragraph = false;
    for event in Parser::new(&body) {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading.clear();
            }
            Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                in_heading = false;
                headings.push(heading.clone());
            }
            Event::Start(Tag::Paragraph) => {
                in_paragraph = true;
                paragraph.clear();
            }
            Event::End(pulldown_cmark::TagEnd::Paragraph) => {
                in_paragraph = false;
                if summary.is_empty() {
                    summary = paragraph.trim().chars().take(180).collect();
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if in_heading {
                    heading.push_str(&t);
                } else if in_paragraph {
                    paragraph.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak if in_paragraph => paragraph.push(' '),
            _ => {}
        }
    }
    let title = headings.first().cloned().unwrap_or_default();
    (
        title,
        summary,
        headings.into_iter().skip(1).take(5).collect(),
    )
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn safe(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let p = root.join(relative).canonicalize().map_err(err)?;
    if !p.starts_with(root) {
        return Err("Cannot access files outside the workspace".into());
    }
    Ok(p)
}
fn id(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}
pub fn open_workspace(path: &Path) -> Result<Graph, String> {
    let root = path.canonicalize().map_err(err)?;
    if !root.is_dir() { return Err("Select a directory".into()); }
    scan_workspace(&root)
}
pub fn scan_workspace(root: &Path) -> Result<Graph, String> {
    let root = root.to_path_buf();
    let mut graph = Graph {
        root: root.to_string_lossy().into(),
        nodes: vec![],
        edges: vec![],
        warnings: vec![],
    };
    let mut known = HashSet::new();
    let mut markdown = vec![];
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !matches!(
                e.file_name().to_str(),
                Some(".git" | "node_modules" | "target" | ".venv" | "dist")
            )
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                graph.warnings.push(e.to_string());
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "md" | "py" | "sh" | "js" | "ts" | "ps1" | "rs") {
            continue;
        }
        let key = id(&root, p);
        let kind = if entry.file_name() == "SKILL.md" {
            "skill"
        } else if ext == "md" {
            "document"
        } else {
            "script"
        };
        known.insert(key.clone());
        let (title, summary, headings) = if ext == "md" {
            fs::read_to_string(p)
                .map(|s| metadata(&s))
                .unwrap_or_default()
        } else {
            Default::default()
        };
        graph.nodes.push(Node {
            id: key,
            label: entry.file_name().to_string_lossy().into(),
            kind: kind.into(),
            title,
            summary,
            headings,
        });
        if ext == "md" {
            markdown.push(p.to_owned());
        }
    }
    for p in markdown {
        let content = match fs::read_to_string(&p) {
            Ok(s) => s,
            Err(e) => {
                graph.warnings.push(format!("{}: {e}", p.display()));
                continue;
            }
        };
        for (dest, line) in references(&content) {
            let target_path = p.parent().unwrap().join(&dest);
            let canonical = target_path.canonicalize();
            if let Ok(ref cp) = canonical {
                if !cp.starts_with(&root) {
                    graph.warnings.push(format!("External reference ignored: {dest}"));
                    continue;
                }
            }
            let missing = canonical.is_err();
            let target = if let Ok(cp) = canonical {
                id(&root, &cp)
            } else {
                format!("missing:{}:{}", id(&root, &p), dest)
            };
            if known.insert(target.clone()) {
                graph.nodes.push(Node {
                    id: target.clone(),
                    label: dest,
                    kind: if missing { "missing" } else { "file" }.into(),
                    title: String::new(),
                    summary: String::new(),
                    headings: vec![],
                });
            }
            graph.edges.push(Edge {
                source: id(&root, &p),
                target,
                line,
                missing,
            });
        }
    }
    graph.nodes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(graph)
}
pub fn read_document(root: &Path, path: &str) -> Result<Document, String> {
    let p = safe(root, path)?;
    let meta = fs::metadata(&p).map_err(err)?;
    if meta.len() > 2_000_000 {
        return Err("Files larger than 2 MB cannot be edited".into());
    }
    Ok(Document {
        content: fs::read_to_string(p).map_err(err)?,
    })
}
pub fn save_document(root: &Path, path: &str, content: &str, original: &str) -> Result<(), String> {
    let p = safe(root, path)?;
    if fs::read_to_string(&p).map_err(err)? != original {
        return Err("The file changed externally. Reload before saving; your edits are retained.".into());
    }
    write_document(&p, content.as_bytes())
}
fn write_document(p: &Path, content: &[u8]) -> Result<(), String> {
    let permissions = fs::metadata(p).map_err(err)?.permissions();
    let mut temporary = tempfile::NamedTempFile::new_in(p.parent().ok_or("Invalid file path")?).map_err(err)?;
    temporary.as_file().set_permissions(permissions).map_err(err)?;
    temporary.write_all(content).map_err(err)?;
    temporary.as_file().sync_all().map_err(err)?;
    temporary.persist(p).map_err(err)?;
    Ok(())
}
pub fn create_skill(root: &Path, name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 80
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err("Use lowercase letters, digits and hyphens; at most 80 characters".into());
    }
    let dir = root.join(name);
    fs::create_dir(&dir).map_err(err)?;
    fs::write(dir.join("SKILL.md"),format!("---\nname: {name}\ndescription: 描述该 Skill 的用途和触发条件\n---\n\n# {name}\n\n## 步骤\n\n1. 确认用户需求。\n2. 执行操作并验证结果。\n\n## 参考资料\n\n在这里添加 Markdown 相对链接。\n")).map_err(err)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_links_and_inline_paths_with_lines() {
        let refs = references("# 测试\n\n[规范](refs/api.md#auth)\n运行 `scripts/check.py`\n");
        assert_eq!(
            refs,
            vec![("refs/api.md".into(), 3), ("scripts/check.py".into(), 4)]
        );
    }
    #[test]
    fn ignores_external_links_and_commands() {
        assert!(references("[web](https://example.com) [anchor](#section) `bash scripts/a.py`\n```sh\npython test.py\n```").is_empty());
    }
    #[test]
    fn resolves_reference_style_links() {
        assert_eq!(
            references("[API][api]\n\n[api]: refs/api.md\n")[0].0,
            "refs/api.md"
        );
    }
    #[test]
    fn metadata_preserves_chinese() {
        let (title, summary, headings) =
            metadata("---\nname: test\n---\n# 中文标题\n\n这是**摘要**。\n\n## 操作步骤\n");
        assert_eq!(title, "中文标题");
        assert_eq!(summary, "这是摘要。");
        assert_eq!(headings, vec!["操作步骤"]);
    }
    #[test]
    fn create_skill_validates_name_and_does_not_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        assert!(create_skill(&root, "../bad").is_err());
        create_skill(&root, "my-skill").unwrap();
        let path = root.join("my-skill/SKILL.md");
        assert!(fs::read_to_string(&path).unwrap().contains("name: my-skill"));
        fs::write(&path, "user content").unwrap();
        assert!(create_skill(&root, "my-skill").is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "user content");
    }
    #[test]
    fn save_rejects_external_changes_and_keeps_disk_content() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::write(root.join("file.md"), "external").unwrap();
        assert!(save_document(&root, "file.md", "local edits", "old").is_err());
        assert_eq!(fs::read_to_string(root.join("file.md")).unwrap(), "external");
        save_document(&root, "file.md", "new", "external").unwrap();
        assert_eq!(fs::read_to_string(root.join("file.md")).unwrap(), "new");
    }
    #[test]
    fn save_replaces_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.md");
        fs::write(&path, "old").unwrap();
        write_document(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }
    #[cfg(unix)]
    #[test]
    fn save_preserves_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("script.sh");
        fs::write(&path, "old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o751)).unwrap();
        write_document(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o751);
    }
    #[test]
    fn workspace_boundary_is_enforced() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        fs::write(temp.path().join("outside.md"), "secret").unwrap();
        fs::write(workspace.join("inside.md"), "ok").unwrap();
        let workspace = workspace.canonicalize().unwrap();
        assert!(safe(&workspace, "inside.md").is_ok());
        assert!(safe(&workspace, "../outside.md").is_err());
        assert!(safe(&workspace, "missing.md").is_err());
    }
}
