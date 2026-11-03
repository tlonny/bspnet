use glam::{UVec2, Vec3, Vec4};

use super::{
    Data, EDGES_ENTRY, FACES_ENTRY, HEADER_SIZE, LEAVES_ENTRY, MARKSURFACES_ENTRY, MODELS_ENTRY,
    NODES_ENTRY, PLANES_ENTRY, SURFEDGES_ENTRY, TEXINFO_ENTRY, TEXTURES_ENTRY, VERSION,
    VERTICES_ENTRY, VISIBILITY_ENTRY, edge, face, leaf, marksurface, model, node, plane, surfedge,
    texinfo, texture,
};

// Place geometry lumps after padding to exercise header-based addressing.
fn file() -> Vec<u8> {
    let offset = HEADER_SIZE + 16;
    let positions = [[1.25_f32, -2.5, 64.0], [0.0, 128.0, -0.125]];
    let mut bytes = vec![0; offset];
    bytes[..4].copy_from_slice(&VERSION.to_le_bytes());
    bytes[VERTICES_ENTRY..VERTICES_ENTRY + 4].copy_from_slice(&(offset as i32).to_le_bytes());
    bytes[VERTICES_ENTRY + 4..VERTICES_ENTRY + 8].copy_from_slice(&24_i32.to_le_bytes());
    bytes[EDGES_ENTRY..EDGES_ENTRY + 4].copy_from_slice(&((offset + 24) as i32).to_le_bytes());
    bytes[EDGES_ENTRY + 4..EDGES_ENTRY + 8].copy_from_slice(&8_i32.to_le_bytes());
    bytes[SURFEDGES_ENTRY..SURFEDGES_ENTRY + 4]
        .copy_from_slice(&((offset + 32) as i32).to_le_bytes());
    bytes[SURFEDGES_ENTRY + 4..SURFEDGES_ENTRY + 8].copy_from_slice(&8_i32.to_le_bytes());
    for position in positions {
        for component in position {
            bytes.extend_from_slice(&component.to_le_bytes());
        }
    }
    for index in [0_u16, 1, 1, 0] {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    for index in [1_i32, -1] {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    for entry in [
        FACES_ENTRY,
        MODELS_ENTRY,
        TEXINFO_ENTRY,
        PLANES_ENTRY,
        NODES_ENTRY,
        LEAVES_ENTRY,
        VISIBILITY_ENTRY,
        MARKSURFACES_ENTRY,
    ] {
        let end = bytes.len() as i32;
        bytes[entry..entry + 4].copy_from_slice(&end.to_le_bytes());
    }
    let texture_offset = bytes.len() as i32;
    bytes[TEXTURES_ENTRY..TEXTURES_ENTRY + 4].copy_from_slice(&texture_offset.to_le_bytes());
    bytes[TEXTURES_ENTRY + 4..TEXTURES_ENTRY + 8].copy_from_slice(&4_i32.to_le_bytes());
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes
}

// Build a nonzero-root tree with an oblique split, an axial split, and three leaves.
fn tree() -> Data {
    let mut data = Data::decode(&file()).unwrap();
    data.planes = vec![
        super::Plane {
            normal: Vec3::Z,
            distance: 5.0,
        },
        super::Plane {
            normal: Vec3::new(1.0, 1.0, 0.0),
            distance: 2.0,
        },
    ];
    data.nodes = vec![
        super::Node {
            plane: 0,
            children: [-3, -1],
        },
        super::Node {
            plane: 1,
            children: [0, -2],
        },
    ];
    data.leaves = vec![
        super::Leaf {
            contents: -2,
            visibility_offset: -1,
            first_marksurface: 0,
            num_marksurfaces: 0,
        },
        super::Leaf {
            contents: -1,
            visibility_offset: -1,
            first_marksurface: 0,
            num_marksurfaces: 0,
        },
        super::Leaf {
            contents: -3,
            visibility_offset: -1,
            first_marksurface: 0,
            num_marksurfaces: 0,
        },
    ];
    data.models = vec![super::Model {
        headnode: 1,
        visleafs: 2,
        first_face: 0,
        num_faces: 0,
    }];
    data
}

#[test]
fn decodes_little_endian_positions_at_the_declared_offset() {
    let data = Data::decode(&file()).unwrap();
    assert_eq!(
        data.vertices,
        [Vec3::new(1.25, -2.5, 64.0), Vec3::new(0.0, 128.0, -0.125)]
    );
    assert_eq!(data.edges, [[0, 1], [1, 0]]);
    assert_eq!(data.surfedges, [1, -1]);
}

#[test]
fn rejects_truncated_headers_and_other_versions() {
    let mut bytes = file();
    for length in 0..HEADER_SIZE {
        assert!(Data::decode(&bytes[..length]).is_err());
    }
    bytes[..4].copy_from_slice(&30_i32.to_le_bytes());
    assert!(Data::decode(&bytes).is_err());
}

#[test]
fn rejects_invalid_lump_ranges_and_partial_vertices() {
    for (offset, length) in [
        (-1_i32, 24_i32),
        (0, 24),
        (HEADER_SIZE as i32, -1),
        (i32::MAX, 24),
        (HEADER_SIZE as i32 + 16, i32::MAX),
        (HEADER_SIZE as i32 + 16, 25),
        (HEADER_SIZE as i32 + 16, 23),
    ] {
        let mut bytes = file();
        bytes[VERTICES_ENTRY..VERTICES_ENTRY + 4].copy_from_slice(&offset.to_le_bytes());
        bytes[VERTICES_ENTRY + 4..VERTICES_ENTRY + 8].copy_from_slice(&length.to_le_bytes());
        assert!(Data::decode(&bytes).is_err(), "accepted {offset}, {length}");
    }
}

#[test]
fn accepts_an_empty_vertices_lump() {
    let mut bytes = file();
    bytes[VERTICES_ENTRY + 4..VERTICES_ENTRY + 8].copy_from_slice(&0_i32.to_le_bytes());
    assert!(Data::decode(&bytes).unwrap().vertices.is_empty());
}

#[test]
fn edges_decode_unsigned_indices_and_reject_partial_records() {
    assert_eq!(
        edge::decode(&[0x34, 0x12, 0xff, 0xff]).unwrap(),
        [[0x1234, u16::MAX]]
    );
    assert!(edge::decode(&[]).unwrap().is_empty());
    for length in 1..4 {
        assert!(edge::decode(&[0; 4][..length]).is_err());
    }
}

#[test]
fn rejects_invalid_edges_lump_ranges_and_lengths() {
    for (offset, length) in [
        (-1_i32, 8_i32),
        (0, 8),
        (HEADER_SIZE as i32, -1),
        (i32::MAX, 8),
        (HEADER_SIZE as i32, i32::MAX),
        (HEADER_SIZE as i32, 7),
    ] {
        let mut bytes = file();
        bytes[EDGES_ENTRY..EDGES_ENTRY + 4].copy_from_slice(&offset.to_le_bytes());
        bytes[EDGES_ENTRY + 4..EDGES_ENTRY + 8].copy_from_slice(&length.to_le_bytes());
        assert!(Data::decode(&bytes).is_err(), "accepted {offset}, {length}");
    }
}

#[test]
fn surfedges_preserve_signed_32_bit_indices() {
    assert_eq!(
        surfedge::decode(&[0x78, 0x56, 0x34, 0x12, 0x88, 0xa9, 0xcb, 0xed,]).unwrap(),
        [0x12345678, -0x12345678]
    );
    assert!(surfedge::decode(&[]).unwrap().is_empty());
    for length in 1..4 {
        assert!(surfedge::decode(&[0; 4][..length]).is_err());
    }
}

#[test]
fn faces_decode_ranges_from_full_records() {
    let bytes = [
        0x34, 0x12, 1, 0, 0x78, 0x56, 0x34, 0x12, 5, 0, 0x21, 0x43, 0, 5, 255, 255, 255, 255, 255,
        255,
    ];
    let faces = face::decode(&bytes.repeat(2)).unwrap();
    assert_eq!(faces.len(), 2);
    let face = &faces[1];
    assert_eq!(face.first_surfedge, 0x12345678);
    assert_eq!(face.num_surfedges, 5);
    assert_eq!(face.texinfo, 0x4321);
    assert!(face::decode(&[]).unwrap().is_empty());
    for length in 1..bytes.len() {
        assert!(face::decode(&bytes[..length]).is_err());
    }
}

#[test]
fn models_decode_face_ranges_from_full_records() {
    let mut bytes = Vec::new();
    for value in [-32.0_f32, -64.0, -128.0, 16.0, 32.0, 64.0, 1.25, -2.5, 4.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0x12345678_i32, 20, 30, -1, 7, 12, 100] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let models = model::decode(&bytes.repeat(2)).unwrap();
    assert_eq!(models.len(), 2);
    let model = &models[1];
    assert_eq!(model.headnode, 0x12345678);
    assert_eq!(model.visleafs, 7);
    assert_eq!(model.first_face, 12);
    assert_eq!(model.num_faces, 100);
    assert!(model::decode(&[]).unwrap().is_empty());
    for length in 1..bytes.len() {
        assert!(model::decode(&bytes[..length]).is_err());
    }
}

#[test]
fn decodes_the_librequake_fixture() {
    let data = Data::decode(include_bytes!("../../assets/maps/librequake/lq_e0m1.bsp")).unwrap();
    assert_eq!(data.planes.len(), 2_222);
    assert_eq!(data.nodes.len(), 2_375);
    assert_eq!(data.leaves.len(), 1_302);
    assert_eq!(data.visibility.len(), 24_332);
    assert_eq!(data.marksurfaces.len(), 5_517);
    assert_eq!(data.models[0].visleafs, 1_047);
    assert!(
        data.marksurfaces
            .iter()
            .all(|&face| usize::from(face) < data.faces.len())
    );
    for leaf in &data.leaves {
        let start = usize::from(leaf.first_marksurface);
        let end = start + usize::from(leaf.num_marksurfaces);
        assert!(end <= data.marksurfaces.len());
        assert!(
            leaf.visibility_offset == -1
                || (leaf.visibility_offset >= 0
                    && (leaf.visibility_offset as usize) < data.visibility.len())
        );
    }
    assert_eq!(data.models[0].headnode, 0);
    assert_eq!(data.leaves[0].contents, -2);
    assert_eq!(data.vertices.len(), 4_643);
    assert_eq!(data.edges.len(), 8_931);
    assert_eq!(data.surfedges.len(), 17_283);
    assert_eq!(data.faces.len(), 4_145);
    assert_eq!(data.models.len(), 26);
    assert_eq!(data.texinfo.len(), 949);
    assert!(data.vertices.iter().all(|value| value.is_finite()));
    assert!(
        data.edges
            .iter()
            .flatten()
            .all(|&index| usize::from(index) < data.vertices.len())
    );
    assert!(
        data.surfedges
            .iter()
            .all(|&index| { index != 0 && (index.unsigned_abs() as usize) < data.edges.len() })
    );
    for face in &data.faces {
        assert!(usize::from(face.texinfo) < data.texinfo.len());
        assert!(face.first_surfedge >= 0);
        assert!(face.num_surfedges >= 3);
        assert!(
            face.first_surfedge as usize + usize::from(face.num_surfedges) <= data.surfedges.len()
        );
    }
    for info in &data.texinfo {
        assert!(info.texture >= 0 && (info.texture as usize) < data.textures.len());
        assert!(info.axes.iter().all(|axis| axis.is_finite()));
    }
    for model in &data.models {
        assert!(model.first_face >= 0 && model.num_faces >= 0);
        assert!(model.first_face as usize + model.num_faces as usize <= data.faces.len());
    }
}

#[test]
fn externalized_fixture_preserves_the_original_mesh() {
    let original =
        Data::decode(include_bytes!("../../assets/maps/librequake/lq_e0m1.bsp")).unwrap();
    let external = Data::decode(include_bytes!(
        "../../assets/maps/librequake/lq_e0m1-notex.bsp"
    ))
    .unwrap();
    assert_eq!(external.visibility, original.visibility);
    assert_eq!(external.marksurfaces, original.marksurfaces);
    assert_eq!(external.textures, original.textures);
    assert_eq!(external.texinfo, original.texinfo);
    assert_eq!(external.textures.len(), 108);
    assert_eq!(external.textures[1].name, "tek_flr3");
    assert_eq!(external.textures[1].dimensions, UVec2::splat(64));
    for texture in &external.textures {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/textures/librequake"
        ))
        .join(format!("{}.png", texture.name));
        let pixels = image::open(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            .into_rgba8();
        assert_eq!(
            UVec2::new(pixels.width(), pixels.height()),
            texture.dimensions
        );
    }
    let original_mesh = crate::level::triangulate(&original).unwrap();
    let external_mesh = crate::level::triangulate(&external).unwrap();
    assert_eq!(external_mesh.indices, original_mesh.indices);
    assert_eq!(external_mesh.ranges, original_mesh.ranges);
    assert_eq!(
        external_mesh
            .vertices
            .iter()
            .map(|vertex| (vertex.position, vertex.uv))
            .collect::<Vec<_>>(),
        original_mesh
            .vertices
            .iter()
            .map(|vertex| (vertex.position, vertex.uv))
            .collect::<Vec<_>>()
    );
}

// Put two texture headers out of directory order, separated by padding.
fn texture_lump() -> Vec<u8> {
    let mut bytes = vec![0; 104];
    bytes[..4].copy_from_slice(&2_i32.to_le_bytes());
    bytes[4..8].copy_from_slice(&64_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(&16_i32.to_le_bytes());
    bytes[16..32].copy_from_slice(b"abcdefghijklmnop");
    bytes[32..36].copy_from_slice(&32_u32.to_le_bytes());
    bytes[36..40].copy_from_slice(&64_u32.to_le_bytes());
    bytes[64..69].copy_from_slice(b"brick");
    bytes[80..84].copy_from_slice(&128_u32.to_le_bytes());
    bytes[84..88].copy_from_slice(&16_u32.to_le_bytes());
    bytes
}

#[test]
fn textures_follow_offsets_and_preserve_directory_order() {
    let textures = texture::decode(&texture_lump()).unwrap();
    assert_eq!(textures.len(), 2);
    assert_eq!(textures[0].name, "brick");
    assert_eq!(textures[0].dimensions, UVec2::new(128, 16));
    assert_eq!(textures[1].name, "abcdefghijklmnop");
    assert_eq!(textures[1].dimensions, UVec2::new(32, 64));
    assert!(texture::decode(&0_i32.to_le_bytes()).unwrap().is_empty());
}

#[test]
fn textures_reject_invalid_counts_offsets_and_headers() {
    for length in 0..4 {
        assert!(texture::decode(&[0; 4][..length]).is_err());
    }
    for count in [-1_i32, i32::MAX] {
        assert!(texture::decode(&count.to_le_bytes()).is_err());
    }
    for offset in [-1_i32, 0, 8, 65, i32::MAX] {
        let mut bytes = texture_lump();
        bytes[4..8].copy_from_slice(&offset.to_le_bytes());
        assert!(
            texture::decode(&bytes).is_err(),
            "accepted texture offset {offset}"
        );
    }
    let mut bytes = texture_lump();
    bytes[64] = 0xff;
    assert!(texture::decode(&bytes).is_err());
    for start in [80, 84] {
        let mut bytes = texture_lump();
        bytes[start..start + 4].copy_from_slice(&0_u32.to_le_bytes());
        assert!(texture::decode(&bytes).is_err());
    }
}

#[test]
fn texinfo_decodes_axes_offsets_and_texture_indices() {
    let mut bytes = Vec::new();
    for value in [2.0_f32, 0.0, 0.0, 16.0, 0.0, 0.0, -0.5, -8.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0x12345678_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    let info = texinfo::decode(&bytes.repeat(2)).unwrap();
    assert_eq!(info.len(), 2);
    assert_eq!(
        info[1].axes,
        [
            Vec4::new(2.0, 0.0, 0.0, 16.0),
            Vec4::new(0.0, 0.0, -0.5, -8.0)
        ]
    );
    assert_eq!(info[1].texture, 0x12345678);
    assert!(texinfo::decode(&[]).unwrap().is_empty());
    for length in 1..bytes.len() {
        assert!(texinfo::decode(&bytes[..length]).is_err());
    }
}

#[test]
fn planes_decode_normals_and_distances_from_full_records() {
    let mut bytes = Vec::new();
    for value in [0.25_f32, -0.5, 1.0, -64.5] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&5_i32.to_le_bytes());
    let planes = plane::decode(&bytes.repeat(2)).unwrap();
    assert_eq!(planes.len(), 2);
    assert_eq!(planes[1].normal, Vec3::new(0.25, -0.5, 1.0));
    assert_eq!(planes[1].distance, -64.5);
    assert!(plane::decode(&[]).unwrap().is_empty());
    for length in 1..bytes.len() {
        assert!(plane::decode(&bytes[..length]).is_err());
    }
}

#[test]
fn nodes_decode_plane_indices_and_signed_children_from_full_records() {
    let mut bytes = vec![0; 24];
    bytes[..4].copy_from_slice(&0x12345678_i32.to_le_bytes());
    bytes[4..6].copy_from_slice(&0x1234_i16.to_le_bytes());
    bytes[6..8].copy_from_slice(&i16::MIN.to_le_bytes());
    let nodes = node::decode(&bytes.repeat(2)).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[1].plane, 0x12345678);
    assert_eq!(nodes[1].children, [0x1234, i16::MIN]);
    assert!(node::decode(&[]).unwrap().is_empty());
    for length in 1..bytes.len() {
        assert!(node::decode(&bytes[..length]).is_err());
    }
}

#[test]
fn leaves_decode_contents_visibility_offsets_and_unsigned_marksurface_ranges() {
    let mut bytes = vec![0; 28];
    bytes[..4].copy_from_slice(&(-3_i32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    bytes[20..22].copy_from_slice(&0x8123_u16.to_le_bytes());
    bytes[22..24].copy_from_slice(&0xfedc_u16.to_le_bytes());
    let leaves = leaf::decode(&bytes.repeat(2)).unwrap();
    assert_eq!(leaves.len(), 2);
    assert_eq!(leaves[1].contents, -3);
    assert_eq!(leaves[1].visibility_offset, -1);
    assert_eq!(leaves[1].first_marksurface, 0x8123);
    assert_eq!(leaves[1].num_marksurfaces, 0xfedc);
    assert!(leaf::decode(&[]).unwrap().is_empty());
    for length in 1..bytes.len() {
        assert!(leaf::decode(&bytes[..length]).is_err());
    }
}

#[test]
fn rejects_invalid_tree_lump_ranges_and_partial_records() {
    for (entry, record_size) in [
        (PLANES_ENTRY, 20_i32),
        (NODES_ENTRY, 24),
        (LEAVES_ENTRY, 28),
    ] {
        for (offset, length) in [
            (-1, 0),
            (0, 0),
            (HEADER_SIZE as i32, -1),
            (i32::MAX, 1),
            (HEADER_SIZE as i32, record_size - 1),
        ] {
            let mut bytes = file();
            bytes[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
            bytes[entry + 4..entry + 8].copy_from_slice(&length.to_le_bytes());
            assert!(Data::decode(&bytes).is_err());
        }
    }
}

#[test]
fn finds_leaves_from_the_world_root_and_sends_plane_boundaries_back() {
    let data = tree();
    for (point, expected) in [
        (Vec3::new(3.0, 0.0, 6.0), 2),
        (Vec3::new(3.0, 0.0, 5.0), 0),
        (Vec3::new(1.0, 0.0, 6.0), 1),
        (Vec3::new(1.0, 1.0, 100.0), 1),
    ] {
        assert_eq!(data.leaf_at(point).unwrap(), expected);
    }
    let mut data = tree();
    data.models[0].headnode = -2;
    data.nodes.clear();
    assert_eq!(data.leaf_at(Vec3::ZERO).unwrap(), 1);
}

#[test]
fn rejects_missing_tree_references_and_cycles_without_hanging() {
    let point = Vec3::new(3.0, 0.0, 6.0);
    let mut data = tree();
    data.models.clear();
    assert!(data.leaf_at(point).is_err());
    for root in [i32::MAX, i32::MIN] {
        let mut data = tree();
        data.models[0].headnode = root;
        assert!(data.leaf_at(point).is_err());
    }
    for plane in [-1, 2] {
        let mut data = tree();
        data.nodes[1].plane = plane;
        assert!(data.leaf_at(point).is_err());
    }
    for child in [2, i16::MIN, 0] {
        let mut data = tree();
        data.nodes[0].children[0] = child;
        assert!(data.leaf_at(point).is_err());
    }
    for point in [Vec3::splat(f32::NAN), Vec3::splat(f32::INFINITY)] {
        assert!(tree().leaf_at(point).is_err());
    }
}

#[test]
fn locates_the_fixture_camera_in_empty_space_and_handles_solid_space() {
    let data = Data::decode(include_bytes!(
        "../../assets/maps/librequake/lq_e0m1-notex.bsp"
    ))
    .unwrap();
    let leaf = data.leaf_at(Vec3::new(512.0, 2048.0, 326.0)).unwrap();
    assert_eq!(leaf, 39);
    assert_eq!(data.leaves[leaf].contents, -1);
    assert_eq!(data.leaf_at(Vec3::ZERO).unwrap(), 0);
    assert_eq!(data.leaf_at(Vec3::splat(1_000_000.0)).unwrap(), 0);
}

#[test]
fn marksurfaces_decode_unsigned_face_indices_and_reject_partial_records() {
    assert_eq!(
        marksurface::decode(&[0x34, 0x12, 0xff, 0xff]).unwrap(),
        [0x1234, u16::MAX]
    );
    assert!(marksurface::decode(&[]).unwrap().is_empty());
    assert!(marksurface::decode(&[0]).is_err());
    assert!(marksurface::decode(&[0, 0, 0]).is_err());
}

#[test]
fn decodes_visibility_bytes_and_marksurfaces_at_their_declared_offsets() {
    let mut bytes = file();
    let visibility = [0x05, 0, 3, 0x80];
    let offset = bytes.len() as i32;
    bytes[VISIBILITY_ENTRY..VISIBILITY_ENTRY + 4].copy_from_slice(&offset.to_le_bytes());
    bytes[VISIBILITY_ENTRY + 4..VISIBILITY_ENTRY + 8].copy_from_slice(&4_i32.to_le_bytes());
    bytes.extend_from_slice(&visibility);
    let offset = bytes.len() as i32;
    bytes[MARKSURFACES_ENTRY..MARKSURFACES_ENTRY + 4].copy_from_slice(&offset.to_le_bytes());
    bytes[MARKSURFACES_ENTRY + 4..MARKSURFACES_ENTRY + 8].copy_from_slice(&4_i32.to_le_bytes());
    bytes.extend_from_slice(&[0x34, 0x12, 0xff, 0xff]);
    let data = Data::decode(&bytes).unwrap();
    assert_eq!(data.visibility, visibility);
    assert_eq!(data.marksurfaces, [0x1234, u16::MAX]);
    let empty = Data::decode(&file()).unwrap();
    assert!(empty.visibility.is_empty());
    assert!(empty.marksurfaces.is_empty());
}

#[test]
fn rejects_invalid_visibility_and_marksurface_lump_ranges() {
    for entry in [VISIBILITY_ENTRY, MARKSURFACES_ENTRY] {
        for (offset, length) in [
            (-1_i32, 0_i32),
            (0, 0),
            (HEADER_SIZE as i32, -1),
            (i32::MAX, 4),
            (HEADER_SIZE as i32, i32::MAX),
        ] {
            let mut bytes = file();
            bytes[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
            bytes[entry + 4..entry + 8].copy_from_slice(&length.to_le_bytes());
            assert!(Data::decode(&bytes).is_err());
        }
    }
    let mut bytes = file();
    bytes[MARKSURFACES_ENTRY + 4..MARKSURFACES_ENTRY + 8].copy_from_slice(&1_i32.to_le_bytes());
    assert!(Data::decode(&bytes).is_err());
}
