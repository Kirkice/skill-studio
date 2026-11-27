use super::{controls::*,icons::{icon,Icon},theme::*};
use super::theme::rgb;
use crate::{Studio,settings};
use gpui::*;

impl Studio {
 pub(super) fn settings_panel(&self,cx:&mut Context<Self>)->Div {
  let mut themes=div().flex().gap(px(8.));
  for (i,name) in settings::THEMES.iter().copied().enumerate(){
   let active=self.preferences.theme==name;
   let preview=super::theme::preview(name);
   themes=themes.child(div().id(("theme",i)).flex_1().min_w(px(0.)).p(px(7.)).rounded(px(7.)).border_1().border_color(rgb(if active{ACCENT}else{BORDER})).cursor(CursorStyle::Arrow).hover(|s|s.border_color(rgb(ACCENT)))
    .child(div().h(px(48.)).flex().gap(px(3.)).p(px(5.)).rounded(px(4.)).bg(gpui::rgb(preview[0])).child(div().w(px(20.)).bg(gpui::rgb(preview[1]))).child(div().flex_1().flex().flex_col().gap(px(4.)).child(div().h(px(6.)).bg(gpui::rgb(preview[2]))).child(div().flex_1().bg(gpui::rgb(preview[1])))))
    .child(div().pt(px(7.)).flex().items_center().justify_between().child(name).children(active.then(||icon(Icon::Check).size(px(13.)).text_color(rgb(ACCENT)))))
    .on_click(cx.listener(move|this,_,w,cx|{this.preferences.theme=name.into();this.apply_preferences(w,cx);})))
  }
  let mut accents=div().flex().gap(px(10.));
  for (i,(name,color)) in settings::ACCENTS.iter().copied().enumerate(){
   let selected=self.preferences.accent==color;
   accents=accents.child(swatch(("accent",i),color,name,selected)
    .on_click(cx.listener(move|this,_,w,cx|{this.preferences.accent=color;this.apply_preferences(w,cx);})))
  }
  div().flex().flex_col()
   .child(bar().h(px(50.)).child(icon(Icon::Settings).text_color(rgb(ACCENT))).child(div().flex_1().font_weight(FontWeight::SEMIBOLD).text_size(px(16.)).child(crate::i18n::text("Settings")))
    .child(icon_button("close-settings",Icon::Close,"Close settings",false,true).on_click(cx.listener(|this,_,w,cx|this.close_overlay(w,cx)))))
   .child(div().id("settings-scroll").max_h(px(500.)).overflow_y_scroll().p(px(20.)).flex().flex_col().gap(px(14.))
    .child(caption(crate::i18n::text("APPEARANCE"))).child(crate::i18n::text("Theme")).child(themes).child(crate::i18n::text("Accent color")).child(accents)
    .child(div().h(px(1.)).bg(rgb(BORDER)).my(px(4.)))
    .child(caption(crate::i18n::text("EDITOR")))
    .child(div().text_size(px(12.5)).text_color(rgb(MUTED)).child("Geist · UI 13 px    /    JetBrains Mono · editor"))
    .child(div().flex().items_center().justify_between().child(crate::i18n::text("Font size")).child(div().flex().items_center().gap(px(8.))
     .child(icon_button("font-smaller",Icon::Minus,"Decrease editor font size",false,self.preferences.font_size>10).on_click(cx.listener(|this,_,w,cx|{this.preferences.font_size=this.preferences.font_size.saturating_sub(1).max(10);this.apply_preferences(w,cx);})))
     .child(format!("{} px",self.preferences.font_size))
     .child(icon_button("font-larger",Icon::Plus,"Increase editor font size",false,self.preferences.font_size<24).on_click(cx.listener(|this,_,w,cx|{this.preferences.font_size=(this.preferences.font_size+1).min(24);this.apply_preferences(w,cx);})))))
    .child(div().flex().items_center().justify_between().child(crate::i18n::text("Word wrap")).child(icon_button("settings-wrap",if self.preferences.soft_wrap{Icon::ToggleOn}else{Icon::ToggleOff},if self.preferences.soft_wrap{"Word wrap enabled — click to disable"}else{"Word wrap disabled — click to enable"},self.preferences.soft_wrap,true).w(px(38.)).on_click(cx.listener(|this,_,w,cx|{this.preferences.soft_wrap=!this.preferences.soft_wrap;this.apply_preferences(w,cx);}))))
    .child(div().flex().items_center().justify_between().child(crate::i18n::text("Line numbers")).child(icon_button("settings-numbers",if self.preferences.line_numbers{Icon::ToggleOn}else{Icon::ToggleOff},if self.preferences.line_numbers{"Line numbers enabled — click to disable"}else{"Line numbers disabled — click to enable"},self.preferences.line_numbers,true).w(px(38.)).on_click(cx.listener(|this,_,w,cx|{this.preferences.line_numbers=!this.preferences.line_numbers;this.apply_preferences(w,cx);}))))
    .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(crate::i18n::text("Changes apply immediately and are saved on this device.")))
    .child(div().h(px(1.)).bg(rgb(BORDER)).my(px(4.)))
    .child(caption(crate::i18n::text("PI TRACE")))
    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(if self.preferences.pi_launcher.is_empty(){"Launcher: pi (PATH)".to_string()}else{format!("Launcher: {}",self.preferences.pi_launcher)}))
    .child(div().text_size(px(11.)).text_color(rgb(DIM)).child("Set SKILLRIDER_PI_LAUNCHER to override this at startup, e.g. H:\\tmp\\pi\\pi-test.ps1")))
   .child(div().px(px(20.)).py(px(12.)).border_t_1().border_color(rgb(BORDER)).flex().items_center().justify_between()
    .child(icon_button("reset-settings",Icon::Undo,"Restore default settings",false,true).on_click(cx.listener(|this,_,w,cx|{this.preferences=settings::Settings::default();this.apply_preferences(w,cx);})))
    .child(caption(crate::i18n::text("No workspace files are modified"))))
 }
}
