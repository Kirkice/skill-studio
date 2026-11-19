use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Write, path::PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Position { pub x: f32, pub y: f32 }
#[derive(Default, Serialize, Deserialize)]
pub struct LayoutFile { pub version: u32, pub positions: Positions, #[serde(default)] pub roles: Roles }
pub type Positions = BTreeMap<String, Position>;
pub type Roles = BTreeMap<String, String>;
pub const MAX_COORDINATE: f32 = 100_000.;
pub const ROLES: &[&str] = &["main", "skill", "subflow", "document", "script", "file", "missing"];
pub fn decode(bytes: &[u8]) -> Result<LayoutFile, String> {
    if bytes.len() > 2_000_000 { return Err("Layout file exceeds 2 MB".into()); }
    let file: LayoutFile = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if file.version != 1 || file.positions.values().any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > MAX_COORDINATE || p.y.abs() > MAX_COORDINATE) || file.roles.values().any(|r| !ROLES.contains(&r.as_str())) { return Err("Invalid layout version, coordinates or roles".into()); }
    // The camera supports every quadrant. Preserve signed coordinates exactly;
    // translating them here would make saved nodes jump after reopening.
    Ok(file)
}

fn path(root: &str) -> Result<PathBuf, String> {
    let config = dirs::config_dir().ok_or("User configuration directory is unavailable")?;
    let digest = Sha256::digest(root.as_bytes());
    Ok(config.join("skill-studio").join("layouts").join(format!("{digest:x}.json")))
}
pub fn load(root: &str) -> Result<LayoutFile, String> {
    let path = path(root)?;
    let bytes = match std::fs::read(path) { Ok(bytes) => bytes, Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LayoutFile {version: 1, ..Default::default()}), Err(e) => return Err(e.to_string()) };
    decode(&bytes)
}
pub fn save(root: &str, positions: &Positions, roles: &Roles) -> Result<(), String> {
    write_file(&path(root)?, positions, roles)
}
pub fn write_file(path: &std::path::Path, positions: &Positions, roles: &Roles) -> Result<(), String> {
    // Validate BEFORE replacing an existing file, preserving signed coordinates.
    let bytes = serde_json::to_vec_pretty(&LayoutFile { version: 1, positions: positions.clone(), roles: roles.clone() }).map_err(|e| e.to_string())?;
    let validated = decode(&bytes)?;
    let parent = path.parent().ok_or("Invalid layout directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut temp, &validated).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn automatic(graph: &crate::core::Graph, visible: &std::collections::HashSet<String>) -> Positions {
    use std::collections::{HashSet, VecDeque};
    let incoming: HashSet<_> = graph.edges.iter().filter(|e| visible.contains(&e.source) && visible.contains(&e.target)).map(|e| e.target.as_str()).collect();
    let mut depth = BTreeMap::new();
    let mut queue = VecDeque::new();
    for n in graph.nodes.iter().filter(|n| visible.contains(&n.id) && !incoming.contains(n.id.as_str())) {depth.insert(n.id.clone(),0_usize);queue.push_back(n.id.clone());}
    let visit=|queue: &mut VecDeque<String>, depth: &mut BTreeMap<String,usize>| {
        while let Some(source)=queue.pop_front() { let next=depth[&source]+1;
            for e in graph.edges.iter().filter(|e| e.source==source && visible.contains(&e.target)) {
                if !depth.contains_key(&e.target) {depth.insert(e.target.clone(),next);queue.push_back(e.target.clone());}
            }
        }
    };
    visit(&mut queue,&mut depth);
    for n in graph.nodes.iter().filter(|n|visible.contains(&n.id)) {if !depth.contains_key(&n.id) {depth.insert(n.id.clone(),0);queue.push_back(n.id.clone());visit(&mut queue,&mut depth);}}
    let mut rows=BTreeMap::<usize,usize>::new();
    graph.nodes.iter().filter(|n|visible.contains(&n.id)).map(|n| {let col=depth[&n.id];let row=rows.entry(col).or_default();let position=Position{x:40.+col as f32*300.,y:40.+*row as f32*240.};*row+=1;(n.id.clone(),position)}).collect()
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn automatic_layout_handles_cycles() {
  use crate::core::{Graph, Node, Edge};
  let nodes=["a","b"].iter().map(|id|Node{id:(*id).into(),label:(*id).into(),kind:"skill".into(),title:String::new(),summary:String::new(),headings:vec![]}).collect();
  let graph=Graph{root:String::new(),entries:vec![],nodes,edges:vec![Edge{source:"a".into(),target:"b".into(),line:1,missing:false},Edge{source:"b".into(),target:"a".into(),line:1,missing:false}],warnings:vec![],flows:vec![]};
  let positions=automatic(&graph,&std::collections::HashSet::from(["a".into(),"b".into()]));
  assert_eq!(positions.len(),2); assert!(positions["b"].x>positions["a"].x);
 }
 #[test] fn imports_web_layout_and_preserves_negative_positions() {
  let parsed = decode(br#"{"version":1,"positions":{"a":{"x":-20,"y":-10},"b":{"x":100,"y":40}},"roles":{"a":"subflow"}}"#).unwrap();
  assert_eq!(parsed.positions["a"], Position{x:-20.,y:-10.});
  assert_eq!(parsed.positions["b"], Position{x:100.,y:40.});
  assert_eq!(parsed.roles["a"], "subflow");
 }
 #[test] fn validates_signed_coordinate_range() {
  assert!(decode(br#"{"version":1,"positions":{"a":{"x":-100000,"y":0},"b":{"x":100000,"y":0}}}"#).is_ok());
  assert!(decode(br#"{"version":1,"positions":{"a":{"x":-100001,"y":0}}}"#).is_err());
 }
 #[test] fn saved_layout_is_reloadable_and_invalid_write_preserves_file() {
  let dir=tempfile::tempdir().unwrap();let path=dir.path().join("layout.json");
  let positions=Positions::from([("a".into(),Position{x:-20.,y:-10.}),("b".into(),Position{x:100.,y:40.})]);
  write_file(&path,&positions,&Roles::new()).unwrap();
  let bytes=std::fs::read(&path).unwrap();let loaded=decode(&bytes).unwrap();
  assert_eq!(loaded.positions,positions);
  let bad=Positions::from([("a".into(),Position{x:f32::NAN,y:0.})]);
  assert!(write_file(&path,&bad,&Roles::new()).is_err());
  assert_eq!(std::fs::read(path).unwrap(),bytes);
 }
 #[test] fn rejects_unknown_role() { assert!(decode(br#"{"version":1,"positions":{},"roles":{"a":"bad"}}"#).is_err()); }
 #[test] fn invalid_json_rejected() { assert!(serde_json::from_str::<LayoutFile>("{\"version\":1,\"positions\":{\"a\":{\"x\":\"bad\",\"y\":1}}}").is_err()); }
}
