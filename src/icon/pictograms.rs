//! The device silhouettes drawn in the middle of the ring. Each one is drawn
//! around a centre `(cx, cy)` at a size `s` (roughly half its height).

use tiny_skia::{LineCap, Paint, Pixmap};

use super::shapes::{arc_points, circle, eraser, fill, fill_rect, polyline, rounded_rect, smooth_closed, stroke};
use crate::device::Kind;

/// Draws the pictogram of `kind` centred in a 64x64 icon.
pub fn draw(pm: &mut Pixmap, kind: Kind, c: f32, col: &Paint) {
    match kind {
        Kind::Mouse => mouse(pm, c, c, 19.5, col),
        Kind::Keyboard => keyboard(pm, c, c, 18.0, col),
        Kind::Headset => headset(pm, c, c + 2.0, 18.0, col),
        Kind::Gamepad => gamepad(pm, c, c, 18.4, col),
    }
}

fn mouse(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let w = s * 0.62;
    fill(pm, &rounded_rect(cx - w, cy - s, cx + w, cy + s, w), col);
    // the button lines are cut out
    let lw = (s * 0.18).max(2.5);
    stroke(pm, &polyline([(cx, cy - s), (cx, cy - s * 0.2)], false), &eraser(), lw, LineCap::Butt);
    stroke(pm, &polyline([(cx - w, cy - s * 0.2), (cx + w, cy - s * 0.2)], false), &eraser(), lw, LineCap::Butt);
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
            fill_rect(pm, x, y, key, key, &eraser());
        }
    }
    // space bar
    fill_rect(pm, cx - w * 0.55, cy + h - gap * 1.6 - key, w * 1.1, key, &eraser());
}

fn headset(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let k = 0.8; // horizontal squeeze so the ear cups don't touch the ring
    let band = polyline(arc_points(cx, cy, s * k * 0.85, s * 0.85, 200.0, 340.0), false);
    stroke(pm, &band, col, s * 0.3, LineCap::Round);
    for side in [-1.0f32, 1.0] {
        let (a, b) = (cx + side * s * 0.58 * k, cx + side * s * 1.18 * k);
        fill(pm, &rounded_rect(a.min(b), cy - s * 0.1, a.max(b), cy + s * 0.75, s * 0.2), col);
    }
}

/// Right half of an Xbox controller outline, clockwise from the top centre, in
/// a design grid about 40 units wide; mirrored for the left half and drawn as
/// one smooth closed curve. Traced by the original HaloBattery project.
const PAD_HALF: [(f32, f32); 16] = [
    (0.0, -13.9),
    (5.5, -13.9),
    (9.3, -12.9),
    (12.8, -10.3),
    (15.2, -7.9),
    (16.3, -5.9),
    (18.1, -0.3),
    (19.7, 4.8),
    (20.0, 7.6),
    (19.5, 10.7),
    (17.9, 12.8),
    (15.3, 14.0),
    (14.5, 13.6),
    (9.0, 8.1),
    (6.0, 6.8),
    (0.0, 6.8),
];
/// Where the sticks sit on an Xbox pad: left one high and to the side, right
/// one lower and closer to the middle.
const PAD_STICKS: [(f32, f32); 2] = [(-9.7, -6.1), (5.2, -0.3)];
const PAD_STICK_R: f32 = 2.6;
/// Size of the design grid the outline is drawn in.
const PAD_GRID: f32 = 18.0;

fn gamepad(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, col: &Paint) {
    let k = s / PAD_GRID;
    let mut outline: Vec<(f32, f32)> = PAD_HALF.to_vec();
    outline.extend(PAD_HALF[1..PAD_HALF.len() - 1].iter().rev().map(|&(x, y)| (-x, y)));
    let body = smooth_closed(&outline, 12).into_iter().map(|(x, y)| (cx + x * k, cy + y * k));
    fill(pm, &polyline(body, true), col);
    for (sx, sy) in PAD_STICKS {
        fill(pm, &circle(cx + sx * k, cy + sy * k, PAD_STICK_R * k), &eraser());
    }
}
