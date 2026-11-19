use crate::{Studio,panes::Divider};
use super::{controls::*,icons::{app_logo,Icon},theme::*,theme::rgb};
use gpui::*;

// Capture at the window level: graph/editor hitboxes can occlude ancestor divs.
// This also completes a drag released outside the original splitter or viewport.
pub(super) fn resize_events(owner:WeakEntity<Studio>)->impl IntoElement{
    canvas(|_,_,_|(),move|_,_,window,_|{
        let moved=owner.clone();
        window.on_mouse_event(move|event:&MouseMoveEvent,phase,window,cx|{
            if phase!=DispatchPhase::Capture{return;}
            let handled=moved.update(cx,|this,cx|{
                if !this.panes.dragging(){return false;}
                if this.panes.update(f32::from(event.position.x),f32::from(window.viewport_size().width)){cx.notify();}true
            }).unwrap_or(false);
            if handled{window.prevent_default();cx.stop_propagation();}
        });
        let released=owner.clone();
        window.on_mouse_event(move|event:&MouseUpEvent,phase,window,cx|{
            if phase!=DispatchPhase::Capture||event.button!=MouseButton::Left{return;}
            let handled=released.update(cx,|this,cx|{
                if !this.panes.dragging(){return false;}
                this.panes.update(f32::from(event.position.x),f32::from(window.viewport_size().width));
                this.panes.commit();cx.notify();true
            }).unwrap_or(false);
            if handled{window.prevent_default();cx.stop_propagation();}
        });
    }).absolute().top(px(0.)).left(px(0.)).size_full()
}

// GPUI 0.3.8 zoom() always maximizes on Windows; restore needs SW_RESTORE.
fn toggle_maximize(window:&Window){
    #[cfg(target_os="windows")]{
        use raw_window_handle::{HasWindowHandle,RawWindowHandle};
        #[link(name="user32")] unsafe extern "system"{fn ShowWindowAsync(hwnd:*mut std::ffi::c_void,command:i32)->i32;}
        if let Ok(handle)=HasWindowHandle::window_handle(window){if let RawWindowHandle::Win32(handle)=handle.as_raw(){
            // The handle belongs to this live GPUI window; ShowWindowAsync does not retain Rust memory.
            unsafe{ShowWindowAsync(handle.hwnd.get() as *mut _,if window.is_maximized(){9}else{3});}return;
        }}
    }
    window.zoom_window();
}

impl Studio {
    pub(super) fn titlebar(&self,root:String,window:&Window,cx:&mut Context<Self>)->Div{
        let drag=div().id("window-drag").flex_1().min_w(px(0.)).h_full().flex().items_center().gap(px(10.)).window_control_area(WindowControlArea::Drag)
            .child(app_logo().size(px(26.))).child(div().font_weight(FontWeight::SEMIBOLD).child("skill studio"))
            .child(separator()).child(div().flex_1().min_w(px(0.)).truncate().text_color(rgb(DIM)).text_size(px(12.)).child(root))
            .on_mouse_down(MouseButton::Left,|_,_window,_|{
                #[cfg(not(target_os="windows"))] _window.start_window_move();
            })
            .on_click(|_event,_window,_|{
                #[cfg(not(target_os="windows"))] if _event.click_count()>1{_window.zoom_window();}
            });
        div().h(px(44.)).flex_none().pl(px(12.)).flex().items_center().gap(px(4.)).border_b_1().border_color(rgb(BORDER)).bg(rgb(PANEL))
            .child(drag)
            .child(icon_button("command-trigger",Icon::Command,"Command palette · Ctrl/Cmd+Shift+P",self.palette,true).on_click(cx.listener(|this,_,w,cx|this.toggle_palette(w,cx))))
            .child(icon_button("settings-trigger",Icon::Settings,"Settings",self.settings_open,true).on_click(cx.listener(|this,_,w,cx|this.open_settings(w,cx))))
            .child(separator())
            .child(icon_button("window-minimize",Icon::Minus,"Minimize window",false,true).w(px(38.)).h_full().rounded(px(0.)).on_click(|_,w,_|w.minimize_window()))
            .child(icon_button("window-maximize",if window.is_maximized(){Icon::Restore}else{Icon::Maximize},if window.is_maximized(){"Restore window"}else{"Maximize window"},false,true).w(px(38.)).h_full().rounded(px(0.)).on_click(|_,w,_|toggle_maximize(w)))
            .child(icon_button("window-close",Icon::Close,"Close window · unsaved tabs are protected",false,true).w(px(42.)).h_full().rounded(px(0.)).hover(|s|s.bg(rgb(0xc7444e)))
                .on_click(cx.listener(|this,_,w,cx|{if this.guard(cx){w.remove_window();}})))
    }
    pub(super) fn pane_handle(&self,divider:Divider,id:&'static str,cx:&mut Context<Self>)->Stateful<Div>{
        div().id(id).w(px(crate::panes::Panes::HANDLE)).h_full().flex_none().cursor_col_resize().bg(rgb(BORDER)).hover(|s|s.bg(rgb(ACCENT)))
            .on_mouse_down(MouseButton::Left,cx.listener(move|this,event:&MouseDownEvent,_,cx|{
                this.panes.begin(divider,f32::from(event.position.x));cx.stop_propagation();cx.notify();
            }))
    }
}
