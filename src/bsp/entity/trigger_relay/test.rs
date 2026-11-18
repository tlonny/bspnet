use super::super::test::decode;
use crate::bsp::{Entity, TriggerRelay};

#[test]
fn parses_a_point_relay_with_optional_one_shot_behavior() {
    for (properties, once) in [
        ("", false),
        (r#""once" "0""#, false),
        (r#""once" "1""#, true),
    ] {
        let text = format!(
            r#"{{"classname" "worldspawn"}} {{"classname" "trigger_relay" "targetname" "open_airlock" "target" "airlock" {properties}}}"#
        );
        let entities = decode(text.as_bytes()).unwrap();
        assert_eq!(
            entities[1],
            Entity::TriggerRelay(TriggerRelay {
                target: Some("airlock".into()),
                targetname: Some("open_airlock".into()),
                delay: 0.0,
                once,
            })
        );
        assert_eq!(entities[1].brush(), None);
    }
}
