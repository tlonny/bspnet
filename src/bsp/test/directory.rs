use super::super::directory::decode;

const VERSION: i32 = 29;
const INTEGER_SIZE: usize = 4;
const ENTRY_SIZE: usize = 8;
const LUMP_COUNT: usize = 15;
const HEADER_SIZE: usize = INTEGER_SIZE + ENTRY_SIZE * LUMP_COUNT;

// Place each lump's bytes at a distinct offset after the header.
fn file() -> Vec<u8> {
    let mut bytes = vec![0; HEADER_SIZE];
    bytes[..INTEGER_SIZE].copy_from_slice(&VERSION.to_le_bytes());
    for index in 0..LUMP_COUNT {
        let entry = INTEGER_SIZE + index * ENTRY_SIZE;
        let offset = bytes.len() as i32;
        bytes[entry..entry + INTEGER_SIZE].copy_from_slice(&offset.to_le_bytes());
        bytes[entry + INTEGER_SIZE..entry + ENTRY_SIZE].copy_from_slice(&1_i32.to_le_bytes());
        bytes.push(index as u8);
    }
    bytes
}

#[test]
fn decodes_used_lump_ranges_at_their_original_directory_positions() {
    let bytes = file();
    let header = decode(&bytes).unwrap();
    for (index, range) in [
        (1, header.planes),
        (2, header.textures),
        (11, header.marksurfaces),
        (3, header.vertices),
        (4, header.visibility),
        (5, header.nodes),
        (6, header.texinfo),
        (7, header.faces),
        (9, header.clipnodes),
        (10, header.leaves),
        (12, header.edges),
        (13, header.surfedges),
        (14, header.models),
    ] {
        assert_eq!(range, HEADER_SIZE + index..HEADER_SIZE + index + 1);
        assert_eq!(&bytes[range], &[index as u8]);
    }
}

#[test]
fn rejects_truncated_headers_and_other_versions() {
    let mut bytes = file();
    for length in 0..HEADER_SIZE {
        assert!(decode(&bytes[..length]).is_err());
    }
    for version in [28_i32, 30, 31] {
        bytes[..INTEGER_SIZE].copy_from_slice(&version.to_le_bytes());
        assert!(decode(&bytes).is_err());
    }
}

#[test]
fn validates_every_lump_range_including_unused_lumps() {
    for index in 0..LUMP_COUNT {
        for (offset, length) in [
            (-1_i32, 0_i32),
            (0, 0),
            (HEADER_SIZE as i32, -1),
            (i32::MAX, 1),
            (HEADER_SIZE as i32, i32::MAX),
        ] {
            let mut bytes = file();
            let entry = INTEGER_SIZE + index * ENTRY_SIZE;
            bytes[entry..entry + INTEGER_SIZE].copy_from_slice(&offset.to_le_bytes());
            bytes[entry + INTEGER_SIZE..entry + ENTRY_SIZE].copy_from_slice(&length.to_le_bytes());
            assert!(
                decode(&bytes).is_err(),
                "accepted lump {index}: {offset}, {length}"
            );
        }
    }
}

#[test]
fn accepts_empty_lumps_at_end_of_file_and_rejects_ranges_beyond_it() {
    let mut bytes = file();
    let end = bytes.len() as i32;
    let entry = INTEGER_SIZE + 3 * ENTRY_SIZE;
    bytes[entry..entry + INTEGER_SIZE].copy_from_slice(&end.to_le_bytes());
    bytes[entry + INTEGER_SIZE..entry + ENTRY_SIZE].copy_from_slice(&0_i32.to_le_bytes());
    assert_eq!(decode(&bytes).unwrap().vertices, end as usize..end as usize);
    bytes[entry..entry + INTEGER_SIZE].copy_from_slice(&(end + 1).to_le_bytes());
    assert!(decode(&bytes).is_err());
}

#[test]
fn ignores_appended_extension_data() {
    let mut bytes = file();
    bytes.extend_from_slice(b"BSPX\xff\xff\xff\xffDECOUPLED_LM RGBLIGHTING");
    assert!(decode(&bytes).is_ok());
}
