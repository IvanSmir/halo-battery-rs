//! Renders every pictogram in every state, on a dark and a light taskbar, to
//! target/icon-preview.png.
//!
//! cargo run --example icon_preview

use halo_battery::device::Kind;
use halo_battery::icon::{self, IconState, SIZE};
use tiny_skia::{Color, ColorU8, Paint, Pixmap, PixmapPaint, Rect, Transform};

const KINDS: [Kind; 4] = [Kind::Mouse, Kind::Keyboard, Kind::Headset, Kind::Gamepad];
/// (level, charging, online): normal, amber, red, charging, asleep.
const STATES: [(Option<u8>, bool, bool); 5] = [
    (Some(84), false, true),
    (Some(27), false, true),
    (Some(12), false, true),
    (Some(60), true, true),
    (Some(60), false, false),
];
const SCALE: u32 = 2;

fn to_pixmap(rgba: &[u8]) -> Pixmap {
    let mut pm = Pixmap::new(SIZE, SIZE).unwrap();
    for (px, c) in pm.pixels_mut().iter_mut().zip(rgba.chunks_exact(4)) {
        *px = ColorU8::from_rgba(c[0], c[1], c[2], c[3]).premultiply();
    }
    pm
}

fn main() {
    let cell = (SIZE * SCALE) as f32;
    let band_h = cell * KINDS.len() as f32;
    let mut sheet = Pixmap::new(SIZE * SCALE * STATES.len() as u32, (band_h * 2.0) as u32).unwrap();

    for (band, light) in [false, true].into_iter().enumerate() {
        let y0 = band as f32 * band_h;
        let mut bg = Paint::default();
        bg.set_color(if light { Color::from_rgba8(238, 238, 238, 255) } else { Color::from_rgba8(32, 32, 32, 255) });
        let rect = Rect::from_xywh(0.0, y0, sheet.width() as f32, band_h).unwrap();
        sheet.fill_rect(rect, &bg, Transform::identity(), None);

        for (row, &kind) in KINDS.iter().enumerate() {
            for (col, &(level, charging, online)) in STATES.iter().enumerate() {
                let st = IconState { level, charging, online, kind, low: 20, light_taskbar: light, pulse: 1.0 };
                let img = to_pixmap(&icon::render(&st));
                let at = Transform::from_scale(SCALE as f32, SCALE as f32)
                    .post_translate(col as f32 * cell, y0 + row as f32 * cell);
                sheet.draw_pixmap(0, 0, img.as_ref(), &PixmapPaint::default(), at, None);
            }
        }
    }

    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/target/icon-preview.png");
    sheet.save_png(out).unwrap();
    println!("{out}");
}
