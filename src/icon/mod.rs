//! Tray icon rendering: a battery ring with a device pictogram in the middle.
//! Pure: a state goes in, RGBA pixels come out.
//!
//! The ring fills clockwise from the top over a dim track. A sleeping device,
//! or one with an unknown level, is translucent and has no arc.
//! - [`palette`]: colours and the arc colour rule
//! - [`shapes`]: drawing primitives
//! - [`pictograms`]: the device silhouettes

pub mod palette;
pub mod pictograms;
pub mod shapes;

use std::f32::consts::PI;

use tiny_skia::{LineCap, Pixmap};

use self::shapes::{arc_points, circle, paint, polyline, stroke};
use crate::device::Kind;

/// Width and height of the icon, in pixels.
pub const SIZE: u32 = 64;

/// Outer radius of the ring and its thickness.
const RING_R: f32 = 31.5;
const RING_W: f32 = 6.5;
/// Alpha of the pictogram of an active and an inactive device.
const ALPHA_ACTIVE: u8 = 255;
const ALPHA_INACTIVE: u8 = 110;
/// Alpha of the ring track of an active and an inactive device.
const TRACK_ACTIVE: u8 = 70;
const TRACK_INACTIVE: u8 = 45;
/// Smallest arc drawn, so a nearly empty battery still shows a dot.
const MIN_ARC_LEVEL: u8 = 2;
/// At the bottom of a charging "breath" the arc dims to 12%.
const BREATH_DEPTH: f32 = 0.88;

/// Everything that decides how an icon looks.
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

impl IconState {
    fn is_active(&self) -> bool {
        self.online && self.level.is_some()
    }
}

/// The icon as straight (not premultiplied) RGBA, `SIZE` x `SIZE`.
pub fn render(st: &IconState) -> Vec<u8> {
    let mut pm = Pixmap::new(SIZE, SIZE).expect("non-zero size");
    let fg = palette::foreground(st.light_taskbar);
    let active = st.is_active();
    let c = SIZE as f32 / 2.0;
    // tiny-skia strokes are centred on the path; the ring's outer edge sits at RING_R
    let r = RING_R - RING_W / 2.0;

    let track = paint(fg, if active { TRACK_ACTIVE } else { TRACK_INACTIVE });
    stroke(&mut pm, &circle(c, c, r), &track, RING_W, LineCap::Butt);

    if let (true, Some(level)) = (active, st.level) {
        let alpha = (255.0 * st.pulse.clamp(0.0, 1.0)) as u8;
        let arc = paint(palette::arc_color(st.level, st.charging, st.low, fg), alpha);
        let level = level.min(100);
        if level == 100 {
            stroke(&mut pm, &circle(c, c, r), &arc, RING_W, LineCap::Butt);
        } else if level > 0 {
            let end = -90.0 + 360.0 * level.max(MIN_ARC_LEVEL) as f32 / 100.0;
            let path = polyline(arc_points(c, c, r, r, -90.0, end), false);
            stroke(&mut pm, &path, &arc, RING_W, LineCap::Round);
        }
    }

    let col = paint(fg, if active { ALPHA_ACTIVE } else { ALPHA_INACTIVE });
    pictograms::draw(&mut pm, st.kind, c, &col);

    pm.pixels()
        .iter()
        .flat_map(|p| {
            let d = p.demultiply();
            [d.red(), d.green(), d.blue(), d.alpha()]
        })
        .collect()
}

/// Arc brightness 0..=1 for phase 0..1: a smooth sine that lingers at the bright end.
pub fn breath_level(phase: f32) -> f32 {
    let k = (0.5 + 0.5 * (2.0 * PI * phase).cos()).powf(0.7);
    (1.0 - BREATH_DEPTH) + BREATH_DEPTH * k
}
