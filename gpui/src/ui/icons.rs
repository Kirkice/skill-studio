use gpui::*;
use super::theme::rgb;
use std::borrow::Cow;

// Original 24px outline assets, embedded in the binary (no filesystem or font dependency).
#[derive(Clone, Copy)]
pub enum Icon { FolderOpen, Plus, Command, Refresh, Save, More, Focus, BrokenLink, Layout, Minus, ZoomIn, ZoomOut, File, Code, Skill, Search, ChevronDown, Close, Import, Export, Undo, Check, Graph, Layers, ArrowRight, Branch, Settings, Wrap, Numbers, Eye, Link, ArrowLeft, ResetZoom, Replace, ReplaceAll, ToggleOn, ToggleOff, Maximize, Restore, ChevronRight, Folder, CollapseAll, Image, Terminal, Copy, Paste, Stop, Mouse, Play }
impl Icon {
    pub const ALL: &[Self] = &[Self::FolderOpen,Self::Plus,Self::Command,Self::Refresh,Self::Save,Self::More,Self::Focus,Self::BrokenLink,Self::Layout,Self::Minus,Self::ZoomIn,Self::ZoomOut,Self::File,Self::Code,Self::Skill,Self::Search,Self::ChevronDown,Self::Close,Self::Import,Self::Export,Self::Undo,Self::Check,Self::Graph,Self::Layers,Self::ArrowRight,Self::Branch,Self::Settings,Self::Wrap,Self::Numbers,Self::Eye,Self::Link,Self::ArrowLeft,Self::ResetZoom,Self::Replace,Self::ReplaceAll,Self::ToggleOn,Self::ToggleOff,Self::Maximize,Self::Restore,Self::ChevronRight,Self::Folder,Self::CollapseAll,Self::Image,Self::Terminal,Self::Copy,Self::Paste,Self::Stop,Self::Mouse,Self::Play];
    fn name(self)-> &'static str { match self {
        Self::FolderOpen=>"folder-open",Self::Plus=>"plus",Self::Command=>"command",Self::Refresh=>"refresh",Self::Save=>"save",Self::More=>"more",Self::Focus=>"focus",Self::BrokenLink=>"broken-link",Self::Layout=>"layout",Self::Minus=>"minus",Self::ZoomIn=>"zoom-in",Self::ZoomOut=>"zoom-out",Self::File=>"file",Self::Code=>"code",Self::Skill=>"skill",Self::Search=>"search",Self::ChevronDown=>"chevron-down",Self::Close=>"close",Self::Import=>"import",Self::Export=>"export",Self::Undo=>"undo",Self::Check=>"check",Self::Graph=>"graph",Self::Layers=>"layers",Self::ArrowRight=>"arrow-right",Self::Branch=>"branch",Self::Settings=>"settings",Self::Wrap=>"wrap",Self::Numbers=>"numbers",Self::Eye=>"eye",Self::Link=>"link",Self::ArrowLeft=>"arrow-left",Self::ResetZoom=>"reset-zoom",Self::Replace=>"replace",Self::ReplaceAll=>"replace-all",Self::ToggleOn=>"toggle-on",Self::ToggleOff=>"toggle-off",Self::Maximize=>"maximize",Self::Restore=>"restore",Self::ChevronRight=>"chevron-right",Self::Folder=>"folder",Self::CollapseAll=>"collapse-all",Self::Image=>"image",Self::Terminal=>"terminal",Self::Copy=>"copy",Self::Paste=>"paste",Self::Stop=>"stop",Self::Mouse=>"mouse",Self::Play=>"play"
    }}
    fn paths(self)-> &'static str { match self {
        Self::FolderOpen=>"M3 7V5h6l2 2h10v4 M3 8v12h16l3-9H7l-4 9",
        Self::Plus=>"M12 5v14 M5 12h14",
        Self::Command=>"M9 9V5a2 2 0 1 0-2 2h10a2 2 0 1 0-2-2v14a2 2 0 1 0 2-2H7a2 2 0 1 0 2 2V9",
        Self::Refresh=>"M20 7v5h-5 M4 17v-5h5 M20 12a8 8 0 0 0-14-5 M4 12a8 8 0 0 0 14 5",
        Self::Save=>"M5 3h12l4 4v14H3V3h2 M7 3v6h10V3 M7 21v-8h10v8",
        Self::More=>"M5 12h.01 M12 12h.01 M19 12h.01",
        Self::Focus=>"M8 3H3v5 M16 3h5v5 M21 16v5h-5 M8 21H3v-5 M8 12a4 4 0 1 0 8 0 4 4 0 1 0-8 0",
        Self::BrokenLink=>"M9 8l3-3a5 5 0 0 1 7 7l-3 3 M15 16l-3 3a5 5 0 0 1-7-7l3-3 M9 15l6-6 M3 3l18 18",
        Self::Layout=>"M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
        Self::Minus=>"M5 12h14",
        Self::ZoomIn=>"M10 3a7 7 0 1 0 0 14 7 7 0 1 0 0-14 M15 15l6 6 M7 10h6 M10 7v6",
        Self::ZoomOut=>"M10 3a7 7 0 1 0 0 14 7 7 0 1 0 0-14 M15 15l6 6 M7 10h6",
        Self::File=>"M14 3H5v18h14V8z M14 3v5h5 M8 12h8 M8 16h6",
        Self::Code=>"M8 6l-6 6 6 6 M16 6l6 6-6 6 M14 3l-4 18",
        Self::Skill=>"M12 2l9 10-9 10L3 12z M8 12h8 M12 8v8",
        Self::Search=>"M10 3a7 7 0 1 0 0 14 7 7 0 1 0 0-14 M15 15l6 6",
        Self::ChevronDown=>"M6 9l6 6 6-6",
        Self::Close=>"M6 6l12 12 M18 6L6 18",
        Self::Import=>"M12 3v12 M7 10l5 5 5-5 M4 15v6h16v-6",
        Self::Export=>"M12 16V3 M7 8l5-5 5 5 M4 15v6h16v-6",
        Self::Undo=>"M3 10h11a6 6 0 0 1 0 12 M8 5l-5 5 5 5",
        Self::Check=>"M4 12l5 5L20 6",
        Self::Graph=>"M4 4h5v5H4z M15 3h5v5h-5z M14 15h6v6h-6z M9 6h6 M6 9v9h8",
        Self::Layers=>"M12 3l10 5-10 5L2 8z M2 12l10 5 10-5 M2 16l10 5 10-5",
        Self::ArrowRight=>"M4 12h16 M14 6l6 6-6 6",
        Self::Branch=>"M6 3v18 M6 12h7a5 5 0 0 0 5-5V3 M3 3h6 M3 21h6 M15 3h6",
        Self::Settings=>"M4 7h16 M4 17h16 M8 4v6 M16 14v6",
        Self::Wrap=>"M3 5h18 M3 10h14a4 4 0 0 1 0 8h-5 M15 15l-3 3 3 3 M3 15h4",
        Self::Numbers=>"M3 4h1v5 M3 13h3l-3 5h3 M10 6h11 M10 12h11 M10 18h11",
        Self::Eye=>"M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z M15 12a3 3 0 1 0-6 0 3 3 0 1 0 6 0",
        Self::Link=>"M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-2 2 M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l2-2",
        Self::ArrowLeft=>"M20 12H4 M10 6l-6 6 6 6",
        Self::ResetZoom=>"M4 9a8 8 0 1 1 1 9 M4 4v5h5 M10 10h3v6 M10 16h6",
        Self::Replace=>"M3 4h7v7H3z M15 4h6 M18 1l3 3-3 3 M3 18h17 M16 14l4 4-4 4",
        Self::ReplaceAll=>"M3 3h6v6H3z M15 3h6v6h-6z M3 15h6v6H3z M14 17h7 M18 14l3 3-3 3",
        Self::ToggleOn=>"M8 5h8a7 7 0 0 1 0 14H8A7 7 0 0 1 8 5z M19 12a3 3 0 1 0-6 0 3 3 0 1 0 6 0",
        Self::ToggleOff=>"M8 5h8a7 7 0 0 1 0 14H8A7 7 0 0 1 8 5z M11 12a3 3 0 1 0-6 0 3 3 0 1 0 6 0",
        Self::Maximize=>"M4 4h16v16H4z",
        Self::Restore=>"M8 8h12v12H8z M4 16V4h12",
        Self::ChevronRight=>"M9 6l6 6-6 6",
        Self::Folder=>"M3 5h6l2 2h10v13H3z",
        Self::CollapseAll=>"M8 8h12v12H8z M4 16V4h12 M11 14h6",
        Self::Image=>"M3 3h18v18H3z M3 17l6-6 4 4 3-3 5 5 M16 7h.01",
        Self::Terminal=>"M3 4h18v16H3z M6 9l3 3-3 3 M12 15h5",
        Self::Copy=>"M8 8h12v13H8z M4 16V3h12v2",
        Self::Paste=>"M9 5H5v16h14V5h-4 M9 3h6v4H9z",
        Self::Stop=>"M6 6h12v12H6z",
        Self::Play=>"M7 4l13 8-13 8z",
        Self::Mouse=>"M6 9a6 6 0 0 1 12 0v6a6 6 0 0 1-12 0z M12 3v7 M9 8l3 3 3-3",
    }}
    fn path(self)->String {format!("icons/{}.svg",self.name())}
    fn bytes(self)->Vec<u8> {let transform=if matches!(self,Self::Command){r#" transform="rotate(90 12 12)""#}else{""};format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="{}"{transform} fill="none" stroke="black" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>"#,self.paths()).into_bytes()}
}
pub fn icon(icon: Icon)->Svg {svg().path(icon.path()).size(px(16.)).flex_none().text_color(rgb(super::theme::MUTED))}
pub fn file_icon(kind:&str)->Icon {match kind {"skill"|"main"=>Icon::Skill,"script"=>Icon::Code,"subflow"=>Icon::Branch,"missing"=>Icon::BrokenLink,_=>Icon::File}}
pub fn command_icon(command:crate::commands::Command)->Icon {use crate::commands::Command::*;match command {Open=>Icon::FolderOpen,Refresh=>Icon::Refresh,Save=>Icon::Save,SaveLayout=>Icon::Layout,Import=>Icon::Import,Export=>Icon::Export,Reset=>Icon::Undo,Create=>Icon::Plus,Settings=>Icon::Settings,Close=>Icon::Close}}
pub fn app_logo()->Img {
    use std::sync::{Arc,OnceLock};
    static LOGO:OnceLock<Arc<Image>>=OnceLock::new();
    img(LOGO.get_or_init(||Arc::new(Image::from_bytes(ImageFormat::Svg,include_bytes!("../../../app-icon.svg").to_vec()))).clone())
}
pub struct Assets;
impl AssetSource for Assets {
    fn load(&self,path:&str)->Result<Option<Cow<'static,[u8]>>> {Ok(Icon::ALL.iter().copied().find(|i|i.path()==path).map(|i|Cow::Owned(i.bytes())))}
    fn list(&self,path:&str)->Result<Vec<SharedString>> {Ok(Icon::ALL.iter().filter(|i|i.path().starts_with(path)).map(|i|i.path().into()).collect())}
}

#[cfg(test)] mod tests {
    use super::{Assets,Icon};
    use gpui::AssetSource;
    #[test] fn every_icon_has_unique_embedded_asset() {
        let assets=Assets;let paths=assets.list("icons/").unwrap();
        assert_eq!(paths.len(),Icon::ALL.len());
        let unique:std::collections::HashSet<_>=paths.iter().collect();assert_eq!(unique.len(),paths.len());
        for path in paths {let data=assets.load(&path).unwrap().unwrap();assert!(std::str::from_utf8(&data).unwrap().contains("stroke-linecap=\"round\""));}
        assert!(assets.load("icons/missing.svg").unwrap().is_none());
    }
}
