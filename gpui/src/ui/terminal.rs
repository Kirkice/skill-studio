//! Native terminal surface. Parsing/output invalidate only this entity, not Studio.
use crate::terminal::{self,Session,Event};
use super::{controls::*,icons::{icon,Icon},theme::*,theme::rgb};
use gpui::*;
use std::{cell::RefCell,rc::Rc,sync::Arc,path::PathBuf,ops::Range,time::Duration};

actions!(terminal_keys,[CopyTerminal,PasteTerminal,ControlC,ControlD,ControlS,ControlW,ControlF,EscapeTerminal,CloseTerminal]);
pub enum TerminalEvent{Hide}
pub fn bind_keys(cx:&mut App){
    let mut keys=vec![KeyBinding::new("ctrl-shift-c",CopyTerminal,Some("Terminal")),KeyBinding::new("ctrl-shift-v",PasteTerminal,Some("Terminal")),KeyBinding::new("ctrl-c",ControlC,Some("Terminal")),KeyBinding::new("ctrl-d",ControlD,Some("Terminal")),KeyBinding::new("ctrl-s",ControlS,Some("Terminal")),KeyBinding::new("ctrl-w",ControlW,Some("Terminal")),KeyBinding::new("ctrl-f",ControlF,Some("Terminal")),KeyBinding::new("escape",EscapeTerminal,Some("Terminal"))];
    if cfg!(target_os="macos"){keys.extend([KeyBinding::new("cmd-c",CopyTerminal,Some("Terminal")),KeyBinding::new("cmd-v",PasteTerminal,Some("Terminal")),KeyBinding::new("cmd-shift-v",PasteTerminal,Some("Terminal")),KeyBinding::new("cmd-w",CloseTerminal,Some("Terminal"))]);}
    else{keys.push(KeyBinding::new("ctrl-v",PasteTerminal,Some("Terminal")));}cx.bind_keys(keys);
}
struct Run{col:u16,line:ShapedLine}
struct Row{cells:Vec<vt100::Cell>,runs:Vec<Run>}
struct Grid{
    parser:vt100::Parser<terminal::Replies>,session:Option<Arc<Session>>,bounds:Bounds<Pixels>,cell:Size<Pixels>,
    rows:Vec<Row>,font_size:f32,marked:String,selection:Option<((u16,u16),(u16,u16))>,selecting:bool,
}
impl Grid{
    fn new()->Self{Self{parser:terminal::parser(12,80),session:None,bounds:Bounds::default(),cell:size(px(8.),px(20.)),rows:vec![],font_size:13.,marked:String::new(),selection:None,selecting:false}}
    fn at(&self,p:Point<Pixels>)->(u16,u16){let(rows,cols)=self.parser.screen().size();(((f32::from(p.y-self.bounds.origin.y)-8.)/f32::from(self.cell.height)).floor().clamp(0.,rows.saturating_sub(1) as f32) as u16,((f32::from(p.x-self.bounds.origin.x)-8.)/f32::from(self.cell.width)).floor().clamp(0.,cols.saturating_sub(1) as f32) as u16)}
    fn selection(&self)->Option<((u16,u16),(u16,u16))>{let(a,b)=self.selection?;if a==b{return None;}Some(if a<b{(a,b)}else{(b,a)})}
    fn copy(&self)->String{if let Some((a,b))=self.selection(){self.parser.screen().contents_between(a.0,a.1,b.0,b.1.saturating_add(1))}else{self.parser.screen().contents()}}
}
pub struct TerminalView{
    pub focus:FocusHandle,grid:Rc<RefCell<Grid>>,session:Option<Arc<Session>>,reader:Option<Task<()>>,
    pub cwd:Option<PathBuf>,next_cwd:Option<PathBuf>,label:String,status:String,starting:bool,generation:u64,confirming:bool,
}
impl EventEmitter<TerminalEvent> for TerminalView{}
impl TerminalView{
    pub fn new(cx:&mut Context<Self>)->Self{Self{focus:cx.focus_handle(),grid:Rc::new(RefCell::new(Grid::new())),session:None,reader:None,cwd:None,next_cwd:dirs::home_dir(),label:"Terminal".into(),status:"Click start to open a local shell".into(),starting:false,generation:0,confirming:false}}
    pub fn target_directory(&mut self,cwd:PathBuf){self.next_cwd=Some(cwd);}
    pub fn active(&self)->bool{self.starting||self.session.as_ref().is_some_and(|s|s.running())}
    pub fn composing(&self)->bool{!self.grid.borrow().marked.is_empty()}
    pub fn invalidate(&mut self,font_size:u16,cx:&mut Context<Self>){let mut g=self.grid.borrow_mut();g.font_size=font_size as f32;g.rows.clear();drop(g);cx.notify();}
    pub fn start(&mut self,cwd:PathBuf,window:&mut Window,cx:&mut Context<Self>){
        self.next_cwd=Some(cwd.clone());self.focus.focus(window,cx);if self.active(){return;}
        self.generation+=1;let generation=self.generation;self.starting=true;self.cwd=Some(cwd.clone());self.status="Starting local shell…".into();
        let (rows,cols)=self.grid.borrow().parser.screen().size();cx.notify();
        cx.spawn_in(window,async move|this,cx|{
            let result=cx.background_executor().spawn(async move{Session::spawn(&cwd,rows,cols)}).await;
            let _=this.update_in(cx,|this,_,cx|{
                if this.generation!=generation{return;}
                this.starting=false;
                match result{
                    Ok((session,events,label))=>{
                        let session=Arc::new(session);this.session=Some(session.clone());this.label=label;this.status=if cfg!(windows){"Local PTY · Ctrl+C depends on the running program"}else{"Local PTY · live shell session"}.into();
                        let mut g=this.grid.borrow_mut();g.parser=terminal::parser(rows,cols);g.rows.clear();g.marked.clear();g.selection=None;g.session=Some(session);drop(g);
                        this.reader=Some(cx.spawn(async move|this,cx|{
                            while let Ok(first)=events.recv().await{
                                let mut batch=vec![first];let mut budget=8192;
                                while budget<65536{let Ok(event)=events.try_recv()else{break;};budget+=match &event{Event::Data(b)=>b.len(),_=>0};batch.push(event);if batch.len()>=32{break;}}
                                let active=this.update(cx,|this,cx|{
                                    if this.generation!=generation{return false;}
                                    let mut g=this.grid.borrow_mut();
                                    for event in batch{match event{Event::Data(bytes)=>g.parser.process(&bytes),Event::Exit(status)=>this.status=format!("Process ended · {status}"),Event::Error(e)=>this.status=e}}
                                    let replies=std::mem::take(&mut g.parser.callbacks_mut().bytes);if !replies.is_empty(){if let Some(s)=&g.session{let _=s.write(&replies);}}
                                    drop(g);cx.notify();true
                                });
                                if !matches!(active,Ok(true)){break;}
                                cx.background_executor().timer(Duration::from_millis(16)).await;
                            }
                        }));
                    }
                    Err(e)=>this.status=format!("Unable to start shell: {e}"),
                }cx.notify();
            });
        }).detach();
    }
    pub fn stop(&mut self,cx:&mut Context<Self>)->Option<async_channel::Receiver<()>>{
        self.generation+=1;self.starting=false;self.reader=None;
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
        if self.composing(){self.grid.borrow_mut().marked.clear();}
        let mut g=self.grid.borrow_mut();g.parser.screen_mut().set_scrollback(0);g.selection=None;
        if let Some(s)=&self.session{if let Err(e)=s.write(bytes){self.status=e;}}else{self.status="Terminal is not ready; wait for the prompt or restart".into();}drop(g);cx.notify();
    }
    fn copy(&mut self,window:&mut Window,cx:&mut Context<Self>){let text=self.grid.borrow().copy();if !text.is_empty(){cx.write_to_clipboard(ClipboardItem::new_string(text));}self.focus.focus(window,cx);}
    fn paste_text(&mut self,text:String,window:&mut Window,cx:&mut Context<Self>){
        if text.len()>1_000_000{self.status="Paste exceeds 1 MB".into();cx.notify();return;}
        if text.contains(['\n','\r']){
            let generation=self.generation;
            let answer=window.prompt(PromptLevel::Warning,"Paste multiple lines?",Some("Pasting line breaks into a shell can execute commands. Review the clipboard before continuing."),&[PromptButton::ok("Paste"),PromptButton::cancel("Cancel")],cx);
            cx.spawn_in(window,async move|this,cx|{if answer.await.ok()!=Some(0){return;}let _=this.update_in(cx,|this,w,cx|{if this.generation==generation{let bytes=terminal::paste_bytes(&text,this.grid.borrow().parser.screen().bracketed_paste());this.send(&bytes,cx);this.focus.focus(w,cx);}});}).detach();
        }else{let bytes=terminal::paste_bytes(&text,self.grid.borrow().parser.screen().bracketed_paste());self.send(&bytes,cx);self.focus.focus(window,cx);}
    }
    fn paste(&mut self,window:&mut Window,cx:&mut Context<Self>){if self.starting||!self.active(){return;}if let Some(text)=cx.read_from_clipboard().and_then(|i|i.text()){self.paste_text(text,window,cx);}}
    fn key_down(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>){
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
fn color(c:vt100::Color,default:u32)->Rgba{
    let value=match c{vt100::Color::Default=>return rgb(default),vt100::Color::Rgb(r,g,b)=>(u32::from(r)<<16)|(u32::from(g)<<8)|u32::from(b),vt100::Color::Idx(n)=>{
        const COLORS:[u32;16]=[0x16181d,0xe06c75,0x98c379,0xe5c07b,0x61afef,0xc678dd,0x56b6c2,0xdcdfe4,0x5c6370,0xff7a85,0xb3e58b,0xffd48a,0x83c7ff,0xe59dff,0x78dce8,0xffffff];
        if n<16{COLORS[n as usize]}else if n>=232{let v=8+(u32::from(n)-232)*10;(v<<16)|(v<<8)|v}else{let n=u32::from(n)-16;let a=[0,95,135,175,215,255];(a[(n/36)as usize]<<16)|(a[((n/6)%6)as usize]<<8)|a[(n%6)as usize]}
    }};gpui::rgb(value)
}
fn paint_grid(g:&mut Grid,bounds:Bounds<Pixels>,focused:bool,window:&mut Window,cx:&mut App){
    let(rows,cols)=g.parser.screen().size();let origin=bounds.origin+point(px(8.),px(8.));let font_size=px(g.font_size);let base_font=font(MONO_FONT);
    while g.rows.len()>rows as usize{g.rows.pop();}
    for row in 0..rows{
        let cells=(0..cols).filter_map(|col|g.parser.screen().cell(row,col).cloned()).collect::<Vec<_>>();
        let changed=g.rows.get(row as usize).is_none_or(|cached|cached.cells!=cells);
        if changed{
            let mut runs=vec![];let mut col=0usize;
            while col<cells.len(){let c=&cells[col];if c.is_wide_continuation(){col+=1;continue;}
                let start=col;let mut text=if c.contents().is_empty(){" ".to_string()}else{c.contents().to_string()};
                let mut f=base_font.clone();if c.bold(){f.weight=FontWeight::BOLD;}if c.italic(){f.style=FontStyle::Italic;}
                let fg=if c.inverse(){color(c.bgcolor(),BG)}else{color(c.fgcolor(),TEXT)};
                if text.is_ascii(){while col+1<cells.len(){let next=&cells[col+1];if next.is_wide()||next.is_wide_continuation()||!next.contents().is_ascii()||next.fgcolor()!=c.fgcolor()||next.bgcolor()!=c.bgcolor()||next.bold()!=c.bold()||next.italic()!=c.italic()||next.inverse()!=c.inverse()||next.dim()!=c.dim(){break;}col+=1;text.push_str(if next.contents().is_empty(){" "}else{next.contents()});}}
                let run=TextRun{len:text.len(),font:f,color:Hsla::from(fg).opacity(if c.dim(){0.6}else{1.}),..Default::default()};
                let line=window.text_system().shape_line(text.into(),font_size,&[run],None);runs.push(Run{col:start as u16,line});col+=1;
            }
            let row_data=Row{cells,runs};if row as usize==g.rows.len(){g.rows.push(row_data);}else{g.rows[row as usize]=row_data;}
        }
        for(col,c)in g.rows[row as usize].cells.iter().enumerate(){let bg=if c.inverse(){Some(color(c.fgcolor(),TEXT))}else if c.bgcolor()!=vt100::Color::Default{Some(color(c.bgcolor(),BG))}else{None};let pos=origin+point(g.cell.width*col as f32,g.cell.height*row as f32);
            if let Some(bg)=bg{window.paint_quad(fill(Bounds::new(pos,g.cell),bg));}
            if c.underline(){window.paint_quad(fill(Bounds::new(pos+point(px(0.),g.cell.height-px(2.)),size(g.cell.width,px(1.))),color(c.fgcolor(),TEXT)));}
        }
        if let Some((a,b))=g.selection(){if row>=a.0&&row<=b.0{let left=if row==a.0{a.1}else{0};let right=if row==b.0{b.1+1}else{cols};window.paint_quad(fill(Bounds::new(origin+point(g.cell.width*left as f32,g.cell.height*row as f32),size(g.cell.width*(right-left) as f32,g.cell.height)),rgb(SELECTED)));}}
        for run in &g.rows[row as usize].runs{let _=run.line.paint(origin+point(g.cell.width*run.col as f32,g.cell.height*row as f32),g.cell.height,TextAlign::Left,None,window,cx);}
    }
    if focused&&!g.parser.screen().hide_cursor()&&g.parser.screen().scrollback()==0{
        let(r,c)=g.parser.screen().cursor_position();if r<rows&&c<cols{let pos=origin+point(g.cell.width*c as f32,g.cell.height*r as f32);window.paint_quad(fill(Bounds::new(pos,size(px(2.),g.cell.height)),rgb(ACCENT)));}
    }
    if focused&&!g.marked.is_empty(){let(r,c)=g.parser.screen().cursor_position();let run=TextRun{len:g.marked.len(),font:base_font,color:rgb(TEXT).into(),..Default::default()};let line=window.text_system().shape_line(g.marked.clone().into(),font_size,&[run],None);let pos=origin+point(g.cell.width*c as f32,g.cell.height*r as f32);window.paint_quad(fill(Bounds::new(pos,size(line.width,g.cell.height)),rgb(BG)));let _=line.paint(pos,g.cell.height,TextAlign::Left,None,window,cx);window.paint_quad(fill(Bounds::new(pos+point(px(0.),g.cell.height-px(1.)),size(line.width,px(1.))),rgb(ACCENT)));}
}
impl Render for TerminalView{
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
        let focused=self.focus.is_focused(window);let grid=self.grid.clone();let paint=self.grid.clone();let focus=self.focus.clone();let input=cx.entity();let owner=cx.entity().downgrade();
        let f=font(MONO_FONT);let fs=self.grid.borrow().font_size;let id=window.text_system().resolve_font(&f);let cell=size(window.text_system().advance(id,px(fs),'M').map(|s|s.width).unwrap_or(px(fs*0.6)),px(fs*1.55));
        let canvas=canvas(move|bounds,_,_|{let mut g=grid.borrow_mut();g.bounds=bounds;if g.cell!=cell{g.rows.clear();g.cell=cell;}
            let cols=((f32::from(bounds.size.width)-16.)/f32::from(cell.width)).floor().clamp(2.,500.) as u16;let rows=((f32::from(bounds.size.height)-16.)/f32::from(cell.height)).floor().clamp(1.,200.) as u16;
            if g.parser.screen().size()!=(rows,cols){g.parser.screen_mut().set_size(rows,cols);g.rows.clear();g.selection=None;if let Some(s)=&g.session{s.resize(rows,cols);}}
        },move|bounds,_,window,cx|{
            window.with_content_mask(Some(ContentMask{bounds}),|window|paint_grid(&mut paint.borrow_mut(),bounds,focused,window,cx));
            if focused{window.handle_input(&focus,ElementInputHandler::new(bounds,input),cx);}
            let owner=owner.clone();window.on_mouse_event(move|event:&MouseUpEvent,phase,_,cx|{if phase==DispatchPhase::Capture&&event.button==MouseButton::Left{let _=owner.update(cx,|this,cx|{let mut g=this.grid.borrow_mut();if g.selecting{g.selecting=false;drop(g);cx.notify();}});}});
        }).size_full();
        let cwd=self.cwd.as_ref().map(|p|crate::terminal::shell_cwd(p).to_string_lossy().into_owned()).unwrap_or_default();let active=self.active();
        div().size_full().min_h(px(0.)).flex().flex_col().bg(rgb(BG)).track_focus(&self.focus).key_context("Terminal")
            .on_action(cx.listener(|this,_:&CopyTerminal,w,cx|{cx.stop_propagation();this.copy(w,cx);}))
            .on_action(cx.listener(|this,_:&PasteTerminal,w,cx|{cx.stop_propagation();this.paste(w,cx);}))
            .on_action(cx.listener(|this,_:&ControlC,_,cx|{cx.stop_propagation();this.status="Ctrl+C sent to terminal".into();this.send(b"\x03",cx);}))
            .on_action(cx.listener(|this,_:&ControlD,_,cx|{cx.stop_propagation();this.send(b"\x04",cx);}))
            .on_action(cx.listener(|this,_:&ControlS,_,cx|{cx.stop_propagation();this.send(b"\x13",cx);}))
            .on_action(cx.listener(|this,_:&ControlW,_,cx|{cx.stop_propagation();this.send(b"\x17",cx);}))
            .on_action(cx.listener(|this,_:&ControlF,_,cx|{cx.stop_propagation();this.send(b"\x06",cx);}))
            .on_action(cx.listener(|this,_:&EscapeTerminal,_,cx|{cx.stop_propagation();if this.composing(){this.grid.borrow_mut().marked.clear();cx.notify();}else{this.send(b"\x1b",cx);}}))
            .on_action(cx.listener(|this,_:&CloseTerminal,w,cx|{cx.stop_propagation();this.ask_stop(false,w,cx);}))
            .on_key_down(cx.listener(|this,e,w,cx|this.key_down(e,w,cx)))
            .child(div().h(px(34.)).flex_none().px(px(10.)).flex().items_center().gap(px(6.)).bg(rgb(PANEL)).border_b_1().border_color(rgb(BORDER))
                .child(icon(Icon::Terminal).size(px(14.)).text_color(rgb(ACCENT))).child(caption("TERMINAL"))
                .child(div().flex_1().min_w(px(0.)).truncate().text_size(px(10.)).text_color(rgb(DIM)).child(format!("{} · start: {cwd}",self.label)))
                .child(icon_button("terminal-copy",Icon::Copy,"复制选中文本；未选择时复制当前屏幕",false,true).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,w,cx|this.copy(w,cx))))
                .child(icon_button("terminal-paste",Icon::Paste,"粘贴 · 多行会先确认",false,active&&!self.starting).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,w,cx|this.paste(w,cx))))
                .child(icon_button("terminal-restart",if active{Icon::Refresh}else{Icon::Plus},"启动/重新启动终端",false,!self.starting).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,w,cx|{if !this.starting{this.ask_stop(true,w,cx);}})))
                .child(icon_button("terminal-stop",Icon::Stop,"停止终端及运行中的命令（需确认）",false,active).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,w,cx|this.ask_stop(false,w,cx))))
                .child(icon_button("terminal-hide",Icon::ChevronDown,"收起终端，进程继续运行",false,true).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,_,cx|{if this.composing(){this.status="Finish IME input before hiding the terminal".into();cx.notify();}else{cx.emit(TerminalEvent::Hide);}}))))
            .child(div().id("terminal-grid").flex_1().min_h(px(0.)).overflow_hidden().cursor(CursorStyle::IBeam).child(canvas)
                .on_mouse_down(MouseButton::Left,cx.listener(|this,event:&MouseDownEvent,w,cx|{this.focus.focus(w,cx);let mut g=this.grid.borrow_mut();let p=g.at(event.position);g.selection=Some((p,p));g.selecting=true;drop(g);cx.stop_propagation();cx.notify();}))
                .on_mouse_move(cx.listener(|this,event:&MouseMoveEvent,_,cx|{let mut g=this.grid.borrow_mut();if g.selecting{let p=g.at(event.position);if let Some((a,_))=g.selection{g.selection=Some((a,p));}drop(g);cx.notify();}}))
                .on_scroll_wheel(cx.listener(|this,event:&ScrollWheelEvent,w,cx|{let delta=f32::from(event.delta.pixel_delta(px(20.)).y);let mut g=this.grid.borrow_mut();let old=g.parser.screen().scrollback();let next=(old as i64+(delta/20.).round() as i64*3).max(0) as usize;g.parser.screen_mut().set_scrollback(next);g.selection=None;drop(g);w.prevent_default();cx.stop_propagation();cx.notify();})))
            .child(div().h(px(20.)).flex_none().px(px(10.)).truncate().text_size(px(10.)).text_color(rgb(DIM)).child(self.status.clone()))
    }
}
impl EntityInputHandler for TerminalView{
    fn accepts_text_input(&self,_:&mut Window,_:&mut Context<Self>)->bool{self.session.as_ref().is_some_and(|s|s.running())&&!self.starting}
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
    use super::key_bytes;
    use gpui::Keystroke;
    #[test]fn key_encoding_does_not_treat_printable_text_as_commands(){assert!(key_bytes(&Keystroke::parse("a").unwrap(),false).is_none());assert_eq!(key_bytes(&Keystroke::parse("ctrl-c").unwrap(),false),Some(vec![3]));assert_eq!(key_bytes(&Keystroke::parse("up").unwrap(),true),Some(b"\x1bOA".to_vec()));}
}
