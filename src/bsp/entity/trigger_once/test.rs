use super::super::test::decode;
use crate::bsp::{Entity, TriggerOnce};

#[test]
fn parses_trigger_volume_and_target_wiring_without_rendering_the_brush() {
    let entities = decode(
        br#"
        {"classname" "worldspawn"}
        {"classname" "trigger_once" "model" "*1" "target" "airlock"}
        {"classname" "trigger_once" "model" "*2" "target" "lights" "targetname" "entrance" "delay" "2.5"}
        "#,
    )
    .unwrap();
    assert_eq!(
        entities[1],
        Entity::TriggerOnce(TriggerOnce {
            model: 1,
            target: Some("airlock".into()),
            targetname: None,
            delay: 0.0,
        })
    );
    assert_eq!(
        entities[2],
        Entity::TriggerOnce(TriggerOnce {
            model: 2,
            target: Some("lights".into()),
            targetname: Some("entrance".into()),
            delay: 2.5,
        })
    );
    assert_eq!(entities[1].brush(), None);
    assert_eq!(entities[2].brush(), None);
}

#[test]
fn rejects_invalid_trigger_models_and_delays() {
    for properties in [
        r#""target" "airlock""#,
        r#""model" "*0""#,
        r#""model" "*9999""#,
        r#""model" "*1" "delay" "-1""#,
        r#""model" "*1" "delay" "NaN""#,
    ] {
        let text =
            format!(r#"{{"classname" "worldspawn"}} {{"classname" "trigger_once" {properties}}}"#);
        assert!(decode(text.as_bytes()).is_err(), "{properties}");
    }
}
