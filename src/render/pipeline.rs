use std::borrow::Cow;

use wgpu::{BindGroupLayout, Device, RenderPipeline, TextureFormat};

use crate::{
    gpu_data::pixel_picker::PixelPicker, interaction::Interaction,
    render::depth_buffer::DepthBuffer, vertex_buffer::VertexBuffer,
};

#[derive(Debug)]
pub(crate) enum FragmentShaderVariant {
    Height,
    Texture,
    TurboColormap,
}

pub(crate) struct Pipeline {
    texture: wgpu::RenderPipeline,
    height: wgpu::RenderPipeline,
    turbo_colormap: wgpu::RenderPipeline,
}

impl Pipeline {
    pub(crate) fn new(
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
                    &texture_bind_group_layout,
                    &image_info_bind_group_layout,
                    &interaction.transformation.bind_group_layout,
                    &interaction.projection.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        // Two render targets: main color + picking texture
        let texture_formats = [
            Some(surface_format.add_srgb_suffix().into()),
            Some(PixelPicker::PICKING_FORMAT.into()),
        ];
        let texture_fs_pipeline_descriptor = &wgpu::RenderPipelineDescriptor {
            label: Some("texture_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBuffer::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_texture"),
                compilation_options: Default::default(),
                targets: &texture_formats,
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                ..Default::default()
            },
            depth_stencil: Some(DepthBuffer::depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        };
        let render_pipeline_texture = device.create_render_pipeline(texture_fs_pipeline_descriptor);

        let mut height_fs_pipeline_descriptor = texture_fs_pipeline_descriptor.clone();
        height_fs_pipeline_descriptor.label = Some("height_pipeline");
        height_fs_pipeline_descriptor.fragment = Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_height"),
            compilation_options: Default::default(),
            targets: &texture_formats,
        });
        let render_pipeline_height = device.create_render_pipeline(&height_fs_pipeline_descriptor);

        let mut turbo_colormap_fs_pipeline_descriptor = texture_fs_pipeline_descriptor.clone();
        turbo_colormap_fs_pipeline_descriptor.label = Some("turbo_colormap_pipeline");
        turbo_colormap_fs_pipeline_descriptor.fragment = Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_turbo_colormap"),
            compilation_options: Default::default(),
            targets: &texture_formats,
        });
        let turbo_colormap = device.create_render_pipeline(&turbo_colormap_fs_pipeline_descriptor);
        Self {
            texture: render_pipeline_texture,
            height: render_pipeline_height,
            turbo_colormap,
        }
    }

    pub(crate) fn get(&self, variant: &FragmentShaderVariant) -> &RenderPipeline {
        match variant {
            FragmentShaderVariant::Height => &self.height,
            FragmentShaderVariant::Texture => &self.texture,
            FragmentShaderVariant::TurboColormap => &self.turbo_colormap,
        }
    }
}
