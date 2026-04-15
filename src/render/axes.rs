use std::borrow::Cow;

use wgpu::{Device, RenderPass, util::DeviceExt};

use crate::{
    gpu_data::pixel_picker::PixelPicker, interaction::Interaction,
    render::depth_buffer::DepthBuffer,
};

/// Per-vertex data for grid lines: position + color.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GridVertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl GridVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GridVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Number of subdivisions per axis on the bottom grid.
const GRID_DIVISIONS: u32 = 10;

/// Build grid vertices for the surface bounding box.
///
/// Surface coordinate space:
///   X: [-aspect_ratio, +aspect_ratio]
///   Y: [-1, +1]
///   Z: [-0.5, +0.5]
///
/// Draws a grid on the two back vertical walls (X = x_min and Y = y_min)
/// with horizontal Z-subdivision lines and vertical edge lines where the walls meet.
fn build_grid(aspect_ratio: f32) -> Vec<GridVertex> {
    let x_min = -aspect_ratio;
    let x_max = aspect_ratio;
    let y_min = -1.0_f32;
    let y_max = 1.0_f32;
    let z_min = -0.5_f32;
    let z_max = 0.5_f32;

    let grid_color = [0.35, 0.35, 0.45, 1.0];
    let edge_color = [0.5, 0.5, 0.6, 1.0];

    let n = GRID_DIVISIONS;
    let mut verts = Vec::new();

    // --- Back wall at X = x_max (YZ plane) ---

    // Horizontal lines (constant Z, spanning Y)
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let z = z_min + t * (z_max - z_min);
        let c = if i == 0 || i == n {
            edge_color
        } else {
            grid_color
        };
        verts.push(GridVertex {
            position: [x_max, y_min, z],
            color: c,
        });
        verts.push(GridVertex {
            position: [x_max, y_max, z],
            color: c,
        });
    }

    // Vertical lines at edges (constant Y, spanning Z)
    for &y in &[y_min, y_max] {
        verts.push(GridVertex {
            position: [x_max, y, z_min],
            color: edge_color,
        });
        verts.push(GridVertex {
            position: [x_max, y, z_max],
            color: edge_color,
        });
    }

    // --- Back wall at Y = y_max (XZ plane) ---

    // Horizontal lines (constant Z, spanning X)
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let z = z_min + t * (z_max - z_min);
        let c = if i == 0 || i == n {
            edge_color
        } else {
            grid_color
        };
        verts.push(GridVertex {
            position: [x_min, y_max, z],
            color: c,
        });
        verts.push(GridVertex {
            position: [x_max, y_max, z],
            color: c,
        });
    }

    // Vertical lines at edges (constant X, spanning Z)
    for &x in &[x_min, x_max] {
        verts.push(GridVertex {
            position: [x, y_max, z_min],
            color: edge_color,
        });
        verts.push(GridVertex {
            position: [x, y_max, z_max],
            color: edge_color,
        });
    }

    verts
}

pub(crate) struct Axes {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl Axes {
    pub(crate) fn new(
        device: &Device,
        surface_format: wgpu::TextureFormat,
        interaction: &Interaction,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid_shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../axes_shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid_pipeline_layout"),
            bind_group_layouts: &[
                &interaction.transformation.bind_group_layout,
                &interaction.projection.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let texture_formats = [
            Some(wgpu::ColorTargetState {
                format: surface_format.add_srgb_suffix(),
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            // The picking texture is not written to by the axes shader, but we still need to specify it as a render target to be able to render the axes together with the main image in the same render pass.
            Some(wgpu::ColorTargetState {
                format: PixelPicker::PICKING_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            }),
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_axes"),
                buffers: &[GridVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_axes"),
                compilation_options: Default::default(),
                targets: &texture_formats,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(DepthBuffer::depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertices = build_grid(1.0);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            vertex_buffer,
            vertex_count: vertices.len() as u32,
        }
    }

    /// Rebuild the grid to match the loaded image dimensions.
    pub(crate) fn update_grid(
        &mut self,
        device: &wgpu::Device,
        image_width: u32,
        image_height: u32,
    ) {
        let aspect_ratio = image_width as f32 / image_height as f32;
        let vertices = build_grid(aspect_ratio);
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.vertex_count = vertices.len() as u32;
    }

    pub(crate) fn draw<'a>(
        &'a self,
        renderpass: &mut RenderPass<'a>,
        interaction: &'a Interaction,
    ) {
        renderpass.set_pipeline(&self.pipeline);
        renderpass.set_bind_group(0, &interaction.transformation.bind_group, &[]);
        renderpass.set_bind_group(1, &interaction.projection.bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.draw(0..self.vertex_count, 0..1);
    }
}
