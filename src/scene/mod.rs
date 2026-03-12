use imbuf::Image;
use std::sync::Arc;

pub use crate::scene::{overlay::*, surface::*, texture_image::*};

mod overlay;
mod surface;
mod texture_image;

pub(crate) struct Scene {
    pub overlay: OverlayTexture,
    pub surface: SurfaceTexture,
    pub texture: TextureImage,
    pub bind_group: wgpu::BindGroup,
}

impl Scene {
    pub(crate) fn new(
        device: &wgpu::Device,
        surface: Image<f32, 1>,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let overlay_texture = OverlayTexture::new(&surface.dimensions().into(), &device);
        let texture = TextureImage::new(&surface.dimensions().into(), &device);
        let surface_data = SurfaceTexture::new(Arc::new(surface), &device);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&surface_data.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&overlay_texture.view),
                },
            ],
        });
        Self {
            overlay: overlay_texture,
            surface: surface_data,
            texture,
            bind_group: group,
        }
    }

    pub(crate) fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Uint,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
        })
    }
}
