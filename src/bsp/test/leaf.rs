use std::collections::HashMap;

use super::super::Contents;
use super::super::leaf;

// Decode with enough marksurfaces for the fixture's unsigned range.
fn decode(
    bytes: &[u8],
    map: &HashMap<usize, std::ops::Range<usize>>,
) -> std::io::Result<Vec<leaf::Leaf>> {
    leaf::decode(bytes, map, 65537)
}

const RECORD_SIZE: usize = 28;

// Build a leaf with distinct bounds, a marksurface range, and all ambient channels.
fn record(contents: i32, offset: i32) -> [u8; RECORD_SIZE] {
    let mut bytes = [0; RECORD_SIZE];
    bytes[..4].copy_from_slice(&contents.to_le_bytes());
    bytes[4..8].copy_from_slice(&offset.to_le_bytes());
    for (value, component) in [i16::MIN, -20, -30, 40, 50, i16::MAX]
        .into_iter()
        .zip(bytes[8..20].chunks_exact_mut(2))
    {
        component.copy_from_slice(&value.to_le_bytes());
    }
    bytes[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
    bytes[22..24].copy_from_slice(&2_u16.to_le_bytes());
    bytes[24..28].copy_from_slice(&[1, 2, 3, 255]);
    bytes
}

#[test]
fn preserves_leaf_fields_and_resolves_pvs_ranges() {
    let map = HashMap::from([(7, 2..5)]);
    let leaf = decode(&record(-3, 7), &map).unwrap().remove(0);
    assert_eq!(leaf.contents, Contents::Water);
    assert_eq!(leaf.pvs, Some(2..5));
    assert_eq!(leaf.marksurfaces, 65535..65537);
}

#[test]
fn distinguishes_missing_pvs_from_an_empty_visible_set() {
    let map = HashMap::from([(0, 4..4)]);
    let mut bytes = record(-2, -1).to_vec();
    bytes.extend_from_slice(&record(-1, 0));
    let leaves = decode(&bytes, &map).unwrap();
    assert_eq!(leaves[0].contents, Contents::Solid);
    assert_eq!(leaves[0].pvs, None);
    assert_eq!(leaves[1].pvs, Some(4..4));
}

#[test]
fn rejects_missing_pvs_rows_negative_offsets_and_unsupported_contents() {
    let map = HashMap::from([(0, 0..1)]);
    for offset in [-2_i32, i32::MIN, 1, i32::MAX] {
        assert!(decode(&record(-1, offset), &map).is_err());
    }
    for contents in [0_i32, -7, -9, i32::MIN] {
        assert!(decode(&record(contents, -1), &map).is_err());
    }
}

#[test]
fn accepts_empty_lumps_and_rejects_partial_records() {
    assert!(decode(&[], &HashMap::new()).unwrap().is_empty());
    for length in 1..RECORD_SIZE {
        assert!(decode(&vec![0; length], &HashMap::new()).is_err());
    }
}
