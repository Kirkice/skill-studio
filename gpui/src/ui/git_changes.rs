use crate::{
    Studio,
    git_diff::{Change, Snapshot},
};
use gpui::*;
use gpui_base::input::{RangeDecoration, RangeDecorationCollection, RangeDecorationStyle};

pub(crate) struct GitChanges {
    pub(super) enabled: bool,
    key: Option<(String, u64, Option<String>)>,
    pub(super) files: Option<std::sync::Arc<Snapshot>>,
    error: Option<String>,
    decoration: Option<RangeDecorationCollection>,
    shown: Option<(EntityId, Option<(String, u64, Option<String>)>)>,
}
impl Default for GitChanges {
    fn default() -> Self {
        Self {
            enabled: true,
            key: None,
            files: None,
            error: None,
            decoration: None,
            shown: None,
        }
    }
}
impl Studio {
    pub(super) fn sync_git(&mut self, cx: &mut Context<Self>) {
        let key = self
            .graph
            .as_ref()
            .filter(|_| self.git.enabled)
            .map(|g| (g.root.clone(), self.graph_revision, self.selected.clone()));
        if self.git.key != key {
            self.git.key = key.clone();
            self.git.files = None;
            self.git.error = None;
            self.git.shown = None;
            let mut docs: Vec<_> = self
                .graph
                .iter()
                .flat_map(|g| g.flows.iter().map(|f| f.source.clone()))
                .collect();
            docs.extend(self.selected.clone());
            docs.sort();
            docs.dedup();
            if let Some(key) = key {
                cx.spawn(async move |this, cx| {
                    let root = key.0.clone();
                    let docs = docs;
                    let result =
                        cx.background_executor()
                            .spawn(async move {
                                crate::git_diff::scan(std::path::Path::new(&root), &docs)
                            })
                            .await;
                    let _ = this.update(cx, |this, cx| {
                        if this.git.key.as_ref() != Some(&key) {
                            return;
                        }
                        match result {
                            Ok(files) => this.git.files = Some(files.into()),
                            Err(e) => this.git.error = Some(e),
                        }
                        this.git.shown = None;
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        let target = if self.git.files.is_some() && !self.dirty(cx) && self.active_text() {
            self.git.key.clone()
        } else {
            None
        };
        let shown = (self.editor.entity_id(), target.clone());
        if self.git.shown.as_ref() == Some(&shown) {
            return;
        }
        self.git.shown = Some(shown);
        if let Some(old) = self.git.decoration.take() {
            old.dispose(cx);
        }
        let Some(file) = target.and_then(|(_, _, id)| self.git.files.as_ref()?.get(&id?)) else {
            return;
        };
        let value = self.editor.read(cx).value();
        let mut start = 0;
        let mut ranges = vec![];
        for (i, line) in value.split_inclusive('\n').enumerate() {
            let row = i + 1;
            let end = start + line.trim_end_matches(['\r', '\n']).len();
            let change = if file.change == Some(Change::Added) {
                Some(Change::Added)
            } else {
                file.lines.iter().find_map(|&(at, n, c)| {
                    ((n == 0 && row == at.max(1)) || (row >= at && row < at.saturating_add(n)))
                        .then_some(c)
                })
            };
            if let Some(change) = change.filter(|_| start < end) {
                ranges.push(
                    RangeDecoration::new(start..end)
                        .with_style(RangeDecorationStyle::Fill)
                        .with_color(Hsla::from(gpui::rgb(change.color())).opacity(0.20)),
                );
            }
            start += line.len();
        }
        self.git.decoration = Some(self.editor.update(cx, |editor, cx| {
            editor.create_range_decorations_collection(ranges, cx)
        }));
    }
    pub(super) fn git_status(&self) -> String {
        if let Some(error) = &self.git.error {
            return error.clone();
        }
        let Some(files) = &self.git.files else {
            return "Comparing…".into();
        };
        let deleted = files
            .values()
            .filter(|f| f.change == Some(Change::Deleted))
            .count();
        format!("{} changed · {deleted} deleted", files.len())
    }
}
pub(super) fn same_snapshot(
    a: &Option<std::sync::Arc<Snapshot>>,
    b: &Option<std::sync::Arc<Snapshot>>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
pub(super) fn outline(x: Pixels, y: Pixels, w: Pixels, h: Pixels, change: Change) -> Div {
    let tint = gpui::rgb(change.color());
    div()
        .absolute()
        .left(x - px(8.))
        .top(y - px(8.))
        .w(w + px(16.))
        .h(h + px(16.))
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let dash = 12.;
                    let pitch = 21.;
                    let width = f32::from(bounds.size.width);
                    let height = f32::from(bounds.size.height);
                    let mut stroke = |horizontal: bool, span: f32, cross: f32| {
                        let count = ((span + 9.) / pitch).floor().max(1.);
                        let margin = (span - (count - 1.) * pitch - dash).max(0.) * 0.5;
                        for i in 0..count as usize {
                            let at = px(margin + i as f32 * pitch);
                            let len = px(dash.min(span));
                            let (offset, block) = if horizontal {
                                (point(at, px(cross)), size(len, px(2.)))
                            } else {
                                (point(px(cross), at), size(px(2.), len))
                            };
                            window
                                .paint_quad(fill(Bounds::new(bounds.origin + offset, block), tint));
                        }
                    };
                    stroke(true, width, 0.);
                    stroke(true, width, height - 2.);
                    stroke(false, height - 8., 0.);
                    stroke(false, height - 8., width - 2.);
                },
            )
            .size_full(),
        )
}
