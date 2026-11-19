//! Pure Markdown preview preparation and workspace-bounded resource resolution.
use std::path::{Path,PathBuf};
pub fn supported(id:&str)->bool{
    Path::new(id).extension().and_then(|s|s.to_str()).is_some_and(|ext|ext.eq_ignore_ascii_case("md")||ext.eq_ignore_ascii_case("markdown"))
}
pub fn body(source:&str)->String{
    let mut lines=source.lines();
    if lines.next().is_some_and(|line|line.trim_end()=="---"){
        let mut offset=source.find('\n').map_or(source.len(),|n|n+1);
        let mut metadata=false;
        for line in lines {
            let len=source[offset..].find('\n').map_or(source.len()-offset,|n|n+1);
            if line.trim()=="---"{return if metadata{source[offset+len..].to_string()}else{source.to_string()};}
            metadata|=line.contains(':');
            offset+=len;
        }
    }
    source.to_string()
}
pub fn resource(root:&Path,document:&str,url:&str)->Result<PathBuf,String>{
    let url=url.split(['#','?']).next().unwrap_or("");
    if url.is_empty()||url.contains(':')||url.starts_with('/')||url.starts_with('\\'){
        return Err("Only relative workspace resources are allowed".into());
    }
    let root=root.canonicalize().map_err(|e|e.to_string())?;
    let document=root.join(document).canonicalize().map_err(|e|e.to_string())?;
    if !document.starts_with(&root){return Err("Document is outside the workspace".into());}
    let target=document.parent().ok_or("Invalid document path")?.join(url).canonicalize().map_err(|e|e.to_string())?;
    if !target.starts_with(&root)||!target.is_file(){return Err("Resource is outside the workspace or not a file".into());}
    Ok(target)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn frontmatter_is_hidden_but_body_preserved(){
  assert_eq!(body("---\nname: test\n---\n# 中文\n"),"# 中文\n");
  assert_eq!(body("---\r\nname: test\r\n---\r\n# 标题\r\n"),"# 标题\r\n");
  assert_eq!(body("---\nnot closed\n"),"---\nnot closed\n");
  assert_eq!(body("---\n# Heading\n\n---\nText"),"---\n# Heading\n\n---\nText");
  assert!(supported("SKILL.MD"));assert!(!supported("script.py"));
 }
 #[test]fn resources_stay_inside_workspace(){
  let dir=tempfile::tempdir().unwrap();let root=dir.path().join("workspace");std::fs::create_dir_all(&root).unwrap();
  std::fs::write(root.join("SKILL.md"),"").unwrap();std::fs::write(root.join("ref.md"),"").unwrap();std::fs::write(dir.path().join("secret.md"),"").unwrap();
  assert!(resource(&root,"SKILL.md","ref.md#section").is_ok());
  for url in ["../secret.md","https://example.com/a.png","file:///secret","javascript:alert(1)","/absolute","\\\\server\\file"]{assert!(resource(&root,"SKILL.md",url).is_err(),"{url}");}
 }
}
