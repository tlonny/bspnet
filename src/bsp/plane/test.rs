use super::super::plane::decode;

// Encode a plane with distinct normal and distance values.
fn record(kind: i32) -> Vec<u8> {
    let mut bytes: Vec<_> = [0.25_f32, -0.5, 0.75, -32.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes
}

#[test]
fn preserves_plane_equations_and_ignores_unused_classifications() {
    for kind in [-1, 0, 1, 2, 3, 4, 5, 6, i32::MIN, i32::MAX] {
        let plane = decode(&record(kind)).unwrap()[0];
        assert_eq!(plane.normal, [0.25, -0.5, 0.75]);
        assert_eq!(plane.distance, -32.0);
    }
}

#[test]
fn rejects_nonfinite_values_and_partial_records() {
    assert!(decode(&[]).unwrap().is_empty());
    for field in [0, 4, 8, 12] {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut bytes = record(0);
            bytes[field..field + 4].copy_from_slice(&value.to_le_bytes());
            assert!(decode(&bytes).is_err());
        }
    }
    for length in 1..20 {
        assert!(decode(&record(0)[..length]).is_err());
    }
}
