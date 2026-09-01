use std::borrow::Cow;

use wgpu::{BindGroupLayout, Device, RenderPipeline, TextureFormat};

use crate::{
    gpu_data::pixel_picker::PixelPicker, interaction::Interaction,
    render::depth_buffer::DepthBuffer, vertex_buffer::VertexBuffer,
};

#[derive(Debug, Clone, Copy)]
pub enum FragmentShaderVariant {
    Height,
    Texture,
    TextureTurbo,
    TurboColormap,
}

pub struct Pipeline {
    texture: wgpu::RenderPipeline,
    texture_turbo: wgpu::RenderPipeline,
    height: wgpu::RenderPipeline,
    turbo_colormap: wgpu::RenderPipeline,
}

impl Pipeline {
    pub fn new(
        device: &Device,
        surface_format: TextureFormat,
        texture_bind_group_layout: &BindGroupLayout,
        image_info_bind_group_layout: &BindGroupLayout,
        interaction: &Interaction,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/main.wgsl"))),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[
                    Some(texture_bind_group_layout),
                    Some(image_info_bind_group_layout),
                    Some(&interaction.transformation.bind_group_layout),
                    Some(&interaction.projection.bind_group_layout),
                ],
                immediate_size: 0,
            });

        // Two render targets: main color + picking texture
        let texture_formats = [
            Some(surface_format.add_srgb_suffix().into()),
            Some(PixelPicker::PICKING_FORMAT.into()),
        ];
        let base_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBuffer::desc()],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthBuffer::depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        };

        let create_pipeline = |label, entry_point| {
            let mut desc = base_descriptor.clone();
            desc.label = Some(label);
            desc.fragment = Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                targets: &texture_formats,
            });
            device.create_render_pipeline(&desc)
        };

        Self {
            texture: create_pipeline("texture_pipeline", "fs_texture"),
            texture_turbo: create_pipeline("texture_turbo_pipeline", "fs_texture_turbo"),
            height: create_pipeline("height_pipeline", "fs_height"),
            turbo_colormap: create_pipeline("turbo_colormap_pipeline", "fs_turbo_colormap"),
        }
    }

    pub fn get(&self, variant: &FragmentShaderVariant) -> &RenderPipeline {
        match variant {
            FragmentShaderVariant::Height => &self.height,
            FragmentShaderVariant::Texture => &self.texture,
            FragmentShaderVariant::TextureTurbo => &self.texture_turbo,
            FragmentShaderVariant::TurboColormap => &self.turbo_colormap,
        }
    }
}
