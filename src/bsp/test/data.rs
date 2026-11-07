use super::super::{ClipChild, ClipRoot, Contents, Data, Direction, Entity, RenderRoot};

const VERSION: i32 = 29;
const HEADER_SIZE: usize = 124;
const ENTITIES: usize = 0;
const PLANES: usize = 1;
const TEXTURES: usize = 2;
const MARKSURFACES: usize = 11;
const VERTICES: usize = 3;
const VISIBILITY: usize = 4;
const NODES: usize = 5;
const TEXINFO: usize = 6;
const TEXINFO_BYTES: [u8; 400] = [0; 400];
const FACES: usize = 7;
const LIGHTING: usize = 8;
const CLIPNODES: usize = 9;
const LEAVES: usize = 10;
const EDGES: usize = 12;
const SURFEDGES: usize = 13;
const MODELS: usize = 14;

// Wrap independent lump payloads in a BSP29 header.
fn file(lumps: &[(usize, &[u8])]) -> Vec<u8> {
    let mut bytes = vec![0; HEADER_SIZE];
    bytes[..4].copy_from_slice(&VERSION.to_le_bytes());
    for entry in bytes[4..HEADER_SIZE].chunks_exact_mut(8) {
        entry[..4].copy_from_slice(&(HEADER_SIZE as i32).to_le_bytes());
    }
    for (index, payload) in [(ENTITIES, b"{\"classname\" \"worldspawn\"}".as_slice())]
        .into_iter()
        .chain(lumps.iter().copied())
    {
        let entry = 4 + index * 8;
        let offset = bytes.len() as i32;
        bytes[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
        bytes[entry + 4..entry + 8].copy_from_slice(&(payload.len() as i32).to_le_bytes());
        bytes.extend_from_slice(payload);
    }
    bytes
}

// Supply a minimal world model and solid leaf unless explicitly replaced.
fn world_file(lumps: &[(usize, &[u8])]) -> Vec<u8> {
    let mut model = [0; 64];
    for root in model[36..52].chunks_exact_mut(4) {
        root.copy_from_slice(&(-1_i32).to_le_bytes());
    }
    let mut leaf = [0; 28];
    leaf[..4].copy_from_slice(&(-2_i32).to_le_bytes());
    leaf[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    let planes = [0; 200];
    let mut textures = vec![0; 388];
    textures[..4].copy_from_slice(&1_i32.to_le_bytes());
    textures[4..8].copy_from_slice(&8_i32.to_le_bytes());
    textures[8..12].copy_from_slice(b"wall");
    textures[24..28].copy_from_slice(&16_u32.to_le_bytes());
    textures[28..32].copy_from_slice(&16_u32.to_le_bytes());
    for (level, offset) in [40_u32, 296, 360, 376].into_iter().enumerate() {
        let field = 32 + level * 4;
        textures[field..field + 4].copy_from_slice(&offset.to_le_bytes());
    }
    let defaults = [
        (PLANES, planes.as_slice()),
        (TEXTURES, textures.as_slice()),
        (MODELS, model.as_slice()),
        (LEAVES, leaf.as_slice()),
    ];
    let lumps: Vec<_> = defaults.into_iter().chain(lumps.iter().copied()).collect();
    file(&lumps)
}

// Build a face with references to unsupported lumps and unused light style slots.
fn face() -> [u8; 20] {
    let mut record = [0; 20];
    record[..2].copy_from_slice(&7_i16.to_le_bytes());
    record[2..4].copy_from_slice(&1_i16.to_le_bytes());
    record[4..8].copy_from_slice(&1_i32.to_le_bytes());
    record[8..10].copy_from_slice(&2_i16.to_le_bytes());
    record[10..12].copy_from_slice(&9_i16.to_le_bytes());
    record[12..16].copy_from_slice(&[0, 3, 255, 255]);
    record[16..20].copy_from_slice(&(-1_i32).to_le_bytes());
    record
}

// Make a fixture with a dummy edge, shared references, and an unreferenced vertex.
fn geometry(record: &[u8]) -> Vec<u8> {
    let positions: Vec<_> = [
        [1.25_f32, -2.5, 64.0],
        [0.0, 128.0, -0.125],
        [99.0, 100.0, 101.0],
    ]
    .into_iter()
    .flatten()
    .flat_map(f32::to_le_bytes)
    .collect();
    let edges: Vec<_> = [0_u16, 0, 0, 1]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let surfedges: Vec<_> = [0_i32, 1, -1]
        .into_iter()
        .flat_map(i32::to_le_bytes)
        .collect();
    // Deliberately use a different physical order from the lump directory.
    world_file(&[
        (FACES, record),
        (TEXINFO, &TEXINFO_BYTES),
        (SURFEDGES, &surfedges),
        (VERTICES, &positions),
        (EDGES, &edges),
    ])
}

// Borrow a fixture lump for malformed-record tests.
fn lump(bytes: &mut [u8], index: usize) -> &mut [u8] {
    let entry = 4 + index * 8;
    let offset = i32::from_le_bytes(bytes[entry..entry + 4].try_into().unwrap()) as usize;
    let length = i32::from_le_bytes(bytes[entry + 4..entry + 8].try_into().unwrap()) as usize;
    &mut bytes[offset..offset + length]
}

// Build a solid leaf zero followed by empty leaves with no PVS references.
fn leaves(count: usize) -> Vec<u8> {
    let mut bytes = vec![0; count * 28];
    for (index, record) in bytes.chunks_exact_mut(28).enumerate() {
        let contents = if index == 0 { -2_i32 } else { -1_i32 };
        record[..4].copy_from_slice(&contents.to_le_bytes());
        record[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    }
    bytes
}

// Build world and brush models sharing a pair of face records.
fn model_file() -> Vec<u8> {
    let mut faces = face().repeat(2);
    for record in faces.chunks_exact_mut(20) {
        record[4..10].fill(0);
    }
    let mut models = [0; 128];
    for (index, record) in models.chunks_exact_mut(64).enumerate() {
        record[36..40].copy_from_slice(&(-(index as i32)).to_le_bytes());
        for root in record[40..52].chunks_exact_mut(4) {
            root.copy_from_slice(&(-1_i32).to_le_bytes());
        }
        record[52..56].copy_from_slice(&(9_i32 - index as i32).to_le_bytes());
        record[56..60].copy_from_slice(&(index as i32).to_le_bytes());
        record[60..64].copy_from_slice(&1_i32.to_le_bytes());
    }
    let mut nodes = [0; 24];
    nodes[4..6].copy_from_slice(&(-1_i16).to_le_bytes());
    nodes[6..8].copy_from_slice(&(-2_i16).to_le_bytes());
    world_file(&[
        (MODELS, &models),
        (FACES, &faces),
        (NODES, &nodes),
        (TEXINFO, &TEXINFO_BYTES),
        (LEAVES, &leaves(10)),
    ])
}

#[test]
fn preserves_independent_records_and_unpacks_semantics() {
    let data = Data::decode(&geometry(&face())).unwrap();
    assert_eq!(data.vertices().len(), 3);
    assert_eq!(data.vertices()[0].position, [1.25, -2.5, 64.0]);
    assert_eq!(data.vertices()[1].position, [0.0, 128.0, -0.125]);
    assert_eq!(data.vertices()[2].position, [99.0, 100.0, 101.0]);
    assert_eq!(data.edges().len(), 2);
    assert_eq!(data.edges()[0].vertices, [0, 0]);
    assert_eq!(data.edges()[1].vertices, [0, 1]);
    assert_eq!(data.surfedges().len(), 3);
    for (surfedge, edge, direction) in [
        (&data.surfedges()[0], 0, Direction::Forward),
        (&data.surfedges()[1], 1, Direction::Forward),
        (&data.surfedges()[2], 1, Direction::Reverse),
    ] {
        assert_eq!(surfedge.edge, edge);
        assert_eq!(surfedge.direction, direction);
    }
    assert_eq!(data.faces().len(), 1);
    let face = &data.faces()[0];
    assert_eq!(face.surfedges, 1..3);
    assert_eq!(face.texinfo, 9);
    assert_eq!(face.styles, [Some(0), Some(3), None, None]);
    assert_eq!(face.lighting_offset, None);
}

#[test]
fn exposes_decoded_entity_maps_and_rejects_malformed_entity_text() {
    let bytes = world_file(&[(
        ENTITIES,
        b"{\"classname\" \"worldspawn\" \"_wateralpha\" \"0.5\"}\0",
    )]);
    let data = Data::decode(&bytes).unwrap();
    assert_eq!(data.entities().len(), 1);
    let Entity::Worldspawn(world) = &data.entities()[0] else {
        panic!("expected worldspawn")
    };
    assert_eq!(world.water_alpha, Some(0.5));
    assert!(Data::decode(&world_file(&[(ENTITIES, b"")])).is_err());
    assert!(Data::decode(&world_file(&[(ENTITIES, b"{\"missing value\"}")])).is_err());
}

#[test]
fn preserves_style_slots_and_lighting_byte_offsets() {
    for offset in [0_i32, 12345] {
        let mut record = face();
        record[2..4].copy_from_slice(&0_i16.to_le_bytes());
        record[12..16].copy_from_slice(&[255, 0, 254, 255]);
        record[16..20].copy_from_slice(&offset.to_le_bytes());
        let data = Data::decode(&geometry(&record)).unwrap();
        assert_eq!(data.faces()[0].styles, [None, Some(0), Some(254), None]);
        assert_eq!(data.faces()[0].lighting_offset, Some(offset as usize));
    }
}

#[test]
fn accepts_empty_optional_lumps_with_a_world_model() {
    let data = Data::decode(&world_file(&[])).unwrap();
    assert!(data.vertices().is_empty());
    assert!(data.edges().is_empty());
    assert!(data.surfedges().is_empty());
    assert!(data.faces().is_empty());
    assert_eq!(data.models().len(), 1);
    assert_eq!(data.models()[0].render_root, RenderRoot::Leaf(0));
    assert!(data.nodes().is_empty());
    assert_eq!(data.leaves().len(), 1);
    assert_eq!(data.leaves()[0].contents, Contents::Solid);
    assert!(data.texinfo().is_empty());
    assert!(data.clipnodes().is_empty());
    assert!(data.visibility().is_empty());
}

#[test]
fn rejects_partial_records_in_each_supported_lump() {
    for (index, size) in [
        (PLANES, 20),
        (MARKSURFACES, 2),
        (VERTICES, 12),
        (EDGES, 4),
        (SURFEDGES, 4),
        (FACES, 20),
        (NODES, 24),
        (TEXINFO, 40),
        (CLIPNODES, 8),
        (LEAVES, 28),
        (MODELS, 64),
    ] {
        for length in 1..size {
            assert!(Data::decode(&world_file(&[(index, &vec![0; length])])).is_err());
        }
    }
}

#[test]
fn validates_edges_and_surfedges_even_when_no_face_uses_them() {
    for endpoint in 0..2 {
        let mut bytes = geometry(&[]);
        lump(&mut bytes, EDGES)[endpoint * 2..endpoint * 2 + 2]
            .copy_from_slice(&3_u16.to_le_bytes());
        assert_eq!(
            Data::decode(&bytes).unwrap_err().to_string(),
            "edge references missing vertex"
        );
    }
    for index in [2_i32, -2, i32::MIN, i32::MAX] {
        let mut bytes = geometry(&[]);
        lump(&mut bytes, SURFEDGES)[..4].copy_from_slice(&index.to_le_bytes());
        assert_eq!(
            Data::decode(&bytes).unwrap_err().to_string(),
            "surfedge references missing edge"
        );
    }
}

#[test]
fn rejects_negative_face_indices_and_counts() {
    for offset in [0, 8, 10] {
        let mut record = face();
        record[offset..offset + 2].copy_from_slice(&(-1_i16).to_le_bytes());
        assert!(Data::decode(&geometry(&record)).is_err());
    }
    for offset in [-2_i32, i32::MIN] {
        let mut record = face();
        record[16..20].copy_from_slice(&offset.to_le_bytes());
        assert!(Data::decode(&geometry(&record)).is_err());
    }
}

#[test]
fn checks_face_boundary_ranges_including_empty_ones() {
    for (start, count, valid) in [
        (0_i32, 3_i16, true),
        (3, 0, true),
        (4, 0, false),
        (2, 2, false),
        (-1, 0, false),
        (i32::MAX, 1, false),
    ] {
        let mut record = face();
        record[4..8].copy_from_slice(&start.to_le_bytes());
        record[8..10].copy_from_slice(&count.to_le_bytes());
        assert_eq!(
            Data::decode(&geometry(&record)).is_ok(),
            valid,
            "{start}, {count}"
        );
    }
}

#[test]
fn preserves_world_and_brush_model_order_and_references() {
    let data = Data::decode(&model_file()).unwrap();
    assert_eq!(data.faces().len(), 2);
    assert_eq!(data.models().len(), 2);
    let world = &data.models()[0];
    assert_eq!(world.render_root, RenderRoot::Node(0));
    assert_eq!(world.clip_roots, [ClipRoot::Contents(Contents::Empty); 3]);
    assert_eq!(world.visleafs, 9);
    assert_eq!(world.faces, 0..1);
    let brush = &data.models()[1];
    assert_eq!(brush.render_root, RenderRoot::Leaf(0));
    assert_eq!(brush.visleafs, 8);
    assert_eq!(brush.faces, 1..2);
}

#[test]
fn validates_model_face_references_against_decoded_faces() {
    let original = model_file();
    for model in [0, 1] {
        for (start, count, valid) in [(2_i32, 0_i32, true), (3, 0, false), (1, 2, false)] {
            let mut bytes = original.clone();
            let record = &mut lump(&mut bytes, MODELS)[model * 64..(model + 1) * 64];
            record[56..60].copy_from_slice(&start.to_le_bytes());
            record[60..64].copy_from_slice(&count.to_le_bytes());
            assert_eq!(
                Data::decode(&bytes).is_ok(),
                valid,
                "model {model}: {start}, {count}"
            );
        }
    }
}

#[test]
fn decodes_clipnodes_and_validates_model_roots() {
    let mut clipnodes = [0; 16];
    clipnodes[..4].copy_from_slice(&7_i32.to_le_bytes());
    clipnodes[4..6].copy_from_slice(&1_i16.to_le_bytes());
    clipnodes[6..8].copy_from_slice(&(-2_i16).to_le_bytes());
    clipnodes[8..12].copy_from_slice(&9_i32.to_le_bytes());
    clipnodes[12..14].copy_from_slice(&(-1_i16).to_le_bytes());
    clipnodes[14..16].copy_from_slice(&(-3_i16).to_le_bytes());
    let mut models = [0; 128];
    for record in models.chunks_exact_mut(64) {
        record[36..40].copy_from_slice(&(-1_i32).to_le_bytes());
        for root in record[40..52].chunks_exact_mut(4) {
            root.copy_from_slice(&1_i32.to_le_bytes());
        }
    }
    let bytes = world_file(&[
        (MODELS, &models),
        (CLIPNODES, &clipnodes),
        (LEAVES, &leaves(1)),
    ]);
    let data = Data::decode(&bytes).unwrap();
    assert_eq!(data.clipnodes().len(), 2);
    assert_eq!(data.clipnodes()[0].plane, 7);
    assert_eq!(data.clipnodes()[0].front, ClipChild::Clipnode(1));
    assert_eq!(
        data.clipnodes()[0].back,
        ClipChild::Contents(Contents::Solid)
    );
    assert_eq!(data.clipnodes()[1].plane, 9);
    assert_eq!(
        data.clipnodes()[1].front,
        ClipChild::Contents(Contents::Empty)
    );
    assert_eq!(
        data.clipnodes()[1].back,
        ClipChild::Contents(Contents::Water)
    );
    assert_eq!(data.models()[0].clip_roots, [ClipRoot::Clipnode(1); 3]);
    for model in 0..2 {
        for hull in 0..3 {
            for value in [2_i32, i32::MAX] {
                let mut invalid = bytes.clone();
                let offset = model * 64 + 40 + hull * 4;
                lump(&mut invalid, MODELS)[offset..offset + 4]
                    .copy_from_slice(&value.to_le_bytes());
                assert_eq!(
                    Data::decode(&invalid).unwrap_err().to_string(),
                    "model references missing clipnode"
                );
            }
        }
    }
    assert!(Data::decode(&world_file(&[(MODELS, &models)])).is_err());
}

#[test]
fn decodes_visibility_using_the_world_models_leaf_count() {
    let mut source = model_file();
    let models = lump(&mut source, MODELS).to_vec();
    let faces = lump(&mut source, FACES).to_vec();
    let nodes = lump(&mut source, NODES).to_vec();
    // World has nine visibility leaves, brush model has eight.
    let bytes = world_file(&[
        (VISIBILITY, &[1, 1, 2, 0, 1]),
        (MODELS, &models),
        (FACES, &faces),
        (NODES, &nodes),
        (TEXINFO, &TEXINFO_BYTES),
        (LEAVES, &leaves(10)),
    ]);
    let data = Data::decode(&bytes).unwrap();
    assert_eq!(data.visibility(), &[1, 9, 2]);
}

#[test]
fn rejects_nonempty_visibility_without_a_world_or_visibility_leaves() {
    let bytes = file(&[(VISIBILITY, &[1])]);
    assert_eq!(
        Data::decode(&bytes).unwrap_err().to_string(),
        "missing world model"
    );
    let mut source = model_file();
    let mut models = lump(&mut source, MODELS).to_vec();
    let faces = lump(&mut source, FACES).to_vec();
    let nodes = lump(&mut source, NODES).to_vec();
    models[52..56].fill(0);
    let bytes = world_file(&[
        (VISIBILITY, &[1]),
        (MODELS, &models),
        (FACES, &faces),
        (NODES, &nodes),
        (TEXINFO, &TEXINFO_BYTES),
        (LEAVES, &leaves(10)),
    ]);
    assert_eq!(
        Data::decode(&bytes).unwrap_err().to_string(),
        "nonempty visibility lump with no visibility leaves"
    );
}

#[test]
fn validates_model_render_roots_against_decoded_nodes() {
    let original = model_file();
    let data = Data::decode(&original).unwrap();
    assert_eq!(data.nodes().len(), 1);
    assert_eq!(data.nodes()[0].front, super::super::Child::Leaf(0));
    assert_eq!(data.nodes()[0].back, super::super::Child::Leaf(1));
    for model in 0..2 {
        for root in [1_i32, i32::MAX] {
            let mut invalid = original.clone();
            let offset = model * 64 + 36;
            lump(&mut invalid, MODELS)[offset..offset + 4].copy_from_slice(&root.to_le_bytes());
            assert_eq!(
                Data::decode(&invalid).unwrap_err().to_string(),
                "model references missing node"
            );
        }
    }
}

#[test]
fn validates_all_face_texinfo_references() {
    for index in [0_i16, 9, 10, i16::MAX] {
        let mut record = face();
        record[10..12].copy_from_slice(&index.to_le_bytes());
        let result = Data::decode(&geometry(&record));
        if index < 10 {
            let data = result.unwrap();
            assert_eq!(data.texinfo().len(), 10);
            assert_eq!(data.faces()[0].texinfo, index as usize);
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                "face references missing texinfo"
            );
        }
    }
    // Faces are checked even when no model references them.
    let mut bytes = geometry(&face());
    let entry = 4 + TEXINFO * 8;
    bytes[entry + 4..entry + 8].copy_from_slice(&0_i32.to_le_bytes());
    assert_eq!(
        Data::decode(&bytes).unwrap_err().to_string(),
        "face references missing texinfo"
    );
}

#[test]
fn ignores_bsp_lighting_and_appended_extensions() {
    let mut bytes = world_file(&[(LIGHTING, &[1, 2, 3, 4, 5])]);
    bytes.extend_from_slice(b"BSPX\xff\xff\xff\xffDECOUPLED_LM RGBLIGHTING");
    assert!(Data::decode(&bytes).is_ok());
}

#[test]
fn resolves_leaf_visibility_offsets_to_shared_ranges() {
    let mut models = [0; 64];
    for root in models[36..52].chunks_exact_mut(4) {
        root.copy_from_slice(&(-1_i32).to_le_bytes());
    }
    models[52..56].copy_from_slice(&9_i32.to_le_bytes());
    let mut leaf_bytes = leaves(10);
    for (index, offset) in [(1_usize, 0_i32), (2, 0), (3, 2)] {
        let start = index * 28 + 4;
        leaf_bytes[start..start + 4].copy_from_slice(&offset.to_le_bytes());
    }
    let data = Data::decode(&world_file(&[
        (LEAVES, &leaf_bytes),
        (MODELS, &models),
        (VISIBILITY, &[5, 1, 0, 2]),
    ]))
    .unwrap();
    assert_eq!(data.leaves().len(), 10);
    assert_eq!(data.leaves()[0].contents, Contents::Solid);
    assert_eq!(data.leaves()[0].pvs, None);
    assert_eq!(data.leaves()[1].pvs, Some(0..3));
    assert_eq!(data.leaves()[2].pvs, Some(0..3));
    assert_eq!(data.leaves()[3].pvs, Some(3..3));
    assert_eq!(data.leaves()[4].pvs, None);
    assert_eq!(
        &data.visibility()[data.leaves()[1].pvs.clone().unwrap()],
        &[1, 3, 9]
    );
}

#[test]
fn validates_node_and_model_leaf_references_and_visibility_counts() {
    let original = model_file();
    for child in [4_usize, 6] {
        let mut bytes = original.clone();
        lump(&mut bytes, NODES)[child..child + 2].copy_from_slice(&(-11_i16).to_le_bytes());
        assert_eq!(
            Data::decode(&bytes).unwrap_err().to_string(),
            "node references missing leaf"
        );
    }
    for model in 0..2 {
        let mut bytes = original.clone();
        let offset = model * 64 + 36;
        lump(&mut bytes, MODELS)[offset..offset + 4].copy_from_slice(&(-11_i32).to_le_bytes());
        assert_eq!(
            Data::decode(&bytes).unwrap_err().to_string(),
            "model references missing leaf"
        );
        let mut bytes = original.clone();
        let offset = model * 64 + 52;
        lump(&mut bytes, MODELS)[offset..offset + 4].copy_from_slice(&10_i32.to_le_bytes());
        assert_eq!(
            Data::decode(&bytes).unwrap_err().to_string(),
            "model visibility leaf count exceeds leaf lump"
        );
    }
}

#[test]
fn requires_a_world_model_even_without_visibility() {
    for bytes in [file(&[]), file(&[(VISIBILITY, &[1])])] {
        let error = Data::decode(&bytes).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "missing world model");
    }
    let mut bytes = model_file();
    let entry = 4 + MODELS * 8;
    bytes[entry + 4..entry + 8].copy_from_slice(&0_i32.to_le_bytes());
    assert_eq!(
        Data::decode(&bytes).unwrap_err().to_string(),
        "missing world model"
    );
}

// Build a world whose nonzero render root splits on an axial and an oblique plane.
fn spatial_file() -> Vec<u8> {
    let mut source = geometry(&face().repeat(2));
    let mut planes = Vec::new();
    for (normal, distance, kind) in [
        ([1.0_f32, 0., 0.], 4.0_f32, 0_i32),
        ([0., 0.5, 0.75], 1.5, 5),
    ] {
        for value in normal.into_iter().chain([distance]) {
            planes.extend_from_slice(&value.to_le_bytes());
        }
        planes.extend_from_slice(&kind.to_le_bytes());
    }
    let mut faces = lump(&mut source, FACES).to_vec();
    for face in faces.chunks_exact_mut(20) {
        face[..2].copy_from_slice(&0_i16.to_le_bytes());
    }
    let mut nodes = [0; 48];
    nodes[..4].copy_from_slice(&1_i32.to_le_bytes());
    nodes[4..6].copy_from_slice(&(-2_i16).to_le_bytes());
    nodes[6..8].copy_from_slice(&(-3_i16).to_le_bytes());
    nodes[28..30].copy_from_slice(&0_i16.to_le_bytes());
    nodes[30..32].copy_from_slice(&(-1_i16).to_le_bytes());
    let mut model = [0; 64];
    model[36..40].copy_from_slice(&1_i32.to_le_bytes());
    for root in model[40..52].chunks_exact_mut(4) {
        root.copy_from_slice(&(-1_i32).to_le_bytes());
    }
    model[52..56].copy_from_slice(&2_i32.to_le_bytes());
    model[60..64].copy_from_slice(&2_i32.to_le_bytes());
    let mut leaf_bytes = leaves(3);
    leaf_bytes[28 + 22..28 + 24].copy_from_slice(&2_u16.to_le_bytes());
    leaf_bytes[56 + 20..56 + 22].copy_from_slice(&1_u16.to_le_bytes());
    leaf_bytes[56 + 22..56 + 24].copy_from_slice(&2_u16.to_le_bytes());
    let marks: Vec<_> = [0_u16, 0, 1]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let vertices = lump(&mut source, VERTICES).to_vec();
    let edges = lump(&mut source, EDGES).to_vec();
    let surfedges = lump(&mut source, SURFEDGES).to_vec();
    world_file(&[
        (PLANES, &planes),
        (MODELS, &model),
        (NODES, &nodes),
        (LEAVES, &leaf_bytes),
        (MARKSURFACES, &marks),
        (FACES, &faces),
        (TEXINFO, &TEXINFO_BYTES),
        (VERTICES, &vertices),
        (EDGES, &edges),
        (SURFEDGES, &surfedges),
    ])
}

#[test]
fn finds_world_leaves_from_axial_and_oblique_planes_and_resolves_marksurfaces() {
    let data = Data::decode(&spatial_file()).unwrap();
    assert_eq!(data.planes().len(), 2);
    assert_eq!(data.marksurfaces(), [0, 0, 1]);
    assert_eq!(
        &data.marksurfaces()[data.leaves()[1].marksurfaces.clone()],
        [0, 0]
    );
    assert_eq!(
        &data.marksurfaces()[data.leaves()[2].marksurfaces.clone()],
        [0, 1]
    );
    for (position, leaf) in [
        ([3., 100., 0.], 0),
        ([4., 100., 0.], 0),
        ([5., 0., 0.], 2),
        ([5., 5., 0.], 1),
        ([5., 0., 2.], 2),
    ] {
        assert_eq!(data.leaf_at(position), Some(leaf));
    }
    let mut bytes = spatial_file();
    lump(&mut bytes, MODELS)[36..40].copy_from_slice(&(-3_i32).to_le_bytes());
    assert_eq!(Data::decode(&bytes).unwrap().leaf_at([0.; 3]), Some(2));
}

#[test]
fn leaf_lookup_handles_invalid_positions_and_cycles_without_hanging() {
    let mut bytes = spatial_file();
    let data = Data::decode(&bytes).unwrap();
    assert_eq!(data.leaf_at([f32::NAN, 0., 0.]), None);
    assert_eq!(data.leaf_at([0., f32::INFINITY, 0.]), None);
    lump(&mut bytes, NODES)[4..6].copy_from_slice(&1_i16.to_le_bytes());
    assert_eq!(Data::decode(&bytes).unwrap().leaf_at([5., 5., 0.]), None);
}

#[test]
fn validates_plane_references_and_leaf_marksurface_ranges() {
    for (index, width) in [(FACES, 2), (NODES, 4), (CLIPNODES, 4)] {
        let mut bytes = if index == CLIPNODES {
            world_file(&[(CLIPNODES, &[0, 0, 0, 0, 255, 255, 254, 255])])
        } else {
            spatial_file()
        };
        let count = Data::decode(&bytes).unwrap().planes().len() as i32;
        lump(&mut bytes, index)[..width].copy_from_slice(&count.to_le_bytes()[..width]);
        assert!(
            Data::decode(&bytes)
                .unwrap_err()
                .to_string()
                .contains("missing plane")
        );
    }
    let mut bytes = spatial_file();
    lump(&mut bytes, MARKSURFACES)[..2].copy_from_slice(&2_u16.to_le_bytes());
    assert_eq!(
        Data::decode(&bytes).unwrap_err().to_string(),
        "marksurface references missing face"
    );
    for (start, count, valid) in [(3_u16, 0_u16, true), (4, 0, false), (2, 2, false)] {
        let mut bytes = spatial_file();
        let leaf = &mut lump(&mut bytes, LEAVES)[28..56];
        leaf[20..22].copy_from_slice(&start.to_le_bytes());
        leaf[22..24].copy_from_slice(&count.to_le_bytes());
        assert_eq!(Data::decode(&bytes).is_ok(), valid);
    }
}

#[test]
fn decodes_indexed_textures_and_validates_even_unreferenced_texinfo_textures() {
    let data = Data::decode(&geometry(&face())).unwrap();
    let texture = data.textures()[0].as_ref().unwrap();
    assert_eq!(texture.name, "wall");
    assert_eq!(texture.dimensions, [16; 2]);
    assert_eq!(
        texture.mips.each_ref().map(|mip| mip.len()),
        [256, 64, 16, 4]
    );
    for index in [1_i32, i32::MAX] {
        let mut bytes = geometry(&face());
        // This texinfo is not used by the fixture face, but still must be valid.
        lump(&mut bytes, TEXINFO)[32..36].copy_from_slice(&index.to_le_bytes());
        assert_eq!(
            Data::decode(&bytes).unwrap_err().to_string(),
            "texinfo references missing texture"
        );
    }
    let mut bytes = geometry(&face());
    lump(&mut bytes, TEXTURES)[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    assert_eq!(
        Data::decode(&bytes).unwrap_err().to_string(),
        "texinfo references missing texture"
    );
    let mut textures = [0; 8];
    textures[..4].copy_from_slice(&1_i32.to_le_bytes());
    textures[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    let data = Data::decode(&world_file(&[(TEXTURES, &textures)])).unwrap();
    assert_eq!(data.textures(), &[None]);
}
