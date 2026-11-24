//! Vertical stage axis with document clicks and breadcrumb-based flow drill-down.
use super::*;
use crate::flow::{Flow,Call};
use super::visual;
const CARD_W:f32=320.;
fn semantic_height(m:&crate::meaning::Meaning)->f32{214.+if m.condition.is_some()||m.confirmation.is_some(){28.}else{0.}+if m.input.is_some(){24.}else{0.}+if m.output.is_some(){24.}else{0.}}
#[derive(Clone)]
struct Card{id:String,title:String,source:String,line:usize,x:f32,y:f32,h:f32,kind:&'static str,context_source:Option<String>,meaning:crate::meaning::Meaning,evidence:Option<(String,usize)>}
#[derive(Clone)]
struct Connection{from:usize,to:usize,side:bool}
#[derive(Default)]
struct Diagram{cards:Vec<Card>,edges:Vec<Connection>}
impl Diagram{
 fn push(&mut self,card:Card)->usize{let index=self.cards.len();self.cards.push(card);index}
 fn call(&mut self,flows:&[Flow],call:&Call,id:String,x:f32,y:f32,source:&str,context:&crate::meaning::Meaning)->usize{
  let child=flows.iter().find(|f|f.source==call.target);
  let mut meaning=child.map(|f|f.meaning.clone()).unwrap_or_default();
  if let Some(condition)=&context.condition{meaning.condition=Some(condition.clone());}
  // Contextual condition belongs to the caller; target semantics keep their own source.
  let title=if meaning.action.text.is_empty(){call.title.clone()}else{meaning.action.text.clone()};
  self.push(Card{id,title,source:call.target.clone(),line:1,x,y,h:semantic_height(&meaning),kind:"call",context_source:context.condition.as_ref().map(|_|source.to_string()),meaning,evidence:Some((source.into(),call.line))})
 }
 fn build(flows:&[Flow],root:&Flow)->Self{
  let mut diagram=Self::default();let x=600.;
  let mut previous=diagram.push(Card{id:root.source.clone(),title:if root.meaning.action.text.is_empty(){root.title.clone()}else{root.meaning.action.text.clone()},source:root.source.clone(),line:1,x,y:20.,h:semantic_height(&root.meaning),kind:if root.entry{"main"}else{"subroot"},context_source:None,meaning:root.meaning.clone(),evidence:None});
  let mut y=20.+diagram.cards[previous].h+64.;
  for stage in &root.stages{
   let id=format!("{}/stage/{}",root.source,stage.line);
   let index=diagram.push(Card{id:id.clone(),title:if stage.meaning.action.text.is_empty(){stage.title.clone()}else{stage.meaning.action.text.clone()},source:root.source.clone(),line:stage.line,x,y,h:semantic_height(&stage.meaning),kind:"stage",context_source:None,meaning:stage.meaning.clone(),evidence:None});
   diagram.edges.push(Connection{from:previous,to:index,side:false});previous=index;
   let mut sides=[y,y];
   for(i,call)in stage.calls.iter().enumerate(){let side=i%2;let call_x=if side==0{240.}else{960.};
    let target=diagram.call(flows,call,format!("{id}/call/{}:{}",call.line,call.target),call_x,sides[side],&root.source,&stage.meaning);
    diagram.edges.push(Connection{from:index,to:target,side:true});sides[side]+=diagram.cards[target].h+42.;
   }
   y=(y+diagram.cards[index].h+64.).max(sides[0]+20.).max(sides[1]+20.);
  }
  diagram
 }
}
#[derive(Clone,Debug,PartialEq)]
struct Location{source:String,camera:Point<Pixels>,zoom:f32,selected:Option<String>}
fn has_flow(flows:&[Flow],source:Option<&str>)->bool{source.is_some_and(|source|flows.iter().any(|f|f.source==source))}
fn restore(history:&mut Vec<Location>,index:usize)->Option<Location>{let location=history.get(index)?.clone();history.truncate(index);Some(location)}
pub(crate) struct FlowCanvas{
 owner:WeakEntity<Studio>,graph:Option<Graph>,revision:u64,root:Option<String>,history:Vec<Location>,diagram:Diagram,selected:Option<String>,trace:HashMap<String,crate::ui::pi_trace::State>,
 camera:Point<Pixels>,zoom:f32,pan:Option<(Point<Pixels>,Point<Pixels>)>,bounds:Rc<Cell<Bounds<Pixels>>>,center_pending:bool,
}
impl FlowCanvas{
 pub(super) fn new(owner:WeakEntity<Studio>)->Self{Self{owner,graph:None,revision:u64::MAX,root:None,history:vec![],diagram:Default::default(),selected:None,trace:HashMap::new(),camera:Point::default(),zoom:0.65,pan:None,bounds:Rc::new(Cell::new(Bounds::default())),center_pending:true}}
 pub(super) fn sync(&mut self,revision:u64,graph:&Graph,cx:&mut Context<Self>){
  if self.revision==revision{return;}let workspace_changed=self.graph.as_ref().is_none_or(|old|old.root!=graph.root);
  self.graph=Some(graph.clone());self.revision=revision;
  if workspace_changed{self.root=None;self.selected=None;self.history.clear();self.center_pending=true;}
  if !has_flow(&graph.flows,self.root.as_deref()){self.root=graph.flows.iter().find(|f|f.entry).map(|f|f.source.clone());self.history.clear();self.center_pending=true;}
  self.rebuild();cx.notify();
 }
 pub(super) fn set_trace(&mut self,trace:HashMap<String,crate::ui::pi_trace::State>,cx:&mut Context<Self>){if self.trace!=trace{self.trace=trace;cx.notify();}}
 fn trace_state(&self,card:&Card)->Option<crate::ui::pi_trace::State>{
  if card.kind=="stage" {return None;}
  if card.kind=="call"&&self.diagram.cards.iter().filter(|c|c.kind=="call"&&c.source==card.source).count()!=1{return None;}
  self.trace.get(&card.source).copied()
 }
 fn rebuild(&mut self){self.diagram=self.graph.as_ref().and_then(|g|g.flows.iter().find(|f|Some(&f.source)==self.root.as_ref()).map(|root|Diagram::build(&g.flows,root))).unwrap_or_default();if self.selected.as_ref().is_some_and(|id|!self.diagram.cards.iter().any(|n|&n.id==id)){self.selected=None;}}
 fn source(&self,path:String,line:usize,window:&mut Window,cx:&mut Context<Self>)->bool{
  self.owner.update(cx,|owner,cx|{
   if !owner.composition_ready(window,cx){return false;}
   owner.reference(path.clone(),line,window,cx);
   true // Accepted; disk loading and source-line navigation complete asynchronously.
  }).unwrap_or(false)
 }
 fn enter(&mut self,target:&str,cx:&mut Context<Self>){
  if self.root.as_deref()==Some(target)||!self.graph.as_ref().is_some_and(|g|has_flow(&g.flows,Some(target))){return;}
  if let Some(source)=self.root.clone(){self.history.push(Location{source,camera:self.camera,zoom:self.zoom,selected:self.selected.clone()});}
  self.root=Some(target.into());self.selected=Some(target.into());self.zoom=0.65;self.center_pending=true;self.pan=None;self.rebuild();cx.notify();
 }
 fn go_back(&mut self,index:usize,cx:&mut Context<Self>){
  let Some(location)=restore(&mut self.history,index)else{return;};
  if !self.graph.as_ref().is_some_and(|g|has_flow(&g.flows,Some(&location.source))){self.go_back(index.saturating_sub(1),cx);return;}
  self.root=Some(location.source);self.selected=location.selected;self.camera=location.camera;self.zoom=location.zoom;self.center_pending=false;self.pan=None;self.rebuild();cx.notify();
 }
 fn zoom_at(&mut self,zoom:f32,anchor:Point<Pixels>,cx:&mut Context<Self>){let zoom=zoom.clamp(MIN_ZOOM,MAX_ZOOM);self.camera=zoom_offset(self.camera,anchor,self.zoom,zoom);self.zoom=zoom;cx.notify();}
 fn fit(&mut self,cx:&mut Context<Self>){
  if self.diagram.cards.is_empty(){return;}let bounds=self.bounds.get().size;
  let (x0,y0,x1,y1)=self.diagram.cards.iter().fold((f32::INFINITY,f32::INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY),|(x0,y0,x1,y1),n|(x0.min(n.x),y0.min(n.y),x1.max(n.x+CARD_W),y1.max(n.y+n.h)));
  self.zoom=((f32::from(bounds.width)-48.).max(1.)/(x1-x0)).min((f32::from(bounds.height)-48.).max(1.)/(y1-y0)).clamp(MIN_ZOOM,MAX_ZOOM);
  self.camera=point(bounds.width*0.5-px((x0+x1)*0.5*self.zoom),bounds.height*0.5-px((y0+y1)*0.5*self.zoom));cx.notify();
 }
}
impl Render for FlowCanvas{
 fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
  let extent=self.bounds.get().size;
  if self.center_pending&&extent.width>px(0.){self.camera=point(extent.width*0.5-px(760.*self.zoom),px(22.));self.center_pending=false;}
  let z=self.zoom;let camera=self.camera;let bounds_cell=self.bounds.clone();let entity_id=cx.entity().entity_id();
  let running=crate::ui::pi_trace::State::Running;
  let pulse=if self.diagram.cards.iter().any(|card|self.trace_state(card)==Some(running)){
   window.request_animation_frame();
   use std::sync::OnceLock;
   static START:OnceLock<std::time::Instant>=OnceLock::new();
   (START.get_or_init(std::time::Instant::now).elapsed().as_secs_f32()*5.).sin()*0.5+0.5
  }else{0.};
  let runtime_color=|state:crate::ui::pi_trace::State|if state==running{0x4e80dd+(((pulse*71.)as u32)<<8)+(pulse*34.)as u32}else{state.color()};
  let segments:Vec<_>=self.diagram.edges.iter().map(|edge|{let a=&self.diagram.cards[edge.from];let b=&self.diagram.cards[edge.to];(a.x,a.y,a.h,b.x,b.y,b.h,edge.side,(if edge.side {self.trace_state(b)}else{None}).map(|state|visual::EdgeStyle{color:runtime_color(state),width:if state==running{2.5+pulse*2.}else{3.},alpha:1.,related:true}).unwrap_or_else(||visual::edge_style(self.selected.as_deref(),&a.id,&b.id)))}).collect();
  let mut content=div().absolute().inset_0().overflow_hidden().child(canvas(move|bounds,_,app|{let old=bounds_cell.replace(bounds);if old.size!=bounds.size{app.notify(entity_id);}},move|bounds,_,window,_|{
   let grid=24.;for x in 0..((f32::from(bounds.size.width)/grid).ceil()as usize+1){for y in 0..((f32::from(bounds.size.height)/grid).ceil()as usize+1){let at=bounds.origin+point(px(f32::from(camera.x).rem_euclid(grid)+x as f32*grid),px(f32::from(camera.y).rem_euclid(grid)+y as f32*grid));window.paint_quad(fill(Bounds::new(at,size(px(1.),px(1.))),rgb(0x2b2e35)));}}
   for(ax,ay,ah,bx,by,bh,side,style)in segments{
    let (from,to)=if side{if bx<ax{((ax,ay+ah*0.5),(bx+CARD_W,by+bh*0.5))}else{((ax+CARD_W,ay+ah*0.5),(bx,by+bh*0.5))}}else{((ax+CARD_W*0.5,ay+ah),(bx+CARD_W*0.5,by))};
    let start=bounds.origin+camera+point(px(from.0*z),px(from.1*z));let end=bounds.origin+camera+point(px(to.0*z),px(to.1*z));
    visual::connection(window,start,end,side,true,style);
   }
  }).absolute().size_full());
  let compact=z<0.38;
  for(i,card)in self.diagram.cards.iter().enumerate(){
   let color=match card.kind{"main"=>0xb5a2ce,"subroot"|"call"=>0x62c7c7,_=>0x4ea1ff};
   let (label,glyph)=match card.kind{"main"=>("MAIN SKILL",Icon::Skill),"subroot"=>("SUBFLOW ROOT",Icon::Skill),"call"=>("SUBFLOW",Icon::Branch),_=>("PHASE / STEP",Icon::Layers)};
   let runtime=self.trace_state(card);let selected=self.selected.as_deref()==Some(card.id.as_str());let border=runtime.map(runtime_color).unwrap_or(if selected{visual::INCOMING}else{BORDER});
   let path=card.source.clone();let line=card.line;let click_path=card.source.clone();let click_line=card.line;let is_call=card.kind=="call";
   let select_id=card.id.clone();let source_id=card.id.clone();
   let filename=card.source.rsplit('/').next().unwrap_or(&card.source).to_string();
   let mut footer=div().h(px((26.*z).max(24.))).flex_none().flex().items_center().justify_between().gap(px(4.*z)).border_t_1().border_color(rgb(BORDER)).pt(px(4.*z));
   footer=footer.child(icon_button(("flow-source",i),Icon::File,format!("Open source · {} · line {line}",card.source),false,true).w(px((24.*z).max(20.))).h(px((22.*z).max(18.)))
    .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.selected=Some(source_id.clone());cx.notify();this.source(path.clone(),line,w,cx);} )));
   if let Some((source,line))=&card.evidence{let source=source.clone();let line=*line;let evidence_id=card.id.clone();
    footer=footer.child(icon_button(("flow-evidence",i),Icon::Link,format!("Open call site · {source} · line {line}"),false,true).w(px((24.*z).max(20.))).h(px((22.*z).max(18.))).on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.selected=Some(evidence_id.clone());cx.notify();this.source(source.clone(),line,w,cx);} )));
   }
   let mut background=rgb(if selected{RAISED}else{BG});
   if let Some(state)=runtime{
    let tint=gpui::rgb(runtime_color(state));
    let strength=if state==running{0.22+pulse*0.16}else{0.22};
    background.r+=(tint.r-background.r)*strength;
    background.g+=(tint.g-background.g)*strength;
    background.b+=(tint.b-background.b)*strength;
   }
   let mut card_view=div().id(SharedString::from(format!("flow-card:{}",card.id))).absolute().left(camera.x+px(card.x*z)).top(camera.y+px(card.y*z))
    .w(px(CARD_W*z)).h(px(card.h*z)).bg(background).rounded(px(9.)).border_color(rgb(border)).overflow_hidden().flex().flex_col().cursor_pointer().shadow_md();
   card_view=if selected||runtime.is_some(){card_view.border_2()}else{card_view.border_1()};
   card_view=card_view
    .child(div().absolute().left(px(0.)).top(px(10.*z)).bottom(px(10.*z)).w(px((3.*z).max(2.)+if runtime==Some(running){pulse*4.}else{0.})).rounded(px(2.)).bg(rgb(runtime.map(runtime_color).unwrap_or(color))))
    .child(div().flex_1().min_h(px(0.)).px(px(15.*z)).py(px(11.*z)).flex().flex_col().gap(px(5.*z))
     .child(div().h(px(18.*z)).flex_none().flex().items_center().justify_between().line_height(px((13.*z).max(10.)))
      .child(div().flex().items_center().gap(px(5.*z)).text_color(rgb(color)).text_size(px((10.*z).max(8.))).font_weight(FontWeight::SEMIBOLD)
       .child(icon(glyph).size(px((13.*z).max(9.))).text_color(rgb(color))).child(if runtime==Some(crate::ui::pi_trace::State::Running){"● RUNNING"}else{label}))
      .child(div().text_size(px((10.*z).max(8.))).text_color(rgb(if selected{visual::INCOMING}else{DIM})).child(if selected{"SELECTED".into()}else{format!("L{}",card.line)})))
     .child(div().h(px(if compact{45.*z}else{44.*z})).flex_none().overflow_hidden().text_size(px((16.*z).max(8.))).line_height(px((21.*z).max(10.))).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(card.title.clone()))
     .children((!compact).then(||{
      let purpose=card.meaning.purpose.as_ref();let purpose_path=card.source.clone();let purpose_id=card.id.clone();
      let purpose_line=purpose.map_or(card.line,|p|p.line);
      div().id(("card-purpose",i)).h(px(48.*z)).flex_none().overflow_hidden().text_size(px((11.*z).max(9.))).line_height(px((16.*z).max(12.))).text_color(rgb(MUTED))
       .child(format!("{}：{}",if card.meaning.purpose_explicit{"目的"}else{"原文"},purpose.map(|p|p.text.as_str()).unwrap_or("暂无明确说明，请查看来源文档")))
       .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
       .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|{cx.stop_propagation();this.selected=Some(purpose_id.clone());let opened=this.source(purpose_path.clone(),purpose_line,w,cx);if opened&&is_call&&event.click_count()>1{this.enter(&purpose_path,cx);}cx.notify();}))
     }))
     .children((!compact&&(card.meaning.condition.is_some()||card.meaning.confirmation.is_some())).then(||{
      let mut tags=div().h(px(24.*z)).flex_none().flex().items_center().gap(px(4.*z)).overflow_hidden();
      if let Some(condition)=&card.meaning.condition{
       let path=card.context_source.clone().unwrap_or_else(||card.source.clone());let line=condition.line;let id=card.id.clone();
       let label:String=condition.text.chars().take(10).collect();
       tags=tags.child(button(("card-condition",i),format!("条件 · {label}"),false,true).max_w(px(170.*z)).h(px((22.*z).max(16.))).px(px(5.*z)).text_size(px((9.*z).max(8.))).bg(visual::tint(0x62c7c7,0.12)).text_color(rgb(0x62c7c7))
        .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.selected=Some(id.clone());this.source(path.clone(),line,w,cx);cx.notify();})));
      }
      if let Some(confirmation)=&card.meaning.confirmation{
       let path=card.source.clone();let line=confirmation.line;let id=card.id.clone();
       tags=tags.child(button(("card-confirmation",i),"需用户确认",false,true).h(px((22.*z).max(16.))).px(px(5.*z)).text_size(px((9.*z).max(8.))).bg(visual::tint(WARNING,0.12)).text_color(rgb(WARNING))
        .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.selected=Some(id.clone());this.source(path.clone(),line,w,cx);cx.notify();})));
      }tags
     }))
     .children((!compact).then(||{
      let mut io=div().flex_none().flex().flex_col().gap(px(3.*z));
      for (label,value)in [("输入",card.meaning.input.as_ref()),("输出",card.meaning.output.as_ref())]{if let Some(value)=value{
       let path=card.source.clone();let line=value.line;let id=card.id.clone();
       io=io.child(div().id(SharedString::from(format!("card-io:{}:{label}",card.id))).h(px(21.*z)).overflow_hidden().text_size(px((10.*z).max(9.))).line_height(px((14.*z).max(11.))).text_color(rgb(MUTED)).truncate().child(format!("{label}：{}",value.text))
        .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.selected=Some(id.clone());this.source(path.clone(),line,w,cx);cx.notify();})));
      }}io
     }))
     .children((!compact).then(||div().flex_1().min_h(px(0.)).flex().items_end().text_size(px((10.*z).max(8.))).text_color(rgb(DIM)).truncate().child(filename)))
     .children((!compact).then_some(footer)))
    .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
    .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|{
      this.selected=Some(select_id.clone());cx.notify();
      let opened=this.source(click_path.clone(),click_line,w,cx);
      if opened&&is_call&&event.click_count()>1{this.enter(&click_path,cx);}
    }));
   content=content.child(card_view);
  }
  if self.diagram.cards.is_empty(){content=content.child(div().size_full().flex().items_center().justify_center().text_color(rgb(MUTED)).child("No SKILL.md entry found. Switch to References or open a Skill workspace."));}
  let mut choices=div().flex().items_center().gap(px(5.)).overflow_hidden();
  if let Some(graph)=&self.graph{for(i,root)in graph.flows.iter().filter(|f|f.entry).enumerate(){let id=root.source.clone();choices=choices.child(button(("flow-root",i),root.title.clone(),Some(&id)==self.root.as_ref(),true).max_w(px(240.)).on_click(cx.listener(move|this,_,_,cx|{this.root=Some(id.clone());this.selected=None;this.history.clear();this.center_pending=true;this.rebuild();cx.notify();})));}}
  let mut trail=div().id("flow-breadcrumbs").flex().items_center().gap(px(4.)).overflow_x_scroll().h(px(30.));
  if !self.history.is_empty(){
   let last=self.history.len()-1;
   trail=trail.child(icon_button("flow-back",Icon::ArrowLeft,"Return to parent graph",false,true).on_click(cx.listener(move|this,_,_,cx|this.go_back(last,cx))));
  }
  for (i,location)in self.history.iter().enumerate(){
   let title=self.graph.as_ref().and_then(|g|g.flows.iter().find(|f|f.source==location.source)).map(|f|f.title.clone()).unwrap_or_else(||location.source.clone());
   trail=trail.child(button(("flow-crumb",i),title,false,true).max_w(px(200.)).on_click(cx.listener(move|this,_,_,cx|this.go_back(i,cx)))).child(caption("›"));
  }
  let current=self.graph.as_ref().and_then(|g|g.flows.iter().find(|f|Some(&f.source)==self.root.as_ref())).map(|f|f.title.clone()).unwrap_or_else(||"No flow".into());
  trail=trail.child(div().text_size(px(12.)).text_color(rgb(ACCENT)).child(current));
  let view=div().id("flow-camera").flex_1().min_h(px(0.)).relative().overflow_hidden().child(content)
   .on_mouse_down(MouseButton::Left,cx.listener(|this,_,_,cx|{this.selected=None;cx.notify();}))
   .on_mouse_down(MouseButton::Right,cx.listener(|this,e:&MouseDownEvent,w,cx|{this.pan=Some((e.position,this.camera));w.prevent_default();cx.stop_propagation();cx.notify();}))
   .on_mouse_move(cx.listener(|this,e:&MouseMoveEvent,_,cx|{if e.pressed_button==Some(MouseButton::Right){if let Some((start,offset))=this.pan{this.camera=pan_offset(offset,e.position-start);cx.notify();}}else{this.pan=None;}}))
   .on_mouse_up(MouseButton::Right,cx.listener(|this,_,_,cx|{this.pan=None;cx.notify();})).on_mouse_up_out(MouseButton::Right,cx.listener(|this,_,_,cx|{this.pan=None;cx.notify();}))
   .on_scroll_wheel(cx.listener(|this,e:&ScrollWheelEvent,w,cx|{w.prevent_default();cx.stop_propagation();if this.pan.is_none(){let anchor=e.position-this.bounds.get().origin;this.zoom_at(wheel_zoom(this.zoom,f32::from(e.delta.pixel_delta(px(20.)).y)),anchor,cx);}}))
   .child(div().absolute().bottom(px(14.)).left(px(14.)).flex().gap(px(4.)).p(px(4.)).rounded(px(7.)).bg(rgb(PANEL)).occlude()
    .child(icon_button("flow-home",Icon::Skill,"Return to main axis",false,true).on_click(cx.listener(|this,_,_,cx|{this.zoom=0.65;this.center_pending=true;cx.notify();})))
    .child(icon_button("flow-fit",Icon::Focus,"Fit all flow nodes",false,true).on_click(cx.listener(|this,_,_,cx|this.fit(cx))))
    .child(caption(format!("{:.0}%",z*100.))));
  div().size_full().flex().flex_col().bg(rgb(CANVAS))
   .child(div().flex_none().px(px(10.)).py(px(7.)).flex().flex_col().gap(px(4.)).border_b_1().border_color(rgb(BORDER))
    .child(choices).child(trail).child(div().text_size(px(10.)).text_color(rgb(WARNING)).child("候选流程：按文档阶段顺序排列；调用边来自引用，不代表已确认的执行、条件或返回关系。")))
   .child(view)
 }
}
#[cfg(test)]mod tests{
 use super::*;
 fn flows()->Vec<Flow>{
  let call=|target:&str|crate::flow::Call{target:target.into(),title:target.into(),line:4};
  let root=Flow{source:"SKILL.md".into(),title:"主技能".into(),entry:true,meaning:Default::default(),stages:vec![crate::flow::Stage{title:"Step 1".into(),line:2,summary:String::new(),meaning:Default::default(),calls:vec![call("a.md"),call("b.md")]},crate::flow::Stage{title:"Step 2".into(),line:8,summary:String::new(),meaning:Default::default(),calls:vec![call("a.md")]}]};
  let child=Flow{source:"a.md".into(),title:"A".into(),entry:false,meaning:Default::default(),stages:vec![crate::flow::Stage{title:"子流程检查".into(),line:16,summary:String::new(),meaning:Default::default(),calls:vec![call("SKILL.md")]}]};
  vec![root,child]
 }
 #[::core::prelude::v1::test]fn axis_and_calls_do_not_embed_child_stages(){
  let flows=flows();let root=Diagram::build(&flows,&flows[0]);
  let stages:Vec<_>=root.cards.iter().filter(|n|n.kind=="stage").collect();assert_eq!(stages[0].x,stages[1].x);assert!(stages[1].y>stages[0].y);
  let calls:Vec<_>=root.cards.iter().filter(|n|n.kind=="call").collect();assert!(calls[0].x<600.&&calls[1].x>600.);assert_ne!(calls[0].id,calls[2].id);
  assert!(!root.cards.iter().any(|n|n.title=="子流程检查"));
  let child=Diagram::build(&flows,&flows[1]);assert!(child.cards.iter().any(|n|n.title=="子流程检查"&&n.source=="a.md"&&n.line==16));
  assert!(has_flow(&flows,Some("a.md")),"a child remains a valid graph after refresh");
 }
 #[::core::prelude::v1::test]fn inherited_conditions_retain_the_caller_source(){
  let mut flows=flows();flows[0].stages[0].meaning.condition=Some(crate::meaning::Excerpt{text:"用户批准后".into(),line:12});
  let graph=Diagram::build(&flows,&flows[0]);let call=graph.cards.iter().find(|c|c.kind=="call").unwrap();
  assert_eq!(call.context_source.as_deref(),Some("SKILL.md"));assert_eq!(call.meaning.condition.as_ref().unwrap().line,12);
  let stages:Vec<_>=graph.cards.iter().filter(|c|c.kind=="stage").collect();assert!(stages[1].y>=stages[0].y+stages[0].h);
 }
 #[::core::prelude::v1::test]fn breadcrumb_restores_camera_and_truncates_descendants(){
  let a=Location{source:"a".into(),camera:point(px(-234.),px(456.)),zoom:1.3,selected:Some("a/stage/1".into())};
  let b=Location{source:"b".into(),camera:point(px(100.),px(-500.)),zoom:0.5,selected:None};
  let mut history=vec![a.clone(),b.clone()];
  assert_eq!(restore(&mut history,1),Some(b));assert_eq!(history,vec![a.clone()]);
  assert_eq!(restore(&mut history,0),Some(a));assert!(history.is_empty());assert!(restore(&mut history,0).is_none());
 }
}
