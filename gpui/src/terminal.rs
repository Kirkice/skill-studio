//! Owned local PTY sessions. All process I/O stays off the UI thread.
use portable_pty::{CommandBuilder,PtySize,ChildKiller};
use std::{io::{Read,Write},path::{Path,PathBuf},sync::{Arc,Mutex,atomic::{AtomicBool,Ordering},mpsc},time::Duration};

pub struct DockDrag{pub problems:bool,pub origin:f32,pub height:f32,pub draft:f32}
pub fn dock_height(height:f32,available:f32)->f32{height.clamp(120.,(available-180.).max(120.))}

pub enum Event{Data(Vec<u8>),Exit(String),Error(String)}
enum Input{Bytes(Vec<u8>)}
pub struct Session{
    input:mpsc::SyncSender<Input>,size:Arc<Mutex<Option<PtySize>>>,alive:Arc<AtomicBool>,
    stopping:Arc<AtomicBool>,killer:Arc<Mutex<Box<dyn ChildKiller+Send+Sync>>>,
    done:async_channel::Receiver<()>,complete:Mutex<Option<async_channel::Sender<()>>>,
}
impl Session{
    pub fn spawn(cwd:&Path,rows:u16,cols:u16)->Result<(Self,async_channel::Receiver<Event>,String),String>{
        let (mut cmd,label)=shell()?;cmd.cwd(shell_cwd(cwd));cmd.env("TERM","xterm-256color");cmd.env("COLORTERM","truecolor");cmd.env("TERM_PROGRAM","skill-studio");
        let (session,events)=Self::with_command(cmd,rows,cols)?;Ok((session,events,label))
    }
    fn with_command(cmd:CommandBuilder,rows:u16,cols:u16)->Result<(Self,async_channel::Receiver<Event>),String>{
        #[cfg(windows)]{
            let dir=std::env::current_exe().map_err(|e|e.to_string())?.parent().ok_or("Executable directory is unavailable")?.to_path_buf();
            if !dir.join("conpty.dll").is_file()||!dir.join("OpenConsole.exe").is_file(){return Err("Modern ConPTY runtime missing beside executable. Run python tools/prepare_conpty.py (keep conpty.dll and OpenConsole.exe with the app).".into());}
        }
        let pair=portable_pty::native_pty_system().openpty(pty_size(rows,cols)).map_err(|e|e.to_string())?;
        let mut reader=pair.master.try_clone_reader().map_err(|e|e.to_string())?;
        let mut writer=pair.master.take_writer().map_err(|e|e.to_string())?;
        let mut child=pair.slave.spawn_command(cmd).map_err(|e|e.to_string())?;drop(pair.slave);
        let killer=Arc::new(Mutex::new(child.clone_killer()));
        let alive=Arc::new(AtomicBool::new(true));let stopping=Arc::new(AtomicBool::new(false));
        let size=Arc::new(Mutex::new(None));let (input,commands)=mpsc::sync_channel(64);
        let (tx,rx)=async_channel::bounded(128);let (complete,done)=async_channel::bounded(1);
        let session=Self{input,size:size.clone(),alive:alive.clone(),stopping:stopping.clone(),killer,done,complete:Mutex::new(Some(complete))};
        let output=tx.clone();
        std::thread::Builder::new().name("studio-pty-read".into()).spawn(move||{
            let mut buffer=[0;8192];loop{match reader.read(&mut buffer){Ok(0)=>break,Ok(n)=>{if output.send_blocking(Event::Data(buffer[..n].to_vec())).is_err(){break;}},Err(e)if e.kind()==std::io::ErrorKind::Interrupted=>continue,Err(_)=>break}}
        }).map_err(|e|e.to_string())?;
        let running=alive.clone();let ended=tx.clone();
        std::thread::Builder::new().name("studio-pty-wait".into()).spawn(move||{let status=child.wait().map(|s|format!("{s}")).unwrap_or_else(|e|e.to_string());running.store(false,Ordering::Release);let _=ended.send_blocking(Event::Exit(status));}).map_err(|e|e.to_string())?;
        std::thread::Builder::new().name("studio-pty-input".into()).spawn(move||{
            let master=pair.master;
            while alive.load(Ordering::Acquire)&&!stopping.load(Ordering::Acquire){
                let resize=size.lock().unwrap_or_else(|e|e.into_inner()).take();
                if let Some(next)=resize{if let Err(e)=master.resize(next){let _=tx.try_send(Event::Error(e.to_string()));}}
                match commands.recv_timeout(Duration::from_millis(16)){
                    Ok(Input::Bytes(bytes))=>if let Err(e)=writer.write_all(&bytes).and_then(|_|writer.flush()){let _=tx.try_send(Event::Error(e.to_string()));break;},
                    Err(mpsc::RecvTimeoutError::Timeout)=>{},Err(mpsc::RecvTimeoutError::Disconnected)=>break,
                }
            }
            drop(writer);drop(master);
        }).map_err(|e|e.to_string())?;
        Ok((session,rx))
    }
    pub fn running(&self)->bool{self.alive.load(Ordering::Acquire)&&!self.stopping.load(Ordering::Acquire)}
    pub fn write(&self,bytes:&[u8])->Result<(),String>{if !self.running(){return Err("Terminal process has ended".into());}self.input.try_send(Input::Bytes(bytes.to_vec())).map_err(|_|"Terminal input is busy; retry shortly".into())}
    pub fn resize(&self,rows:u16,cols:u16){*self.size.lock().unwrap_or_else(|e|e.into_inner())=Some(pty_size(rows,cols));}
    /// Stops only the process owned by this session. Completion can be awaited before closing the app.
    pub fn stop(&self)->async_channel::Receiver<()>{
        if !self.stopping.swap(true,Ordering::AcqRel){
            if let Some(done)=self.complete.lock().unwrap_or_else(|e|e.into_inner()).take(){
                let killer=self.killer.clone();let alive=self.alive.clone();
                std::thread::spawn(move||{if alive.swap(false,Ordering::AcqRel){let _=killer.lock().unwrap_or_else(|e|e.into_inner()).kill();}let _=done.send_blocking(());});
            }
        }self.done.clone()
    }
}
impl Drop for Session{fn drop(&mut self){self.stop();}}
fn pty_size(rows:u16,cols:u16)->PtySize{PtySize{rows:rows.clamp(1,200),cols:cols.clamp(2,500),pixel_width:0,pixel_height:0}}
pub(crate) fn shell_cwd(path:&Path)->PathBuf{
    #[cfg(windows)]{PathBuf::from(crate::files::shell_display_path(path))}
    #[cfg(not(windows))]{path.to_path_buf()}
}
fn shell()->Result<(CommandBuilder,String),String>{
    #[cfg(windows)]{
        let pwsh=std::env::var_os("PATH").and_then(|paths|std::env::split_paths(&paths).map(|p|p.join("pwsh.exe")).find(|p|p.is_file()))
            .or_else(||std::env::var_os("ProgramFiles").map(PathBuf::from).map(|p|p.join("PowerShell/7/pwsh.exe")).filter(|p|p.is_file()));
        let path=pwsh.unwrap_or_else(||PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(||"C:\\Windows".into())).join("System32/WindowsPowerShell/v1.0/powershell.exe"));
        if !path.is_file(){return Err("PowerShell was not found".into());}
        let label=path.file_stem().unwrap_or_default().to_string_lossy().into_owned();let mut cmd=CommandBuilder::new(path);cmd.arg("-NoLogo");Ok((cmd,label))
    }
    #[cfg(not(windows))]{
        let path=std::env::var_os("SHELL").map(PathBuf::from).filter(|p|p.is_file()).unwrap_or_else(||if Path::new("/bin/zsh").is_file(){"/bin/zsh".into()}else{"/bin/sh".into()});
        let label=path.file_name().unwrap_or_default().to_string_lossy().into_owned();let mut cmd=CommandBuilder::new(path);cmd.arg("-l");Ok((cmd,label))
    }
}

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub enum CursorShape{#[default]Block,Underline,Bar}
pub struct Replies{pub cursor:CursorShape,pub bytes:Vec<u8>,pub foreground:u32,pub background:u32,pub ansi:[u32;16]}
impl Default for Replies{fn default()->Self{Self{cursor:CursorShape::Block,bytes:vec![],foreground:0xd2d4d9,background:0x17191d,ansi:[0;16]}}}
pub fn indexed_color(index:u8,ansi:&[u32;16])->u32{
    if index<16{ansi[index as usize]}else if index>=232{let v=8+(u32::from(index)-232)*10;(v<<16)|(v<<8)|v}else{let n=u32::from(index)-16;let a=[0,95,135,175,215,255];(a[(n/36)as usize]<<16)|(a[((n/6)%6)as usize]<<8)|a[(n%6)as usize]}
}
#[derive(Default)]pub struct ScrollAccumulator{x:f32,y:f32}
impl ScrollAccumulator{
    pub fn reset(&mut self){self.x=0.;self.y=0.;}
    pub fn steps(&mut self,x:f32,y:f32,limit:f32)->(i32,i32){
        fn step(remainder:&mut f32,delta:f32,limit:f32)->i32{if !delta.is_finite(){return 0;}*remainder=(*remainder+delta).clamp(-limit.max(1.),limit.max(1.));let n=remainder.trunc() as i32;*remainder-=n as f32;n}
        (step(&mut self.x,x,limit),step(&mut self.y,y,limit))
    }
}
pub fn wheel_report(encoding:vt100::MouseProtocolEncoding,button:u8,x:u16,y:u16,modifiers:u8)->Vec<u8>{
    let code=button|modifiers;
    match encoding{
        vt100::MouseProtocolEncoding::Sgr=>format!("\x1b[<{code};{x};{y}M").into_bytes(),
        vt100::MouseProtocolEncoding::Default=>if x<=223&&y<=223{vec![27,91,77,code+32,(x+32) as u8,(y+32) as u8]}else{vec![]},
        vt100::MouseProtocolEncoding::Utf8=>{let mut s=String::from("\x1b[M");for n in [u32::from(code)+32,u32::from(x)+32,u32::from(y)+32]{if let Some(c)=char::from_u32(n){s.push(c);}}s.into_bytes()},
    }
}
#[derive(Debug,PartialEq,Eq)]pub enum WheelAction{History(usize),Input(Vec<u8>)}
pub fn route_wheel(screen:&vt100::Screen,steps:(i32,i32),position:(u16,u16),modifiers:u8,force_history:bool)->WheelAction{
    let(dx,dy)=(steps.0.clamp(-500,500),steps.1.clamp(-200,200));
    if screen.mouse_protocol_mode()!=vt100::MouseProtocolMode::None&&!force_history{
        let mut bytes=vec![];for(steps,positive,negative)in[(dy,64,65),(dx,66,67)]{for _ in 0..steps.unsigned_abs(){bytes.extend(wheel_report(screen.mouse_protocol_encoding(),if steps>0{positive}else{negative},position.0,position.1,modifiers));}}WheelAction::Input(bytes)
    }else if screen.alternate_screen()&&!force_history{
        let up=if screen.application_cursor(){b"\x1bOA"}else{b"\x1b[A"};let down=if screen.application_cursor(){b"\x1bOB"}else{b"\x1b[B"};let mut bytes=vec![];for _ in 0..dy.unsigned_abs(){bytes.extend_from_slice(if dy>0{up}else{down});}WheelAction::Input(bytes)
    }else{WheelAction::History((screen.scrollback() as i64+i64::from(dy)).max(0) as usize)}
}

impl vt100::Callbacks for Replies{
    fn unhandled_osc(&mut self,_:&mut vt100::Screen,params:&[&[u8]]){
        fn reply(out:&mut Vec<u8>,key:&str,rgb:u32){if out.len()>8192{return;}out.extend_from_slice(format!("\x1b]{key};rgb:{:04x}/{:04x}/{:04x}\x1b\\",((rgb>>16)&255)*257,((rgb>>8)&255)*257,(rgb&255)*257).as_bytes());}
        let Some(command)=params.first().and_then(|p|std::str::from_utf8(p).ok()).and_then(|s|s.parse::<u16>().ok())else{return;};
        if command==4{for pair in params[1..].chunks_exact(2){if pair[1]!=b"?"{continue;}if let Some(index)=std::str::from_utf8(pair[0]).ok().and_then(|s|s.parse::<u8>().ok()){reply(&mut self.bytes,&format!("4;{index}"),indexed_color(index,&self.ansi));}}}
        else if (10..=12).contains(&command){for (i,q) in params[1..].iter().enumerate(){let key=command+i as u16;if *q==b"?"&&key<=12{let rgb=if key==11{self.background}else{self.foreground};reply(&mut self.bytes,&key.to_string(),rgb);}}}
    }
    fn unhandled_csi(&mut self,screen:&mut vt100::Screen,first:Option<u8>,_:Option<u8>,params:&[&[u16]],c:char){
        if c=='q'&&first==Some(b' '){let style=params.first().and_then(|p|p.first()).copied().unwrap_or(0);self.cursor=match style{3|4=>CursorShape::Underline,5|6=>CursorShape::Bar,_=>CursorShape::Block};return;}
        if self.bytes.len()>8192{return;}
        let p=params.first().and_then(|p|p.first()).copied().unwrap_or(0);
        let reply=match (c,p,first){('n',6,_)=>{let(r,c)=screen.cursor_position();format!("\x1b[{};{}R",r+1,c+1)},('n',5,_)=>"\x1b[0n".into(),('c',0,Some(b'>'))=>"\x1b[>0;1;0c".into(),('c',0,_)=>"\x1b[?1;2c".into(),_=>return};self.bytes.extend_from_slice(reply.as_bytes());
    }
}
pub fn parser(rows:u16,cols:u16)->vt100::Parser<Replies>{vt100::Parser::new_with_callbacks(rows,cols,2000,Replies::default())}
pub fn paste_bytes(text:&str,bracketed:bool)->Vec<u8>{let text=text.replace("\r\n","\n");if bracketed{format!("\x1b[200~{text}\x1b[201~").into_bytes()}else{text.replace('\n',"\r").into_bytes()}}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn vt_preserves_colors_unicode_and_alternate_screen(){let mut p=parser(10,40);p.process("\x1b[31m中文\x1b[0m".as_bytes());assert!(p.screen().contents().contains("中文"));assert_eq!(p.screen().cell(0,0).unwrap().fgcolor(),vt100::Color::Idx(1));p.process(b"\x1b[?1049h");assert!(p.screen().alternate_screen());}
    #[test]fn query_responses_are_split_safe(){let mut p=parser(10,40);p.process(b"\x1b[2;3H\x1b[");p.process(b"6n");assert_eq!(p.callbacks().bytes,b"\x1b[2;3R");}
    #[test]fn precise_scroll_accumulates_instead_of_dropping_small_deltas(){let mut s=ScrollAccumulator::default();for _ in 0..3{assert_eq!(s.steps(0.,0.25,20.),(0,0));}assert_eq!(s.steps(0.,0.25,20.),(0,1));assert_eq!(s.steps(0.,-1.,20.),(0,-1));}
    #[test]fn full_screen_mouse_wheel_reaches_application(){let mut p=parser(10,40);p.process(b"\x1b[?1049h\x1b[?1000h\x1b[?1006h");assert_eq!(route_wheel(p.screen(),(0,1),(4,3),0,false),WheelAction::Input(b"\x1b[<64;4;3M".to_vec()));assert!(matches!(route_wheel(p.screen(),(0,1),(4,3),4,true),WheelAction::History(_)));}
    #[test]fn alternate_screen_without_mouse_uses_cursor_keys(){let mut p=parser(10,40);p.process(b"\x1b[?1049h\x1b[?1h");assert_eq!(route_wheel(p.screen(),(0,-1),(1,1),0,false),WheelAction::Input(b"\x1bOB".to_vec()));}
    #[test]fn normal_screen_scrolls_real_history(){let mut p=parser(3,20);p.process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive");if let WheelAction::History(n)=route_wheel(p.screen(),(0,2),(1,1),0,false){p.screen_mut().set_scrollback(n);}else{panic!("history expected");}assert_eq!(p.screen().scrollback(),2);assert!(p.screen().contents().contains("one"));}
    #[test]fn terminal_cursor_style_follows_decscusr(){let mut p=parser(3,20);p.process(b"\x1b[6 q");assert_eq!(p.callbacks().cursor,CursorShape::Bar);p.process(b"\x1b[4 q");assert_eq!(p.callbacks().cursor,CursorShape::Underline);p.process(b"\x1b[2 q");assert_eq!(p.callbacks().cursor,CursorShape::Block);}
    #[test]fn color_queries_return_exact_terminal_colors(){let mut p=parser(3,20);p.callbacks_mut().ansi[1]=0xdc8c95;p.process(b"\x1b]11;?\x07\x1b]10;?\x07\x1b]4;1;?\x07");let text=String::from_utf8(p.callbacks().bytes.clone()).unwrap();assert!(text.contains("11;rgb:1717/1919/1d1d"));assert!(text.contains("10;rgb:d2d2/d4d4/d9d9"));assert!(text.contains("4;1;rgb:dcdc/8c8c/9595"));}
    #[test]fn legacy_and_utf8_wheel_encodings_preserve_coordinates(){assert_eq!(wheel_report(vt100::MouseProtocolEncoding::Default,65,1,2,0),vec![27,91,77,97,33,34]);assert!(wheel_report(vt100::MouseProtocolEncoding::Default,64,300,1,0).is_empty());assert!(String::from_utf8(wheel_report(vt100::MouseProtocolEncoding::Utf8,64,300,1,0)).is_ok());}
    #[cfg(windows)]#[test]fn conpty_preserves_mouse_mode_and_delivers_real_wheel_input(){
        let script=r#"Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class RawWheel{[DllImport("kernel32.dll")]public static extern IntPtr GetStdHandle(int n);[DllImport("kernel32.dll")]public static extern bool SetConsoleMode(IntPtr h,uint mode);}';if(![RawWheel]::SetConsoleMode([RawWheel]::GetStdHandle(-10),512)){throw "raw mode failed"};$e=[char]27;[Console]::Write("${e}[?1000h${e}[?1006hREADY");$b=New-Object byte[] 128;$input=[Console]::OpenStandardInput();$n=$input.Read($b,0,128);[Console]::Write("CAPTURE:"+[BitConverter]::ToString($b,0,$n))"#;
        let mut cmd=CommandBuilder::new("powershell.exe");cmd.arg("-NoProfile");cmd.arg("-Command");cmd.arg(script);
        let(s,rx)=Session::with_command(cmd,10,40).unwrap();let mut output=vec![];let mut vt=parser(10,40);let deadline=std::time::Instant::now()+Duration::from_secs(8);let mut sent=false;
        while std::time::Instant::now()<deadline{
            while let Ok(event)=rx.try_recv(){if let Event::Data(data)=event{vt.process(&data);let reply=std::mem::take(&mut vt.callbacks_mut().bytes);if !reply.is_empty(){s.write(&reply).unwrap();}output.extend(data);}}
            if !sent&&String::from_utf8_lossy(&output).contains("READY"){assert_ne!(vt.screen().mouse_protocol_mode(),vt100::MouseProtocolMode::None,"ConPTY swallowed DECSET mouse mode");let WheelAction::Input(bytes)=route_wheel(vt.screen(),(0,1),(4,3),0,false)else{panic!("application must receive wheel")};s.write(&bytes).unwrap();sent=true;}
            if String::from_utf8_lossy(&output).contains("CAPTURE:1B-5B-3C-36-34-3B-34-3B-33-4D"){s.stop();return;}
            std::thread::sleep(Duration::from_millis(20));
        }s.stop();panic!("Wheel did not reach child stdin: {}",String::from_utf8_lossy(&output));
    }
    #[test]fn dock_height_keeps_room_for_document(){assert_eq!(dock_height(800.,830.),650.);assert_eq!(dock_height(20.,830.),120.);}
    #[test]fn paste_respects_bracketed_mode(){assert_eq!(paste_bytes("a\r\nb",false),b"a\rb");assert_eq!(paste_bytes("a\nb",true),b"\x1b[200~a\nb\x1b[201~");}
    #[test]fn pty_runs_private_command_and_exits(){
        let dir=tempfile::tempdir().unwrap();
        #[cfg(windows)]let mut cmd=CommandBuilder::new(PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/WindowsPowerShell/v1.0/powershell.exe"));
        #[cfg(windows)]cmd.args(["-NoLogo","-NoProfile","-Command","Write-Output STUDIO_PTY_OK"]);
        #[cfg(not(windows))]let mut cmd=CommandBuilder::new("/bin/sh");
        #[cfg(not(windows))]cmd.args(["-c","printf STUDIO_PTY_OK"]);
        cmd.cwd(dir.path());let(session,rx)=Session::with_command(cmd,10,80).unwrap();let start=std::time::Instant::now();let mut bytes=vec![];let mut parser=parser(10,80);
        while start.elapsed()<Duration::from_secs(10){match rx.try_recv(){Ok(Event::Data(b))=>{parser.process(&b);let replies=std::mem::take(&mut parser.callbacks_mut().bytes);if !replies.is_empty(){let _=session.write(&replies);}bytes.extend(b);},Ok(_)=>{},Err(async_channel::TryRecvError::Closed)=>break,Err(_)=>std::thread::sleep(Duration::from_millis(10))}}
        session.stop();assert!(String::from_utf8_lossy(&bytes).contains("STUDIO_PTY_OK"),"PTY output: {:?}",String::from_utf8_lossy(&bytes));
    }
}
