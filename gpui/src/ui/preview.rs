use crate::{Studio,preview};
use super::{theme::*,theme::rgb};
use gpui::*;
use gpui_base::{TextView,TextViewStyle};
use std::{path::Path,sync::{Arc,OnceLock}};

fn unavailable_image()->ImageSource{
    static IMAGE:OnceLock<Arc<Image>>=OnceLock::new();
    ImageSource::Image(IMAGE.get_or_init(||Arc::new(Image::from_bytes(ImageFormat::Svg,
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="48"><rect width="240" height="48" rx="6" fill="#282c34"/><text x="12" y="29" fill="#9ba5b6" font-size="12">Image unavailable / remote blocked</text></svg>"##.to_vec()))).clone())
}
impl Studio {
    pub(super) fn is_preview(&self)->bool{
        self.selected.as_ref().and_then(|id|self.documents.get(id)).is_some_and(|tab|tab.preview)
    }
    pub(super) fn toggle_preview(&mut self,window:&mut Window,cx:&mut Context<Self>){
        let Some(id)=self.selected.clone().filter(|id|preview::supported(id))else{return;};
        if !self.composition_ready(window,cx){return;}
        let Some(tab)=self.documents.get_mut(&id)else{return;};tab.set_preview(!tab.preview);
        if tab.preview{
            self.find_open=false;self.editor.update(cx,|editor,cx|editor.close_search(cx));self.focus_handle.focus(window,cx);
            self.status="Markdown preview · showing current buffer, including unsaved edits".into();
        }else{self.editor.update(cx,|editor,cx|editor.focus(window,cx));self.status="Source editor".into();}
        cx.notify();
    }
    fn preview_link(&mut self,source:&str,url:&str,window:&mut Window,cx:&mut Context<Self>){
        let lower=url.to_ascii_lowercase();
        if lower.starts_with("https://")||lower.starts_with("http://"){cx.open_url(url);return;}
        if url.starts_with('#'){
            self.status="Heading links are not yet supported in preview".into();cx.notify();return;
        }
        let Some(root)=self.graph.as_ref().map(|g|g.root.clone())else{return;};
        match preview::resource(Path::new(&root),source,url){
            Ok(path)=>{
                let root=Path::new(&root).canonicalize().unwrap_or_else(|_|Path::new(&root).into());
                let Ok(relative)=path.strip_prefix(root)else{return;};let id=relative.to_string_lossy().replace('\\',"/");
                self.documents.pin(source);self.select_file(id.clone(),true,window,cx);
                if self.selected.as_deref()==Some(id.as_str())&&preview::supported(&id){
                    if let Some(tab)=self.documents.get_mut(&id){tab.set_preview(true);}
                    self.find_open=false;self.focus_handle.focus(window,cx);
                }
                cx.notify();
            }
            Err(e)=>{self.status=format!("Cannot open preview link: {e}");cx.notify();}
        }
    }
    pub(super) fn markdown_preview(&self,cx:&mut Context<Self>)->Div{
        let Some(id)=self.selected.as_ref()else{return div();};
        let Some(tab)=self.documents.get(id)else{return div();};
        let root=self.graph.as_ref().map(|g|g.root.clone()).unwrap_or_default();let image_root=root.clone();let image_document=id.clone();
        let source=id.clone();let owner=cx.entity().downgrade();
        let content=tab.preview_body.clone().unwrap_or_else(||tab.buffer.clone());
        let mut code_style=StyleRefinement::default().font_family(MONO_FONT).text_size(px(13.)).line_height(px(21.)).text_color(rgb(TEXT)).bg(rgb(RAISED)).p(px(12.)).rounded(px(7.));
        code_style.overflow.x=Some(Overflow::Scroll);
        let style=TextViewStyle::default()
            .with_foreground(rgb(TEXT).into()).with_muted_foreground(rgb(MUTED).into())
            .with_link(rgb(ACCENT).into()).with_code_background(rgb(RAISED).into())
            .with_border(rgb(BORDER).into()).with_selection(rgb(SELECTED).into())
            .with_inline_code(HighlightStyle{color:Some(rgb(ACCENT).into()),background_color:Some(rgb(RAISED).into()),..Default::default()})
            .with_code_block(code_style);
        let document=TextView::markdown(("rendered-markdown",tab.token),content).style(style).selectable(true).scrollable(false)
            .image_source(move|uri|{
                match preview::resource(Path::new(&image_root),&image_document,uri.as_ref()){
                    Ok(path)if std::fs::metadata(&path).is_ok_and(|m|m.len()<=10_000_000)=>ImageSource::from(path.as_path()),
                    _=>unavailable_image(),
                }
            })
            .on_link_click(move|url,event,window,cx|{
                if event.is_right_click(){return;}
                let _=owner.update(cx,|studio,cx|studio.preview_link(&source,url.as_ref(),window,cx));
            });
        div().flex_1().min_h(px(0.)).min_w(px(0.)).flex().flex_col().bg(rgb(BG))
            .child(div().px(px(20.)).py(px(7.)).border_b_1().border_color(rgb(BORDER)).text_size(px(10.)).text_color(rgb(MUTED))
                .child(if self.dirty(cx){"PREVIEW · Unsaved buffer"}else{"PREVIEW · Markdown document"}))
            .child(div().id(("markdown-scroll",tab.token)).flex_1().min_h(px(0.)).overflow_y_scroll().track_scroll(&tab.preview_scroll).px(px(24.)).py(px(20.))
                .font_family(UI_FONT).text_size(px(15.)).line_height(px(25.))
                .child(div().w_full().max_w(px(940.)).mx_auto().child(document)))
    }
}
