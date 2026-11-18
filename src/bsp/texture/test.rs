use super::super::texture::{Texture, decode};

const DIRECTORY_SIZE: usize = 12;
const HEADER_SIZE: usize = 40;

// Store two textures with distinct names, dimensions, and palette indices at every mip.
fn file() -> Vec<u8> {
    let mut bytes = vec![0; DIRECTORY_SIZE];
    bytes[..4].copy_from_slice(&2_i32.to_le_bytes());
    for (index, name, dimensions) in [
        (0, b"wall".as_slice(), [64_u32, 128]),
        (1, b"*water", [16, 32]),
    ] {
        let start = bytes.len();
        let entry = 4 + index * 4;
        bytes[entry..entry + 4].copy_from_slice(&(start as i32).to_le_bytes());
        bytes.resize(start + HEADER_SIZE, 0);
        bytes[start..start + name.len()].copy_from_slice(name);
        bytes[start + 16..start + 20].copy_from_slice(&dimensions[0].to_le_bytes());
        bytes[start + 20..start + 24].copy_from_slice(&dimensions[1].to_le_bytes());
        for mip in 0..4 {
            let offset = (bytes.len() - start) as u32;
            let field = start + 24 + mip * 4;
            bytes[field..field + 4].copy_from_slice(&offset.to_le_bytes());
            let count = ((dimensions[0] >> mip) * (dimensions[1] >> mip)) as usize;
            bytes.resize(bytes.len() + count, (index * 4 + mip) as u8);
        }
    }
    bytes
}

#[test]
fn groups_texture_families_in_frame_order_without_changing_texture_ids() {
    let names = [
        "+Bbutton",
        "+1Button",
        "wall",
        "+0button",
        "+aBUTTON",
        "+1orphan",
        "+0still",
        "+0loop",
        "+2loop",
        "+1button",
        "button",
        "*slime0",
        "+Aorphan_alt",
    ];
    let mut bytes = vec![0; (names.len() + 1) * 4];
    bytes[..4].copy_from_slice(&(names.len() as i32).to_le_bytes());
    for (id, name) in names.into_iter().enumerate() {
        let start = bytes.len();
        let entry = (id + 1) * 4;
        bytes[entry..entry + 4].copy_from_slice(&(start as i32).to_le_bytes());
        bytes.resize(start + HEADER_SIZE, 0);
        bytes[start..start + name.len()].copy_from_slice(name.as_bytes());
        bytes[start + 16..start + 20].copy_from_slice(&16_u32.to_le_bytes());
        bytes[start + 20..start + 24].copy_from_slice(&16_u32.to_le_bytes());
    }

    let (textures, families) = decode(&bytes).unwrap();
    for (id, name) in names.into_iter().enumerate() {
        assert_eq!(textures[id].as_ref().unwrap().name(), name);
    }
    let button = &families[&3];
    assert_eq!(button.normal, [3, 1]);
    assert_eq!(button.alternate.as_deref(), Some(&[4, 0][..]));
    for id in [0, 1, 4, 9] {
        assert!(std::sync::Arc::ptr_eq(&families[&id], button));
    }
    assert_eq!(families[&7].normal, [7, 8]);
    assert_eq!(families[&8].alternate, None);
    for id in [2, 5, 6, 10, 11] {
        assert_eq!(families[&id].normal, [id]);
        assert_eq!(families[&id].alternate, None);
    }
    assert_eq!(families[&12].normal, [12]);
    assert_eq!(families[&12].alternate.as_deref(), Some(&[12][..]));
}

#[test]
fn classifies_texture_names_during_decode() {
    use super::super::texture::Kind;

    for (name, expected) in [
        ("wall", Kind::Solid),
        ("portal", Kind::Portal),
        ("PoRtAl_front", Kind::Portal),
        ("wall_portal", Kind::Solid),
        ("sky", Kind::Sky),
        ("SkYcloud", Kind::Sky),
        ("*water0", Kind::Water),
        ("*slime", Kind::Slime),
        ("*SLIME2", Kind::Slime),
        ("*lava", Kind::Lava),
        ("*LAVA2", Kind::Lava),
        ("{fence", Kind::Solid),
    ] {
        let mut bytes = file();
        let field = &mut bytes[DIRECTORY_SIZE..DIRECTORY_SIZE + 16];
        field.fill(0);
        field[..name.len()].copy_from_slice(name.as_bytes());
        let (textures, _) = decode(&bytes).unwrap();
        let texture = textures[0].as_ref().unwrap();
        assert_eq!(texture.kind(), expected, "{name}");
        assert_eq!(texture.name(), name);
    }
}

#[test]
fn preserves_texture_names_dimensions_and_four_indexed_mips() {
    let mut bytes = file();
    let (textures, _) = decode(&bytes).unwrap();
    let Texture::Indexed(first) = textures[0].as_ref().unwrap() else {
        panic!("expected indexed texture")
    };
    assert_eq!(first.name, "wall");
    assert_eq!(first.dimensions, [64, 128]);
    assert_eq!(
        first.mips.each_ref().map(|mip| mip.len()),
        [8192, 2048, 512, 128]
    );
    for (mip, pixels) in first.mips.iter().enumerate() {
        assert!(pixels.iter().all(|&pixel| pixel == mip as u8));
    }
    let Texture::Indexed(second) = textures[1].as_ref().unwrap() else {
        panic!("expected indexed texture")
    };
    assert_eq!(second.name, "*water");
    assert_eq!(second.dimensions, [16, 32]);
    assert_eq!(second.mips[3], [7; 8]);
    // A missing first entry must not shift the second texture's index.
    bytes[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    let (textures, _) = decode(&bytes).unwrap();
    assert!(textures[0].is_none());
    assert_eq!(textures[1].as_ref().unwrap().name(), "*water");
}

#[test]
fn rejects_missing_or_truncated_mips_and_cross_texture_reads() {
    let original = file();
    for mip in 0..4 {
        for offset in [0_u32, 39, u32::MAX] {
            let mut bytes = original.clone();
            let field = DIRECTORY_SIZE + 24 + mip * 4;
            bytes[field..field + 4].copy_from_slice(&offset.to_le_bytes());
            assert!(decode(&bytes).is_err());
        }
    }
    let mut bytes = original.clone();
    let second = i32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    bytes[DIRECTORY_SIZE + 24..DIRECTORY_SIZE + 28]
        .copy_from_slice(&((second - DIRECTORY_SIZE) as u32).to_le_bytes());
    assert!(decode(&bytes).is_err());
    assert!(decode(&original[..original.len() - 1]).is_err());
}

#[test]
fn rejects_invalid_names_dimensions_counts_and_offsets_without_panicking() {
    for bytes in [&[][..], &0_i32.to_le_bytes()] {
        let (textures, families) = decode(bytes).unwrap();
        assert!(textures.is_empty());
        assert!(families.is_empty());
    }
    for count in [-1_i32, i32::MAX] {
        assert!(decode(&count.to_le_bytes()).is_err());
    }
    for offset in [i32::MIN, -2, 0, 4, 11, i32::MAX] {
        let mut bytes = file();
        bytes[4..8].copy_from_slice(&offset.to_le_bytes());
        assert!(decode(&bytes).is_err());
    }
    for value in [0_u8, 255] {
        let mut bytes = file();
        bytes[DIRECTORY_SIZE..DIRECTORY_SIZE + 16].fill(value);
        assert!(decode(&bytes).is_err());
    }
    for field in [16, 20] {
        let mut bytes = file();
        bytes[DIRECTORY_SIZE + field..DIRECTORY_SIZE + field + 4].fill(0);
        assert!(decode(&bytes).is_err());
    }
    let bytes = file();
    for end in 1..DIRECTORY_SIZE + HEADER_SIZE {
        assert!(decode(&bytes[..end]).is_err());
    }
}

#[test]
fn preserves_headers_without_embedded_pixels() {
    let mut bytes = file();
    bytes[DIRECTORY_SIZE + 24..DIRECTORY_SIZE + HEADER_SIZE].fill(0);
    let (textures, _) = decode(&bytes).unwrap();
    let texture = textures[0].as_ref().unwrap();
    assert_eq!(texture.name(), "wall");
    assert_eq!(texture.dimensions(), [64, 128]);
    assert!(matches!(texture, Texture::External(_)));
    assert!(matches!(textures[1], Some(Texture::Indexed(_))));

    bytes.truncate(DIRECTORY_SIZE + HEADER_SIZE);
    bytes[8..12].copy_from_slice(&(-1_i32).to_le_bytes());
    assert!(matches!(
        decode(&bytes).unwrap().0[0],
        Some(Texture::External(_))
    ));
}
