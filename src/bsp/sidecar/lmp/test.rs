use crate::bsp::sidecar::lmp;

#[test]
fn borrows_all_256_palette_colors_without_a_header() {
    let mut bytes = [0; 768];
    bytes[..3].copy_from_slice(&[1, 2, 3]);
    bytes[765..].copy_from_slice(&[254, 128, 0]);
    let palette = lmp::decode(&bytes).unwrap();
    assert_eq!(palette[0], [1, 2, 3]);
    assert_eq!(palette[255], [254, 128, 0]);
    assert_eq!(palette.as_ptr().cast::<u8>(), bytes.as_ptr());
    for size in [0, 1, 765, 767, 769, 1024] {
        assert!(lmp::decode(&vec![0; size]).is_err());
    }
}
