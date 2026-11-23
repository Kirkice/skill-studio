use super::*;
impl TerminalView{
    pub(super) fn more_menu(&self,cx:&mut Context<Self>)->Option<Stateful<Div>>{
        if !self.menu{return None;}
        let active=self.active();let g=self.grid.borrow();let fs=g.font_size as u16;let compact=(g.line_height-1.).abs()<0.01;let app_scroll=g.force_app_scroll;let height=(f32::from(g.bounds.size.height)-4.).max(70.);drop(g);
        Some(div().id("terminal-more-menu").absolute().right(px(8.)).top(px(32.)).w(px(242.)).max_h(px(height)).overflow_y_scroll().p(px(6.)).rounded(px(8.)).border_1().border_color(rgb(BORDER)).bg(rgb(POPOVER)).shadow_lg().occlude().flex().flex_col().gap(px(2.))
            .child(div().px(px(8.)).py(px(4.)).truncate().text_size(px(10.)).text_color(rgb(DIM)).child(self.status.clone()))
            .child(icon_label("terminal-copy",Icon::Copy,"复制选中 / 当前屏幕",false,true).w_full().justify_start().bg(rgb(POPOVER)).on_click(cx.listener(|this,_,w,cx|this.copy(w,cx))))
            .child(icon_label("terminal-paste",Icon::Paste,"粘贴（多行先确认）",false,active&&!self.starting).w_full().justify_start().bg(rgb(POPOVER)).on_click(cx.listener(|this,_,w,cx|this.paste(w,cx))))
            .child(icon_label("terminal-live",Icon::Focus,"回到实时输出",false,true).w_full().justify_start().bg(rgb(POPOVER)).on_click(cx.listener(|this,_,w,cx|this.return_live(w,cx))))
            .child(icon_label("terminal-app-scroll",Icon::Mouse,"应用滚动 · ConPTY 兼容",app_scroll,true).w_full().justify_start().bg(rgb(if app_scroll{SELECTED}else{POPOVER})).on_click(cx.listener(|this,_,w,cx|{this.menu=false;let mut g=this.grid.borrow_mut();g.force_app_scroll=!g.force_app_scroll;g.wheel.reset();drop(g);this.focus.focus(w,cx);cx.notify();})))
            .child(div().h(px(1.)).my(px(4.)).bg(rgb(BORDER)))
            .child(div().h(px(30.)).px(px(8.)).flex().items_center().gap(px(5.)).child(div().flex_1().text_size(px(12.)).child("终端字号"))
                .child(icon_button("terminal-font-smaller",Icon::Minus,"减小终端字号（不影响文档）",false,fs>10).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|this.zoom(-1,cx))))
                .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(format!("{fs}")))
                .child(icon_button("terminal-font-larger",Icon::Plus,"增大终端字号（不影响文档）",false,fs<26).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|this.zoom(1,cx))))
                .child(icon_button("terminal-font-reset",Icon::Undo,"还原终端字号 14 px",false,true).w(px(22.)).h(px(22.)).on_click(cx.listener(|this,_,_,cx|{let mut g=this.grid.borrow_mut();g.font_size=14.;g.rows.clear();drop(g);cx.emit(TerminalEvent::FontSize(14));cx.notify();}))))
            .child(icon_label("terminal-compact-lines",Icon::Numbers,if compact{"紧凑行距 · 已开启"}else{"紧凑行距"},compact,true).w_full().justify_start().bg(rgb(if compact{SELECTED}else{POPOVER})).on_click(cx.listener(|this,_,_,cx|{let mut g=this.grid.borrow_mut();g.line_height=if (g.line_height-1.).abs()<0.01{1.15}else{1.};let height=g.line_height;g.rows.clear();drop(g);cx.emit(TerminalEvent::LineHeight(height));cx.notify();})))
            .child(div().h(px(1.)).my(px(4.)).bg(rgb(BORDER)))
            .child(icon_label("terminal-restart",Icon::Refresh,"启动 / 重新启动终端",false,!self.starting).w_full().justify_start().bg(rgb(POPOVER)).on_click(cx.listener(|this,_,w,cx|{this.menu=false;if !this.starting{this.ask_stop(true,w,cx);}cx.notify();})))
            .child(icon_label("terminal-stop",Icon::Stop,"停止终端（需确认）",false,active).w_full().justify_start().bg(rgb(POPOVER)).on_click(cx.listener(|this,_,w,cx|{this.menu=false;this.ask_stop(false,w,cx);cx.notify();})))
            .on_mouse_down_out(cx.listener(|this,_,_,cx|{this.menu=false;cx.notify();})))
    }
}
