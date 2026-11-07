use crate::{
    bsp, gpu,
    level::{lightmap, mesh::worldspawn::Mesh},
};
use glam::{Vec2, Vec3};

const BSP_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/lq_e0m1.bsp"
);
const LIT_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/lq_e0m1.lit"
);
const PALETTE_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/maps/librequake/palette.lmp"
);
const POSITION: Vec3 = Vec3::new(512., 2048., 326.);
const ROTATION: Vec2 = Vec2::new(0., 75f32.to_radians());
const WIDTH: u32 = 256;
const HEIGHT: u32 = 144;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn fixture_decodes_with_embedded_mips_sidecars_and_nontrivial_pvs() {
    let bsp = bsp::Data::decode(&std::fs::read(BSP_FILE).unwrap()).unwrap();
    let lit_bytes = std::fs::read(LIT_FILE).unwrap();
    let palette_bytes = std::fs::read(PALETTE_FILE).unwrap();
    assert_eq!(bsp.faces().len(), 4145);
    assert_eq!(bsp.textures().len(), 108);
    assert_eq!(bsp::sidecar::lit::decode(&lit_bytes).unwrap().len(), 187232);
    assert_eq!(
        bsp::sidecar::lmp::decode(&palette_bytes).unwrap().len(),
        256
    );
    let leaf_id = bsp.leaf_at(POSITION.to_array()).unwrap();
    assert_ne!(bsp.leaves()[leaf_id].contents, bsp::Contents::Solid);
    let pvs = bsp.leaves()[leaf_id].pvs.clone().unwrap();
    let visible = &bsp.visibility()[pvs];
    assert!(!visible.is_empty());
    assert!(visible.len() < bsp.models()[0].visleafs);
}

#[test]
#[ignore = "requires a graphics adapter; run explicitly for GPU validation"]
fn renders_librequake_with_palette_lighting_and_pvs_culling() {
    let bsp = bsp::Data::decode(&std::fs::read(BSP_FILE).unwrap()).unwrap();
    let lit_bytes = std::fs::read(LIT_FILE).unwrap();
    let palette_bytes = std::fs::read(PALETTE_FILE).unwrap();
    let samples = bsp::sidecar::lit::decode(&lit_bytes).unwrap();
    let palette = bsp::sidecar::lmp::decode(&palette_bytes).unwrap();
    let leaf = bsp.leaf_at(POSITION.to_array()).unwrap();
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let mut lightmap = lightmap::Builder::create(samples);
    let mut mesh = Mesh::build(&bsp, &mut lightmap, &device, &queue);
    let lightmaps = lightmap.upload(&device, &queue);
    let properties_buffer = crate::gpu::buffer::uniform::Properties::create(&device);
    properties_buffer.write(&queue, crate::level::properties::build(&bsp).water);
    let properties = crate::gpu::bind_group::Properties::create(&device, &properties_buffer);
    let sky_pipeline = crate::gpu::pipeline::sky::create(
        &device,
        FORMAT,
        include_str!("../../../shader/sky.wgsl"),
    );
    let liquid_pipeline = crate::gpu::pipeline::liquid::opaque::create(
        &device,
        FORMAT,
        include_str!("../../../shader/liquid.wgsl"),
    );
    let textures: Vec<_> = bsp
        .textures()
        .iter()
        .enumerate()
        .map(|(index, source)| {
            source
                .as_ref()
                .map(|_| crate::level::texture::build(&bsp, index, palette, &device, &queue))
        })
        .collect();
    for kind in [
        crate::bsp::texture::Kind::Solid,
        crate::bsp::texture::Kind::Sky,
        crate::bsp::texture::Kind::Liquid,
    ] {
        assert!(
            textures
                .iter()
                .flatten()
                .any(|texture| texture.kind() == kind)
        );
    }
    drop(bsp);
    mesh.set_leaf(&queue, leaf).unwrap();
    let camera = gpu::buffer::uniform::Camera::create(&device);
    camera.write(
        &queue,
        gpu::buffer::uniform::camera::Data::construct(
            POSITION,
            ROTATION,
            WIDTH as f32 / HEIGHT as f32,
        ),
    );
    let binding = gpu::bind_group::Camera::create(&device, &camera);
    let pipeline = gpu::pipeline::solid::opaque::create(
        &device,
        FORMAT,
        include_str!("../../../shader/solid.wgsl"),
    );
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let depth = gpu::texture::Depth::create(&device, WIDTH, HEIGHT);
    let row_bytes = WIDTH * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row_bytes * HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = gpu::render_pass::World::create(&mut encoder, &view, depth.view());
        mesh.draw_opaque(
            &mut pass,
            &pipeline,
            &binding,
            &lightmaps,
            &textures,
            &sky_pipeline,
            &liquid_pipeline,
            &properties,
            1.0,
            &properties,
            1.0,
        );
        mesh.draw_transparent(
            &mut pass,
            &liquid_pipeline,
            &binding,
            &lightmaps,
            &textures,
            &properties,
            1.0,
            &properties,
            1.0,
        );
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(HEIGHT),
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
    let colored = pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[..3] != [0; 3])
        .count();
    let mut image = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    image.extend(
        pixels
            .chunks_exact(4)
            .flat_map(|pixel| pixel[..3].iter().copied()),
    );
    std::fs::write(
        std::env::temp_dir().join("bspnet-librequake-smoke.ppm"),
        image,
    )
    .unwrap();
    assert!(
        colored > (WIDTH * HEIGHT / 4) as usize,
        "only {colored} pixels drawn in leaf {leaf}"
    );
}
