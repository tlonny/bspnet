use crate::bsp::{Data as Bsp, sidecar};
use crate::level::{lightmap, mesh};

const HEADER_SIZE: usize = 124;
const MODEL_SIZE: usize = 64;
const FACE_SIZE: usize = 20;
const TEXINFO_SIZE: usize = 40;
const FACE_INDICES: [[u32; 3]; 2] = [[0, 1, 2], [3, 4, 5]];
const MISSING_LIGHTMAP: u32 = crate::gpu::buffer::vertex::world::MISSING_LIGHTMAP;

// Share an offscreen device between tests of the complete mesh upload path.
fn gpu() -> &'static (wgpu::Device, wgpu::Queue) {
    static GPU: std::sync::OnceLock<(wgpu::Device, wgpu::Queue)> = std::sync::OnceLock::new();
    GPU.get_or_init(|| {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        pollster::block_on(adapter.request_device(&Default::default())).unwrap()
    })
}

// Pack external RGB samples through the incremental interface used by mesh construction.
fn build_lighting(bsp: &Bsp, samples: &[[u8; 3]]) -> (Vec<lightmap::Face>, Vec<Vec<[u8; 4]>>) {
    let mut builder = lightmap::Builder::create(samples);
    let faces = (0..bsp.faces().len())
        .map(|index| builder.build(bsp, index))
        .collect();
    (faces, builder.finish())
}

// Concatenate individually constructed faces to inspect world-buffer positions and UVs.
fn vertices(bsp: &Bsp, faces: &[lightmap::Face]) -> Vec<crate::gpu::buffer::vertex::world::Data> {
    faces
        .iter()
        .enumerate()
        .flat_map(|(index, face)| mesh::worldspawn::vertices(bsp, index, face))
        .collect()
}

// Build one embedded indexed texture with four black-index mip levels.
fn texture_bytes(dimensions: [u32; 2]) -> Vec<u8> {
    let mut bytes = vec![0; 48];
    bytes[..4].copy_from_slice(&1_i32.to_le_bytes());
    bytes[4..8].copy_from_slice(&8_i32.to_le_bytes());
    bytes[8..12].copy_from_slice(b"wall");
    bytes[24..28].copy_from_slice(&dimensions[0].to_le_bytes());
    bytes[28..32].copy_from_slice(&dimensions[1].to_le_bytes());
    for level in 0..4 {
        let offset = (bytes.len() - 8) as u32;
        bytes[32 + level * 4..36 + level * 4].copy_from_slice(&offset.to_le_bytes());
        bytes.resize(
            bytes.len() + ((dimensions[0] >> level) * (dimensions[1] >> level)) as usize,
            0,
        );
    }
    bytes
}

// Build two faces sharing vertices but using different winding and texture projections.
fn bsp_bytes() -> Vec<u8> {
    let positions: Vec<_> = [[1_f32, 2., 3.], [4., 5., 6.], [7., 8., 9.]]
        .into_iter()
        .flatten()
        .flat_map(f32::to_le_bytes)
        .collect();
    let edges: Vec<_> = [0_u16, 0, 0, 1, 1, 2, 0, 2]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let surfedges: Vec<_> = [1_i32, 2, -3, -1, 3, -2]
        .into_iter()
        .flat_map(i32::to_le_bytes)
        .collect();
    let mut texinfo = [0; TEXINFO_SIZE * 2];
    for (axes, record) in [
        [2_f32, 0., 0., 8., 0., -1., 0., 16.],
        [0., 0., 4., -32., 0.5, 1., 0., -2.],
    ]
    .into_iter()
    .zip(texinfo.chunks_exact_mut(TEXINFO_SIZE))
    {
        for (value, component) in axes.into_iter().zip(record[..32].chunks_exact_mut(4)) {
            component.copy_from_slice(&value.to_le_bytes());
        }
    }
    let mut faces = [0; FACE_SIZE * 2];
    for (index, record) in faces.chunks_exact_mut(FACE_SIZE).enumerate() {
        record[4..8].copy_from_slice(&(index as i32 * 3).to_le_bytes());
        record[8..10].copy_from_slice(&3_i16.to_le_bytes());
        record[10..12].copy_from_slice(&(index as i16).to_le_bytes());
        record[12..16].fill(255);
        record[16..20].copy_from_slice(&(-1_i32).to_le_bytes());
    }
    let mut model = [0; MODEL_SIZE];
    for root in model[36..52].chunks_exact_mut(4) {
        root.copy_from_slice(&(-1_i32).to_le_bytes());
    }
    model[60..64].copy_from_slice(&2_i32.to_le_bytes());
    let mut leaf = [0; 28];
    leaf[..4].copy_from_slice(&(-2_i32).to_le_bytes());
    leaf[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    let textures = texture_bytes([16; 2]);
    let mut bytes = vec![0; HEADER_SIZE];
    bytes[..4].copy_from_slice(&29_i32.to_le_bytes());
    for entry in bytes[4..].chunks_exact_mut(8) {
        entry[..4].copy_from_slice(&(HEADER_SIZE as i32).to_le_bytes());
    }
    for (index, payload) in [
        (0, b"{\"classname\" \"worldspawn\"}".as_slice()),
        (1, &[0; 20][..]),
        (2, textures.as_slice()),
        (3, positions.as_slice()),
        (6, texinfo.as_slice()),
        (7, faces.as_slice()),
        (10, leaf.as_slice()),
        (12, edges.as_slice()),
        (13, surfedges.as_slice()),
        (14, model.as_slice()),
    ] {
        let offset = bytes.len() as i32;
        let entry = 4 + index * 8;
        bytes[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
        bytes[entry + 4..entry + 8].copy_from_slice(&(payload.len() as i32).to_le_bytes());
        bytes.extend_from_slice(payload);
    }

    bytes
}

// Borrow a native fixture lump for focused corruption and projection tests.
fn lump(bytes: &mut [u8], index: usize) -> &mut [u8] {
    let entry = 4 + index * 8;
    let offset = i32::from_le_bytes(bytes[entry..entry + 4].try_into().unwrap()) as usize;
    let length = i32::from_le_bytes(bytes[entry + 4..entry + 8].try_into().unwrap()) as usize;
    &mut bytes[offset..offset + length]
}

// Give both faces a single style and adjacent spans in external RGB data.
fn bsp() -> Bsp {
    let mut bytes = bsp_bytes();
    for (face, offset) in lump(&mut bytes, 7)
        .chunks_exact_mut(FACE_SIZE)
        .zip([0_i32, 6])
    {
        face[12] = 0;
        face[16..20].copy_from_slice(&offset.to_le_bytes());
    }
    Bsp::decode(&bytes).unwrap()
}

// Give the first face a rectangular footprint on the standard sixteen-unit lightmap grid.
fn grid_bsp(dimensions: [usize; 2], styles: [u8; 4], offset: i32) -> Bsp {
    let mut bytes = bsp_bytes();
    let face = &mut lump(&mut bytes, 7)[..FACE_SIZE];
    face[12..16].copy_from_slice(&styles);
    face[16..20].copy_from_slice(&offset.to_le_bytes());
    let positions = [
        [0_f32; 3],
        [(dimensions[0] - 1) as f32 * 16., 0., 0.],
        [0., (dimensions[1] - 1) as f32 * 16., 0.],
    ];
    for (position, record) in positions
        .into_iter()
        .zip(lump(&mut bytes, 3).chunks_exact_mut(12))
    {
        for (value, field) in position.into_iter().zip(record.chunks_exact_mut(4)) {
            field.copy_from_slice(&value.to_le_bytes());
        }
    }
    let axes = &mut lump(&mut bytes, 6)[..32];
    axes.fill(0);
    axes[..4].copy_from_slice(&1_f32.to_le_bytes());
    axes[20..24].copy_from_slice(&1_f32.to_le_bytes());
    Bsp::decode(&bytes).unwrap()
}

#[test]
fn expands_face_winding_and_normalizes_texture_uvs() {
    let bsp = bsp();
    let faces = build_lighting(&bsp, &[[1; 3]; 14]).0;
    let vertices = vertices(&bsp, &faces);
    let positions = [[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]];
    for (vertex, index) in vertices.iter().zip([0, 1, 2, 1, 0, 2]) {
        assert_eq!(vertex.position, positions[index]);
    }
    assert_eq!(vertices.len(), 6);
    assert_eq!(
        vertices.iter().map(|vertex| vertex.uv).collect::<Vec<_>>(),
        [
            [0.625, 0.875],
            [1., 0.6875],
            [1.375, 0.5],
            [-0.5, 0.3125],
            [-1.25, 0.03125],
            [0.25, 0.59375]
        ]
    );
}

#[test]
fn liquid_uvs_use_ironwail_scale_and_ignore_texture_offsets() {
    let mut bytes = bsp_bytes();
    let name = &mut lump(&mut bytes, 2)[8..24];
    name.fill(0);
    name[..7].copy_from_slice(b"*water0");
    let bsp = Bsp::decode(&bytes).unwrap();
    let vertices = mesh::worldspawn::vertices(&bsp, 0, &lightmap::Face::Unlit);
    assert_eq!(vertices[0].uv, [2. / 128., -2. / 128.]);
    assert_eq!(vertices[1].uv, [8. / 128., -5. / 128.]);
}

#[test]
fn derives_dimensions_and_uvs_from_texinfo_on_the_standard_grid() {
    let bsp = bsp();
    let faces = build_lighting(&bsp, &[[1; 3]; 14]).0;
    let dimensions: Vec<_> = faces
        .iter()
        .map(|face| match face {
            lightmap::Face::Lit { dimensions, .. } => *dimensions,
            lightmap::Face::Unlit => panic!("expected a lit face"),
        })
        .collect();
    assert_eq!(dimensions, [[3, 2], [4, 2]]);
    let vertices = vertices(&bsp, &faces);
    assert_eq!(
        vertices[0].light_uv[0],
        [(0.5 + 10. / 16.) / 1024., (0.5 + 14. / 16.) / 1024.]
    );
}

#[test]
fn properties_use_worldspawn_overrides_and_default_sky_speeds() {
    let mut bytes = bsp_bytes();
    let bsp = Bsp::decode(&bytes).unwrap();
    let properties = crate::level::properties::build(&bsp);
    assert_eq!(properties.water.sky_foreground_speed, 16.);
    assert_eq!(properties.water.sky_background_speed, 8.);
    assert_eq!(properties.water.seconds, 0.);
    assert_eq!(properties.water.liquid_alpha, 0.5);
    assert_eq!(properties.slime.liquid_alpha, 0.5);
    assert_eq!(
        properties.slime.liquid_tint,
        [0., 25. / 255., 5. / 255., 150. / 255.]
    );
    assert_eq!(
        properties.water.liquid_tint,
        [130. / 255., 80. / 255., 50. / 255., 128. / 255.]
    );

    let entities =
        br#"{"classname" "worldspawn" "_sky_foreground_speed" "-4" "_sky_background_speed" "0" "_wateralpha" "0.6" "_water_tint" "0.1 0.2 0.3" "_water_tint_strength" "0" "_slimealpha" "0.2" "_slime_tint" "0.4 0.5 0.6" "_slime_tint_strength" "0.7"}"#;
    let offset = bytes.len() as i32;
    bytes[4..8].copy_from_slice(&offset.to_le_bytes());
    bytes[8..12].copy_from_slice(&(entities.len() as i32).to_le_bytes());
    bytes.extend_from_slice(entities);
    let bsp = Bsp::decode(&bytes).unwrap();
    let properties = crate::level::properties::build(&bsp);
    assert_eq!(properties.water.sky_foreground_speed, -4.);
    assert_eq!(properties.water.sky_background_speed, 0.);
    assert_eq!(properties.water.liquid_alpha, 0.6);
    assert_eq!(properties.water.liquid_tint, [0.1, 0.2, 0.3, 0.]);
    assert_eq!(properties.slime.liquid_alpha, 0.2);
    assert_eq!(properties.slime.liquid_tint, [0.4, 0.5, 0.6, 0.7]);
    assert_eq!(properties.slime.sky_foreground_speed, -4.);
    assert_eq!(properties.slime.sky_background_speed, 0.);
}

#[test]
fn shared_builder_packs_separate_geometry_into_the_same_atlas() {
    let first = grid_bsp([3, 2], [0, 255, 255, 255], 0);
    let second = grid_bsp([3, 2], [0, 255, 255, 255], 6);
    let samples = [[10; 3]; 6]
        .into_iter()
        .chain([[20; 3]; 6])
        .collect::<Vec<_>>();
    let mut builder = lightmap::Builder::create(&samples);
    let first_face = builder.build(&first, 0);
    let second_face = builder.build(&second, 0);
    let first_vertex = mesh::worldspawn::vertices(&first, 0, &first_face)[0];
    let second_vertex = mesh::worldspawn::vertices(&second, 0, &second_face)[0];
    let atlases = builder.finish();
    assert_eq!(atlases.len(), 1);
    assert_eq!(atlases[0][0], [10, 10, 10, 255]);
    assert_eq!(atlases[0][3], [20, 20, 20, 255]);
    assert_eq!(first_vertex.lightmap_layers[0], 0);
    assert_eq!(second_vertex.lightmap_layers[0], 0);
    assert_eq!(first_vertex.light_uv[0], [0.5 / 1024.; 2]);
    assert_eq!(second_vertex.light_uv[0], [3.5 / 1024., 0.5 / 1024.]);
}

#[test]
fn packs_each_style_rectangle_using_external_sample_offsets_and_preserves_rgb() {
    let bsp = grid_bsp([3, 2], [0, 32, 255, 255], 2);
    let mut samples = vec![[1, 2, 3], [4, 5, 6]];
    samples.extend((1..=12).map(|value| [value * 10, value, 200]));
    let (faces, atlases) = build_lighting(&bsp, &samples);
    assert_eq!(atlases.len(), 1);
    let lightmap::Face::Lit { references, .. } = &faces[0] else {
        panic!("expected a lit face");
    };
    assert_eq!(references[0].unwrap().position, [0, 0]);
    assert_eq!(references[1].unwrap().position, [3, 0]);
    assert_eq!(references[1].unwrap().style, 32);
    assert_eq!(references[2..], [None, None]);
    let page = &atlases[0];
    assert_eq!(page[0], [10, 1, 200, 255]);
    assert_eq!(page[2], [30, 3, 200, 255]);
    assert_eq!(page[lightmap::ATLAS_SIZE], [40, 4, 200, 255]);
    assert_eq!(page[3], [70, 7, 200, 255]);
    assert_eq!(page[lightmap::ATLAS_SIZE + 5], [120, 12, 200, 255]);
    assert_eq!(page[6], [0; 4]);
    let vertices = vertices(&bsp, &faces);
    assert_eq!(vertices[0].light_uv[0], [0.5 / 1024.; 2]);
    assert_eq!(vertices[0].light_uv[1], [3.5 / 1024., 0.5 / 1024.]);
    assert_eq!(
        vertices[0].lightmap_layers,
        [0, 0, MISSING_LIGHTMAP, MISSING_LIGHTMAP]
    );
}

#[test]
fn returns_full_atlases_and_puts_layer_indices_on_vertices() {
    let bsp = grid_bsp([1024; 2], [0, 3, 255, 255], 0);
    let area = 1024 * 1024;
    let mut samples = vec![[10; 3]; area];
    samples.extend(vec![[20; 3]; area]);
    let (faces, atlases) = build_lighting(&bsp, &samples);
    assert_eq!(atlases.len(), 2);
    assert_eq!(atlases[0][0], [10, 10, 10, 255]);
    assert_eq!(atlases[1][0], [20, 20, 20, 255]);
    assert_eq!(
        vertices(&bsp, &faces)[0].lightmap_layers,
        [0, 1, MISSING_LIGHTMAP, MISSING_LIGHTMAP]
    );
}

#[test]
fn skips_partial_styles_and_rejects_oversized_projection_bounds_without_panicking() {
    let bsp = grid_bsp([3, 2], [0, 32, 255, 255], 0);
    let (faces, atlases) = build_lighting(&bsp, &[[1; 3]; 8]);
    assert_eq!(atlases.len(), 1);
    let lightmap::Face::Lit { references, .. } = &faces[0] else {
        panic!("expected the complete style");
    };
    assert!(references[0].is_some());
    assert!(references[1].is_none());
    for dimensions in [[1025, 1], [65535, 65535]] {
        let bsp = grid_bsp(dimensions, [0, 255, 255, 255], 0);
        let (faces, atlases) = build_lighting(&bsp, &[[1; 3]]);
        assert!(atlases.is_empty());
        assert!(matches!(faces[0], lightmap::Face::Unlit));
    }
}

#[test]
fn malformed_extents_and_missing_or_incomplete_samples_leave_faces_unlit() {
    for value in [f32::NAN, f32::INFINITY, f32::MAX] {
        let mut bytes = bsp_bytes();
        let face = &mut lump(&mut bytes, 7)[..FACE_SIZE];
        face[12] = 0;
        face[16..20].fill(0);
        lump(&mut bytes, 3)[..4].copy_from_slice(&value.to_le_bytes());
        let bsp = Bsp::decode(&bytes).unwrap();
        assert!(matches!(
            build_lighting(&bsp, &[[1; 3]; 6]).0[0],
            lightmap::Face::Unlit
        ));
    }
    for (styles, offset, samples) in [
        ([0, 255, 255, 255], -1, &[[1; 3]; 6][..]),
        ([255; 4], 0, &[[1; 3]; 6][..]),
        ([0, 255, 255, 255], 0, &[][..]),
        ([0, 32, 255, 255], 0, &[[1; 3]; 5][..]),
        ([0, 255, 255, 255], i32::MAX, &[[1; 3]; 6][..]),
    ] {
        let bsp = grid_bsp([3, 2], styles, offset);
        let (faces, atlases) = build_lighting(&bsp, samples);
        assert!(atlases.is_empty());
        assert!(
            vertices(&bsp, &faces)
                .iter()
                .all(|vertex| vertex.lightmap_layers == [MISSING_LIGHTMAP; 4])
        );
    }
}

#[test]
fn keeps_all_four_style_slots_and_their_uvs() {
    let bsp = grid_bsp([1; 2], [0, 32, 3, 63], 0);
    let (faces, atlases) = build_lighting(&bsp, &[[10; 3], [20; 3], [30; 3], [40; 3]]);
    assert_eq!(atlases.len(), 1);
    assert_eq!(
        &atlases[0][..4],
        [
            [10, 10, 10, 255],
            [20, 20, 20, 255],
            [30, 30, 30, 255],
            [40, 40, 40, 255]
        ]
    );
    let vertex = vertices(&bsp, &faces)[0];
    assert_eq!(vertex.lightmap_layers, [0; 4]);
    for slot in 0..4 {
        assert_eq!(
            vertex.light_uv[slot],
            [(slot as f32 + 0.5) / 1024., 0.5 / 1024.]
        );
    }
}

// Append or replace a native lump without changing existing payload offsets.
fn append_lump(bytes: &mut Vec<u8>, index: usize, payload: &[u8]) {
    let offset = bytes.len() as i32;
    let entry = 4 + index * 8;
    bytes[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
    bytes[entry + 4..entry + 8].copy_from_slice(&(payload.len() as i32).to_le_bytes());
    bytes.extend_from_slice(payload);
}

// Give two world leaves shared face references and optional compressed PVS rows.
fn visibility_bsp(
    pvs: Option<&[u8]>,
    marks: &[u16],
    ranges: [[u16; 2]; 2],
    world_faces: i32,
    texture_ids: [i32; 2],
) -> Bsp {
    let mut bytes = bsp_bytes();
    let single_texture = texture_bytes([16; 2]);
    let mut textures = vec![0; 12];
    textures[..4].copy_from_slice(&2_i32.to_le_bytes());
    for index in 0..2 {
        let offset = textures.len() as i32;
        textures[4 + index * 4..8 + index * 4].copy_from_slice(&offset.to_le_bytes());
        textures.extend_from_slice(&single_texture[8..]);
    }
    append_lump(&mut bytes, 2, &textures);
    for (record, texture_id) in lump(&mut bytes, 6)
        .chunks_exact_mut(TEXINFO_SIZE)
        .zip(texture_ids)
    {
        record[32..36].copy_from_slice(&texture_id.to_le_bytes());
    }
    let model = lump(&mut bytes, 14);
    model[52..56].copy_from_slice(&2_i32.to_le_bytes());
    model[60..64].copy_from_slice(&world_faces.to_le_bytes());
    let mut leaves = [0; 28 * 3];
    leaves[..4].copy_from_slice(&(-2_i32).to_le_bytes());
    leaves[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    for (index, [start, count]) in ranges.into_iter().enumerate() {
        let record = &mut leaves[(index + 1) * 28..(index + 2) * 28];
        record[..4].copy_from_slice(&(-1_i32).to_le_bytes());
        let offset = pvs.map_or(
            -1,
            |rows| if index == 0 { 0 } else { rows.len() as i32 - 1 },
        );
        record[4..8].copy_from_slice(&offset.to_le_bytes());
        record[20..22].copy_from_slice(&start.to_le_bytes());
        record[22..24].copy_from_slice(&count.to_le_bytes());
    }
    let marks: Vec<_> = marks.iter().flat_map(|value| value.to_le_bytes()).collect();
    append_lump(&mut bytes, 10, &leaves);
    append_lump(&mut bytes, 11, &marks);
    if let Some(pvs) = pvs {
        append_lump(&mut bytes, 4, pvs);
    }
    Bsp::decode(&bytes).unwrap()
}

#[test]
fn keeps_liquids_solids_and_sky_in_texture_batches_independent_of_opacity() {
    for opaque_name in ["wall", "sky1"] {
        let mut bytes = bsp_bytes();
        let single = texture_bytes([16; 2]);
        let mut textures = vec![0; 12];
        textures[..4].copy_from_slice(&2_i32.to_le_bytes());
        for (index, name) in ["*water0", opaque_name].into_iter().enumerate() {
            let offset = textures.len();
            textures[4 + index * 4..8 + index * 4].copy_from_slice(&(offset as i32).to_le_bytes());
            textures.extend_from_slice(&single[8..]);
            textures[offset..offset + 16].fill(0);
            textures[offset..offset + name.len()].copy_from_slice(name.as_bytes());
        }
        append_lump(&mut bytes, 2, &textures);
        lump(&mut bytes, 6)[TEXINFO_SIZE + 32..TEXINFO_SIZE + 36]
            .copy_from_slice(&1_i32.to_le_bytes());
        let bsp = Bsp::decode(&bytes).unwrap();
        let faces = FACE_INDICES.map(Vec::from);
        let (indices, batches) = mesh::batch::build(&bsp, &faces, 0);
        assert_eq!(indices, [0, 1, 2, 3, 4, 5]);
        assert_eq!(
            batches,
            [
                mesh::batch::Draw {
                    texture_id: 0,
                    indices: 0..3
                },
                mesh::batch::Draw {
                    texture_id: 1,
                    indices: 3..6
                },
            ]
        );
    }
}

#[test]
fn deduplicates_shared_pvs_faces_and_sorts_triangles_by_texture() {
    let bsp = visibility_bsp(Some(&[3, 2]), &[1, 0, 1, 0, 1], [[0, 3], [3, 2]], 2, [1, 0]);
    let faces = FACE_INDICES.map(Vec::from);
    let (indices, batches) = mesh::batch::build(&bsp, &faces, 1);
    assert_eq!(indices, [3, 4, 5, 0, 1, 2]);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].texture_id, 0);
    assert_eq!(batches[0].indices, 0..3);
    assert_eq!(batches[1].texture_id, 1);
    assert_eq!(batches[1].indices, 3..6);
}

#[test]
fn merges_faces_with_the_same_texture_into_one_draw_range() {
    let bsp = visibility_bsp(Some(&[3, 2]), &[0, 1], [[0, 1], [1, 1]], 2, [0; 2]);
    let faces = FACE_INDICES.map(Vec::from);
    let (indices, batches) = mesh::batch::build(&bsp, &faces, 1);
    assert_eq!(indices.len(), 6);
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].indices, 0..6);
}

#[test]
fn includes_camera_leaf_faces_and_excludes_brush_model_faces() {
    let bsp = visibility_bsp(Some(&[2, 2]), &[0, 1], [[0, 1], [1, 1]], 2, [0; 2]);
    let faces = FACE_INDICES.map(Vec::from);
    let (indices, _) = mesh::batch::build(&bsp, &faces, 1);
    assert_eq!(indices.len(), 6);
    let bsp = visibility_bsp(Some(&[3, 2]), &[0, 1], [[0, 1], [1, 1]], 1, [0; 2]);
    let (indices, batches) = mesh::batch::build(&bsp, &faces, 1);
    assert_eq!(indices, [0, 1, 2]);
    assert_eq!(batches.len(), 1);
    assert!(mesh::batch::build(&bsp, &faces, 2).0.is_empty());
}

#[test]
fn missing_pvs_shows_world_faces_but_empty_pvs_does_not() {
    let faces = FACE_INDICES.map(Vec::from);
    let bsp = visibility_bsp(None, &[1], [[0, 0], [0, 1]], 2, [0; 2]);
    for leaf in 0..3 {
        assert_eq!(mesh::batch::build(&bsp, &faces, leaf).0.len(), 6);
    }
    let bsp = visibility_bsp(Some(&[0, 1, 2]), &[1], [[0, 0], [0, 1]], 2, [0; 2]);
    let (indices, batches) = mesh::batch::build(&bsp, &faces, 1);
    assert!(indices.is_empty());
    assert!(batches.is_empty());
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn mesh_uses_precomputed_leaves_after_bsp_drop_and_skips_repeated_updates() {
    let (device, queue) = gpu();
    let bsp = visibility_bsp(Some(&[0, 1, 2]), &[1], [[0, 0], [0, 1]], 2, [0; 2]);
    let mut lightmap = lightmap::Builder::create(&[]);
    let mut mesh = mesh::worldspawn::Mesh::build(&bsp, &mut lightmap, device, queue);
    drop(bsp);
    assert!(mesh.set_leaf(queue, 1).unwrap());
    assert!(!mesh.set_leaf(queue, 1).unwrap());
    assert!(mesh.set_leaf(queue, 2).unwrap());
    assert!(mesh.set_leaf(queue, 3).is_err());
    assert!(!mesh.set_leaf(queue, 2).unwrap());
    assert!(mesh.set_leaf(queue, 0).unwrap());
    assert!(mesh.set_leaf(queue, 1).unwrap());
    assert!(mesh.set_leaf(queue, 2).unwrap());
}

// Render a palette-expanded face through opaque and OIT passes at the requested liquid opacity.
fn assert_mesh_opacity(name: &str, water_alpha: f32, slime_alpha: f32, expected_red: u8) {
    let (device, queue) = gpu();
    let mut bytes = bsp_bytes();
    let texture_name = &mut lump(&mut bytes, 2)[8..24];
    texture_name.fill(0);
    texture_name[..name.len()].copy_from_slice(name.as_bytes());
    let positions = [
        [-768_f32, 512., -768.],
        [0., 512., 768.],
        [768., 512., -768.],
    ];
    for (position, record) in positions
        .into_iter()
        .zip(lump(&mut bytes, 3).chunks_exact_mut(12))
    {
        for (value, field) in position.into_iter().zip(record.chunks_exact_mut(4)) {
            field.copy_from_slice(&value.to_le_bytes());
        }
    }
    lump(&mut bytes, 6)[..32].fill(0);
    let face = &mut lump(&mut bytes, 7)[..FACE_SIZE];
    face[12] = 0;
    face[16..20].copy_from_slice(&2_i32.to_le_bytes());
    let bsp = Bsp::decode(&bytes).unwrap();
    let lit_bytes = [
        b'Q', b'L', b'I', b'T', 1, 0, 0, 0, 0, 255, 0, 0, 0, 255, 160, 0, 0,
    ];
    let samples = sidecar::lit::decode(&lit_bytes).unwrap();
    let mut palette_bytes = [0; 768];
    palette_bytes[..3].copy_from_slice(&[255, 0, 0]);
    let palette = sidecar::lmp::decode(&palette_bytes).unwrap();
    let mut lightmap = lightmap::Builder::create(samples);
    let mut mesh = mesh::worldspawn::Mesh::build(&bsp, &mut lightmap, device, queue);
    let lightmaps = lightmap.upload(device, queue);
    let properties_buffer = crate::gpu::buffer::uniform::Properties::create(device);
    let mut data = crate::level::properties::build(&bsp);
    data.water.liquid_alpha = water_alpha;
    data.slime.liquid_alpha = slime_alpha;
    properties_buffer.write(queue, data.water);
    let properties = crate::gpu::bind_group::Properties::create(device, &properties_buffer);
    let slime_buffer = crate::gpu::buffer::uniform::Properties::create(device);
    slime_buffer.write(queue, data.slime);
    let slime_properties = crate::gpu::bind_group::Properties::create(device, &slime_buffer);
    let sky_pipeline = crate::gpu::pipeline::sky::create(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        include_str!("../../../shader/sky.wgsl"),
    );
    let liquid_pipeline = crate::gpu::pipeline::liquid::opaque::create(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        include_str!("../../../shader/liquid.wgsl"),
    );
    let transparent_pipeline = crate::gpu::pipeline::liquid::transparent::create(
        device,
        include_str!("../../../shader/liquid.wgsl"),
    );
    let composite_pipeline = crate::gpu::pipeline::composite::create(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        include_str!("../../../shader/composite.wgsl"),
    );
    let accumulation = crate::gpu::texture::Accumulation::create(device, 4, 4);
    let revealage = crate::gpu::texture::Revealage::create(device, 4, 4);
    let composite_binding =
        crate::gpu::bind_group::Composite::create(device, &accumulation, &revealage);
    let textures: Vec<_> = bsp
        .textures()
        .iter()
        .enumerate()
        .map(|(index, source)| {
            source
                .as_ref()
                .map(|_| crate::level::texture::build(&bsp, index, palette, device, queue))
        })
        .collect();
    drop(bsp);
    mesh.set_leaf(queue, 0).unwrap();
    let pipeline = crate::gpu::pipeline::solid::opaque::create(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        include_str!("../../../shader/solid.wgsl"),
    );
    let camera = crate::gpu::buffer::uniform::Camera::create(device);
    camera.write(
        queue,
        crate::gpu::buffer::uniform::camera::Data::construct(
            glam::Vec3::ZERO,
            glam::Vec2::ZERO,
            1.,
        ),
    );
    let camera_binding = crate::gpu::bind_group::Camera::create(device, &camera);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = crate::gpu::texture::Depth::create(device, 4, 4);
    let row_bytes = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row_bytes * 4),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = crate::gpu::render_pass::World::create(&mut encoder, &view, depth.view());
        mesh.draw_opaque(
            &mut pass,
            &pipeline,
            &camera_binding,
            &lightmaps,
            &textures,
            &sky_pipeline,
            &liquid_pipeline,
            &properties,
            water_alpha,
            &slime_properties,
            slime_alpha,
        );
    }
    {
        let mut pass = crate::gpu::render_pass::World::transparent(
            &mut encoder,
            &accumulation,
            &revealage,
            depth.view(),
        );
        mesh.draw_transparent(
            &mut pass,
            &transparent_pipeline,
            &camera_binding,
            &lightmaps,
            &textures,
            &properties,
            water_alpha,
            &slime_properties,
            slime_alpha,
        );
    }
    {
        let mut pass = crate::gpu::render_pass::Composite::create(&mut encoder, &view);
        pass.draw_transparency(&composite_pipeline, &composite_binding);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(4),
            },
        },
        target.size(),
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback.map_async(wgpu::MapMode::Read, .., move |result| {
        sender.send(result).unwrap();
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let pixels = readback.get_mapped_range(..).unwrap();
    let center = (row_bytes * 2 + 8) as usize;
    // The doubled 160/255 light sample saturates before transparency is blended.
    assert!(
        pixels[center].abs_diff(expected_red) <= 1,
        "{name} at opacity {water_alpha}, expected red {expected_red}; center RGBA: {:?}",
        &pixels[center..center + 4]
    );
    assert_eq!(&pixels[center + 1..center + 4], &[0, 0, 255]);
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn builds_palette_textures_and_routes_liquids_by_current_opacity() {
    for alpha in [0.0, 0.5, 1.0] {
        assert_mesh_opacity("wall", alpha, 1.0 - alpha, 255);
    }
    for (alpha, expected_red) in [(0.0, 0), (0.5, 128), (1.0, 255)] {
        assert_mesh_opacity("*water", alpha, 1.0 - alpha, expected_red);
        assert_mesh_opacity("*slime2", 1.0 - alpha, alpha, expected_red);
    }
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn builds_only_worldspawn_vertices_indices_and_lightmaps_with_original_face_ids() {
    let (device, queue) = gpu();
    for world_face in [0_usize, 1] {
        let mut bytes = bsp_bytes();
        let brush_face = 1 - world_face;
        let mut models = lump(&mut bytes, 14).to_vec();
        let mut brush_model = models.clone();
        models[56..60].copy_from_slice(&(world_face as i32).to_le_bytes());
        models[60..64].copy_from_slice(&1_i32.to_le_bytes());
        brush_model[56..60].copy_from_slice(&(brush_face as i32).to_le_bytes());
        brush_model[60..64].copy_from_slice(&1_i32.to_le_bytes());
        models.extend_from_slice(&brush_model);
        append_lump(&mut bytes, 14, &models);
        for mapping in lump(&mut bytes, 6).chunks_exact_mut(TEXINFO_SIZE) {
            mapping[..32].fill(0);
        }
        // Only the separate brush model has baked lighting to pack.
        let brush = &mut lump(&mut bytes, 7)[brush_face * FACE_SIZE..(brush_face + 1) * FACE_SIZE];
        brush[12] = 0;
        brush[16..20].copy_from_slice(&0_i32.to_le_bytes());
        let bsp = Bsp::decode(&bytes).unwrap();
        let mut lightmaps = lightmap::Builder::create(&[[42; 3]]);
        let mesh = mesh::worldspawn::Mesh::build(&bsp, &mut lightmaps, device, queue);
        assert_eq!(
            mesh.vertex_buffer.buffer().size(),
            (3 * std::mem::size_of::<crate::gpu::buffer::vertex::world::Data>()) as u64,
        );
        assert_eq!(mesh.leaves[0].indices, [0, 2, 1]);
        assert_eq!(mesh.leaves[0].batches.len(), 1);
        assert!(
            lightmaps.finish().is_empty(),
            "brush model lighting was packed"
        );
    }
}
