use super::{controls::*, theme::*,icons::{icon,Icon,file_icon}};
use crate::{layout, role, Studio};
use gpui::*;
use super::theme::rgb;
use std::collections::{HashMap, HashSet};
use crate::core::Graph;
mod minimap;
mod visual;
pub(crate) mod flow;
use std::{cell::Cell, rc::Rc};

impl Studio {
    pub(super) fn graph_view(&mut self, visible:&HashSet<String>,cx:&mut Context<Self>)->Div {
        let mut panel=div().size_full().min_w(px(0.)).overflow_hidden().bg(rgb(CANVAS)).flex().flex_col();
        let Some(graph)=&self.graph else {
            return panel.child(bar().child(caption("Reference graph"))).child(div().flex_1().flex().flex_col().items_center().justify_center().gap(px(16.))
                .child(icon(Icon::Graph).size(px(44.)).text_color(rgb(ACCENT)))
                .child(div().text_size(px(21.)).font_weight(FontWeight::SEMIBOLD).child("Start with a workspace"))
                .child(div().text_color(rgb(MUTED)).child("Explore references between Skills, documents and scripts."))
                .child(primary_icon("welcome-open",Icon::FolderOpen,"Open workspace").h(px(34.)).on_click(cx.listener(|this,_,w,cx|this.open(w,cx)))));
        };

        if self.graph_canvas.is_none(){let owner=cx.entity().downgrade();self.graph_canvas=Some(cx.new(|_|GraphCanvas::new(owner)));}
        let entity=self.graph_canvas.as_ref().unwrap().clone();
        entity.update(cx,|canvas,cx|canvas.sync(self.graph_revision,graph,visible,&self.positions,&self.roles,self.selected.as_deref(),self.zoom,cx));
        let link_count=entity.read(cx).references.len();
        if self.flow_canvas.is_none(){let owner=cx.entity().downgrade();self.flow_canvas=Some(cx.new(|_|flow::FlowCanvas::new(owner)));}
        let flow_entity=self.flow_canvas.as_ref().unwrap().clone();
        let paths=self.pi_trace.read(cx).paths();
        flow_entity.update(cx,|canvas,cx|{canvas.sync(self.graph_revision,graph,cx);canvas.set_trace(paths,cx);});
        panel=panel.child(bar().bg(rgb(PANEL)).child(div().flex_1().min_w(px(0.)).flex().items_center().gap(px(8.)).child(icon(Icon::Graph).text_color(rgb(ACCENT))).child(if self.flow_mode{"Skill orchestration"}else{"Reference graph"}).child(caption(if self.flow_mode{"Document stages · candidate".into()}else{format!("{} nodes · {link_count} links",visible.len())})))
            .child(icon_button("graph-refresh",Icon::Refresh,"Refresh workspace index",false,true).on_click(cx.listener(|this,_,w,cx|this.refresh(w,cx)))));
        let mut scopes=div().flex().gap(px(2.)).p(px(2.)).bg(rgb(BG)).rounded(px(6.));
        for (i,(scope,glyph,hint)) in [(crate::filter::Scope::Direct,Icon::ArrowRight,"Direct Skill references"),(crate::filter::Scope::Recursive,Icon::Branch,"Recursive Skill references"),(crate::filter::Scope::All,Icon::Layers,"All files")].into_iter().enumerate(){
            scopes=scopes.child(icon_button(("scope",i),glyph,hint,self.scope==scope,true).h(px(26.)).on_click(cx.listener(move|this,_,_,cx|{this.scope=scope;this.focus=false;cx.notify();})));
        }
        let mut toolbar=bar().h(px(40.)).child(icon_button("flow-view",Icon::Branch,"Skill orchestration",self.flow_mode,true).on_click(cx.listener(|this,_,_,cx|{this.flow_mode=true;cx.notify();})))
            .child(icon_button("references-view",Icon::Link,"File references",!self.flow_mode,true).on_click(cx.listener(|this,_,_,cx|{this.flow_mode=false;cx.notify();})));
        if !self.flow_mode{
            toolbar=toolbar.child(scopes).child(separator())
                .child(icon_button("focus-selection",Icon::Focus,if self.focus{"Clear focus"}else{"Focus selected file and its references"},self.focus,self.selected.is_some()).on_click(cx.listener(|this,_,_,cx|{if this.selected.is_some(){this.focus=!this.focus;cx.notify();}})))
                .child(icon_button("missing-toggle",Icon::BrokenLink,if self.show_missing{"Hide broken references"}else{"Show broken references"},self.show_missing,true).on_click(cx.listener(|this,_,_,cx|{this.show_missing=!this.show_missing;cx.notify();})))
                .child(div().flex_1()).child(icon_button("layout-reset",Icon::Layout,"Reset to automatic layout",false,true).on_click(cx.listener(|this,_,_,cx|this.reset_layout(cx))));
        }
        panel=panel.child(toolbar);
        // Reserve the remaining column height explicitly for the entity.
        if self.flow_mode{panel.child(div().flex_1().min_h(px(0.)).w_full().overflow_hidden().child(flow_entity))}
        else{panel.child(div().flex_1().min_h(px(0.)).w_full().overflow_hidden().child(entity))}
    }
}

#[derive(Clone, Debug)]
struct ReferenceGroup { source:String, target:String, lines:Vec<usize>, missing:bool }
fn group_references(graph:&Graph,visible:&HashSet<String>)->Vec<ReferenceGroup>{
    let mut groups=std::collections::BTreeMap::<(String,String),ReferenceGroup>::new();
    for e in &graph.edges {
        if !visible.contains(&e.source)||!visible.contains(&e.target){continue;}
        let group=groups.entry((e.source.clone(),e.target.clone())).or_insert_with(||ReferenceGroup{source:e.source.clone(),target:e.target.clone(),lines:vec![],missing:e.missing});
        group.lines.push(e.line);
    }
    groups.into_values().map(|mut g|{g.lines.sort_unstable();g.lines.dedup();g}).collect()
}
// Camera state uses unrestricted screen pixels, never node coordinates.
fn pan_offset(start:Point<Pixels>,delta:Point<Pixels>)->Point<Pixels>{start+delta}
const MIN_ZOOM:f32=0.05;
const MAX_ZOOM:f32=3.;
fn drag_position(start:layout::Position,delta:Point<Pixels>,zoom:f32)->layout::Position{
    layout::Position{
        x:(start.x+f32::from(delta.x)/zoom).clamp(-layout::MAX_COORDINATE,layout::MAX_COORDINATE),
        y:(start.y+f32::from(delta.y)/zoom).clamp(-layout::MAX_COORDINATE,layout::MAX_COORDINATE),
    }
}
fn wheel_zoom(zoom:f32,delta:f32)->f32{
    if !delta.is_finite(){return zoom;}
    (zoom*(delta.clamp(-240.,240.)*0.0016).exp()).clamp(MIN_ZOOM,MAX_ZOOM)
}
fn zoom_offset(offset:Point<Pixels>,anchor:Point<Pixels>,old_zoom:f32,new_zoom:f32)->Point<Pixels>{
    // Keep the world-space point under the pointer at the same screen position.
    anchor-(anchor-offset)*(new_zoom/old_zoom)
}
fn world_bounds(positions:&layout::Positions)->(f32,f32,f32,f32){
    positions.values().fold((f32::INFINITY,f32::INFINITY,f32::NEG_INFINITY,f32::NEG_INFINITY),|(x0,y0,x1,y1),p|(x0.min(p.x),y0.min(p.y),x1.max(p.x+264.),y1.max(p.y+188.)))
}
pub(crate) struct GraphCanvas {
    owner:WeakEntity<Studio>,
    revision:u64,
    graph:Option<Graph>,
    visible:HashSet<String>,
    saved_positions:layout::Positions,
    automatic_positions:layout::Positions,
    positions:layout::Positions,
    roles:layout::Roles,
    node_roles:HashMap<String,String>,
    meanings:HashMap<String,crate::meaning::Meaning>,
    incoming:HashMap<String,HashSet<String>>,
    outgoing:HashMap<String,HashSet<String>>,
    references:Vec<ReferenceGroup>,
    reference_menu:Option<ReferenceGroup>,
    selected:Option<String>,
    zoom:f32,
    drag:Option<(String,Point<Pixels>,layout::Position)>,
    drag_moved:bool,
    camera:Point<Pixels>,
    viewport_bounds:Rc<Cell<Bounds<Pixels>>>,
    pan:Option<(Point<Pixels>,Point<Pixels>)>,
    minimap:minimap::MiniMapState,
}
impl GraphCanvas {
    fn new(owner:WeakEntity<Studio>)->Self{
        Self{owner,revision:u64::MAX,graph:None,visible:HashSet::new(),saved_positions:Default::default(),automatic_positions:Default::default(),positions:Default::default(),roles:Default::default(),node_roles:Default::default(),meanings:Default::default(),incoming:Default::default(),outgoing:Default::default(),references:vec![],reference_menu:None,selected:None,zoom:1.,drag:None,drag_moved:false,camera:Point::default(),viewport_bounds:Rc::new(Cell::new(Bounds::default())),pan:None,minimap:Default::default()}
    }
    fn sync(&mut self,revision:u64,graph:&Graph,visible:&HashSet<String>,saved:&layout::Positions,roles:&layout::Roles,selected:Option<&str>,zoom:f32,cx:&mut Context<Self>){
        let changed=self.revision!=revision || self.visible!=*visible;
        let roles_changed=self.roles!=*roles;
        let positions_changed=self.saved_positions!=*saved;
        let presentation_changed=self.selected.as_deref()!=selected || self.zoom!=zoom;
        if changed{
            if self.visible!=*visible||self.graph.as_ref().is_none_or(|old|old.root!=graph.root){self.camera=Point::default();self.pan=None;}
            self.graph=Some(graph.clone());self.revision=revision;self.visible=visible.clone();
            self.references=group_references(graph,visible);self.reference_menu=None;
            self.meanings=graph.flows.iter().map(|flow|(flow.source.clone(),flow.meaning.clone())).collect();
            self.automatic_positions=layout::automatic(graph,visible);
            self.incoming.clear();self.outgoing.clear();
            for e in &graph.edges {self.incoming.entry(e.target.clone()).or_default().insert(e.source.clone());self.outgoing.entry(e.source.clone()).or_default().insert(e.target.clone());}
        }
        if changed||roles_changed{self.roles=roles.clone();self.node_roles=role::index(graph,roles);}
        if changed||positions_changed {
            self.positions=self.automatic_positions.clone();
            for(id,p)in saved{if visible.contains(id){self.positions.insert(id.clone(),*p);}}
            self.saved_positions=saved.clone();self.drag=None;
        }
        self.selected=selected.map(str::to_string);self.zoom=zoom;
        if changed||roles_changed||positions_changed||presentation_changed{cx.notify();}
    }
    pub(crate) fn close_reference_menu(&mut self,cx:&mut Context<Self>){if self.reference_menu.take().is_some(){cx.notify();}}
    fn reference(&mut self,id:String,line:usize,window:&mut Window,cx:&mut Context<Self>){self.reference_menu=None;let _=self.owner.update(cx,|owner,cx|owner.reference(id,line,window,cx));cx.notify();}
    fn commit_drag(&mut self,cx:&mut Context<Self>){
        if let Some((id,_,_))=self.drag.take(){if self.drag_moved{
            if let Some(position)=self.positions.get(&id).copied(){let _=self.owner.update(cx,|owner,cx|{owner.positions.insert(id,position);owner.save_layout(cx);});}
        }}
    }
    fn end_pan(&mut self,cx:&mut Context<Self>){if self.pan.take().is_some(){cx.notify();}}
    fn set_zoom_at(&mut self,zoom:f32,anchor:Point<Pixels>,cx:&mut Context<Self>){
        let zoom=zoom.clamp(MIN_ZOOM,MAX_ZOOM);if (zoom-self.zoom).abs()<0.00001{return;}
        self.camera=zoom_offset(self.camera,anchor,self.zoom,zoom);
        self.zoom=zoom;
        let _=self.owner.update(cx,|owner,cx|{owner.zoom=zoom;cx.notify();});cx.notify();
    }
    fn fit_all(&mut self,cx:&mut Context<Self>){
        let bounds=self.viewport_bounds.get();
        if self.positions.is_empty()||bounds.size.width<=px(0.)||bounds.size.height<=px(0.){return;}
        let (min_x,min_y,max_x,max_y)=world_bounds(&self.positions);
        let zoom=((f32::from(bounds.size.width)-64.).max(1.)/(max_x-min_x)).min((f32::from(bounds.size.height)-64.).max(1.)/(max_y-min_y)).clamp(MIN_ZOOM,MAX_ZOOM);
        self.zoom=zoom;
        self.camera=point(bounds.size.width*0.5-px((min_x+max_x)*0.5*zoom),bounds.size.height*0.5-px((min_y+max_y)*0.5*zoom));
        self.pan=None;
        let _=self.owner.update(cx,|owner,cx|{owner.zoom=zoom;cx.notify();});cx.notify();
    }
    fn home(&mut self,cx:&mut Context<Self>){
        let target=self.graph.as_ref().and_then(|graph|graph.nodes.iter().find(|n|self.visible.contains(&n.id)&&self.node_roles.get(&n.id).is_some_and(|role|role=="main")).or_else(||graph.nodes.iter().find(|n|self.visible.contains(&n.id)&&n.kind=="skill"))).and_then(|n|self.positions.get(&n.id)).copied();
        if let Some(p)=target{
            let bounds=self.viewport_bounds.get();self.zoom=1.;self.camera=point(bounds.size.width*0.5-px(p.x+132.),bounds.size.height*0.5-px(p.y+82.));self.pan=None;
            let _=self.owner.update(cx,|owner,cx|{owner.zoom=1.;cx.notify();});cx.notify();
        }else{self.fit_all(cx);}
    }
    fn set_zoom(&mut self,zoom:f32,cx:&mut Context<Self>){
        let bounds=self.viewport_bounds.get();
        self.set_zoom_at(zoom,point(bounds.size.width*0.5,bounds.size.height*0.5),cx);
    }
}
impl Render for GraphCanvas {
    fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
        let Some(graph)=&self.graph else{return div().size_full();};
        let positions=&self.positions;
        let visible=&self.visible;
        let z=self.zoom;
        let camera=self.camera;
        let viewport_bounds=self.viewport_bounds.clone();
        let canvas_id=cx.entity().entity_id();


        let links:Vec<_>=self.references.iter().filter_map(|e|Some((*positions.get(&e.source)?,*positions.get(&e.target)?,e.missing,visual::edge_style(self.selected.as_deref(),&e.source,&e.target)))).collect();

        let mut layer=div().absolute().inset_0().overflow_hidden().child(canvas(move|bounds,_,app|{
            let previous=viewport_bounds.replace(bounds);
            if previous.size!=bounds.size{app.notify(canvas_id);}
        },move|bounds,_,w,_|{
            // Paint only the viewport; the world has no scroll bounds.
            let spacing=(28.*z).max(14.);
            let x0=f32::from(camera.x).rem_euclid(spacing);
            let y0=f32::from(camera.y).rem_euclid(spacing);
            for x in 0..((f32::from(bounds.size.width)/spacing).ceil() as usize+1){
                for y in 0..((f32::from(bounds.size.height)/spacing).ceil() as usize+1){
                    let p=bounds.origin+point(px(x0+x as f32*spacing),px(y0+y as f32*spacing));
                    w.paint_quad(fill(Bounds::new(p,size(px(1.),px(1.))),rgb(0x2b2e35)));
                }
            }
            for (from,to,missing,mut style) in links {
                let start=bounds.origin+camera+point(px((from.x+264.)*z),px((from.y+94.)*z));
                let end=bounds.origin+camera+point(px(to.x*z),px((to.y+94.)*z));
                if missing&&!style.related{style.color=0xdc8c95;}
                visual::connection(w,start,end,true,missing,style);
            }
        }).absolute().size_full());
        for (i,e) in self.references.iter().enumerate() {if let(Some(a),Some(b))=(positions.get(&e.source),positions.get(&e.target)) {
            let group=e.clone();
            let edge_style=visual::edge_style(self.selected.as_deref(),&e.source,&e.target);
            let label=if e.lines.len()==1{format!("L{}",e.lines[0])}else{format!("{} references",e.lines.len())};
            layer=layer.child(button(("reference",i),label,false,true).absolute().left(camera.x+px((a.x+264.+b.x)*0.5*z)).top(camera.y+px(((a.y+b.y)*0.5+94.)*z-10.)).h(px(20.)).px(px(5.)).text_size(px(9.)).text_color(rgb(if edge_style.related{edge_style.color}else{DIM})).border_1().border_color(rgb(if edge_style.related{edge_style.color}else{BORDER}))
                .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
                .on_click(cx.listener(move|this,_,w,cx|{
                    if group.lines.len()==1 {this.reference(group.source.clone(),group.lines[0],w,cx);}
                    else {this.reference_menu=Some(group.clone());cx.notify();}
                })));
        }}
        for (i,n) in graph.nodes.iter().filter(|n|visible.contains(&n.id)).enumerate(){
            let p=positions[&n.id];let id=n.id.clone();let click_id=id.clone();let missing=n.kind=="missing";
            let kind=self.node_roles.get(&n.id).map(String::as_str).unwrap_or("file");let color=role::color(kind);let selected=self.selected.as_deref()==Some(&id);
            let meaning=self.meanings.get(&n.id);
            let title=meaning.filter(|m|!m.action.text.is_empty()).map(|m|m.action.text.clone()).unwrap_or_else(||if n.title.is_empty(){n.label.clone()}else{n.title.clone()});
            let summary=meaning.and_then(|m|m.purpose.as_ref()).map(|p|p.text.clone()).unwrap_or_else(||n.summary.clone());
            let context=meaning.and_then(|m|m.condition.as_ref()).map(|p|format!("条件：{}",p.text)).unwrap_or_else(||n.headings.iter().take(3).cloned().collect::<Vec<_>>().join(" · "));
            let count_out=self.outgoing.get(n.id.as_str()).map_or(0,HashSet::len);let count_in=self.incoming.get(n.id.as_str()).map_or(0,HashSet::len);
            let hint=if kind=="subflow"&&!self.roles.contains_key(&n.id){" · inferred"}else{""};
            layer=layer.child(div().id(("graph-node",i)).absolute().left(camera.x+px(p.x*z)).top(camera.y+px(p.y*z)).w(px(264.*z)).h(px(188.*z)).flex().flex_col().overflow_hidden().rounded(px(8.)).border_1().border_color(rgb(if selected{visual::INCOMING}else{BORDER})).bg(rgb(if selected{RAISED}else{BG})).shadow_md().cursor(if self.pan.is_some(){CursorStyle::ClosedHand}else{CursorStyle::PointingHand}).hover(|s|s.border_color(rgb(color)))
                .child(div().absolute().left(px(0.)).top(px(10.*z)).bottom(px(10.*z)).w(px((3.*z).max(2.))).rounded(px(2.)).bg(rgb(color)))
                .child(div().flex_1().min_h(px(0.)).px(px(13.*z)).pt(px(10.*z)).flex().flex_col().gap(px(5.*z))
                    .child(div().flex().items_center().gap(px(5.*z)).text_size(px(11.*z)).line_height(px(14.*z)).text_color(rgb(color)).child(icon(file_icon(kind)).size(px(13.*z)).text_color(rgb(color))).child(format!("{}{}",role::label(kind),hint)))
                    .child(div().h(px(42.*z)).flex_none().overflow_hidden().text_size(px(16.*z)).line_height(px(21.*z)).font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().text_size(px(11.*z)).line_height(px(14.*z)).text_color(rgb(DIM)).truncate().child(n.id.clone()))
                    .child(div().h(px(32.*z)).overflow_hidden().text_size(px(12.5*z)).line_height(px(17.*z)).text_color(rgb(0xa3a8b2)).child(summary))
                    .child(div().text_size(px(11.*z)).line_height(px(14.*z)).text_color(rgb(DIM)).truncate().child(context)))
                .child(div().h(px(25.*z)).flex_none().px(px(13.*z)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between().text_size(px(10.5*z)).line_height(px(14.*z)).text_color(rgb(DIM)).child(format!("↗ {count_out} outgoing")).child(format!("↙ {count_in} incoming")))
                .on_mouse_down(MouseButton::Left,cx.listener(move|this,e:&MouseDownEvent,_,cx|{this.drag=Some((id.clone(),e.position,p));this.drag_moved=false;cx.stop_propagation();}))
                .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|{if !missing&&!this.drag_moved{let pin=event.click_count()>1;let id=click_id.clone();let _=this.owner.update(cx,|owner,cx|owner.select_file(id,pin,w,cx));}})));
        }
        let minimap=self.minimap_view(cx);
        let viewport=div().id("graph-viewport").flex_1().min_h(px(0.)).relative().overflow_hidden()
            .child(div().id("graph-camera").size_full().overflow_hidden()
                // Explicit wheel handling: native scroll must not also shift the view.
                .on_scroll_wheel(cx.listener(|this,e:&ScrollWheelEvent,window,cx|{
                    window.prevent_default();cx.stop_propagation();
                    if this.drag.is_some()||this.pan.is_some(){return;}
                    let delta=f32::from(e.delta.pixel_delta(px(20.)).y);
                    let anchor=e.position-this.viewport_bounds.get().origin;
                    this.set_zoom_at(wheel_zoom(this.zoom,delta),anchor,cx);
                }))
                .cursor(if self.pan.is_some(){CursorStyle::ClosedHand}else{CursorStyle::Arrow})
                .on_mouse_down(MouseButton::Right,cx.listener(|this,e:&MouseDownEvent,window,cx|{
                    this.drag=None;this.pan=Some((e.position,this.camera));
                    window.prevent_default();cx.stop_propagation();cx.notify();
                }))
                .on_mouse_up(MouseButton::Right,cx.listener(|this,_,_,cx|{this.end_pan(cx);cx.stop_propagation();}))
                .on_mouse_up_out(MouseButton::Right,cx.listener(|this,_,_,cx|this.end_pan(cx)))
                .on_mouse_move(cx.listener(|this,e:&MouseMoveEvent,_,cx|{
                    if e.pressed_button==Some(MouseButton::Right){
                        if let Some((start,offset))=this.pan{
                            this.camera=pan_offset(offset,e.position-start);
                            cx.stop_propagation();cx.notify();
                        }
                        return;
                    }
                    // Also recover if a release happened outside the application.
                    this.end_pan(cx);
                    if e.pressed_button!=Some(MouseButton::Left){return;}
                    if let Some((id,start,pos))=&this.drag{
                        let dx=f32::from(e.position.x-start.x);let dy=f32::from(e.position.y-start.y);
                        if dx.abs()+dy.abs()>3.{this.drag_moved=true;this.positions.insert(id.clone(),drag_position(*pos,point(px(dx),px(dy)),this.zoom));cx.notify();}
                    }
                }))
                .on_mouse_up(MouseButton::Left,cx.listener(|this,_,_,cx|{this.commit_drag(cx);}))
                .on_mouse_up_out(MouseButton::Left,cx.listener(|this,_,_,cx|{this.commit_drag(cx);})).child(layer))
            .child(div().absolute().left(px(14.)).bottom(px(14.)).flex().gap(px(2.)).p(px(4.)).rounded(px(7.)).bg(rgb(PANEL)).border_1().border_color(rgb(BORDER)).occlude()
                .child(icon_button("zoom-minus",Icon::ZoomOut,"Zoom out",false,true).on_click(cx.listener(|this,_,_,cx|{this.set_zoom((this.zoom-0.1).max(MIN_ZOOM),cx);})))
                .child(icon_button("zoom-reset",Icon::ResetZoom,"Reset zoom to 100%",false,true).on_click(cx.listener(|this,_,_,cx|{this.set_zoom(1.,cx);})))
                .child(icon_button("zoom-plus",Icon::ZoomIn,"Zoom in",false,true).on_click(cx.listener(|this,_,_,cx|{this.set_zoom((this.zoom+0.1).min(MAX_ZOOM),cx);})))
                .child(caption(format!("{:.0}%",z*100.)))
                .child(icon_button("fit-graph",Icon::Focus,"Fit all nodes",false,true).on_click(cx.listener(|this,_,_,cx|this.fit_all(cx))))
                .child(icon_button("home-skill",Icon::Skill,"Return to main Skill",false,true).on_click(cx.listener(|this,_,_,cx|this.home(cx)))))
            .child(minimap);

        // viewport uses flex_1, so its entity root MUST be a flex column.
        // A plain block root gives this child zero height and hides the whole graph.
        let mut root=div().size_full().min_h(px(0.)).relative().flex().flex_col().child(viewport)
            .child(div().flex_none().h(px(20.)).px(px(10.)).text_size(px(10.)).text_color(rgb(DIM)).child("Wheel to zoom · Right-drag to pan freely · Left-drag to move nodes"));
        if let Some(group)=self.reference_menu.clone(){
            let mut menu=div().id("reference-menu").occlude().w(px(300.)).max_h(px(320.)).overflow_y_scroll().p(px(10.)).bg(rgb(POPOVER)).border_1().border_color(rgb(BORDER)).rounded(px(8.)).flex().flex_col().gap(px(5.))
                .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
                .child(caption("Select reference location"));
            for line in group.lines {
                let source=group.source.clone();
                menu=menu.child(button(("reference-line",line),format!("Line {line}"),false,true).on_click(cx.listener(move|this,_,w,cx|this.reference(source.clone(),line,w,cx))));
            }
            root=root.child(div().absolute().inset_0().occlude().bg(rgba(0x00000060)).flex().justify_center().items_center().child(menu)
                .on_mouse_down(MouseButton::Left,cx.listener(|this,_,_,cx|{this.reference_menu=None;cx.notify();})));
        }
        root
    }
}
#[cfg(test)]mod tests{
 use super::*;
 #[::core::prelude::v1::test]fn wheel_zoom_direction_limits_and_anchor(){
  assert!(wheel_zoom(1.,60.)>1.);assert!(wheel_zoom(1.,-60.)<1.);
  assert_eq!(wheel_zoom(MAX_ZOOM,120.),MAX_ZOOM);assert_eq!(wheel_zoom(MIN_ZOOM,-120.),MIN_ZOOM);
  assert_eq!(wheel_zoom(1.,0.),1.);assert_eq!(wheel_zoom(1.,f32::NAN),1.);
  let offset=point(px(-100.),px(-200.));let anchor=point(px(250.),px(350.));
  let next=zoom_offset(offset,anchor,1.,1.5);
  assert_eq!((anchor-offset)/1.,(anchor-next)/1.5);
 }
 #[::core::prelude::v1::test]fn nodes_cross_origin_at_every_zoom_and_round_trip(){
  let start=layout::Position{x:40.,y:40.};
  for zoom in [0.05,0.5,1.,2.,3.]{
    let moved=drag_position(start,point(px(-100.*zoom),px(-120.*zoom)),zoom);
    assert_eq!(moved,layout::Position{x:-60.,y:-80.});
    let positions=layout::Positions::from([("node".into(),moved)]);
    let bytes=serde_json::to_vec(&layout::LayoutFile{version:1,positions:positions.clone(),roles:Default::default()}).unwrap();
    assert_eq!(layout::decode(&bytes).unwrap().positions,positions);
  }
 }
 #[::core::prelude::v1::test]fn pan_is_unbounded_and_reversible(){
  let start=point(px(-100.),px(-200.));
  assert_eq!(pan_offset(start,point(px(1000.),px(1000.))),point(px(900.),px(800.)));
  let moved=pan_offset(start,point(px(-1000000.),px(1000000.)));
  assert_eq!(pan_offset(moved,point(px(1000000.),px(-1000000.))),start);
 }
 #[::core::prelude::v1::test]fn duplicate_links_group_with_distinct_sorted_locations(){
  let graph=Graph{root:String::new(),entries:vec![],nodes:vec![],warnings:vec![],flows:vec![],edges:vec![
   crate::core::Edge{source:"a".into(),target:"b".into(),line:50,missing:false},
   crate::core::Edge{source:"a".into(),target:"b".into(),line:10,missing:false},
   crate::core::Edge{source:"a".into(),target:"b".into(),line:10,missing:false}]};
  let groups=group_references(&graph,&HashSet::from(["a".into(),"b".into()]));
  assert_eq!(groups.len(),1);assert_eq!(groups[0].lines,vec![10,50]);
 }
}
