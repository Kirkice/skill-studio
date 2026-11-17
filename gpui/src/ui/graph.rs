use super::{controls::*, theme::*,icons::{icon,Icon,file_icon}};
use crate::{layout, role, Studio};
use gpui::*;
use super::theme::rgb;
use std::collections::{HashMap, HashSet};

impl Studio {
    pub(super) fn graph_view(&self, visible:&HashSet<String>,cx:&mut Context<Self>)->Div {
        let mut panel=div().size_full().min_w(px(0.)).bg(rgb(CANVAS)).flex().flex_col();
        let Some(graph)=&self.graph else {
            return panel.child(bar().child(caption("Reference graph"))).child(div().flex_1().flex().flex_col().items_center().justify_center().gap(px(16.))
                .child(icon(Icon::Graph).size(px(44.)).text_color(rgb(ACCENT)))
                .child(div().text_size(px(21.)).font_weight(FontWeight::SEMIBOLD).child("Start with a workspace"))
                .child(div().text_color(rgb(MUTED)).child("Explore references between Skills, documents and scripts."))
                .child(primary("welcome-open","Open workspace…").child(icon(Icon::FolderOpen).text_color(rgb(BG))).h(px(34.)).on_click(cx.listener(|this,_,w,cx|this.open(w,cx)))));
        };
        let z=self.zoom;
        let mut positions=layout::automatic(graph,visible);
        for (id,p) in &self.positions{if visible.contains(id){positions.insert(id.clone(),*p);}}
        let width=positions.values().map(|p|p.x+340.).fold(900_f32,f32::max)*z;
        let height=positions.values().map(|p|p.y+220.).fold(700_f32,f32::max)*z;
        let links:Vec<_>=graph.edges.iter().filter_map(|e|Some((*positions.get(&e.source)?,*positions.get(&e.target)?,e.missing))).collect();
        let link_count=links.len();
        let mut layer=div().relative().w(px(width)).h(px(height)).flex_none().child(canvas(|_,_,_|(),move|bounds,_,w,_|{
            // Sparse grid stays intentionally subtle; cap work for large imported layouts.
            for x in 0..((width/28.).min(140.) as usize){for y in 0..((height/28.).min(140.) as usize){
                let p=bounds.origin+point(px(x as f32*28.),px(y as f32*28.));w.paint_quad(fill(Bounds::new(p,size(px(1.),px(1.))),rgb(0x2b2e35)));
            }}
            for (from,to,missing) in links {
                let start=bounds.origin+point(px((from.x+264.)*z),px((from.y+78.)*z));
                let end=bounds.origin+point(px(to.x*z),px((to.y+78.)*z));
                let bend=px((f32::from(end.x-start.x).abs()*0.5).max(45.*z));
                let mut b=PathBuilder::stroke(px(1.5));if missing{b=b.dash_array(&[px(5.),px(4.)]);}
                b.move_to(start);b.cubic_bezier_to(end,start+point(bend,px(0.)),end-point(bend,px(0.)));
                if let Ok(path)=b.build(){w.paint_path(path,rgb(if missing{0xdc8c95}else{0x545f70}));}
                let mut arrow=PathBuilder::stroke(px(1.5));arrow.move_to(end+point(px(-5.*z),px(-4.*z)));arrow.line_to(end);arrow.line_to(end+point(px(-5.*z),px(4.*z)));
                if let Ok(path)=arrow.build(){w.paint_path(path,rgb(if missing{0xdc8c95}else{0x718098}));}
            }
        }).absolute().size_full());
        let mut incoming:HashMap<&str,HashSet<&str>>=HashMap::new();let mut outgoing:HashMap<&str,HashSet<&str>>=HashMap::new();
        for e in &graph.edges{incoming.entry(&e.target).or_default().insert(&e.source);outgoing.entry(&e.source).or_default().insert(&e.target);}
        for (i,e) in graph.edges.iter().enumerate(){if let(Some(a),Some(b))=(positions.get(&e.source),positions.get(&e.target)){
            let id=e.source.clone();let line=e.line;
            layer=layer.child(button(("reference",i),format!("L{line}"),false,true).absolute().left(px((a.x+264.+b.x)*0.5*z)).top(px(((a.y+b.y)*0.5+78.)*z-10.)).h(px(20.)).px(px(5.)).text_size(px(9.)).border_1().border_color(rgb(BORDER))
                .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).on_click(cx.listener(move|this,_,w,cx|this.reference(id.clone(),line,w,cx))));
        }}
        for (i,n) in graph.nodes.iter().filter(|n|visible.contains(&n.id)).enumerate(){
            let p=positions[&n.id];let id=n.id.clone();let click_id=id.clone();let missing=n.kind=="missing";
            let kind=role::role(n,graph,&self.roles);let color=role::color(kind);let selected=self.selected.as_deref()==Some(&id);
            let title=if n.title.is_empty(){n.label.clone()}else{n.title.clone()};
            let count_out=outgoing.get(n.id.as_str()).map_or(0,HashSet::len);let count_in=incoming.get(n.id.as_str()).map_or(0,HashSet::len);
            let hint=if kind=="subflow"&&!self.roles.contains_key(&n.id){" · inferred"}else{""};
            layer=layer.child(div().id(("graph-node",i)).absolute().left(px(p.x*z)).top(px(p.y*z)).w(px(264.*z)).h(px(164.*z)).flex().flex_col().overflow_hidden().rounded(px(8.)).border_1().border_color(rgb(if selected{color}else{BORDER})).bg(rgb(if selected{RAISED}else{BG})).shadow_sm().cursor_pointer().hover(|s|s.border_color(rgb(color)))
                .child(div().h(px(3.*z)).flex_none().bg(rgb(color)))
                .child(div().flex_1().min_h(px(0.)).px(px(13.*z)).pt(px(10.*z)).flex().flex_col().gap(px(5.*z))
                    .child(div().flex().items_center().gap(px(5.*z)).text_size(px(11.*z)).line_height(px(14.*z)).text_color(rgb(color)).child(icon(file_icon(kind)).size(px(13.*z)).text_color(rgb(color))).child(format!("{}{}",role::label(kind),hint)))
                    .child(div().text_size(px(14.*z)).line_height(px(20.*z)).font_weight(FontWeight::SEMIBOLD).truncate().child(title))
                    .child(div().text_size(px(11.*z)).line_height(px(14.*z)).text_color(rgb(DIM)).truncate().child(n.id.clone()))
                    .child(div().h(px(32.*z)).overflow_hidden().text_size(px(12.5*z)).line_height(px(17.*z)).text_color(rgb(0xa3a8b2)).child(n.summary.clone()))
                    .child(div().text_size(px(11.*z)).line_height(px(14.*z)).text_color(rgb(DIM)).truncate().child(n.headings.iter().take(3).cloned().collect::<Vec<_>>().join(" · "))))
                .child(div().h(px(25.*z)).flex_none().px(px(13.*z)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between().text_size(px(10.5*z)).line_height(px(14.*z)).text_color(rgb(DIM)).child(format!("↗ {count_out} outgoing")).child(format!("↙ {count_in} incoming")))
                .on_mouse_down(MouseButton::Left,cx.listener(move|this,e:&MouseDownEvent,_,cx|{this.drag=Some((id.clone(),e.position,p));this.drag_moved=false;cx.stop_propagation();}))
                .on_click(cx.listener(move|this,_,w,cx|{if !missing&&!this.drag_moved{this.select(click_id.clone(),w,cx);}})));
        }
        let overview:Vec<_>=positions.values().copied().collect();let map_w=width/z;let map_h=height/z;
        let minimap=div().absolute().right(px(14.)).bottom(px(14.)).w(px(130.)).h(px(84.)).rounded(px(6.)).bg(rgb(PANEL)).border_1().border_color(rgb(BORDER)).occlude()
            .child(canvas(|_,_,_|(),move|bounds,_,w,_|{for p in overview{let at=bounds.origin+point(px(6.+p.x/map_w*115.),px(6.+p.y/map_h*68.));w.paint_quad(fill(Bounds::new(at,size(px(8.),px(5.))),rgb(0x71839d)));}}).size_full());
        panel=panel.child(bar().bg(rgb(PANEL)).child(div().flex_1().min_w(px(0.)).flex().items_center().gap(px(8.)).child(icon(Icon::Graph).text_color(rgb(ACCENT))).child("Reference graph").child(caption(format!("{} nodes · {link_count} links",positions.len()))))
            .child(icon_button("graph-refresh",Icon::Refresh,"Refresh workspace index",false,true).on_click(cx.listener(|this,_,w,cx|this.refresh(w,cx)))));
        let mut scopes=div().flex().gap(px(2.)).p(px(2.)).bg(rgb(BG)).rounded(px(6.));
        for (i,(scope,glyph,hint)) in [(crate::filter::Scope::Direct,Icon::ArrowRight,"Direct Skill references"),(crate::filter::Scope::Recursive,Icon::Branch,"Recursive Skill references"),(crate::filter::Scope::All,Icon::Layers,"All files")].into_iter().enumerate(){
            scopes=scopes.child(icon_button(("scope",i),glyph,hint,self.scope==scope,true).h(px(26.)).on_click(cx.listener(move|this,_,_,cx|{this.scope=scope;this.focus=false;cx.notify();})));
        }
        panel=panel.child(bar().h(px(40.)).child(scopes).child(separator())
            .child(icon_button("focus-selection",Icon::Focus,if self.focus{"Clear focus"}else{"Focus selected file and its references"},self.focus,self.selected.is_some()).on_click(cx.listener(|this,_,_,cx|{if this.selected.is_some(){this.focus=!this.focus;cx.notify();}})))
            .child(icon_button("missing-toggle",Icon::BrokenLink,if self.show_missing{"Hide broken references"}else{"Show broken references"},self.show_missing,true).on_click(cx.listener(|this,_,_,cx|{this.show_missing=!this.show_missing;cx.notify();})))
            .child(div().flex_1()).child(icon_button("layout-reset",Icon::Layout,"Reset to automatic layout",false,true).on_click(cx.listener(|this,_,_,cx|this.reset_layout(cx)))));
        let viewport=div().id("graph-viewport").flex_1().min_h(px(0.)).relative().overflow_hidden()
            .child(div().id("graph-scroll").size_full().overflow_scroll()
                .on_mouse_move(cx.listener(|this,e:&MouseMoveEvent,_,cx|{
                    if e.pressed_button!=Some(MouseButton::Left){return;}
                    if let Some((id,start,pos))=&this.drag{
                        let dx=f32::from(e.position.x-start.x);let dy=f32::from(e.position.y-start.y);
                        if dx.abs()+dy.abs()>3.{this.drag_moved=true;this.positions.insert(id.clone(),layout::Position{x:(pos.x+dx/this.zoom).clamp(0.,100000.),y:(pos.y+dy/this.zoom).clamp(0.,100000.)});cx.notify();}
                    }
                }))
                .on_mouse_up(MouseButton::Left,cx.listener(|this,_,_,cx|{if this.drag.take().is_some()&&this.drag_moved{this.save_layout(cx);}}))
                .on_mouse_up_out(MouseButton::Left,cx.listener(|this,_,_,cx|{if this.drag.take().is_some()&&this.drag_moved{this.save_layout(cx);}})).child(layer))
            .child(div().absolute().left(px(14.)).bottom(px(14.)).flex().gap(px(2.)).p(px(4.)).rounded(px(7.)).bg(rgb(PANEL)).border_1().border_color(rgb(BORDER)).occlude()
                .child(icon_button("zoom-minus",Icon::ZoomOut,"Zoom out",false,true).on_click(cx.listener(|this,_,_,cx|{this.zoom=(this.zoom-0.1).max(0.4);cx.notify();})))
                .child(button("zoom-reset",format!("{:.0}%",z*100.),false,true).on_click(cx.listener(|this,_,_,cx|{this.zoom=1.;cx.notify();})))
                .child(icon_button("zoom-plus",Icon::ZoomIn,"Zoom in",false,true).on_click(cx.listener(|this,_,_,cx|{this.zoom=(this.zoom+0.1).min(2.);cx.notify();}))))
            .child(minimap);
        panel.child(viewport)
    }
}
