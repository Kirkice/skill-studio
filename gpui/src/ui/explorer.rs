use crate::Studio;
use super::{controls::*,icons::{icon,Icon},theme::*,theme::rgb};
use gpui::*;
use gpui::prelude::FluentBuilder;

impl Studio {
    pub(super) fn sidebar(&self,cx:&mut Context<Self>)->Div{
        let name=self.graph.as_ref().and_then(|g|std::path::Path::new(&g.root).file_name()).map(|s|s.to_string_lossy().into_owned()).unwrap_or_else(||"Local workspace".into());
        let header=div().px(px(14.)).pt(px(12.)).pb(px(10.)).flex().flex_col().gap(px(10.))
            .child(div().h(px(24.)).flex().items_center().justify_between().child(caption("EXPLORER"))
                .child(icon_button("sidebar-open",Icon::FolderOpen,"Open workspace",false,true).w(px(24.)).h(px(24.)).on_click(cx.listener(|this,_,w,cx|this.open(w,cx)))))
            .child(div().flex().items_center().gap(px(8.)).child(icon(Icon::FolderOpen).size(px(15.)).text_color(rgb(ACCENT)))
                .child(div().flex_1().min_w(px(0.)).truncate().font_weight(FontWeight::SEMIBOLD).text_size(px(13.)).child(name)))
            .child(input("file-search",&self.search));
        let actions=div().h(px(32.)).px(px(12.)).flex_none().flex().items_center().gap(px(7.))
            .child(caption("FILES"))
            .child(div().flex_1().text_size(px(10.)).text_color(rgb(DIM)).child(format!("{}",self.explorer.matched_files)))
            .child(icon_button("explorer-reveal",Icon::Focus,"Reveal active file",false,self.selected.is_some()).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,_,cx|{if let Some(id)=&this.selected{this.explorer.reveal(id);cx.notify();}})))
            .child(icon_button("explorer-collapse",Icon::CollapseAll,"Collapse all folders",false,self.graph.is_some()).w(px(23.)).h(px(23.)).on_click(cx.listener(|this,_,_,cx|{this.explorer.collapse_all();cx.notify();})));
        let mut list=div().id("file-list").flex_1().min_h(px(0.)).overflow_y_scroll().pb(px(8.));
        for &index in &self.explorer.rows{
            let entry=&self.explorer.entries[index];
            let path=entry.path.clone();let title=entry.title.clone();
            let selected=!entry.directory&&self.selected.as_deref()==Some(&entry.path);
            let expanded=entry.directory&&self.explorer.expanded(&entry.path);
            let glyph=if entry.directory{if expanded{Icon::FolderOpen}else{Icon::Folder}}else if entry.kind=="skill"{Icon::Skill}else if entry.kind=="script"||matches!(entry.name.rsplit('.').next(),Some("json"|"toml"|"yaml"|"yml"|"css"|"html")){Icon::Code}else{Icon::File};
            let color=if entry.directory{MUTED}else if entry.kind=="skill"{ACCENT}else if matches!(glyph,Icon::Code){WARNING}else if entry.name.ends_with(".md"){0x89aacd}else{DIM};
            let hint:SharedString=if entry.directory{format!("{} · {} files\n{}",if expanded{"Collapse folder"}else{"Expand folder"},entry.files,path).into()}else if title.is_empty(){path.clone().into()}else{format!("{path}\n{title}").into()};
            let mut row=div().id(("explorer-entry",index)).relative().h(px(30.)).mx(px(6.)).mb(px(1.)).pr(px(8.)).pl(px(8.+entry.depth as f32*16.)).rounded(px(4.)).flex().items_center().gap(px(5.)).overflow_hidden()
                .aria_label(hint.clone()).bg(rgb(if selected{SELECTED}else{PANEL})).hover(|s|s.bg(rgb(if selected{SELECTED}else{HOVER})))
                .tooltip(move|_,cx|cx.new(|_|Hint(hint.clone())).into());
            for level in 0..entry.depth{
                row=row.child(div().absolute().left(px(13.+level as f32*16.)).top(px(0.)).bottom(px(0.)).w(px(1.)).bg(rgb(BORDER)));
            }
            row=row.children(selected.then(||div().absolute().left(px(0.)).top(px(5.)).bottom(px(5.)).w(px(2.)).rounded_full().bg(rgb(ACCENT))))
                .child(if entry.directory{icon(if expanded{Icon::ChevronDown}else{Icon::ChevronRight}).size(px(12.)).text_color(rgb(DIM)).into_any_element()}else{div().w(px(12.)).flex_none().into_any_element()})
                .child(icon(glyph).size(px(15.)).flex_none().text_color(rgb(color)))
                .child(div().flex_1().min_w(px(0.)).truncate().text_size(px(12.5)).text_color(rgb(if selected{TEXT}else if entry.directory{MUTED}else{TEXT})).when(entry.kind=="skill",|s|s.font_weight(FontWeight::MEDIUM)).child(entry.name.clone()));
            if entry.directory{
                row=row.child(div().flex_none().text_size(px(10.)).text_color(rgb(DIM)).child(format!("{}",entry.files)))
                    .on_click(cx.listener(move|this,event:&ClickEvent,_,cx|{if event.click_count()==1{this.explorer.toggle(&path);cx.notify();}}));
            }else{
                row=row.children(self.documents.get(&path).is_some_and(|tab|tab.dirty).then(||div().flex_none().w(px(5.)).h(px(5.)).rounded_full().bg(rgb(WARNING))))
                    .on_click(cx.listener(move|this,event:&ClickEvent,w,cx|this.select_file(path.clone(),event.click_count()>1,w,cx)));
            }
            list=list.child(row);
        }
        if self.explorer.rows.is_empty(){list=list.child(div().p(px(18.)).text_size(px(12.)).text_color(rgb(DIM)).child(if self.graph.is_some(){"No matching files"}else{"Open a folder to explore its files and Skills."}));}
        div().size_full().min_w(px(0.)).overflow_hidden().bg(rgb(PANEL)).flex().flex_col().child(header).child(actions).child(list)
            .child(div().h(px(38.)).px(px(12.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between()
                .child(div().text_size(px(10.)).text_color(rgb(DIM)).child("LOCAL WORKSPACE"))
                .child(icon_button("new-skill",Icon::Plus,"New Skill",false,self.graph.is_some()).w(px(24.)).h(px(24.)).on_click(cx.listener(|this,_,w,cx|this.command(crate::commands::Command::Create,w,cx)))))
    }
}
