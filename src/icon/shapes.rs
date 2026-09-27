//! Geometry and drawing primitives on top of tiny-skia.

use std::f32::consts::PI;

use tiny_skia::{BlendMode, Color, FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

use super::palette::Rgb;

pub fn paint(rgb: Rgb, alpha: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(rgb[0], rgb[1], rgb[2], alpha);
    p.anti_alias = true;
    p
}

/// Paint that erases what is under it, for the cut-outs in the pictograms, so
/// the taskbar shows through on any theme.
pub fn eraser() -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::BLACK);
    p.blend_mode = BlendMode::Clear;
    p.anti_alias = true;
    p
}

pub fn circle(cx: f32, cy: f32, r: f32) -> Path {
    PathBuilder::from_circle(cx, cy, r).expect("positive radius")
}

pub fn rounded_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Path {
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    // a quarter circle drawn with a cubic: control points at 0.5523 * r
    let k = r * 0.5523;
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.cubic_to(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.cubic_to(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0);
    pb.close();
    pb.finish().expect("non-empty rectangle")
}

/// An open polyline through `points`, closed when `close` is set.
pub fn polyline(points: impl IntoIterator<Item = (f32, f32)>, close: bool) -> Path {
    let mut pb = PathBuilder::new();
    for (i, (x, y)) in points.into_iter().enumerate() {
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    if close {
        pb.close();
    }
    pb.finish().expect("at least two points")
}

/// Points along an ellipse arc; angles in degrees, 0 at 3 o'clock, clockwise.
pub fn arc_points(cx: f32, cy: f32, rx: f32, ry: f32, from_deg: f32, to_deg: f32) -> Vec<(f32, f32)> {
    let steps = (((to_deg - from_deg).abs() / 3.0).ceil() as usize).max(2);
    (0..=steps)
        .map(|i| {
            let t = (from_deg + (to_deg - from_deg) * i as f32 / steps as f32) * PI / 180.0;
            (cx + rx * t.cos(), cy + ry * t.sin())
        })
        .collect()
}

/// Catmull-Rom spline through a closed list of points.
pub fn smooth_closed(pts: &[(f32, f32)], steps: usize) -> Vec<(f32, f32)> {
    let n = pts.len();
    let mut out = Vec::with_capacity(n * steps);
    for i in 0..n {
        let p0 = pts[(i + n - 1) % n];
        let p1 = pts[i];
        let p2 = pts[(i + 1) % n];
        let p3 = pts[(i + 2) % n];
        for j in 0..steps {
            let t = j as f32 / steps as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b
                    + (-a + c) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    out
}

pub fn fill(pm: &mut Pixmap, path: &Path, p: &Paint) {
    pm.fill_path(path, p, FillRule::Winding, Transform::identity(), None);
}

pub fn fill_rect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, p: &Paint) {
    if let Some(r) = Rect::from_xywh(x, y, w, h) {
        pm.fill_rect(r, p, Transform::identity(), None);
    }
}

pub fn stroke(pm: &mut Pixmap, path: &Path, p: &Paint, width: f32, cap: LineCap) {
    let s = Stroke { width, line_cap: cap, ..Stroke::default() };
    pm.stroke_path(path, p, &s, Transform::identity(), None);
}
