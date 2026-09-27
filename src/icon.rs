//! Tray icon rendering: a battery ring with a device pictogram in the middle.
//!
//! The ring fills clockwise from the top over a dim track. Colours follow the
//! system battery icon: normal charge uses the taskbar colour (white on a dark
//! taskbar, black on a light one), close to the threshold it is amber, at or
//! below the threshold red, and green while charging. A sleeping device, or one
//! with an unknown level, is translucent and has no arc.

use std::f32::consts::PI;

use tiny_skia::{
    BlendMode, Color, FillRule, LineCap, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

use crate::device::Kind;

pub const SIZE: u32 = 64;

const RED: [u8; 3] = [232, 17, 35];
const AMBER: [u8; 3] = [255, 185, 0];
const GREEN: [u8; 3] = [16, 196, 80];

const RING_R: f32 = 31.5;
const RING_W: f32 = 6.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconState {
    pub level: Option<u8>,
    pub charging: bool,
    pub online: bool,
    pub kind: Kind,
    /// Low-battery threshold in percent.
    pub low: u8,
    pub light_taskbar: bool,
    /// Arc brightness 0..=1, one frame of the charging "breathing" animation.
    pub pulse: f32,
}

fn arc_color(level: Option<u8>, charging: bool, low: u8, fg: [u8; 3]) -> [u8; 3] {
    if charging {
        return GREEN;
    }
    let thr = low.max(10);
    match level {
        Some(l) if l <= thr => RED,
        Some(l) if l <= thr + 10 => AMBER,
        _ => fg,
    }
}

fn paint(rgb: [u8; 3], alpha: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(rgb[0], rgb[1], rgb[2], alpha);
    p.anti_alias = true;
    p
}

/// Paint that erases what is under it, for the cut-outs in the pictograms, so
/// they show the taskbar through on any theme.
fn eraser() -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::BLACK);
    p.blend_mode = BlendMode::Clear;
    p.anti_alias = true;
    p
}

fn circle(cx: f32, cy: f32, r: f32) -> Path {
    PathBuilder::from_circle(cx, cy, r).unwrap()
}

fn rounded_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Path {
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
    pb.finish().unwrap()
}

fn arc(cx: f32, cy: f32, r: f32, from_deg: f32, to_deg: f32) -> Path {
    let steps = (((to_deg - from_deg).abs() / 3.0).ceil() as usize).max(2);
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let t = (from_deg + (to_deg - from_deg) * i as f32 / steps as f32) * PI / 180.0;
        let (x, y) = (cx + r * t.cos(), cy + r * t.sin());
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.finish().unwrap()
}

fn fill(pm: &mut Pixmap, path: &Path, p: &Paint) {
    pm.fill_path(path, p, FillRule::Winding, Transform::identity(), None);
}

fn stroke(pm: &mut Pixmap, path: &Path, p: &Paint, width: f32, cap: LineCap) {
    let s = Stroke { width, line_cap: cap, ..Stroke::default() };
    pm.stroke_path(path, p, &s, Transform::identity(), None);
}

// ------------------------------------------------------------------ pictograms

fn mouse(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let w = s * 0.62;
    fill(pm, &rounded_rect(cx - w, cy - s, cx + w, cy + s, w), col);
    let lw = (s * 0.18).max(2.5);
    let mut pb = PathBuilder::new();
    pb.move_to(cx, cy - s);
    pb.line_to(cx, cy - s * 0.2);
    pb.move_to(cx - w, cy - s * 0.2);
    pb.line_to(cx + w, cy - s * 0.2);
    stroke(pm, &pb.finish().unwrap(), &eraser(), lw, LineCap::Butt);
}

fn keyboard(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let (w, h) = (s * 1.05, s * 0.62);
    fill(pm, &rounded_rect(cx - w, cy - h, cx + w, cy + h, s * 0.18), col);
    let key = s * 0.26;
    let gap = s * 0.14;
    for row in 0..2 {
        let y = cy - h + gap * 1.6 + row as f32 * (key + gap);
        for i in 0..4 {
            let x = cx - w + gap * 1.8 + i as f32 * (key + gap * 1.6);
            let r = Rect::from_xywh(x, y, key, key).unwrap();
            pm.fill_rect(r, &eraser(), Transform::identity(), None);
        }
    }
    let bar = Rect::from_xywh(cx - w * 0.55, cy + h - gap * 1.6 - key, w * 1.1, key).unwrap();
    pm.fill_rect(bar, &eraser(), Transform::identity(), None);
}

fn headset(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let k = 0.8; // horizontal squeeze so the ear cups don't touch the ring
    let mut pb = PathBuilder::new();
    for i in 0..=40 {
        let t = (200.0 + 140.0 * i as f32 / 40.0) * PI / 180.0;
        let (x, y) = (cx + s * k * 0.85 * t.cos(), cy + s * 0.85 * t.sin());
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    stroke(pm, &pb.finish().unwrap(), col, s * 0.3, LineCap::Round);
    for side in [-1.0f32, 1.0] {
        let (a, b) = (cx + side * s * 0.58 * k, cx + side * s * 1.18 * k);
        fill(pm, &rounded_rect(a.min(b), cy - s * 0.1, a.max(b), cy + s * 0.75, s * 0.2), col);
    }
}

/// Right half of an Xbox controller outline, clockwise from the top centre, in
/// a design grid about 40 units wide; mirrored for the left half and drawn as
/// one smooth closed curve. Traced by the original HaloBattery project.
const PAD_HALF: [(f32, f32); 16] = [
    (0.0, -13.9), (5.5, -13.9), (9.3, -12.9), (12.8, -10.3), (15.2, -7.9), (16.3, -5.9),
    (18.1, -0.3), (19.7, 4.8), (20.0, 7.6), (19.5, 10.7), (17.9, 12.8), (15.3, 14.0),
    (14.5, 13.6), (9.0, 8.1), (6.0, 6.8), (0.0, 6.8),
];
const PAD_STICKS: [(f32, f32); 2] = [(-9.7, -6.1), (5.2, -0.3)];
const PAD_STICK_R: f32 = 2.6;

/// Catmull-Rom spline through a closed list of points.
fn smooth_closed(pts: &[(f32, f32)], steps: usize) -> Vec<(f32, f32)> {
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
                0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    out
}

fn gamepad(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let k = s / 18.0;
    let mut outline: Vec<(f32, f32)> = PAD_HALF.to_vec();
    outline.extend(PAD_HALF[1..PAD_HALF.len() - 1].iter().rev().map(|&(x, y)| (-x, y)));
    let mut pb = PathBuilder::new();
    for (i, (x, y)) in smooth_closed(&outline, 12).into_iter().enumerate() {
        let (px, py) = (cx + x * k, cy + y * k);
        if i == 0 {
            pb.move_to(px, py);
        } else {
            pb.line_to(px, py);
        }
    }
    pb.close();
    fill(pm, &pb.finish().unwrap(), col);
    for (sx, sy) in PAD_STICKS {
        fill(pm, &circle(cx + sx * k, cy + sy * k, PAD_STICK_R * k), &eraser());
    }
}

// ------------------------------------------------------------------ icon

/// The icon as straight (not premultiplied) RGBA, `SIZE` x `SIZE`.
pub fn render(st: &IconState) -> Vec<u8> {
    let mut pm = Pixmap::new(SIZE, SIZE).unwrap();
    let fg = if st.light_taskbar { [0, 0, 0] } else { [255, 255, 255] };
    let active = st.online && st.level.is_some();
    let alpha = if active { 255 } else { 110 };
    let c = SIZE as f32 / 2.0;
    let r = RING_R - RING_W / 2.0;

    // ring track
    let track = paint(fg, if active { 70 } else { 45 });
    stroke(&mut pm, &circle(c, c, r), &track, RING_W, LineCap::Butt);

    // charge arc
    if let (true, Some(level)) = (active, st.level) {
        let a = (255.0 * st.pulse.clamp(0.0, 1.0)) as u8;
        let p = paint(arc_color(st.level, st.charging, st.low, fg), a);
        let level = level.min(100);
        if level >= 100 {
            stroke(&mut pm, &circle(c, c, r), &p, RING_W, LineCap::Butt);
        } else if level > 0 {
            let end = -90.0 + 360.0 * level.max(2) as f32 / 100.0;
            stroke(&mut pm, &arc(c, c, r, -90.0, end), &p, RING_W, LineCap::Round);
        }
    }

    // device pictogram
    let col = paint(fg, alpha);
    match st.kind {
        Kind::Mouse => mouse(&mut pm, c, c, 19.5, &col),
        Kind::Keyboard => keyboard(&mut pm, c, c, 18.0, &col),
        Kind::Headset => headset(&mut pm, c, c + 2.0, 18.0, &col),
        Kind::Gamepad => gamepad(&mut pm, c, c, 18.4, &col),
    }

    pm.pixels().iter().flat_map(|p| {
        let d = p.demultiply();
        [d.red(), d.green(), d.blue(), d.alpha()]
    }).collect()
}

/// Arc brightness 0..=1 for phase 0..1: a smooth sine that lingers at the bright end.
pub fn breath_level(phase: f32) -> f32 {
    const DEPTH: f32 = 0.88; // at the bottom of the cycle the arc dims to 12%
    let k = (0.5 + 0.5 * (2.0 * PI * phase).cos()).powf(0.7);
    (1.0 - DEPTH) + DEPTH * k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_colour_follows_threshold() {
        let fg = [255, 255, 255];
        assert_eq!(arc_color(Some(80), false, 20, fg), fg);
        assert_eq!(arc_color(Some(25), false, 20, fg), AMBER);
        assert_eq!(arc_color(Some(20), false, 20, fg), RED);
        assert_eq!(arc_color(Some(5), true, 20, fg), GREEN);
    }

    #[test]
    fn renders_full_size_rgba() {
        let st = IconState {
            level: Some(50),
            charging: false,
            online: true,
            kind: Kind::Mouse,
            low: 20,
            light_taskbar: false,
            pulse: 1.0,
        };
        assert_eq!(render(&st).len(), (SIZE * SIZE * 4) as usize);
    }

    /// Writes every pictogram at a few levels to target/icon-preview.png.
    /// Run with `cargo test preview -- --ignored`.
    #[test]
    #[ignore]
    fn preview() {
        let kinds = [Kind::Mouse, Kind::Keyboard, Kind::Headset, Kind::Gamepad];
        let states: [(Option<u8>, bool, bool); 5] =
            [(Some(84), false, true), (Some(27), false, true), (Some(12), false, true),
             (Some(60), true, true), (Some(60), false, false)];
        let scale = 2;
        let (w, h) = (SIZE * scale * states.len() as u32, SIZE * scale * kinds.len() as u32 * 2);
        let mut sheet = Pixmap::new(w, h).unwrap();
        for (band, light) in [false, true].into_iter().enumerate() {
            let bg = if light { Color::from_rgba8(238, 238, 238, 255) } else { Color::from_rgba8(32, 32, 32, 255) };
            let y0 = (band as u32 * kinds.len() as u32 * SIZE * scale) as f32;
            let bgr = Rect::from_xywh(0.0, y0, w as f32, (kinds.len() as u32 * SIZE * scale) as f32).unwrap();
            let mut bp = Paint::default();
            bp.set_color(bg);
            sheet.fill_rect(bgr, &bp, Transform::identity(), None);
            for (row, kind) in kinds.iter().enumerate() {
                for (col, &(level, charging, online)) in states.iter().enumerate() {
                    let st = IconState { level, charging, online, kind: *kind, low: 20, light_taskbar: light, pulse: 1.0 };
                    let rgba = render(&st);
                    let img = tiny_skia::IntSize::from_wh(SIZE, SIZE)
                        .and_then(|sz| {
                            let mut pm = Pixmap::new(sz.width(), sz.height())?;
                            for (i, px) in pm.pixels_mut().iter_mut().enumerate() {
                                let c = &rgba[i * 4..i * 4 + 4];
                                *px = tiny_skia::ColorU8::from_rgba(c[0], c[1], c[2], c[3]).premultiply();
                            }
                            Some(pm)
                        })
                        .unwrap();
                    let x = (col as u32 * SIZE * scale) as f32;
                    let y = y0 + (row as u32 * SIZE * scale) as f32;
                    sheet.draw_pixmap(0, 0, img.as_ref(), &tiny_skia::PixmapPaint::default(),
                        Transform::from_scale(scale as f32, scale as f32).post_translate(x, y), None);
                }
            }
        }
        sheet.save_png(concat!(env!("CARGO_MANIFEST_DIR"), "/target/icon-preview.png")).unwrap();
    }
}
