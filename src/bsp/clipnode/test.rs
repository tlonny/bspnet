use super::super::Contents;
use super::super::clipnode::{self, Child};

// Decode with the fixture's ten available planes.
fn decode(bytes: &[u8]) -> std::io::Result<Vec<clipnode::Clipnode>> {
    clipnode::decode(bytes, 10)
}

// Serialize clipnode records with explicit plane and signed child fields.
fn records(nodes: &[(i32, i16, i16)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for &(plane, front, back) in nodes {
        bytes.extend_from_slice(&plane.to_le_bytes());
        bytes.extend_from_slice(&front.to_le_bytes());
        bytes.extend_from_slice(&back.to_le_bytes());
    }
    bytes
}

#[test]
fn preserves_order_and_shared_children() {
    let nodes = decode(&records(&[(7, 2, 1), (8, 2, -2), (9, -1, -3)])).unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].plane, 7);
    assert_eq!(nodes[0].front, Child::Clipnode(2));
    assert_eq!(nodes[0].back, Child::Clipnode(1));
    assert_eq!(nodes[1].plane, 8);
    assert_eq!(nodes[1].front, Child::Clipnode(2));
    assert_eq!(nodes[1].back, Child::Contents(Contents::Solid));
    assert_eq!(nodes[2].plane, 9);
    assert_eq!(nodes[2].front, Child::Contents(Contents::Empty));
    assert_eq!(nodes[2].back, Child::Contents(Contents::Water));
}

#[test]
fn decodes_supported_contents_and_rejects_other_codes_in_both_children() {
    for (raw, expected) in [
        (-1_i16, Contents::Empty),
        (-2, Contents::Solid),
        (-3, Contents::Water),
        (-4, Contents::Slime),
        (-5, Contents::Lava),
        (-6, Contents::Sky),
        (-8, Contents::Clip),
    ] {
        let nodes = decode(&records(&[(0, raw, raw)])).unwrap();
        assert_eq!(nodes[0].front, Child::Contents(expected));
        assert_eq!(nodes[0].back, Child::Contents(expected));
    }
    for raw in [-7_i16, -9, -10, -11, -12, -13, -14, -15, -16, i16::MIN] {
        for node in [(0, raw, -1), (0, -1, raw)] {
            assert_eq!(
                decode(&records(&[node])).unwrap_err().to_string(),
                "invalid contents"
            );
        }
    }
}

#[test]
fn rejects_missing_children_in_unreferenced_records_and_negative_planes() {
    for raw in [2_i16, i16::MAX] {
        for node in [(0, raw, -1), (0, -1, raw)] {
            assert_eq!(
                decode(&records(&[(0, -1, -2), node]))
                    .unwrap_err()
                    .to_string(),
                "clipnode references missing clipnode"
            );
        }
    }
    for plane in [-1_i32, i32::MIN] {
        assert_eq!(
            decode(&records(&[(plane, -1, -2)]))
                .unwrap_err()
                .to_string(),
            "negative clipnode plane index"
        );
    }
}

#[test]
fn accepts_empty_lumps_and_rejects_partial_records() {
    assert!(decode(&[]).unwrap().is_empty());
    for length in 1..8 {
        assert!(decode(&vec![0; length]).is_err());
    }
}
