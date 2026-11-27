use super::theme::rgb;
use super::{
    controls::*,
    icons::{Icon, icon},
    theme::*,
};
use crate::{
    Studio,
    diagnostics::{self, Severity},
};
use gpui::*;

impl Studio {
    pub(super) fn problems_panel(&self, cx: &mut Context<Self>) -> Div {
        let (errors, warnings, info) = diagnostics::counts(&self.diagnostics);
        let mut list = div()
            .id("problems-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .bg(rgb(BG));
        let visible = |severity| {
            self.problem_filters[match severity {
                Severity::Error => 0,
                Severity::Warning => 1,
                Severity::Info => 2,
            }]
        };
        if self.diagnostics.is_empty() {
            list = list.child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.))
                    .child(icon(Icon::Check).size(px(24.)).text_color(rgb(GREEN)))
                    .child(div().text_color(rgb(MUTED)).child(if self.graph.is_some() {
                        "No problems found"
                    } else {
                        "Open a workspace to run checks"
                    })),
            );
        } else if !self.diagnostics.iter().any(|d| visible(d.severity)) {
            list = list.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(MUTED))
                    .child("No problems match the active filters"),
            );
        } else {
            for (index, diagnostic) in self
                .diagnostics
                .iter()
                .enumerate()
                .filter(|(_, d)| visible(d.severity))
            {
                let color = match diagnostic.severity {
                    Severity::Error => 0xdc8c95,
                    Severity::Warning => WARNING,
                    Severity::Info => ACCENT,
                };
                let glyph = match diagnostic.severity {
                    Severity::Error => Icon::Close,
                    Severity::Warning => Icon::BrokenLink,
                    Severity::Info => Icon::File,
                };
                let location = if diagnostic.file.is_empty() {
                    "Workspace".into()
                } else {
                    diagnostic.location()
                };
                let file = diagnostic.file.clone();
                let line = diagnostic.line;
                list = list.child(
                    div()
                        .id(("problem-row", index))
                        .w_full()
                        .px(px(12.))
                        .py(px(8.))
                        .border_b_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .gap(px(9.))
                        .cursor(if file.is_empty() {
                            CursorStyle::Arrow
                        } else {
                            CursorStyle::PointingHand
                        })
                        .hover(|s| s.bg(rgb(HOVER)))
                        .child(icon(glyph).size(px(14.)).mt(px(2.)).text_color(rgb(color)))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(color))
                                                .child(diagnostic.code),
                                        )
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(11.))
                                                .text_color(rgb(DIM))
                                                .font_family(MONO_FONT)
                                                .child(location),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(TEXT))
                                        .child(diagnostic.message.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(MUTED))
                                        .child(diagnostic.detail.clone()),
                                ),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if !file.is_empty() {
                                this.reference(file.clone(), line.unwrap_or(1), window, cx);
                            }
                        })),
                );
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BG))
            .child(
                div()
                    .h(px(34.))
                    .flex_none()
                    .px(px(10.))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Problems"),
                    )
                    .children(
                        [
                            ("×", "Errors", errors, 0xdc8c95),
                            ("△", "Warnings", warnings, WARNING),
                            ("○", "Info", info, ACCENT),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (glyph, name, count, color))| {
                            let active = self.problem_filters[i];
                            let hint: SharedString = format!(
                                "{} {name} ({count})",
                                if active { "Hide" } else { "Show" }
                            )
                            .into();
                            button(
                                ("problem-filter", i),
                                format!("{glyph} {count}"),
                                active,
                                true,
                            )
                            .h(px(24.))
                            .px(px(6.))
                            .text_size(px(11.))
                            .text_color(rgb(if active { color } else { DIM }))
                            .aria_label(hint.clone())
                            .tooltip(move |_, cx| cx.new(|_| Hint(hint.clone())).into())
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.problem_filters[i] = !this.problem_filters[i];
                                    cx.notify();
                                },
                            ))
                        }),
                    )
                    .child(div().flex_1())
                    .child(
                        icon_button(
                            "rerun-checks",
                            Icon::Refresh,
                            "Run workspace checks",
                            false,
                            self.graph.is_some(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.run_checks(cx))),
                    )
                    .child(
                        icon_button(
                            "hide-problems",
                            Icon::ChevronDown,
                            "Hide Problems",
                            false,
                            true,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.toggle_problems(window, cx)),
                        ),
                    ),
            )
            .child(list)
    }
}
