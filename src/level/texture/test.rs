use crate::bsp::texture::TextureMetadata;

use super::{External, Indexed, Kind, expand_indexed, expand_rgb};

// Make a named external texture with BSP dimensions.
fn external() -> External {
    External {
        name: "wall".into(),
        metadata: TextureMetadata {
            frame_index: 0,
            alternate: false,
            kind: Kind::Solid,
            group: "wall".into(),
        },
        dimensions: [2, 1],
    }
}

#[test]
fn rgb_expands_external_pixels_with_opaque_alpha() {
    let pixels = expand_rgb(&external(), &[[1, 2, 3], [4, 5, 6]]).unwrap();
    assert_eq!(pixels, [[1, 2, 3, 255], [4, 5, 6, 255]]);
}

#[test]
fn rgb_rejects_empty_dimensions_and_incorrect_pixel_counts() {
    let mut texture = external();
    for dimensions in [[0, 1], [1, 0], [2, 1], [16, 16]] {
        texture.dimensions = dimensions;
        assert!(expand_rgb(&texture, &[[0; 3]]).is_err());
    }
}

#[test]
fn palette_preserves_sky_mask_and_mips() {
    let mut texture = Indexed {
        name: "sky".into(),
        metadata: TextureMetadata {
            frame_index: 0,
            alternate: false,
            kind: Kind::Sky,
            group: "sky".into(),
        },
        dimensions: [16; 2],
        mips: std::array::from_fn(|level| vec![0; (16 >> level) * (16 >> level)]),
    };
    let mips = expand_indexed(&texture, &[[1, 2, 3]; 256]);
    assert_eq!(mips.len(), 4);
    for (level, pixels) in mips.iter().enumerate() {
        let width = 16 >> level;
        assert_eq!(pixels[0], [0; 4]);
        assert_eq!(pixels[width / 2], [1, 2, 3, 255]);
    }
    texture.metadata.kind = Kind::Solid;
    let mips = expand_indexed(&texture, &[[1, 2, 3]; 256]);
    assert!(mips.iter().flatten().all(|pixel| *pixel == [1, 2, 3, 255]));
}
