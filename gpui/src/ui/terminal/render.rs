//! Fixed-grid row rendering, shared baselines and connected one-pixel TUI borders.
use super::{Grid,Row,Run,color};
use super::metrics::{Metrics,cell_font,snap_line};
use gpui::*;

fn connections(text:&str)->Option<u8>{Some(match text{"─"=>10,"│"=>5,"┌"=>6,"┐"=>12,"└"=>3,"┘"=>9,"├"=>7,"┤"=>13,"┬"=>14,"┴"=>11,"┼"=>15,_=>return None})}
fn border(bits:u8,origin:Point<Pixels>,m:Metrics,color:Rgba,window:&mut Window){
    let w=m.size.width;let h=m.size.height;let x=px((f32::from(w)*m.scale/2.).floor()/m.scale);let y=px((f32::from(h)*m.scale/2.).floor()/m.scale);let stroke=m.stroke;
    if bits&1!=0{window.paint_quad(fill(Bounds::new(origin+point(x,px(0.)),size(stroke,y+stroke)),color));}
    if bits&2!=0{window.paint_quad(fill(Bounds::new(origin+point(x,y),size(w-x,stroke)),color));}
    if bits&4!=0{window.paint_quad(fill(Bounds::new(origin+point(x,y),size(stroke,h-y)),color));}
    if bits&8!=0{window.paint_quad(fill(Bounds::new(origin+point(px(0.),y),size(x+stroke,stroke)),color));}
}
fn selected_columns(g:&Grid,row:u16,cols:u16)->Option<(u16,u16)>{
    let(a,b)=g.selection()?;if row<a.0||row>b.0{return None;}
    let left=if row==a.0{a.1}else{0};let right=if row==b.0{b.1+if g.parser.screen().cell(b.0,b.1).is_some_and(|c|c.is_wide()){2}else{1}}else{cols};Some((left.min(cols),right.min(cols)))
}
fn selected(range:Option<(u16,u16)>,col:usize)->bool{range.is_some_and(|(a,b)|col>=a as usize&&col<b as usize)}
pub(super) fn paint(g:&mut Grid,bounds:Bounds<Pixels>,focused:bool,window:&mut Window,cx:&mut App){
    let(rows,cols)=g.parser.screen().size();let origin=bounds.origin+point(px(8.),px(8.));let fs=px(g.font_size);let p=g.palette;let m=g.metrics;
    g.rows.truncate(rows as usize);
    for row in 0..rows{
        let cells=(0..cols).filter_map(|col|g.parser.screen().cell(row,col).cloned()).collect::<Vec<_>>();let selection=selected_columns(g,row,cols);
        let changed=g.rows.get(row as usize).is_none_or(|r|r.cells!=cells||r.selection!=selection);
        if changed{
            let mut runs=vec![];let mut col=0usize;
            while col<cells.len(){
                let c=&cells[col];if c.is_wide_continuation()||connections(c.contents()).is_some(){col+=1;continue;}
                let start=col;let mut text=if c.contents().is_empty(){" ".to_string()}else{c.contents().to_string()};let ascii=text.is_ascii();
                let fg=if selected(selection,col){gpui::rgb(p.foreground)}else if c.inverse(){color(c.bgcolor(),p.background,&p)}else{color(c.fgcolor(),p.foreground,&p)};
                if ascii{while col+1<cells.len(){let next=&cells[col+1];if next.is_wide()||next.is_wide_continuation()||!next.contents().is_ascii()||next.fgcolor()!=c.fgcolor()||next.bgcolor()!=c.bgcolor()||next.bold()!=c.bold()||next.italic()!=c.italic()||next.inverse()!=c.inverse()||next.dim()!=c.dim()||selected(selection,col+1)!=selected(selection,start){break;}col+=1;text.push_str(if next.contents().is_empty(){" "}else{next.contents()});}}
                let span=if c.is_wide(){2}else{col-start+1};let width=m.size.width*span as f32;
                let f=cell_font(c.bold(),c.italic());let make=|_size|TextRun{len:text.len(),font:f.clone(),color:Hsla::from(fg).opacity(if c.dim()&&!selected(selection,start){0.65}else{1.}),..Default::default()};
                let mut line=window.text_system().shape_line(text.clone().into(),fs,&[make(fs)],None);
                if !ascii&&line.width()>width+px(0.5){let fitted=fs*(f32::from(width)/f32::from(line.width())).clamp(0.55,1.);line=window.text_system().shape_line(text.clone().into(),fitted,&[make(fitted)],None);}
                let pad=if ascii{px(0.)}else{((width-line.width())/2.).max(px(0.))};
                let positions=if ascii{text.char_indices().map(|(i,_)|(i,i as u16)).collect::<Vec<_>>()}else{vec![(0,0)]};
                let line=snap_line(line,&positions,m.size.width,width);runs.push(Run{col:start as u16,line,width,pad});col+=1;
            }
            let data=Row{cells,runs,selection};if row as usize==g.rows.len(){g.rows.push(data);}else{g.rows[row as usize]=data;}
        }
        for (col,c) in g.rows[row as usize].cells.iter().enumerate(){
            let pos=origin+point(m.size.width*col as f32,m.size.height*row as f32);
            let bg=if c.inverse(){Some(color(c.fgcolor(),p.foreground,&p))}else if c.bgcolor()!=vt100::Color::Default{Some(color(c.bgcolor(),p.background,&p))}else{None};
            if let Some(bg)=bg{window.paint_quad(fill(Bounds::new(pos,m.size),bg));}
        }
        if let Some((left,right))=selection{window.paint_quad(fill(Bounds::new(origin+point(m.size.width*left as f32,m.size.height*row as f32),size(m.size.width*(right-left) as f32,m.size.height)),gpui::rgb(p.selection)));}
        for(col,c)in g.rows[row as usize].cells.iter().enumerate(){
            let pos=origin+point(m.size.width*col as f32,m.size.height*row as f32);let mut fg=if selected(selection,col){gpui::rgb(p.foreground)}else if c.inverse(){color(c.bgcolor(),p.background,&p)}else{color(c.fgcolor(),p.foreground,&p)};if c.dim()&&!selected(selection,col){fg.a*=0.65;}
            if let Some(bits)=connections(c.contents()){border(bits,pos,m,fg,window);}
            if c.underline(){window.paint_quad(fill(Bounds::new(pos+point(px(0.),m.underline),size(m.size.width,m.stroke)),fg));}
        }
        for run in &g.rows[row as usize].runs{
            let pos=origin+point(m.size.width*run.col as f32,m.size.height*row as f32);let clip=Bounds::new(pos,size(run.width,m.size.height));let at=m.text_origin(&run.line,pos)+point(run.pad,px(0.));
            window.with_content_mask(Some(ContentMask{bounds:clip}),|window|{let _=run.line.paint(at,m.size.height,TextAlign::Left,None,window,cx);});
        }
    }
    if !g.parser.screen().hide_cursor()&&g.parser.screen().scrollback()==0&&g.session.as_ref().is_some_and(|s|s.running()){
        let(r,mut c)=g.parser.screen().cursor_position();if r<rows&&c<cols{
            if g.parser.screen().cell(r,c).is_some_and(|c|c.is_wide_continuation())&&c>0{c-=1;}
            let cell=g.parser.screen().cell(r,c);let width=m.size.width*if cell.is_some_and(|c|c.is_wide()){2.}else{1.};let pos=origin+point(m.size.width*c as f32,m.size.height*r as f32);let cursor=gpui::rgb(p.cursor);
            if !focused{window.paint_quad(outline(Bounds::new(pos,size(width,m.size.height)),cursor,BorderStyle::Solid));}
            else{match g.parser.callbacks().cursor{
                crate::terminal::CursorShape::Bar=>window.paint_quad(fill(Bounds::new(pos,size((m.stroke*2.).max(px(1.)),m.size.height)),cursor)),
                crate::terminal::CursorShape::Underline=>window.paint_quad(fill(Bounds::new(pos+point(px(0.),m.size.height-m.stroke*2.),size(width,m.stroke*2.)),cursor)),
                crate::terminal::CursorShape::Block=>{
                    window.paint_quad(fill(Bounds::new(pos,size(width,m.size.height)),cursor));
                    if let Some(cell)=cell.filter(|c|c.has_contents()){
                        let f=cell_font(cell.bold(),cell.italic());let text=cell.contents();let run=TextRun{len:text.len(),font:f,color:gpui::rgb(p.background).into(),..Default::default()};let line=window.text_system().shape_line(text.to_string().into(),fs,&[run],None);
                        let line=snap_line(line,&[(0,0)],m.size.width,width);let at=m.text_origin(&line,pos);let _=line.paint(at,m.size.height,TextAlign::Left,None,window,cx);
                    }
                }
            }}
        }
    }
    if let Some((top,height,_))=g.thumb(){window.paint_quad(fill(Bounds::new(point(bounds.right()-px(6.),bounds.top()+px(top)),size(px(3.),px(height))),Hsla::from(gpui::rgb(p.cursor)).opacity(0.45)));}
    if focused&&!g.marked.is_empty(){
        let(r,c)=g.parser.screen().cursor_position();let run=TextRun{len:g.marked.len(),font:cell_font(false,false),color:gpui::rgb(p.foreground).into(),..Default::default()};let line=window.text_system().shape_line(g.marked.clone().into(),fs,&[run],None);let pos=origin+point(m.size.width*c as f32,m.size.height*r as f32);window.paint_quad(fill(Bounds::new(pos,size(line.width,m.size.height)),gpui::rgb(p.background)));let _=line.paint(m.text_origin(&line,pos),m.size.height,TextAlign::Left,None,window,cx);window.paint_quad(fill(Bounds::new(pos+point(px(0.),m.underline),size(line.width,m.stroke)),gpui::rgb(p.cursor)));
    }
}
#[cfg(test)]mod tests{
    use super::connections;
    #[test]fn box_connections_meet_at_cell_boundaries(){assert_eq!(connections("┌"),Some(6));assert_eq!(connections("─"),Some(10));assert_eq!(connections("┼"),Some(15));assert_eq!(connections("A"),None);}
}
