use crate::image::SurfaceAmplitudeImage;
pub use crate::texture::{amplitude::*, overlay::*, surface::*};

mod amplitude;
mod overlay;
mod surface;

pub(crate) struct Texture {
    pub overlay: OverlayTexture,
    pub surface: SurfaceTexture,
    amplitude: AmplitudeTexture,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Texture {
    pub(crate) fn new(device: &wgpu::Device, image: SurfaceAmplitudeImage) -> Self {
        let overlay_texture = OverlayTexture::new(&image.surface.size, &device);
        let surface_texture = SurfaceTexture::new(image.surface, &device);
        let amplitude_texture = AmplitudeTexture::new(image.amplitude, &device);
        let layout = Self::create_bind_group_layout(&device);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &layout,
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
            bind_group_layout: layout,
            overlay: overlay_texture,
            surface: surface_texture,
            amplitude: amplitude_texture,
            bind_group: group,
        }
    }

    pub fn write_to_queue(&self, queue: &wgpu::Queue) {
        self.overlay.write_to_queue(queue);
        self.surface.write_to_queue(queue);
        self.amplitude.write_to_queue(queue);
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
