use std::io::ErrorKind;

use super::super::visibility::decode;

#[test]
fn expands_rows_to_leaf_indices_and_maps_compressed_offsets() {
    // Seventeen leaves give three bytes per row; bit zero means leaf one.
    let (visibility, ranges) = decode(&[0x81, 0, 1, 0xff, 0, 3, 0, 2, 1], 17).unwrap();
    assert_eq!(visibility, [1, 8, 17, 17]);
    assert_eq!(ranges.len(), 3);
    assert_eq!(ranges[&0], 0..3);
    assert_eq!(ranges[&4], 3..3);
    assert_eq!(ranges[&6], 3..4);
    // Bytes inside a row or its run encoding cannot stand in for row offsets.
    for offset in [1, 2, 3, 5, 7, 8, 9] {
        assert!(!ranges.contains_key(&offset));
    }
    assert_eq!(&visibility[ranges[&0].clone()], &[1, 8, 17]);
    assert_eq!(&visibility[ranges[&6].clone()], &[17]);
}

#[test]
fn preserves_distinct_row_offsets_even_when_rows_are_identical() {
    let (visibility, ranges) = decode(&[0x05, 0x05], 8).unwrap();
    assert_eq!(visibility, [1, 3, 1, 3]);
    assert_eq!(ranges[&0], 0..2);
    assert_eq!(ranges[&1], 2..4);
}

#[test]
fn ignores_padding_bits_and_includes_the_last_visibility_leaf() {
    for count in [1_usize, 7, 8, 9, 16, 17] {
        let row = vec![0xff; count.div_ceil(8)];
        let (visibility, ranges) = decode(&row, count).unwrap();
        assert_eq!(visibility, (1..=count).collect::<Vec<_>>());
        assert_eq!(ranges[&0], 0..count);
    }
}

#[test]
fn supports_zero_runs_longer_than_one_run_record() {
    let (visibility, ranges) = decode(&[0, 255, 0, 1, 1], 2056).unwrap();
    assert_eq!(visibility, [2049]);
    assert_eq!(ranges[&0], 0..1);
}

#[test]
fn preserves_absent_visibility_without_inventing_rows() {
    for count in [0, 9] {
        let (visibility, ranges) = decode(&[], count).unwrap();
        assert!(visibility.is_empty());
        assert!(ranges.is_empty());
    }
    assert!(decode(&[1], 0).is_err());
}

#[test]
fn rejects_truncation_zero_length_runs_and_runs_crossing_row_boundaries() {
    for bytes in [
        &[1][..],
        &[0],
        &[0, 0],
        &[0, 3],
        &[1, 0],
        &[1, 0, 2],
        &[1, 1, 0],
        &[1, 1, 0, 0],
        &[1, 1, 1],
    ] {
        assert_eq!(
            decode(bytes, 9).unwrap_err().kind(),
            ErrorKind::InvalidData,
            "{bytes:?}"
        );
    }
}
