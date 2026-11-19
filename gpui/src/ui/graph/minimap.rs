//! A proportional, theme-aware navigator; all interactions change only camera state.
use super::*;

const WIDTH:f32=212.;
const MAP_HEIGHT:f32=118.;
#[derive(Clone,Copy,Debug)]
struct Projection { min_x:f32,min_y:f32,scale:f32,origin:Point<Pixels> }
impl Projection {
    fn new(world:(f32,f32,f32,f32),extent:Size<Pixels>)->Self {
        let (x,y,right,bottom)=world;
        let w=(right-x).max(1.);let h=(bottom-y).max(1.);
        let available_w=(f32::from(extent.width)-24.).max(1.);
        let available_h=(f32::from(extent.height)-20.).max(1.);
        let scale=(available_w/w).min(available_h/h);
        Self{min_x:x,min_y:y,scale,origin:point(px(12.+(available_w-w*scale)*0.5),px(10.+(available_h-h*scale)*0.5))}
    }
    fn project(self,x:f32,y:f32)->Point<Pixels>{self.origin+point(px((x-self.min_x)*self.scale),px((y-self.min_y)*self.scale))}
    fn unproject(self,p:Point<Pixels>)->layout::Position{layout::Position{x:self.min_x+f32::from(p.x-self.origin.x)/self.scale,y:self.min_y+f32::from(p.y-self.origin.y)/self.scale}}
}
#[derive(Default)]
pub(super) struct MiniMapState {
    collapsed:bool,
    drag:Option<Projection>,
    bounds:Rc<Cell<Bounds<Pixels>>>,
    projection:Rc<Cell<Option<Projection>>>,
}
fn translucent(token:u32,alpha:f32)->Rgba{let mut c=rgb(token);c.a=alpha;c}
fn rounded_outline(bounds:Bounds<Pixels>,color:Rgba,window:&mut Window){
    window.paint_quad(quad(bounds,px(3.),translucent(CANVAS,0.),px(1.),color,BorderStyle::Solid));
}
impl GraphCanvas {
    fn map_navigate(&mut self,position:Point<Pixels>,projection:Projection,cx:&mut Context<Self>){
        let bounds=self.minimap.bounds.get();
        let local=position-bounds.origin;
        let local=point(local.x.clamp(px(0.),bounds.size.width),local.y.clamp(px(0.),bounds.size.height));
        let target=projection.unproject(local);let extent=self.viewport_bounds.get().size;
        self.camera=point(extent.width*0.5-px(target.x*self.zoom),extent.height*0.5-px(target.y*self.zoom));
        cx.notify();
    }
    pub(super) fn minimap_view(&mut self,cx:&mut Context<Self>)->Div {
        let width=f32::from(self.viewport_bounds.get().size.width);
        // Leave room for the bottom-left zoom controls as well as the card.
        let narrow=width>0.&&width<460.;
        if self.minimap.collapsed||narrow {
            return div().absolute().right(px(16.)).bottom(px(16.)).occlude()
                .child(icon_button("expand-overview",Icon::Graph,if narrow{"Fit graph · expand panel for overview"}else{"Show overview"},false,true)
                    .w(px(32.)).h(px(32.)).rounded(px(9.)).border_1().border_color(rgb(BORDER)).shadow_md()
                    .on_click(cx.listener(move|this,_,_,cx|{if narrow{this.fit_all(cx);}else{this.minimap.collapsed=false;cx.notify();}})));
        }
        let nodes:Vec<_>=self.positions.iter().map(|(id,p)|{
            let color=role::color(self.node_roles.get(id).map(String::as_str).unwrap_or("file"));
            (*p,color,self.selected.as_deref()==Some(id.as_str()))
        }).collect();
        let links:Vec<_>=self.references.iter().filter_map(|e|Some((*self.positions.get(&e.source)?,*self.positions.get(&e.target)?))).collect();
        let (mut left,mut top,mut right,mut bottom)=if self.positions.is_empty(){(0.,0.,1.,1.)}else{world_bounds(&self.positions)};
        let extent=self.viewport_bounds.get().size;
        let vx=-f32::from(self.camera.x)/self.zoom;let vy=-f32::from(self.camera.y)/self.zoom;
        let vw=f32::from(extent.width)/self.zoom;let vh=f32::from(extent.height)/self.zoom;
        left=left.min(vx);top=top.min(vy);right=right.max(vx+vw);bottom=bottom.max(vy+vh);
        let world=(left,top,right,bottom);
        let bounds_cell=self.minimap.bounds.clone();let projection_cell=self.minimap.projection.clone();
        let frozen=self.minimap.drag;
        let count=self.positions.len();let zoom=self.zoom;
        let map=div().id("overview-map").w_full().h(px(MAP_HEIGHT)).relative().overflow_hidden().bg(rgb(CANVAS))
            .cursor(if self.minimap.drag.is_some(){CursorStyle::ClosedHand}else{CursorStyle::Crosshair})
            .child(canvas(move|bounds,_,_|{bounds_cell.set(bounds);},move|bounds,_,window,_|{
                let projection=frozen.unwrap_or_else(||Projection::new(world,bounds.size));projection_cell.set(Some(projection));
                // Quiet grid with enough negative space to read the miniature structure.
                for x in 1..14 {for y in 1..8 {
                    let at=bounds.origin+point(px(x as f32*15.),px(y as f32*15.));
                    window.paint_quad(fill(Bounds::new(at,size(px(1.),px(1.))),translucent(MUTED,0.13)));
                }}
                for (a,b) in links {
                    let from=bounds.origin+projection.project(a.x+264.,a.y+82.);
                    let to=bounds.origin+projection.project(b.x,b.y+82.);
                    let bend=px((f32::from(to.x-from.x).abs()*0.5).max(5.));
                    let mut path=PathBuilder::stroke(px(0.8));path.move_to(from);path.cubic_bezier_to(to,from+point(bend,px(0.)),to-point(bend,px(0.)));
                    if let Ok(path)=path.build(){window.paint_path(path,translucent(MUTED,0.36));}
                }
                let view_origin=bounds.origin+projection.project(vx,vy);
                let view_size=size(px((vw*projection.scale).max(2.)),px((vh*projection.scale).max(2.)));
                // Soft wash below nodes, crisp outline above them.
                window.paint_quad(fill(Bounds::new(view_origin,view_size),translucent(ACCENT,0.08)).corner_radii(px(3.)));
                for (p,color,selected) in nodes {
                    let at=bounds.origin+projection.project(p.x,p.y);
                    let rect=Bounds::new(at,size(px((264.*projection.scale).max(3.)),px((188.*projection.scale).max(2.))));
                    if selected {
                        window.paint_quad(fill(Bounds::new(at-point(px(2.),px(2.)),rect.size+size(px(4.),px(4.))),translucent(ACCENT,0.17)).corner_radii(px(3.)));
                    }
                    window.paint_quad(fill(rect,translucent(color,if selected{1.}else{0.7})).corner_radii(px(2.)));
                    if selected{rounded_outline(rect,rgb(TEXT),window);}
                }
                rounded_outline(Bounds::new(view_origin,view_size),translucent(ACCENT,0.9),window);
                // Tiny corner grips make the current viewport immediately recognizable.
                for (x,y,sx,sy) in [(0.,0.,1.,1.),(1.,0.,-1.,1.),(0.,1.,1.,-1.),(1.,1.,-1.,-1.)] {
                    let corner=view_origin+point(view_size.width*x,view_size.height*y);
                    let arm_x=view_size.width.min(px(5.));let arm_y=view_size.height.min(px(5.));
                    let mut path=PathBuilder::stroke(px(1.5));path.move_to(corner+point(arm_x*sx,px(0.)));path.line_to(corner);path.line_to(corner+point(px(0.),arm_y*sy));
                    if let Ok(path)=path.build(){window.paint_path(path,rgb(ACCENT));}
                }
            }).absolute().size_full())
            .on_mouse_down(MouseButton::Left,cx.listener(|this,e:&MouseDownEvent,window,cx|{
                window.prevent_default();cx.stop_propagation();
                if let Some(projection)=this.minimap.projection.get(){this.minimap.drag=Some(projection);this.map_navigate(e.position,projection,cx);}
            }))
            .on_mouse_move(cx.listener(|this,e:&MouseMoveEvent,_,cx|{
                if e.pressed_button==Some(MouseButton::Left){if let Some(projection)=this.minimap.drag{this.map_navigate(e.position,projection,cx);cx.stop_propagation();}}
                else if this.minimap.drag.take().is_some(){cx.notify();}
            }))
            .on_mouse_up(MouseButton::Left,cx.listener(|this,_,_,cx|{this.minimap.drag=None;cx.stop_propagation();cx.notify();}))
            .on_mouse_up_out(MouseButton::Left,cx.listener(|this,_,_,cx|{this.minimap.drag=None;cx.notify();}))
            .on_scroll_wheel(|_,window,cx|{window.prevent_default();cx.stop_propagation();});
        div().absolute().right(px(16.)).bottom(px(16.)).w(px(WIDTH)).rounded(px(11.)).overflow_hidden().bg(rgb(POPOVER)).border_1().border_color(rgb(BORDER)).shadow_lg().occlude()
            .child(div().h(px(30.)).px(px(10.)).flex().items_center().gap(px(6.)).border_b_1().border_color(rgb(BORDER))
                .child(icon(Icon::Graph).size(px(12.)).text_color(rgb(ACCENT)))
                .child(div().flex_1().text_size(px(9.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(MUTED)).child("OVERVIEW"))
                .child(icon_button("overview-fit",Icon::Focus,"Fit all nodes",false,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|this.fit_all(cx))))
                .child(icon_button("collapse-overview",Icon::Minus,"Collapse overview",false,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|{this.minimap.collapsed=true;cx.notify();}))))
            .child(map)
            .child(div().h(px(25.)).px(px(10.)).flex().items_center().justify_between().border_t_1().border_color(rgb(BORDER)).text_size(px(9.)).text_color(rgb(DIM))
                .child(format!("{count} nodes · drag to navigate"))
                .child(div().text_color(rgb(ACCENT)).child(format!("{:.0}%",zoom*100.))))
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[::core::prelude::v1::test]fn projection_preserves_aspect_and_round_trips_negative_world(){
        let p=Projection::new((-100.,-200.,900.,800.),size(px(212.),px(118.)));
        let a=p.project(-100.,-200.);let b=p.project(0.,-100.);
        assert!(f32::from((b.x-a.x)-(b.y-a.y)).abs()<0.001);
        let original=layout::Position{x:234.,y:-123.};let actual=p.unproject(p.project(original.x,original.y));
        assert!((actual.x-original.x).abs()<0.001);assert!((actual.y-original.y).abs()<0.001);
    }
    #[::core::prelude::v1::test]fn empty_or_tiny_bounds_are_safe(){
        let p=Projection::new((0.,0.,0.,0.),size(px(0.),px(0.)));
        assert!(p.scale.is_finite()&&p.scale>0.);
        let restored=p.unproject(p.project(0.,0.));assert_eq!(restored,layout::Position{x:0.,y:0.});
    }
}
