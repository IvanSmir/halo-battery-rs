use halo_battery::config::RingColor;
use halo_battery::device::Kind;
use halo_battery::icon::palette::{AMBER, BLACK, GREEN, RED, WHITE, arc_color, foreground, ring_color};
use halo_battery::icon::{IconState, SIZE, breath_level, render};

fn state(level: Option<u8>) -> IconState {
    IconState {
        level,
        charging: false,
        online: true,
        kind: Kind::Mouse,
        low: 20,
        light_taskbar: false,
        ring: None,
        pictogram: true,
        pulse: 1.0,
    }
}

/// RGBA of the pixel at (x, y).
fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIZE + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

/// A point on the ring at 12 o'clock, where every arc starts.
const RING_TOP: (u32, u32) = (SIZE / 2, 3);

mod palette {
    use super::*;

    #[test]
    fn foreground_follows_the_taskbar() {
        assert_eq!(foreground(false), WHITE);
        assert_eq!(foreground(true), BLACK);
    }

    #[test]
    fn arc_colour_by_level() {
        assert_eq!(arc_color(Some(80), false, 20, WHITE), WHITE);
        assert_eq!(arc_color(Some(30), false, 20, WHITE), AMBER);
        assert_eq!(arc_color(Some(21), false, 20, WHITE), AMBER);
        assert_eq!(arc_color(Some(20), false, 20, WHITE), RED);
        assert_eq!(arc_color(None, false, 20, WHITE), WHITE);
    }

    #[test]
    fn charging_is_always_green() {
        assert_eq!(arc_color(Some(5), true, 20, WHITE), GREEN);
    }

    #[test]
    fn the_red_threshold_never_goes_below_ten() {
        assert_eq!(arc_color(Some(10), false, 5, WHITE), RED);
    }
}

mod rendering {
    use super::*;

    #[test]
    fn produces_a_full_rgba_image() {
        assert_eq!(render(&state(Some(50))).len(), (SIZE * SIZE * 4) as usize);
    }

    #[test]
    fn every_kind_renders() {
        for kind in [Kind::Mouse, Kind::Keyboard, Kind::Headset, Kind::Gamepad] {
            let img = render(&IconState { kind, ..state(Some(50)) });
            assert!(img.chunks_exact(4).any(|p| p[3] > 0), "{kind:?} draws something");
        }
    }

    #[test]
    fn the_arc_is_drawn_in_its_colour() {
        let [r, g, b, a] = pixel(&render(&state(Some(10))), RING_TOP.0, RING_TOP.1);
        assert_eq!([r, g, b], RED);
        assert!(a > 200);
    }

    #[test]
    fn asleep_devices_have_only_a_faint_track() {
        let img = render(&IconState { online: false, ..state(Some(80)) });
        let [.., a] = pixel(&img, RING_TOP.0, RING_TOP.1);
        assert!(a < 100, "no arc, only the track (alpha {a})");
    }

    #[test]
    fn a_light_taskbar_gets_a_dark_icon() {
        let img = render(&IconState { light_taskbar: true, ..state(Some(80)) });
        let [r, g, b, _] = pixel(&img, RING_TOP.0, RING_TOP.1);
        assert_eq!([r, g, b], BLACK);
    }

    #[test]
    fn a_chosen_ring_colour_replaces_the_taskbar_colour() {
        let mint = ring_color(RingColor::Mint).unwrap();
        let img = render(&IconState { ring: Some(mint), ..state(Some(80)) });
        let [r, g, b, _] = pixel(&img, RING_TOP.0, RING_TOP.1);
        assert_eq!([r, g, b], mint);
    }

    #[test]
    fn warnings_keep_their_colour_whatever_the_ring_colour() {
        let img = render(&IconState { ring: ring_color(RingColor::Mint), ..state(Some(10)) });
        let [r, g, b, _] = pixel(&img, RING_TOP.0, RING_TOP.1);
        assert_eq!([r, g, b], RED);
    }

    #[test]
    fn without_pictogram_the_centre_stays_empty() {
        let c = SIZE / 2;
        assert!(pixel(&render(&state(Some(50))), c, c)[3] > 0, "the mouse covers the centre");
        let [.., a] = pixel(&render(&IconState { pictogram: false, ..state(Some(50)) }), c, c);
        assert_eq!(a, 0);
    }
}

#[test]
fn breathing_stays_visible_and_peaks_at_the_start() {
    assert!((breath_level(0.0) - 1.0).abs() < 1e-6);
    for i in 0..100 {
        let b = breath_level(i as f32 / 100.0);
        assert!((0.1..=1.0).contains(&b), "phase {i}: {b}");
    }
}
