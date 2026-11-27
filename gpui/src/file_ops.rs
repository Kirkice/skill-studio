use std::{
    fs, io,
    path::{Path, PathBuf},
};
#[derive(Clone)]
pub enum Op {
    Rename(String),
    Move(String),
    Copy(String),
    Delete,
}
fn invalid() -> io::Error {
    io::Error::other("Invalid or protected workspace path")
}
fn valid(name: &str) -> bool {
    !name.is_empty()
        && !matches!(name, "." | "..")
        && !name.eq_ignore_ascii_case(".git")
        && !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
}
fn resolve(root: &Path, id: &str) -> io::Result<PathBuf> {
    let mut p = root.to_owned();
    if !id.is_empty() {
        for component in id.split('/') {
            if !valid(component) {
                return Err(invalid());
            }
            p.push(component);
            let m = fs::symlink_metadata(&p)?;
            if m.file_type().is_symlink() {
                return Err(invalid());
            }
        }
    }
    let canonical = p.canonicalize()?;
    if !canonical.starts_with(root) {
        return Err(invalid());
    }
    Ok(p)
}
fn copy(source: &Path, dest: &Path) -> io::Result<()> {
    let m = fs::symlink_metadata(source)?;
    if m.is_dir() && !m.file_type().is_symlink() {
        fs::create_dir(dest)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            if !valid(&entry.file_name().to_string_lossy()) {
                return Err(invalid());
            }
            copy(&entry.path(), &dest.join(entry.file_name()))?;
        }
        Ok(())
    } else if m.is_file() {
        let mut input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)?;
        io::copy(&mut input, &mut output)?;
        Ok(())
    } else {
        Err(invalid())
    }
}
pub fn run(root: &Path, id: &str, op: Op) -> io::Result<Option<String>> {
    if id.is_empty() {
        return Err(invalid());
    }
    let root = root.canonicalize()?;
    let source = resolve(&root, id)?;
    let copy_op = matches!(&op, Op::Copy(_));
    let (parent, mut name) = match op {
        Op::Delete => {
            if source.is_dir() {
                fs::remove_dir_all(source)?
            } else {
                fs::remove_file(source)?
            }
            return Ok(None);
        }
        Op::Rename(name) => (source.parent().ok_or_else(invalid)?.to_owned(), name),
        Op::Move(dir) | Op::Copy(dir) => (
            resolve(&root, &dir)?,
            source
                .file_name()
                .ok_or_else(invalid)?
                .to_string_lossy()
                .into_owned(),
        ),
    };
    if !valid(&name) || !parent.is_dir() || parent.starts_with(&source) {
        return Err(invalid());
    }
    let mut dest = parent.join(&name);
    if copy_op && dest == source {
        let original = Path::new(&name);
        let stem = if source.is_dir() {
            name.as_str()
        } else {
            original
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&name)
        };
        let ext = if source.is_dir() {
            String::new()
        } else {
            original
                .extension()
                .map(|s| format!(".{}", s.to_string_lossy()))
                .unwrap_or_default()
        };
        name=(1..=1000).map(|i|format!("{stem} - Copy{}{ext}",if i==1{String::new()}else{format!(" ({i})")})).find(|n|matches!(fs::symlink_metadata(parent.join(n)),Err(e) if e.kind()==io::ErrorKind::NotFound)).ok_or_else(invalid)?;
        dest = parent.join(&name);
    }
    if dest == source {
        return Ok(Some(id.into()));
    }
    match fs::symlink_metadata(&dest) {
        Ok(_) => return Err(io::Error::other("Destination already exists")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    if copy_op {
        let temp = tempfile::tempdir_in(&parent)?;
        let payload = temp.path().join("payload");
        copy(&source, &payload)?;
        fs::rename(payload, &dest)?;
    } else {
        fs::rename(source, &dest)?;
    }
    Ok(Some(
        dest.strip_prefix(root)
            .map_err(|_| invalid())?
            .to_string_lossy()
            .replace('\\', "/"),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_edits() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        fs::create_dir(r.join("folder")).unwrap();
        fs::write(r.join("a.md"), "test").unwrap();
        assert!(run(r, "a.md", Op::Rename("../b".into())).is_err());
        assert_eq!(
            run(r, "a.md", Op::Copy("".into())).unwrap().unwrap(),
            "a - Copy.md"
        );
        assert_eq!(
            run(r, "a.md", Op::Move("folder".into())).unwrap().unwrap(),
            "folder/a.md"
        );
        assert!(run(r, "folder", Op::Move("folder".into())).is_err());
        run(r, "folder/a.md", Op::Rename("b.md".into())).unwrap();
        run(r, "folder/b.md", Op::Delete).unwrap();
        assert!(!r.join("folder/b.md").exists());
    }
}
