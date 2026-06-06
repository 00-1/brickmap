//! Headless offscreen render → PNG (milestone D1). Native only; uses software
//! Vulkan (Mesa **llvmpipe**) so it needs no GPU or display — install
//! `mesa-vulkan-drivers` for the ICD. It renders the same demo scene with the same
//! `shader.wgsl` and packed vertices as the windowed app; the pipeline setup is
//! duplicated from `gfx` for now (sharing it via a common `Renderer` is a
//! follow-up, see the D1 brief).

use std::sync::mpsc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::mesh::pack;
use crate::particles::{ParticleInstance, ParticleSystem, CUBE_INDICES, CUBE_POSITIONS};

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.02,
    g: 0.03,
    b: 0.06,
    a: 1.0,
};

// These must mirror `gfx` + `shader.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    palette: [[f32; 4]; 8],
    params: [f32; 4],
    camera_pos: [f32; 4],
    fog_color: [f32; 4],
    flags: [f32; 4],
    cam_right: [f32; 4], // xyz = camera right; w = wind time
    cam_up: [f32; 4],    // xyz = camera up
}

/// Distance fog (terrain fades to the sky/clear colour). The live cruise camera
/// flies low over the terrain; this headless hero shot frames the whole demo world
/// from far back, so its fog pushes out to match — only the far edge fades.
const FOG_START: f32 = 150.0;
const FOG_END: f32 = 300.0;

/// Default aesthetic dials (must match the windowed defaults in `lib`).
pub const DEFAULT_AESTHETIC: [f32; 2] = [85.0, 4.0];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ChunkUniform {
    origin: [f32; 4],
}

const PALETTE: [[f32; 4]; 8] = [
    [0.95, 0.10, 0.95, 1.0],
    [0.55, 0.55, 0.58, 1.0],
    [0.55, 0.40, 0.25, 1.0],
    [0.40, 0.70, 0.35, 1.0],
    [0.80, 0.78, 0.65, 1.0],
    [0.92, 0.94, 0.98, 1.0], // snow
    [0.45, 0.95, 1.00, 1.0], // crystal (emissive)
    [0.18, 0.38, 0.62, 1.0], // water
];

/// How to drive the palette post-process for a headless capture: which entry of
/// [`crate::palette::PALETTES`], how many of its colours to use, and the dither spread.
/// `None` (the default) skips the pass entirely so the hero shot is unchanged.
#[derive(Copy, Clone)]
pub struct PaletteSpec {
    pub index: usize,
    pub count: u32,
    pub dither: f32,
}

/// Render the demo scene to a PNG at `path`. Panics on setup failure (it's a dev
/// tool); the most likely cause is a missing software-Vulkan ICD.
pub fn capture(width: u32, height: u32, path: &str) {
    capture_view(width, height, path, None, None, None, true, 1);
}

/// As [`capture`], but with an optional camera override (`eye` looking at `target`) so
/// the dev tool can frame the world from any angle — invaluable for verifying features
/// (water, rivers, caves) that the default hero shot hides. Both `None` → the default
/// whole-world framing.
#[allow(clippy::too_many_arguments)]
pub fn capture_view(
    width: u32,
    height: u32,
    path: &str,
    eye: Option<glam::Vec3>,
    target: Option<glam::Vec3>,
    palette: Option<PaletteSpec>,
    sun: bool,
    scale: u32,
) {
    // Internal render resolution (E10 pixel scale): scene + post render at (iw, ih), then the
    // present/palette pass upscales (nearest) to the full-res `target`.
    let scale = scale.clamp(1, 8);
    let (iw, ih) = ((width / scale).max(1), (height / scale).max(1));
    let instances = crate::build_world_meshes(&crate::demo_world());
    let camera = match (eye, target) {
        (Some(e), Some(t)) => crate::scene::Camera::looking_at(e, t),
        _ => crate::frame_camera(&instances).0,
    };
    // Camera basis for billboarding foliage splats (E6).
    let fwd = camera.forward();
    let cam_right = fwd.cross(glam::Vec3::Y).normalize_or_zero();
    let cam_up = cam_right.cross(fwd).normalize_or_zero();
    // All foliage in one instance buffer (the hero shot draws everything, no culling).
    let foliage_all: Vec<crate::foliage::SplatInstance> =
        instances.iter().flat_map(|i| i.foliage.clone()).collect();

    // Draw front-to-back (M8): sort chunks by distance to the camera so early-Z rejects
    // overdraw behind nearer ones. Opaque output is order-independent, so this is a pure
    // perf change — the render is identical, which also verifies the windowed app's sort.
    let mut instances = instances;
    instances.sort_by(|a, b| {
        let c = |i: &crate::gfx::ChunkInstance| {
            (glam::Vec3::from(i.mesh.aabb.min) + glam::Vec3::from(i.mesh.aabb.max)) * 0.5 + i.origin
        };
        (c(a) - camera.position)
            .length_squared()
            .total_cmp(&(c(b) - camera.position).length_squared())
    });

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("no headless GPU adapter — is mesa-vulkan-drivers installed?");
    log::info!("headless adapter: {:?}", adapter.get_info());

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("headless-device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .expect("failed to create headless device");

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless-color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // Offscreen scene target (bloom reads this, then composites into `target`). At the
    // internal resolution (iw, ih).
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless-scene"),
        size: wgpu::Extent3d {
            width: iw,
            height: ih,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());

    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless-depth"),
        size: wgpu::Extent3d {
            width: iw,
            height: ih,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let uniform_bgl = |vis| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: vis,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    };
    let globals_bgl = uniform_bgl(wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT);
    let chunk_bgl = uniform_bgl(wgpu::ShaderStages::VERTEX);

    // Material texture array (group 2) — shared construction with the windowed renderer.
    let material_bgl = crate::gfx::material_bind_group_layout(&device);
    let material_bind_group = crate::gfx::build_material_bind_group(&device, &queue, &material_bgl);

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("headless-pipeline-layout"),
        bind_group_layouts: &[Some(&globals_bgl), Some(&chunk_bgl), Some(&material_bgl)],
        immediate_size: 0,
    });
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: (2 * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Uint32, 1 => Uint32],
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("headless-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let sky_pipeline = crate::gfx::build_sky_pipeline(&device, COLOR_FORMAT);
    let bloom = crate::post::Bloom::new(&device, COLOR_FORMAT, iw, ih);

    // Present/palette pass (E10). Bloom composites into an intermediate `post` texture at the
    // internal resolution; this pass then writes the full-res `target`, palettising (when a
    // palette is requested) and always upscaling by the pixel scale with nearest sampling.
    // Needed whenever there's a palette *or* an upscale; otherwise bloom writes `target` direct.
    let palette_pass = (palette.is_some() || scale > 1).then(|| {
        let pass = crate::palette::PalettePass::new(&device, COLOR_FORMAT);
        match palette {
            Some(spec) => pass.set(&queue, spec.index, spec.count, spec.dither, true),
            None => pass.set(&queue, 0, 1, 0.0, false), // passthrough (upscale only)
        }
        let post = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless-post"),
            size: wgpu::Extent3d {
                width: iw,
                height: ih,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let post_view = post.create_view(&wgpu::TextureViewDescriptor::default());
        let bg = pass.make_bind_group(&device, &post_view);
        (pass, post_view, bg)
    });

    let globals = Globals {
        view_proj: camera
            .view_proj(width as f32 / height as f32)
            .to_cols_array_2d(),
        palette: PALETTE,
        params: [
            DEFAULT_AESTHETIC[0],
            DEFAULT_AESTHETIC[1],
            FOG_START,
            FOG_END,
        ],
        // w = directional-sun flag (0 = point-lit only), mirroring `gfx`.
        camera_pos: [
            camera.position.x,
            camera.position.y,
            camera.position.z,
            if sun { 1.0 } else { 0.0 },
        ],
        // Horizon band of the sky gradient (see sky.wgsl) so terrain melts into it.
        fog_color: [0.30, 0.33, 0.42, 1.0],
        // All features on for the hero shot (AO, block light, emissive, relief).
        flags: [1.0, 1.0, 1.0, 1.0],
        // Camera basis for billboarding foliage; w = a fixed wind time for the still.
        cam_right: [cam_right.x, cam_right.y, cam_right.z, 0.6],
        cam_up: [cam_up.x, cam_up.y, cam_up.z, 0.0],
    };
    let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("globals"),
        contents: bytemuck::bytes_of(&globals),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("globals-bg"),
        layout: &globals_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: globals_buffer.as_entire_binding(),
        }],
    });

    struct Draw {
        vbuf: wgpu::Buffer,
        ibuf: wgpu::Buffer,
        n: u32,
        origin_bg: wgpu::BindGroup,
    }
    let draws: Vec<Draw> = instances
        .iter()
        .filter(|i| !i.mesh.is_empty())
        .map(|inst| {
            let packed: Vec<[u32; 2]> = inst.mesh.vertices.iter().map(pack).collect();
            let origin = ChunkUniform {
                origin: [inst.origin.x, inst.origin.y, inst.origin.z, 0.0],
            };
            let origin_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunk-origin"),
                contents: bytemuck::bytes_of(&origin),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            Draw {
                vbuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chunk-vertices"),
                    contents: bytemuck::cast_slice(&packed),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                ibuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chunk-indices"),
                    contents: bytemuck::cast_slice(&inst.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                n: inst.mesh.indices.len() as u32,
                origin_bg: device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("chunk-origin-bg"),
                    layout: &chunk_bgl,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: origin_buffer.as_entire_binding(),
                    }],
                }),
            }
        })
        .collect();

    // Particles: simulate ~0.7 s so a burst is mid-flight, then build the instanced
    // emissive-cube pipeline (shares the globals group).
    let mut psys = ParticleSystem::new(crate::demo_emitters());
    for _ in 0..78 {
        psys.update(1.0 / 60.0);
    }
    let particle_instances = psys.instances();

    let particle_shader = device.create_shader_module(wgpu::include_wgsl!("particles.wgsl"));
    let particle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("headless-particle-layout"),
        bind_group_layouts: &[Some(&globals_bgl)],
        immediate_size: 0,
    });
    let cube_layout = wgpu::VertexBufferLayout {
        array_stride: (3 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    };
    let inst_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ParticleInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32, 3 => Float32x3],
    };
    let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("headless-particle-pipeline"),
        layout: Some(&particle_layout),
        vertex: wgpu::VertexState {
            module: &particle_shader,
            entry_point: Some("vs_main"),
            buffers: &[cube_layout, inst_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &particle_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let cube_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cube-vertices"),
        contents: bytemuck::bytes_of(&CUBE_POSITIONS),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let cube_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("cube-indices"),
        contents: bytemuck::bytes_of(&CUBE_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });
    let particle_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("particle-instances"),
        contents: bytemuck::cast_slice(if particle_instances.is_empty() {
            &[ParticleInstance {
                offset: [0.0; 3],
                size: 0.0,
                color: [0.0; 3],
                _pad: 0.0,
            }]
        } else {
            &particle_instances
        }),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // In-world text demo (E17): a few glowing inscriptions standing on the terrain, in mixed
    // scripts (Greek + Latin), to verify the world-text billboard path offscreen.
    let mut world_text =
        crate::text::WorldText::new(&device, COLOR_FORMAT, DEPTH_FORMAT, &globals_bgl);
    for (txt, x, z, col) in [
        ("ΒΡΙΚΜΑΠ", -20i32, 0i32, [0.95, 0.97, 0.75]),
        ("brickmap", 24, -28, [0.75, 0.9, 1.0]),
        ("ΔΟΟΜ", 40, 34, [0.95, 0.6, 0.4]),
    ] {
        let gy = crate::worldgen::height(x, z, crate::WORLD_SEED) as f32;
        world_text.add(
            &device,
            &queue,
            txt,
            glam::Vec3::new(x as f32, gy + 9.0, z as f32),
            6.0,
            col,
        );
    }

    // Foliage splats (E6): instanced billboards sharing the globals; one buffer for all.
    let splat_pipeline = crate::gfx::build_splat_pipeline(&device, &globals_bgl, COLOR_FORMAT);
    let splat_vbuf = (!foliage_all.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("foliage-instances"),
            contents: bytemuck::cast_slice(&foliage_all),
            usage: wgpu::BufferUsages::VERTEX,
        })
    });

    // Row pitch must be aligned to 256 bytes for texture-to-buffer copies.
    let unpadded_bpr = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bpr = unpadded_bpr.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded_bpr * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("headless-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &scene_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard, // nothing reads depth after this pass (M8)
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&sky_pipeline);
        pass.draw(0..3, 0..1);

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &globals_bind_group, &[]);
        pass.set_bind_group(2, &material_bind_group, &[]);
        for d in &draws {
            pass.set_bind_group(1, &d.origin_bg, &[]);
            pass.set_vertex_buffer(0, d.vbuf.slice(..));
            pass.set_index_buffer(d.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..d.n, 0, 0..1);
        }
        // Foliage splats over the terrain (before particles so embers draw on top).
        if let Some(buf) = &splat_vbuf {
            pass.set_pipeline(&splat_pipeline);
            pass.set_bind_group(0, &globals_bind_group, &[]);
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..6, 0..foliage_all.len() as u32);
        }
        if !particle_instances.is_empty() {
            pass.set_pipeline(&particle_pipeline);
            pass.set_bind_group(0, &globals_bind_group, &[]);
            pass.set_vertex_buffer(0, cube_vbuf.slice(..));
            pass.set_vertex_buffer(1, particle_vbuf.slice(..));
            pass.set_index_buffer(cube_ibuf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(
                0..CUBE_INDICES.len() as u32,
                0,
                0..particle_instances.len() as u32,
            );
        }
        // In-world text inscriptions (E17), depth-tested against the scene.
        world_text.draw(&mut pass, &globals_bind_group);
    }
    // Bloom: composite scene + glow. With a palette it lands in `post`, which the palette
    // pass then maps into `target`; without one it composites straight into `target`.
    match &palette_pass {
        Some((pass, post_view, bg)) => {
            bloom.render(&device, &mut encoder, &scene_view, post_view);
            pass.render(&mut encoder, bg, &target_view);
        }
        None => bloom.render(&device, &mut encoder, &scene_view, &target_view),
    }
    // In-engine HUD overlay — same code path as the live app, so the hero shot verifies it.
    let mut hud = crate::hud::HudOverlay::new(&device, COLOR_FORMAT);
    hud.set_text(
        &device,
        &queue,
        &format!(
            "brickmap {} - {} chunks - seed {}",
            crate::BUILD,
            instances.len(),
            crate::WORLD_SEED
        ),
    );
    hud.draw(&queue, &mut encoder, &target_view, width, height);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    // Map the readback buffer and wait for the GPU.
    let (tx, rx) = mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().expect("map channel").expect("buffer map failed");

    let data = readback.slice(..).get_mapped_range();
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded_bpr) as usize;
        rgba.extend_from_slice(&data[start..start + unpadded_bpr as usize]);
    }
    drop(data);
    readback.unmap();

    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgba)
        .expect("png data");
    log::info!("wrote {path} ({width}x{height}, {} chunks)", draws.len());
}
