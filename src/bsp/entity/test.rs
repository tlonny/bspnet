use std::sync::OnceLock;

use super::super::entity;
use super::super::{Data, Entity, PortalValidation};

// Decode entity text against the fixture's inline brush models.
pub fn decode(bytes: &[u8]) -> std::io::Result<Vec<Entity>> {
    static BSP: OnceLock<Data> = OnceLock::new();
    let bsp = BSP.get_or_init(|| {
        Data::decode(include_bytes!(
            "../../../assets/maps/librequake/lq_e0m1.bsp"
        ))
        .unwrap()
    });
    entity::decode(bytes, bsp)
}

#[test]
fn invalid_portal_properties_leave_the_map_loadable() {
    for (properties, expected_model, diagnostic) in [
        (r#""uri" "other.bsp""#, None, "model"),
        (r#""model" "*1" "uri" "two words""#, Some(1), "uri"),
    ] {
        let text =
            format!(r#"{{"classname" "worldspawn"}} {{"classname" "func_portal" {properties}}}"#);
        let entities = decode(text.as_bytes()).unwrap();
        let Entity::Portal(portal) = &entities[1] else {
            panic!("expected a portal");
        };
        assert_eq!(portal.model, expected_model);
        assert!(
            matches!(&portal.validation, PortalValidation::Invalid(reason)
            if reason.contains(diagnostic))
        );
    }
}

#[test]
fn brush_entities_share_optional_alpha_semantics() {
    for classname in ["func_door", "func_button", "func_wall", "func_illusionary"] {
        for (key, expected) in [
            ("", 1.0),
            (r#""alpha" "0.35""#, 0.35),
            (r#""alpha" "0""#, 0.0),
        ] {
            let text = format!(
                r#"{{"classname" "worldspawn"}} {{"classname" "{classname}" "model" "*1" {key}}}"#
            );
            assert_eq!(
                decode(text.as_bytes()).unwrap()[1].brush(),
                Some((1, expected))
            );
        }
        for invalid in ["-0.1", "1.1", "NaN"] {
            let text = format!(
                r#"{{"classname" "worldspawn"}} {{"classname" "{classname}" "model" "*1" "alpha" "{invalid}"}}"#
            );
            assert!(decode(text.as_bytes()).is_err());
        }
    }
}

#[test]
fn preserves_entity_order_and_parses_recognised_fields() {
    let entities = decode(
        br#"// Map properties
        { "classname" "worldspawn" "message" "A map with spaces { and }"
          "_wateralpha" "0.5" }
        { "classname" "info_player_start" "origin" "128 256 64" }"#,
    )
    .unwrap();
    assert_eq!(entities.len(), 2);
    let Entity::Worldspawn(world) = &entities[0] else {
        panic!("expected worldspawn")
    };
    assert_eq!(world.water_alpha, Some(0.5));
    assert_eq!(entities[1], Entity::Unknown);
}

#[test]
fn handles_comments_empty_values_duplicates_and_literal_backslashes() {
    let entities = decode(
        br#"{"classname" "worldspawn" "_wateralpha" "junk" "_wateralpha" "0.5"}
        {"classname" "custom" "message" // before value
        "old" "message" "line\n//literal" "empty" ""}// end
        "#,
    )
    .unwrap();
    let Entity::Worldspawn(world) = &entities[0] else {
        panic!("expected worldspawn")
    };
    assert_eq!(world.water_alpha, Some(0.5));
    assert_eq!(entities[1], Entity::Unknown);
    assert_eq!(
        decode(b"{\"classname\" \"worldspawn\"}\0").unwrap().len(),
        1
    );
}

#[test]
fn rejects_malformed_entities_without_panicking() {
    for bytes in [
        b"{".as_slice(),
        b"}",
        b"{\"key\"}",
        b"{\"key\" \"unterminated}",
        b"{ key \"value\" }",
        b"{{}}",
        b"{}garbage",
        b"{}\0{}",
        b"{\"key\" \"\xff\"}",
    ] {
        assert_eq!(
            decode(bytes).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn requires_worldspawn_first_and_rejects_duplicates_and_missing_classnames() {
    for bytes in [
        b"".as_slice(),
        b"{}",
        b"{\"classname\" \"func_door\"}",
        b"{\"classname\" \"worldspawn\"}{\"classname\" \"worldspawn\"}",
        b"{\"classname\" \"worldspawn\"}{}",
        b"{\"classname\" \"worldspawn\" \"_wateralpha\" \"junk\"}",
    ] {
        assert!(decode(bytes).is_err());
    }
}
