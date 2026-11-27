//! Shared selection language inspired by RenderMax's RenderGraph (not execution status).
use super::*;
/// A subdued two-level grid anchored to world coordinates in both graph views.
pub(super) fn graph_background(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    camera: Point<Pixels>,
    zoom: f32,
) {
    let light = super::super::theme::is_light();
    // The graph panel already paints the theme's CANVAS color. Only overlay lines.
    let (minor, major) = if light {
        (rgba(0x00000009), rgba(0x00000020))
    } else {
        (rgba(0xffffff07), rgba(0xffffff15))
    };
    let mut step = 16. * zoom.max(0.01);
    while step < 8. {
        step *= 2.;
    }
    let scale = window.scale_factor();
    let stroke = px(1. / scale);
    let snap = |coordinate: Pixels| px((f32::from(coordinate) * scale).round() / scale);
    for (spacing, color) in [(step, minor), (step * 8., major)] {
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let mut x = f32::from(camera.x).rem_euclid(spacing);
        while x < width {
            let origin = point(snap(bounds.origin.x + px(x)), bounds.origin.y);
            window.paint_quad(fill(
                Bounds::new(origin, size(stroke, bounds.size.height)),
                color,
            ));
            x += spacing;
        }
        let mut y = f32::from(camera.y).rem_euclid(spacing);
        while y < height {
            let origin = point(bounds.origin.x, snap(bounds.origin.y + px(y)));
            window.paint_quad(fill(
                Bounds::new(origin, size(bounds.size.width, stroke)),
                color,
            ));
            y += spacing;
        }
    }
}
pub(super) const INCOMING: u32 = 0xffb800;
pub(super) const OUTGOING: u32 = 0x1fe482;
#[derive(Clone, Copy, Debug)]
pub(super) struct EdgeStyle {
    pub color: u32,
    pub width: f32,
    pub alpha: f32,
    pub related: bool,
}
pub(super) fn edge_style(selected: Option<&str>, source: &str, target: &str) -> EdgeStyle {
    if selected == Some(target) {
        EdgeStyle {
            color: INCOMING,
            width: 2.8,
            alpha: 1.,
            related: true,
        }
    } else if selected == Some(source) {
        EdgeStyle {
            color: OUTGOING,
            width: 2.8,
            alpha: 1.,
            related: true,
        }
    } else {
        EdgeStyle {
            color: 0x71859d,
            width: 1.1,
            alpha: if selected.is_some() { 0.22 } else { 0.65 },
            related: false,
        }
    }
}
pub(super) fn tint(color: u32, alpha: f32) -> Rgba {
    let mut color = rgb(color);
    color.a = alpha;
    color
}
pub(super) fn connection(
    window: &mut Window,
    start: Point<Pixels>,
    end: Point<Pixels>,
    horizontal: bool,
    dashed: bool,
    style: EdgeStyle,
) {
    if style.related {
        let mut path = PathBuilder::stroke(px(style.width));
        if dashed {
            path = path.dash_array(&[px(5.), px(4.)]);
        }
        path.move_to(start);
        if horizontal {
            let bend = px((f32::from(end.x - start.x).abs() * 0.5).max(24.));
            let direction = if end.x >= start.x { 1. } else { -1. };
            path.cubic_bezier_to(
                end,
                start + point(bend * direction, px(0.)),
                end - point(bend * direction, px(0.)),
            );
        } else {
            let bend = (end.y - start.y) * 0.5;
            path.cubic_bezier_to(end, start + point(px(0.), bend), end - point(px(0.), bend));
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, tint(style.color, style.alpha));
        }
    } else if horizontal {
        let stroke = px(style.width);
        let color = tint(style.color, style.alpha);
        let mid_x = (start.x + end.x) * 0.5;
        let x1 = start.x.min(mid_x);
        let w1 = (mid_x - start.x).abs().max(stroke);
        window.paint_quad(fill(
            Bounds::new(point(x1, start.y - stroke * 0.5), size(w1, stroke)),
            color,
        ));
        let y_min = start.y.min(end.y);
        let h_mid = (end.y - start.y).abs().max(stroke);
        window.paint_quad(fill(
            Bounds::new(point(mid_x - stroke * 0.5, y_min), size(stroke, h_mid)),
            color,
        ));
        let x2 = mid_x.min(end.x);
        let w2 = (end.x - mid_x).abs().max(stroke);
        window.paint_quad(fill(
            Bounds::new(point(x2, end.y - stroke * 0.5), size(w2, stroke)),
            color,
        ));
    } else {
        let stroke = px(style.width);
        let color = tint(style.color, style.alpha);
        let y_min = start.y.min(end.y);
        let h = (end.y - start.y).abs().max(stroke);
        window.paint_quad(fill(
            Bounds::new(point(start.x - stroke * 0.5, y_min), size(stroke, h)),
            color,
        ));
    }
    if style.related {
        let arm = px(5.5);
        let mut arrow = PathBuilder::stroke(px(1.8));
        if horizontal {
            let d = if end.x >= start.x { 1. } else { -1. };
            arrow.move_to(end + point(-arm * d, -arm));
            arrow.line_to(end);
            arrow.line_to(end + point(-arm * d, arm));
        } else {
            let d = if end.y >= start.y { 1. } else { -1. };
            arrow.move_to(end + point(-arm, -arm * d));
            arrow.line_to(end);
            arrow.line_to(end + point(arm, -arm * d));
        }
        if let Ok(path) = arrow.build() {
            window.paint_path(path, tint(style.color, style.alpha));
        }
    }
    for at in [start, end] {
        let r = if style.related { 3. } else { 2. };
        window.paint_quad(
            fill(
                Bounds::new(at - point(px(r), px(r)), size(px(r * 2.), px(r * 2.))),
                tint(style.color, style.alpha),
            )
            .corner_radii(px(r)),
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn selection_only_highlights_incident_edges_and_separates_direction() {
        assert_eq!(edge_style(Some("a"), "b", "a").color, INCOMING);
        assert_eq!(edge_style(Some("a"), "a", "b").color, OUTGOING);
        let unrelated = edge_style(Some("a"), "b", "c");
        assert!(!unrelated.related);
        assert!(unrelated.alpha < edge_style(None, "b", "c").alpha);
        assert!(!edge_style(Some("root/stage1/call"), "root/stage2/call", "x").related);
    }
}
