use glam::{UVec2, Vec3, Vec4};

use crate::bsp::{self, Data};

use super::super::{MeshData, TextureRange, triangulate};
use super::Visibility;

// Build a clockwise quad whose boundary uses both edge directions.
fn quad() -> Data {
    Data {
        planes: vec![],
        nodes: vec![],
        leaves: vec![],
        visibility: vec![],
        marksurfaces: vec![],
        textures: vec![bsp::Texture {
            name: String::new(),
            dimensions: UVec2::splat(16),
        }],
        texinfo: vec![bsp::Texinfo {
            axes: [Vec4::ZERO; 2],
            texture: 0,
        }],
        vertices: vec![Vec3::ZERO, Vec3::Y, Vec3::X + Vec3::Y, Vec3::X],
        edges: vec![[0, 0], [0, 1], [2, 1], [2, 3], [0, 3]],
        surfedges: vec![1, -2, 3, -4],
        faces: vec![bsp::Face {
            first_surfedge: 0,
            num_surfedges: 4,
            texinfo: 0,
        }],
        models: vec![bsp::Model {
            headnode: 0,
            visleafs: 0,
            first_face: 0,
            num_faces: 1,
        }],
    }
}

#[test]
fn triangulates_signed_edges_into_counterclockwise_fans() {
    let data = quad();
    let MeshData {
        indices,
        vertices,
        ranges,
        ..
    } = triangulate(&data).unwrap();
    assert_eq!(indices, [0, 2, 1, 0, 3, 2]);
    assert_eq!(
        ranges,
        [TextureRange {
            model_id: 0,
            texture_id: 0,
            indices: 0..6
        }]
    );
    assert_eq!(
        vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
        data.vertices
    );
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = std::array::from_fn(|i| vertices[triangle[i] as usize].position);
        assert!((b - a).cross(c - a).z > 0.0);
    }
}

#[test]
fn appends_faces_to_one_index_array_without_duplicating_vertices() {
    let mut data = quad();
    data.faces.push(bsp::Face {
        first_surfedge: 1,
        num_surfedges: 3,
        texinfo: 0,
    });
    data.models[0].num_faces = 2;
    let MeshData {
        indices,
        vertices,
        ranges,
        ..
    } = triangulate(&data).unwrap();
    assert_eq!(
        ranges,
        [TextureRange {
            model_id: 0,
            texture_id: 0,
            indices: 0..9
        }]
    );
    assert_eq!(indices, [0, 2, 1, 0, 3, 2, 1, 3, 2]);
    assert_eq!(vertices.len(), 4);
}

#[test]
fn rejects_invalid_face_and_edge_references() {
    let mut cases = Vec::new();
    let mut data = quad();
    data.faces[0].num_surfedges = 2;
    cases.push(data);
    let mut data = quad();
    data.faces[0].first_surfedge = -1;
    cases.push(data);
    let mut data = quad();
    data.faces[0].first_surfedge = i32::MAX;
    cases.push(data);
    let mut data = quad();
    data.faces[0].num_surfedges = 5;
    cases.push(data);
    for invalid in [0, 5, -5, i32::MIN] {
        let mut data = quad();
        data.surfedges[0] = invalid;
        cases.push(data);
    }
    let mut data = quad();
    data.edges[2][1] = u16::MAX;
    cases.push(data);
    for data in cases {
        assert!(triangulate(&data).is_err());
    }
}

#[test]
fn triangulates_the_librequake_fixture() {
    let data = Data::decode(include_bytes!("../../assets/maps/librequake/lq_e0m1.bsp")).unwrap();
    let MeshData {
        indices,
        vertices,
        ranges,
        ..
    } = triangulate(&data).unwrap();
    assert!(vertices.len() >= data.vertices.len());
    assert!(vertices.iter().all(|vertex| vertex.uv.is_finite()));
    assert_eq!(indices.len(), 26_979);
    let mut end = 0;
    let mut previous = None;
    for range in ranges {
        assert_eq!(range.indices.start, end);
        assert!(range.indices.end > range.indices.start);
        assert_eq!((range.indices.end - range.indices.start) % 3, 0);
        let key = (range.model_id, range.texture_id);
        assert!(previous.is_none_or(|previous| previous < key));
        assert!(range.model_id < data.models.len());
        assert!(range.texture_id < data.textures.len());
        previous = Some(key);
        end = range.indices.end;
    }
    assert_eq!(end as usize, indices.len());
    assert!(
        indices
            .iter()
            .all(|&index| (index as usize) < vertices.len())
    );
}

#[test]
fn groups_noncontiguous_faces_by_model_then_texture() {
    let mut data = quad();
    data.textures.push(bsp::Texture {
        name: String::new(),
        dimensions: UVec2::splat(32),
    });
    data.texinfo.push(bsp::Texinfo {
        axes: [Vec4::ZERO; 2],
        texture: 1,
    });
    data.faces = [1, 0, 1, 0]
        .into_iter()
        .map(|texinfo| bsp::Face {
            first_surfedge: if texinfo == 0 { 1 } else { 0 },
            num_surfedges: 3,
            texinfo,
        })
        .collect();
    data.models = vec![
        bsp::Model {
            headnode: 0,
            visleafs: 0,
            first_face: 0,
            num_faces: 3,
        },
        bsp::Model {
            headnode: 0,
            visleafs: 0,
            first_face: 3,
            num_faces: 1,
        },
    ];
    let mesh = triangulate(&data).unwrap();
    assert_eq!(mesh.indices, [0, 2, 1, 3, 5, 4, 3, 5, 4, 0, 2, 1]);
    assert_eq!(
        mesh.ranges,
        [
            TextureRange {
                model_id: 0,
                texture_id: 0,
                indices: 0..3
            },
            TextureRange {
                model_id: 0,
                texture_id: 1,
                indices: 3..9
            },
            TextureRange {
                model_id: 1,
                texture_id: 0,
                indices: 9..12
            },
        ]
    );
    assert_eq!(
        data.faces
            .iter()
            .map(|face| face.texinfo)
            .collect::<Vec<_>>(),
        [1, 0, 1, 0]
    );
    assert_eq!(mesh.vertices.len(), 6);
}

#[test]
fn rejects_invalid_model_and_texture_references() {
    for (start, count) in [(-1, 1), (0, -1), (1, 1), (0, 2), (i32::MAX, i32::MAX)] {
        let mut data = quad();
        data.models[0].first_face = start;
        data.models[0].num_faces = count;
        assert!(triangulate(&data).is_err());
    }
    let mut data = quad();
    data.faces[0].texinfo = 1;
    assert!(triangulate(&data).is_err());
    for texture in [-1, 1] {
        let mut data = quad();
        data.texinfo[0].texture = texture;
        assert!(triangulate(&data).is_err());
    }
}

#[test]
fn computes_uvs_from_axes_and_offsets_and_splits_texture_seams() {
    let mut data = quad();
    data.texinfo[0].axes = [
        Vec4::new(16.0, 0.0, 0.0, 8.0),
        Vec4::new(0.0, 32.0, 0.0, -16.0),
    ];
    data.texinfo.push(bsp::Texinfo {
        axes: [Vec4::ZERO; 2],
        texture: 0,
    });
    data.faces.push(bsp::Face {
        first_surfedge: 0,
        num_surfedges: 4,
        texinfo: 1,
    });
    data.models[0].num_faces = 2;
    let mesh = triangulate(&data).unwrap();
    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(mesh.vertices[0].uv, glam::Vec2::new(0.5, -1.0));
    assert_eq!(mesh.vertices[2].uv, glam::Vec2::new(1.5, 1.0));
    assert_eq!(mesh.vertices[4].position, mesh.vertices[0].position);
    assert_eq!(mesh.vertices[4].uv, glam::Vec2::ZERO);
}

// Build overlapping leaf references to faces whose texture order differs from BSP order.
fn visible_quads() -> Data {
    let mut data = quad();
    data.textures.push(bsp::Texture {
        name: "second".into(),
        dimensions: UVec2::splat(16),
    });
    data.texinfo.push(bsp::Texinfo {
        axes: [Vec4::ZERO; 2],
        texture: 1,
    });
    data.faces = [1, 0, 1]
        .into_iter()
        .map(|texinfo| bsp::Face {
            first_surfedge: 0,
            num_surfedges: 4,
            texinfo,
        })
        .collect();
    data.models[0].num_faces = 3;
    data.models[0].visleafs = 3;
    data.leaves = [(0, 0), (0, 2), (2, 2), (4, 1)]
        .into_iter()
        .map(|(first, count)| bsp::Leaf {
            contents: -1,
            visibility_offset: 0,
            first_marksurface: first,
            num_marksurfaces: count,
        })
        .collect();
    data.marksurfaces = vec![0, 1, 1, 2, 2];
    data.visibility = vec![0b0000_0011];
    data
}

// Prepare visibility selection without requiring a GPU adapter.
fn selection(data: &Data) -> Visibility {
    let mesh = triangulate(data).unwrap();
    Visibility::build(data, mesh.indices, mesh.face_indices).unwrap()
}

#[test]
fn pvs_deduplicates_faces_and_packs_contiguous_texture_batches() {
    let data = visible_quads();
    let mut visibility = selection(&data);
    visibility.select(1).unwrap();
    assert_eq!(visibility.selected, [1, 0, 2]);
    assert_eq!(visibility.packed.len(), 18);
    assert_eq!(
        visibility.ranges,
        [
            TextureRange {
                model_id: 0,
                texture_id: 0,
                indices: 0..6
            },
            TextureRange {
                model_id: 0,
                texture_id: 1,
                indices: 6..18
            },
        ]
    );
    for (&id, packed) in visibility
        .selected
        .iter()
        .zip(visibility.packed.chunks_exact(6))
    {
        let range = visibility.faces[id].indices.clone();
        assert_eq!(
            packed,
            &visibility.source_indices[range.start as usize..range.end as usize]
        );
    }
    // PVS bit 2 selects leaf 3, independently of leaves 1 and 2.
    visibility.visibility[0] = 4;
    visibility.select(3).unwrap();
    assert_eq!(visibility.selected, [2]);
    assert_eq!(visibility.packed.len(), 6);
    assert_eq!(visibility.ranges.len(), 1);
}

#[test]
fn pvs_decodes_zero_runs_and_keeps_the_target_leaf() {
    let mut data = visible_quads();
    data.models[0].visleafs = 9;
    while data.leaves.len() < 10 {
        data.leaves.push(bsp::Leaf {
            contents: -1,
            visibility_offset: 0,
            first_marksurface: 4,
            num_marksurfaces: 1,
        });
    }
    data.visibility = vec![0, 1, 1];
    let mut visibility = selection(&data);
    visibility.select(1).unwrap();
    assert_eq!(visibility.selected, [1, 0, 2]);
    visibility.visibility = vec![0, 2];
    visibility.select(1).unwrap();
    assert_eq!(visibility.selected, [1, 0]);
}

#[test]
fn missing_visibility_and_solid_leaf_fall_back_to_world_faces() {
    let mut data = visible_quads();
    data.leaves[1].visibility_offset = -1;
    // A brush-model face must not enter the world's fallback draw.
    data.models[0].num_faces = 2;
    data.models.push(bsp::Model {
        headnode: 0,
        visleafs: 0,
        first_face: 2,
        num_faces: 1,
    });
    data.marksurfaces = vec![0, 1, 1, 0, 0];
    let mut visibility = selection(&data);
    for leaf in [0, 1] {
        visibility.select(leaf).unwrap();
        assert_eq!(visibility.selected, [1, 0]);
        assert_eq!(visibility.packed.len(), 12);
    }
}

#[test]
fn pvs_rejects_invalid_rows_and_references() {
    let data = visible_quads();
    let mut visibility = selection(&data);
    for row in [vec![], vec![0], vec![0, 0], vec![0, 2]] {
        visibility.visibility = row;
        assert!(visibility.select(1).is_err());
    }
    visibility.leaves[1].visibility_offset = -2;
    assert!(visibility.select(1).is_err());
    assert!(visibility.select(4).is_err());
    let mut data = visible_quads();
    data.marksurfaces[0] = 3;
    let mesh = triangulate(&data).unwrap();
    assert!(Visibility::build(&data, mesh.indices, mesh.face_indices).is_err());
    data.marksurfaces[0] = 0;
    data.leaves[1].num_marksurfaces = 6;
    let mesh = triangulate(&data).unwrap();
    assert!(Visibility::build(&data, mesh.indices, mesh.face_indices).is_err());
}

#[test]
fn selects_every_leaf_of_the_librequake_fixture() {
    let data = Data::decode(include_bytes!("../../assets/maps/librequake/lq_e0m1.bsp")).unwrap();
    let mut visibility = selection(&data);
    for id in 0..data.leaves.len() {
        visibility.select(id).unwrap();
        assert!(visibility.packed.len() <= visibility.source_indices.len());
        assert!(
            visibility
                .ranges
                .windows(2)
                .all(|pair| pair[0].texture_id < pair[1].texture_id)
        );
    }
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn renders_leaf_batches_and_reuses_indices_until_the_leaf_changes() {
    use super::Mesh;
    use crate::gpu::{
        bind_group,
        buffer::uniform::{Camera, camera},
        pipeline, render_pass,
        texture::{Color, Depth},
    };

    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const SIZE: u32 = 4;
    const SHADER: &str = include_str!("../../shader/world.wgsl");

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let data = visible_quads();
    let mut mesh = Mesh::build(&device, &queue, &data).unwrap();
    assert_eq!(mesh.textures(), data.textures);
    drop(data);
    let pipeline = pipeline::World::create(&device, FORMAT, SHADER);
    let camera = Camera::create(&device);
    camera.write(
        &queue,
        camera::Data::construct(Vec3::ZERO, glam::Vec2::ZERO, 1.0),
    );
    let camera = bind_group::Camera::create(&device, &camera);
    let sampler = device.create_sampler(&Default::default());
    let texture = Color::create(&device, UVec2::splat(1));
    texture.write(&queue, &[255; 4]);
    let textures = [
        bind_group::Color::create(&device, &texture, &sampler),
        bind_group::Color::create(&device, &texture, &sampler),
    ];
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = Depth::create(&device, SIZE, SIZE);
    for leaf in [1, 1, 3] {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = render_pass::World::create(&mut encoder, &view, depth.view());
            mesh.render_leaf(&queue, &mut pass, &pipeline, &camera, &textures, leaf)
                .unwrap();
        }
        queue.submit([encoder.finish()]);
        assert_eq!(mesh.cached_leaf, Some(leaf));
        if leaf == 1 {
            assert_eq!(mesh.indices.range(), 0..18);
            // An unchanged leaf must reuse its packed data without decoding this row.
            mesh.visibility.visibility[0] = 0;
        } else {
            // Leaf changes must decode the replacement row and upload the new indices.
            assert_eq!(mesh.indices.range(), 0..6);
        }
        if leaf == 1 && mesh.visibility.visibility[0] == 0 {
            // Give leaf 3 a valid row while leaving leaf 1's cached row invalid.
            mesh.visibility.leaves[3].visibility_offset = 1;
            if mesh.visibility.visibility.len() == 1 {
                mesh.visibility.visibility.push(4);
            }
        }
    }
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
}
