//! Owned local PTY sessions. All process I/O stays off the UI thread.
use portable_pty::{CommandBuilder,PtySize,ChildKiller};
use std::{io::{Read,Write},path::{Path,PathBuf},sync::{Arc,Mutex,atomic::{AtomicBool,Ordering},mpsc},time::Duration};

pub struct DockDrag{pub origin:f32,pub height:f32,pub draft:f32}
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
fn shell_cwd(path:&Path)->PathBuf{
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

#[derive(Default)]pub struct Replies{pub bytes:Vec<u8>}
impl vt100::Callbacks for Replies{
    fn unhandled_csi(&mut self,screen:&mut vt100::Screen,first:Option<u8>,_:Option<u8>,params:&[&[u16]],c:char){
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
