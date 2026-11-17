use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Write, path::PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Position { pub x: f32, pub y: f32 }
#[derive(Default, Serialize, Deserialize)]
struct LayoutFile { version: u32, positions: BTreeMap<String, Position> }
pub type Positions = BTreeMap<String, Position>;

fn path(root: &str) -> Result<PathBuf, String> {
    let config = dirs::config_dir().ok_or("找不到用户配置目录")?;
    let digest = Sha256::digest(root.as_bytes());
    Ok(config.join("skill-studio").join("layouts").join(format!("{digest:x}.json")))
}
pub fn load(root: &str) -> Result<Positions, String> {
    let path = path(root)?;
    let bytes = match std::fs::read(path) { Ok(bytes) => bytes, Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Positions::new()), Err(e) => return Err(e.to_string()) };
    if bytes.len() > 2_000_000 { return Err("布局文件超过 2MB".into()); }
    let file: LayoutFile = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if file.version != 1 || file.positions.values().any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x < 0. || p.y < 0. || p.x > 100000. || p.y > 100000.) { return Err("布局版本或坐标无效".into()); }
    Ok(file.positions)
}
pub fn save(root: &str, positions: &Positions) -> Result<(), String> {
    let path = path(root)?;
    let parent = path.parent().ok_or("无效的布局目录")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut temp, &LayoutFile { version: 1, positions: positions.clone() }).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn default_position(index: usize) -> Position { Position { x: 40. + (index % 3) as f32 * 300., y: 40. + (index / 3) as f32 * 160. } }

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn deterministic_grid() { assert_eq!(default_position(0), Position{x:40.,y:40.}); assert_eq!(default_position(3), Position{x:40.,y:200.}); }
 #[test] fn invalid_json_rejected() { assert!(serde_json::from_str::<LayoutFile>("{\"version\":1,\"positions\":{\"a\":{\"x\":\"bad\",\"y\":1}}}").is_err()); }
}
