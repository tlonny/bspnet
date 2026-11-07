use super::super::model::decode;
use super::super::{ClipRoot, Contents, RenderRoot};

const RECORD_SIZE: usize = 64;

// Build a model with distinct vectors, four hull roots, and a nonzero face offset.
fn record() -> [u8; RECORD_SIZE] {
    let mut bytes = [0; RECORD_SIZE];
    for (value, component) in [-10.5_f32, -20.0, -30.0, 40.0, 50.0, 60.25, 1.0, 2.5, -3.0]
        .into_iter()
        .zip(bytes[..36].chunks_exact_mut(4))
    {
        component.copy_from_slice(&value.to_le_bytes());
    }
    for (value, field) in [7_i32, 11, -1, -2, 9, 2, 3]
        .into_iter()
        .zip(bytes[36..].chunks_exact_mut(4))
    {
        field.copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[test]
fn preserves_model_roots_visibility_count_and_face_range() {
    let models = decode(&record(), 5, 8, 12).unwrap();
    assert_eq!(models.len(), 1);
    let model = &models[0];
    assert_eq!(model.render_root, RenderRoot::Node(7));
    assert_eq!(
        model.clip_roots,
        [
            ClipRoot::Clipnode(11),
            ClipRoot::Contents(Contents::Empty),
            ClipRoot::Contents(Contents::Solid),
        ]
    );
    assert_eq!(model.visleafs, 9);
    assert_eq!(model.faces, 2..5);
}

#[test]
fn unpacks_render_leaf_roots_without_signed_overflow() {
    for (raw, expected) in [
        (0_i32, RenderRoot::Node(0)),
        (-1, RenderRoot::Leaf(0)),
        (-4, RenderRoot::Leaf(3)),
        (i32::MIN, RenderRoot::Leaf(i32::MAX as usize)),
    ] {
        let mut bytes = record();
        bytes[36..40].copy_from_slice(&raw.to_le_bytes());
        assert_eq!(decode(&bytes, 5, 8, 12).unwrap()[0].render_root, expected);
    }
}

#[test]
fn unpacks_contents_for_every_collision_hull() {
    for (raw, contents) in [
        (-1_i32, Contents::Empty),
        (-2, Contents::Solid),
        (-3, Contents::Water),
        (-4, Contents::Slime),
        (-5, Contents::Lava),
        (-6, Contents::Sky),
        (-8, Contents::Clip),
    ] {
        for hull in 0..3 {
            let mut bytes = record();
            let offset = 40 + hull * 4;
            bytes[offset..offset + 4].copy_from_slice(&raw.to_le_bytes());
            assert_eq!(
                decode(&bytes, 5, 8, 12).unwrap()[0].clip_roots[hull],
                ClipRoot::Contents(contents)
            );
        }
    }
    for hull in 0..3 {
        for raw in [
            0_i32,
            i32::MAX,
            -7,
            -9,
            -10,
            -11,
            -12,
            -13,
            -14,
            -15,
            -16,
            i32::MIN,
        ] {
            let mut bytes = record();
            let offset = 40 + hull * 4;
            bytes[offset..offset + 4].copy_from_slice(&raw.to_le_bytes());
            let result = decode(&bytes, 5, 8, usize::MAX);
            if raw >= 0 {
                assert_eq!(
                    result.unwrap()[0].clip_roots[hull],
                    ClipRoot::Clipnode(raw as usize)
                );
            } else {
                assert_eq!(result.unwrap_err().to_string(), "invalid contents");
            }
        }
    }
}

#[test]
fn validates_model_counts_and_face_ranges() {
    for (start, count, valid) in [
        (0_i32, 5_i32, true),
        (5, 0, true),
        (6, 0, false),
        (4, 2, false),
        (-1, 0, false),
        (0, -1, false),
        (i32::MAX, i32::MAX, false),
    ] {
        let mut bytes = record();
        bytes[56..60].copy_from_slice(&start.to_le_bytes());
        bytes[60..64].copy_from_slice(&count.to_le_bytes());
        assert_eq!(decode(&bytes, 5, 8, 12).is_ok(), valid, "{start}, {count}");
    }
    for count in [-1_i32, i32::MIN] {
        let mut bytes = record();
        bytes[52..56].copy_from_slice(&count.to_le_bytes());
        assert_eq!(
            decode(&bytes, 5, 8, 12).unwrap_err().to_string(),
            "negative model visibility leaf count"
        );
    }
    let mut bytes = [0; RECORD_SIZE];
    bytes[52..56].copy_from_slice(&i32::MAX.to_le_bytes());
    assert_eq!(
        decode(&bytes, 0, 8, 12).unwrap()[0].visleafs,
        i32::MAX as usize
    );
}

#[test]
fn accepts_empty_lumps_and_rejects_partial_records() {
    assert!(decode(&[], 0, 8, 12).unwrap().is_empty());
    for length in 1..RECORD_SIZE {
        assert!(decode(&vec![0; length], 0, 8, 12).is_err());
        assert!(decode(&vec![0; RECORD_SIZE + length], 0, 8, 12).is_err());
    }
}

#[test]
fn validates_render_node_roots_but_retains_leaf_roots() {
    for (root, valid) in [
        (0_i32, true),
        (7, true),
        (8, false),
        (i32::MAX, false),
        (-1, true),
    ] {
        let mut bytes = record();
        bytes[36..40].copy_from_slice(&root.to_le_bytes());
        assert_eq!(decode(&bytes, 5, 8, 12).is_ok(), valid, "{root}");
    }
    let mut bytes = record();
    bytes[36..40].copy_from_slice(&0_i32.to_le_bytes());
    assert_eq!(
        decode(&bytes, 5, 0, 12).unwrap_err().to_string(),
        "model references missing node"
    );
}
