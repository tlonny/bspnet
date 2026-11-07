use super::super::Entity;
use super::super::entity::decode;

mod worldspawn;

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
