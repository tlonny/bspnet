use super::super::test::decode;
use crate::bsp::{Entity, TriggerMultiple};

#[test]
fn parses_a_repeatable_trigger_volume_and_its_cooldown() {
    for (properties, wait) in [("", 0.2), (r#""wait" "0""#, 0.2), (r#""wait" "3""#, 3.0)] {
        let text = format!(
            r#"{{"classname" "worldspawn"}} {{"classname" "trigger_multiple" "model" "*1" "target" "lights" "delay" "1.5" {properties}}}"#
        );
        let entities = decode(text.as_bytes()).unwrap();
        assert_eq!(
            entities[1],
            Entity::TriggerMultiple(TriggerMultiple {
                model: 1,
                target: Some("lights".into()),
                targetname: None,
                delay: 1.5,
                wait,
            })
        );
        assert_eq!(entities[1].brush(), None);
    }
}
