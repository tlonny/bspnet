use crate::bsp::{Angle, Data as Bsp, Entity};
use crate::level::STEP;
use crate::level::state::{Button, Door, Motion, State};

const BSP_BYTES: &[u8] = include_bytes!("../../../assets/maps/librequake/lq_e0m1.bsp");

// Select the same brush model for movement checks while keeping the BSP immutable.
fn door(bsp: &Bsp) -> Door {
    let config = bsp
        .entities()
        .iter()
        .find_map(|entity| match entity {
            Entity::Door(config) if config.model == 3 => Some(config),
            _ => None,
        })
        .unwrap();
    Door::build(config)
}

#[test]
fn movement_uses_fixed_ticks_and_stops_at_both_endpoints() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut door = door(&bsp);
    door.angle = Angle::Up;
    door.lip = 8.;
    door.speed = 100.;
    door.activate();
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0., 0., 1.]);
    assert_eq!(door.motion, Motion::Opening);

    door.speed = 100_000.;
    door.update(&bsp, 3);
    // The compiled bounds span 126 units vertically, and the lip is eight units.
    assert_eq!(door.translation, [0., 0., 118.]);
    assert_eq!(door.motion, Motion::Open);
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0., 0., 118.]);

    door.motion = Motion::Closing;
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0.; 3]);
    assert_eq!(door.motion, Motion::Closed);
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0.; 3]);
}

#[test]
fn diagonal_travel_uses_both_horizontal_extents() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut door = door(&bsp);
    door.angle = Angle::Yaw(135.);
    door.lip = 0.;
    door.speed = 100_000.;
    door.activate();
    door.update(&bsp, 3);
    // Projecting the 78 by 14 unit bounds at 135 degrees gives (-46, 46).
    let expected = glam::Vec3::new(-46., 46., 0.);
    assert!(glam::Vec3::from(door.translation).abs_diff_eq(expected, 0.0001));
}

#[test]
fn scheduled_return_counts_down_before_closing() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut door = door(&bsp);
    door.translation = [0., 0., 10.];
    door.motion = Motion::Open;
    door.return_in = Some(STEP.as_secs_f32() * 2.);
    door.update(&bsp, 3);
    assert_eq!(door.return_in, Some(STEP.as_secs_f32()));
    assert_eq!(door.motion, Motion::Open);
    door.update(&bsp, 3);
    assert_eq!(door.return_in, None);
    assert_eq!(door.motion, Motion::Closing);
    assert_eq!(door.translation, [0., 0., 10.]);
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0., 0., 9.]);
}

#[test]
fn movement_can_reverse_from_its_current_position() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut door = door(&bsp);
    door.angle = Angle::Down;
    door.speed = 100.;
    door.activate();
    door.update(&bsp, 3);
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0., 0., -2.]);
    door.motion = Motion::Closing;
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0., 0., -1.]);
    door.activate();
    door.update(&bsp, 3);
    assert_eq!(door.translation, [0., 0., -2.]);
}

#[test]
fn initializes_states_in_entity_order_with_independent_doors() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut states: Vec<_> = bsp.entities().iter().map(State::build).collect();
    assert_eq!(states.len(), bsp.entities().len());
    assert_eq!(states[0], State::Empty);
    let mut doors = Vec::new();
    for (index, entity) in bsp.entities().iter().enumerate() {
        match entity {
            Entity::Door(config) => {
                let State::Door(door) = &states[index] else {
                    panic!("missing runtime door state");
                };
                assert_eq!(door.translation, [0.; 3]);
                assert_eq!(door.angle, config.angle);
                assert_eq!(door.speed, config.speed);
                assert_eq!(door.lip, config.lip);
                assert_eq!(door.motion, Motion::Closed);
                assert_eq!(door.return_in, None);
                doors.push(index);
            }
            Entity::Button(_) => {
                let State::Button(button) = &states[index] else {
                    panic!("missing runtime button state");
                };
                assert!(!button.is_pressed());
            }
            Entity::TriggerOnce(_) => {
                assert!(matches!(states[index], State::TriggerOnce(_)));
            }
            Entity::TriggerMultiple(_) => {
                assert!(matches!(states[index], State::TriggerMultiple(_)));
            }
            Entity::TriggerRelay(_) => {
                assert!(matches!(states[index], State::TriggerRelay(_)));
            }
            Entity::Unknown
            | Entity::Worldspawn(_)
            | Entity::Wall(_)
            | Entity::Illusionary(_)
            | Entity::Portal(_) => {
                assert_eq!(states[index], State::Empty)
            }
        }
    }
    assert_eq!(doors.len(), 7);
    let Entity::Door(config) = &bsp.entities()[doors[0]] else {
        unreachable!()
    };
    let original = (config.angle, config.speed, config.lip);
    let State::Door(door) = &mut states[doors[0]] else {
        unreachable!()
    };
    door.translation = [12., 0., 0.];
    door.angle = Angle::Down;
    door.speed = 25.;
    door.lip = 4.;
    door.motion = Motion::Opening;
    door.return_in = Some(3.);
    assert_eq!((config.angle, config.speed, config.lip), original);
    for &index in &doors[1..] {
        let State::Door(door) = &states[index] else {
            unreachable!()
        };
        let Entity::Door(config) = &bsp.entities()[index] else {
            unreachable!()
        };
        assert_eq!(door.translation, [0.; 3]);
        assert_eq!(door.angle, config.angle);
        assert_eq!(door.speed, config.speed);
        assert_eq!(door.lip, config.lip);
        assert_eq!(door.motion, Motion::Closed);
        assert_eq!(door.return_in, None);
    }
}

#[test]
fn button_state_is_independent_of_configuration_and_other_buttons() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut states: Vec<_> = bsp.entities().iter().map(State::build).collect();
    let buttons: Vec<_> = bsp
        .entities()
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| matches!(entity, Entity::Button(_)).then_some(index))
        .collect();
    assert_eq!(buttons.len(), 2);
    let State::Button(button) = &mut states[buttons[0]] else {
        unreachable!()
    };
    button.press();
    assert!(button.is_pressed());
    let Entity::Button(config) = &bsp.entities()[buttons[0]] else {
        unreachable!()
    };
    assert_eq!(config.wait, Some(0.05));
    let State::Button(other) = &states[buttons[1]] else {
        unreachable!()
    };
    assert!(!other.is_pressed());
}

#[test]
fn button_switches_immediately_and_resets_after_its_wait() {
    let config = crate::bsp::Button {
        model: 3,
        alpha: None,
        target: None,
        wait: Some(STEP.as_secs_f32() * 2.),
    };
    let mut button = Button::build(&config);
    button.update();
    assert!(!button.is_pressed());
    button.press();
    assert!(button.is_pressed());
    button.update();
    assert!(button.is_pressed());
    button.press();
    button.update();
    assert!(!button.is_pressed());
    button.press();
    assert!(button.is_pressed());
}

#[test]
fn button_with_no_return_wait_stays_pressed() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let config = bsp
        .entities()
        .iter()
        .find_map(|entity| match entity {
            Entity::Button(button) if button.model == 18 => Some(button),
            _ => None,
        })
        .unwrap();
    let mut button = Button::build(config);
    button.press();
    for _ in 0..200 {
        button.update();
    }
    assert!(button.is_pressed());
}

#[test]
fn activating_an_open_or_opening_door_does_not_restart_it() {
    let bsp = Bsp::decode(BSP_BYTES).unwrap();
    let mut door = door(&bsp);
    door.activate();
    door.update(&bsp, 3);
    let translation = door.translation;
    door.activate();
    assert_eq!(door.translation, translation);
    assert_eq!(door.motion, Motion::Opening);
    door.motion = Motion::Open;
    door.return_in = Some(2.);
    door.activate();
    assert_eq!(door.motion, Motion::Open);
    assert_eq!(door.return_in, Some(2.));
}

#[test]
fn trigger_once_fires_immediately_or_once_after_its_delay() {
    for (delay, ticks) in [(0.0, 0), (0.001, 1), (0.05, 5)] {
        let entity = Entity::TriggerOnce(crate::bsp::TriggerOnce {
            model: 1,
            target: Some("airlock".into()),
            targetname: None,
            delay,
        });
        let State::TriggerOnce(mut trigger) = State::build(&entity) else {
            panic!("missing runtime trigger state");
        };
        assert!(!trigger.update());
        assert_eq!(trigger.activate(), ticks == 0);
        for tick in 1..=ticks {
            assert!(!trigger.activate());
            assert_eq!(trigger.update(), tick == ticks);
        }
        assert!(!trigger.activate());
        assert!(!trigger.update());
    }
}

#[test]
fn trigger_multiple_cooldown_allows_a_second_output_while_the_first_is_pending() {
    let entity = Entity::TriggerMultiple(crate::bsp::TriggerMultiple {
        model: 1,
        target: Some("airlock".into()),
        targetname: None,
        delay: 0.03,
        wait: 0.02,
    });
    let State::TriggerMultiple(mut trigger) = State::build(&entity) else {
        panic!("missing runtime trigger state");
    };
    assert!(!trigger.activate());
    assert!(!trigger.activate());
    assert_eq!(trigger.update(), 0);
    assert!(!trigger.activate());
    assert_eq!(trigger.update(), 0);
    assert!(!trigger.activate());
    assert_eq!(trigger.update(), 1);
    assert_eq!(trigger.update(), 0);
    assert_eq!(trigger.update(), 1);
    assert_eq!(trigger.update(), 0);
}

#[test]
fn trigger_relay_preserves_pending_inputs_and_consumes_one_shots_on_activation() {
    for (once, outputs) in [(false, 2), (true, 1)] {
        let entity = Entity::TriggerRelay(crate::bsp::TriggerRelay {
            target: Some("airlock".into()),
            targetname: Some("entrance".into()),
            delay: 0.02,
            once,
        });
        let State::TriggerRelay(mut relay) = State::build(&entity) else {
            panic!("missing runtime relay state");
        };
        assert!(!relay.activate());
        assert!(!relay.activate());
        assert_eq!(relay.update(), 0);
        assert_eq!(relay.update(), outputs);
        assert!(!relay.activate());
        assert_eq!(relay.update(), 0);
        assert_eq!(relay.update(), usize::from(!once));
    }
}
