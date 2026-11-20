use gpui::{App,Hsla,Rgba};
use gpui_base::{ColorTokens,Theme,ThemeAppearance};
use gpui_base::input::InputEditorStyle;
use crate::settings::Settings;
use std::cell::RefCell;

// Semantic keys. Resolved using Herdr settings.rs's palette/chrome formulas.
pub const BG:u32=0x17191d;pub const PANEL:u32=0x1d1f24;pub const RAISED:u32=0x25282f;pub const HOVER:u32=0x30343d;pub const BORDER:u32=0x30343c;pub const TEXT:u32=0xd2d4d9;pub const MUTED:u32=0x818793;pub const ACCENT:u32=0x96b2d7;pub const SELECTED:u32=0x303e52;pub const GREEN:u32=0xa6bd91;pub const WARNING:u32=0xd8be87;
pub const DIM:u32=0x696a6d;pub const CANVAS:u32=0x131518;pub const POPOVER:u32=0x25272b;
pub const UI_FONT:&str="Geist";pub const MONO_FONT:&str="JetBrains Mono";pub const UI_SIZE:f32=13.;
pub fn load_fonts(cx:&mut App)->Result<(),String> {
 cx.text_system().add_fonts(vec![
  std::borrow::Cow::Borrowed(include_bytes!("../../assets/fonts/Geist.ttf").as_slice()),
  std::borrow::Cow::Borrowed(include_bytes!("../../assets/fonts/JetBrainsMono-Variable.ttf").as_slice()),
  std::borrow::Cow::Borrowed(include_bytes!("../../assets/fonts/JetBrainsMono-Italic-Variable.ttf").as_slice()),
 ]).map_err(|e|e.to_string())
}
thread_local! {static ACTIVE:RefCell<Settings>=RefCell::new(Settings::default());}
pub fn is_light()->bool {ACTIVE.with(|s|s.borrow().theme=="paper")}
fn mix(a:u32,b:u32,t:f32)->u32 {
 [16,8,0].into_iter().map(|shift|{let a=((a>>shift)&255) as f32;let b=((b>>shift)&255) as f32;((a+(b-a)*t).round() as u32)<<shift}).fold(0,|a,b|a|b)
}
fn resolve(value:u32,s:&Settings)->u32 {
 let light=s.theme=="paper";
 let bg=match s.theme.as_str(){"ink"=>0x111720,"moss"=>0x151c19,"paper"=>0xffffff,_=>0x17191d};
 let fg=if light{0x1b1b1e}else{0xeeeeef};
 match value {
  BG=>bg,PANEL=>if light{mix(bg,0xe8e8e2,0.6)}else{mix(bg,0,0.32)},
  CANVAS=>if light{mix(bg,0xe8e8e2,0.2)}else{mix(bg,0,0.16)},
  POPOVER=>if light{bg}else{mix(bg,0xffffff,0.06)},
  RAISED=>if light{mix(bg,0,0.035)}else{mix(bg,0xffffff,0.09)},
  TEXT=>fg,MUTED=>mix(bg,fg,if light{0.66}else{0.6}),DIM=>mix(bg,fg,if light{0.46}else{0.38}),
  // Solid approximations for contrast tests; painting uses the alpha tokens below.
  HOVER=>mix(bg,fg,0.055),BORDER=>mix(bg,fg,0.094),SELECTED=>mix(bg,fg,0.09),
  ACCENT if s.accent==ACCENT=>match s.theme.as_str(){"ink"=>0x91b6ee,"moss"=>0x99b9cd,"paper"=>0x1f62d0,_=>s.accent},
  ACCENT=>s.accent,
  GREEN if light=>0x1a7f37,WARNING if light=>0x9a6700,
  0x545963=>mix(bg,fg,0.38),0x21242a=>mix(bg,fg,0.035),0x252e3c=>mix(bg,fg,0.09),0x2b2e35=>mix(bg,fg,0.07),
  0x393126 if light=>0xfff1d6,0xa3a8b2=>mix(bg,fg,if light{0.66}else{0.6}),other=>other
 }
}
pub fn preview(name:&str)->[u32;3] {
 let settings=Settings{theme:name.into(),..Default::default()};
 [resolve(BG,&settings),resolve(PANEL,&settings),resolve(RAISED,&settings)]
}
pub fn rgb(value:u32)->Rgba {
 ACTIVE.with(|s|{let s=s.borrow();let light=s.theme=="paper";
  match value {
   HOVER=>gpui::rgba(if light{0x0000000c}else{0xffffff0e}),
   BORDER=>gpui::rgba(if light{0x00000017}else{0xffffff18}),
   SELECTED=>gpui::rgba(if light{0x00000012}else{0xffffff17}),
   _=>gpui::rgb(resolve(value,&s))
  }
 })
}
pub fn install(settings:&Settings,cx:&mut App) {
 ACTIVE.with(|s|*s.borrow_mut()=settings.clone());let light=is_light();
 let t=Theme::global_mut(cx);t.appearance=if light{ThemeAppearance::Light}else{ThemeAppearance::Dark};
 let mut c=if light{ColorTokens::light()}else{ColorTokens::dark()};
 c.background=rgb(BG).into();c.foreground=rgb(TEXT).into();c.surface=rgb(BG).into();c.surface_foreground=rgb(TEXT).into();c.muted=rgb(RAISED).into();c.muted_foreground=rgb(MUTED).into();c.border=rgb(BORDER).into();c.input=gpui::rgba(if light{0x00000026}else{0xffffff24}).into();c.accent=rgb(HOVER).into();c.accent_foreground=rgb(TEXT).into();c.primary=rgb(ACCENT).into();c.primary_foreground=rgb(BG).into();c.selection=Hsla::from(gpui::rgb(if light{0xcfdcf5}else{0x394454}));
 t.tokens.colors=c;t.tokens.typography.sans=UI_FONT.into();t.tokens.typography.mono=MONO_FONT.into();
}
// Terminal colors are literal ANSI colors, not the UI's semantic foreground tokens.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct TerminalPalette{pub background:u32,pub foreground:u32,pub selection:u32,pub cursor:u32,pub ansi:[u32;16]}
pub fn terminal_palette()->TerminalPalette{ACTIVE.with(|s|terminal_palette_for(&s.borrow().theme))}
fn terminal_palette_for(name:&str)->TerminalPalette{
    let(background,foreground,selection,ansi)=match name{
        "ink"=>(0x111720,0xcdd6e4,0x344861,[0x414b5a,0xe58c9a,0x89c5aa,0xd9be82,0x91b6ee,0xb9a0e2,0x85c4d0,0xcdd6e4,0x7f91a7,0xf5a5b2,0xa5dec3,0xedd39c,0xaccbfa,0xd0b7f4,0xa0dce5,0xf0f5fc]),
        "moss"=>(0x151c19,0xcdd7ce,0x354a40,[0x435248,0xdf978d,0xa0c4a2,0xd4bc8b,0x99b9cd,0xbba4c8,0x8bbdb5,0xcdd7ce,0x82968a,0xf1afa3,0xb8dab9,0xe9d2a5,0xb1d0e2,0xd1bddc,0xa4d6ca,0xeff5ee]),
        "paper"=>(0xffffff,0x25272c,0xcfdcf5,[0x25272c,0xc62a2a,0x1a7f37,0x8f5d00,0x1f62d0,0x7a45c8,0x0f6f74,0x5f6068,0x7b7e87,0xd1242f,0x1f883d,0x9a6700,0x0969da,0x8250df,0x1b7c83,0x111214]),
        _=>(0x17191d,0xd2d4d9,0x394454,[0x45484f,0xdc8c95,0xa6bd91,0xd8be87,0x96b2d7,0xb5a2ce,0x8cbdc3,0xd2d4d9,0x818793,0xf0a5ad,0xbad5a2,0xead2a1,0xb0cbef,0xcbb7e6,0xa4d5db,0xf0f0f2]),
    };TerminalPalette{background,foreground,selection,cursor:foreground,ansi}
}
pub fn input_style()->InputEditorStyle {InputEditorStyle {foreground:rgb(TEXT).into(),muted_foreground:rgb(MUTED).into(),background:rgb(BG).into(),border:rgb(BORDER).into(),caret:rgb(TEXT).into(),selection:Hsla::from(gpui::rgb(if is_light(){0xcfdcf5}else{0x394454})),editor_gutter_background:Some(rgb(BG).into()),editor_active_line:Some(rgb(0x21242a).into()),..Default::default()}}
pub fn editor_style()->InputEditorStyle {input_style()}

#[cfg(test)] mod tests {
 use super::{editor_style,input_style,BG,TEXT,MUTED,DIM,PANEL,CANVAS,POPOVER,resolve};use gpui::{rgb,Hsla};use crate::settings::Settings;
 #[test] fn terminal_palette_uses_herdr_colors_not_ui_white(){let p=super::terminal_palette_for("graphite");assert_eq!(p.foreground,0xd2d4d9);assert_eq!(p.ansi[1],0xdc8c95);assert_eq!(p.selection,0x394454);assert_eq!(super::terminal_palette_for("paper").foreground,0x25272c);}
 #[test] fn editor_gutter_has_explicit_dark_background(){let s=editor_style();assert_eq!(s.editor_gutter_background,Some(rgb(BG).into()));assert_eq!(s.background,Hsla::from(rgb(BG)));assert!(s.foreground.l>s.background.l);}
 #[test] fn text_fields_have_visible_caret_and_selection(){let s=input_style();assert_eq!(s.caret,Hsla::from(rgb(0xeeeeef)));assert!(s.selection.a>0.);assert!(s.muted_foreground.l>s.background.l);}
 #[test] fn presets_preserve_foreground_contrast(){for name in crate::settings::THEMES {let s=Settings{theme:(*name).into(),..Default::default()};let fg=Hsla::from(rgb(resolve(TEXT,&s)));let bg=Hsla::from(rgb(resolve(BG,&s)));assert!((fg.l-bg.l).abs()>0.4);}}
 #[test] fn graphite_matches_herdr_chrome(){let s=Settings::default();assert_eq!(resolve(TEXT,&s),0xeeeeef);assert_eq!(resolve(MUTED,&s),0x98999b);assert_eq!(resolve(DIM,&s),0x696a6d);assert_eq!(resolve(PANEL,&s),0x101114);assert_eq!(resolve(CANVAS,&s),0x131518);assert_eq!(resolve(POPOVER,&s),0x25272b);}
}
