//! Device-pixel terminal metrics from the bundled face, not an editor line-height guess.
use gpui::*;
use std::sync::{Arc,OnceLock};

#[derive(Clone,Copy,Debug,PartialEq)]
pub(super) struct Metrics{pub size:Size<Pixels>,pub baseline:Pixels,pub glyph_x:Pixels,pub underline:Pixels,pub stroke:Pixels,pub scale:f32}
#[derive(Clone,Copy)]struct Face{units:f32,ascent:f32,descent:f32,gap:f32,advance:f32,underline:f32}
fn face()->Face{
    static FACE:OnceLock<Face>=OnceLock::new();
    *FACE.get_or_init(||{
        let font=ttf_parser::Face::parse(include_bytes!("../../../assets/fonts/JetBrainsMono-Variable.ttf"),0).expect("bundled terminal font");
        let advance=(' '..='~').filter_map(|c|font.glyph_index(c).and_then(|id|font.glyph_hor_advance(id))).max().unwrap_or(600) as f32;
        Face{units:font.units_per_em() as f32,ascent:font.ascender() as f32,descent:font.descender() as f32,gap:font.line_gap() as f32,advance,underline:font.underline_metrics().map_or(-100.,|m|m.position as f32)}
    })
}
impl Metrics{
    pub fn new(font_size:f32,line_height:f32,scale:f32)->Self{
        let scale=scale.max(0.5);let f=face();
        let em=if cfg!(target_os="macos"){font_size*scale}else{(font_size*scale).round().max(1.)};
        let unit=em/f.units;let natural=(f.ascent-f.descent+f.gap)*unit;
        let face_height=natural.round().max(1.);let height=(face_height*line_height).round().max(1.);
        let baseline=(f.ascent*unit+f.gap*unit/2.+(height-natural)/2.).round();
        let advance=f.advance*unit;let width=advance.round().max(1.);
        let glyph_x=if cfg!(target_os="macos"){(width-advance)/2.}else{((width-advance)/2.).max(0.).round()};
        Self{size:size(px(width/scale),px(height/scale)),baseline:px(baseline/scale),glyph_x:px(glyph_x/scale),underline:px((baseline-f.underline*unit).round().min(height-1.)/scale),stroke:px(1./scale),scale}
    }
    pub fn text_origin(&self,line:&ShapedLine,origin:Point<Pixels>)->Point<Pixels>{
        let shaped_baseline=(self.size.height-line.ascent-line.descent)/2.+line.ascent;
        origin+point(self.glyph_x,self.baseline-shaped_baseline)
    }
}
pub(super) fn cell_font(bold:bool,italic:bool)->Font{
    let mut font=font(super::MONO_FONT);font.features=FontFeatures(Arc::new(vec![("calt".into(),0),("liga".into(),0),("clig".into(),0)]));
    if bold{font.weight=FontWeight::BOLD;}if italic{font.style=FontStyle::Italic;}font
}
/// Clone the cached layout before aligning glyph clusters; never mutate GPUI's shared shaping cache.
pub(super) fn snap_line(mut line:ShapedLine,positions:&[(usize,u16)],cell:Pixels,width:Pixels)->ShapedLine{
    if positions.is_empty(){return line;}
    let mut layout=LineLayout{font_size:line.font_size,width,ascent:line.ascent,descent:line.descent,runs:line.runs.clone(),len:line.len()};
    let mut origins=vec![None;positions.len()];
    for run in &layout.runs{for glyph in &run.glyphs{
        let index=positions.partition_point(|(byte,_)|*byte<=glyph.index).saturating_sub(1);
        origins[index].get_or_insert(glyph.position.x);
    }}
    for run in &mut layout.runs{for glyph in &mut run.glyphs{
        let index=positions.partition_point(|(byte,_)|*byte<=glyph.index).saturating_sub(1);
        glyph.position.x=cell*positions[index].1 as f32+glyph.position.x-origins[index].unwrap_or(px(0.));
    }}
    *std::ops::DerefMut::deref_mut(&mut line)=Arc::new(layout);line
}
#[cfg(test)]mod tests{
    use super::Metrics;
    #[test]fn face_metrics_are_compact_and_fit_the_baseline(){let m=Metrics::new(14.,1.,1.);assert!(f32::from(m.size.height)<14.*1.55);assert!(m.baseline>gpui::px(0.)&&m.baseline<m.size.height);assert!(m.underline<m.size.height);}
    #[test]fn metrics_land_on_device_pixels_at_fractional_dpi(){for scale in [1.,1.25,1.5,2.]{let m=Metrics::new(14.,1.,scale);for n in [f32::from(m.size.width),f32::from(m.size.height),f32::from(m.baseline)]{let physical=n*scale;assert!((physical-physical.round()).abs()<0.001);}}}
    #[test]fn snapping_does_not_mutate_shared_shaping_layout(){
        use gpui::*;use std::sync::Arc;
        let mut original=ShapedLine::default();
        *std::ops::DerefMut::deref_mut(&mut original)=Arc::new(LineLayout{font_size:px(14.),width:px(16.8),ascent:px(14.),descent:px(4.),len:2,runs:vec![ShapedRun{font_id:FontId(0),glyphs:vec![ShapedGlyph{id:GlyphId(1),position:point(px(0.),px(0.)),index:0,is_emoji:false},ShapedGlyph{id:GlyphId(2),position:point(px(8.4),px(0.)),index:1,is_emoji:false}]}]});
        let aligned=super::snap_line(original.clone(),&[(0,0),(1,1)],px(8.),px(16.));
        assert_eq!(aligned.runs[0].glyphs[1].position.x,px(8.));assert_eq!(original.runs[0].glyphs[1].position.x,px(8.4));assert_eq!(aligned.width,px(16.));
    }
    #[test]fn combining_mark_keeps_its_offset_from_the_base(){
        use gpui::*;use std::sync::Arc;
        let mut original=ShapedLine::default();
        *std::ops::DerefMut::deref_mut(&mut original)=Arc::new(LineLayout{font_size:px(14.),width:px(8.4),ascent:px(14.),descent:px(4.),len:3,runs:vec![ShapedRun{font_id:FontId(0),glyphs:vec![ShapedGlyph{id:GlyphId(1),position:point(px(0.),px(0.)),index:0,is_emoji:false},ShapedGlyph{id:GlyphId(2),position:point(px(-1.),px(0.)),index:1,is_emoji:false}]}]});
        let aligned=super::snap_line(original,&[(0,0)],px(8.),px(8.));assert_eq!(aligned.runs[0].glyphs[0].position.x,px(0.));assert_eq!(aligned.runs[0].glyphs[1].position.x,px(-1.));
    }
    #[test]fn line_height_changes_rows_not_character_width(){let a=Metrics::new(14.,1.,1.);let b=Metrics::new(14.,1.15,1.);assert_eq!(a.size.width,b.size.width);assert!(b.size.height>a.size.height);}
}
