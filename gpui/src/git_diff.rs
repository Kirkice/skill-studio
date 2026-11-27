use std::{collections::HashMap, path::Path, process::Command};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    Added,
    Modified,
    Deleted,
}
impl Change {
    pub fn color(self) -> u32 {
        match self {
            Self::Added => 0x74c68d,
            Self::Modified => 0xdf7984,
            Self::Deleted => 0x666c76,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "+ Added",
            Self::Modified => "~ Modified",
            Self::Deleted => "− Deleted",
        }
    }
}
#[derive(Debug, Default)]
pub struct File {
    pub change: Option<Change>,
    pub lines: Vec<(usize, usize, Change)>,
    pub head_frontmatter_tokens: Option<usize>,
    pub head_body_tokens: Option<usize>,
    pub head_total_tokens: Option<usize>,
}
pub type Snapshot = HashMap<String, File>;
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(crate::terminal::shell_cwd(root))
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["--literal-pathspecs", "-c", "core.quotePath=false"])
        .args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().into())
    }
}
fn hunks(patch: &str) -> Vec<(usize, usize, Change)> {
    patch
        .lines()
        .filter_map(|line| {
            let mut words = line.strip_prefix("@@ -")?.split_whitespace();
            let old = words.next()?;
            let new = words.next()?.strip_prefix('+')?;
            if words.next()? != "@@" {
                return None;
            }
            let parse = |s: &str| -> Option<(usize, usize)> {
                let (a, b) = s.split_once(',').unwrap_or((s, "1"));
                Some((a.parse().ok()?, b.parse().ok()?))
            };
            let (_, removed) = parse(old)?;
            let (row, count) = parse(new)?;
            Some((
                row,
                count,
                if count == 0 {
                    Change::Deleted
                } else if removed == 0 {
                    Change::Added
                } else {
                    Change::Modified
                },
            ))
        })
        .collect()
}
pub fn scan(root: &Path, docs: &[String]) -> Result<Snapshot, String> {
    git(root, &["rev-parse", "--verify", "HEAD"])
        .map_err(|_| "Git Diff needs a Git repository with an initial commit".to_string())?;
    let names = git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--name-status",
            "--no-renames",
            "--relative",
            "-z",
            "HEAD",
            "--",
            ".",
        ],
    )?;
    let mut fields = names.split(|b| *b == 0);
    let mut files = Snapshot::new();
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let Ok(path) = std::str::from_utf8(path) else {
            continue;
        };
        let change = match status {
            b"A" => Change::Added,
            b"D" => Change::Deleted,
            _ => Change::Modified,
        };
        files.insert(
            path.into(),
            File {
                change: Some(change),
                lines: vec![],
                head_frontmatter_tokens: None,
                head_body_tokens: None,
                head_total_tokens: None,
            },
        );
    }
    for name in git(
        root,
        &[
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            ".",
        ],
    )?
    .split(|b| *b == 0)
    {
        if let Ok(name) = std::str::from_utf8(name) {
            if !name.is_empty() {
                files.insert(
                    name.into(),
                    File {
                        change: Some(Change::Added),
                        lines: vec![],
                        head_frontmatter_tokens: Some(0),
                        head_body_tokens: Some(0),
                        head_total_tokens: Some(0),
                    },
                );
            }
        }
    }
    for id in docs {
        if let Some(file) = files
            .get_mut(id)
            .filter(|f| f.change == Some(Change::Modified))
        {
            let patch = git(
                root,
                &[
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--unified=0",
                    "HEAD",
                    "--",
                    id,
                ],
            )?;
            file.lines = hunks(&String::from_utf8_lossy(&patch));
            if let Ok(raw) = git(root, &["show", &format!("HEAD:{id}")]) {
                let text = String::from_utf8_lossy(&raw);
                let body = crate::preview::body(&text);
                let fm_len = text.len().saturating_sub(body.len());
                let fm_tok = crate::core::estimate_tokens(&text[..fm_len]);
                let body_tok = crate::core::estimate_tokens(&body);
                file.head_frontmatter_tokens = Some(fm_tok);
                file.head_body_tokens = Some(body_tok);
                file.head_total_tokens = Some(fm_tok + body_tok);
            }
        } else if let Some(file) = files
            .get_mut(id)
            .filter(|f| f.change == Some(Change::Added))
        {
            file.head_frontmatter_tokens = Some(0);
            file.head_body_tokens = Some(0);
            file.head_total_tokens = Some(0);
        }
    }
    Ok(files)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hunk_kinds() {
        assert_eq!(
            hunks("@@ -0,0 +1,2 @@\n@@ -4 +5 @@\n@@ -8 +8,0 @@"),
            vec![
                (1, 2, Change::Added),
                (5, 1, Change::Modified),
                (8, 0, Change::Deleted)
            ]
        );
    }
}
