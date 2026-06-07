//! Space cruiser model + renderer (E19). A small **polygonal** ship (boxes — fuselage, wings,
//! fin, cockpit, engines) with glowing **nav-lights** (front/back + port/starboard wingtips),
//! drawn in its own pass *after* the palette post-process so its true colours survive (see
//! `ship.wgsl`). Has its own depth buffer so it self-occludes; draws over the world as an
//! always-visible landmark when parked.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// World-units per model unit (the authored ship is ~9 model units long).
pub const SHIP_SCALE: f32 = 0.7;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
    normal: [f32; 3],
    emissive: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ShipU {
    mvp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

/// Append a box (centre `c`, half-extents `h`) of colour `col` (24/36-vert, per-face normals).
fn add_box(v: &mut Vec<Vertex>, c: [f32; 3], h: [f32; 3], col: [f32; 3], emissive: f32) {
    // (face normal, 4 corner sign-triples in CCW-ish order — winding is irrelevant since the
    // pass doesn't cull and uses a depth buffer; normals are explicit for lighting).
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        (
            [1.0, 0.0, 0.0],
            [
                [1.0, -1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, 1.0, 1.0],
                [1.0, -1.0, 1.0],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [-1.0, -1.0, -1.0],
                [-1.0, -1.0, 1.0],
                [-1.0, 1.0, 1.0],
                [-1.0, 1.0, -1.0],
            ],
        ),
        (
            [0.0, 1.0, 0.0],
            [
                [-1.0, 1.0, -1.0],
                [-1.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, -1.0],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [-1.0, -1.0, -1.0],
                [1.0, -1.0, -1.0],
                [1.0, -1.0, 1.0],
                [-1.0, -1.0, 1.0],
            ],
        ),
        (
            [0.0, 0.0, 1.0],
            [
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, 1.0],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [-1.0, -1.0, -1.0],
                [-1.0, 1.0, -1.0],
                [1.0, 1.0, -1.0],
                [1.0, -1.0, -1.0],
            ],
        ),
    ];
    for (normal, quad) in faces {
        let corner = |q: [f32; 3]| [c[0] + q[0] * h[0], c[1] + q[1] * h[1], c[2] + q[2] * h[2]];
        for &i in &[0usize, 1, 2, 0, 2, 3] {
            v.push(Vertex {
                pos: corner(quad[i]),
                color: col,
                normal,
                emissive,
            });
        }
    }
}

/// Build the ship mesh (model space; +z = forward, +y = up).
fn ship_mesh() -> Vec<Vertex> {
    let hull = [0.55, 0.60, 0.68];
    let dark = [0.14, 0.18, 0.26];
    let mut v = Vec::new();
    add_box(&mut v, [0.0, 0.0, 0.0], [0.8, 0.7, 3.5], hull, 0.0); // fuselage
    add_box(&mut v, [0.0, 0.0, 3.7], [0.45, 0.45, 0.8], hull, 0.0); // nose
    add_box(&mut v, [0.0, 0.55, 1.0], [0.5, 0.4, 1.1], dark, 0.0); // cockpit
    add_box(&mut v, [2.2, -0.1, -0.4], [1.6, 0.15, 1.2], hull, 0.0); // right wing
    add_box(&mut v, [-2.2, -0.1, -0.4], [1.6, 0.15, 1.2], hull, 0.0); // left wing
    add_box(&mut v, [0.0, 0.95, -3.0], [0.12, 0.9, 0.7], hull, 0.0); // tail fin
    add_box(&mut v, [0.6, 0.0, -3.9], [0.4, 0.4, 0.5], dark, 0.0); // engines
    add_box(&mut v, [-0.6, 0.0, -3.9], [0.4, 0.4, 0.5], dark, 0.0);
    // Nav-lights (emissive, true colour over the palette): white-blue nose, amber tail, and
    // port-red / starboard-green wingtips.
    add_box(
        &mut v,
        [0.0, 0.1, 4.6],
        [0.22, 0.22, 0.22],
        [0.7, 0.9, 1.0],
        1.0,
    );
    add_box(
        &mut v,
        [0.0, 0.45, -3.9],
        [0.22, 0.22, 0.22],
        [1.0, 0.7, 0.2],
        1.0,
    );
    add_box(
        &mut v,
        [-3.7, -0.1, -0.4],
        [0.25, 0.2, 0.25],
        [1.0, 0.2, 0.2],
        1.0,
    );
    add_box(
        &mut v,
        [3.7, -0.1, -0.4],
        [0.25, 0.2, 0.25],
        [0.2, 1.0, 0.3],
        1.0,
    );
    v
}

/// Renders the cruiser over the finished (palettised) frame, in true colour, with self-occlusion.
pub struct ShipRenderer {
    pipeline: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vbuf: wgpu::Buffer,
    vcount: u32,
}

impl ShipRenderer {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> ShipRenderer {
        use wgpu::util::DeviceExt;
        let shader = device.create_shader_module(wgpu::include_wgsl!("ship.wgsl"));
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ship-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ship-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ship-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None, // depth-sorted instead (winding-agnostic)
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let mesh = ship_mesh();
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ship-verts"),
            contents: bytemuck::cast_slice(&mesh),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ship-uniform"),
            size: std::mem::size_of::<ShipU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ship-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        ShipRenderer {
            pipeline,
            ubuf,
            bind_group,
            vbuf,
            vcount: mesh.len() as u32,
        }
    }

    /// Set the transform: `view_proj` from the frame, `pos` (world) + `yaw` (radians) of the ship.
    pub fn set_transform(&self, queue: &wgpu::Queue, view_proj: Mat4, pos: Vec3, yaw: f32) {
        let model = Mat4::from_translation(pos)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::splat(SHIP_SCALE));
        let u = ShipU {
            mvp: (view_proj * model).to_cols_array_2d(),
            model: model.to_cols_array_2d(),
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
    }

    /// Draw the ship over `view` (its own `depth` buffer, cleared here, gives self-occlusion).
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ship-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vbuf.slice(..));
        pass.draw(0..self.vcount, 0..1);
    }
}
