use crate::{Studio,files::View};
use super::{controls::*,icons::{icon,Icon},theme::*,theme::rgb};
use gpui::*;
impl Studio{
    pub(super) fn non_text_panel(&self,cx:&mut Context<Self>)->Div{
        let Some(id)=&self.selected else{return div();};
        let Some(tab)=self.documents.get(id)else{return div();};
        let file=id.rsplit('/').next().unwrap_or(id).to_string();
        let mut body=div().flex_1().min_h(px(0.)).min_w(px(0.)).flex().flex_col();
        match &tab.view{
            View::Image{data,width,height,stamp}=>{
                body=body.child(div().flex_1().min_h(px(0.)).p(px(18.)).child(div().size_full().rounded(px(9.)).overflow_hidden().border_1().border_color(rgb(BORDER)).bg(rgb(RAISED))
                    .child(img(data.clone()).size_full().object_fit(ObjectFit::Contain))))
                    .child(div().h(px(34.)).flex_none().px(px(14.)).flex().items_center().justify_between().border_t_1().border_color(rgb(BORDER))
                        .child(caption(format!("{width} × {height} · {}",crate::files::size(stamp.bytes))))
                        .child(caption("Read-only · first frame")));
            }
            View::Unsupported{stamp,reason}=>{
                let ext=std::path::Path::new(id).extension().and_then(|s|s.to_str()).unwrap_or("FILE").to_ascii_uppercase();
                body=body.child(div().flex_1().min_h(px(0.)).p(px(24.)).flex().flex_col().items_center().justify_center().gap(px(14.))
                    .child(div().w(px(76.)).h(px(86.)).rounded(px(12.)).bg(rgb(RAISED)).border_1().border_color(rgb(BORDER)).flex().flex_col().items_center().justify_center().gap(px(7.))
                        .child(icon(Icon::File).size(px(30.)).text_color(rgb(ACCENT))).child(caption(ext)))
                    .child(div().max_w(px(380.)).truncate().font_weight(FontWeight::SEMIBOLD).text_size(px(15.)).child(file))
                    .child(caption(format!("{} · 只读",crate::files::size(stamp.bytes))))
                    .child(div().w_full().max_w(px(400.)).text_size(px(12.5)).line_height(px(21.)).text_color(rgb(MUTED)).child(reason.clone()))
                    .child(div().text_size(px(10.5)).text_color(rgb(DIM)).child("源文件未修改 · 未自动转码 · 不会自动执行")));
            }
            View::Text=>return body,
        }
        let path=self.graph.as_ref().map(|g|std::path::Path::new(&g.root).join(id));
        body.child(div().h(px(38.)).px(px(14.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between()
            .child(caption("FILE PREVIEW"))
            .child(icon_button("reveal-local-file",Icon::FolderOpen,"在文件夹中显示，不执行文件",false,path.is_some()).on_click(cx.listener(move|_,_,_,cx|{if let Some(path)=&path{cx.reveal_path(path);}}))))
    }
}
