use std::sync::Arc;

use crate::image::Image;
pub use crate::texture::{amplitude::*, overlay::*, surface::*};

mod amplitude;
mod overlay;
mod surface;

pub(crate) struct Texture {
    pub overlay: OverlayTexture,
    pub surface: SurfaceTexture,
    pub amplitude: AmplitudeTexture,
    pub bind_group: wgpu::BindGroup,
}

impl Texture {
    pub(crate) fn new(
        device: &wgpu::Device,
        surface: Image<f32>,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let overlay_texture = OverlayTexture::new(&surface.size, &device);
        let amplitude_texture = AmplitudeTexture::new(&surface.size, &device);
        let surface_texture = SurfaceTexture::new(Arc::new(surface), &device);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&surface_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&amplitude_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&overlay_texture.view),
                },
            ],
        });
        Self {
            overlay: overlay_texture,
            surface: surface_texture,
            amplitude: amplitude_texture,
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
