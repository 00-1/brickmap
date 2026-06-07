//! Generic **polygonal mesh renderer** for a tracked object (the game's space cruiser).
//! Content-free (M9): it draws whatever vertex mesh the game hands it — the cruiser's
//! geometry lives in the game crate. Two draw entry points let the caller place each part
//! in the right post-process stage:
//!
//! - [`ShipRenderer::draw_hull`] draws the **hull** *inside the scene pass*, so it is lit
//!   and then **palettised** by the post chain exactly like the terrain (and depth-tests
//!   against it — it can be occluded by hills).
//! - [`ShipRenderer::draw_lights`] draws the **nav-lights** in a small pass *after* the
//!   palette, so their true colour survives as bright points.
//!
//! Both use one pipeline + the shared `ship.wgsl` (the per-vertex `emissive` flag selects
//! lit hull vs. full-bright light). Scene target and surface share the colour format, so
//! one pipeline serves both passes.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// World-units per model unit (the cruiser is authored at ~9 model units long).
pub const SHIP_SCALE: f32 = 0.7;

/// A mesh vertex the game builds the cruiser from. `emissive` 0 = lit hull, 1 = full-bright
/// nav-light. Public so the game crate can author the geometry.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub emissive: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ShipU {
    mvp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

/// Draws a game-supplied mesh for a tracked object, in two stages (hull / nav-lights).
pub struct ShipRenderer {
    pipeline: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    hull: Option<(wgpu::Buffer, u32)>,
    lights: Option<(wgpu::Buffer, u32)>,
}

impl ShipRenderer {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> ShipRenderer {
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
            hull: None,
            lights: None,
        }
    }

    /// Upload the game's cruiser geometry: the lit `hull` (drawn into the scene, palettised)
    /// and the emissive `lights` (drawn after the palette, true colour). Either may be empty.
    pub fn set_meshes(&mut self, device: &wgpu::Device, hull: &[Vertex], lights: &[Vertex]) {
        use wgpu::util::DeviceExt;
        let make = |verts: &[Vertex], label: &str| {
            (!verts.is_empty()).then(|| {
                let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                (buf, verts.len() as u32)
            })
        };
        self.hull = make(hull, "ship-hull-verts");
        self.lights = make(lights, "ship-light-verts");
    }

    /// Set the transform: `view_proj` from the frame, `pos` (world) + `yaw` (radians).
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

    /// Draw the lit hull **inside** the caller's scene render pass, so the post chain
    /// (bloom + palette) maps it like the rest of the world and it depth-tests against terrain.
    pub fn draw_hull<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let Some((vbuf, count)) = &self.hull else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.draw(0..*count, 0..1);
    }

    /// Draw the emissive nav-lights over `view` (its own `depth`, cleared here), *after* the
    /// palette pass — so their true colour survives as bright points.
    pub fn draw_lights(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) {
        let Some((vbuf, count)) = &self.lights else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ship-lights-pass"),
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
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.draw(0..*count, 0..1);
    }
}
