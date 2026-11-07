use std::collections::HashMap;

use super::super::super::Worldspawn;

// Build an entity property map from literal pairs.
fn properties(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|&(key, value)| (key.into(), value.into()))
        .collect()
}

#[test]
fn allows_absent_overrides_and_ignores_unknown_properties() {
    let entity = properties(&[("classname", "worldspawn"), ("unknown", "arbitrary junk")]);
    assert_eq!(
        Worldspawn::parse(0, &entity).unwrap(),
        Worldspawn {
            slime_alpha: None,
            slime_tint: None,
            slime_tint_strength: None,
            water_alpha: None,
            water_tint: None,
            water_tint_strength: None,
            sky_foreground_speed: None,
            sky_background_speed: None,
        }
    );
}

#[test]
fn parses_independent_overrides_including_stationary_and_reversed_sky() {
    for (key, value, expected) in [
        ("_wateralpha", " 0.5 ", [Some(0.5), None, None]),
        ("_sky_foreground_speed", "-16", [None, Some(-16.), None]),
        ("_sky_background_speed", "0", [None, None, Some(0.)]),
    ] {
        let entity = properties(&[("classname", "worldspawn"), (key, value)]);
        let world = Worldspawn::parse(0, &entity).unwrap();
        assert_eq!(
            [
                world.water_alpha,
                world.sky_foreground_speed,
                world.sky_background_speed
            ],
            expected
        );
    }
}

#[test]
fn parses_water_tint_overrides_independently() {
    let entity = properties(&[
        ("classname", "worldspawn"),
        ("_water_tint", " 0.1  0.5\t1.0 "),
    ]);
    let world = Worldspawn::parse(0, &entity).unwrap();
    assert_eq!(world.water_tint, Some([0.1, 0.5, 1.]));
    assert_eq!(world.water_tint_strength, None);
    for strength in ["0", "0.4", "1"] {
        let entity = properties(&[
            ("classname", "worldspawn"),
            ("_water_tint_strength", strength),
        ]);
        let world = Worldspawn::parse(0, &entity).unwrap();
        assert_eq!(world.water_tint, None);
        assert_eq!(world.water_tint_strength, Some(strength.parse().unwrap()));
    }
}

#[test]
fn parses_slime_overrides_without_changing_water() {
    for (key, value) in [
        ("_slimealpha", "0.25"),
        ("_slime_tint", "0.1 0.8 0.2"),
        ("_slime_tint_strength", "0.7"),
    ] {
        let entity = properties(&[("classname", "worldspawn"), (key, value)]);
        let world = Worldspawn::parse(0, &entity).unwrap();
        assert_eq!(world.slime_alpha, (key == "_slimealpha").then_some(0.25));
        assert_eq!(
            world.slime_tint,
            (key == "_slime_tint").then_some([0.1, 0.8, 0.2])
        );
        assert_eq!(
            world.slime_tint_strength,
            (key == "_slime_tint_strength").then_some(0.7)
        );
        assert_eq!(world.water_alpha, None);
        assert_eq!(world.water_tint, None);
        assert_eq!(world.water_tint_strength, None);
    }
}

#[test]
fn rejects_malformed_tints_and_out_of_range_strengths() {
    for value in [
        "",
        "0 1",
        "0 1 0 1",
        "red green blue",
        "NaN 0 1",
        "0 inf 1",
        "-0.1 0 1",
        "0 1.1 0",
    ] {
        for key in ["_water_tint", "_slime_tint"] {
            let entity = properties(&[("classname", "worldspawn"), (key, value)]);
            assert!(Worldspawn::parse(0, &entity).is_err(), "{key}: {value}");
        }
    }
    for value in ["-0.1", "1.1"] {
        for key in ["_water_tint_strength", "_slime_tint_strength"] {
            let entity = properties(&[("classname", "worldspawn"), (key, value)]);
            assert!(Worldspawn::parse(0, &entity).is_err());
        }
    }
}

#[test]
fn requires_worldspawn_classname_and_entity_zero() {
    for entity in [properties(&[]), properties(&[("classname", "func_door")])] {
        assert!(Worldspawn::parse(0, &entity).is_err());
    }
    let entity = properties(&[("classname", "worldspawn")]);
    for index in [1, usize::MAX] {
        assert!(Worldspawn::parse(index, &entity).is_err());
    }
}

#[test]
fn rejects_invalid_known_overrides() {
    for key in [
        "_wateralpha",
        "_slimealpha",
        "_slime_tint_strength",
        "_water_tint_strength",
        "_sky_foreground_speed",
        "_sky_background_speed",
    ] {
        for value in ["", "junk", "NaN", "inf", "-inf", "1e100"] {
            let entity = properties(&[("classname", "worldspawn"), (key, value)]);
            let error = Worldspawn::parse(0, &entity).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert!(error.to_string().contains(key));
        }
    }
}
