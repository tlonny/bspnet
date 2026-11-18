use crate::bsp::sidecar::lit;

#[test]
fn borrows_version_one_lit_samples_after_the_header() {
    let bytes = [b'Q', b'L', b'I', b'T', 1, 0, 0, 0, 1, 2, 3, 254, 128, 0];
    let samples = lit::decode(&bytes).unwrap();
    assert_eq!(samples, [[1, 2, 3], [254, 128, 0]]);
    assert_eq!(samples.as_ptr().cast::<u8>(), bytes[8..].as_ptr());
    assert!(lit::decode(&bytes[..8]).unwrap().is_empty());
}

#[test]
fn rejects_bad_lit_headers_versions_and_partial_samples() {
    let valid = [b'Q', b'L', b'I', b'T', 1, 0, 0, 0, 1, 2, 3];
    for end in 0..8 {
        assert!(lit::decode(&valid[..end]).is_err());
    }
    for end in [9, 10] {
        assert!(lit::decode(&valid[..end]).is_err());
    }
    let mut bytes = valid;
    bytes[0] = b'X';
    assert!(lit::decode(&bytes).is_err());
    for version in [0_u32, 2, u32::MAX] {
        let mut bytes = valid;
        bytes[4..8].copy_from_slice(&version.to_le_bytes());
        assert!(lit::decode(&bytes).is_err());
    }
}
