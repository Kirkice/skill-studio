//! Bounded, content-aware local opening. Never launch files or convert encodings.
use std::{fs,io::{Cursor,Read},path::{Path,PathBuf},sync::Arc,time::SystemTime};

pub const TEXT_LIMIT:u64=2_000_000;
const IMAGE_LIMIT:u64=20_000_000;
const PIXEL_LIMIT:u64=8_000_000;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Stamp{pub bytes:u64,pub modified:Option<SystemTime>}
pub fn stamp(path:&Path)->Result<Stamp,String>{let meta=fs::metadata(path).map_err(|e|e.to_string())?;if !meta.is_file(){return Err("Not a regular file".into());}Ok(Stamp{bytes:meta.len(),modified:meta.modified().ok()})}
#[derive(Clone,Default)]
pub enum View{
    #[default] Text,
    Image{data:Arc<gpui::RenderImage>,width:u32,height:u32,stamp:Stamp},
    Unsupported{stamp:Stamp,reason:String},
}
impl View{
    pub fn is_text(&self)->bool{matches!(self,Self::Text)}
    pub fn stamp(&self)->Option<Stamp>{match self{Self::Text=>None,Self::Image{stamp,..}|Self::Unsupported{stamp,..}=>Some(*stamp)}}
}
pub struct Loaded{pub content:String,pub view:View}
fn unsupported(stamp:Stamp,reason:impl Into<String>)->Loaded{Loaded{content:String::new(),view:View::Unsupported{stamp,reason:reason.into()}}}
fn binary_extension(path:&Path)->bool{
    let ext=path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    matches!(ext.as_str(),"pdf"|"doc"|"docx"|"docm"|"xls"|"xlsx"|"xlsm"|"ppt"|"pptx"|"pptm"|"zip"|"7z"|"rar"|"gz"|"tar"|"exe"|"dll"|"msi"|"lnk"|"ttf"|"otf"|"woff"|"woff2"|"db"|"sqlite"|"sqlite3"|"mp3"|"mp4"|"mov"|"wav"|"flac"|"ogg"|"psd"|"glb"|"unitypackage"|"ico"|"tif"|"tiff"|"avif"|"heic"|"heif")
}
pub fn load(root:&Path,id:&str)->Result<Loaded,String>{
    let root=root.canonicalize().map_err(|e|e.to_string())?;
    let path=crate::core::safe(&root,id)?;let stamp=stamp(&path)?;
    // Check metadata before opening: directories/devices/FIFOs are never text inputs.
    let mut file=fs::File::open(&path).map_err(|e|e.to_string())?;
    let mut bytes=Vec::new();file.by_ref().take(512).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    let format=image::guess_format(&bytes).ok().filter(|f|matches!(f,image::ImageFormat::Png|image::ImageFormat::Jpeg|image::ImageFormat::Gif|image::ImageFormat::WebP|image::ImageFormat::Bmp));
    let limit=if format.is_some(){IMAGE_LIMIT}else{TEXT_LIMIT};
    if stamp.bytes>limit{return Ok(unsupported(stamp,if format.is_some(){"图片超过 20 MB，已跳过解码，避免阻塞界面。"}else{"文件超过 2 MB，不载入文本编辑器，避免卡顿。"}));}
    if format.is_none()&&(binary_extension(&path)||bytes.starts_with(b"%PDF-")||bytes.starts_with(b"PK\x03\x04")){return Ok(unsupported(stamp,"此类文档、媒体、压缩包或二进制文件暂不支持内嵌预览。不会将其作为文本打开，也不会自动执行。"));}
    file.take(limit+1-bytes.len() as u64).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len() as u64>limit{return Ok(unsupported(stamp,"读取期间文件变大，已停止载入。"));}
    if let Some(format)=format{
        let reader=image::ImageReader::with_format(Cursor::new(&bytes),format);
        let (width,height)=match reader.into_dimensions(){Ok(size)=>size,Err(_)=>return Ok(unsupported(stamp,"图片损坏或格式不受支持，无法安全预览。"))};
        if width==0||height==0||width>8192||height>8192||u64::from(width)*u64::from(height)>PIXEL_LIMIT{return Ok(unsupported(stamp,"图片尺寸过大（上限 800 万像素、单边 8192），已跳过解码。"));}
        let mut reader=image::ImageReader::with_format(Cursor::new(&bytes),format);
        let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(96_000_000);reader.limits(limits);
        let mut pixels=match reader.decode(){Ok(image)=>image.to_rgba8(),Err(_)=>return Ok(unsupported(stamp,"图片解码失败或超出资源限制，源文件未被修改。"))};
        // Decode only the first frame; GPUI's texture format is BGRA.
        for pixel in pixels.chunks_exact_mut(4){pixel.swap(0,2);}
        let data=Arc::new(gpui::RenderImage::new(vec![image::Frame::new(pixels)]));
        return Ok(Loaded{content:String::new(),view:View::Image{data,width,height,stamp}});
    }
    if bytes.starts_with(&[0xff,0xfe])||bytes.starts_with(&[0xfe,0xff]){return Ok(unsupported(stamp,"检测到 UTF-16 编码。当前只编辑 UTF-8 文本，不会擅自转码或覆盖文件。"));}
    let content=match String::from_utf8(bytes){Ok(text)=>text,Err(_)=>return Ok(unsupported(stamp,"文件不是有效的 UTF-8 文本。可能是其他编码或二进制文件，已禁止文本编辑。"))};
    if content.chars().any(|c|c.is_control()&&!matches!(c,'\n'|'\r'|'\t')){return Ok(unsupported(stamp,"检测到二进制控制字符，不作为文本打开，避免破坏源文件。"));}
    Ok(Loaded{content,view:View::Text})
}
#[derive(Debug)]
pub enum DropTarget{Folder(PathBuf),File{path:PathBuf,relative:Option<String>}}
pub fn resolve_drop(paths:&[PathBuf],current_root:Option<&str>)->Result<DropTarget,String>{
    if paths.len()!=1{return Err("请一次拖入一个文件或一个文件夹。".into());}
    let path=paths[0].canonicalize().map_err(|e|format!("无法打开拖入路径：{e}"))?;
    if path.to_str().is_none(){return Err("暂不支持非 UTF-8 文件路径。".into());}
    let meta=fs::metadata(&path).map_err(|e|e.to_string())?;
    if meta.is_dir(){return Ok(DropTarget::Folder(path));}
    if !meta.is_file(){return Err("仅支持普通文件或文件夹，不打开设备或管道。".into());}
    let relative=current_root.and_then(|root|path.strip_prefix(root).ok()).map(|p|p.to_string_lossy().replace('\\',"/"));
    Ok(DropTarget::File{path,relative})
}
pub fn size(bytes:u64)->String{if bytes>=1_000_000{format!("{:.1} MB",bytes as f64/1_000_000.)}else if bytes>=1000{format!("{:.1} KB",bytes as f64/1000.)}else{format!("{bytes} B")}}
#[cfg(target_os="windows")]
pub(crate) fn shell_display_path(path:&Path)->String{
    let display=path.to_string_lossy();
    if let Some(unc)=display.strip_prefix(r"\\?\UNC\"){format!(r"\\{unc}")}else{display.strip_prefix(r"\\?\").unwrap_or(&display).to_string()}
}

// A dedicated COM-initialized thread avoids relying on executor-thread COM state.
// cidl=0 makes the shell open the parent and select the supplied absolute item.
#[cfg(target_os="windows")]
pub fn reveal_windows(path:PathBuf)->Result<(),String>{
    std::thread::spawn(move||{
        use std::ffi::c_void;
        use std::os::windows::ffi::OsStrExt;
        #[link(name="ole32")] unsafe extern "system"{fn CoInitializeEx(reserved:*mut c_void,mode:u32)->i32;fn CoUninitialize();fn CoTaskMemFree(memory:*mut c_void);}
        #[link(name="shell32")] unsafe extern "system"{
            fn SHParseDisplayName(name:*const u16,context:*mut c_void,item:*mut *mut c_void,attributes:u32,result:*mut u32)->i32;
            fn SHOpenFolderAndSelectItems(item:*const c_void,count:u32,children:*const *const c_void,flags:u32)->i32;
        }
        let display=shell_display_path(&path);
        let wide=std::ffi::OsStr::new(&display).encode_wide().chain(Some(0)).collect::<Vec<_>>();
        // All pointers refer to owned buffers or shell-allocated PIDLs; both are released on this thread.
        unsafe{
            let initialized=CoInitializeEx(std::ptr::null_mut(),2);
            if initialized<0{return Err(format!("COM 0x{:08X}",initialized as u32));}
            let mut item=std::ptr::null_mut();
            let parsed=SHParseDisplayName(wide.as_ptr(),std::ptr::null_mut(),&mut item,0,std::ptr::null_mut());
            let result=if parsed<0{parsed}else if item.is_null(){-2147467259}else{SHOpenFolderAndSelectItems(item,0,std::ptr::null(),0)};
            if !item.is_null(){CoTaskMemFree(item);}CoUninitialize();
            if result<0{Err(format!("Shell 0x{:08X}",result as u32))}else{Ok(())}
        }
    }).join().map_err(|_|"Shell thread failed".to_string())?
}

#[cfg(test)]mod tests{
    use super::*;
    fn open(name:&str,bytes:&[u8])->Loaded{let dir=tempfile::tempdir().unwrap();fs::write(dir.path().join(name),bytes).unwrap();load(dir.path(),name).unwrap()}
    #[test]fn utf8_text_and_script_are_editable_without_encoding_conversion(){for name in ["a.md","a.json","a.py","a.svg","a.unknown"]{let file=open(name,"\u{feff}中文\r\nhello".as_bytes());assert!(file.view.is_text());assert_eq!(file.content,"\u{feff}中文\r\nhello");}}
    #[test]fn binary_and_non_utf8_are_never_text_even_with_md_extension(){for bytes in [vec![0,1,2],vec![0xff,0xfe,65,0],vec![0xff,0x88]]{assert!(!open("bad.md",&bytes).view.is_text());}assert!(!open("a.exe",b"hello").view.is_text());assert!(!open("a.pdf",b"%PDF-1.4").view.is_text());assert!(!open("renamed.md",b"%PDF-1.4").view.is_text());}
    #[test]fn images_use_bounded_first_frame_and_bgra(){let mut bytes=Cursor::new(Vec::new());image::RgbaImage::from_pixel(2,2,image::Rgba([250,10,20,255])).write_to(&mut bytes,image::ImageFormat::Png).unwrap();assert!(matches!(open("photo.png",bytes.get_ref()).view,View::Image{width:2,height:2,..}));}
    #[cfg(target_os="windows")]
    #[test]fn shell_paths_preserve_drive_and_unc_locations(){
        assert_eq!(shell_display_path(Path::new(r"\\?\C:\a b\中文.md")),r"C:\a b\中文.md");
        assert_eq!(shell_display_path(Path::new(r"\\?\UNC\server\share\a.md")),r"\\server\share\a.md");
        assert_eq!(shell_display_path(Path::new(r"C:\plain.md")),r"C:\plain.md");
    }
    #[test]fn large_text_is_info_only(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("large.txt");fs::File::create(&path).unwrap().set_len(TEXT_LIMIT+1).unwrap();assert!(!load(dir.path(),"large.txt").unwrap().view.is_text());}
    #[test]fn drop_routes_existing_files_and_folders_and_rejects_multiple(){let dir=tempfile::tempdir().unwrap();let root=dir.path().canonicalize().unwrap();let file=root.join("中文.md");fs::write(&file,"# A").unwrap();assert!(matches!(resolve_drop(&[file.clone()],root.to_str()).unwrap(),DropTarget::File{relative:Some(_),..}));assert!(matches!(resolve_drop(&[root.clone()],None).unwrap(),DropTarget::Folder(_)));assert!(resolve_drop(&[file,root],None).is_err());}
}
