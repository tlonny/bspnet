use std::collections::HashMap;

use super::super::super::{Button, Data as Bsp, Entity};
use super::super::test::decode;

const BSP_BYTES: &[u8] = include_bytes!("../../../../assets/maps/librequake/lq_e0m1.bsp");
const MODEL_COUNT: usize = 4;

// Construct a button with required fields and optional overrides.
fn properties(extra: &[(&str, &str)]) -> HashMap<String, String> {
    [("classname", "func_button"), ("model", "*1")]
        .into_iter()
        .chain(extra.iter().copied())
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn requires_model_and_accepts_an_optional_target_and_extra_properties() {
    let button = Button::parse(1, &properties(&[]), MODEL_COUNT).unwrap();
    assert_eq!(
        button,
        Button {
            model: 1,
            alpha: None,
            target: None,
            wait: Some(1.),
        }
    );
    let button = Button::parse(
        1,
        &properties(&[
            ("model", " *3 "),
            ("target", "exit_door"),
            ("angle", "ignored"),
            ("lip", "ignored"),
            ("speed", "ignored"),
            ("unused", "junk"),
        ]),
        MODEL_COUNT,
    )
    .unwrap();
    assert_eq!(button.model, 3);
    assert_eq!(button.target.as_deref(), Some("exit_door"));
    assert_eq!(
        Button::parse(1, &properties(&[("target", "")]), MODEL_COUNT)
            .unwrap()
            .target,
        None
    );
}

#[test]
fn parses_wait_and_unpacks_never_return() {
    for invalid in ["", "junk", "1 2", "NaN", "inf", "1e100"] {
        assert!(Button::parse(1, &properties(&[("wait", invalid)]), MODEL_COUNT).is_err());
    }
    for (text, expected) in [("0", Some(1.)), ("0.05", Some(0.05)), ("-1", None)] {
        assert_eq!(
            Button::parse(1, &properties(&[("wait", text)]), MODEL_COUNT)
                .unwrap()
                .wait,
            expected
        );
    }
}

#[test]
fn rejects_missing_or_invalid_models_and_incorrect_identity() {
    let mut missing = properties(&[]);
    missing.remove("model");
    assert!(Button::parse(1, &missing, MODEL_COUNT).is_err());
    for model in ["", "button.bsp", "*", "*-1", "*0", "*4", "*1junk"] {
        assert!(Button::parse(1, &properties(&[("model", model)]), MODEL_COUNT).is_err());
    }
    assert!(Button::parse(0, &properties(&[]), MODEL_COUNT).is_err());
    assert!(Button::parse(1, &properties(&[("classname", "func_door")]), MODEL_COUNT).is_err());
    let mut missing = properties(&[]);
    missing.remove("classname");
    assert!(Button::parse(1, &missing, MODEL_COUNT).is_err());
}

#[test]
fn lump_decode_keeps_button_indices_and_uses_the_last_target() {
    let entities = decode(
        br#"
        {"classname" "worldspawn"}
        {"classname" "func_button" "model" "*1" "target" "old" "target" "exit_door"}
        {"classname" "unsupported"}
    "#,
    )
    .unwrap();
    let Entity::Button(button) = &entities[1] else {
        panic!("expected button")
    };
    assert_eq!(button.model, 1);
    assert_eq!(button.target.as_deref(), Some("exit_door"));
    assert_eq!(entities[2], Entity::Unknown);
    assert!(decode(br#"{"classname" "worldspawn"}{"classname" "func_button"}"#).is_err());
}

#[test]
fn parses_both_librequake_buttons_and_the_exit_door_link() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let buttons: Vec<_> = bsp
        .entities()
        .iter()
        .filter_map(|entity| match entity {
            Entity::Button(button) => Some(button),
            _ => None,
        })
        .collect();
    assert_eq!(buttons.len(), 2);
    assert_eq!(buttons[0].model, 2);
    assert_eq!(buttons[0].target, None);
    assert_eq!(buttons[0].wait, Some(0.05));
    assert_eq!(buttons[1].model, 18);
    assert_eq!(buttons[1].target.as_deref(), Some("exit_door"));
    assert_eq!(buttons[1].wait, None);
    assert!(bsp.entities().iter().any(|entity| matches!(entity,
        Entity::Door(door) if door.targetname.as_deref() == buttons[1].target.as_deref()
    )));
}
