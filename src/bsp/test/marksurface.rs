use super::super::marksurface::decode;

#[test]
fn preserves_shared_faces_and_unsigned_indices_in_lump_order() {
    let bytes: Vec<_> = [2_u16, 0, 2, u16::MAX]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(decode(&bytes, 65536).unwrap(), [2, 0, 2, 65535]);
}

#[test]
fn rejects_missing_faces_and_partial_records() {
    assert!(decode(&[], 0).unwrap().is_empty());
    assert!(decode(&[0], 1).is_err());
    assert!(decode(&0_u16.to_le_bytes(), 0).is_err());
    assert!(decode(&3_u16.to_le_bytes(), 3).is_err());
}
