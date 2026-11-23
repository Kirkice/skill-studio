//! Native terminal surface. Parsing/output invalidate only this entity, not Studio.
use crate::terminal::{self,Session,Event};
use super::{controls::*,icons::{icon,Icon},theme::*,theme::rgb};
use gpui::*;
mod metrics;
mod render;
mod menu;
use metrics::Metrics;
use std::{cell::RefCell,rc::Rc,sync::Arc,path::PathBuf,ops::Range,time::Duration};

actions!(terminal_keys,[CopyTerminal,PasteTerminal,ControlC,ControlD,ControlS,ControlW,ControlF,EscapeTerminal,CloseTerminal]);
pub enum TerminalEvent{Hide,Maximize(bool),FontSize(u16),LineHeight(f32)}
pub fn bind_keys(cx:&mut App){
    let mut keys=vec![KeyBinding::new("ctrl-shift-c",CopyTerminal,Some("Terminal")),KeyBinding::new("ctrl-shift-v",PasteTerminal,Some("Terminal")),KeyBinding::new("ctrl-c",ControlC,Some("Terminal")),KeyBinding::new("ctrl-d",ControlD,Some("Terminal")),KeyBinding::new("ctrl-s",ControlS,Some("Terminal")),KeyBinding::new("ctrl-w",ControlW,Some("Terminal")),KeyBinding::new("ctrl-f",ControlF,Some("Terminal")),KeyBinding::new("escape",EscapeTerminal,Some("Terminal"))];
    if cfg!(target_os="macos"){keys.extend([KeyBinding::new("cmd-c",CopyTerminal,Some("Terminal")),KeyBinding::new("cmd-v",PasteTerminal,Some("Terminal")),KeyBinding::new("cmd-shift-v",PasteTerminal,Some("Terminal")),KeyBinding::new("cmd-w",CloseTerminal,Some("Terminal"))]);}
    else{keys.push(KeyBinding::new("ctrl-v",PasteTerminal,Some("Terminal")));}cx.bind_keys(keys);
}
struct Run{col:u16,line:ShapedLine,width:Pixels,pad:Pixels}
struct Row{cells:Vec<vt100::Cell>,runs:Vec<Run>,selection:Option<(u16,u16)>}
struct Grid{
    parser:vt100::Parser<terminal::Replies>,session:Option<Arc<Session>>,bounds:Bounds<Pixels>,cell:Size<Pixels>,metrics:Metrics,line_height:f32,
    rows:Vec<Row>,palette:TerminalPalette,wheel:terminal::ScrollAccumulator,history:usize,force_app_scroll:bool,scrollbar_grab:Option<(f32,usize)>,font_size:f32,marked:String,selection:Option<((u16,u16),(u16,u16))>,selecting:bool,
}
impl Grid{
    fn new()->Self{Self{parser:terminal::parser(12,80),session:None,bounds:Bounds::default(),cell:size(px(8.),px(20.)),metrics:Metrics::new(14.,1.,1.),line_height:1.,rows:vec![],palette:terminal_palette(),wheel:Default::default(),history:0,force_app_scroll:false,scrollbar_grab:None,font_size:14.,marked:String::new(),selection:None,selecting:false}}
    fn sync_theme(&mut self){let p=terminal_palette();if self.palette!=p{self.rows.clear();self.palette=p;}let replies=self.parser.callbacks_mut();replies.foreground=p.foreground;replies.background=p.background;replies.ansi=p.ansi;}
    fn history(&mut self){let screen=self.parser.screen_mut();if screen.alternate_screen(){self.history=0;}else{let offset=screen.scrollback();screen.set_scrollback(usize::MAX);self.history=screen.scrollback();screen.set_scrollback(offset);}}
    fn thumb(&self)->Option<(f32,f32,f32)>{if self.history==0||self.parser.screen().alternate_screen(){return None;}let track=(f32::from(self.bounds.size.height)-16.).max(1.);let rows=self.parser.screen().size().0 as f32;let height=(track*rows/(self.history as f32+rows)).max(18.).min(track);let travel=track-height;let top=8.+travel*(1.-self.parser.screen().scrollback() as f32/self.history as f32);Some((top,height,travel))}
    fn at(&self,p:Point<Pixels>)->(u16,u16){let(rows,cols)=self.parser.screen().size();let row=((f32::from(p.y-self.bounds.origin.y)-8.)/f32::from(self.cell.height)).floor().clamp(0.,rows.saturating_sub(1) as f32) as u16;let mut col=((f32::from(p.x-self.bounds.origin.x)-8.)/f32::from(self.cell.width)).floor().clamp(0.,cols.saturating_sub(1) as f32) as u16;if col>0&&self.parser.screen().cell(row,col).is_some_and(|c|c.is_wide_continuation()){col-=1;}(row,col)}
    fn selection(&self)->Option<((u16,u16),(u16,u16))>{let(a,b)=self.selection?;if a==b{return None;}Some(if a<b{(a,b)}else{(b,a)})}
    fn copy(&self)->String{if let Some((a,b))=self.selection(){self.parser.screen().contents_between(a.0,a.1,b.0,b.1.saturating_add(if self.parser.screen().cell(b.0,b.1).is_some_and(|c|c.is_wide()){2}else{1}))}else{self.parser.screen().contents()}}
}
pub struct TerminalView{
    pub focus:FocusHandle,grid:Rc<RefCell<Grid>>,session:Option<Arc<Session>>,reader:Option<Task<()>>,
    pub cwd:Option<PathBuf>,next_cwd:Option<PathBuf>,label:String,status:String,starting:bool,generation:u64,confirming:bool,menu:bool,maximized:bool,notice:bool,
}
impl EventEmitter<TerminalEvent> for TerminalView{}
impl TerminalView{
    pub fn new(font_size:u16,line_height:f32,cx:&mut Context<Self>)->Self{let mut grid=Grid::new();grid.font_size=font_size as f32;grid.line_height=line_height;Self{focus:cx.focus_handle(),grid:Rc::new(RefCell::new(grid)),session:None,reader:None,cwd:None,next_cwd:dirs::home_dir(),label:"Terminal".into(),status:"Click start to open a local shell".into(),starting:false,generation:0,confirming:false,menu:false,maximized:false,notice:false}}
    pub fn target_directory(&mut self,cwd:PathBuf){self.next_cwd=Some(cwd);}
    pub fn restore_document(&mut self,cx:&mut Context<Self>){self.maximized=false;self.menu=false;cx.notify();}
    pub fn dismiss_menu(&mut self,cx:&mut Context<Self>){self.menu=false;cx.notify();}
    fn zoom(&mut self,delta:i16,cx:&mut Context<Self>){let mut g=self.grid.borrow_mut();let size=(g.font_size as i16+delta).clamp(10,26) as u16;g.font_size=size as f32;g.rows.clear();drop(g);cx.emit(TerminalEvent::FontSize(size));cx.notify();}
    fn return_live(&mut self,window:&mut Window,cx:&mut Context<Self>){self.menu=false;let mut g=self.grid.borrow_mut();g.parser.screen_mut().set_scrollback(0);g.selection=None;g.wheel.reset();drop(g);self.focus.focus(window,cx);cx.notify();}
    pub fn active(&self)->bool{self.starting||self.session.as_ref().is_some_and(|s|s.running())}
    pub fn composing(&self)->bool{!self.grid.borrow().marked.is_empty()}
    pub fn invalidate(&mut self,font_size:u16,line_height:f32,cx:&mut Context<Self>){let mut g=self.grid.borrow_mut();g.font_size=font_size as f32;g.line_height=line_height;g.rows.clear();g.sync_theme();drop(g);cx.notify();}
    pub fn start(&mut self,cwd:PathBuf,window:&mut Window,cx:&mut Context<Self>){
        self.next_cwd=Some(cwd.clone());self.focus.focus(window,cx);if self.active(){return;}
        self.generation+=1;let generation=self.generation;self.starting=true;self.menu=false;self.notice=false;self.cwd=Some(cwd.clone());self.status="Starting local shell…".into();
        let (rows,cols)=self.grid.borrow().parser.screen().size();cx.notify();
        cx.spawn_in(window,async move|this,cx|{
            let result=cx.background_executor().spawn(async move{Session::spawn(&cwd,rows,cols)}).await;
            let _=this.update_in(cx,|this,_,cx|{
                if this.generation!=generation{return;}
                this.starting=false;this.notice=false;
                match result{
                    Ok((session,events,label))=>{
                        let session=Arc::new(session);this.session=Some(session.clone());this.label=label;this.status=if cfg!(windows){"Local PTY · Ctrl+C depends on the running program"}else{"Local PTY · live shell session"}.into();
                        let mut g=this.grid.borrow_mut();g.parser=terminal::parser(rows,cols);g.rows.clear();g.marked.clear();g.selection=None;g.session=Some(session);g.sync_theme();g.history();drop(g);
                        this.reader=Some(cx.spawn(async move|this,cx|{
                            while let Ok(first)=events.recv().await{
                                let mut batch=vec![first];let mut budget=8192;
                                while budget<65536{let Ok(event)=events.try_recv()else{break;};budget+=match &event{Event::Data(b)=>b.len(),_=>0};batch.push(event);if batch.len()>=32{break;}}
                                let active=this.update(cx,|this,cx|{
                                    if this.generation!=generation{return false;}
                                    let mut g=this.grid.borrow_mut();
                                    for event in batch{match event{Event::Data(bytes)=>g.parser.process(&bytes),Event::Exit(status)=>{this.status=format!("Process ended · {status}");this.notice=true;},Event::Error(e)=>{this.status=e;this.notice=true;}}}
                                    g.history();
                                    let replies=std::mem::take(&mut g.parser.callbacks_mut().bytes);if !replies.is_empty(){if let Some(s)=&g.session{let _=s.write(&replies);}}
                                    drop(g);cx.notify();true
                                });
                                if !matches!(active,Ok(true)){break;}
                                cx.background_executor().timer(Duration::from_millis(16)).await;
                            }
                        }));
                    }
                    Err(e)=>{this.status=format!("Unable to start shell: {e}");this.notice=true;},
                }cx.notify();
            });
        }).detach();
    }
    pub fn stop(&mut self,cx:&mut Context<Self>)->Option<async_channel::Receiver<()>>{
        self.generation+=1;self.starting=false;self.reader=None;self.menu=false;self.notice=true;
        let done=self.session.take().map(|s|s.stop());let mut g=self.grid.borrow_mut();g.session=None;g.marked.clear();g.selecting=false;drop(g);
        self.status="Terminal stopped · restart to open a new shell".into();cx.notify();done
    }
    fn ask_stop(&mut self,restart:bool,window:&mut Window,cx:&mut Context<Self>){
        if self.confirming{return;}
        if !self.active(){if restart{if let Some(cwd)=self.next_cwd.clone().or_else(||self.cwd.clone()){self.start(cwd,window,cx);}}return;}
        self.confirming=true;let generation=self.generation;
        let answer=window.prompt(PromptLevel::Warning,if restart{"Restart terminal?"}else{"Stop terminal?"},Some("The shell and commands in this terminal will be terminated. Hiding the panel keeps them running."),&[PromptButton::ok(if restart{"Stop and restart"}else{"Stop terminal"}),PromptButton::cancel("Cancel")],cx);
        cx.spawn_in(window,async move|this,cx|{let answer=answer.await.ok();let _=this.update_in(cx,|this,w,cx|{this.confirming=false;if answer==Some(0)&&this.generation==generation{let done=this.stop(cx);let cwd=this.next_cwd.clone().or_else(||this.cwd.clone());if restart{cx.spawn_in(w,async move|this,cx|{if let Some(done)=done{let _=done.recv().await;}let _=this.update_in(cx,|this,w,cx|{if let Some(cwd)=cwd{this.start(cwd,w,cx);}});}).detach();}}});}).detach();
    }
    fn send(&mut self,bytes:&[u8],cx:&mut Context<Self>){
        self.notice=false;
        if self.composing(){self.grid.borrow_mut().marked.clear();}
        let mut g=self.grid.borrow_mut();g.parser.screen_mut().set_scrollback(0);g.selection=None;g.wheel.reset();
        if let Some(s)=&self.session{if let Err(e)=s.write(bytes){self.status=e;self.notice=true;}}else{self.status="Terminal is not ready; wait for the prompt or restart".into();self.notice=true;}drop(g);cx.notify();
    }
    fn copy(&mut self,window:&mut Window,cx:&mut Context<Self>){self.menu=false;cx.notify();let text=self.grid.borrow().copy();if !text.is_empty(){cx.write_to_clipboard(ClipboardItem::new_string(text));}self.focus.focus(window,cx);}
    fn paste_text(&mut self,text:String,window:&mut Window,cx:&mut Context<Self>){
        if text.len()>1_000_000{self.status="Paste exceeds 1 MB".into();self.notice=true;cx.notify();return;}
        if text.contains(['\n','\r']){
            let generation=self.generation;
            let answer=window.prompt(PromptLevel::Warning,"Paste multiple lines?",Some("Pasting line breaks into a shell can execute commands. Review the clipboard before continuing."),&[PromptButton::ok("Paste"),PromptButton::cancel("Cancel")],cx);
            cx.spawn_in(window,async move|this,cx|{if answer.await.ok()!=Some(0){return;}let _=this.update_in(cx,|this,w,cx|{if this.generation==generation{let bytes=terminal::paste_bytes(&text,this.grid.borrow().parser.screen().bracketed_paste());this.send(&bytes,cx);this.focus.focus(w,cx);}});}).detach();
        }else{let bytes=terminal::paste_bytes(&text,self.grid.borrow().parser.screen().bracketed_paste());self.send(&bytes,cx);self.focus.focus(window,cx);}
    }
    fn paste(&mut self,window:&mut Window,cx:&mut Context<Self>){self.menu=false;cx.notify();if self.starting||!self.active(){return;}if let Some(text)=cx.read_from_clipboard().and_then(|i|i.text()){self.paste_text(text,window,cx);}}
    fn wheel(&mut self,event:&ScrollWheelEvent,window:&mut Window,cx:&mut Context<Self>){
        window.prevent_default();cx.stop_propagation();
        let mut g=self.grid.borrow_mut();if event.touch_phase==TouchPhase::Started{g.wheel.reset();}
        let(x,y)=match event.delta{ScrollDelta::Lines(p)=>{let y=if cfg!(target_os="macos")&&p.y!=0.{p.y.signum()*p.y.abs().max(1.)*3.}else{p.y};(p.x,y)},ScrollDelta::Pixels(p)=>(f32::from(p.x)/f32::from(g.cell.width),f32::from(p.y)/f32::from(g.cell.height))};
        let limit=g.parser.screen().size().0 as f32;let(dx,dy)=g.wheel.steps(x,y,limit);if dx==0&&dy==0{return;}
        let(row,col)=g.at(event.position);let mods=u8::from(event.modifiers.shift)*4+u8::from(event.modifiers.alt)*8+u8::from(event.modifiers.control)*16;
        let bytes=if g.force_app_scroll&&!event.modifiers.shift{
            let mut bytes=vec![];for(steps,positive,negative)in[(dy,64,65),(dx,66,67)]{for _ in 0..steps.unsigned_abs(){bytes.extend(terminal::wheel_report(vt100::MouseProtocolEncoding::Sgr,if steps>0{positive}else{negative},col+1,row+1,mods));}}bytes
        }else{match terminal::route_wheel(g.parser.screen(),(dx,dy),(col+1,row+1),mods,event.modifiers.shift){
            terminal::WheelAction::History(offset)=>{g.parser.screen_mut().set_scrollback(offset);vec![]},terminal::WheelAction::Input(bytes)=>bytes,
        }};
        g.selection=None;if !bytes.is_empty(){if let Some(s)=&g.session{if let Err(e)=s.write(&bytes){self.status=e;}}}
        drop(g);cx.notify();
    }
    fn key_down(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>){
        if self.menu{window.prevent_default();cx.stop_propagation();return;}
        if self.composing(){return;}
        let application=self.grid.borrow().parser.screen().application_cursor();
        if let Some(bytes)=key_bytes(&event.keystroke,application){
            self.send(&bytes,cx);window.prevent_default();cx.stop_propagation();
        }
    }
}
impl Drop for TerminalView{fn drop(&mut self){if let Some(session)=&self.session{session.stop();}}}

fn key_bytes(key:&Keystroke,application:bool)->Option<Vec<u8>>{
    let m=&key.modifiers;
    if m.platform{return None;}
    if m.control&&!m.alt{
        if key.key=="space"{return Some(vec![0]);}
        if key.key.len()==1{let c=key.key.as_bytes()[0];if c.is_ascii(){return Some(vec![c.to_ascii_uppercase()&31]);}}
    }
    let modifier=1+u8::from(m.shift)+2*u8::from(m.alt)+4*u8::from(m.control);
    let arrow=match key.key.as_str(){"up"=>Some('A'),"down"=>Some('B'),"right"=>Some('C'),"left"=>Some('D'),"home"=>Some('H'),"end"=>Some('F'),_=>None};
    if let Some(c)=arrow{return Some(if modifier>1{format!("\x1b[1;{modifier}{c}")}else if application{format!("\x1bO{c}")}else{format!("\x1b[{c}")}.into_bytes());}
    let s=match key.key.as_str(){"enter"=>"\r".into(),"backspace"=>if m.control{"\x17".into()}else{"\x7f".into()},"tab"=>if m.shift{"\x1b[Z".into()}else{"\t".into()},"escape"=>"\x1b".into(),"delete"=>"\x1b[3~".into(),"insert"=>"\x1b[2~".into(),"pageup"=>"\x1b[5~".into(),"pagedown"=>"\x1b[6~".into(),"f1"=>"\x1bOP".into(),"f2"=>"\x1bOQ".into(),"f3"=>"\x1bOR".into(),"f4"=>"\x1bOS".into(),"f5"=>"\x1b[15~".into(),"f6"=>"\x1b[17~".into(),"f7"=>"\x1b[18~".into(),"f8"=>"\x1b[19~".into(),"f9"=>"\x1b[20~".into(),"f10"=>"\x1b[21~".into(),"f11"=>"\x1b[23~".into(),"f12"=>"\x1b[24~".into(),_=>{if m.alt&&!m.control&&!cfg!(target_os="macos"){format!("\x1b{}",key.key_char.as_deref().unwrap_or(&key.key))}else{return None;}}};Some(s.into_bytes())
}
fn color(c:vt100::Color,default:u32,palette:&TerminalPalette)->Rgba{
    gpui::rgb(match c{vt100::Color::Default=>default,vt100::Color::Rgb(r,g,b)=>(u32::from(r)<<16)|(u32::from(g)<<8)|u32::from(b),vt100::Color::Idx(n)=>terminal::indexed_color(n,&palette.ansi)})
}
impl Render for TerminalView{
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
        self.grid.borrow_mut().sync_theme();
        let focused=self.focus.is_focused(window);let grid=self.grid.clone();let paint=self.grid.clone();let focus=self.focus.clone();let input=cx.entity();let owner=cx.entity().downgrade();
        let fs=self.grid.borrow().font_size;let metrics=Metrics::new(fs,self.grid.borrow().line_height,window.scale_factor());let cell=metrics.size;
        let canvas=canvas(move|bounds,_,_|{let mut g=grid.borrow_mut();g.bounds=bounds;if g.cell!=cell||g.metrics!=metrics{g.rows.clear();g.cell=cell;g.metrics=metrics;}
            let cols=((f32::from(bounds.size.width)-16.)/f32::from(cell.width)).floor().clamp(2.,500.) as u16;let rows=((f32::from(bounds.size.height)-16.)/f32::from(cell.height)).floor().clamp(1.,200.) as u16;
            if g.parser.screen().size()!=(rows,cols){g.parser.screen_mut().set_size(rows,cols);g.rows.clear();g.selection=None;g.history();if let Some(s)=&g.session{s.resize(rows,cols);}}
        },move|bounds,_,window,cx|{
            window.with_content_mask(Some(ContentMask{bounds}),|window|render::paint(&mut paint.borrow_mut(),bounds,focused,window,cx));
            if focused{window.handle_input(&focus,ElementInputHandler::new(bounds,input),cx);}
            let owner=owner.clone();window.on_mouse_event(move|event:&MouseUpEvent,phase,_,cx|{if phase==DispatchPhase::Capture&&event.button==MouseButton::Left{let _=owner.update(cx,|this,cx|{let mut g=this.grid.borrow_mut();if g.selecting||g.scrollbar_grab.is_some(){g.selecting=false;g.scrollbar_grab=None;drop(g);cx.notify();}});}});
        }).size_full();
        let cwd=self.cwd.as_ref().map(|p|crate::terminal::shell_cwd(p).to_string_lossy().into_owned()).unwrap_or_default();let active=self.active();
        div().relative().size_full().min_h(px(0.)).flex().flex_col().bg(gpui::rgb(self.grid.borrow().palette.background)).track_focus(&self.focus).key_context("Terminal")
            .on_action(cx.listener(|this,_:&CopyTerminal,w,cx|{cx.stop_propagation();this.copy(w,cx);}))
            .on_action(cx.listener(|this,_:&PasteTerminal,w,cx|{cx.stop_propagation();this.paste(w,cx);}))
            .on_action(cx.listener(|this,_:&ControlC,_,cx|{cx.stop_propagation();this.status="Ctrl+C sent to terminal".into();this.send(b"\x03",cx);}))
            .on_action(cx.listener(|this,_:&ControlD,_,cx|{cx.stop_propagation();this.send(b"\x04",cx);}))
            .on_action(cx.listener(|this,_:&ControlS,_,cx|{cx.stop_propagation();this.send(b"\x13",cx);}))
            .on_action(cx.listener(|this,_:&ControlW,_,cx|{cx.stop_propagation();this.send(b"\x17",cx);}))
            .on_action(cx.listener(|this,_:&ControlF,_,cx|{cx.stop_propagation();this.send(b"\x06",cx);}))
            .on_action(cx.listener(|this,_:&EscapeTerminal,_,cx|{cx.stop_propagation();if this.menu{this.menu=false;cx.notify();}else if this.composing(){this.grid.borrow_mut().marked.clear();cx.notify();}else{this.send(b"\x1b",cx);}}))
            .on_action(cx.listener(|this,_:&CloseTerminal,w,cx|{cx.stop_propagation();this.ask_stop(false,w,cx);}))
            .on_key_down(cx.listener(|this,e,w,cx|this.key_down(e,w,cx)))
            .child(div().h(px(30.)).flex_none().px(px(8.)).flex().items_center().gap(px(6.)).bg(rgb(PANEL)).border_b_1().border_color(rgb(BORDER))
                .child(div().h_full().px(px(3.)).flex().items_center().gap(px(6.)).border_b_2().border_color(rgb(ACCENT)).child(icon(Icon::Terminal).size(px(13.)).text_color(rgb(ACCENT))).child(div().text_size(px(10.)).font_weight(FontWeight::MEDIUM).child("TERMINAL")))
                .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(if self.starting{WARNING}else if active{GREEN}else{DIM})))
                .child(div().id("terminal-directory").flex_1().min_w(px(0.)).truncate().text_size(px(11.)).text_color(rgb(MUTED)).child(self.label.clone()).tooltip({let hint:SharedString=format!("启动目录：{cwd}").into();move|_,cx|cx.new(|_|Hint(hint.clone())).into()}))
                .children((self.grid.borrow().parser.screen().scrollback()>0).then(||icon_button("terminal-history-badge",Icon::Focus,format!("查看历史 ↑{} · 点击回到实时输出",self.grid.borrow().parser.screen().scrollback()),true,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,w,cx|this.return_live(w,cx)))))
                .child(icon_button("terminal-maximize",if self.maximized{Icon::Restore}else{Icon::Maximize},if self.maximized{"还原文档/终端分栏"}else{"放大到整个 Document 区域"},self.maximized,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,w,cx|{this.menu=false;this.maximized=!this.maximized;this.focus.focus(w,cx);cx.emit(TerminalEvent::Maximize(this.maximized));cx.notify();})))
                .child(icon_button("terminal-more",Icon::More,"终端菜单 · 字号、行距、复制、粘贴、停止",self.menu,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|{if this.composing(){this.status="Finish IME input before opening terminal menu".into();this.notice=true;}else{this.menu=!this.menu;}cx.notify();})))
                .child(icon_button("terminal-hide",Icon::ChevronDown,"收起终端，进程继续运行",false,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|{if this.composing(){this.status="Finish IME input before hiding the terminal".into();this.notice=true;cx.notify();}else{this.menu=false;cx.emit(TerminalEvent::Hide);}}))))
            .child(div().id("terminal-grid").flex_1().min_h(px(0.)).overflow_hidden().cursor(CursorStyle::IBeam).child(canvas)
                .on_mouse_down(MouseButton::Left,cx.listener(|this,event:&MouseDownEvent,w,cx|{this.focus.focus(w,cx);let mut g=this.grid.borrow_mut();if f32::from(event.position.x)>=f32::from(g.bounds.right())-8.&&g.thumb().is_some(){let(top,height,travel)=g.thumb().unwrap();let y=f32::from(event.position.y-g.bounds.top());if y<top||y>top+height{let offset=((1.-((y-8.-height/2.)/travel.max(1.)).clamp(0.,1.))*g.history as f32).round() as usize;g.parser.screen_mut().set_scrollback(offset);}g.scrollbar_grab=Some((f32::from(event.position.y),g.parser.screen().scrollback()));g.selecting=false;}else{let p=g.at(event.position);g.selection=Some((p,p));g.selecting=true;}drop(g);cx.stop_propagation();cx.notify();}))
                .on_mouse_move(cx.listener(|this,event:&MouseMoveEvent,_,cx|{let mut g=this.grid.borrow_mut();if let Some((start,offset))=g.scrollbar_grab{let travel=g.thumb().map(|t|t.2).unwrap_or(1.).max(1.);let next=(offset as f32-(f32::from(event.position.y)-start)/travel*g.history as f32).round().clamp(0.,g.history as f32) as usize;g.parser.screen_mut().set_scrollback(next);g.selection=None;drop(g);cx.notify();return;}if g.selecting{let p=g.at(event.position);if let Some((a,_))=g.selection{g.selection=Some((a,p));}drop(g);cx.notify();}}))
                .on_scroll_wheel(cx.listener(Self::wheel)))
            .children((self.starting||!active||self.notice).then(||div().h(px(20.)).flex_none().px(px(10.)).truncate().text_size(px(10.)).text_color(rgb(DIM)).child(self.status.clone())))
            .children(self.more_menu(cx))
    }
}
impl EntityInputHandler for TerminalView{
    fn accepts_text_input(&self,_:&mut Window,_:&mut Context<Self>)->bool{self.session.as_ref().is_some_and(|s|s.running())&&!self.starting&&!self.menu}
    fn text_for_range(&mut self,range:Range<usize>,adjusted:&mut Option<Range<usize>>,_:&mut Window,_:&mut Context<Self>)->Option<String>{let g=self.grid.borrow();let text=g.marked.encode_utf16().collect::<Vec<_>>();let start=range.start.min(text.len());let end=range.end.min(text.len()).max(start);*adjusted=Some(start..end);Some(String::from_utf16_lossy(&text[start..end]))}
    fn selected_text_range(&mut self,_:bool,_:&mut Window,_:&mut Context<Self>)->Option<UTF16Selection>{let n=self.grid.borrow().marked.encode_utf16().count();Some(UTF16Selection{range:n..n,reversed:false})}
    fn marked_text_range(&self,_:&mut Window,_:&mut Context<Self>)->Option<Range<usize>>{let n=self.grid.borrow().marked.encode_utf16().count();(n>0).then_some(0..n)}
    fn unmark_text(&mut self,_:&mut Window,cx:&mut Context<Self>){self.grid.borrow_mut().marked.clear();cx.notify();}
    fn replace_text_in_range(&mut self,_:Option<Range<usize>>,text:&str,_:&mut Window,cx:&mut Context<Self>){self.send(text.as_bytes(),cx);}
    fn replace_and_mark_text_in_range(&mut self,_:Option<Range<usize>>,text:&str,_:Option<Range<usize>>,_:&mut Window,cx:&mut Context<Self>){self.grid.borrow_mut().marked=text.into();cx.notify();}
    fn paste(&mut self,item:ClipboardItem,w:&mut Window,cx:&mut Context<Self>){if let Some(text)=item.text(){self.paste_text(text,w,cx);}}
    fn bounds_for_range(&mut self,_:Range<usize>,_:Bounds<Pixels>,_:&mut Window,_:&mut Context<Self>)->Option<Bounds<Pixels>>{let g=self.grid.borrow();let(r,c)=g.parser.screen().cursor_position();Some(Bounds::new(g.bounds.origin+point(px(8.)+g.cell.width*c as f32,px(8.)+g.cell.height*r as f32),g.cell))}
    fn character_index_for_point(&mut self,_:Point<Pixels>,_:&mut Window,_:&mut Context<Self>)->Option<usize>{None}
}
#[cfg(test)]mod tests{
    use super::{key_bytes,Grid};
    use gpui::{Keystroke,point,px};
    #[test]fn selecting_the_second_half_of_a_wide_glyph_selects_the_whole_cell(){let mut g=Grid::new();g.parser.process("A中B".as_bytes());assert_eq!(g.at(point(px(28.),px(12.))),(0,1));g.selection=Some(((0,0),(0,1)));assert_eq!(g.copy(),"A中");}
#[test]fn key_encoding_does_not_treat_printable_text_as_commands(){assert!(key_bytes(&Keystroke::parse("a").unwrap(),false).is_none());assert_eq!(key_bytes(&Keystroke::parse("ctrl-c").unwrap(),false),Some(vec![3]));assert_eq!(key_bytes(&Keystroke::parse("up").unwrap(),true),Some(b"\x1bOA".to_vec()));}
}
