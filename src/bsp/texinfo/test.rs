use super::super::texture::{External, Kind, TextureMetadata};
use super::super::{Texinfo, Texture, texinfo};

const RECORD_SIZE: usize = 40;

// Decode against eight present texture entries.
fn decode(bytes: &[u8]) -> std::io::Result<Vec<Texinfo>> {
    let textures = vec![
        Some(Texture::External(External {
            name: "wall".to_owned(),
            metadata: TextureMetadata {
                frame_index: 0,
                alternate: false,
                kind: Kind::Solid,
                group: "wall".into(),
            },
            dimensions: [16; 2],
        }));
        8
    ];
    texinfo::decode(bytes, &textures)
}

// Build texture projections with distinguishable axes, offsets, and a texture index.
fn record(flags: i32) -> [u8; RECORD_SIZE] {
    let mut bytes = [0; RECORD_SIZE];
    for (value, component) in [1.25_f32, -2.5, 3.0, 64.0, -4.0, 5.0, 6.0, -128.0]
        .into_iter()
        .zip(bytes[..32].chunks_exact_mut(4))
    {
        component.copy_from_slice(&value.to_le_bytes());
    }
    bytes[32..36].copy_from_slice(&7_i32.to_le_bytes());
    bytes[36..40].copy_from_slice(&flags.to_le_bytes());
    bytes
}

#[test]
fn decodes_projection_rows_in_record_order_with_either_supported_flag() {
    let mut bytes = record(0).to_vec();
    let mut second = record(1);
    second[32..36].copy_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&second);
    let mappings = decode(&bytes).unwrap();
    assert_eq!(mappings.len(), 2);
    assert_eq!(
        mappings[0].axes,
        [[1.25, -2.5, 3.0, 64.0], [-4.0, 5.0, 6.0, -128.0]]
    );
    assert_eq!(mappings[0].texture, 7);
    assert_eq!(mappings[1].texture, 0);
}

#[test]
fn rejects_negative_texture_indices_and_unsupported_flags() {
    for texture in [-1_i32, i32::MIN] {
        let mut bytes = record(0);
        bytes[32..36].copy_from_slice(&texture.to_le_bytes());
        assert_eq!(
            decode(&bytes).unwrap_err().to_string(),
            "negative texinfo texture index"
        );
    }
    for flags in [-1, 2, 3, i32::MAX] {
        assert_eq!(
            decode(&record(flags)).unwrap_err().to_string(),
            "unsupported texinfo flags"
        );
    }
}

#[test]
fn accepts_empty_lumps_and_rejects_partial_records() {
    assert!(decode(&[]).unwrap().is_empty());
    for length in 1..RECORD_SIZE {
        assert!(decode(&vec![0; length]).is_err());
    }
}
