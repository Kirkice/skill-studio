use serde::{Deserialize, Serialize};
use std::io::Write;

pub const THEMES: &[&str] = &["graphite", "ink", "moss", "paper"];
pub const ACCENTS: &[(&str,u32)] = &[("Blue",0x96b2d7),("Violet",0xb5a2ce),("Teal",0x8cbdc3),("Green",0xa6bd91),("Amber",0xd8be87),("Rose",0xdc8c95)];
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {pub language:crate::i18n::Language,pub theme:String,pub accent:u32,pub font_size:u16,pub terminal_font_size:u16,pub terminal_line_height:f32,pub soft_wrap:bool,pub line_numbers:bool,pub pi_launcher:String}
impl Default for Settings {fn default()->Self {Self {language:Default::default(),theme:"graphite".into(),accent:ACCENTS[0].1,font_size:14,terminal_font_size:14,terminal_line_height:1.,soft_wrap:true,line_numbers:true,pi_launcher:String::new()}}}
impl Settings {
 pub fn validate(&self)->Result<(),String> {
  if !THEMES.contains(&self.theme.as_str()) {return Err("Unknown theme preset".into());}
  if !ACCENTS.iter().any(|(_,color)|*color==self.accent) {return Err("Unsupported accent color".into());}
  if !(10..=24).contains(&self.font_size) {return Err("Editor font size must be between 10 and 24".into());}
  if !(10..=26).contains(&self.terminal_font_size){return Err("Terminal font size must be between 10 and 26".into());}
  if !self.terminal_line_height.is_finite()||!(1.0..=1.6).contains(&self.terminal_line_height){return Err("Terminal line height must be between 1.0 and 1.6".into());}
  if self.pi_launcher.len()>4096||self.pi_launcher.contains(['\r','\n']){return Err("Pi launcher path is invalid".into());}
  Ok(())
 }
}
fn path()->Result<std::path::PathBuf,String> {if let Some(dir)=std::env::var_os("SKILL_STUDIO_CONFIG_DIR").filter(|d|!d.is_empty()){return Ok(std::path::PathBuf::from(dir).join("settings.json"));}Ok(dirs::config_dir().ok_or("User configuration directory is unavailable")?.join("skill-studio/settings.json"))}
pub fn load()->Result<Settings,String> {
 let bytes=match std::fs::read(path()?) {Ok(v)=>v,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(Settings::default()),Err(e)=>return Err(e.to_string())};
 if bytes.len()>65536 {return Err("Settings file is too large".into());}
 let settings:Settings=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;settings.validate()?;Ok(settings)
}
pub fn save(settings:&Settings)->Result<(),String> {
 settings.validate()?;let path=path()?;let parent=path.parent().ok_or("Invalid settings path")?;
 std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;
 let mut file=tempfile::NamedTempFile::new_in(parent).map_err(|e|e.to_string())?;
 serde_json::to_writer_pretty(&mut file,settings).map_err(|e|e.to_string())?;file.flush().map_err(|e|e.to_string())?;file.as_file().sync_all().map_err(|e|e.to_string())?;file.persist(path).map_err(|e|e.to_string())?;Ok(())
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn defaults_and_round_trip(){let s=Settings::default();s.validate().unwrap();let copy:Settings=serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();assert_eq!(copy.theme,s.theme);assert_eq!(copy.font_size,14);assert!(copy.soft_wrap);}
 #[test] fn missing_wrap_defaults_to_enabled_and_explicit_choice_is_preserved(){
  let legacy:Settings=serde_json::from_str("{}").unwrap();assert!(legacy.soft_wrap);
  let explicit:Settings=serde_json::from_str(r#"{"soft_wrap":false}"#).unwrap();assert!(!explicit.soft_wrap);
 }
 #[test]fn legacy_editor_size_does_not_change_terminal_size(){let s:Settings=serde_json::from_str(r#"{"font_size":20}"#).unwrap();assert_eq!(s.font_size,20);assert_eq!(s.terminal_font_size,14);assert_eq!(s.terminal_line_height,1.);}
 #[test]fn rejects_invalid_terminal_geometry(){let mut s=Settings::default();s.terminal_font_size=40;assert!(s.validate().is_err());s.terminal_font_size=14;s.terminal_line_height=f32::NAN;assert!(s.validate().is_err());s.terminal_line_height=0.5;assert!(s.validate().is_err());}
 #[test] fn rejects_invalid_preferences(){let mut s=Settings::default();s.font_size=500;assert!(s.validate().is_err());s.font_size=13;s.theme="unknown".into();assert!(s.validate().is_err());}
}
