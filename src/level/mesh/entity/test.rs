use crate::{
    bsp, gpu,
    level::{lightmap, mesh},
};

const BSP: &[u8] = include_bytes!("../../../../assets/maps/librequake/lq_e0m1.bsp");
const LIT: &[u8] = include_bytes!("../../../../assets/maps/librequake/lq_e0m1.lit");

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn builds_brush_models_without_pvs_and_shares_worldspawn_lightmaps() {
    let bsp = bsp::Data::decode(BSP).unwrap();
    let samples = bsp::sidecar::lit::decode(LIT).unwrap();
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let mut builder = lightmap::Builder::create(samples);
    let _world = mesh::worldspawn::Mesh::build(&bsp, &mut builder, &device, &queue);
    let mut meshes = Vec::new();
    for (model_index, model) in bsp.models().iter().enumerate().skip(1) {
        let faces = &bsp.faces()[model.faces.clone()];
        let corners: usize = faces.iter().map(|face| face.surfedges.len()).sum();
        let triangles: usize = faces
            .iter()
            .map(|face| face.surfedges.len().saturating_sub(2) * 3)
            .sum();
        let mesh = super::Mesh::build(&bsp, model_index, &mut builder, &device, &queue).unwrap();
        assert_eq!(
            mesh.vertices().buffer().size(),
            (corners * std::mem::size_of::<gpu::buffer::vertex::world::Data>()) as u64
        );
        assert_eq!(mesh.indices().range(), 0..triangles as u32);
        assert_eq!(
            mesh.indices().buffer().size(),
            (triangles * std::mem::size_of::<u32>()) as u64
        );
        let mut end = 0;
        let mut last_texture = None;
        for batch in mesh.batches() {
            assert_eq!(batch.indices.start, end);
            assert!(batch.indices.end > end);
            assert!(last_texture.is_none_or(|previous| previous < batch.texture_id));
            assert!(
                faces
                    .iter()
                    .any(|face| bsp.texinfo()[face.texinfo].texture == batch.texture_id)
            );
            end = batch.indices.end;
            last_texture = Some(batch.texture_id);
        }
        assert_eq!(end, triangles as u32);
        meshes.push(mesh);
    }
    // Invalid model indices fail before changing the shared lightmap builder.
    assert!(super::Mesh::build(&bsp, bsp.models().len(), &mut builder, &device, &queue).is_err());
    // Repacking the first brush face must follow all the already packed model lightmaps.
    let first_brush_face = bsp.models()[1].faces.start;
    let lightmap::Face::Lit { references, .. } = builder.build(&bsp, first_brush_face) else {
        panic!("expected a lit brush face")
    };
    assert!(
        references
            .iter()
            .flatten()
            .any(|reference| reference.atlas > 0 || reference.position != [0, 0])
    );
    let atlases = builder.finish();
    assert!(!atlases.is_empty());
    assert!(atlases.iter().flatten().any(|pixel| pixel[..3] != [0; 3]));
    drop(bsp);
    assert_eq!(meshes.len(), 25);
    assert!(meshes.iter().all(|mesh| !mesh.indices().range().is_empty()));
}
