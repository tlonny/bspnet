use super::super::node::{self, Child};

// Decode with the fixture's eight available planes.
fn decode(bytes: &[u8], face_count: usize) -> std::io::Result<Vec<node::Node>> {
    node::decode(bytes, face_count, 8)
}

const RECORD_SIZE: usize = 24;

// Build a render node with distinct bounds and a nonzero face offset.
fn record(front: i16, back: i16) -> [u8; RECORD_SIZE] {
    let mut bytes = [0; RECORD_SIZE];
    bytes[..4].copy_from_slice(&7_i32.to_le_bytes());
    bytes[4..6].copy_from_slice(&front.to_le_bytes());
    bytes[6..8].copy_from_slice(&back.to_le_bytes());
    for (value, component) in [i16::MIN, -20, -30, 40, 50, i16::MAX]
        .into_iter()
        .zip(bytes[8..20].chunks_exact_mut(2))
    {
        component.copy_from_slice(&value.to_le_bytes());
    }
    bytes[20..22].copy_from_slice(&2_u16.to_le_bytes());
    bytes[22..24].copy_from_slice(&3_u16.to_le_bytes());
    bytes
}

#[test]
fn preserves_plane_and_explicit_child_references() {
    let mut bytes = record(1, -1).to_vec();
    bytes.extend_from_slice(&record(-2, i16::MIN));
    let nodes = decode(&bytes, 5).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].plane, 7);
    assert_eq!(nodes[0].front, Child::Node(1));
    assert_eq!(nodes[0].back, Child::Leaf(0));
    assert_eq!(nodes[1].front, Child::Leaf(1));
    assert_eq!(nodes[1].back, Child::Leaf(i16::MAX as usize));
}

#[test]
fn validates_both_node_children_and_rejects_negative_plane_indices() {
    for index in [1_i16, i16::MAX] {
        for (front, back) in [(index, -1), (-1, index)] {
            assert_eq!(
                decode(&record(front, back), 5).unwrap_err().to_string(),
                "node references missing node"
            );
        }
    }
    for plane in [-1_i32, i32::MIN] {
        let mut bytes = record(-1, -1);
        bytes[..4].copy_from_slice(&plane.to_le_bytes());
        assert_eq!(
            decode(&bytes, 5).unwrap_err().to_string(),
            "negative node plane index"
        );
    }
}

#[test]
fn validates_unsigned_face_ranges_including_empty_ranges() {
    for (start, count, valid) in [
        (0_u16, 5_u16, true),
        (5, 0, true),
        (6, 0, false),
        (4, 2, false),
    ] {
        let mut bytes = record(-1, -1);
        bytes[20..22].copy_from_slice(&start.to_le_bytes());
        bytes[22..24].copy_from_slice(&count.to_le_bytes());
        assert_eq!(decode(&bytes, 5).is_ok(), valid, "{start}, {count}");
    }
    let mut bytes = record(-1, -1);
    bytes[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
    bytes[22..24].copy_from_slice(&1_u16.to_le_bytes());
    assert!(decode(&bytes, 65536).is_ok());
}

#[test]
fn accepts_empty_lumps_and_rejects_partial_records() {
    assert!(decode(&[], 0).unwrap().is_empty());
    for length in 1..RECORD_SIZE {
        assert!(decode(&vec![0; length], 0).is_err());
    }
}
