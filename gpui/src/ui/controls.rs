use super::theme::*;
use super::icons::{icon,Icon};
use gpui::*;
use super::theme::rgb;
use gpui_base::{Input, InputBase};
use gpui_base::input::InputState;

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, active: bool, enabled: bool) -> Stateful<Div> {
    let label=label.into();
    let bg = if active { SELECTED } else { PANEL };
    div().id(id).h(px(28.)).flex_none().px(px(10.)).flex().items_center().justify_center().gap(px(6.)).rounded(px(5.))
        .text_size(px(UI_SIZE)).line_height(px(20.)).bg(rgb(bg)).text_color(rgb(if !enabled { 0x545963 } else if active { TEXT } else { MUTED }))
        .cursor(if enabled { CursorStyle::PointingHand } else { CursorStyle::Arrow })
        .hover(move |style| style.bg(rgb(if enabled { HOVER } else { bg })))
        .children((!label.is_empty()).then_some(label))
}
pub(super) struct Hint(pub SharedString);
impl Render for Hint {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
        div().max_w(px(480.)).px(px(9.)).py(px(5.)).rounded(px(5.)).bg(rgb(POPOVER)).border_1().border_color(rgb(BORDER)).shadow_sm().text_color(rgb(TEXT)).text_size(px(12.)).font_family(UI_FONT).child(self.0.clone())
    }
}
pub fn icon_button(id:impl Into<ElementId>,glyph:Icon,hint:impl Into<SharedString>,active:bool,enabled:bool)->Stateful<Div> {
    let hint=hint.into();
    button(id,"",active,enabled).aria_label(hint.clone()).group("icon-button").w(px(28.)).px(px(0.)).gap(px(0.)).child(icon(glyph).size(px(15.)).text_color(rgb(if !enabled{DIM}else if active{TEXT}else{MUTED})).group_hover("icon-button",move|style|style.text_color(rgb(if enabled{TEXT}else{DIM}))))
        .tooltip(move|_,cx|cx.new(|_|Hint(hint.clone())).into())
}
pub fn swatch(id:impl Into<ElementId>,color:u32,hint:&'static str,selected:bool)->Stateful<Div> {
    button(id,"",false,true).w(px(38.)).h(px(32.)).px(px(0.)).gap(px(0.)).border_1().border_color(rgb(if selected{TEXT}else{BORDER})).bg(gpui::rgb(color))
        .children(selected.then(||icon(Icon::Check).text_color(gpui::rgb(0x17191d))))
        .tooltip(move|_,cx|cx.new(|_|Hint(hint.into())).into())
}
pub fn icon_label(id:impl Into<ElementId>,glyph:Icon,label:impl Into<SharedString>,active:bool,enabled:bool)->Stateful<Div> {
    button(id,"",active,enabled).child(icon(glyph).size(px(14.)).text_color(rgb(if !enabled{DIM}else if active{TEXT}else{MUTED}))).child(label.into())
}
pub fn primary_icon(id:impl Into<ElementId>,glyph:Icon,hint:impl Into<SharedString>)->Stateful<Div>{
    let hint=hint.into();
    button(id,"",false,true).aria_label(hint.clone()).w(px(34.)).h(px(32.)).px(px(0.)).bg(rgb(ACCENT)).hover(|s|s.bg(rgb(0xb0cbef)))
        .child(icon(glyph).size(px(17.)).text_color(rgb(BG)))
        .tooltip(move|_,cx|cx.new(|_|Hint(hint.clone())).into())
}
pub fn input(id: &'static str, state: &Entity<InputState>) -> InputBase {
    let focus = state.clone();
    InputBase::new(id).w_full().h(px(34.)).px(px(10.)).bg(rgb(RAISED)).rounded(px(6.)).border_1().border_color(rgb(BORDER))
        .on_mouse_down(MouseButton::Left,move |_,window,cx|focus.update(cx,|state,cx|state.focus(window,cx)))
        .child(div().size_full().flex().items_center().gap(px(7.)).child(icon(if id=="new-skill-name"{Icon::Skill}else{Icon::Search}).size(px(14.)).text_color(rgb(MUTED))).child(div().flex_1().min_w(px(0.)).child(Input::new(state))))
}
pub fn caption(label: impl Into<SharedString>) -> Div { div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(DIM)).child(label.into()) }
pub fn separator() -> Div { div().w(px(1.)).h(px(16.)).flex_none().bg(rgb(BORDER)).mx(px(3.)) }
pub fn bar() -> Div { div().h(px(42.)).flex_none().px(px(12.)).flex().items_center().gap(px(6.)).border_b_1().border_color(rgb(BORDER)) }
