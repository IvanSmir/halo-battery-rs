use halo_battery::device::Kind;
use halo_battery::providers::logitech::protocol::{
    battery_function, kind_from_type, match_reply, parse_battery, request, unit_id, voltage_to_percent, Reply,
    F_STATUS, F_UNIFIED, F_VOLTAGE, LONG_LEN, SWID,
};

mod request_layout {
    use super::*;

    #[test]
    fn builds_a_long_report() {
        let r = request(1, 0x05, 1, &[0xAB, 0xCD]);
        assert_eq!(r.len(), LONG_LEN);
        assert_eq!(&r[..6], &[0x11, 1, 0x05, 0x10 | SWID, 0xAB, 0xCD]);
        assert!(r[6..].iter().all(|&b| b == 0));
    }

    #[test]
    fn truncates_params_that_do_not_fit() {
        let r = request(1, 0, 0, &[0xFF; 32]);
        assert_eq!(r.len(), LONG_LEN);
        assert!(r[4..].iter().all(|&b| b == 0xFF));
    }
}

mod reply_matching {
    use super::*;

    const FUNC: u8 = 1;

    fn answer(idx: u8, feat: u8, params: &[u8]) -> Vec<u8> {
        let mut r = vec![0x11, idx, feat, (FUNC << 4) | SWID];
        r.extend_from_slice(params);
        r
    }

    #[test]
    fn returns_padded_params_of_the_matching_answer() {
        let Some(Reply::Ok(p)) = match_reply(&answer(1, 4, &[84, 8, 0]), 1, 4, FUNC) else {
            panic!("expected an answer");
        };
        assert_eq!(&p[..3], &[84, 8, 0]);
        assert!(p.len() >= 16, "params are zero padded");
    }

    #[test]
    fn recognises_short_and_long_error_replies() {
        assert_eq!(match_reply(&[0x10, 1, 0x8F, 4, 0, 0, 0], 1, 4, FUNC), Some(Reply::Error));
        assert_eq!(match_reply(&[0x11, 1, 0xFF, 4, 0, 0, 0], 1, 4, FUNC), Some(Reply::Error));
    }

    #[test]
    fn ignores_other_slots_features_and_functions() {
        assert_eq!(match_reply(&answer(2, 4, &[1]), 1, 4, FUNC), None);
        assert_eq!(match_reply(&answer(1, 5, &[1]), 1, 4, FUNC), None);
        assert_eq!(match_reply(&answer(1, 4, &[1]), 1, 4, FUNC + 1), None);
    }

    #[test]
    fn ignores_truncated_reports() {
        assert_eq!(match_reply(&[0x11, 1, 4], 1, 4, FUNC), None);
    }
}

mod battery_decoding {
    use super::*;

    #[test]
    fn unified_battery() {
        assert_eq!(parse_battery(F_UNIFIED, &[80, 8, 0]), (Some(80), false));
        assert_eq!(parse_battery(F_UNIFIED, &[40, 8, 1]), (Some(40), true));
        assert_eq!(parse_battery(F_UNIFIED, &[100, 8, 3]), (Some(100), true), "full on the charger");
        assert_eq!(parse_battery(F_UNIFIED, &[0xFF, 0, 0]), (None, false));
    }

    #[test]
    fn battery_status() {
        assert_eq!(parse_battery(F_STATUS, &[55, 0, 0]), (Some(55), false));
        assert_eq!(parse_battery(F_STATUS, &[55, 0, 1]), (Some(55), true));
        assert_eq!(parse_battery(F_STATUS, &[0, 0, 0]), (None, false), "0 means unknown");
    }

    #[test]
    fn battery_voltage() {
        // 0x0EE3 = 3811 mV = 50 %, bit 7 of the flags: external power
        assert_eq!(parse_battery(F_VOLTAGE, &[0x0E, 0xE3, 0x80]), (Some(50), true));
        assert_eq!(parse_battery(F_VOLTAGE, &[0x0E, 0xE3, 0x00]), (Some(50), false));
        assert_eq!(parse_battery(F_VOLTAGE, &[0x00, 0x10, 0x00]), (None, false), "implausible voltage");
    }

    #[test]
    fn rejects_short_payloads_and_unknown_features() {
        assert_eq!(parse_battery(F_UNIFIED, &[80]), (None, false));
        assert_eq!(parse_battery(0x1234, &[80, 0, 0]), (None, false));
    }

    #[test]
    fn voltage_curve_interpolates_between_points() {
        assert_eq!(voltage_to_percent(4300), 100);
        assert_eq!(voltage_to_percent(4186), 100);
        assert_eq!(voltage_to_percent(3811), 50);
        assert_eq!(voltage_to_percent(3835), 55);
        assert_eq!(voltage_to_percent(3500), 0);
        assert_eq!(voltage_to_percent(3000), 0);
    }

    #[test]
    fn only_unified_battery_reads_with_function_1() {
        assert_eq!(battery_function(F_UNIFIED), 1);
        assert_eq!(battery_function(F_STATUS), 0);
        assert_eq!(battery_function(F_VOLTAGE), 0);
    }
}

mod identity {
    use super::*;

    #[test]
    fn device_type_to_kind() {
        assert_eq!(kind_from_type(0), Kind::Keyboard);
        assert_eq!(kind_from_type(2), Kind::Keyboard);
        assert_eq!(kind_from_type(3), Kind::Mouse);
        assert_eq!(kind_from_type(5), Kind::Mouse);
    }

    #[test]
    fn unit_id_as_hex_or_none_when_blank() {
        assert_eq!(unit_id(&[1, 0xD9, 0x88, 0x09, 0x5B]), Some("D988095B".into()));
        assert_eq!(unit_id(&[1, 0, 0, 0, 0]), None);
        assert_eq!(unit_id(&[1, 2]), None);
    }
}
