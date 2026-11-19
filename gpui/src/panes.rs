//! Divider feedback is cheap; expensive editor wrapping happens only on commit.
#[derive(Clone, Copy, Debug)]
pub enum Divider { Workspace, Editor }
#[derive(Clone, Copy, Debug)]
struct Drag { divider:Divider, origin:f32, start_workspace:f32, start_editor:f32, workspace:f32, editor:f32 }
#[derive(Debug)]
pub struct Panes { pub workspace:f32, pub editor:f32, drag:Option<Drag> }
impl Default for Panes { fn default()->Self { Self{workspace:250.,editor:480.,drag:None} } }
impl Panes {
    pub const HANDLE:f32=6.;
    pub fn fit(&mut self,total:f32){
        let total=total.max(612.);
        self.workspace=self.workspace.clamp(150.,420.).min(total-462.);
        self.editor=self.editor.clamp(300.,10000.).min(total-self.workspace-162.);
    }
    pub fn begin(&mut self,divider:Divider,x:f32){
        self.drag=Some(Drag{divider,origin:x,start_workspace:self.workspace,start_editor:self.editor,workspace:self.workspace,editor:self.editor});
    }
    pub fn update(&mut self,x:f32,total:f32)->bool{
        let Some(drag)=&mut self.drag else{return false;};
        if !x.is_finite()||!total.is_finite(){return false;}
        let delta=x-drag.origin;
        let old=(drag.workspace,drag.editor);
        match drag.divider {
            Divider::Workspace=>drag.workspace=(drag.start_workspace+delta).clamp(150.,420_f32.min(total-drag.start_editor-162.).max(150.)),
            Divider::Editor=>drag.editor=(drag.start_editor-delta).clamp(300.,10000_f32.min(total-drag.start_workspace-162.).max(300.)),
        }
        old!=(drag.workspace,drag.editor)
    }
    pub fn guide(&self,total:f32)->Option<f32>{self.drag.map(|d|match d.divider{Divider::Workspace=>d.workspace+Self::HANDLE/2.,Divider::Editor=>total-d.editor-Self::HANDLE/2.})}
    pub fn commit(&mut self)->bool{
        let Some(d)=self.drag.take()else{return false;};self.workspace=d.workspace;self.editor=d.editor;true
    }
    pub fn cancel(&mut self)->bool{self.drag.take().is_some()}
    pub fn dragging(&self)->bool{self.drag.is_some()}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn dragging_never_changes_editor_layout_until_commit(){
        let mut p=Panes::default();p.begin(Divider::Editor,900.);
        for x in (500..900).rev(){p.update(x as f32,1440.);assert_eq!(p.editor,480.);}
        assert!(p.guide(1440.).is_some());assert!(p.commit());assert_eq!(p.editor,880.);assert!(!p.dragging());
    }
    #[test]fn minimums_and_cancel_are_preserved(){
        let mut p=Panes::default();p.fit(640.);assert_eq!(p.editor,300.);assert!(p.workspace>=150.);
        p.begin(Divider::Workspace,150.);p.update(-10000.,640.);assert!(p.cancel());assert!(p.workspace>=150.);
        p.begin(Divider::Editor,0.);p.update(10000.,640.);p.commit();assert_eq!(p.editor,300.);
        p.begin(Divider::Editor,0.);p.update(-10000.,1440.);p.commit();assert!(1440.-p.workspace-p.editor-12.>=150.);
    }
}
