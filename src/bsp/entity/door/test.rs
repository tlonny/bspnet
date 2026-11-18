use std::collections::HashMap;
use std::io::ErrorKind;

use super::super::super::{Angle, Data as Bsp, Door, Entity};
use super::super::test::decode;

const BSP_BYTES: &[u8] = include_bytes!("../../../../assets/maps/librequake/lq_e0m1.bsp");
const MODEL_COUNT: usize = 4;

// Construct a door with the required fields and optional property overrides.
fn properties(extra: &[(&str, &str)]) -> HashMap<String, String> {
    [("classname", "func_door"), ("model", "*1")]
        .into_iter()
        .chain(extra.iter().copied())
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn requires_only_model_and_applies_quake_defaults() {
    let door = Door::parse(
        1,
        &properties(&[("unrecognised", "arbitrary junk")]),
        MODEL_COUNT,
    )
    .unwrap();
    assert_eq!(
        door,
        Door {
            model: 1,
            alpha: None,
            angle: Angle::Yaw(0.),
            speed: 100.,
            lip: 8.,
            targetname: None,
        }
    );
}

#[test]
fn parses_supported_properties_without_retaining_unknown_fields() {
    let door = Door::parse(
        2,
        &properties(&[
            ("model", " *3 "),
            ("angle", " 270 "),
            ("speed", "2.5e1"),
            ("lip", "16.5"),
            ("targetname", "open this door"),
            ("wait", "not implemented yet"),
            ("alpha", "0.75"),
        ]),
        MODEL_COUNT,
    )
    .unwrap();
    assert_eq!(door.model, 3);
    assert_eq!(door.alpha, Some(0.75));
    assert_eq!(door.angle, Angle::Yaw(270.));
    assert_eq!(door.speed, 25.);
    assert_eq!(door.lip, 16.5);
    assert_eq!(door.targetname.as_deref(), Some("open this door"));
}

#[test]
fn unpacks_vertical_angles_and_preserves_ordinary_yaw_values() {
    for (text, expected) in [
        ("-1", Angle::Up),
        ("-2", Angle::Down),
        ("0", Angle::Yaw(0.)),
        ("360", Angle::Yaw(360.)),
        ("-90", Angle::Yaw(-90.)),
    ] {
        let door = Door::parse(1, &properties(&[("angle", text)]), MODEL_COUNT).unwrap();
        assert_eq!(door.angle, expected);
    }
}

#[test]
fn zero_speed_and_lip_use_defaults_and_empty_targetname_is_absent() {
    let door = Door::parse(
        1,
        &properties(&[("speed", "0"), ("lip", "-0.0"), ("targetname", "")]),
        MODEL_COUNT,
    )
    .unwrap();
    assert_eq!(door.speed, 100.);
    assert_eq!(door.lip, 8.);
    assert_eq!(door.targetname, None);
    // Negative lip is ordinary extra travel, rather than a signed sentinel.
    assert_eq!(
        Door::parse(1, &properties(&[("lip", "-4")]), MODEL_COUNT)
            .unwrap()
            .lip,
        -4.
    );
    assert!(Door::parse(1, &properties(&[("speed", "-1")]), MODEL_COUNT).is_err());
}

#[test]
fn rejects_missing_external_world_and_out_of_bounds_model_references() {
    let mut missing = properties(&[]);
    missing.remove("model");
    assert!(Door::parse(1, &missing, MODEL_COUNT).is_err());
    for model in [
        "",
        "1",
        "door.bsp",
        "*",
        "*-1",
        "*+1",
        "*1.0",
        "*1junk",
        "*0",
        "*4",
        "*999999999999999999999999999999999",
    ] {
        let error = Door::parse(1, &properties(&[("model", model)]), MODEL_COUNT).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData, "{model}");
        assert!(error.to_string().contains("model"), "{model}");
    }
}

#[test]
fn rejects_malformed_and_non_finite_numeric_properties() {
    for key in ["angle", "speed", "lip"] {
        for value in ["", "junk", "10junk", "1 2", "NaN", "inf", "-inf", "1e100"] {
            let error = Door::parse(1, &properties(&[(key, value)]), MODEL_COUNT).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidData);
            assert!(error.to_string().contains(key), "{key}: {value}");
        }
    }
}

#[test]
fn requires_the_correct_classname_and_nonzero_entity_index() {
    assert!(Door::parse(0, &properties(&[]), MODEL_COUNT).is_err());
    assert!(
        Door::parse(
            1,
            &properties(&[("classname", "func_door_secret")]),
            MODEL_COUNT
        )
        .is_err()
    );
    let mut missing = properties(&[]);
    missing.remove("classname");
    assert!(Door::parse(1, &missing, MODEL_COUNT).is_err());
}

#[test]
fn lump_decode_preserves_indices_duplicates_and_unknown_classes() {
    let entities = decode(
        br#"
        {"classname" "worldspawn"}
        {"classname" "func_door" "model" "*1" "speed" "junk" "speed" "50" "angle" "-1"}
        {"classname" "func_door_secret" "model" "*2"}
        {"classname" "func_door" "model" "*3" "targetname" "exit"}
    "#,
    )
    .unwrap();
    assert_eq!(entities.len(), 4);
    let Entity::Door(door) = &entities[1] else {
        panic!("expected door")
    };
    assert_eq!(door.speed, 50.);
    assert_eq!(door.angle, Angle::Up);
    assert_eq!(entities[2], Entity::Unknown);
    let Entity::Door(door) = &entities[3] else {
        panic!("expected door")
    };
    assert_eq!(door.model, 3);
    assert_eq!(door.targetname.as_deref(), Some("exit"));
    assert!(decode(br#"{"classname" "worldspawn"}{"classname" "func_door"}"#).is_err());
}

#[test]
fn bsp_decode_checks_door_references_against_the_actual_model_count() {
    let model_count = Bsp::decode(BSP_BYTES).unwrap().models().len();
    for (model, valid) in [(model_count - 1, true), (model_count, false)] {
        let mut bytes = BSP_BYTES.to_vec();
        let text = format!(
            "{{\"classname\" \"worldspawn\"}}{{\"classname\" \"func_door\" \"model\" \"*{model}\"}}"
        );
        let offset = bytes.len() as i32;
        bytes[4..8].copy_from_slice(&offset.to_le_bytes());
        bytes[8..12].copy_from_slice(&(text.len() as i32).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
        assert_eq!(Bsp::decode(&bytes).is_ok(), valid);
    }
}

#[test]
fn parses_all_seven_regular_doors_in_librequake() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let doors: Vec<_> = bsp
        .entities()
        .iter()
        .filter_map(|entity| match entity {
            Entity::Door(door) => Some(door),
            _ => None,
        })
        .collect();
    assert_eq!(
        doors.iter().map(|door| door.model).collect::<Vec<_>>(),
        [3, 4, 6, 7, 15, 16, 19]
    );
    assert_eq!(doors[0].angle, Angle::Yaw(0.));
    assert_eq!(doors[0].lip, 24.);
    assert_eq!(doors[0].targetname.as_deref(), Some("door_afterkey1"));
    assert_eq!(doors[1].angle, Angle::Yaw(180.));
    assert_eq!(doors[2].angle, Angle::Yaw(90.));
    assert_eq!(doors[3].angle, Angle::Yaw(270.));
    assert_eq!(doors[6].angle, Angle::Up);
    assert_eq!(doors[6].speed, 25.);
    assert_eq!(doors[6].lip, 8.);
    assert_eq!(doors[6].targetname.as_deref(), Some("exit_door"));
}
